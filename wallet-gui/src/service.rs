//! Background service task — single `select!` loop, no spawns, no sleeps.
//!
//! The service owns all async I/O. It receives [`UiEvent`]s from the UI thread,
//! calls the masternode JSON-RPC client, and sends [`ServiceEvent`]s back.
//! WebSocket events are forwarded as-is.

use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use std::time::Instant;

use crate::config_new::Config;
use crate::events::{Screen, ServiceEvent, UiEvent};
use crate::masternode_client::{MasternodeClient, TransactionRecord, TransactionStatus};
use crate::peer_discovery;
use crate::state::{AddressInfo, PeerInfo};
use crate::wallet_dat;
use crate::wallet_db::{AddressContact, WalletDb};
use crate::wallet_manager::WalletManager;
use crate::ws_client::{WsClient, WsEvent};
use wallet::NetworkType;

type DiscoveryHandle = tokio::task::JoinHandle<Result<(String, Vec<PeerInfo>), ()>>;

/// Run the service loop until the cancellation token fires.
///
/// This is the **only** `tokio::spawn`ed task in the application. It owns the
/// masternode client, wallet manager, and WebSocket connection.
pub async fn run(
    token: CancellationToken,
    mut ui_rx: mpsc::UnboundedReceiver<UiEvent>,
    svc_tx: mpsc::UnboundedSender<ServiceEvent>,
    mut config: Config,
) {
    let (ws_event_tx, mut ws_event_rx) = mpsc::unbounded_channel::<WsEvent>();
    let (ws_shutdown_tx, ws_shutdown_rx) = tokio::sync::watch::channel(false);

    let network_type = if config.is_testnet() {
        NetworkType::Testnet
    } else {
        NetworkType::Mainnet
    };

    // Open wallet metadata database
    let db_path = config.wallet_dir().join("wallet_db");
    if let Some(parent) = db_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let wallet_db = WalletDb::open(&db_path).ok();
    if wallet_db.is_some() {
        log::info!("📂 Wallet database opened at: {}", db_path.display());
    }

    // Load persisted display preferences
    if let Some(ref db) = wallet_db {
        if let Ok(Some(dp_str)) = db.get_setting("decimal_places") {
            if let Ok(dp) = dp_str.parse::<usize>() {
                let _ = svc_tx.send(ServiceEvent::DecimalPlacesLoaded(dp));
            }
        }
    }

    // Load editor preference from config
    let _ = svc_tx.send(ServiceEvent::EditorLoaded(config.editor.clone()));

    let mut state = ServiceState {
        svc_tx,
        client: None,
        wallet: None,
        wallet_db,
        addresses: Vec::new(),
        network_type,
        config: config.clone(),
        ws_event_tx,
        ws_shutdown_rx,
        ws_handle: None,
        last_peers: Vec::new(),
    };

    // Auto-load wallet if it exists
    if WalletManager::exists(network_type) {
        if WalletManager::is_encrypted(network_type).unwrap_or(false) {
            let _ = state.svc_tx.send(ServiceEvent::PasswordRequired);
        } else {
            state.load_wallet(None);
        }
    }

    // Kick off peer discovery in the background (skip on first run — wait for network selection)
    let mut is_testnet = config.is_testnet();
    let mut manual_endpoints = config.manual_endpoints();
    let mut rpc_credentials = config.rpc_credentials();
    let mut discovery_handle: Option<DiscoveryHandle> = if config.is_first_run {
        None
    } else {
        let discovery_svc_tx = state.svc_tx.clone();
        let discovery_endpoints = manual_endpoints.clone();
        let discovery_creds = rpc_credentials.clone();
        let max_conn = config.max_connections;
        Some(tokio::spawn(async move {
            discover_peers(
                is_testnet,
                discovery_endpoints,
                discovery_creds,
                &discovery_svc_tx,
                max_conn,
            )
            .await
        }))
    };

    // Single 5-second poll: block height every tick, heavy data every 3rd tick (15s)
    let mut poll_interval = tokio::time::interval(std::time::Duration::from_secs(5));
    poll_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    poll_interval.tick().await;
    let mut poll_tick: u8 = 0;

    // Peer discovery refresh interval (separate from data poll)
    let mut peer_refresh_interval = tokio::time::interval(std::time::Duration::from_secs(5));
    peer_refresh_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    peer_refresh_interval.tick().await;

    log::info!("🚀 Service loop started ({})", state.config.network);

    let mut initial_sync_done = false;

    loop {
        tokio::select! {
            _ = token.cancelled() => {
                log::info!("🛑 Service loop shutting down");
                let _ = ws_shutdown_tx.send(true);
                break;
            }

            // Unified poll: block height every 5s, heavy data every 15s
            _ = poll_interval.tick() => {
                poll_tick = poll_tick.wrapping_add(1);
                if let Some(ref client) = state.client {
                    // Fast: block height every tick
                    if let Ok(height) = client.get_block_height().await {
                        let _ = state.svc_tx.send(ServiceEvent::BlockHeightUpdated(height));
                    }

                    // Heavy: balance / transactions / UTXOs every 3rd tick (15s),
                    // but always on the first tick for instant startup.
                    if (poll_tick == 1 || poll_tick.is_multiple_of(3)) && !state.addresses.is_empty() {
                        match client.get_balances(&state.addresses).await {
                            Ok(bal) => {
                                if let Some(ref db) = state.wallet_db {
                                    let _ = db.save_cached_balance(&bal);
                                }
                                let _ = state.svc_tx.send(ServiceEvent::BalanceUpdated(bal));
                            }
                            Err(e) => log::warn!("Balance poll failed: {}", e),
                        }
                        if let Ok(txs) = client.get_transactions_multi(&state.addresses, 0).await {
                            if let Some(ref db) = state.wallet_db {
                                let _ = db.save_cached_transactions(&txs);
                            }
                            let _ = state.svc_tx.send(ServiceEvent::TransactionsUpdated(txs));
                            if !initial_sync_done {
                                initial_sync_done = true;
                                let _ = state.svc_tx.send(ServiceEvent::SyncComplete);
                            }
                        }
                        let mut all_utxos = Vec::new();
                        for addr in &state.addresses {
                            if let Ok(utxos) = client.get_utxos(addr).await {
                                all_utxos.extend(utxos);
                            }
                        }
                        state.send_utxos_updated(all_utxos);
                    }
                }
            }

            // Periodic peer refresh
            _ = peer_refresh_interval.tick(), if discovery_handle.is_none() => {
                let tx = state.svc_tx.clone();
                let eps = manual_endpoints.clone();
                let creds = rpc_credentials.clone();
                let max_conn = state.config.max_connections;
                discovery_handle = Some(tokio::spawn(async move {
                    discover_peers(is_testnet, eps, creds, &tx, max_conn).await
                }));
            }

            // Peer discovery completes in the background
            Some(result) = async {
                if let Some(ref mut handle) = discovery_handle {
                    Some(handle.await)
                } else {
                    std::future::pending::<Option<Result<Result<(String, Vec<PeerInfo>), ()>, tokio::task::JoinError>>>().await
                }
            } => {
                discovery_handle = None;
                if let Ok(Ok((endpoint, peer_infos))) = result {
                    // Check before move: is the active peer still healthy?
                    let active_is_healthy = state.client.as_ref().map(|c| {
                        peer_infos.iter().any(|p| p.endpoint == c.endpoint() && p.is_healthy)
                    }).unwrap_or(false);

                    state.last_peers = peer_infos.clone();
                    let _ = state.svc_tx.send(ServiceEvent::PeersDiscovered(peer_infos));

                    // Switch peer if we have none, or if the active one has become unhealthy
                    if state.client.is_none() || !active_is_healthy {
                        if let Some(ref old) = state.client {
                            log::warn!("🔄 Active peer {} is unhealthy, reconnecting to {}", old.endpoint(), endpoint);
                        } else {
                            log::info!("🔗 Using peer: {}", endpoint);
                        }
                        state.client = Some(MasternodeClient::new(endpoint.clone(), rpc_credentials.clone()));
                        state.config.active_endpoint = Some(endpoint.clone());
                        config.active_endpoint = Some(endpoint);

                        // If wallet is already loaded, restart WS and refresh data
                        if !state.addresses.is_empty() {
                            state.start_ws();
                            if let Some(ref client) = state.client {
                                if let Ok(bal) = client.get_balances(&state.addresses).await {
                                    let _ = state.svc_tx.send(ServiceEvent::BalanceUpdated(bal));
                                }
                            }
                        }
                    }
                }
            }

            Some(event) = ui_rx.recv() => {
                match event {
                    UiEvent::Shutdown => {
                        let _ = ws_shutdown_tx.send(true);
                        break;
                    }

                    UiEvent::LoadWallet { password } => {
                        state.load_wallet(password);
                    }

                    UiEvent::CreateWallet { mnemonic, password } => {
                        state.create_wallet(&mnemonic, password);
                    }

                    UiEvent::PrepareNewWallet => {
                        let wallet_path = wallet_dat::WalletDat::default_path(state.network_type);
                        let backed_up = if wallet_path.exists() {
                            let date = chrono::Local::now().format("%Y-%m-%d_%H%M%S");
                            let backup_name = format!("time-wallet-{}.dat", date);
                            let backup_path = wallet_path.with_file_name(&backup_name);
                            match std::fs::rename(&wallet_path, &backup_path) {
                                Ok(_) => {
                                    log::info!("Backed up wallet to {}", backup_path.display());
                                    Some(backup_path.display().to_string())
                                }
                                Err(e) => {
                                    let _ = state.svc_tx.send(ServiceEvent::Error(
                                        format!("Failed to backup wallet: {}", e),
                                    ));
                                    return;
                                }
                            }
                        } else {
                            None
                        };
                        let _ = state.svc_tx.send(ServiceEvent::ReadyForMnemonic {
                            backed_up_path: backed_up,
                        });
                    }

                    UiEvent::RefreshBalance => {
                        if let Some(ref client) = state.client {
                            if !state.addresses.is_empty() {
                                match client.get_balances(&state.addresses).await {
                                    Ok(balance) => { let _ = state.svc_tx.send(ServiceEvent::BalanceUpdated(balance)); }
                                    Err(e) => { let _ = state.svc_tx.send(ServiceEvent::Error(e.to_string())); }
                                }
                            }
                        }
                    }

                    UiEvent::RefreshTransactions => {
                        if let Some(ref client) = state.client {
                            if !state.addresses.is_empty() {
                                match client.get_transactions_multi(&state.addresses, 0).await {
                                    Ok(txs) => { let _ = state.svc_tx.send(ServiceEvent::TransactionsUpdated(txs)); }
                                    Err(e) => { let _ = state.svc_tx.send(ServiceEvent::Error(e.to_string())); }
                                }
                            }
                        }
                    }

                    UiEvent::RefreshUtxos => {
                        if let Some(ref client) = state.client {
                            let mut all_utxos = Vec::new();
                            for addr in &state.addresses {
                                match client.get_utxos(addr).await {
                                    Ok(utxos) => all_utxos.extend(utxos),
                                    Err(e) => {
                                        let _ = state.svc_tx.send(ServiceEvent::Error(e.to_string()));
                                        break;
                                    }
                                }
                            }
                            state.send_utxos_updated(all_utxos);
                        }
                    }

                    UiEvent::SendTransaction { to, amount, fee } => {
                        if let Some(ref client) = state.client {
                            if let Some(ref mut wm) = state.wallet {
                                // Retry loop: wait for locked UTXOs to finalize
                                let max_retries = 5;
                                let total_needed = amount + wallet::calculate_fee(amount);
                                let mut all_utxos = Vec::new();
                                let mut tx_result: Result<wallet::Transaction, String> =
                                    Err(String::new());

                                for attempt in 0..max_retries {
                                    // Fetch UTXOs from masternode and sync into wallet
                                    all_utxos.clear();
                                    for addr in &state.addresses {
                                        match client.get_utxos(addr).await {
                                            Ok(utxos) => {
                                                all_utxos.extend(utxos);
                                            }
                                            Err(e) => {
                                                log::warn!("Failed to fetch UTXOs for {}: {}", addr, e);
                                            }
                                        }
                                    }
                                    log::debug!("UTXO sync (attempt {}): {} UTXOs fetched", attempt + 1, all_utxos.len());
                                    let wallet_inner = wm.get_active_wallet_mut();
                                    while !wallet_inner.utxos().is_empty() {
                                        let u = wallet_inner.utxos()[0].clone();
                                        wallet_inner.remove_utxo(&u.tx_hash, u.output_index);
                                    }
                                    let mut total_balance = 0u64;
                                    for utxo in &all_utxos {
                                        let mut tx_hash = [0u8; 32];
                                        let hex_chars: Vec<u8> = utxo.txid.bytes().collect();
                                        let mut valid = hex_chars.len() == 64;
                                        if valid {
                                            for i in 0..32 {
                                                let hi = match hex_chars[i * 2] {
                                                    b'0'..=b'9' => hex_chars[i * 2] - b'0',
                                                    b'a'..=b'f' => hex_chars[i * 2] - b'a' + 10,
                                                    b'A'..=b'F' => hex_chars[i * 2] - b'A' + 10,
                                                    _ => { valid = false; break; }
                                                };
                                                let lo = match hex_chars[i * 2 + 1] {
                                                    b'0'..=b'9' => hex_chars[i * 2 + 1] - b'0',
                                                    b'a'..=b'f' => hex_chars[i * 2 + 1] - b'a' + 10,
                                                    b'A'..=b'F' => hex_chars[i * 2 + 1] - b'A' + 10,
                                                    _ => { valid = false; break; }
                                                };
                                                tx_hash[i] = (hi << 4) | lo;
                                            }
                                        }
                                        if valid && utxo.spendable {
                                            wallet_inner.add_utxo(wallet::wallet::UTXO {
                                                tx_hash,
                                                output_index: utxo.vout,
                                                amount: utxo.amount,
                                                address: utxo.address.clone(),
                                            });
                                            total_balance += utxo.amount;
                                        }
                                    }
                                    wallet_inner.set_balance(total_balance);

                                    tx_result = wm.create_transaction(&to, amount, fee);
                                    if tx_result.is_ok() {
                                        break;
                                    }

                                    // Only retry on InsufficientFunds when masternode confirms we have enough
                                    if let Err(ref msg) = tx_result {
                                        if msg.contains("Insufficient funds") && attempt < max_retries - 1 {
                                            let mn_total = client.get_balances(&state.addresses).await
                                                .map(|b| b.total).unwrap_or(0);
                                            if mn_total >= total_needed {
                                                log::info!(
                                                    "UTXOs temporarily locked, waiting for finalization (attempt {}/{})",
                                                    attempt + 1, max_retries
                                                );
                                                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                                                continue;
                                            }
                                        }
                                    }
                                    break;
                                }

                                match tx_result {
                                    Ok(mut tx) => {
                                        // Re-sign ALL inputs with correct BIP-44 HD keypairs.
                                        // create_transaction signs with Wallet.keypair which uses
                                        // m/44'/0'/0' (account-level), but addresses are derived at
                                        // m/44'/0'/0'/0/index (full BIP-44). We must re-sign every input.
                                        let addr_to_index: std::collections::HashMap<String, u32> =
                                            (0..wm.get_address_count())
                                                .filter_map(|i| wm.derive_address(i).ok().map(|a| (a, i)))
                                                .collect();

                                        let mut resignings: Vec<(usize, u32)> = Vec::new();
                                        for (input_idx, input) in tx.inputs.iter().enumerate() {
                                            let input_txid: String = input.previous_output.txid.iter().map(|b| format!("{:02x}", b)).collect();
                                            let input_vout = input.previous_output.vout;
                                            if let Some(utxo) = all_utxos.iter().find(|u| u.txid == input_txid && u.vout == input_vout) {
                                                if let Some(&hd_index) = addr_to_index.get(&utxo.address) {
                                                    resignings.push((input_idx, hd_index));
                                                }
                                            }
                                        }
                                        for (input_idx, hd_index) in resignings {
                                            if let Ok(kp) = wm.derive_keypair(hd_index) {
                                                log::info!("Signing input {} with HD key index {}", input_idx, hd_index);
                                                let _ = tx.sign(&kp, input_idx);
                                            }
                                        }

                                        let actual_fee = wallet::calculate_fee(amount);
                                        // Serialize to bincode bytes then hex-encode for sendrawtransaction
                                        match tx.to_bytes() {
                                            Ok(bytes) => {
                                                let tx_hex: String = bytes.iter().map(|b| format!("{:02x}", b)).collect();
                                                match client.broadcast_transaction(&tx_hex).await {
                                                    Ok(txid) => {
                                                        let _ = state.svc_tx.send(ServiceEvent::TransactionSent { txid: txid.clone() });
                                                        let now = std::time::SystemTime::now()
                                                            .duration_since(std::time::UNIX_EPOCH)
                                                            .map(|d| d.as_secs() as i64)
                                                            .unwrap_or(0);
                                                        let sent_record = crate::masternode_client::TransactionRecord {
                                                            txid: txid.clone(),
                                                            vout: 0,
                                                            is_send: true,
                                                            address: to.clone(),
                                                            amount,
                                                            fee: actual_fee,
                                                            timestamp: now,
                                                            status: crate::masternode_client::TransactionStatus::Pending,
                                                            is_fee: false,
                                                            is_change: false,
                                                        };
                                                        let _ = state.svc_tx.send(ServiceEvent::TransactionInserted(sent_record.clone()));
                                                        // Persist send record so correct amount survives restarts
                                                        if let Some(ref db) = state.wallet_db {
                                                            let _ = db.save_send_record(&sent_record);
                                                        }
                                                        // Insert fee as a separate ledger entry
                                                        if actual_fee > 0 {
                                                            let fee_record = crate::masternode_client::TransactionRecord {
                                                                txid: txid.clone(),
                                                                vout: 0,
                                                                is_send: true,
                                                                address: "Network Fee".to_string(),
                                                                amount: actual_fee,
                                                                fee: 0,
                                                                timestamp: now,
                                                                status: crate::masternode_client::TransactionStatus::Pending,
                                                                is_fee: true,
                                                                is_change: false,
                                                            };
                                                            let _ = state.svc_tx.send(ServiceEvent::TransactionInserted(fee_record));
                                                        }
                                                        // Self-send: also insert a pending receive entry immediately
                                                        let is_self_send = state.addresses.contains(&to);
                                                        if is_self_send {
                                                            let recv_record = crate::masternode_client::TransactionRecord {
                                                                txid: txid.clone(),
                                                                vout: 0,
                                                                is_send: false,
                                                                address: to.clone(),
                                                                amount,
                                                                fee: 0,
                                                                timestamp: now,
                                                                status: crate::masternode_client::TransactionStatus::Pending,
                                                                is_fee: false,
                                                                is_change: false,
                                                            };
                                                            let _ = state.svc_tx.send(ServiceEvent::TransactionInserted(recv_record));
                                                        }
                                                        if !state.addresses.is_empty() {
                                                            if let Ok(balance) = client.get_balances(&state.addresses).await {
                                                                let _ = state.svc_tx.send(ServiceEvent::BalanceUpdated(balance));
                                                            }
                                                        }
                                                    }
                                                    Err(e) => {
                                                        let _ = state.svc_tx.send(ServiceEvent::Error(
                                                            format!("Broadcast failed: {}", e),
                                                        ));
                                                    }
                                                }
                                            }
                                            Err(e) => {
                                                let _ = state.svc_tx.send(ServiceEvent::Error(
                                                    format!("Failed to serialize transaction: {}", e),
                                                ));
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        let _ = state.svc_tx.send(ServiceEvent::Error(
                                            format!("Failed to create transaction: {}", e),
                                        ));
                                    }
                                }
                            } else {
                                let _ = state.svc_tx.send(ServiceEvent::Error("No wallet loaded".to_string()));
                            }
                        } else {
                            let _ = state.svc_tx.send(ServiceEvent::Error("Not connected to any peer".to_string()));
                        }
                    }

                    UiEvent::NavigatedTo(screen) => {
                        if let Some(ref client) = state.client {
                            if !state.addresses.is_empty() {
                                match screen {
                                    Screen::Overview => {
                                        if let Ok(bal) = client.get_balances(&state.addresses).await {
                                            let _ = state.svc_tx.send(ServiceEvent::BalanceUpdated(bal));
                                        }
                                    }
                                    Screen::Transactions => {
                                        if let Ok(txs) = client.get_transactions_multi(&state.addresses, 0).await {
                                            let _ = state.svc_tx.send(ServiceEvent::TransactionsUpdated(txs));
                                        }
                                    }
                                    Screen::Utxos => {
                                        let mut all = Vec::new();
                                        for addr in &state.addresses {
                                            if let Ok(utxos) = client.get_utxos(addr).await {
                                                all.extend(utxos);
                                            }
                                        }
                                        state.send_utxos_updated(all);
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }

                    UiEvent::CheckHealth => {
                        if let Some(ref client) = state.client {
                            match client.health_check().await {
                                Ok(health) => { let _ = state.svc_tx.send(ServiceEvent::HealthUpdated(health)); }
                                Err(e) => { let _ = state.svc_tx.send(ServiceEvent::Error(e.to_string())); }
                            }
                        }
                    }

                    UiEvent::SwitchNetwork { network: _ } => {
                        let _ = state.svc_tx.send(ServiceEvent::Error(
                            "Network switch requires restart".to_string(),
                        ));
                    }

                    UiEvent::SelectNetwork { network } => {
                        let selected_testnet = network == "testnet";
                        state.config.network = network;
                        state.network_type = if selected_testnet {
                            NetworkType::Testnet
                        } else {
                            NetworkType::Mainnet
                        };
                        // Save config now that user has chosen
                        if let Err(e) = state.config.save() {
                            log::error!("Failed to save config: {}", e);
                        }
                        // Reopen wallet_db at the correct path
                        let db_path = state.config.wallet_dir().join("wallet_db");
                        if let Some(parent) = db_path.parent() {
                            let _ = std::fs::create_dir_all(parent);
                        }
                        state.wallet_db = WalletDb::open(&db_path).ok();
                        if state.wallet_db.is_some() {
                            log::info!("📂 Wallet database reopened at: {}", db_path.display());
                        }
                        // Check if a wallet already exists for this network
                        let exists = WalletManager::exists(state.network_type);
                        let _ = state.svc_tx.send(ServiceEvent::WalletExists(exists));
                        let _ = state.svc_tx.send(ServiceEvent::NetworkConfigured { is_testnet: selected_testnet });

                        // Re-trigger peer discovery with the correct network
                        is_testnet = selected_testnet;
                        manual_endpoints = state.config.manual_endpoints();
                        rpc_credentials = state.config.rpc_credentials();
                        state.client = None;
                        let tx = state.svc_tx.clone();
                        let eps = manual_endpoints.clone();
                        let tn = is_testnet;
                        let creds = rpc_credentials.clone();
                        let max_conn = state.config.max_connections;
                        discovery_handle = Some(tokio::spawn(async move {
                            discover_peers(tn, eps, creds, &tx, max_conn).await
                        }));
                    }

                    UiEvent::UpdateAddressLabel { index, label } => {
                        if let Some(addr) = state.addresses.get(index) {
                            if let Some(ref db) = state.wallet_db {
                                let now = chrono::Utc::now().timestamp();
                                let mut contact = db
                                    .get_contact(addr)
                                    .ok()
                                    .flatten()
                                    .unwrap_or_else(|| AddressContact {
                                        address: addr.clone(),
                                        label: String::new(),
                                        name: None,
                                        email: None,
                                        phone: None,
                                        notes: None,
                                        is_default: index == 0,
                                        is_owned: true,
                                        derivation_index: Some(index as u32),
                                        created_at: now,
                                        updated_at: now,
                                    });
                                contact.label = label;
                                contact.updated_at = now;
                                if let Err(e) = db.save_contact(&contact) {
                                    log::warn!("Failed to save address label: {}", e);
                                }
                            }
                        }
                    }

                    UiEvent::GenerateAddress => {
                        if let Some(ref mut wm) = state.wallet {
                            match wm.get_next_address() {
                                Ok(addr) => {
                                    let index = state.addresses.len();
                                    state.addresses.push(addr.clone());
                                    let label = format!("Address #{}", index);
                                    if let Some(ref db) = state.wallet_db {
                                        let now = chrono::Utc::now().timestamp();
                                        let contact = AddressContact {
                                            address: addr.clone(),
                                            label: label.clone(),
                                            name: None,
                                            email: None,
                                            phone: None,
                                            notes: None,
                                            is_default: false,
                                            is_owned: true,
                                            derivation_index: Some(index as u32),
                                            created_at: now,
                                            updated_at: now,
                                        };
                                        let _ = db.save_contact(&contact);
                                    }
                                    let _ = state.svc_tx.send(ServiceEvent::AddressGenerated(
                                        AddressInfo { address: addr, label },
                                    ));
                                    // Re-subscribe WS with updated address list
                                    if state.config.active_endpoint.is_some() {
                                        state.start_ws();
                                    }
                                }
                                Err(e) => {
                                    let _ = state.svc_tx.send(ServiceEvent::Error(
                                        format!("Failed to generate address: {}", e),
                                    ));
                                }
                            }
                        } else {
                            let _ = state.svc_tx.send(ServiceEvent::Error(
                                "No wallet loaded".to_string(),
                            ));
                        }
                    }

                    UiEvent::SaveContact { name, address } => {
                        if let Some(ref db) = state.wallet_db {
                            let contact = crate::wallet_db::AddressContact {
                                address: address.clone(),
                                label: name.clone(),
                                name: Some(name),
                                email: None,
                                phone: None,
                                notes: None,
                                is_default: false,
                                is_owned: false,
                                derivation_index: None,
                                created_at: chrono::Utc::now().timestamp(),
                                updated_at: chrono::Utc::now().timestamp(),
                            };
                            if let Err(e) = db.save_contact(&contact) {
                                log::warn!("Failed to save contact: {}", e);
                            }
                            // Reload contacts list
                            if let Ok(contacts) = db.get_external_contacts() {
                                let infos: Vec<crate::state::ContactInfo> = contacts
                                    .into_iter()
                                    .map(|c| crate::state::ContactInfo {
                                        name: c.name.unwrap_or(c.label),
                                        address: c.address,
                                    })
                                    .collect();
                                let _ = state.svc_tx.send(ServiceEvent::ContactsUpdated(infos));
                            }
                        }
                    }

                    UiEvent::DeleteContact { address } => {
                        if let Some(ref db) = state.wallet_db {
                            if let Err(e) = db.delete_contact(&address) {
                                log::warn!("Failed to delete contact: {}", e);
                            }
                            if let Ok(contacts) = db.get_external_contacts() {
                                let infos: Vec<crate::state::ContactInfo> = contacts
                                    .into_iter()
                                    .map(|c| crate::state::ContactInfo {
                                        name: c.name.unwrap_or(c.label),
                                        address: c.address,
                                    })
                                    .collect();
                                let _ = state.svc_tx.send(ServiceEvent::ContactsUpdated(infos));
                            }
                        }
                    }

                    UiEvent::UpdateDecimalPlaces(dp) => {
                        if let Some(ref db) = state.wallet_db {
                            if let Err(e) = db.save_setting("decimal_places", &dp.to_string()) {
                                log::warn!("Failed to save decimal_places: {}", e);
                            }
                        }
                        let _ = state.svc_tx.send(ServiceEvent::DecimalPlacesLoaded(dp));
                    }

                    UiEvent::ResyncWallet => {
                        log::info!("🔄 Resync requested — clearing cached data");
                        if let Some(ref db) = state.wallet_db {
                            if let Err(e) = db.clear_all_utxos() {
                                log::warn!("Failed to clear UTXOs: {}", e);
                            }
                            let _ = db.save_cached_transactions(&[]);
                            let _ = db.save_cached_balance(&crate::masternode_client::Balance {
                                confirmed: 0,
                                pending: 0,
                                total: 0,
                            });
                        }

                        // Re-fetch everything from masternode before updating UI
                        if let Some(ref client) = state.client {
                            if !state.addresses.is_empty() {
                                match client.get_transactions_multi(&state.addresses, 0).await {
                                    Ok(txs) => {
                                        if let Some(ref db) = state.wallet_db {
                                            let _ = db.save_cached_transactions(&txs);
                                        }
                                        let _ = state
                                            .svc_tx
                                            .send(ServiceEvent::TransactionsUpdated(txs));
                                    }
                                    Err(e) => {
                                        log::error!("Resync fetch failed: {}", e);
                                        let _ = state.svc_tx.send(ServiceEvent::Error(
                                            format!("Resync failed: {}", e),
                                        ));
                                    }
                                }
                                match client.get_balances(&state.addresses).await {
                                    Ok(bal) => {
                                        if let Some(ref db) = state.wallet_db {
                                            let _ = db.save_cached_balance(&bal);
                                        }
                                        let _ = state.svc_tx.send(ServiceEvent::BalanceUpdated(bal));
                                    }
                                    Err(e) => log::warn!("Resync balance fetch failed: {}", e),
                                }
                                let mut all_utxos = Vec::new();
                                for addr in &state.addresses {
                                    if let Ok(utxos) = client.get_utxos(addr).await {
                                        all_utxos.extend(utxos);
                                    }
                                }
                                state.send_utxos_updated(all_utxos);
                            }
                        }
                        let _ = state.svc_tx.send(ServiceEvent::ResyncComplete);
                    }

                    UiEvent::RepairDatabase => {
                        log::info!("🔧 Database repair requested");
                        let db_path = state.config.wallet_dir().join("wallet_db");
                        let backup_path = state.config.wallet_dir().join(format!(
                            "wallet_db_backup_{}",
                            chrono::Local::now().format("%Y%m%d_%H%M%S")
                        ));

                        // Drop the current database handle
                        state.wallet_db = None;

                        // Back up the existing database directory
                        let mut backed_up = false;
                        if db_path.exists() {
                            match std::fs::rename(&db_path, &backup_path) {
                                Ok(()) => {
                                    log::info!("📦 Backed up corrupt database to {}", backup_path.display());
                                    backed_up = true;
                                }
                                Err(e) => {
                                    log::error!("Failed to back up database: {}", e);
                                    // Try removing instead
                                    if let Err(e2) = std::fs::remove_dir_all(&db_path) {
                                        log::error!("Failed to remove database: {}", e2);
                                        let _ = state.svc_tx.send(ServiceEvent::Error(
                                            format!("Failed to repair database: backup failed ({}), removal failed ({})", e, e2),
                                        ));
                                        let _ = state.svc_tx.send(ServiceEvent::DatabaseRepaired {
                                            message: "Repair failed — could not move or delete database".to_string(),
                                        });
                                        continue;
                                    }
                                }
                            }
                        }

                        // Reopen a fresh database
                        if let Some(parent) = db_path.parent() {
                            let _ = std::fs::create_dir_all(parent);
                        }
                        state.wallet_db = WalletDb::open(&db_path).ok();

                        if state.wallet_db.is_none() {
                            let _ = state.svc_tx.send(ServiceEvent::Error(
                                "Failed to create new database after repair".to_string(),
                            ));
                            let _ = state.svc_tx.send(ServiceEvent::DatabaseRepaired {
                                message: "Repair failed — could not create new database".to_string(),
                            });
                            continue;
                        }

                        log::info!("✅ Fresh database created");

                        // Re-persist owned addresses
                        if let Some(ref db) = state.wallet_db {
                            if state.wallet.is_some() {
                                for (i, addr) in state.addresses.iter().enumerate() {
                                    let contact = AddressContact {
                                        address: addr.clone(),
                                        label: format!("Address {}", i + 1),
                                        name: None,
                                        email: None,
                                        phone: None,
                                        notes: None,
                                        is_default: i == 0,
                                        is_owned: true,
                                        derivation_index: Some(i as u32),
                                        created_at: chrono::Utc::now().timestamp(),
                                        updated_at: chrono::Utc::now().timestamp(),
                                    };
                                    let _ = db.save_contact(&contact);
                                }
                            }
                        }

                        // Re-fetch all data from masternodes
                        if let Some(ref client) = state.client {
                            if !state.addresses.is_empty() {
                                match client.get_transactions_multi(&state.addresses, 0).await {
                                    Ok(txs) => {
                                        if let Some(ref db) = state.wallet_db {
                                            let _ = db.save_cached_transactions(&txs);
                                        }
                                        let _ = state.svc_tx.send(ServiceEvent::TransactionsUpdated(txs));
                                    }
                                    Err(e) => log::warn!("Repair: failed to fetch transactions: {}", e),
                                }
                                match client.get_balances(&state.addresses).await {
                                    Ok(bal) => {
                                        if let Some(ref db) = state.wallet_db {
                                            let _ = db.save_cached_balance(&bal);
                                        }
                                        let _ = state.svc_tx.send(ServiceEvent::BalanceUpdated(bal));
                                    }
                                    Err(e) => log::warn!("Repair: failed to fetch balance: {}", e),
                                }
                                let mut all_utxos = Vec::new();
                                for addr in &state.addresses {
                                    if let Ok(utxos) = client.get_utxos(addr).await {
                                        all_utxos.extend(utxos);
                                    }
                                }
                                state.send_utxos_updated(all_utxos);
                            }
                        }

                        let msg = if backed_up {
                            format!("Database repaired. Backup saved to {}", backup_path.display())
                        } else {
                            "Database repaired. Fresh database created.".to_string()
                        };
                        let _ = state.svc_tx.send(ServiceEvent::DatabaseRepaired { message: msg });
                    }

                    UiEvent::ConsolidateUtxos => {
                        log::info!("🔄 UTXO consolidation requested");
                        if let (Some(ref client), Some(ref wm)) = (&state.client, &state.wallet) {
                            // Pre-extract everything the background task needs so we don't
                            // block the service loop during consolidation.
                            let addr_count = wm.get_address_count();
                            let addr_to_keypair: std::collections::HashMap<String, wallet::Keypair> =
                                (0..addr_count)
                                    .filter_map(|i| {
                                        let addr = wm.derive_address(i).ok()?;
                                        let kp = wm.derive_keypair(i).ok()?;
                                        Some((addr, kp))
                                    })
                                    .collect();
                            let client_clone = client.clone();
                            let svc_tx_clone = state.svc_tx.clone();
                            let addresses = state.addresses.clone();
                            let dest_addr = state.addresses.first().cloned().unwrap_or_default();

                            tokio::spawn(async move {
                                consolidate_utxos_background(
                                    client_clone,
                                    svc_tx_clone,
                                    addresses,
                                    dest_addr,
                                    addr_to_keypair,
                                )
                                .await;
                            });
                        } else {
                            let _ = state.svc_tx.send(ServiceEvent::Error(
                                "Cannot consolidate: no masternode connection or wallet.".to_string(),
                            ));
                        }
                    }

                    UiEvent::OpenConfigFile { path } => {
                        log::info!("Opening config file: {}", path.display());
                        // Create file with template if it doesn't exist
                        if !path.exists() {
                            if let Some(parent) = path.parent() {
                                let _ = std::fs::create_dir_all(parent);
                            }
                            let template = config_file_template(&path);
                            if let Err(e) = std::fs::write(&path, template) {
                                log::error!("Failed to create {}: {}", path.display(), e);
                                let _ = state.svc_tx.send(ServiceEvent::Error(
                                    format!("Failed to create {}: {}", path.display(), e),
                                ));
                                continue;
                            }
                            log::info!("Created {}", path.display());
                        }
                        let editor = state.config.editor.clone();
                        let svc_tx = state.svc_tx.clone();
                        tokio::task::spawn_blocking(move || {
                            let result = if let Some(ref ed) = editor {
                                std::process::Command::new(ed).arg(&path).spawn().map(|_| ())
                            } else {
                                open::that(&path)
                            };
                            if let Err(e) = result {
                                log::error!("Failed to open editor: {}", e);
                                let _ = svc_tx.send(ServiceEvent::Error(
                                    format!("Failed to open editor: {}", e),
                                ));
                            }
                        });
                    }

                    UiEvent::EncryptWallet { password } => {
                        if let Some(ref mut wm) = state.wallet {
                            match wm.encrypt_wallet(&password) {
                                Ok(()) => {
                                    log::info!("✅ Wallet encrypted successfully");
                                    let _ = state.svc_tx.send(ServiceEvent::WalletEncrypted);
                                }
                                Err(e) => {
                                    let _ = state.svc_tx.send(ServiceEvent::Error(
                                        format!("Failed to encrypt wallet: {}", e),
                                    ));
                                }
                            }
                        } else {
                            let _ = state.svc_tx.send(ServiceEvent::Error(
                                "No wallet loaded".to_string(),
                            ));
                        }
                    }

                    UiEvent::SetEditor { editor } => {
                        state.config.editor = editor;
                        if let Err(e) = state.config.save() {
                            log::error!("Failed to save config: {}", e);
                        }
                    }

                    UiEvent::SetMaxConnections(n) => {
                        state.config.max_connections = n;
                        if let Err(e) = state.config.save() {
                            log::error!("Failed to save config: {}", e);
                        }
                        let _ = state.svc_tx.send(ServiceEvent::MaxConnectionsUpdated(n));
                    }

                    UiEvent::PersistSendRecords(records) => {
                        if let Some(ref db) = state.wallet_db {
                            for record in &records {
                                let _ = db.save_send_record(record);
                            }
                        }
                    }

                    UiEvent::SaveMasternodeEntry(entry) => {
                        if let Some(ref db) = state.wallet_db {
                            match db.save_masternode_entry(&entry) {
                                Ok(()) => {
                                    log::info!("💾 Saved masternode entry '{}'", entry.alias);
                                    if let Ok(entries) = db.get_masternode_entries() {
                                        let _ = state.svc_tx.send(ServiceEvent::MasternodeEntriesLoaded(entries));
                                    }
                                }
                                Err(e) => {
                                    let _ = state.svc_tx.send(ServiceEvent::Error(
                                        format!("Failed to save masternode '{}': {}", entry.alias, e),
                                    ));
                                }
                            }
                        } else {
                            let _ = state.svc_tx.send(ServiceEvent::Error(
                                "Cannot save masternode: wallet database not available".to_string(),
                            ));
                        }
                    }

                    UiEvent::DeleteMasternodeEntry { alias } => {
                        if let Some(ref db) = state.wallet_db {
                            let _ = db.delete_masternode_entry(&alias);
                            if let Ok(entries) = db.get_masternode_entries() {
                                let _ = state.svc_tx.send(ServiceEvent::MasternodeEntriesLoaded(entries));
                            }
                        }
                    }

                    UiEvent::UpdateMasternodeEntry { old_alias, new_entry } => {
                        if let Some(ref db) = state.wallet_db {
                            let _ = db.delete_masternode_entry(&old_alias);
                            match db.save_masternode_entry(&new_entry) {
                                Ok(()) => {
                                    log::info!("Updated masternode '{}' -> '{}'", old_alias, new_entry.alias);
                                    if let Ok(entries) = db.get_masternode_entries() {
                                        let _ = state.svc_tx.send(ServiceEvent::MasternodeEntriesLoaded(entries));
                                    }
                                }
                                Err(e) => {
                                    let _ = state.svc_tx.send(ServiceEvent::Error(
                                        format!("Failed to update masternode '{}': {}", old_alias, e),
                                    ));
                                }
                            }
                        }
                    }

                    UiEvent::ImportMasternodeConf { path } => {
                        if let Some(ref db) = state.wallet_db {
                            match std::fs::read_to_string(&path) {
                                Ok(contents) => {
                                    let mut count = 0;
                                    for line in contents.lines() {
                                        if let Some(entry) = crate::wallet_db::MasternodeEntry::parse_conf_line(line) {
                                            let _ = db.save_masternode_entry(&entry);
                                            count += 1;
                                        }
                                    }
                                    log::info!("Imported {} masternode entries from {}", count, path.display());
                                    if let Ok(entries) = db.get_masternode_entries() {
                                        let _ = state.svc_tx.send(ServiceEvent::MasternodeEntriesLoaded(entries));
                                    }
                                }
                                Err(e) => {
                                    let _ = state.svc_tx.send(ServiceEvent::Error(
                                        format!("Failed to read {}: {}", path.display(), e),
                                    ));
                                }
                            }
                        }
                    }

                    UiEvent::RegisterMasternode {
                        alias,
                        ip,
                        port,
                        collateral_txid,
                        collateral_vout,
                        payout_address,
                    } => {
                        if let (Some(ref client), Some(ref mut wm)) =
                            (&state.client, &mut state.wallet)
                        {
                            match build_masternode_reg_tx(
                                wm,
                                client,
                                &state.addresses,
                                &collateral_txid,
                                collateral_vout,
                                &ip,
                                port,
                                &payout_address,
                            )
                            .await
                            {
                                Ok((tx_hex, txid)) => {
                                    match client.broadcast_transaction(&tx_hex).await {
                                        Ok(broadcast_txid) => {
                                            let final_txid = if broadcast_txid.is_empty() {
                                                txid
                                            } else {
                                                broadcast_txid
                                            };
                                            // Lock the collateral UTXO
                                            if let Some(ref db) = state.wallet_db {
                                                let _ = db.lock_collateral(
                                                    &collateral_txid,
                                                    collateral_vout,
                                                    &alias,
                                                );
                                            }
                                            let _ = state.svc_tx.send(
                                                ServiceEvent::MasternodeRegistered {
                                                    alias,
                                                    txid: final_txid,
                                                },
                                            );
                                        }
                                        Err(e) => {
                                            let _ = state.svc_tx.send(ServiceEvent::Error(
                                                format!("Failed to broadcast MN registration: {}", e),
                                            ));
                                        }
                                    }
                                }
                                Err(e) => {
                                    let _ = state.svc_tx.send(ServiceEvent::Error(
                                        format!("Failed to build MN registration tx: {}", e),
                                    ));
                                }
                            }
                        }
                    }

                    UiEvent::UpdateMasternodePayout {
                        masternode_id,
                        new_payout_address,
                    } => {
                        if let (Some(ref client), Some(ref mut wm)) =
                            (&state.client, &mut state.wallet)
                        {
                            match build_masternode_update_tx(
                                wm,
                                client,
                                &state.addresses,
                                &masternode_id,
                                &new_payout_address,
                            )
                            .await
                            {
                                Ok((tx_hex, txid)) => {
                                    match client.broadcast_transaction(&tx_hex).await {
                                        Ok(broadcast_txid) => {
                                            let final_txid = if broadcast_txid.is_empty() {
                                                txid
                                            } else {
                                                broadcast_txid
                                            };
                                            let _ = state.svc_tx.send(
                                                ServiceEvent::MasternodePayoutUpdated {
                                                    masternode_id,
                                                    txid: final_txid,
                                                },
                                            );
                                        }
                                        Err(e) => {
                                            let _ = state.svc_tx.send(ServiceEvent::Error(
                                                format!(
                                                    "Failed to broadcast payout update: {}",
                                                    e
                                                ),
                                            ));
                                        }
                                    }
                                }
                                Err(e) => {
                                    let _ = state.svc_tx.send(ServiceEvent::Error(
                                        format!("Failed to build payout update tx: {}", e),
                                    ));
                                }
                            }
                        }
                    }
                }
            }

            Some(ws_event) = ws_event_rx.recv() => {
                match ws_event {
                    WsEvent::TransactionReceived(notification) => {
                        let amount_sats = crate::masternode_client::json_to_satoshis(&notification.amount);

                        // Determine if this is a change output vs a real receive
                        let send_record = state.wallet_db.as_ref()
                            .and_then(|db| db.get_send_records().ok())
                            .and_then(|recs| recs.get(&notification.txid).cloned());
                        let is_own_addr = state.addresses.contains(&notification.address);
                        let is_change = if let Some(ref sr) = send_record {
                            // It's from a txid we sent — change unless it's send-to-self receive
                            let is_self_send = state.addresses.contains(&sr.address);
                            if is_self_send && is_own_addr && amount_sats == sr.amount {
                                false // actual send-to-self receive, keep it
                            } else {
                                is_own_addr // other receives to own addr are change
                            }
                        } else {
                            false // not a txid we sent, it's a real receive
                        };

                        if !is_change {
                            let tx_record = TransactionRecord {
                                txid: notification.txid.clone(),
                                vout: 0,
                                is_send: false,
                                address: notification.address.clone(),
                                amount: amount_sats,
                                fee: 0,
                                timestamp: notification.timestamp,
                                status: TransactionStatus::Pending,
                                is_fee: false,
                                is_change: false,
                            };
                            let _ = state.svc_tx.send(ServiceEvent::TransactionInserted(tx_record));
                            let _ = state.svc_tx.send(ServiceEvent::TransactionReceived(notification.clone()));
                        }

                        // Refresh balance and transactions immediately
                        if let Some(ref client) = state.client {
                            if !state.addresses.is_empty() {
                                match client.get_balances(&state.addresses).await {
                                    Ok(bal) => { let _ = state.svc_tx.send(ServiceEvent::BalanceUpdated(bal)); }
                                    Err(e) => log::warn!("Failed to refresh balance after receive: {}", e),
                                }
                                // Refresh transactions so instant-finality status is reflected immediately
                                if let Ok(txs) = client.get_transactions_multi(&state.addresses, 0).await {
                                    let _ = state.svc_tx.send(ServiceEvent::TransactionsUpdated(txs));
                                }
                            }
                        }
                    }
                    WsEvent::UtxoFinalized(notif) => {
                        // UTXO finalized by masternode consensus — mark tx as Approved
                        let amount_sats = crate::masternode_client::json_to_satoshis(&notif.amount);
                        log::info!("✅ UTXO finalized: txid={}... vout={} amount={}", &notif.txid[..16.min(notif.txid.len())], notif.output_index, amount_sats);

                        // Determine if this is a change output
                        let send_record = state.wallet_db.as_ref()
                            .and_then(|db| db.get_send_records().ok())
                            .and_then(|recs| recs.get(&notif.txid).cloned());
                        let is_own_addr = state.addresses.contains(&notif.address);
                        let is_change = if let Some(ref sr) = send_record {
                            let is_self_send = state.addresses.contains(&sr.address);
                            if is_self_send && is_own_addr && amount_sats == sr.amount {
                                false // send-to-self receive
                            } else {
                                is_own_addr
                            }
                        } else {
                            false
                        };

                        if !is_change {
                            let tx_record = TransactionRecord {
                                txid: notif.txid.clone(),
                                vout: notif.output_index,
                                is_send: false,
                                address: notif.address.clone(),
                                amount: amount_sats,
                                fee: 0,
                                timestamp: chrono::Utc::now().timestamp(),
                                status: TransactionStatus::Approved,
                                is_fee: false,
                                is_change: false,
                            };
                            let _ = state.svc_tx.send(ServiceEvent::TransactionInserted(tx_record));
                        }

                        let _ = state.svc_tx.send(ServiceEvent::TransactionFinalityUpdated {
                            txid: notif.txid,
                            finalized: true,
                        });

                        // Refresh balance, transactions, and UTXOs after finalization
                        if let Some(ref client) = state.client {
                            if !state.addresses.is_empty() {
                                match client.get_balances(&state.addresses).await {
                                    Ok(bal) => {
                                        log::info!("🔍 Post-finalization balance: total={} available={}", bal.total, bal.confirmed);
                                        let _ = state.svc_tx.send(ServiceEvent::BalanceUpdated(bal));
                                    }
                                    Err(e) => log::warn!("Failed to refresh balance after finalization: {}", e),
                                }
                                if let Ok(txs) = client.get_transactions_multi(&state.addresses, 0).await {
                                    let _ = state.svc_tx.send(ServiceEvent::TransactionsUpdated(txs));
                                }
                                let mut all_utxos = Vec::new();
                                for addr in &state.addresses {
                                    if let Ok(utxos) = client.get_utxos(addr).await {
                                        all_utxos.extend(utxos);
                                    }
                                }
                                let utxo_sum: u64 = all_utxos.iter().map(|u| u.amount).sum();
                                log::info!("🔍 Post-finalization UTXOs: count={} total={}", all_utxos.len(), utxo_sum);
                                state.send_utxos_updated(all_utxos);
                            }
                        }
                    }
                    WsEvent::Connected(_) => {
                        let _ = state.svc_tx.send(ServiceEvent::WsConnected);
                    }
                    WsEvent::Disconnected(_) => {
                        let _ = state.svc_tx.send(ServiceEvent::WsDisconnected);
                    }
                    WsEvent::CapacityFull(url) => {
                        log::warn!("⚠️ Masternode at capacity: {}. Attempting failover…", url);
                        let _ = state.svc_tx.send(ServiceEvent::WsCapacityFull(url.clone()));
                        // The WS URL is derived from the RPC endpoint (port+1), so reverse to find
                        // the capacity-full endpoint and pick the next healthy one.
                        let current_endpoint = state.config.active_endpoint.clone();
                        let next = state.last_peers.iter().find(|p| {
                            p.is_healthy && Some(&p.endpoint) != current_endpoint.as_ref()
                        }).cloned();
                        if let Some(peer) = next {
                            log::info!("🔀 Failing over to {}", peer.endpoint);
                            state.client = Some(MasternodeClient::new(peer.endpoint.clone(), rpc_credentials.clone()));
                            state.config.active_endpoint = Some(peer.endpoint.clone());
                            config.active_endpoint = Some(peer.endpoint);
                            if !state.addresses.is_empty() {
                                state.start_ws();
                            }
                        } else {
                            log::warn!("⚠️ No healthy fallback peer available for WS failover");
                        }
                    }
                    WsEvent::TransactionRejected(notif) => {
                        log::warn!("❌ Transaction {} rejected: {}", &notif.txid[..16.min(notif.txid.len())], notif.reason);
                        let _ = state.svc_tx.send(ServiceEvent::TransactionFinalityUpdated {
                            txid: notif.txid,
                            finalized: false,
                        });
                    }
                }
            }
        }
    }

    log::info!("👋 Service loop exited");
}

/// Discover and health-check peers in the background.
/// Returns the best endpoint and the full peer info list.
async fn discover_peers(
    is_testnet: bool,
    manual_endpoints: Vec<String>,
    rpc_credentials: Option<(String, String)>,
    svc_tx: &mpsc::UnboundedSender<ServiceEvent>,
    max_connections: usize,
) -> Result<(String, Vec<PeerInfo>), ()> {
    let rpc_port = if is_testnet { 24101 } else { 24001 };
    let mut endpoints = manual_endpoints;
    match peer_discovery::fetch_peers(is_testnet).await {
        Ok(api_peers) => {
            log::info!("🌐 API returned {} peers", api_peers.len());
            endpoints.extend(api_peers);
        }
        Err(e) => {
            log::warn!("⚠ Peer discovery failed: {}", e);
        }
    }

    endpoints.sort();
    endpoints.dedup();

    if endpoints.is_empty() {
        let _ = svc_tx.send(ServiceEvent::Error(
            "No peers available. Add peers to time.conf or check your internet connection."
                .to_string(),
        ));
        return Err(());
    }

    // Probe all peers in parallel with a short timeout
    let probe_timeout = std::time::Duration::from_secs(8);
    let mut handles = Vec::new();
    for endpoint in endpoints.clone() {
        let creds = rpc_credentials.clone();
        handles.push(tokio::spawn(async move {
            // Always probe the https:// form first; plain-http form is the fallback.
            let https_ep = if endpoint.starts_with("http://") {
                endpoint.replacen("http://", "https://", 1)
            } else {
                endpoint.clone()
            };
            let http_ep = https_ep.replacen("https://", "http://", 1);

            // TCP connect for accurate network ping (strips scheme)
            let tcp_addr = https_ep
                .strip_prefix("https://")
                .unwrap_or(&https_ep)
                .trim_end_matches('/');
            let tcp_start = Instant::now();
            let tcp_ok = tokio::time::timeout(
                std::time::Duration::from_secs(2),
                tokio::net::TcpStream::connect(tcp_addr),
            )
            .await
            .map(|r| r.is_ok())
            .unwrap_or(false);
            let ping_ms = if tcp_ok {
                Some(tcp_start.elapsed().as_millis() as u64)
            } else {
                None
            };

            let (is_healthy, block_height, version, working_ep) = if tcp_ok {
                // Try HTTPS first
                let client = MasternodeClient::new(https_ep.clone(), creds.clone());
                match tokio::time::timeout(probe_timeout, client.health_check()).await {
                    Ok(Ok(health)) => (true, Some(health.block_height), Some(health.version), https_ep.clone()),
                    _ => {
                        // HTTPS failed — retry with plain HTTP (masternode auto-detects)
                        log::debug!("HTTPS failed for {}, retrying with HTTP", https_ep);
                        let http_client = MasternodeClient::new(http_ep.clone(), creds.clone());
                        match tokio::time::timeout(probe_timeout, http_client.health_check()).await {
                            Ok(Ok(health)) => {
                                log::info!("✅ Peer {} reachable via HTTP (no TLS)", http_ep);
                                (true, Some(health.block_height), Some(health.version), http_ep.clone())
                            }
                            Ok(Err(e)) => {
                                log::warn!("⚠ Peer {} unhealthy: {}", http_ep, e);
                                (false, None, None, endpoint.clone())
                            }
                            Err(_) => {
                                log::warn!("⚠ Peer {} timed out", http_ep);
                                (false, None, None, endpoint.clone())
                            }
                        }
                    }
                }
            } else {
                (false, None, None, endpoint.clone())
            };

            // Probe WebSocket connectivity (WS port = RPC port + 1)
            let ws_available = if is_healthy {
                let ws_url = crate::config_new::Config::derive_ws_url(&working_ep);
                tokio::time::timeout(
                    std::time::Duration::from_secs(5),
                    tokio_tungstenite::connect_async_tls_with_config(
                        &ws_url,
                        None,
                        false,
                        Some(crate::ws_client::make_tls_connector()),
                    ),
                )
                .await
                .map(|r| r.is_ok())
                .unwrap_or(false)
            } else {
                false
            };

            PeerInfo {
                endpoint: working_ep,
                is_active: false,
                is_healthy,
                ws_available,
                ping_ms,
                block_height,
                version,
            }
        }));
    }

    let mut peer_infos = Vec::new();
    for handle in handles {
        if let Ok(info) = handle.await {
            peer_infos.push(info);
        }
    }

    // Sort: WS-capable first, then healthy by fastest ping, unhealthy last
    peer_infos.sort_by(|a, b| {
        b.is_healthy
            .cmp(&a.is_healthy)
            .then(b.ws_available.cmp(&a.ws_available))
            .then(
                a.ping_ms
                    .unwrap_or(u64::MAX)
                    .cmp(&b.ping_ms.unwrap_or(u64::MAX)),
            )
    });

    // Keep only healthy peers; cap if max_connections is set below usize::MAX
    peer_infos.retain(|p| p.is_healthy);
    if peer_infos.len() > max_connections {
        peer_infos.truncate(max_connections);
    }

    let active_endpoint = peer_infos
        .iter()
        .find(|p| p.is_healthy)
        .map(|p| p.endpoint.clone())
        .unwrap_or_else(|| {
            log::warn!("⚠ No peers responded to health check, using first peer");
            endpoints[0].clone()
        });

    // Gossip discovery: ask ALL healthy peers for their known masternodes.
    // Querying only one peer means a stale registry on that node hides others.
    // Normalise to "ip:rpc_port" for dedup — ignore http/https scheme differences.
    let known_hosts: std::collections::HashSet<String> = peer_infos
        .iter()
        .filter_map(|p| {
            p.endpoint
                .trim_start_matches("https://")
                .trim_start_matches("http://")
                .trim_end_matches('/')
                .split(':')
                .next()
                .map(|ip| format!("{}:{}", ip, rpc_port))
        })
        .collect();

    let gossip_endpoints: Vec<String> = peer_infos
        .iter()
        .filter(|p| p.is_healthy)
        .map(|p| p.endpoint.clone())
        .collect();

    let mut new_endpoints: std::collections::HashSet<String> = std::collections::HashSet::new();
    for ep in &gossip_endpoints {
        let client = MasternodeClient::new(ep.clone(), rpc_credentials.clone());
        if let Ok(gossip_peers) = client.get_peer_info().await {
            for gp in &gossip_peers {
                // Include both active and inactive peers — let the health probe decide.
                // A node that just restarted will show active=false in its neighbour's
                // peer list but may already be accepting connections.
                // addr format is "IP:P2P_PORT" — extract IP and use RPC port
                let ip = gp.addr.split(':').next().unwrap_or(&gp.addr);
                let host_key = format!("{}:{}", ip, rpc_port);
                if !known_hosts.contains(&host_key) {
                    new_endpoints.insert(format!("https://{}:{}", ip, rpc_port));
                }
            }
        }
    }

    if !new_endpoints.is_empty() {
        log::info!("🔗 Gossip discovery: found {} new peers", new_endpoints.len());
        // Probe new peers in parallel
        let probe_timeout2 = std::time::Duration::from_secs(8);
        let mut gossip_handles = Vec::new();
        for ep in new_endpoints {
            let creds = rpc_credentials.clone();
            gossip_handles.push(tokio::spawn(async move {
                // Gossip peers are built as https://; fall back to http:// like the initial probe.
                let http_ep = ep.replacen("https://", "http://", 1);

                // TCP connect for accurate network ping
                let tcp_addr = ep
                    .strip_prefix("https://")
                    .unwrap_or(&ep)
                    .trim_end_matches('/');
                let tcp_start = Instant::now();
                let tcp_ok = tokio::time::timeout(
                    std::time::Duration::from_secs(2),
                    tokio::net::TcpStream::connect(tcp_addr),
                )
                .await
                .map(|r| r.is_ok())
                .unwrap_or(false);
                let ping_ms = if tcp_ok {
                    Some(tcp_start.elapsed().as_millis() as u64)
                } else {
                    None
                };

                let (is_healthy, block_height, version, working_ep) = if tcp_ok {
                    let c = MasternodeClient::new(ep.clone(), creds.clone());
                    match tokio::time::timeout(probe_timeout2, c.health_check()).await {
                        Ok(Ok(health)) => (true, Some(health.block_height), Some(health.version), ep.clone()),
                        _ => {
                            log::debug!("HTTPS failed for gossip peer {}, retrying with HTTP", ep);
                            let hc = MasternodeClient::new(http_ep.clone(), creds.clone());
                            match tokio::time::timeout(probe_timeout2, hc.health_check()).await {
                                Ok(Ok(health)) => {
                                    log::info!("✅ Gossip peer {} reachable via HTTP (no TLS)", http_ep);
                                    (true, Some(health.block_height), Some(health.version), http_ep.clone())
                                }
                                _ => (false, None, None, ep.clone()),
                            }
                        }
                    }
                } else {
                    (false, None, None, ep.clone())
                };
                let ws_available = if is_healthy {
                    let ws_url = crate::config_new::Config::derive_ws_url(&working_ep);
                    tokio::time::timeout(
                        std::time::Duration::from_secs(5),
                        tokio_tungstenite::connect_async_tls_with_config(
                            &ws_url,
                            None,
                            false,
                            Some(crate::ws_client::make_tls_connector()),
                        ),
                    )
                    .await
                    .map(|r| r.is_ok())
                    .unwrap_or(false)
                } else {
                    false
                };
                PeerInfo {
                    endpoint: working_ep,
                    is_active: false,
                    is_healthy,
                    ws_available,
                    ping_ms,
                    block_height,
                    version,
                }
            }));
        }
        for handle in gossip_handles {
            if let Ok(info) = handle.await {
                if !info.is_healthy {
                    continue; // Don't bother adding unhealthy gossip peers
                }
                log::info!(
                    "✅ Gossip peer {} is healthy ({}ms)",
                    info.endpoint,
                    info.ping_ms.unwrap_or(0)
                );
                // Add the new peer if we're below the connection cap.
                // Only replace the slowest when at the limit.
                if peer_infos.len() < max_connections {
                    peer_infos.push(info);
                } else {
                    let slowest_idx = peer_infos
                        .iter()
                        .enumerate()
                        .filter(|(_, p)| p.is_healthy)
                        .max_by_key(|(_, p)| p.ping_ms.unwrap_or(u64::MAX));
                    let new_ping = info.ping_ms.unwrap_or(u64::MAX);
                    if let Some((idx, slowest)) = slowest_idx {
                        if slowest.ping_ms.unwrap_or(u64::MAX) > new_ping {
                            log::info!(
                                "🔀 Replacing slow peer {} ({}ms) with {} ({}ms)",
                                peer_infos[idx].endpoint,
                                slowest.ping_ms.unwrap_or(0),
                                info.endpoint,
                                new_ping
                            );
                            peer_infos[idx] = info;
                        }
                    }
                }
            }
        }
        // Re-sort after integrating gossip peers
        peer_infos.sort_by(|a, b| {
            b.is_healthy
                .cmp(&a.is_healthy)
                .then(b.ws_available.cmp(&a.ws_available))
                .then(
                    a.ping_ms
                        .unwrap_or(u64::MAX)
                        .cmp(&b.ping_ms.unwrap_or(u64::MAX)),
                )
        });
    }

    // Cache all discovered healthy peers for offline use
    let healthy_endpoints: Vec<String> = peer_infos
        .iter()
        .filter(|p| p.is_healthy)
        .map(|p| p.endpoint.clone())
        .collect();
    if !healthy_endpoints.is_empty() {
        peer_discovery::save_discovered_peers(is_testnet, &healthy_endpoints);
    }

    for p in &mut peer_infos {
        p.is_active = p.is_healthy && p.endpoint == active_endpoint;
    }

    Ok((active_endpoint, peer_infos))
}

/// Mutable state owned by the service loop.
struct ServiceState {
    svc_tx: mpsc::UnboundedSender<ServiceEvent>,
    client: Option<MasternodeClient>,
    wallet: Option<WalletManager>,
    wallet_db: Option<WalletDb>,
    addresses: Vec<String>,
    network_type: NetworkType,
    config: Config,
    ws_event_tx: mpsc::UnboundedSender<WsEvent>,
    ws_shutdown_rx: tokio::sync::watch::Receiver<bool>,
    ws_handle: Option<tokio::task::JoinHandle<()>>,
    /// Most recent peer list from discovery, for failover on capacity-full.
    last_peers: Vec<PeerInfo>,
}

impl ServiceState {
    /// Load a wallet and start the WebSocket connection.
    fn load_wallet(&mut self, password: Option<String>) {
        let result = match password {
            Some(pw) => WalletManager::load_with_password(self.network_type, &pw),
            None => {
                // Check if encrypted first
                if WalletManager::is_encrypted(self.network_type).unwrap_or(false) {
                    let _ = self.svc_tx.send(ServiceEvent::PasswordRequired);
                    return;
                }
                WalletManager::load(self.network_type)
            }
        };
        self.finish_wallet_init(result);
    }

    /// Create a wallet from mnemonic and start the WebSocket connection.
    fn create_wallet(&mut self, mnemonic: &str, password: Option<String>) {
        let result = match password {
            Some(pw) => {
                WalletManager::create_from_mnemonic_encrypted(self.network_type, mnemonic, &pw)
            }
            None => WalletManager::create_from_mnemonic(self.network_type, mnemonic),
        };
        self.finish_wallet_init(result);
    }

    fn finish_wallet_init(&mut self, result: Result<WalletManager, wallet_dat::WalletDatError>) {
        match result {
            Ok(mut wm) => {
                // Sync address index from DB so all generated addresses are restored
                if let Some(ref db) = self.wallet_db {
                    if let Ok(owned) = db.get_owned_addresses() {
                        if let Some(max_idx) = owned.iter().filter_map(|c| c.derivation_index).max()
                        {
                            wm.sync_address_index(max_idx);
                        }
                    }
                }
                let raw_addrs = derive_addresses(&mut wm);
                self.addresses = raw_addrs.clone();
                let address_infos: Vec<AddressInfo> = raw_addrs
                    .iter()
                    .enumerate()
                    .map(|(i, addr)| {
                        let label = self
                            .wallet_db
                            .as_ref()
                            .and_then(|db| db.get_contact(addr).ok().flatten())
                            .map(|c| c.label)
                            .unwrap_or_else(|| format!("Address #{}", i));
                        AddressInfo {
                            address: addr.clone(),
                            label,
                        }
                    })
                    .collect();
                let is_testnet = self.network_type == NetworkType::Testnet;
                let is_encrypted = wm.is_wallet_encrypted();
                let _ = self.svc_tx.send(ServiceEvent::WalletLoaded {
                    addresses: address_infos,
                    is_testnet,
                    is_encrypted,
                });

                // Load cached data from database for instant startup
                if let Some(ref db) = self.wallet_db {
                    // Load persisted send records FIRST so they're available for merge.
                    // Cross-reference with cached transactions: if a send record's
                    // txid isn't in the cache, the masternode rejected it.
                    let cached_txs = db.get_cached_transactions().unwrap_or_default();
                    let cached_txids: std::collections::HashSet<&str> =
                        cached_txs.iter().map(|t| t.txid.as_str()).collect();

                    if let Ok(mut send_records) = db.get_send_records() {
                        let mut updated = Vec::new();
                        for (txid, sr) in send_records.iter_mut() {
                            if matches!(
                                sr.status,
                                crate::masternode_client::TransactionStatus::Pending
                            ) && !cached_txids.contains(txid.as_str())
                            {
                                log::info!(
                                    "Marking send record {} as Declined (not in cached txs)",
                                    &txid[..16.min(txid.len())]
                                );
                                sr.status = crate::masternode_client::TransactionStatus::Declined;
                                updated.push(sr.clone());
                            }
                        }
                        // Persist the Declined status immediately
                        for sr in &updated {
                            let _ = db.save_send_record(sr);
                        }
                        if !send_records.is_empty() {
                            log::info!("Loaded {} persisted send records", send_records.len());
                            let _ = self
                                .svc_tx
                                .send(ServiceEvent::SendRecordsLoaded(send_records));
                        }
                    }
                    if let Ok(Some(bal)) = db.get_cached_balance() {
                        log::info!("Loaded cached balance from database");
                        let _ = self.svc_tx.send(ServiceEvent::BalanceUpdated(bal));
                    }
                    // Load masternode entries
                    if let Ok(entries) = db.get_masternode_entries() {
                        if entries.is_empty() {
                            // Auto-import masternode.conf if it exists and DB has no entries
                            let mn_conf_path = self.config.wallet_dir().join("masternode.conf");
                            if mn_conf_path.exists() {
                                if let Ok(contents) = std::fs::read_to_string(&mn_conf_path) {
                                    let mut count = 0;
                                    for line in contents.lines() {
                                        if let Some(entry) =
                                            crate::wallet_db::MasternodeEntry::parse_conf_line(line)
                                        {
                                            let _ = db.save_masternode_entry(&entry);
                                            count += 1;
                                        }
                                    }
                                    if count > 0 {
                                        log::info!(
                                            "📥 Auto-imported {} entries from {}",
                                            count,
                                            mn_conf_path.display()
                                        );
                                        if let Ok(imported) = db.get_masternode_entries() {
                                            let _ = self.svc_tx.send(
                                                ServiceEvent::MasternodeEntriesLoaded(imported),
                                            );
                                        }
                                    }
                                }
                            }
                        } else {
                            log::info!("Loaded {} masternode entries", entries.len());
                            // Ensure collateral UTXOs are locked for all known entries
                            for entry in &entries {
                                let _ = db.lock_collateral(
                                    &entry.collateral_txid,
                                    entry.collateral_vout,
                                    &entry.alias,
                                );
                            }
                            let _ = self
                                .svc_tx
                                .send(ServiceEvent::MasternodeEntriesLoaded(entries));
                        }
                    }
                    if !cached_txs.is_empty() {
                        log::info!(
                            "Loaded {} cached transactions from database",
                            cached_txs.len()
                        );
                        let _ = self
                            .svc_tx
                            .send(ServiceEvent::TransactionsUpdated(cached_txs));
                    }
                    // Load persisted UTXOs so balance displays correctly before sync
                    if let Ok(utxo_records) = db.get_all_utxos() {
                        if !utxo_records.is_empty() {
                            let utxos: Vec<crate::masternode_client::Utxo> = utxo_records
                                .iter()
                                .map(|r| crate::masternode_client::Utxo {
                                    txid: r.tx_hash.clone(),
                                    vout: r.output_index,
                                    amount: r.amount,
                                    address: r.address.clone(),
                                    confirmations: r.confirmations as u32,
                                    spendable: true,
                                })
                                .collect();
                            log::info!("Loaded {} cached UTXOs from database", utxos.len());
                            self.send_utxos_updated(utxos);
                        }
                    }
                    // Load external contacts for send address book
                    if let Ok(contacts) = db.get_external_contacts() {
                        let infos: Vec<crate::state::ContactInfo> = contacts
                            .into_iter()
                            .map(|c| crate::state::ContactInfo {
                                name: c.name.unwrap_or(c.label),
                                address: c.address,
                            })
                            .collect();
                        let _ = self.svc_tx.send(ServiceEvent::ContactsUpdated(infos));
                    }
                }

                self.wallet = Some(wm);
                // Only start WS if we already have a peer connection
                if self.config.active_endpoint.is_some() {
                    self.start_ws();
                }
            }
            Err(e) => {
                let _ = self
                    .svc_tx
                    .send(ServiceEvent::Error(format!("Wallet error: {}", e)));
            }
        }
    }

    /// Send UtxosUpdated event and persist UTXOs to sled for instant startup.
    /// Also backfills collateral_amount on masternode entries that are missing it.
    fn send_utxos_updated(&self, utxos: Vec<crate::masternode_client::Utxo>) {
        if let Some(ref db) = self.wallet_db {
            let _ = db.clear_all_utxos();
            for u in &utxos {
                let _ = db.save_utxo(&crate::wallet_db::UtxoRecord {
                    tx_hash: u.txid.clone(),
                    output_index: u.vout,
                    amount: u.amount,
                    address: u.address.clone(),
                    block_height: 0,
                    confirmations: u.confirmations as u64,
                });
            }
            // Backfill collateral_amount on masternode entries and persist
            if let Ok(entries) = db.get_masternode_entries() {
                for mut entry in entries {
                    if entry.collateral_amount.is_none() {
                        if let Some(u) = utxos.iter().find(|u| {
                            u.txid == entry.collateral_txid && u.vout == entry.collateral_vout
                        }) {
                            entry.collateral_amount = Some(u.amount);
                            let _ = db.save_masternode_entry(&entry);
                            log::info!("💾 Backfilled collateral {} for '{}'", u.amount, entry.alias);
                        }
                    }
                }
            }
        }
        let _ = self.svc_tx.send(ServiceEvent::UtxosUpdated(utxos));
    }

    /// Start (or restart) the WebSocket client for current addresses.
    fn start_ws(&mut self) {
        if let Some(h) = self.ws_handle.take() {
            h.abort();
        }
        let handle = WsClient::start(
            self.config.ws_url(),
            self.addresses.clone(),
            self.ws_event_tx.clone(),
            self.ws_shutdown_rx.clone(),
        );
        self.ws_handle = Some(handle);
    }
}

/// Build a masternode registration transaction.
///
/// Returns `(hex_encoded_tx, txid)` on success.
/// The transaction:
/// - Uses a wallet UTXO for the fee (minimum fee, since no value is transferred)
/// - Contains `special_data` with the registration payload
/// - The registration payload is signed with the collateral owner's Ed25519 key
/// - The transaction input is signed with the fee UTXO owner's key
#[allow(clippy::too_many_arguments)]
async fn build_masternode_reg_tx(
    wm: &mut WalletManager,
    client: &MasternodeClient,
    addresses: &[String],
    collateral_txid: &str,
    collateral_vout: u32,
    masternode_ip: &str,
    masternode_port: u16,
    payout_address: &str,
) -> Result<(String, String), String> {
    use sha2::{Digest, Sha256};
    use wallet::Transaction;

    // 1. Find the collateral UTXO owner address and derive their HD keypair
    let mut collateral_owner_addr: Option<String> = None;
    for addr in addresses {
        match client.get_utxos(addr).await {
            Ok(utxos) => {
                for utxo in &utxos {
                    if utxo.txid == collateral_txid && utxo.vout == collateral_vout {
                        collateral_owner_addr = Some(addr.clone());
                        break;
                    }
                }
            }
            Err(_) => continue,
        }
        if collateral_owner_addr.is_some() {
            break;
        }
    }
    let collateral_addr =
        collateral_owner_addr.ok_or("Collateral UTXO not found in wallet".to_string())?;

    // Find HD index for collateral owner
    let addr_to_index: std::collections::HashMap<String, u32> = (0..wm.get_address_count())
        .filter_map(|i| wm.derive_address(i).ok().map(|a| (a, i)))
        .collect();
    let collateral_hd_index = addr_to_index
        .get(&collateral_addr)
        .copied()
        .ok_or("Collateral address not found in HD wallet".to_string())?;
    let collateral_kp = wm
        .derive_keypair(collateral_hd_index)
        .map_err(|e| format!("Failed to derive collateral keypair: {}", e))?;

    // 2. Sign the registration payload
    let collateral_outpoint = format!("{}:{}", collateral_txid, collateral_vout);
    let signing_message = format!(
        "MN_REG:{}:{}:{}:{}",
        collateral_outpoint, masternode_ip, masternode_port, payout_address
    );
    let msg_hash: [u8; 32] = Sha256::digest(signing_message.as_bytes()).into();
    let signature_bytes = collateral_kp.sign(&msg_hash);
    let signature_hex = hex::encode(&signature_bytes);
    let owner_pubkey_hex = hex::encode(collateral_kp.public_key_bytes());

    // 3. Fetch all UTXOs to find one for the fee
    let min_fee: u64 = 1_000_000; // 0.01 TIME
    let mut fee_utxo = None;
    let mut all_utxos = Vec::new();
    for addr in addresses {
        match client.get_utxos(addr).await {
            Ok(utxos) => {
                for utxo in &utxos {
                    // Don't use the collateral UTXO for fee payment
                    let is_collateral =
                        utxo.txid == collateral_txid && utxo.vout == collateral_vout;
                    if !is_collateral && utxo.amount >= min_fee && fee_utxo.is_none() {
                        fee_utxo = Some(utxo.clone());
                    }
                    all_utxos.push(utxo.clone());
                }
            }
            Err(_) => continue,
        }
    }
    let fee_utxo = fee_utxo.ok_or("No UTXO available to pay registration fee".to_string())?;

    // 4. Build the transaction
    let mut tx = Transaction::new();

    // Add fee input
    let mut tx_hash = [0u8; 32];
    hex::decode_to_slice(&fee_utxo.txid, &mut tx_hash)
        .map_err(|e| format!("Invalid fee UTXO txid: {}", e))?;
    let input = wallet::TxInput::new(tx_hash, fee_utxo.vout);
    tx.add_input(input);

    // Add change output (fee_utxo.amount - min_fee)
    let change = fee_utxo.amount.saturating_sub(min_fee);
    if change > 0 {
        let change_addr = wallet::Address::from_string(&fee_utxo.address)
            .map_err(|e| format!("Invalid change address: {}", e))?;
        let change_output = wallet::TxOutput::new(change, change_addr);
        tx.add_output(change_output)
            .map_err(|e| format!("Failed to add change output: {}", e))?;
    }

    // Set the special_data
    tx.special_data = Some(wallet::SpecialTransactionData::MasternodeReg {
        collateral_outpoint,
        masternode_ip: masternode_ip.to_string(),
        masternode_port,
        payout_address: payout_address.to_string(),
        owner_pubkey: owner_pubkey_hex,
        signature: signature_hex,
    });

    // 5. Sign the fee input with the fee UTXO owner's key
    let fee_hd_index = addr_to_index
        .get(&fee_utxo.address)
        .copied()
        .ok_or("Fee UTXO address not found in HD wallet".to_string())?;
    let fee_kp = wm
        .derive_keypair(fee_hd_index)
        .map_err(|e| format!("Failed to derive fee keypair: {}", e))?;
    tx.sign(&fee_kp, 0)
        .map_err(|e| format!("Failed to sign fee input: {}", e))?;

    // 6. Serialize and return
    let txid = tx.txid();
    let bytes = tx
        .to_bytes()
        .map_err(|e| format!("Failed to serialize tx: {}", e))?;
    let tx_hex: String = bytes.iter().map(|b| format!("{:02x}", b)).collect();

    log::info!(
        "Built MN registration tx: txid={}, fee_utxo={}, collateral={}:{}",
        txid,
        fee_utxo.txid,
        collateral_txid,
        collateral_vout
    );

    Ok((tx_hex, txid))
}

/// Build a masternode payout update transaction.
///
/// Returns `(hex_encoded_tx, txid)` on success.
async fn build_masternode_update_tx(
    wm: &mut WalletManager,
    client: &MasternodeClient,
    addresses: &[String],
    masternode_id: &str,
    new_payout_address: &str,
) -> Result<(String, String), String> {
    use sha2::{Digest, Sha256};
    use wallet::Transaction;

    // Use the first address's key as the owner key (the owner registered the MN)
    let addr_to_index: std::collections::HashMap<String, u32> = (0..wm.get_address_count())
        .filter_map(|i| wm.derive_address(i).ok().map(|a| (a, i)))
        .collect();

    // Find the first wallet address that has a UTXO for the fee
    let min_fee: u64 = 1_000_000; // 0.01 TIME
    let mut fee_utxo = None;
    for addr in addresses {
        match client.get_utxos(addr).await {
            Ok(utxos) => {
                for utxo in utxos {
                    if utxo.amount >= min_fee && fee_utxo.is_none() {
                        fee_utxo = Some(utxo);
                    }
                }
            }
            Err(_) => continue,
        }
    }
    let fee_utxo = fee_utxo.ok_or("No UTXO available to pay update fee".to_string())?;

    // Sign the update payload with address #0 (the owner key)
    let owner_index = 0u32;
    let owner_kp = wm
        .derive_keypair(owner_index)
        .map_err(|e| format!("Failed to derive owner keypair: {}", e))?;

    let signing_message = format!("MN_UPDATE:{}:{}", masternode_id, new_payout_address);
    let msg_hash: [u8; 32] = Sha256::digest(signing_message.as_bytes()).into();
    let signature_bytes = owner_kp.sign(&msg_hash);
    let signature_hex = hex::encode(&signature_bytes);
    let owner_pubkey_hex = hex::encode(owner_kp.public_key_bytes());

    // Build the transaction
    let mut tx = Transaction::new();

    let mut tx_hash = [0u8; 32];
    hex::decode_to_slice(&fee_utxo.txid, &mut tx_hash)
        .map_err(|e| format!("Invalid fee UTXO txid: {}", e))?;
    let input = wallet::TxInput::new(tx_hash, fee_utxo.vout);
    tx.add_input(input);

    let change = fee_utxo.amount.saturating_sub(min_fee);
    if change > 0 {
        let change_addr = wallet::Address::from_string(&fee_utxo.address)
            .map_err(|e| format!("Invalid change address: {}", e))?;
        let change_output = wallet::TxOutput::new(change, change_addr);
        tx.add_output(change_output)
            .map_err(|e| format!("Failed to add change output: {}", e))?;
    }

    tx.special_data = Some(wallet::SpecialTransactionData::MasternodePayoutUpdate {
        masternode_id: masternode_id.to_string(),
        new_payout_address: new_payout_address.to_string(),
        owner_pubkey: owner_pubkey_hex,
        signature: signature_hex,
    });

    // Sign the fee input
    let fee_hd_index = addr_to_index
        .get(&fee_utxo.address)
        .copied()
        .ok_or("Fee UTXO address not found in HD wallet".to_string())?;
    let fee_kp = wm
        .derive_keypair(fee_hd_index)
        .map_err(|e| format!("Failed to derive fee keypair: {}", e))?;
    tx.sign(&fee_kp, 0)
        .map_err(|e| format!("Failed to sign fee input: {}", e))?;

    let txid = tx.txid();
    let bytes = tx
        .to_bytes()
        .map_err(|e| format!("Failed to serialize tx: {}", e))?;
    let tx_hex: String = bytes.iter().map(|b| format!("{:02x}", b)).collect();

    log::info!(
        "Built MN payout update tx: txid={}, masternode_id={}",
        txid,
        masternode_id
    );

    Ok((tx_hex, txid))
}

/// Derive all known addresses from the wallet manager.
/// Ensures at least one address exists.
fn derive_addresses(wm: &mut WalletManager) -> Vec<String> {
    // Ensure at least one address is derived
    if wm.get_address_count() == 0 {
        let _ = wm.get_next_address();
    }
    (0..wm.get_address_count())
        .filter_map(|i| wm.derive_address(i).ok())
        .collect()
}

/// Return a default template for a config file based on its filename.
pub fn config_file_template(path: &std::path::Path) -> &'static str {
    match path.file_name().and_then(|n| n.to_str()) {
        Some("time.conf") => {
            "\
# TIME Coin Wallet Configuration
# Lines starting with # are comments.

# Network: 1=testnet, 0=mainnet
testnet=0

# Masternode peers (IP, IP:port, or http://IP:port). Repeat for multiple.
#addnode=64.91.241.10:24001
#addnode=50.28.104.50:24001

# RPC credentials (from the masternode's time.conf)
#rpcuser=timecoinrpc
#rpcpassword=

# Maximum peer connections (0 = unlimited)
maxconnections=0
"
        }
        Some("masternode.conf") => {
            "\
# TIME Coin Masternode Configuration
#
# Format (one entry per line):
#   alias  IP:port  masternodeprivkey  collateral_txid  collateral_vout
#
# Example:
#   mn1  69.167.168.176:24100  5KCgSQS9uFLz...  fc5b049a3980...  0
#
"
        }
        _ => "",
    }
}

/// Run UTXO consolidation in a background task.
///
/// This function owns all the data it needs (no references to ServiceState),
/// so it can run concurrently without blocking the service select! loop.
async fn consolidate_utxos_background(
    client: MasternodeClient,
    svc_tx: mpsc::UnboundedSender<ServiceEvent>,
    addresses: Vec<String>,
    dest_addr: String,
    addr_to_keypair: std::collections::HashMap<String, wallet::Keypair>,
) {
    // Fetch all spendable UTXOs across all addresses.
    let mut all_utxos: Vec<crate::masternode_client::Utxo> = Vec::new();
    for addr in &addresses {
        if let Ok(utxos) = client.get_utxos(addr).await {
            all_utxos.extend(utxos);
        }
    }

    // Only include spendable UTXOs.
    all_utxos.retain(|u| u.spendable);

    if all_utxos.len() <= 1 {
        let _ = svc_tx.send(ServiceEvent::ConsolidationComplete {
            message: "Nothing to consolidate — already 1 UTXO or fewer.".to_string(),
        });
        return;
    }

    // Validate destination address once up-front.
    let dest_address = match wallet::Address::from_string(&dest_addr) {
        Ok(a) => a,
        Err(e) => {
            let _ = svc_tx.send(ServiceEvent::ConsolidationComplete {
                message: format!("Consolidation aborted: invalid destination address — {}", e),
            });
            return;
        }
    };

    let batch_size = 50;
    let total_batches = all_utxos.len().div_ceil(batch_size);
    let mut consolidated = 0usize;
    let mut failed = 0usize;

    for (batch_idx, chunk) in all_utxos.chunks(batch_size).enumerate() {
        if chunk.len() <= 1 {
            continue;
        }

        let _ = svc_tx.send(ServiceEvent::ConsolidationProgress {
            batch: batch_idx + 1,
            total_batches,
            message: format!(
                "Consolidating batch {}/{} ({} UTXOs)…",
                batch_idx + 1,
                total_batches,
                chunk.len()
            ),
        });

        // Build transaction directly — bypass create_transaction to avoid
        // double-fee calculation and temp-wallet address mismatch.
        let mut tx = wallet::Transaction::new();
        let mut valid_utxos: Vec<&crate::masternode_client::Utxo> = Vec::new();

        for utxo in chunk {
            let mut tx_hash = [0u8; 32];
            match hex::decode(&utxo.txid) {
                Ok(bytes) if bytes.len() == 32 => {
                    tx_hash.copy_from_slice(&bytes);
                    tx.add_input(wallet::TxInput::new(tx_hash, utxo.vout));
                    valid_utxos.push(utxo);
                }
                _ => {
                    log::warn!(
                        "Consolidation batch {}: skipping UTXO with invalid txid '{}'",
                        batch_idx + 1,
                        utxo.txid
                    );
                }
            }
        }

        if valid_utxos.is_empty() {
            failed += 1;
            continue;
        }

        let batch_total: u64 = valid_utxos.iter().map(|u| u.amount).sum();
        let fee = wallet::calculate_fee(batch_total);
        let send_amount = batch_total.saturating_sub(fee);

        if send_amount == 0 {
            log::info!(
                "Consolidation batch {}: skipped — batch value {} <= min fee {}",
                batch_idx + 1,
                batch_total,
                fee
            );
            continue;
        }

        if tx.add_output(wallet::TxOutput::new(send_amount, dest_address.clone())).is_err() {
            log::warn!("Consolidation batch {}: failed to add output", batch_idx + 1);
            failed += 1;
            continue;
        }

        // Sign each input with its address's keypair.
        let mut unsigned_inputs = 0usize;
        for (input_idx, utxo) in valid_utxos.iter().enumerate() {
            match addr_to_keypair.get(&utxo.address) {
                Some(kp) => {
                    if let Err(e) = tx.sign(kp, input_idx) {
                        log::warn!(
                            "Consolidation batch {}: sign input {} failed: {}",
                            batch_idx + 1,
                            input_idx,
                            e
                        );
                    }
                }
                None => {
                    log::warn!(
                        "Consolidation batch {}: no keypair for address {} (input {})",
                        batch_idx + 1,
                        utxo.address,
                        input_idx
                    );
                    unsigned_inputs += 1;
                }
            }
        }
        if unsigned_inputs > 0 {
            log::warn!(
                "Consolidation batch {}: {} unsigned inputs — skipping broadcast",
                batch_idx + 1,
                unsigned_inputs
            );
            failed += 1;
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            continue;
        }

        match tx.to_bytes() {
            Ok(bytes) => {
                let tx_hex: String = bytes.iter().map(|b| format!("{:02x}", b)).collect();
                match client.broadcast_transaction(&tx_hex).await {
                    Ok(txid) => {
                        log::info!(
                            "✅ Consolidation batch {}/{}: {}",
                            batch_idx + 1,
                            total_batches,
                            txid
                        );
                        consolidated += 1;
                    }
                    Err(e) => {
                        log::warn!(
                            "❌ Consolidation batch {} broadcast failed: {}",
                            batch_idx + 1,
                            e
                        );
                        failed += 1;
                    }
                }
            }
            Err(e) => {
                log::warn!("❌ Consolidation batch {} serialize failed: {}", batch_idx + 1, e);
                failed += 1;
            }
        }

        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    }

    let msg = if failed == 0 {
        format!("Consolidation complete: {} batch(es) sent.", consolidated)
    } else {
        format!("Consolidation finished: {} succeeded, {} failed.", consolidated, failed)
    };
    let _ = svc_tx.send(ServiceEvent::ConsolidationComplete { message: msg });

    // Refresh UTXOs and balance after consolidation.
    let mut refreshed = Vec::new();
    for addr in &addresses {
        if let Ok(utxos) = client.get_utxos(addr).await {
            refreshed.extend(utxos);
        }
    }
    let _ = svc_tx.send(ServiceEvent::UtxosUpdated(refreshed));
    if let Ok(bal) = client.get_balances(&addresses).await {
        let _ = svc_tx.send(ServiceEvent::BalanceUpdated(bal));
    }
}
