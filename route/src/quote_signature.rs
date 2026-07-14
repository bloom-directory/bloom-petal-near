use crate::api_types::QuoteResponse;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

pub const MANAGER_PUBLIC_KEY: &str = "ed25519:reYaWhvwu8Jzo3WUM3zhn6VrhuMEF4eADL17qtRVifc";

// Exact projection port of defuse-protocol/one-click-sdk-typescript
// src/quote-signature.ts at ae28ef0348f616dd30c174cb22dd1b1126d8f76b.

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum VerifyError {
    #[error("invalid base58 encoding")]
    Base58,
    #[error("invalid Ed25519 key or signature length")]
    Length,
    #[error("quote signature verification failed")]
    InvalidSignature,
    #[error("quote projection cannot be serialized")]
    Serialization,
}

fn truthy_string(map: &mut BTreeMap<String, Value>, key: &str, value: &Option<String>) {
    map.remove(key);
    if let Some(value) = value.as_ref().filter(|v| !v.is_empty()) {
        map.insert(key.into(), Value::String(value.clone()));
    }
}

fn required<T: serde::Serialize>(map: &mut BTreeMap<String, Value>, key: &str, value: &T) {
    map.insert(
        key.into(),
        serde_json::to_value(value).expect("scalar serialization"),
    );
}

pub fn signed_projection(response: &QuoteResponse) -> Value {
    let r = &response.quote_request;
    let q = &response.quote;
    let mut p = BTreeMap::new();
    required(&mut p, "dry", &r.dry);
    required(&mut p, "swapType", &r.swap_type);
    required(&mut p, "slippageTolerance", &r.slippage_tolerance);
    required(&mut p, "originAsset", &r.origin_asset);
    required(&mut p, "depositType", &r.deposit_type);
    required(&mut p, "destinationAsset", &r.destination_asset);
    required(&mut p, "amount", &r.amount);
    required(&mut p, "refundTo", &r.refund_to);
    required(&mut p, "refundType", &r.refund_type);
    required(&mut p, "recipient", &r.recipient);
    required(&mut p, "recipientType", &r.recipient_type);
    required(&mut p, "deadline", &r.deadline);
    if r.quote_waiting_time_ms.unwrap_or(0) != 0 {
        required(
            &mut p,
            "quoteWaitingTimeMs",
            &r.quote_waiting_time_ms.unwrap(),
        );
    }
    truthy_string(&mut p, "referral", &r.referral);
    truthy_string(&mut p, "virtualChainRecipient", &r.virtual_chain_recipient);
    truthy_string(
        &mut p,
        "virtualChainRefundRecipient",
        &r.virtual_chain_refund_recipient,
    );
    truthy_string(&mut p, "customRecipientMsg", &r.custom_recipient_msg);

    for (key, value) in [
        ("amountIn", &q.amount_in),
        ("amountInFormatted", &q.amount_in_formatted),
        ("amountInUsd", &q.amount_in_usd),
        ("minAmountIn", &q.min_amount_in),
        ("amountOut", &q.amount_out),
        ("amountOutFormatted", &q.amount_out_formatted),
        ("amountOutUsd", &q.amount_out_usd),
        ("minAmountOut", &q.min_amount_out),
    ] {
        required(&mut p, key, value);
    }
    if !r.dry {
        truthy_string(&mut p, "depositAddress", &q.deposit_address);
        truthy_string(&mut p, "depositMemo", &q.deposit_memo);
        truthy_string(&mut p, "deadline", &q.deadline);
        truthy_string(&mut p, "timeWhenInactive", &q.time_when_inactive);
        p.remove("timeEstimate");
        if q.time_estimate != 0 {
            required(&mut p, "timeEstimate", &q.time_estimate);
        }
        truthy_string(&mut p, "virtualChainRecipient", &q.virtual_chain_recipient);
        truthy_string(
            &mut p,
            "virtualChainRefundRecipient",
            &q.virtual_chain_refund_recipient,
        );
        truthy_string(&mut p, "customRecipientMsg", &q.custom_recipient_msg);
        truthy_string(&mut p, "refundFee", &q.refund_fee);
        truthy_string(&mut p, "withdrawFee", &q.withdraw_fee);
    }
    required(&mut p, "timestamp", &response.timestamp);
    Value::Object(Map::from_iter(p))
}

pub fn quote_hash(response: &QuoteResponse) -> Result<String, VerifyError> {
    let encoded =
        serde_json::to_vec(&signed_projection(response)).map_err(|_| VerifyError::Serialization)?;
    Ok(bs58::encode(Sha256::digest(encoded)).into_string())
}

fn decode(value: &str) -> Result<Vec<u8>, VerifyError> {
    bs58::decode(value.strip_prefix("ed25519:").unwrap_or(value))
        .into_vec()
        .map_err(|_| VerifyError::Base58)
}

pub fn verify_with_key(response: &QuoteResponse, public_key: &str) -> Result<String, VerifyError> {
    let key: [u8; 32] = decode(public_key)?
        .try_into()
        .map_err(|_| VerifyError::Length)?;
    let sig: [u8; 64] = decode(&response.signature)?
        .try_into()
        .map_err(|_| VerifyError::Length)?;
    let hash = quote_hash(response)?;
    VerifyingKey::from_bytes(&key)
        .map_err(|_| VerifyError::Length)?
        .verify(hash.as_bytes(), &Signature::from_bytes(&sig))
        .map_err(|_| VerifyError::InvalidSignature)?;
    Ok(hash)
}

