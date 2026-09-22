petal::route_file!(
    spec: petal::write_spec().caps(&["bloom:store"]),
    read: |ctx: &petal::Ctx| {
        let wallet = match petal::wallet_param(ctx) {
            Ok(value) => value,
            Err(response) => return response,
        };
        let mut host = crate::workflow::BloomHost;
        match crate::policy::read_bytes(&mut host, wallet) {
            Ok(bytes) => petal::DispatchResponse::Read(bytes),
            Err(error) => petal::error(-4, crate::redaction::sanitize_message(&error)),
        }
    },
    write: |ctx: &petal::Ctx, body: &[u8]| {
        let wallet = match petal::wallet_param(ctx) {
            Ok(value) => value,
            Err(response) => return response,
        };
        let mut host = crate::workflow::BloomHost;
        match crate::policy::write(&mut host, wallet, body) {
            Ok(()) => petal::DispatchResponse::Write,
            Err(error) => petal::error(-3, crate::redaction::sanitize_message(&error)),
        }
    }
);
