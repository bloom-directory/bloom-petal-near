use crate::api_types::TokenResponse;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChainMapping {
    pub oneclick: &'static str,
    pub bloom: &'static str,
    pub chain_id: u64,
}

pub const CHAINS: &[ChainMapping] = &[
    ChainMapping {
        oneclick: "eth",
        bloom: "ethereum",
        chain_id: 1,
    },
    ChainMapping {
        oneclick: "base",
        bloom: "base",
        chain_id: 8453,
    },
    ChainMapping {
        oneclick: "arb",
        bloom: "arbitrum",
        chain_id: 42161,
    },
    ChainMapping {
        oneclick: "op",
        bloom: "optimism",
        chain_id: 10,
    },
    ChainMapping {
        oneclick: "pol",
        bloom: "polygon",
        chain_id: 137,
    },
    ChainMapping {
        oneclick: "bsc",
        bloom: "bsc",
        chain_id: 56,
    },
    ChainMapping {
        oneclick: "avax",
        bloom: "avalanche",
        chain_id: 43114,
    },
    ChainMapping {
        oneclick: "gnosis",
        bloom: "gnosis",
        chain_id: 100,
    },
];

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResolvedOrigin {
    pub asset_id: String,
    pub symbol: String,
    pub decimals: u8,
    pub blockchain: String,
    pub bloom_chain: String,
    pub expected_chain_id: u64,
    pub contract_address: Option<String>,
    pub amount_in_usd: Option<String>,
}

pub fn chain_mapping(blockchain: &str) -> Option<ChainMapping> {
    CHAINS.iter().copied().find(|m| m.oneclick == blockchain)
}

pub fn resolve(tokens: &[TokenResponse], asset_id: &str) -> Result<ResolvedOrigin, String> {
    let matching: Vec<_> = tokens.iter().filter(|t| t.asset_id == asset_id).collect();
    if matching.len() != 1 {
        return Err(format!(
            "origin asset must match exactly one token record (matched {})",
            matching.len()
        ));
    }
    let token = matching[0];
    let mapping = chain_mapping(&token.blockchain)
        .ok_or_else(|| "origin blockchain is not executable in Bloom".to_string())?;
    if let Some(address) = &token.contract_address {
        validate_address(address)?;
    }
    Ok(ResolvedOrigin {
        asset_id: token.asset_id.clone(),
        symbol: token.symbol.clone(),
        decimals: token.decimals,
        blockchain: token.blockchain.clone(),
        bloom_chain: mapping.bloom.into(),
        expected_chain_id: mapping.chain_id,
        contract_address: token.contract_address.clone(),
        amount_in_usd: None,
    })
}

pub fn validate_address(address: &str) -> Result<(), String> {
    let raw = address
        .strip_prefix("0x")
        .ok_or("EVM address must start with 0x")?;
    if raw.len() != 40 || !raw.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("EVM address must contain exactly 20 hex bytes".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mapping_is_fixed() {
        assert_eq!(chain_mapping("arb").unwrap().chain_id, 42161);
        assert!(chain_mapping("scroll").is_none());
    }
}
