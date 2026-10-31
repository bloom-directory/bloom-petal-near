petal::route_file!(
    spec: petal::write_spec().caps(&["bloom:http", "bloom:store", "bloom:chain", "bloom:vfs.read"]),
    read: |_ctx: &petal::Ctx| {
        petal::DispatchResponse::Read(
            br#"Write one JSON object to request a quote. Example, exactly 1 USDC from Ethereum to Arbitrum:
{"session_id":"usdc-eth-arb-1","swap_type":"EXACT_INPUT","origin_asset":"nep141:eth-0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48.omft.near","destination_asset":"nep141:arb-0xaf88d065e77c8cc2239327c5edb3a432268e5831.omft.near","amount":"1000000","recipient":"<destination address>"}
amount is in base units. EXACT_INPUT: units of origin_asset (amount sent). EXACT_OUTPUT: units of destination_asset (amount received). USDC has 6 decimals, so 1 USDC is "1000000"; to receive exactly 0.01 ETH (18 decimals) with EXACT_OUTPUT, amount is "10000000000000000". Asset IDs and decimals are in /petals/near-intents/tokens.json.
The origin must be an asset tokens.json marks "execution": "executable" (an EVM chain this Petal maps) on a chain configured in Bloom; a "quote_only" origin fails. The destination can be any chain 1Click supports.
Optional: slippage_bps (default 100, at most 1000), deadline_seconds (300 to 3600, default 900), refund_to.
Success: <session_id>/status.json shows state "quoted". On "quote_failed", read last_error and retry with a new session_id.
Next: write confirm to <session_id>/confirm twice (quoted -> prepared -> staged). staged is where the user decides; nothing has been asked of the owner yet. The next confirm asks the owner to approve.
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
