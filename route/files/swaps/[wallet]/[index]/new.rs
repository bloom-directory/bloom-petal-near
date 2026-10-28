petal::route_file!(
    spec: petal::write_spec().caps(&["bloom:http", "bloom:store", "bloom:chain", "bloom:vfs.read"]),
    read: |_ctx: &petal::Ctx| {
        // The complete request shape and the steps after it, so an agent
        // never has to read this package's source to swap.
        petal::DispatchResponse::Read(
            br#"Write JSON to request a 1Click quote. It creates <session_id>/ beside this file with status.json state "quoted"; nothing moves until you confirm.
{"session_id":"<your own id>","swap_type":"EXACT_INPUT","origin_asset":"nep141:eth-0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48.omft.near","destination_asset":"nep141:arb-0xaf88d065e77c8cc2239327c5edb3a432268e5831.omft.near","amount":"1000000","recipient":"0x<address on the destination chain>"}
origin_asset, destination_asset: assetId values from ../../../tokens.json; the origin must be an EVM asset this account holds.
amount: an integer in the origin asset's smallest unit (decimals in tokens.json), so "1000000" is 1 USDC.
recipient: the address that receives on the destination chain, usually this account's own address.
Optional: slippage_bps (default 100, at most 1000), deadline_seconds (300 to 3600, default 900), refund_to.
Then write "confirm" to <session_id>/confirm to prepare the deposit (state "prepared"), and once more to stage it in this account's outbox (state "staged"); read <session_id>/status.json after each write. The staged outbox entry needs the owner's approval like any transfer.
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
