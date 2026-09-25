petal::route_file!(spec: petal::store_read_spec(), read: |ctx: &petal::Ctx| {
    use crate::workflow::Host;

    let wallet = match petal::wallet_param(ctx) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let mut host = crate::workflow::BloomHost;
    match host.get(&format!("swaps/{wallet}/latest"), 128) {
        Ok(Some(value)) => match crate::workflow::project_latest(
            wallet,
            &value,
            petal::route_param(ctx, "bloom.route_prefix"),
        ) {
            Ok(projected) => petal::DispatchResponse::Read(projected),
            Err(error) => petal::error(-4, error),
        },
        Ok(None) => petal::error(-1, "no swap session"),
        Err(error) => petal::error(-4, error),
    }
});
