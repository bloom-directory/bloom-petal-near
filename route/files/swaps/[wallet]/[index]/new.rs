petal::route_file!(
    spec: petal::write_spec().caps(&["bloom:http", "bloom:store", "bloom:chain", "bloom:vfs.read"]),
    read: |_ctx: &petal::Ctx| {
        petal::DispatchResponse::Read(
            br#"Write one JSON object to request a quote. Example, exactly 1 USDC from Ethereum to Arbitrum:
{"session_id":"usdc-eth-arb-1","swap_type":"EXACT_INPUT","origin_asset":"nep141:eth-0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48.omft.near","destination_asset":"nep141:arb-0xaf88d065e77c8cc2239327c5edb3a432268e5831.omft.near","amount":"1000000","recipient":"<destination address>"}
amount is in base units: USDC has 6 decimals, so 1 USDC is "1000000". Asset IDs and decimals are in /petals/near-intents/tokens.json.
Only the origin chain must be configured in Bloom; the destination can be any chain 1Click supports.
Success: <session_id>/status.json shows state "quoted". On "quote_failed", read last_error and retry with a new session_id.
Next: write confirm to <session_id>/confirm twice (quoted -> prepared -> staged). staged is the owner's approval point; stop there unless the user asked to submit.
"#
            .to_vec(),
        )
    },
    write: |ctx: &petal::Ctx, body: &[u8]| {
        let wallet = match petal::wallet_param(ctx) {
            Ok(value) => value,
            Err(response) => return response,
        };
        let account = match crate::account_number(ctx) {
            Ok(value) => value,
            Err(response) => return response,
        };
        let mut host = crate::workflow::BloomHost;
        match crate::workflow::create_for_account(&mut host, wallet, account, body) {
            Ok(_) => petal::DispatchResponse::Write,
            Err(error) => petal::error(-4, crate::redaction::sanitize_message(&error)),
        }
    }
);
