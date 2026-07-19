petal::route_file!(spec: petal::store_read_spec(), read: |ctx: &petal::Ctx| {
    let wallet = match petal::param(ctx, "wallet") {
        Ok(value) => value,
        Err(response) => return response,
    };
    let id = match petal::param(ctx, "id") {
        Ok(value) => value,
        Err(response) => return response,
    };
    let mut host = crate::workflow::BloomHost;
    let session = match crate::workflow::load(&mut host, wallet, id) {
        Ok(value) => value,
        Err(error) => return petal::error(-1, error),
    };
    if let Some(outbox) = &session.plan_md {
        return petal::DispatchResponse::Read(
            format!(
                "# NEAR Intents 1Click swap\n\nThe following is Bloom's authoritative transaction plan.\n\n{outbox}"
            )
            .into_bytes(),
        );
    }
    let quote = &session.quote.quote;
    let origin = &session.origin;
    let transaction = session.prepared_transaction.as_ref();
    petal::DispatchResponse::Read(
        format!(
            "# NEAR Intents 1Click swap\n\nFunds will be transferred temporarily into the signed 1Click swapping flow.\n\n- Wallet: `{}` (`{}`)\n- Origin: `{}` (chain ID {})\n- Input: {} {} (`{}` decimals), contract: `{}`\n- Deposit amount: `{}`; informational USD estimate: `{}`\n- Destination asset: `{}`\n- Recipient: `{}`\n- Quoted/min output: `{}` / `{}`\n- Slippage: {} bps\n- Refund address: `{}`; refund fee: `{}`\n- Withdrawal fee: `{}`; estimated execution: {} seconds\n- Signed deposit address: `{}`\n- Quote verified: {}\n- Correlation ID: `{}`\n- Quote hash: `{}`\n- Deadline / inactive time: `{}` / `{}`\n- EVM to/value/data: `{}` / `{}` / `{}`\n\nWarnings: NEAR Intents has no testnet; mainnet broadcasting remains controlled by Bloom opt-in; settlement may take minutes.\n",
            session.wallet,
            session.wallet_address,
            origin.bloom_chain,
            origin.expected_chain_id,
            origin.symbol,
            origin.asset_id,
            origin.decimals,
            origin.contract_address.as_deref().unwrap_or("native"),
            quote.amount_in,
            quote.amount_in_usd,
            session.quote.quote_request.destination_asset,
            session.quote.quote_request.recipient,
            quote.amount_out,
            quote.min_amount_out,
            session.quote.quote_request.slippage_tolerance,
            session.wallet_address,
            quote.refund_fee.as_deref().unwrap_or("not quoted"),
            quote.withdraw_fee.as_deref().unwrap_or("not quoted"),
            quote.time_estimate,
            quote.deposit_address.as_deref().unwrap_or("missing"),
            session.quote_verified,
            session.quote.correlation_id,
            session.quote_hash,
            quote.deadline.as_deref().unwrap_or("missing"),
            quote.time_when_inactive.as_deref().unwrap_or("not provided"),
            transaction
                .map(|value| value.to.as_str())
                .unwrap_or("not prepared"),
            transaction
                .map(|value| value.value_wei.as_str())
                .unwrap_or("not prepared"),
            transaction
                .map(|value| value.data_hex.as_str())
                .unwrap_or("not prepared"),
        )
        .into_bytes(),
    )
});
