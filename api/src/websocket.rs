pub async fn notify_transaction_invalidated(
    ws_connections: &HashMap<String, WebSocket>,
    event: TxInvalidationEvent
) {
    for address in &event.affected_addresses {
        if let Some(ws) = ws_connections.get(address) {
            ws.send(json!({
                "type": "tx_invalidated",
                "txid": event.txid,
                "reason": event.reason,
                "timestamp": event.timestamp
            })).await;
        }
    }
}
