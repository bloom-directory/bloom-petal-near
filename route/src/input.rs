use serde::{Deserialize, Serialize};

fn default_slippage() -> u16 {
    100
}
fn default_deadline() -> u32 {
    900
}
fn default_wait() -> u32 {
    3000
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct NewSwapRequest {
    pub swap_type: String,
    pub origin_asset: String,
    pub destination_asset: String,
    pub amount: String,
    pub recipient: String,
    #[serde(default = "default_slippage")]
    pub slippage_bps: u16,
    #[serde(default = "default_deadline")]
    pub deadline_seconds: u32,
    #[serde(default = "default_wait")]
    pub quote_waiting_time_ms: u32,
    #[serde(default)]
    pub refund_to: Option<String>,
}

pub fn canonical_amount(value: &str) -> bool {
    !value.is_empty()
        && value != "0"
        && value.bytes().all(|b| b.is_ascii_digit())
        && !value.starts_with('0')
}

fn safe(value: &str, max: usize) -> bool {
    !value.is_empty() && value.len() <= max && !value.chars().any(char::is_control)
}

impl NewSwapRequest {
    pub fn validate(&self) -> Result<(), String> {
        if !matches!(self.swap_type.as_str(), "EXACT_INPUT" | "EXACT_OUTPUT") {
            return Err("swap_type must be EXACT_INPUT or EXACT_OUTPUT".into());
        }
        if !canonical_amount(&self.amount) {
            return Err("amount must be a positive canonical integer".into());
        }
        if self.slippage_bps > 1000 {
            return Err("slippage_bps must be at most 1000".into());
        }
        if !(300..=3600).contains(&self.deadline_seconds) {
            return Err("deadline_seconds must be 300..=3600".into());
        }
        if self.quote_waiting_time_ms > 10_000 {
            return Err("quote_waiting_time_ms must be at most 10000".into());
        }
        for (name, value) in [
            ("origin_asset", &self.origin_asset),
            ("destination_asset", &self.destination_asset),
            ("recipient", &self.recipient),
        ] {
            if !safe(value, 1024) {
                return Err(format!("{name} is invalid"));
            }
        }
        if let Some(value) = &self.refund_to
            && !safe(value, 128)
        {
            return Err("refund_to is invalid".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn canonical_integer_rules() {
        for bad in ["", "0", "01", "-1", "+1", "1.0", "1e2"] {
            assert!(!canonical_amount(bad), "{bad}");
        }
        assert!(canonical_amount("1"));
        assert!(canonical_amount("1000000000000000000"));
    }
}
