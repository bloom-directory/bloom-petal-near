petal::route_file!(spec: petal::store_read_spec(), read: |_ctx: &petal::Ctx| {
    let mut host = crate::workflow::BloomHost;
    match crate::workflow::credential_status(&mut host) {
        Ok(credential) => petal::read_json_value(&serde_json::json!({
            "credential": credential,
            "endpoint_binding": "oneclick",
            "supported_origins": crate::assets::CHAINS
                .iter()
                .map(|mapping| serde_json::json!({
                    "oneclick": mapping.oneclick,
                    "bloom": mapping.bloom,
                    "chain_id": mapping.chain_id,
                }))
                .collect::<Vec<_>>(),
        })),
        Err(error) => petal::error(-4, error),
    }
});
