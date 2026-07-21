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
            "correlation_id": session.quote.correlation_id,
            "timestamp": session.quote.timestamp,
            "signature": session.quote.signature,
            "quote_request": session.quote.quote_request,
            "quote": session.quote.quote,
            "verified": session.quote_verified,
            "hash": session.quote_hash,
        })),
        Err(error) => petal::error(-1, error),
    }
});
