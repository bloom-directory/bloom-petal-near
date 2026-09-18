petal::route_file!(spec: petal::store_read_spec(), read: |ctx: &petal::Ctx| {
    let wallet = match petal::wallet_param(ctx) {
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
    let venue_policy = session
        .policy_checks
        .as_ref()
        .and_then(|checks| checks.as_array())
        .map(|checks| {
            checks
                .iter()
                .map(|entry| {
                    format!(
                        "- **{}** ({}): {}\n",
                        entry.get("rule").and_then(|v| v.as_str()).unwrap_or("?"),
                        entry.get("outcome").and_then(|v| v.as_str()).unwrap_or("?"),
                        entry.get("message").and_then(|v| v.as_str()).unwrap_or(""),
                    )
                })
                .collect::<String>()
        })
        .unwrap_or_else(|| "- not evaluated\n".into());
    let transaction = session.prepared_transaction.as_ref();
    let app_fee = session
        .quote
        .quote_request
        .app_fees
        .as_deref()
        .and_then(|fees| fees.first())
        .map(|fee| {
            format!(
                "{} bps to `{}`",
                fee.fee,
                fee.recipient.replace('\\', "\\\\").replace('`', "\\`")
            )
        })
        .unwrap_or_else(|| "none".into());
    petal::DispatchResponse::Read(
        format!(
            "# NEAR Intents 1Click swap\n\nFunds will be transferred temporarily into the signed 1Click swapping flow.\n\n- Wallet: `{}` (`{}`)\n- Account number: `{}`\n- Origin: `{}` (chain ID {})\n- Input: {} {} (`{}` decimals), contract: `{}`\n- Deposit amount: `{}`; informational USD estimate: `{}`\n- Upstream application fee: {}\n- Destination asset: `{}`\n- Recipient: `{}`\n- Quoted/min output: `{}` / `{}`\n- Slippage: {} bps\n- Refund address: `{}`; refund fee: `{}`\n- Withdrawal fee: `{}`; estimated execution: {} seconds\n- Signed deposit address: `{}`\n- Quote verified: {}\n- Quote credential: {}\n- Correlation ID: `{}`\n- Quote hash: `{}`\n- Deadline / inactive time: `{}` / `{}`\n- EVM to/value/data: `{}` / `{}` / `{}`\n\n## Venue policy\n\nThis wallet's own rules, at `settings/wallets/<wallet>/venue.toml`. They are\nguard rails the Petal enforces; Bloom's wallet policy still decides what may be\nsigned.\n\n{}\nWarnings: NEAR Intents has no testnet; mainnet broadcasting remains controlled by Bloom opt-in; settlement may take minutes.\n",
            session.wallet,
            session.wallet_address,
            session.account.as_ref().map(|a| a.number.to_string()).unwrap_or_else(|| "unbound legacy session".into()),
            origin.bloom_chain,
            origin.expected_chain_id,
            origin.symbol,
            origin.asset_id,
            origin.decimals,
            origin.contract_address.as_deref().unwrap_or("native"),
            quote.amount_in,
            quote.amount_in_usd,
            app_fee,
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
            match session.quote_authenticated {
                Some(true) => "partner JWT (partner fee schedule)",
                Some(false) => "none; 1Click charges its unauthenticated platform fee",
                None => "not recorded",
            },
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
            venue_policy,
        )
        .into_bytes(),
    )
});
