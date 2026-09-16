petal::route_file!(spec: petal::http_read_spec(30_000), read: |_ctx: &petal::Ctx| {
    let mut host = crate::workflow::BloomHost;
    match crate::api::tokens(&mut host) {
        Ok((tokens, _raw)) => petal::read_json_value(
            &tokens
                .iter()
                .map(|token| {
                    serde_json::json!({
                        "assetId": token.asset_id,
                        "decimals": token.decimals,
                        "blockchain": token.blockchain,
                        "symbol": token.symbol,
                        "price": token.price,
                        "priceUpdatedAt": token.price_updated_at,
                        "contractAddress": token.contract_address,
                        "execution": if crate::assets::chain_mapping(&token.blockchain).is_some() {
                            "executable"
                        } else {
                            "quote_only"
                        },
                    })
                })
                .collect::<Vec<_>>(),
        ),
        Err(error) => petal::error(-4, error),
    }
});
