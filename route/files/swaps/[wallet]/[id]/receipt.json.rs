petal::route_file!(spec: petal::store_read_spec(), read: |ctx: &petal::Ctx| {
    let wallet = match petal::param(ctx, "wallet") {
        Ok(value) => value,
        Err(response) => return response,
    };
    let id = match petal::param(ctx, "id") {
        Ok(value) => value,
        Err(response) => return response,
    };
    let mut host = crate::workflow::BloomHost;
    let session = match crate::workflow::load(&mut host, wallet, id) {
        Ok(value) => value,
        Err(error) => return petal::error(-1, error),
    };
    if !session.terminal() || session.state == "abandoned" {
        return petal::error(-1, "terminal receipt not available");
    }
    let swap_details = session
        .swap_details
        .as_ref()
        .map(crate::render::sanitize_swap_details)
        .unwrap_or_else(|| serde_json::json!({}));
    let outbox_receipt = session
        .outbox_receipt
        .as_ref()
        .and_then(crate::render::sanitize_outbox_receipt_value);
    petal::read_json_value(&serde_json::json!({
        "state": session.state,
        "origin_tx_hash": session.origin_tx_hash,
        "outbox_receipt": outbox_receipt,
        "upstream_status": session.upstream_status,
        "upstream_updated_at": session.upstream_updated_at,
        "swap_details": swap_details,
        "updated_ms": session.updated_ms,
    }))
});
