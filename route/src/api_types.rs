use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AppFee {
    pub recipient: String,
    pub fee: u32,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Rebate {
    pub recipient: String,
    pub share: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuoteRequest {
    pub dry: bool,
    pub deposit_mode: Option<String>,
    pub swap_type: String,
    pub slippage_tolerance: u16,
    pub origin_asset: String,
    pub deposit_type: String,
    pub destination_asset: String,
    pub amount: String,
    pub refund_to: String,
    pub refund_type: String,
    pub recipient: String,
    pub connected_wallets: Option<Vec<String>>,
    pub session_id: Option<String>,
    pub virtual_chain_recipient: Option<String>,
    pub virtual_chain_refund_recipient: Option<String>,
    pub custom_recipient_msg: Option<String>,
    pub recipient_type: String,
    pub deadline: String,
    pub confidentiality: Option<String>,
    pub referral: Option<String>,
    pub rebates: Option<Vec<Rebate>>,
    pub quote_waiting_time_ms: Option<u32>,
    pub app_fees: Option<Vec<AppFee>>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChainDepositAddress {
    pub blockchain: String,
    pub address: String,
    pub memo: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Quote {
    pub deposit_address: Option<String>,
    pub deposit_memo: Option<String>,
    pub chain_deposit_addresses: Option<Vec<ChainDepositAddress>>,
    pub amount_in: String,
    pub amount_in_formatted: String,
    pub amount_in_usd: String,
    pub min_amount_in: String,
    pub amount_out: String,
    pub amount_out_formatted: String,
    pub amount_out_usd: String,
    pub min_amount_out: String,
    pub deadline: Option<String>,
    pub time_when_inactive: Option<String>,
    pub time_estimate: u64,
    pub virtual_chain_recipient: Option<String>,
    pub virtual_chain_refund_recipient: Option<String>,
    pub custom_recipient_msg: Option<String>,
    pub refund_fee: Option<String>,
    pub withdraw_fee: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuoteResponse {
    pub correlation_id: String,
    pub timestamp: String,
    pub signature: String,
    pub quote_request: QuoteRequest,
    pub quote: Quote,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenResponse {
    pub asset_id: String,
    pub decimals: u8,
    pub blockchain: String,
    pub symbol: String,
    pub price: f64,
    pub price_updated_at: String,
    pub contract_address: Option<String>,
    pub coingecko_id: Option<String>,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionStatus {
    pub correlation_id: String,
    pub quote_response: QuoteResponse,
    pub status: String,
    pub updated_at: String,
    #[serde(default)]
    pub swap_details: serde_json::Value,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

pub fn from_slice_no_duplicates<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T, String> {
    use serde::de::{DeserializeSeed, MapAccess, SeqAccess, Visitor};
    struct Json;
    impl<'de> Visitor<'de> for Json {
        type Value = serde_json::Value;
        fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("JSON")
        }
        fn visit_bool<E: serde::de::Error>(self, v: bool) -> Result<Self::Value, E> {
            Ok(v.into())
        }
        fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<Self::Value, E> {
            Ok(v.into())
        }
        fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<Self::Value, E> {
            Ok(v.into())
        }
        fn visit_f64<E: serde::de::Error>(self, v: f64) -> Result<Self::Value, E> {
            serde_json::Number::from_f64(v)
                .map(serde_json::Value::Number)
                .ok_or_else(|| E::custom("non-finite number"))
        }
        fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Self::Value, E> {
            Ok(v.into())
        }
        fn visit_string<E: serde::de::Error>(self, v: String) -> Result<Self::Value, E> {
            Ok(v.into())
        }
        fn visit_none<E: serde::de::Error>(self) -> Result<Self::Value, E> {
            Ok(serde_json::Value::Null)
        }
        fn visit_unit<E: serde::de::Error>(self) -> Result<Self::Value, E> {
            Ok(serde_json::Value::Null)
        }
        fn visit_some<D: serde::Deserializer<'de>>(self, d: D) -> Result<Self::Value, D::Error> {
            d.deserialize_any(Json)
        }
        fn visit_seq<A: SeqAccess<'de>>(self, mut a: A) -> Result<Self::Value, A::Error> {
            let mut v = Vec::new();
            while let Some(x) = a.next_element_seed(Json)? {
                v.push(x)
            }
            Ok(v.into())
        }
        fn visit_map<A: MapAccess<'de>>(self, mut a: A) -> Result<Self::Value, A::Error> {
            let mut out = serde_json::Map::new();
            while let Some(k) = a.next_key::<String>()? {
                if out.contains_key(&k) {
                    return Err(serde::de::Error::custom(format!("duplicate JSON key {k}")));
                }
                let v = a.next_value_seed(Json)?;
                out.insert(k, v);
            }
            Ok(out.into())
        }
    }
    impl<'de> DeserializeSeed<'de> for Json {
        type Value = serde_json::Value;
        fn deserialize<D: serde::Deserializer<'de>>(self, d: D) -> Result<Self::Value, D::Error> {
            d.deserialize_any(self)
        }
    }
    let mut d = serde_json::Deserializer::from_slice(bytes);
    let value =
        serde::de::Deserializer::deserialize_any(&mut d, Json).map_err(|e| e.to_string())?;
    d.end().map_err(|e| e.to_string())?;
    serde_json::from_value(value).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quote_envelope_is_strict_but_tokens_are_forward_compatible() {
        let quote = r#"{"correlationId":"x","timestamp":"t","signature":"s","unknown":1,"quoteRequest":{},"quote":{}}"#;
        assert!(serde_json::from_str::<QuoteResponse>(quote).is_err());
        let token = r#"{"assetId":"a","decimals":18,"blockchain":"eth","symbol":"A","price":1,"priceUpdatedAt":"t","future":true}"#;
        let token: TokenResponse = serde_json::from_str(token).unwrap();
        assert_eq!(token.extra.get("future"), Some(&serde_json::json!(true)));
    }

    #[test]
    fn security_json_rejects_duplicate_keys() {
        let input = br#"{"correlationId":"a","correlationId":"b"}"#;
        assert!(
            from_slice_no_duplicates::<serde_json::Value>(input)
                .unwrap_err()
                .contains("duplicate")
        );
    }
}
