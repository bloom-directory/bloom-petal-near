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
        Ok(session) => session
            .approval
            .as_ref()
            .map(petal::read_json_value)
            .unwrap_or_else(|| petal::error(-1, "no approval pending")),
        Err(error) => petal::error(-1, error),
    }
});
