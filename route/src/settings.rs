use serde::Serialize;

pub const JWT_KEY: &str = "credentials/partner-jwt";

#[derive(Clone)]
pub struct PartnerJwt(String);
impl std::fmt::Debug for PartnerJwt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PartnerJwt([redacted])")
    }
}
impl PartnerJwt {
    pub fn expose(&self) -> &str {
        &self.0
    }
}

#[derive(Serialize)]
pub struct CredentialStatus {
    pub configured: bool,
    pub storage: &'static str,
    pub encrypted_at_rest: bool,
}

pub fn parse_jwt(body: &[u8]) -> Result<PartnerJwt, String> {
    let text = std::str::from_utf8(body)
        .map_err(|_| "API key must be UTF-8")?
        .trim();
    let token = if text.starts_with('{') {
        #[derive(serde::Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Input {
            jwt: String,
        }
        serde_json::from_str::<Input>(text)
            .map_err(|_| "API key JSON must contain only jwt")?
            .jwt
    } else {
        text.to_string()
    };
    let token = token.trim();
    if token.is_empty()
        || token.len() > 8192
        || token.chars().any(|c| c.is_whitespace() || c.is_control())
    {
        return Err("API key must be 1..=8192 non-whitespace characters".into());
    }
    Ok(PartnerJwt(token.into()))
}

pub fn status(configured: bool) -> CredentialStatus {
    CredentialStatus {
        configured,
        storage: "persistent_private_store",
        encrypted_at_rest: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_without_echoing_on_error() {
        let secret = "top-secret token";
        let err = parse_jwt(secret.as_bytes()).unwrap_err();
        assert!(!err.contains(secret));
        assert_eq!(
            format!("{:?}", PartnerJwt("x".into())),
            "PartnerJwt([redacted])"
        );
    }
}
