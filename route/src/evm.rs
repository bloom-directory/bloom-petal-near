use crate::assets::validate_address;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct PreparedTransaction {
    pub to: String,
    pub value_wei: String,
    pub data_hex: String,
}

fn word(bytes: &[u8]) -> [u8; 32] {
    let mut out = [0u8; 32];
    out[32 - bytes.len()..].copy_from_slice(bytes);
    out
}

pub fn transfer_calldata(recipient: &str, amount: &str) -> Result<String, String> {
    validate_address(recipient)?;
    if !crate::input::canonical_amount(amount) {
        return Err("transfer amount is invalid".into());
    }
    let address = hex::decode(&recipient[2..]).map_err(|_| "invalid recipient hex")?;
    let amount = alloy::primitives::U256::from_str_radix(amount, 10)
        .map_err(|_| "transfer amount exceeds uint256")?;
    let mut data = Vec::with_capacity(68);
    data.extend_from_slice(&[0xa9, 0x05, 0x9c, 0xbb]);
    data.extend_from_slice(&word(&address));
    data.extend_from_slice(&amount.to_be_bytes::<32>());
    Ok(format!("0x{}", hex::encode(data)))
}

pub fn prepare(
    contract: Option<&str>,
    deposit: &str,
    amount: &str,
) -> Result<PreparedTransaction, String> {
    validate_address(deposit)?;
    if !crate::input::canonical_amount(amount) {
        return Err("deposit amount is invalid".into());
    }
    match contract {
        None => Ok(PreparedTransaction {
            to: deposit.into(),
            value_wei: amount.into(),
            data_hex: "0x".into(),
        }),
        Some(contract) => {
            validate_address(contract)?;
            Ok(PreparedTransaction {
                to: contract.into(),
                value_wei: "0".into(),
                data_hex: transfer_calldata(deposit, amount)?,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn erc20_transfer_vector() {
        assert_eq!(
            transfer_calldata("0x1111111111111111111111111111111111111111", "1").unwrap(),
            "0xa9059cbb00000000000000000000000011111111111111111111111111111111111111110000000000000000000000000000000000000000000000000000000000000001"
        );
    }
}
