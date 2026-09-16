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
        Ok(session) => petal::read_json_value(&session.request),
        Err(error) => petal::error(-1, error),
    }
});
