use serde::Serialize;

pub const JWT_KEY: &str = "credentials/partner-jwt";
const EMBEDDED_PARTNER_JWT: Option<&str> = option_env!("NEAR_INTENTS_PARTNER_JWT");

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

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CredentialSource {
    PrivateStore,
    EmbeddedPartner,
    Unconfigured,
}

#[derive(Debug)]
pub struct ResolvedPartnerJwt {
    pub jwt: PartnerJwt,
    pub source: CredentialSource,
}

#[derive(Debug, Serialize)]
pub struct CredentialStatus {
    pub configured: bool,
    pub source: CredentialSource,
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

pub fn resolve_partner_jwt(
    private_store: Option<&[u8]>,
    embedded: Option<&str>,
) -> Result<ResolvedPartnerJwt, String> {
    if let Some(raw) = private_store {
        return Ok(ResolvedPartnerJwt {
            jwt: parse_jwt(raw)?,
            source: CredentialSource::PrivateStore,
        });
    }
    if let Some(raw) = embedded {
        return Ok(ResolvedPartnerJwt {
            jwt: parse_jwt(raw.as_bytes())?,
            source: CredentialSource::EmbeddedPartner,
        });
    }
    Err("1Click API key is not configured".into())
}

pub fn configured_partner_jwt(private_store: Option<&[u8]>) -> Result<ResolvedPartnerJwt, String> {
    resolve_partner_jwt(private_store, EMBEDDED_PARTNER_JWT)
}

pub fn status(private_store: Option<&[u8]>, embedded: Option<&str>) -> CredentialStatus {
    let (configured, source, storage) = if let Some(raw) = private_store {
        (
            parse_jwt(raw).is_ok(),
            CredentialSource::PrivateStore,
            "persistent_private_store",
        )
    } else if let Some(raw) = embedded {
        (
            parse_jwt(raw.as_bytes()).is_ok(),
            CredentialSource::EmbeddedPartner,
            "release_artifact",
        )
    } else {
        (false, CredentialSource::Unconfigured, "none")
    };
    CredentialStatus {
        configured,
        source,
        storage,
        encrypted_at_rest: false,
    }
}

pub fn configured_status(private_store: Option<&[u8]>) -> CredentialStatus {
    status(private_store, EMBEDDED_PARTNER_JWT)
}

#[cfg(test)]
mod tests {
    use super::*;
    const TEST_TOKEN: &str = "test.jwt.must-never-appear";

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

    #[test]
    fn embedded_fallback_succeeds() {
        let resolved = resolve_partner_jwt(None, Some(TEST_TOKEN)).unwrap();
        assert_eq!(resolved.source, CredentialSource::EmbeddedPartner);
        assert_eq!(resolved.jwt.expose(), TEST_TOKEN);
    }

    #[test]
    fn private_store_wins_over_embedded_fallback() {
        let resolved =
            resolve_partner_jwt(Some(b"private.jwt.override"), Some(TEST_TOKEN)).unwrap();
        assert_eq!(resolved.source, CredentialSource::PrivateStore);
        assert_eq!(resolved.jwt.expose(), "private.jwt.override");
    }

    #[test]
    fn missing_credentials_fails() {
        assert_eq!(
            resolve_partner_jwt(None, None).unwrap_err(),
            "1Click API key is not configured"
        );
    }

    #[test]
    fn malformed_embedded_value_fails_without_echoing_it() {
        let malformed = "test.jwt.must-never-appear malformed";
        let error = resolve_partner_jwt(None, Some(malformed)).unwrap_err();
        assert!(!error.contains(malformed));
        assert!(!format!("{:?}", resolve_partner_jwt(None, Some(malformed))).contains(malformed));
    }

    #[test]
    fn debug_and_public_status_never_reveal_token() {
        let resolved = resolve_partner_jwt(None, Some(TEST_TOKEN)).unwrap();
        let embedded_status = status(None, Some(TEST_TOKEN));
        let private_status = status(Some(TEST_TOKEN.as_bytes()), Some("fallback.jwt"));
        let unconfigured_status = status(None, None);
        let public_route_projection = serde_json::json!({
            "credential": &embedded_status,
            "endpoint_binding": "oneclick",
        });
        let output = format!(
            "{resolved:?}\n{embedded_status:?}\n{}\n{}\n{public_route_projection}",
            serde_json::to_string(&embedded_status).unwrap(),
            serde_json::to_string(&private_status).unwrap()
        );
        assert!(!output.contains(TEST_TOKEN));
        assert!(output.contains("embedded_partner"));
        assert!(output.contains("release_artifact"));
        assert_eq!(private_status.source, CredentialSource::PrivateStore);
        assert_eq!(private_status.storage, "persistent_private_store");
        assert!(private_status.configured);
        assert_eq!(unconfigured_status.source, CredentialSource::Unconfigured);
        assert_eq!(unconfigured_status.storage, "none");
        assert!(!unconfigured_status.configured);
    }

    #[test]
    fn malformed_candidates_return_structured_status_but_execution_stays_strict() {
        let malformed_private = "private.test.jwt must-never-appear";
        let malformed_embedded = "embedded.test.jwt must-never-appear";

        let private_status = status(
            Some(malformed_private.as_bytes()),
            Some("valid.embedded.fallback"),
        );
        assert!(!private_status.configured);
        assert_eq!(private_status.source, CredentialSource::PrivateStore);
        assert_eq!(private_status.storage, "persistent_private_store");
        assert!(
            resolve_partner_jwt(
                Some(malformed_private.as_bytes()),
                Some("valid.embedded.fallback")
            )
            .is_err()
        );

        let embedded_status = status(None, Some(malformed_embedded));
        assert!(!embedded_status.configured);
        assert_eq!(embedded_status.source, CredentialSource::EmbeddedPartner);
        assert_eq!(embedded_status.storage, "release_artifact");
        assert!(resolve_partner_jwt(None, Some(malformed_embedded)).is_err());

        let public_output = serde_json::to_string(&(private_status, embedded_status)).unwrap();
        assert!(!public_output.contains(malformed_private));
        assert!(!public_output.contains(malformed_embedded));
    }
}
