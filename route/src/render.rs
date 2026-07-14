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

fn bounded_string(value: &serde_json::Value, max: usize) -> Option<serde_json::Value> {
    value
        .as_str()
        .filter(|value| value.len() <= max)
        .map(|value| serde_json::Value::String(value.into()))
}

fn copy_strings(
    input: &serde_json::Map<String, serde_json::Value>,
    output: &mut serde_json::Map<String, serde_json::Value>,
    names: &[&str],
) {
    for name in names {
        if let Some(value) = input
            .get(*name)
            .and_then(|value| bounded_string(value, 256))
        {
            output.insert((*name).into(), value);
        }
    }
}

fn hash_array(value: &serde_json::Value) -> Option<serde_json::Value> {
    let values = value.as_array()?;
    if values.len() > 64 {
        return None;
    }
    Some(
        values
            .iter()
            .filter_map(|entry| {
                if let Some(hash) = bounded_string(entry, 256) {
                    return Some(hash);
                }
                let entry = entry.as_object()?;
                let hash = bounded_string(entry.get("hash")?, 256)?;
                let mut clean = serde_json::Map::new();
                clean.insert("hash".into(), hash);
                if let Some(url) = entry
                    .get("explorerUrl")
                    .and_then(|value| value.as_str())
                    .filter(|url| url.len() <= 2048 && url.starts_with("https://"))
                {
                    clean.insert("explorerUrl".into(), url.into());
                }
                Some(clean.into())
            })
            .collect::<Vec<_>>()
            .into(),
    )
}

pub fn sanitize_swap_details(value: &serde_json::Value) -> serde_json::Value {
    let Some(input) = value.as_object() else {
        return serde_json::json!({});
    };
    let mut output = serde_json::Map::new();
    copy_strings(
        input,
        &mut output,
        &[
            "amountIn",
            "amountInFormatted",
            "amountInUsd",
            "amountOut",
            "amountOutFormatted",
            "amountOutUsd",
            "depositedAmount",
            "depositedAmountFormatted",
            "depositedAmountUsd",
            "refundedAmount",
            "refundedAmountFormatted",
            "refundedAmountUsd",
            "refundFee",
            "withdrawFee",
        ],
    );
    for name in [
        "intentHashes",
        "nearTxHashes",
        "originChainTxHashes",
        "destinationChainTxHashes",
    ] {
        if let Some(value) = input.get(name).and_then(hash_array) {
            output.insert(name.into(), value);
        }
    }
    if let Some(value) = input.get("refundReason") {
        if value.is_null() {
            output.insert("refundReason".into(), serde_json::Value::Null);
        } else if let Some(value) = bounded_string(value, 512) {
            output.insert("refundReason".into(), value);
        }
    }
    if let Some(value) = input.get("slippage").filter(|value| value.is_number()) {
        output.insert("slippage".into(), value.clone());
    }
    output.into()
}

pub fn sanitize_outbox_receipt_value(value: &serde_json::Value) -> Option<serde_json::Value> {
    let input = value.as_object()?;
    let mut output = serde_json::Map::new();
    copy_strings(
        input,
        &mut output,
        &[
            "transactionHash",
            "blockHash",
            "blockNumber",
            "status",
            "gasUsed",
            "cumulativeGasUsed",
            "effectiveGasPrice",
            "from",
            "to",
            "contractAddress",
            "type",
        ],
    );
    Some(output.into())
}

pub fn sanitize_outbox_receipt(raw: &str) -> Option<serde_json::Value> {
    if raw.len() > 128 * 1024 {
        return None;
    }
    let value: serde_json::Value = serde_json::from_str(raw).ok()?;
    sanitize_outbox_receipt_value(&value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_sanitizers_allowlist_and_bound_fields() {
        let swap = sanitize_swap_details(&serde_json::json!({
            "amountIn":"1",
            "intentHashes":["intent"],
            "destinationChainTxHashes":[{"hash":"0xabc","explorerUrl":"https://example.test/tx","secret":"no"}],
            "unknownSecret":"no"
        }));
        let text = swap.to_string();
        assert!(text.contains("amountIn"));
        assert!(text.contains("explorerUrl"));
        assert!(!text.contains("unknownSecret"));
        assert!(!text.contains("secret"));

        let receipt = sanitize_outbox_receipt(
            r#"{"transactionHash":"0xabc","status":"0x1","logs":[{"data":"secret"}],"unknown":"no"}"#,
        )
        .unwrap();
        let text = receipt.to_string();
        assert!(text.contains("transactionHash"));
        assert!(!text.contains("logs"));
        assert!(!text.contains("unknown"));
    }
}
