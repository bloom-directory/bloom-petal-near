petal::route_file!(
    spec: petal::write_spec().caps(&["bloom:store"]),
    read: |_ctx: &petal::Ctx| {
        let mut host = crate::workflow::BloomHost;
        match crate::workflow::credential_status(&mut host) {
            Ok(status) => petal::read_json_value(&status),
            Err(error) => petal::error(-4, error),
        }
    },
    write: |_ctx: &petal::Ctx, body: &[u8]| {
        let mut host = crate::workflow::BloomHost;
        match crate::workflow::write_api_key(&mut host, body) {
            Ok(()) => petal::DispatchResponse::Write,
            Err(error) => petal::error(-4, crate::redaction::sanitize_message(&error)),
        }
    }
);
