petal::route_file!(
    spec: petal::store_dir_spec().caps(&["bloom:store"]),
    ctx_list: |_ctx: &petal::Ctx| {
        let mut host = crate::workflow::BloomHost;
        match crate::policy::configured_wallets(&mut host) {
            Ok(wallets) => Ok(wallets.into_iter().map(petal::dir).collect()),
            Err(error) => Err(petal::error(-4, crate::redaction::sanitize_message(&error))),
        }
    }
);
