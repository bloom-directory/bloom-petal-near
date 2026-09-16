petal::route_file!(spec: petal::store_read_spec(), read: |ctx: &petal::Ctx| {
    let wallet = match petal::wallet_param(ctx) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let id = match petal::param(ctx, "id") {
        Ok(value) => value,
        Err(response) => return response,
    };
    let mut host = crate::workflow::BloomHost::default();
    match crate::workflow::load(&mut host, wallet, id) {
        Ok(session) => {
            let receipt = session
                .outbox_receipt
                .as_ref()
                .and_then(crate::render::sanitize_outbox_receipt_value);
            petal::read_json_value(&serde_json::json!({
                "id": session.outbox_id,
                "chain": session.origin.bloom_chain,
                "account": session.account,
                "outbox_root": session.account.as_ref().map(|account| format!("/wallets/{}/{}/chains/{}/outbox", session.wallet, account.number, session.origin.bloom_chain)),
                "state": session.outbox_state,
                "tx_hash": session.origin_tx_hash,
                "receipt": receipt,
            }))
        }
        Err(error) => petal::error(-1, error),
    }
});
