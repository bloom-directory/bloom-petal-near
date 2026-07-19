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
    match crate::workflow::load(&mut host, wallet, id) {
        Ok(session) => petal::read_json_value(&serde_json::json!({
            "prepared_artifact_digest": session.prepared_digest,
            "wallet": session.wallet,
            "wallet_address": session.wallet_address,
            "origin": session.origin,
            "quote_hash": session.quote_hash,
            "transaction": session.prepared_transaction,
            "destination_asset": session.quote.quote_request.destination_asset,
            "recipient": session.quote.quote_request.recipient,
            "minimum_output": session.quote.quote.min_amount_out,
            "refund_to": session.quote.quote_request.refund_to,
        })),
        Err(error) => petal::error(-1, error),
    }
});
