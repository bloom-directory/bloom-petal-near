petal::route_file!(
    spec: petal::write_spec().caps(&["bloom:store", "bloom:vfs.read"]),
    read: |_ctx: &petal::Ctx| {
        petal::DispatchResponse::Read(
            b"write abandon before any outbox transaction exists\n".to_vec(),
        )
    },
    write: |ctx: &petal::Ctx, _body: &[u8]| {
        let wallet = match petal::wallet_param(ctx) {
            Ok(value) => value,
            Err(response) => return response,
        };
        let id = match petal::param(ctx, "id") {
            Ok(value) => value,
            Err(response) => return response,
        };
        let mut host = crate::workflow::BloomHost::new(ctx);
        match crate::workflow::abandon(&mut host, wallet, id) {
            Ok(()) => petal::DispatchResponse::Write,
            Err(error) => petal::error(-4, crate::redaction::sanitize_message(&error)),
        }
    }
);
