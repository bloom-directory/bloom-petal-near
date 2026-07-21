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
            "outcome",
            "tx_hash",
            "revert_reason",
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
    if let Some(value) = input.get("block_number").filter(|value| value.is_u64()) {
        output.insert("block_number".into(), value.clone());
    }
    if input
        .get("revert_reason")
        .is_some_and(|value| value.is_null())
    {
        output.insert("revert_reason".into(), serde_json::Value::Null);
    }
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
            r#"{"outcome":"success","tx_hash":"0xabc","block_number":42,"revert_reason":null,"logs":[{"data":"secret"}],"unknown":"no"}"#,
        )
        .unwrap();
        let text = receipt.to_string();
        assert!(text.contains("\"outcome\":\"success\""));
        assert!(text.contains("\"tx_hash\":\"0xabc\""));
        assert!(text.contains("\"block_number\":42"));
        assert!(text.contains("\"revert_reason\":null"));
        assert!(!text.contains("logs"));
        assert!(!text.contains("unknown"));
    }
}
