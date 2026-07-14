use crate::{
    api_types::{ExecutionStatus, QuoteRequest, QuoteResponse, TokenResponse},
    settings::PartnerJwt,
};

pub const DEFAULT_ORIGIN: &str = "https://1click.chaindefuser.com";
pub const MAX_HTTP_BYTES: usize = 2 * 1024 * 1024;

pub fn endpoint<H: crate::workflow::Host>(host: &mut H) -> String {
    host.setting("endpoint.oneclick")
        .ok()
        .flatten()
        .unwrap_or_else(|| DEFAULT_ORIGIN.into())
}

fn request<H: crate::workflow::Host>(
    host: &mut H,
    method: &str,
    path: &str,
    jwt: Option<&PartnerJwt>,
    body: Vec<u8>,
) -> Result<Vec<u8>, String> {
    let mut headers = vec![("accept".into(), "application/json".into())];
    if !body.is_empty() {
        headers.push(("content-type".into(), "application/json".into()));
    }
    if let Some(jwt) = jwt {
        headers.push(("authorization".into(), format!("Bearer {}", jwt.expose())));
    }
    let origin = endpoint(host);
    let response = host.http(
        petal::sdk::HttpRequest {
            method: method.into(),
            url: format!("{}{path}", origin.trim_end_matches('/')),
            headers,
            body,
        },
        MAX_HTTP_BYTES,
    )?;
    if !(200..300).contains(&response.status) {
        return Err(format!(
            "1Click HTTP {}: {}",
            response.status,
            crate::redaction::sanitize_message(&String::from_utf8_lossy(&response.body))
        ));
    }
    Ok(response.body)
}

pub fn tokens<H: crate::workflow::Host>(
    host: &mut H,
) -> Result<(Vec<TokenResponse>, Vec<u8>), String> {
    let raw = request(host, "GET", "/v0/tokens", None, vec![])?;
    let tokens: Vec<TokenResponse> =
        serde_json::from_slice(&raw).map_err(|e| format!("tokens response: {e}"))?;
    if tokens.len() > 10_000 {
        return Err("tokens response exceeds 10000 records".into());
    }
    Ok((tokens, raw))
}

pub fn quote<H: crate::workflow::Host>(
    host: &mut H,
    jwt: &PartnerJwt,
    body: &QuoteRequest,
) -> Result<(QuoteResponse, Vec<u8>), String> {
    let encoded = serde_json::to_vec(body).map_err(|e| e.to_string())?;
    let raw = request(host, "POST", "/v0/quote", Some(jwt), encoded)?;
    let quote = crate::api_types::from_slice_no_duplicates(&raw)
        .map_err(|e| format!("strict quote response: {e}"))?;
    Ok((quote, raw))
}

pub fn submit<H: crate::workflow::Host>(
    host: &mut H,
    jwt: &PartnerJwt,
    tx_hash: &str,
    deposit_address: &str,
) -> Result<Vec<u8>, String> {
    request(
        host,
        "POST",
        "/v0/deposit/submit",
        Some(jwt),
        serde_json::to_vec(&serde_json::json!({"txHash":tx_hash,"depositAddress":deposit_address}))
            .unwrap(),
    )
}

pub fn status<H: crate::workflow::Host>(
    host: &mut H,
    jwt: &PartnerJwt,
    deposit_address: &str,
) -> Result<(ExecutionStatus, Vec<u8>), String> {
    let query = url::form_urlencoded::Serializer::new(String::new())
        .append_pair("depositAddress", deposit_address)
        .finish();
    let raw = request(
        host,
        "GET",
        &format!("/v0/status?{query}"),
        Some(jwt),
        vec![],
    )?;
    let status = crate::api_types::from_slice_no_duplicates(&raw)
        .map_err(|e| format!("status response: {e}"))?;
    Ok((status, raw))
}
