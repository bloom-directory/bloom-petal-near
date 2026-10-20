petal::route_file!(spec: petal::store_read_spec(), read: |ctx: &petal::Ctx| {
    use crate::workflow::Host;

    let account = match crate::wallet::account(ctx) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let mut host = crate::workflow::BloomHost;
    match host.get(&format!("swaps/{}/latest", account.wallet()), 512) {
        Ok(Some(value)) => match crate::workflow::project_latest(
            account.wallet(),
            account.number(),
            &value,
        ) {
            Ok(projected) => petal::DispatchResponse::Read(projected),
            Err(error) => petal::error(-4, error),
        },
        Ok(None) => petal::error(-1, "no swap session"),
        Err(error) => petal::error(-4, error),
    }
});
