petal::route_file!(
    spec: petal::write_spec().caps(&["bloom:http", "bloom:store", "bloom:chain", "bloom:vfs.read"]),
    read: |_ctx: &petal::Ctx| {
        petal::DispatchResponse::Read(
            br#"{"session_id":"agent-known-id","account_fingerprint":"<64 lowercase hex characters from accounts.json>","derivation_path":"m/44'/60'/0'/0/<n>","swap_type":"EXACT_INPUT","origin_asset":"...","destination_asset":"...","amount":"1","recipient":"..."}"#
                .to_vec(),
        )
    },
    write: |ctx: &petal::Ctx, body: &[u8]| {
        let wallet = match petal::wallet_param(ctx) {
            Ok(value) => value,
            Err(response) => return response,
        };
        let mut host = crate::workflow::BloomHost::new(ctx);
        match crate::workflow::create(&mut host, wallet, body) {
            Ok(_) => petal::DispatchResponse::Write,
            Err(error) => petal::error(-4, crate::redaction::sanitize_message(&error)),
        }
    }
);
