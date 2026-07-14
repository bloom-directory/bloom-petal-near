use crate::session::Session;

pub fn plan(session: &Session) -> String {
    if let Some(outbox) = &session.plan_md {
        return format!(
            "# NEAR Intents 1Click swap\n\nThe following is Bloom's authoritative transaction plan.\n\n{outbox}"
        );
    }
    let q = &session.quote.quote;
    let token = &session.origin;
    let tx = session.prepared_transaction.as_ref();
    format!(
        "# NEAR Intents 1Click swap\n\nFunds will be transferred temporarily into the signed 1Click swapping flow.\n\n- Wallet: `{}` (`{}`)\n- Origin: `{}` (chain ID {})\n- Input: {} {} (`{}` decimals), contract: `{}`\n- Deposit amount: `{}`; informational USD estimate: `{}`\n- Destination asset: `{}`\n- Recipient: `{}`\n- Quoted/min output: `{}` / `{}`\n- Slippage: {} bps\n- Refund address: `{}`; refund fee: `{}`\n- Withdrawal fee: `{}`; estimated execution: {} seconds\n- Signed deposit address: `{}`\n- Quote verified: {}\n- Correlation ID: `{}`\n- Quote hash: `{}`\n- Deadline / inactive time: `{}` / `{}`\n- EVM to/value/data: `{}` / `{}` / `{}`\n\nWarnings: NEAR Intents has no testnet; mainnet broadcasting remains controlled by Bloom opt-in; settlement may take minutes.\n",
        session.wallet,
        session.wallet_address,
        token.bloom_chain,
        token.expected_chain_id,
        token.symbol,
        token.asset_id,
        token.decimals,
        token.contract_address.as_deref().unwrap_or("native"),
        q.amount_in,
        q.amount_in_usd,
        session.quote.quote_request.destination_asset,
        session.quote.quote_request.recipient,
        q.amount_out,
        q.min_amount_out,
        session.quote.quote_request.slippage_tolerance,
        session.wallet_address,
        q.refund_fee.as_deref().unwrap_or("not quoted"),
        q.withdraw_fee.as_deref().unwrap_or("not quoted"),
        q.time_estimate,
        q.deposit_address.as_deref().unwrap_or("missing"),
        session.quote_verified,
        session.quote.correlation_id,
        session.quote_hash,
        q.deadline.as_deref().unwrap_or("missing"),
        q.time_when_inactive.as_deref().unwrap_or("not provided"),
        tx.map(|x| x.to.as_str()).unwrap_or("not prepared"),
        tx.map(|x| x.value_wei.as_str()).unwrap_or("not prepared"),
        tx.map(|x| x.data_hex.as_str()).unwrap_or("not prepared")
    )
}

pub fn public_status(session: &Session) -> serde_json::Value {
    serde_json::json!({"id":session.id,"wallet":session.wallet,"state":session.state,"updated_ms":session.updated_ms,"quote_verified":session.quote_verified,"outbox_id":session.outbox_id,"outbox_state":session.outbox_state,"origin_tx_hash":session.origin_tx_hash,"upstream_status":session.upstream_status,"last_error":session.last_error,"history":session.history})
}
