//! Dynamic peer discovery via the TIME Coin website API.
//!
//! Discovery order:
//! 1. Manual peers from `config.toml` (`peers = [...]`)
//! 2. API peers from `https://time-coin.io/api/v1/nodes/peers/{network}`
//! 3. Cached peers from `~/.time-wallet/peers.dat` (always merged when present)
//!
//! The website list is the **trusted bootstrap seed set**, not a live dump of
//! every masternode. Rediscovery must not shrink the candidate pool to only
//! those seeds — otherwise the Connections UI oscillates (seed count ↔ gossip
//! expanded mesh) every refresh cycle.
//!
//! `peers.dat` is written only after a full discovery pass (API + probes +
//! gossip) via [`save_discovered_peers`], so the cache reflects healthy peers
//! rather than the thin official seed list alone.

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;

/// Mainnet peer list URL.
const MAINNET_PEERS_URL: &str = "https://time-coin.io/api/v1/nodes/peers/mainnet";

/// Testnet peer list URL.
const TESTNET_PEERS_URL: &str = "https://time-coin.io/api/v1/nodes/peers/testnet";

/// Mainnet RPC port.
const MAINNET_PORT: u16 = 24001;

/// Testnet RPC port.
const TESTNET_PORT: u16 = 24101;

/// (mainnet, testnet) memo slots for the website seed list.
type SeedMemo = (Option<Vec<String>>, Option<Vec<String>>);

/// Process-lifetime memo of the website seed list (one HTTP fetch per network).
/// Subsequent calls reuse this in-memory result; they never re-hit time-coin.io.
static WEBSITE_SEED_MEMO: Mutex<SeedMemo> = Mutex::new((None, None));

/// Cached peer list stored in `peers.dat`.
#[derive(Debug, Serialize, Deserialize)]
struct PeerCache {
    network: String,
    peers: Vec<String>,
}

/// Fetch peers from the website API **once on startup** (memoized per process),
/// always unioning with the local cache when present.
///
/// Callers must only invoke this from the startup discovery path. Periodic
/// refresh and network switch use cache + gossip instead.
///
/// Returns `https://{ip}:{port}` endpoint URLs (RPC ports).
pub async fn fetch_peers(is_testnet: bool) -> Result<Vec<String>, PeerDiscoveryError> {
    match fetch_from_api_once(is_testnet).await {
        Ok(api_endpoints) => {
            let api_count = api_endpoints.len();
            let mut endpoints = api_endpoints;
            if let Some(cached) = load_cache(is_testnet) {
                let before = endpoints.len();
                endpoints.extend(cached);
                endpoints.sort();
                endpoints.dedup();
                if endpoints.len() > before {
                    log::info!(
                        "📦 Merged cache into API seeds: {} seeds + cache → {} candidates",
                        api_count,
                        endpoints.len()
                    );
                }
            }
            // Intentionally do not save_cache here. The service writes the full
            // healthy set after probes + gossip via save_discovered_peers.
            Ok(endpoints)
        }
        Err(api_err) => {
            log::warn!("⚠ API peer discovery failed: {}", api_err);
            match load_cache(is_testnet) {
                Some(cached) => {
                    log::info!("📦 Using {} cached peers from peers.dat", cached.len());
                    Ok(cached)
                }
                None => Err(api_err),
            }
        }
    }
}

/// HTTP fetch of the website seed list, memoized for the process lifetime.
///
/// First call for mainnet or testnet performs the request. Later calls return
/// the same list without contacting the network.
async fn fetch_from_api_once(is_testnet: bool) -> Result<Vec<String>, PeerDiscoveryError> {
    {
        let memo = WEBSITE_SEED_MEMO.lock().unwrap_or_else(|e| e.into_inner());
        let slot = if is_testnet { &memo.1 } else { &memo.0 };
        if let Some(cached) = slot {
            log::debug!(
                "🌐 Reusing in-memory website seed list ({} peers, no HTTP)",
                cached.len()
            );
            return Ok(cached.clone());
        }
    }

    let endpoints = fetch_from_api(is_testnet).await?;

    {
        let mut memo = WEBSITE_SEED_MEMO.lock().unwrap_or_else(|e| e.into_inner());
        let slot = if is_testnet { &mut memo.1 } else { &mut memo.0 };
        // Only store on first success; concurrent first callers may race — keep first.
        if slot.is_none() {
            *slot = Some(endpoints.clone());
        }
    }

    Ok(endpoints)
}