pub fn verify(response: &QuoteResponse) -> Result<String, VerifyError> {
    verify_with_key(response, MANAGER_PUBLIC_KEY)
}

#[cfg(test)]
mod tests {
    use super::*;

    const STAGING_KEY: &str = "ed25519:5J5tkaxyPoR3Q9S8LXfo5bWnXK5Z2bctJ4mB9gENh7co";
    const FIXTURE: &str = r#"{
      "correlationId":"d4f1b110-46cc-4682-aa3f-44d81ffe4b80","timestamp":"2026-06-23T17:10:41.104Z",
      "signature":"ed25519:53wcpim7FDNLbBHVezUpakthWq2TR9Lag3PwW3e8Cxmz4bFEodcc4rui5BiVHRRaHocYE9URVapzJD8JxLNDs8K9",
      "quoteRequest":{"dry":false,"depositMode":"SIMPLE","swapType":"EXACT_INPUT","slippageTolerance":100,"originAsset":"1cs_v1:btc:native:coin","depositType":"ORIGIN_CHAIN","destinationAsset":"nep141:eth-0xdac17f958d2ee523a2206206994597c13d831ec7.stft.near","amount":"10000","refundTo":"bc1q6mte80265ghwq4vsrpm9lnaz46uvdreu9z8wly","refundType":"ORIGIN_CHAIN","recipient":"0xcac3C41676deF4FE375E57118f3eB83A99105577","recipientType":"DESTINATION_CHAIN","deadline":"2026-06-23T19:00:00.000Z","confidentiality":"public","quoteWaitingTimeMs":0,"appFees":[{"recipient":"5880ad2b362620fadf759cbceb1cd5737ce8c6ed7fb8e9942881e6731f9247dd","fee":10}]},
      "quote":{"amountIn":"10000","amountInFormatted":"0.0001","amountInUsd":"6.237600000000","minAmountIn":"10000","amountOut":"5931560","amountOutFormatted":"5.93156","amountOutUsd":"5.925171709880","minAmountOut":"5872244","timeEstimate":812,"refundFee":"1900","withdrawFee":"300000","deadline":"2026-06-26T19:00:00.000Z","timeWhenInactive":"2026-06-26T19:00:00.000Z","depositAddress":"bc1q873cxltdc560dth6tpwqpehq9uvhxxcdgwnmnw"}}
    "#;

    #[test]
    fn verifies_official_sdk_fixture() {
        let q: QuoteResponse = serde_json::from_str(FIXTURE).unwrap();
        assert!(verify_with_key(&q, STAGING_KEY).is_ok());
    }

    #[test]
    fn quote_side_none_overwrites_request_deadline_and_zero_is_omitted() {
        let mut q: QuoteResponse = serde_json::from_str(FIXTURE).unwrap();
        q.quote.deadline = None;
        q.quote.time_estimate = 0;
        let p = signed_projection(&q);
        assert!(p.get("deadline").is_none());
        assert!(p.get("quoteWaitingTimeMs").is_none());
        assert!(p.get("timeEstimate").is_none());
    }

    #[test]
    fn upstream_insured_default_is_not_in_the_sdk_signed_projection() {
        let mut q: QuoteResponse = serde_json::from_str(FIXTURE).unwrap();
        let original = signed_projection(&q);
        q.quote_request.insured = Some(false);
        assert_eq!(signed_projection(&q), original);
        assert!(signed_projection(&q).get("insured").is_none());
    }

    #[test]
    fn tampering_and_bad_lengths_fail_closed() {
        let mut q: QuoteResponse = serde_json::from_str(FIXTURE).unwrap();
        q.quote.deposit_address = Some("tampered".into());
        assert_eq!(
            verify_with_key(&q, STAGING_KEY),
            Err(VerifyError::InvalidSignature)
        );
        q.signature = "ed25519:1".into();
        assert_eq!(verify_with_key(&q, STAGING_KEY), Err(VerifyError::Length));
    }

    #[test]
    fn every_execution_binding_mutation_invalidates_the_signature() {
        let original: QuoteResponse = serde_json::from_str(FIXTURE).unwrap();
        let mut mutations = Vec::new();
        let mut q = original.clone();
        q.quote.amount_in = "10001".into();
        mutations.push(q);
        let mut q = original.clone();
        q.quote_request.recipient = "other".into();
        mutations.push(q);
        let mut q = original.clone();
        q.quote_request.refund_to = "other".into();
        mutations.push(q);
        let mut q = original.clone();
        q.quote.deadline = Some("2027-01-01T00:00:00Z".into());
        mutations.push(q);
        let mut q = original.clone();
        q.timestamp = "2027-01-01T00:00:00Z".into();
        mutations.push(q);
        for q in mutations {
            assert_eq!(
                verify_with_key(&q, STAGING_KEY),
                Err(VerifyError::InvalidSignature)
            );
        }
        let mut malformed = original;
        malformed.signature = "ed25519:not-base58!".into();
        assert_eq!(
            verify_with_key(&malformed, STAGING_KEY),
            Err(VerifyError::Base58)
        );
    }
}