/// Fetch the peer list directly from the website API (single HTTP request).
async fn fetch_from_api(is_testnet: bool) -> Result<Vec<String>, PeerDiscoveryError> {
    let url = if is_testnet {
        TESTNET_PEERS_URL
    } else {
        MAINNET_PEERS_URL
    };

    let port = if is_testnet {
        TESTNET_PORT
    } else {
        MAINNET_PORT
    };

    log::info!("🔍 Fetching peers from {} (startup, once)", url);

    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .connect_timeout(Duration::from_secs(5))
        .build()?;

    let response = client.get(url).send().await?;

    if !response.status().is_success() {
        return Err(PeerDiscoveryError::HttpStatus(response.status().as_u16()));
    }

    let ips: Vec<String> = response.json().await?;

    if ips.is_empty() {
        return Err(PeerDiscoveryError::NoPeers);
    }

    let endpoints: Vec<String> = ips
        .into_iter()
        .map(|ip| format!("https://{}:{}", ip, port))
        .collect();

    log::info!(
        "✅ Discovered {} peers from API (startup, not polled again)",
        endpoints.len()
    );
    Ok(endpoints)
}

// ============================================================================
// Cache
// ============================================================================

fn cache_path() -> Option<PathBuf> {
    let home = dirs::home_dir()?;
    Some(home.join(".time-wallet").join("peers.dat"))
}

fn save_cache(is_testnet: bool, endpoints: &[String]) {
    let Some(path) = cache_path() else { return };
    let cache = PeerCache {
        network: if is_testnet { "testnet" } else { "mainnet" }.to_string(),
        peers: endpoints.to_vec(),
    };
    match serde_json::to_string(&cache) {
        Ok(json) => {
            if let Err(e) = fs::write(&path, json) {
                log::warn!("⚠ Failed to write peers.dat: {}", e);
            } else {
                log::info!("💾 Cached {} peers to {}", endpoints.len(), path.display());
            }
        }
        Err(e) => log::warn!("⚠ Failed to serialize peer cache: {}", e),
    }
}

/// Save discovered peers to the cache (called from service after gossip discovery).
pub fn save_discovered_peers(is_testnet: bool, endpoints: &[String]) {
    save_cache(is_testnet, endpoints);
}

/// Load cached peers without contacting the website API.
/// Used on periodic refresh so rediscovery does not hit time-coin.io.
pub fn load_cached_peers(is_testnet: bool) -> Option<Vec<String>> {
    load_cache(is_testnet)
}

fn load_cache(is_testnet: bool) -> Option<Vec<String>> {
    let path = cache_path()?;
    let contents = fs::read_to_string(&path).ok()?;
    let cache: PeerCache = serde_json::from_str(&contents).ok()?;
    let expected_network = if is_testnet { "testnet" } else { "mainnet" };
    if cache.network != expected_network {
        log::warn!(
            "⚠ Cached peers are for {}, need {} — ignoring",
            cache.network,
            expected_network
        );
        return None;
    }
    if cache.peers.is_empty() {
        return None;
    }
    Some(cache.peers)
}

// ============================================================================
// Errors
// ============================================================================

#[derive(Debug, thiserror::Error)]
pub enum PeerDiscoveryError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Peer API returned status {0}")]
    HttpStatus(u16),

    #[error("No peers returned by API")]
    NoPeers,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_urls() {
        assert!(MAINNET_PEERS_URL.contains("time-coin.io"));
        assert!(TESTNET_PEERS_URL.contains("testnet"));
    }

    #[test]
    fn test_ports() {
        assert_eq!(MAINNET_PORT, 24001);
        assert_eq!(TESTNET_PORT, 24101);
    }

    #[test]
    fn test_cache_roundtrip() {
        let cache = PeerCache {
            network: "mainnet".to_string(),
            peers: vec!["http://1.2.3.4:24001".to_string()],
        };
        let json = serde_json::to_string(&cache).unwrap();
        let loaded: PeerCache = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.network, "mainnet");
        assert_eq!(loaded.peers.len(), 1);
    }

    #[test]
    fn test_cache_wrong_network() {
        let cache = PeerCache {
            network: "testnet".to_string(),
            peers: vec!["http://1.2.3.4:24101".to_string()],
        };
        let json = serde_json::to_string(&cache).unwrap();
        let loaded: PeerCache = serde_json::from_str(&json).unwrap();
        // Simulates load_cache rejecting wrong network
        assert_ne!(loaded.network, "mainnet");
    }

    /// Official seed list must not discard a larger previously discovered mesh.
    #[test]
    fn test_api_seed_union_with_cache_does_not_shrink() {
        let api_seeds = vec![
            "https://1.1.1.1:24001".to_string(),
            "https://2.2.2.2:24001".to_string(),
        ];
        let cached = vec![
            "https://1.1.1.1:24001".to_string(),
            "https://3.3.3.3:24001".to_string(),
            "https://4.4.4.4:24001".to_string(),
            "https://5.5.5.5:24001".to_string(),
        ];
        let mut merged = api_seeds;
        merged.extend(cached);
        merged.sort();
        merged.dedup();
        // 2 seeds + 3 unique cache peers (1.1.1.1 already present) = 5
        assert_eq!(merged.len(), 5);
        assert!(
            merged.len() > 2,
            "union must not collapse to seed-only size"
        );
    }
}
