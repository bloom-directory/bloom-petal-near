use crate::{
    api,
    api_types::{QuoteRequest, QuoteResponse},
    assets, evm,
    input::NewSwapRequest,
    quote_signature, render,
    session::{self, Session},
    settings::{self, PartnerJwt},
};
use petal::sdk::{EvmTransaction, HttpRequest, HttpResponse, OutboxInspection, StagedTransaction};
use sha2::{Digest, Sha256};

pub trait Host {
    fn now_ms(&mut self) -> u64;
    fn random(&mut self, len: usize) -> Result<Vec<u8>, String>;
    fn setting(&mut self, key: &str) -> Result<Option<String>, String>;
    fn http(&mut self, req: HttpRequest, max: usize) -> Result<HttpResponse, String>;
    fn get(&mut self, key: &str, max: usize) -> Result<Option<Vec<u8>>, String>;
    fn list(&mut self, prefix: &str, max: usize) -> Result<Vec<String>, String>;
    fn put(&mut self, key: &str, value: &[u8], secret: bool) -> Result<(), String>;
    fn put_new(&mut self, key: &str, value: &[u8], secret: bool) -> Result<(), String>;
    fn delete_if(&mut self, key: &str, expected: &[u8]) -> Result<(), String>;
    fn vfs_read(&mut self, path: &str, max: usize) -> Result<Vec<u8>, String>;
    fn chain_read(&mut self, chain: &str, method: &str, params: &str) -> Result<String, String>;
    fn tx_stage(&mut self, tx: &EvmTransaction) -> Result<StagedTransaction, String>;
    fn tx_confirm(
        &mut self,
        wallet: &str,
        chain: &str,
        id: &str,
        warnings: bool,
    ) -> Result<StagedTransaction, String>;
    fn tx_inspect(
        &mut self,
        wallet: &str,
        chain: &str,
        id: &str,
    ) -> Result<OutboxInspection, String>;
}

pub struct BloomHost;
impl Host for BloomHost {
    fn now_ms(&mut self) -> u64 {
        petal::sdk::now_ms()
    }
    fn random(&mut self, len: usize) -> Result<Vec<u8>, String> {
        petal::sdk::random_bytes(len).map_err(|e| e.message())
    }
    fn setting(&mut self, key: &str) -> Result<Option<String>, String> {
        petal::sdk::runtime_setting(key).map_err(|e| e.message())
    }
    fn http(&mut self, req: HttpRequest, max: usize) -> Result<HttpResponse, String> {
        petal::sdk::http_fetch(&req, max).map_err(|e| e.message())
    }
    fn get(&mut self, key: &str, max: usize) -> Result<Option<Vec<u8>>, String> {
        match petal::sdk::store_get(key, max) {
            Ok(v) => Ok(Some(v)),
            Err(petal::SdkError::Host(petal::HostStatus::NotFound)) => Ok(None),
            Err(e) => Err(e.message()),
        }
    }
    fn list(&mut self, prefix: &str, max: usize) -> Result<Vec<String>, String> {
        petal::sdk::store_list(prefix, max).map_err(|e| e.message())
    }
    fn put(&mut self, key: &str, value: &[u8], secret: bool) -> Result<(), String> {
        petal::sdk::store_put(key, value, secret).map_err(|e| e.message())
    }
    fn put_new(&mut self, key: &str, value: &[u8], secret: bool) -> Result<(), String> {
        petal::sdk::store_put_new(key, value, secret).map_err(|e| e.message())
    }
    fn delete_if(&mut self, key: &str, expected: &[u8]) -> Result<(), String> {
        petal::sdk::store_del_if_value(key, expected).map_err(|e| e.message())
    }
    fn vfs_read(&mut self, path: &str, max: usize) -> Result<Vec<u8>, String> {
        petal::sdk::vfs_read(path, max).map_err(|e| e.message())
    }
    fn chain_read(&mut self, chain: &str, method: &str, params: &str) -> Result<String, String> {
        petal::sdk::chain_read(chain, method, params).map_err(|e| e.message())
    }
    fn tx_stage(&mut self, tx: &EvmTransaction) -> Result<StagedTransaction, String> {
        petal::sdk::tx_stage(tx).map_err(|e| e.message())
    }
    fn tx_confirm(
        &mut self,
        wallet: &str,
        chain: &str,
        id: &str,
        warnings: bool,
    ) -> Result<StagedTransaction, String> {
        petal::sdk::tx_confirm(wallet, chain, id, warnings).map_err(|e| e.message())
    }
    fn tx_inspect(
        &mut self,
        wallet: &str,
        chain: &str,
        id: &str,
    ) -> Result<OutboxInspection, String> {
        petal::sdk::tx_inspect(wallet, chain, id).map_err(|e| e.message())
    }
}

fn json<T: serde::Serialize>(value: &T) -> Result<Vec<u8>, String> {
    serde_json::to_vec(value).map_err(|e| e.to_string())
}
fn save<H: Host>(host: &mut H, s: &Session) -> Result<(), String> {
    host.put(&s.key(), &json(s)?, false)
}

fn dispatch(result: Result<(), String>) -> petal::DispatchResponse {
    match result {
        Ok(()) => petal::DispatchResponse::Write,
        Err(e) => petal::error(-4, crate::redaction::sanitize_message(&e)),
    }
}

pub fn write_api_key_route(body: &[u8]) -> petal::DispatchResponse {
    let mut h = BloomHost;
    dispatch(write_api_key(&mut h, body))
}
pub fn read_api_key_route() -> petal::DispatchResponse {
    let mut h = BloomHost;
    match credential_status(&mut h) {
        Ok(v) => petal::read_json_value(&v),
        Err(e) => petal::error(-4, e),
    }
}
pub fn settings_status_route() -> petal::DispatchResponse {
    let mut h = BloomHost;
    match credential_status(&mut h) {
        Ok(v) => petal::read_json_value(
            &serde_json::json!({"credential":v,"endpoint_binding":"oneclick","supported_origins":assets::CHAINS.iter().map(|m|serde_json::json!({"oneclick":m.oneclick,"bloom":m.bloom,"chain_id":m.chain_id})).collect::<Vec<_>>() }),
        ),
        Err(e) => petal::error(-4, e),
    }
}
pub fn tokens_route() -> petal::DispatchResponse {
    let mut h = BloomHost;
    match api::tokens(&mut h){Ok((v,_))=>petal::read_json_value(&v.iter().map(|t|serde_json::json!({"assetId":t.asset_id,"decimals":t.decimals,"blockchain":t.blockchain,"symbol":t.symbol,"price":t.price,"priceUpdatedAt":t.price_updated_at,"contractAddress":t.contract_address,"execution":if assets::chain_mapping(&t.blockchain).is_some(){"executable"}else{"quote_only"}})).collect::<Vec<_>>()),Err(e)=>petal::error(-4,e)}
}
pub fn create_route(wallet: &str, body: &[u8]) -> petal::DispatchResponse {
    let mut h = BloomHost;
    match create(&mut h, wallet, body) {
        Ok(_) => petal::DispatchResponse::Write,
        Err(e) => petal::error(-4, crate::redaction::sanitize_message(&e)),
    }
}
pub fn confirm_route(wallet: &str, id: &str, body: &[u8]) -> petal::DispatchResponse {
    let mut h = BloomHost;
    dispatch(confirm(&mut h, wallet, id, body))
}
pub fn refresh_route(wallet: &str, id: &str, body: &[u8]) -> petal::DispatchResponse {
    let mut h = BloomHost;
    dispatch(refresh(&mut h, wallet, id, body))
}
pub fn abandon_route(wallet: &str, id: &str) -> petal::DispatchResponse {
    let mut h = BloomHost;
    dispatch(abandon(&mut h, wallet, id))
}
pub fn latest_route(wallet: &str) -> petal::DispatchResponse {
    let mut h = BloomHost;
    match h.get(&format!("swaps/{wallet}/latest"), 128) {
        Ok(Some(v)) => petal::DispatchResponse::Read(v),
        Ok(None) => petal::error(-1, "no swap session"),
        Err(e) => petal::error(-4, e),
    }
}
pub fn session_children(wallet: &str) -> Result<Vec<petal::RouteChild>, petal::DispatchResponse> {
    let mut h = BloomHost;
    let prefix = format!("swaps/{wallet}/");
    let keys = h
        .list(&prefix, 1024 * 1024)
        .map_err(|e| petal::error(-4, e))?;
    let mut ids = std::collections::BTreeSet::new();
    for key in keys {
        if let Some(rest) = key.strip_prefix(&prefix)
            && let Some((id, file)) = rest.split_once('/')
            && matches!(file, "session.json" | "failure.json")
            && petal::is_safe_segment(id)
        {
            ids.insert(id.to_string());
        }
    }
    let mut out = vec![petal::writable("new"), petal::file("latest")];
    out.extend(ids.into_iter().map(petal::dir));
    Ok(out)
}
pub fn session_route(wallet: &str, id: &str, field: &str) -> petal::DispatchResponse {
    let mut h = BloomHost;
    let s = match load(&mut h, wallet, id) {
        Ok(s) => s,
        Err(e) => {
            if field == "status"
                && let Ok(Some(raw)) = h.get(&session::failure_key(wallet, id), 64 * 1024)
            {
                return petal::DispatchResponse::Read(raw);
            }
            return petal::error(-1, e);
        }
    };
    match field {
        "request" => petal::read_json_value(&s.request),
        "quote" => petal::read_json_value(
            &serde_json::json!({"correlation_id":s.quote.correlation_id,"timestamp":s.quote.timestamp,"signature":s.quote.signature,"quote_request":s.quote.quote_request,"quote":s.quote.quote,"verified":s.quote_verified,"hash":s.quote_hash}),
        ),
        "review" => petal::read_json_value(
            &serde_json::json!({"prepared_artifact_digest":s.prepared_digest,"wallet":s.wallet,"wallet_address":s.wallet_address,"origin":s.origin,"quote_hash":s.quote_hash,"transaction":s.prepared_transaction,"destination_asset":s.quote.quote_request.destination_asset,"recipient":s.quote.quote_request.recipient,"minimum_output":s.quote.quote.min_amount_out,"refund_to":s.quote.quote_request.refund_to}),
        ),
        "plan" => petal::DispatchResponse::Read(render::plan(&s).into_bytes()),
        "policy" => petal::read_json_value(
            &serde_json::json!({"quote_verified":s.quote_verified,"chain_id_expected":s.origin.expected_chain_id,"state":s.state,"outbox_state":s.outbox_state}),
        ),
        "approval" => s
            .approval
            .as_ref()
            .map(petal::read_json_value)
            .unwrap_or_else(|| petal::error(-1, "no approval pending")),
        "outbox" => petal::read_json_value(
            &serde_json::json!({"id":s.outbox_id,"chain":s.origin.bloom_chain,"state":s.outbox_state,"tx_hash":s.origin_tx_hash,"receipt":s.outbox_receipt}),
        ),
        "status" => petal::read_json_value(&render::public_status(&s)),
        "receipt" => {
            if s.terminal() && s.state != "abandoned" {
                petal::read_json_value(
                    &serde_json::json!({"state":s.state,"origin_tx_hash":s.origin_tx_hash,"outbox_receipt":s.outbox_receipt,"upstream_status":s.upstream_status,"upstream_updated_at":s.upstream_updated_at,"swap_details":s.swap_details,"updated_ms":s.updated_ms}),
                )
            } else {
                petal::error(-1, "terminal receipt not available")
            }
        }
        _ => petal::error(-3, "unknown session view"),
    }
}
pub fn load<H: Host>(host: &mut H, wallet: &str, id: &str) -> Result<Session, String> {
    let raw = host
        .get(&session::key(wallet, id), 2 * 1024 * 1024)?
        .ok_or("session not found")?;
    serde_json::from_slice(&raw).map_err(|e| format!("corrupt session: {e}"))
}
fn jwt<H: Host>(host: &mut H) -> Result<PartnerJwt, String> {
    let raw = host
        .get(settings::JWT_KEY, 8192)?
        .ok_or("1Click API key is not configured")?;
    settings::parse_jwt(&raw)
}

fn acquire_lock<H: Host>(host: &mut H, key: &str, ttl_ms: u64) -> Result<Vec<u8>, String> {
    #[derive(serde::Serialize, serde::Deserialize)]
    struct Lock {
        owner: String,
        expires_ms: u64,
    }
    if let Some(existing) = host.get(key, 1024)?
        && let Ok(lock) = serde_json::from_slice::<Lock>(&existing)
        && lock.expires_ms <= host.now_ms()
    {
        host.delete_if(key, &existing)?;
    }
    let lock = Lock {
        owner: hex::encode(host.random(16)?),
        expires_ms: host.now_ms().saturating_add(ttl_ms),
    };
    let bytes = json(&lock)?;
    host.put_new(key, &bytes, false)?;
    Ok(bytes)
}

fn finish_locked<T>(result: Result<T, String>, release: Result<(), String>) -> Result<T, String> {
    match result {
        Err(error) => Err(error),
        Ok(value) => {
            release?;
            Ok(value)
        }
    }
}

pub fn write_api_key<H: Host>(host: &mut H, body: &[u8]) -> Result<(), String> {
    let jwt = settings::parse_jwt(body)?;
    host.put(settings::JWT_KEY, jwt.expose().as_bytes(), true)
}
pub fn credential_status<H: Host>(host: &mut H) -> Result<settings::CredentialStatus, String> {
    Ok(settings::status(
        host.get(settings::JWT_KEY, 8192)?.is_some(),
    ))
}

fn wallet_details<H: Host>(host: &mut H, wallet: &str) -> Result<(String, String), String> {
    if wallet.is_empty()
        || wallet.len() > 128
        || !wallet
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
    {
        return Err("wallet name is invalid".into());
    }
    let address = String::from_utf8(host.vfs_read(&format!("wallets/{wallet}/address"), 128)?)
        .map_err(|_| "wallet address is not UTF-8")?
        .trim()
        .to_string();
    assets::validate_address(&address)?;
    let kind = String::from_utf8(host.vfs_read(&format!("wallets/{wallet}/kind"), 64)?)
        .map_err(|_| "wallet kind is not UTF-8")?
        .trim()
        .to_string();
    if kind == "watch" {
        return Err("watch-only wallets cannot create executable swaps".into());
    }
    Ok((address, kind))
}

fn iso_deadline(now_ms: u64, seconds: u32) -> Result<String, String> {
    let t = time::OffsetDateTime::from_unix_timestamp_nanos(
        (now_ms as i128 + seconds as i128 * 1000) * 1_000_000,
    )
    .map_err(|_| "clock out of range")?;
    t.format(&time::format_description::well_known::Rfc3339)
        .map_err(|e| e.to_string())
}
fn parse_time_ms(value: &str) -> Result<u64, String> {
    let t = time::OffsetDateTime::parse(value, &time::format_description::well_known::Rfc3339)
        .map_err(|_| "quote deadline is not RFC3339")?;
    u64::try_from(t.unix_timestamp_nanos() / 1_000_000)
        .map_err(|_| "quote deadline predates epoch".into())
}

fn validate_echo(
    sent: &QuoteRequest,
    got: &QuoteResponse,
    wallet: &str,
    now: u64,
) -> Result<(), String> {
    let r = &got.quote_request;
    let q = &got.quote;
    if r.dry
        || r.deposit_type != "ORIGIN_CHAIN"
        || r.refund_type != "ORIGIN_CHAIN"
        || r.recipient_type != "DESTINATION_CHAIN"
    {
        return Err("quote is not an executable origin-chain quote".into());
    }
    if r.deposit_mode.as_deref() != Some("SIMPLE")
        || r.insured.unwrap_or(false)
        || r.connected_wallets.as_ref().is_some_and(|v| !v.is_empty())
        || r.session_id.is_some()
        || r.virtual_chain_recipient.is_some()
        || r.virtual_chain_refund_recipient.is_some()
        || r.custom_recipient_msg.is_some()
        || r.referral.is_some()
        || r.rebates.as_ref().is_some_and(|v| !v.is_empty())
        || r.app_fees.as_ref().is_some_and(|v| !v.is_empty())
        || !matches!(r.confidentiality.as_deref(), None | Some("public"))
        || q.chain_deposit_addresses
            .as_ref()
            .is_some_and(|v| !v.is_empty())
        || q.virtual_chain_recipient.is_some()
        || q.virtual_chain_refund_recipient.is_some()
        || q.custom_recipient_msg.is_some()
    {
        return Err("quote contains unsupported execution metadata".into());
    }
    if r.swap_type != sent.swap_type
        || r.slippage_tolerance != sent.slippage_tolerance
        || r.origin_asset != sent.origin_asset
        || r.destination_asset != sent.destination_asset
        || r.amount != sent.amount
        || r.refund_to != wallet
        || r.recipient != sent.recipient
        || r.deadline != sent.deadline
        || r.quote_waiting_time_ms != sent.quote_waiting_time_ms
    {
        return Err("quote echoed different execution fields".into());
    }
    let deposit = q
        .deposit_address
        .as_deref()
        .ok_or("quote has no deposit address")?;
    assets::validate_address(deposit)?;
    if q.deposit_memo.as_deref().is_some_and(|v| !v.is_empty()) {
        return Err("EVM origin quote unexpectedly requires a memo".into());
    }
    for (name, value) in [
        ("amountIn", &q.amount_in),
        ("amountOut", &q.amount_out),
        ("minAmountIn", &q.min_amount_in),
        ("minAmountOut", &q.min_amount_out),
    ] {
        if !crate::input::canonical_amount(value) {
            return Err(format!("quote {name} is not a positive integer"));
        }
    }
    let u = |v: &str| {
        alloy::primitives::U256::from_str_radix(v, 10)
            .map_err(|_| "quote amount exceeds uint256".to_string())
    };
    if u(&q.min_amount_in)? > u(&q.amount_in)? || u(&q.min_amount_out)? > u(&q.amount_out)? {
        return Err("quote minimum exceeds quoted amount".into());
    }
    let deadline = parse_time_ms(q.deadline.as_deref().ok_or("quote has no deadline")?)?;
    if deadline < now.saturating_add(120_000) {
        return Err("quote deadline is too close or expired".into());
    }
    if let Some(inactive) = &q.time_when_inactive
        && parse_time_ms(inactive)? <= now
    {
        return Err("quote deposit address is inactive".into());
    }
    Ok(())
}

fn parse_hex_quantity(raw: &str) -> Result<alloy::primitives::U256, String> {
    let s: String = serde_json::from_str(raw).map_err(|_| "RPC result is not a string")?;
    alloy::primitives::U256::from_str_radix(s.strip_prefix("0x").unwrap_or(&s), 16)
        .map_err(|_| "RPC result is not a hex quantity".into())
}
fn eth_call<H: Host>(
    host: &mut H,
    chain: &str,
    to: &str,
    data: &str,
) -> Result<alloy::primitives::U256, String> {
    let p = serde_json::json!([{"to":to,"data":data},"latest"]).to_string();
    parse_hex_quantity(&host.chain_read(chain, "eth_call", &p)?)
}
fn address_word(address: &str) -> Result<String, String> {
    assets::validate_address(address)?;
    Ok(format!("{:0>64}", &address[2..].to_ascii_lowercase()))
}

fn preflight<H: Host>(
    host: &mut H,
    origin: &assets::ResolvedOrigin,
    wallet: &str,
    amount: &str,
) -> Result<(), String> {
    const MIN_GAS_RESERVE_WEI: u64 = 100_000_000_000_000;
    let chain_id =
        parse_hex_quantity(&host.chain_read(&origin.bloom_chain, "eth_chainId", "[]")?)?;
    if chain_id != alloy::primitives::U256::from(origin.expected_chain_id) {
        return Err("live chain ID does not match origin token".into());
    }
    let native = parse_hex_quantity(&host.chain_read(
        &origin.bloom_chain,
        "eth_getBalance",
        &serde_json::json!([wallet, "latest"]).to_string(),
    )?)?;
    let wanted = alloy::primitives::U256::from_str_radix(amount, 10)
        .map_err(|_| "amount exceeds uint256")?;
    if let Some(contract) = &origin.contract_address {
        if native < alloy::primitives::U256::from(MIN_GAS_RESERVE_WEI) {
            return Err("wallet native balance is below the conservative gas reserve".into());
        }
        let code: String = serde_json::from_str(&host.chain_read(
            &origin.bloom_chain,
            "eth_getCode",
            &serde_json::json!([contract, "latest"]).to_string(),
        )?)
        .map_err(|_| "code result invalid")?;
        if code == "0x" || code == "0x0" {
            return Err("origin token contract has no code".into());
        }
        if eth_call(host, &origin.bloom_chain, contract, "0x313ce567")?
            != alloy::primitives::U256::from(origin.decimals)
        {
            return Err("token decimals do not match 1Click metadata".into());
        }
        let balance = eth_call(
            host,
            &origin.bloom_chain,
            contract,
            &format!("0x70a08231{}", address_word(wallet)?),
        )?;
        if balance < wanted {
            return Err("insufficient token balance".into());
        }
    } else if native < wanted.saturating_add(alloy::primitives::U256::from(MIN_GAS_RESERVE_WEI)) {
        return Err("insufficient native balance after reserving gas".into());
    }
    Ok(())
}

pub fn create<H: Host>(host: &mut H, wallet: &str, body: &[u8]) -> Result<String, String> {
    create_with_verifier(host, wallet, body, |quote| {
        quote_signature::verify(quote).map_err(|e| e.to_string())
    })
}

fn create_with_verifier<H: Host, V: Fn(&QuoteResponse) -> Result<String, String>>(
    host: &mut H,
    wallet: &str,
    body: &[u8],
    verifier: V,
) -> Result<String, String> {
    let req: NewSwapRequest =
        serde_json::from_slice(body).map_err(|e| format!("swap request JSON: {e}"))?;
    req.validate()?;
    let (wallet_address, _) = wallet_details(host, wallet)?;
    if req
        .refund_to
        .as_deref()
        .is_some_and(|v| !v.eq_ignore_ascii_case(&wallet_address))
    {
        return Err("refund_to must equal the selected wallet address".into());
    }
    let jwt = jwt(host)?;
    let (tokens, _raw_tokens) = api::tokens(host)?;
    let token = tokens
        .iter()
        .find(|t| t.asset_id == req.origin_asset)
        .cloned()
        .ok_or("origin asset not found")?;
    let origin = assets::resolve(&tokens, &req.origin_asset)?;
    let now = host.now_ms();
    let id = req.session_id.clone();
    let reservation = format!("swaps/{wallet}/{id}/reservation");
    host.put_new(&reservation, &Sha256::digest(body), false)
        .map_err(|_| {
            "session_id already exists; inspect the existing session instead".to_string()
        })?;
    let lock = format!("locks/swaps/{wallet}/{id}");
    let lock_token = acquire_lock(host, &lock, 60_000)?;
    let sent = QuoteRequest {
        dry: false,
        deposit_mode: Some("SIMPLE".into()),
        insured: None,
        swap_type: req.swap_type.clone(),
        slippage_tolerance: req.slippage_bps,
        origin_asset: req.origin_asset.clone(),
        deposit_type: "ORIGIN_CHAIN".into(),
        destination_asset: req.destination_asset.clone(),
        amount: req.amount.clone(),
        refund_to: wallet_address.clone(),
        refund_type: "ORIGIN_CHAIN".into(),
        recipient: req.recipient.clone(),
        connected_wallets: None,
        session_id: None,
        virtual_chain_recipient: None,
        virtual_chain_refund_recipient: None,
        custom_recipient_msg: None,
        recipient_type: "DESTINATION_CHAIN".into(),
        deadline: iso_deadline(now, req.deadline_seconds)?,
        confidentiality: None,
        referral: None,
        rebates: None,
        quote_waiting_time_ms: Some(req.quote_waiting_time_ms),
        app_fees: None,
    };
    let result: Result<String, String> = (|| {
        let (quote, raw) = api::quote(host, &jwt, &sent)?;
        let hash = verifier(&quote)?;
        validate_echo(&sent, &quote, &wallet_address, now)?;
        preflight(host, &origin, &wallet_address, &quote.quote.amount_in)?;
        host.put(&format!("swaps/{wallet}/{id}/quote.raw.json"), &raw, false)?;
        let mut s = Session {
            schema_version: 1,
            id: id.clone(),
            wallet: wallet.into(),
            wallet_address,
            created_ms: now,
            updated_ms: now,
            state: "quoted".into(),
            request: req,
            origin_token: token,
            origin,
            quote,
            quote_hash: hash,
            quote_verified: true,
            prepared_transaction: None,
            prepared_digest: None,
            plan_md: None,
            staging_started: false,
            outbox_id: None,
            outbox_state: None,
            origin_tx_hash: None,
            outbox_receipt: None,
            approval: None,
            deposit_submit_state: None,
            upstream_status: None,
            upstream_updated_at: None,
            swap_details: None,
            last_error: None,
            history: vec![],
        };
        s.transition(now, "quoted", "verified 1Click quote persisted");
        save(host, &s)?;
        host.put(
            &format!("swaps/{wallet}/latest"),
            serde_json::to_string(
                &serde_json::json!({"id":id,"path":format!("swaps/{wallet}/{id}")}),
            )
            .unwrap()
            .as_bytes(),
            false,
        )?;
        Ok(id.clone())
    })();
    if let Err(error) = &result {
        let failure = session::FailedSession {
            schema_version: 1,
            id: id.clone(),
            wallet: wallet.into(),
            created_ms: now,
            updated_ms: host.now_ms(),
            state: "quote_failed".into(),
            last_error: crate::redaction::sanitize_message(error),
        };
        let _ = host.put(
            &session::failure_key(wallet, &id),
            &json(&failure).unwrap_or_default(),
            false,
        );
        let latest =
            serde_json::to_vec(&serde_json::json!({"id":id,"path":format!("swaps/{wallet}/{id}")}))
                .unwrap();
        let _ = host.put(&format!("swaps/{wallet}/latest"), &latest, false);
    }
    let _ = host.delete_if(&lock, &lock_token);
    result
}

#[derive(serde::Deserialize)]
struct Confirm {
    confirm: bool,
    #[serde(default)]
    acknowledge_warnings: bool,
}
fn confirmation(body: &[u8]) -> Result<Confirm, String> {
    let t = std::str::from_utf8(body)
        .map_err(|_| "confirmation must be UTF-8")?
        .trim();
    if matches!(t, "confirm" | "y") {
        Ok(Confirm {
            confirm: true,
            acknowledge_warnings: false,
        })
    } else {
        let c: Confirm = serde_json::from_str(t)
            .map_err(|_| "confirmation requires confirm, y, or JSON confirmation")?;
        if !c.confirm {
            Err("confirm must be true".into())
        } else {
            Ok(c)
        }
    }
}

pub fn confirm<H: Host>(host: &mut H, wallet: &str, id: &str, body: &[u8]) -> Result<(), String> {
    confirm_with_verifier(host, wallet, id, body, |quote| {
        quote_signature::verify(quote).map_err(|e| e.to_string())
    })
}

fn confirm_with_verifier<H: Host, V: Fn(&QuoteResponse) -> Result<String, String>>(
    host: &mut H,
    wallet: &str,
    id: &str,
    body: &[u8],
    verifier: V,
) -> Result<(), String> {
    let c = confirmation(body)?;
    let lock_key = format!("locks/swaps/{wallet}/{id}");
    let lock_token = acquire_lock(host, &lock_key, 120_000)?;
    let result = (|| {
        let now = host.now_ms();
        let mut s = load(host, wallet, id)?;
        if s.terminal() {
            return Err("session is terminal".into());
        }
        validate_echo(&s.quote.quote_request, &s.quote, &s.wallet_address, now)?;
        verifier(&s.quote)?;
        match s.state.as_str() {
            "quoted" => {
                preflight(host, &s.origin, &s.wallet_address, &s.quote.quote.amount_in)?;
                let tx = evm::prepare(
                    s.origin.contract_address.as_deref(),
                    s.quote.quote.deposit_address.as_deref().unwrap(),
                    &s.quote.quote.amount_in,
                )?;
                let digest = hex::encode(Sha256::digest(json(
                    &serde_json::json!({"app":"near-intents","session":s.id,"wallet":s.wallet,"wallet_address":s.wallet_address,"chain":s.origin.bloom_chain,"chain_id":s.origin.expected_chain_id,"asset":s.origin.asset_id,"contract":s.origin.contract_address,"deposit":s.quote.quote.deposit_address,"amount":s.quote.quote.amount_in,"transaction":tx,"correlation_id":s.quote.correlation_id,"quote_hash":s.quote_hash,"signature":s.quote.signature,"deadline":s.quote.quote.deadline,"destination":s.quote.quote_request.destination_asset,"recipient":s.quote.quote_request.recipient,"min_output":s.quote.quote.min_amount_out,"refund":s.quote.quote_request.refund_to}),
                )?));
                s.prepared_transaction = Some(tx);
                s.prepared_digest = Some(digest);
                s.transition(now, "prepared", "immutable deposit transaction prepared");
                save(host, &s)
            }
            "prepared" => {
                preflight(host, &s.origin, &s.wallet_address, &s.quote.quote.amount_in)?;
                s.staging_started = true;
                s.transition(
                    now,
                    "staging_started",
                    "durable ambiguity marker before outbox stage",
                );
                save(host, &s)?;
                let tx = s
                    .prepared_transaction
                    .clone()
                    .ok_or("prepared transaction missing")?;
                match host.tx_stage(&EvmTransaction {
                    wallet: s.wallet.clone(),
                    chain: s.origin.bloom_chain.clone(),
                    to: tx.to,
                    value_wei: tx.value_wei,
                    data_hex: tx.data_hex,
                    nonce: None,
                    max_fee_per_gas: None,
                    max_priority_fee_per_gas: None,
                }) {
                    Ok(staged) => {
                        s.outbox_id = Some(staged.outbox_id);
                        s.plan_md = Some(staged.plan_md);
                        s.approval=staged.approval.map(|a|serde_json::json!({"action_id":a.action_id,"ceremony_url":a.ceremony_url,"expires_ms":a.expires_ms}));
                        s.staging_started = false;
                        s.transition(now, "staged", "Bloom outbox transaction staged");
                        save(host, &s)
                    }
                    Err(e) => {
                        s.transition(
                            now,
                            "staging_ambiguous",
                            "outbox stage returned without durable id",
                        );
                        s.last_error = Some(crate::redaction::sanitize_message(&e));
                        save(host, &s)?;
                        Err("outbox staging is ambiguous; manual recovery required".into())
                    }
                }
            }
            "staged" | "approval_required" => {
                preflight(host, &s.origin, &s.wallet_address, &s.quote.quote.amount_in)?;
                let id = s.outbox_id.clone().ok_or("outbox ID missing")?;
                let out = host.tx_confirm(
                    &s.wallet,
                    &s.origin.bloom_chain,
                    &id,
                    c.acknowledge_warnings,
                )?;
                s.plan_md = Some(out.plan_md);
                s.approval=out.approval.map(|a|serde_json::json!({"action_id":a.action_id,"ceremony_url":a.ceremony_url,"expires_ms":a.expires_ms}));
                let next = if s.approval.is_some() {
                    "approval_required"
                } else {
                    "deposit_broadcast_pending"
                };
                s.transition(now, next, "Bloom outbox confirmation advanced");
                save(host, &s)
            }
            "deposit_broadcast_pending" | "deposit_sent" => inspect_outbox(host, &mut s),
            "staging_started" | "staging_ambiguous" => {
                Err("prior staging may have succeeded; refusing to restage".into())
            }
            _ => Err(format!("confirm cannot advance state {}", s.state)),
        }
    })();
    let release = host.delete_if(&lock_key, &lock_token);
    finish_locked(result, release)
}

fn inspect_outbox<H: Host>(host: &mut H, s: &mut Session) -> Result<(), String> {
    let id = s.outbox_id.clone().ok_or("outbox ID missing")?;
    let i = host.tx_inspect(&s.wallet, &s.origin.bloom_chain, &id)?;
    s.outbox_state = Some(i.state.clone());
    if let Some(hash) = i.tx_hash {
        s.origin_tx_hash = Some(hash);
    }
    if let Some(receipt) = i.receipt_json.as_deref() {
        s.outbox_receipt = render::sanitize_outbox_receipt(receipt);
    }
    let now = host.now_ms();
    if matches!(i.state.as_str(), "reverted" | "failed" | "cancelled") {
        s.transition(now, "deposit_failed", "Bloom outbox reported failure")
    } else if s.origin_tx_hash.is_some() {
        s.transition(now, "deposit_sent", "origin transaction hash observed")
    }
    save(host, s)
}

pub fn refresh<H: Host>(host: &mut H, wallet: &str, id: &str, body: &[u8]) -> Result<(), String> {
    refresh_with_verifier(host, wallet, id, body, |quote| {
        quote_signature::verify(quote).map_err(|e| e.to_string())
    })
}

fn refresh_with_verifier<H: Host, V: Fn(&QuoteResponse) -> Result<String, String>>(
    host: &mut H,
    wallet: &str,
    id: &str,
    body: &[u8],
    verifier: V,
) -> Result<(), String> {
    let text = std::str::from_utf8(body)
        .map_err(|_| "refresh must be UTF-8")?
        .trim();
    if text != "refresh"
        && serde_json::from_str::<serde_json::Value>(text)
            .ok()
            .and_then(|v| v.get("refresh").and_then(|x| x.as_bool()))
            != Some(true)
    {
        return Err("refresh requires refresh or {\"refresh\":true}".into());
    }
    let lock_key = format!("locks/swaps/{wallet}/{id}");
    let lock_token = acquire_lock(host, &lock_key, 120_000)?;
    let result = (|| {
        let mut s = load(host, wallet, id)?;
        if s.terminal() {
            return Ok(());
        }
        if s.outbox_id.is_some() && s.origin_tx_hash.is_none() {
            inspect_outbox(host, &mut s)?;
            s = load(host, wallet, id)?;
            if s.origin_tx_hash.is_none() {
                s.last_error = None;
                save(host, &s)?;
                return Ok(());
            }
        }
        let jwt = jwt(host)?;
        if let Some(hash) = s.origin_tx_hash.clone()
            && s.deposit_submit_state.as_deref() != Some("submitted")
        {
            s.deposit_submit_state = Some("submit_ambiguous".into());
            save(host, &s)?;
            match api::submit(
                host,
                &jwt,
                &hash,
                s.quote.quote.deposit_address.as_deref().unwrap(),
            ) {
                Ok(raw) => {
                    host.put(&format!("swaps/{wallet}/{id}/submit.raw.json"), &raw, false)?;
                    s.deposit_submit_state = Some("submitted".into());
                    save(host, &s)?;
                }
                Err(e) => {
                    s.last_error = Some(crate::redaction::sanitize_message(&e));
                    save(host, &s)?;
                    return Err("deposit submit outcome is ambiguous; refresh may retry".into());
                }
            }
        }
        let (status, raw) = match api::status(
            host,
            &jwt,
            s.quote.quote.deposit_address.as_deref().unwrap(),
        ) {
            Ok(value) => value,
            Err(e) => {
                s.last_error = Some(crate::redaction::sanitize_message(&e));
                save(host, &s)?;
                return Err(e);
            }
        };
        host.put(&format!("swaps/{wallet}/{id}/status.raw.json"), &raw, false)?;
        if status.correlation_id.len() > 256
            || status.status.is_empty()
            || status.status.len() > 64
            || status.updated_at.len() > 64
        {
            s.last_error = Some("status metadata exceeds public safety bounds".into());
            save(host, &s)?;
            return Err("status metadata exceeds public safety bounds".into());
        }
        let mut status_quote = match status
            .quote_response
            .into_verified_shape(&s.quote.correlation_id)
        {
            Ok(quote) => quote,
            Err(e) => {
                s.last_error = Some(e.clone());
                save(host, &s)?;
                return Err(e);
            }
        };
        if status_quote.quote_request.quote_waiting_time_ms.is_none() {
            status_quote.quote_request.quote_waiting_time_ms =
                s.quote.quote_request.quote_waiting_time_ms;
        }
        let hash = match verifier(&status_quote) {
            Ok(hash) => hash,
            Err(e) => {
                s.last_error = Some(crate::redaction::sanitize_message(&e));
                save(host, &s)?;
                return Err(format!("status quote: {e}"));
            }
        };
        if hash != s.quote_hash
            || status_quote.quote.deposit_address != s.quote.quote.deposit_address
            || status_quote.quote_request.recipient != s.quote.quote_request.recipient
            || status_quote.quote_request.refund_to != s.quote.quote_request.refund_to
        {
            s.last_error = Some("status response does not correlate to persisted quote".into());
            save(host, &s)?;
            return Err("status response does not correlate to persisted quote".into());
        }
        s.upstream_status = Some(status.status.clone());
        s.upstream_updated_at = Some(status.updated_at);
        s.swap_details = Some(render::sanitize_swap_details(&status.swap_details));
        s.last_error = None;
        let next = match status.status.as_str() {
            "SUCCESS" => "settled_success",
            "REFUNDED" => "settled_refunded",
            "FAILED" => "settled_failed",
            "INCOMPLETE_DEPOSIT" => "deposit_incomplete",
            "KNOWN_DEPOSIT_TX" => "known_deposit",
            "PROCESSING" => "processing",
            "PENDING_DEPOSIT" => "pending_deposit",
            _ => "upstream_unknown",
        };
        s.transition(host.now_ms(), next, "verified 1Click status response");
        save(host, &s)
    })();
    let release = host.delete_if(&lock_key, &lock_token);
    finish_locked(result, release)
}

pub fn abandon<H: Host>(host: &mut H, wallet: &str, id: &str) -> Result<(), String> {
    let lock_key = format!("locks/swaps/{wallet}/{id}");
    let lock_token = acquire_lock(host, &lock_key, 120_000)?;
    let result = (|| {
        let mut s = load(host, wallet, id)?;
        if s.outbox_id.is_some()
            || matches!(
                s.state.as_str(),
                "deposit_broadcast_pending" | "deposit_sent"
            )
        {
            return Err("cannot abandon after an outbox transaction exists".into());
        }
        s.transition(host.now_ms(), "abandoned", "user abandoned before deposit");
        save(host, &s)
    })();
    let release = host.delete_if(&lock_key, &lock_token);
    finish_locked(result, release)
}

#[cfg(test)]
mod workflow_tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};
    use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

    const WALLET: &str = "0x1111111111111111111111111111111111111111";
    const DEPOSIT: &str = "0x2222222222222222222222222222222222222222";
    const JWT: &str = "test.jwt.must-never-appear";

    #[derive(Default)]
    struct Shared {
        store: BTreeMap<String, Vec<u8>>,
        last_quote: Option<QuoteResponse>,
        quote_request_json: Option<serde_json::Value>,
        stage_calls: usize,
        confirm_calls: usize,
        submit_calls: usize,
        quote_calls: usize,
        status_calls: usize,
        erc20: bool,
        stage_fails: bool,
        corrupt_signature: bool,
        response_insured: Option<bool>,
        inspect_pending: bool,
        malformed_status: bool,
        inspect_denied: bool,
    }

    #[derive(Clone)]
    struct MockHost(Rc<RefCell<Shared>>);

    fn test_key() -> SigningKey {
        SigningKey::from_bytes(&[7u8; 32])
    }
    fn test_verify(q: &QuoteResponse) -> Result<String, String> {
        let key = format!(
            "ed25519:{}",
            bs58::encode(test_key().verifying_key().as_bytes()).into_string()
        );
        quote_signature::verify_with_key(q, &key).map_err(|e| e.to_string())
    }

    fn signed_quote(req: QuoteRequest) -> QuoteResponse {
        let mut q = QuoteResponse {
            correlation_id: "mock-correlation".into(),
            timestamp: "2027-01-15T08:00:00Z".into(),
            signature: String::new(),
            quote_request: req.clone(),
            quote: crate::api_types::Quote {
                deposit_address: Some(DEPOSIT.into()),
                deposit_memo: None,
                chain_deposit_addresses: None,
                amount_in: req.amount.clone(),
                amount_in_formatted: "0.000000000000001".into(),
                amount_in_usd: "0.01".into(),
                min_amount_in: req.amount.clone(),
                amount_out: "2000".into(),
                amount_out_formatted: "0.002".into(),
                amount_out_usd: "0.01".into(),
                min_amount_out: "1900".into(),
                deadline: Some(req.deadline.clone()),
                time_when_inactive: Some(req.deadline.clone()),
                time_estimate: 30,
                virtual_chain_recipient: None,
                virtual_chain_refund_recipient: None,
                custom_recipient_msg: None,
                refund_fee: Some("1".into()),
                withdraw_fee: Some("2".into()),
            },
        };
        let hash = quote_signature::quote_hash(&q).unwrap();
        q.signature = format!(
            "ed25519:{}",
            bs58::encode(test_key().sign(hash.as_bytes()).to_bytes()).into_string()
        );
        q
    }

    impl Host for MockHost {
        fn now_ms(&mut self) -> u64 {
            1_800_000_000_000
        }
        fn random(&mut self, len: usize) -> Result<Vec<u8>, String> {
            Ok(vec![9; len])
        }
        fn setting(&mut self, _: &str) -> Result<Option<String>, String> {
            Ok(Some("https://mock.invalid".into()))
        }
        fn http(&mut self, req: HttpRequest, _: usize) -> Result<HttpResponse, String> {
            let path = url::Url::parse(&req.url).unwrap().path().to_string();
            let body = match (req.method.as_str(), path.as_str()) {
                ("GET", "/v0/tokens") => {
                    let contract = self
                        .0
                        .borrow()
                        .erc20
                        .then_some("0x3333333333333333333333333333333333333333");
                    serde_json::to_vec(&vec![serde_json::json!({"assetId":"nep141:eth.omft.near","decimals":18,"blockchain":"eth","symbol":"ETH","price":1000.0,"priceUpdatedAt":"2027-01-15T08:00:00Z","contractAddress":contract})]).unwrap()
                }
                ("POST", "/v0/quote") => {
                    self.0.borrow_mut().quote_calls += 1;
                    assert!(
                        req.headers
                            .iter()
                            .any(|(k, v)| k == "authorization" && v == &format!("Bearer {JWT}"))
                    );
                    let request_json: serde_json::Value =
                        serde_json::from_slice(&req.body).unwrap();
                    self.0.borrow_mut().quote_request_json = Some(request_json.clone());
                    let request: QuoteRequest = serde_json::from_value(request_json).unwrap();
                    let mut quote = signed_quote(request);
                    quote.quote_request.insured = self.0.borrow().response_insured;
                    if self.0.borrow().corrupt_signature {
                        quote.signature = "ed25519:1".into();
                    }
                    self.0.borrow_mut().last_quote = Some(quote.clone());
                    serde_json::to_vec(&quote).unwrap()
                }
                ("POST", "/v0/deposit/submit") => {
                    self.0.borrow_mut().submit_calls += 1;
                    b"{}".to_vec()
                }
                ("GET", "/v0/status") => {
                    self.0.borrow_mut().status_calls += 1;
                    if self.0.borrow().malformed_status {
                        return Ok(HttpResponse {
                            status: 200,
                            headers: vec![],
                            body: br#"{"message":"deposit not known yet"}"#.to_vec(),
                        });
                    }
                    let q = self.0.borrow().last_quote.clone().unwrap();
                    let mut status_quote = serde_json::to_value(&q).unwrap();
                    let status_quote = status_quote.as_object_mut().unwrap();
                    status_quote.remove("correlationId");
                    let status_request = status_quote
                        .get_mut("quoteRequest")
                        .unwrap()
                        .as_object_mut()
                        .unwrap();
                    status_request.remove("quoteWaitingTimeMs");
                    status_request.remove("insured");
                    status_request.insert("appFees".into(), serde_json::json!([]));
                    serde_json::to_vec(&serde_json::json!({"correlationId":"independent-status-response-id","quoteResponse":status_quote,"status":"SUCCESS","updatedAt":"2027-01-15T08:01:00Z","swapDetails":{"intentHashes":["intent"],"nearTxHashes":["near"],"originChainTxHashes":[],"destinationChainTxHashes":[],"unknownSecret":"must-not-persist"}})).unwrap()
                }
                _ => return Err(format!("unexpected HTTP {} {}", req.method, path)),
            };
            Ok(HttpResponse {
                status: 200,
                headers: vec![],
                body,
            })
        }
        fn get(&mut self, key: &str, _: usize) -> Result<Option<Vec<u8>>, String> {
            Ok(self.0.borrow().store.get(key).cloned())
        }
        fn list(&mut self, prefix: &str, _: usize) -> Result<Vec<String>, String> {
            Ok(self
                .0
                .borrow()
                .store
                .keys()
                .filter(|k| k.starts_with(prefix))
                .cloned()
                .collect())
        }
        fn put(&mut self, key: &str, value: &[u8], _: bool) -> Result<(), String> {
            self.0.borrow_mut().store.insert(key.into(), value.into());
            Ok(())
        }
        fn put_new(&mut self, key: &str, value: &[u8], _: bool) -> Result<(), String> {
            let mut s = self.0.borrow_mut();
            if s.store.contains_key(key) {
                return Err("already exists".into());
            }
            s.store.insert(key.into(), value.into());
            Ok(())
        }
        fn delete_if(&mut self, key: &str, expected: &[u8]) -> Result<(), String> {
            let mut s = self.0.borrow_mut();
            if s.store.get(key).is_some_and(|v| v == expected) {
                s.store.remove(key);
            }
            Ok(())
        }
        fn vfs_read(&mut self, path: &str, _: usize) -> Result<Vec<u8>, String> {
            if path.ends_with("/address") {
                Ok(WALLET.as_bytes().into())
            } else if path.ends_with("/kind") {
                Ok(b"passkey".into())
            } else {
                Err("not found".into())
            }
        }
        fn chain_read(&mut self, _: &str, method: &str, params: &str) -> Result<String, String> {
            Ok(match method {
                "eth_chainId" => r#""0x1""#,
                "eth_getBalance" => r#""0xffffffffffffffff""#,
                "eth_getCode" => r#""0x6000""#,
                "eth_call" if params.contains("313ce567") => r#""0x12""#,
                "eth_call" => r#""0xffffffffffffffff""#,
                _ => return Err("unexpected RPC".into()),
            }
            .into())
        }
        fn tx_stage(&mut self, tx: &EvmTransaction) -> Result<StagedTransaction, String> {
            let mut s = self.0.borrow_mut();
            s.stage_calls += 1;
            if s.stage_fails {
                return Err("backend timeout".into());
            }
            if s.erc20 {
                assert_eq!(tx.to, "0x3333333333333333333333333333333333333333");
                assert!(tx.data_hex.starts_with("0xa9059cbb"));
            } else {
                assert_eq!(tx.to, DEPOSIT);
            }
            Ok(StagedTransaction {
                outbox_id: "outbox-1".into(),
                plan_md: "# Bloom outbox plan".into(),
                approval: None,
            })
        }
        fn tx_confirm(
            &mut self,
            _: &str,
            _: &str,
            id: &str,
            _: bool,
        ) -> Result<StagedTransaction, String> {
            assert_eq!(id, "outbox-1");
            self.0.borrow_mut().confirm_calls += 1;
            Ok(StagedTransaction {
                outbox_id: id.into(),
                plan_md: "# Bloom outbox plan confirmed".into(),
                approval: None,
            })
        }
        fn tx_inspect(&mut self, _: &str, _: &str, id: &str) -> Result<OutboxInspection, String> {
            if self.0.borrow().inspect_denied {
                return Err("denied".into());
            }
            if self.0.borrow().inspect_pending {
                return Ok(OutboxInspection {
                    outbox_id: id.into(),
                    state: "pending".into(),
                    tx_hash: None,
                    receipt_json: None,
                });
            }
            Ok(OutboxInspection {
                outbox_id: id.into(),
                state: "sent".into(),
                tx_hash: Some("0xabc".into()),
                receipt_json: Some("{}".into()),
            })
        }
    }

    #[test]
    fn persistent_secret_and_restart_safe_end_to_end_workflow() {
        let shared = Rc::new(RefCell::new(Shared::default()));
        let mut host = MockHost(shared.clone());
        write_api_key(&mut host, JWT.as_bytes()).unwrap();
        drop(host);
        let mut restarted = MockHost(shared.clone());
        assert!(credential_status(&mut restarted).unwrap().configured);
        let request = serde_json::json!({"session_id":"test-e2e-session","swap_type":"EXACT_INPUT","origin_asset":"nep141:eth.omft.near","destination_asset":"nep141:sol.omft.near","amount":"1000","recipient":"recipient-on-destination","deadline_seconds":900});
        let id = create_with_verifier(
            &mut restarted,
            "alice",
            &serde_json::to_vec(&request).unwrap(),
            test_verify,
        )
        .unwrap();
        assert_eq!(load(&mut restarted, "alice", &id).unwrap().state, "quoted");
        drop(restarted);
        let mut restarted = MockHost(shared.clone());
        confirm_with_verifier(&mut restarted, "alice", &id, b"confirm", test_verify).unwrap();
        assert_eq!(
            load(&mut restarted, "alice", &id).unwrap().state,
            "prepared"
        );
        drop(restarted);
        let mut restarted = MockHost(shared.clone());
        confirm_with_verifier(&mut restarted, "alice", &id, b"confirm", test_verify).unwrap();
        assert_eq!(load(&mut restarted, "alice", &id).unwrap().state, "staged");
        drop(restarted);
        let mut restarted = MockHost(shared.clone());
        confirm_with_verifier(&mut restarted, "alice", &id, b"confirm", test_verify).unwrap();
        assert_eq!(
            load(&mut restarted, "alice", &id).unwrap().state,
            "deposit_broadcast_pending"
        );
        refresh_with_verifier(&mut restarted, "alice", &id, b"refresh", test_verify).unwrap();
        let session = load(&mut restarted, "alice", &id).unwrap();
        assert_eq!(session.state, "settled_success");
        assert_eq!(session.origin_tx_hash.as_deref(), Some("0xabc"));
        assert_eq!(session.outbox_receipt, Some(serde_json::json!({})));
        assert!(
            !session
                .swap_details
                .as_ref()
                .unwrap()
                .to_string()
                .contains("unknownSecret")
        );
        let shared = shared.borrow();
        assert_eq!(shared.stage_calls, 1);
        assert_eq!(shared.confirm_calls, 1);
        assert_eq!(shared.submit_calls, 1);
        let request = shared
            .quote_request_json
            .as_ref()
            .unwrap()
            .as_object()
            .unwrap();
        for prohibited in [
            "insured",
            "confidentiality",
            "connectedWallets",
            "sessionId",
            "virtualChainRecipient",
            "virtualChainRefundRecipient",
            "customRecipientMsg",
            "referral",
            "rebates",
            "appFees",
        ] {
            assert!(
                !request.contains_key(prohibited),
                "sent prohibited field {prohibited}"
            );
        }
        let public = serde_json::to_string(&render::public_status(&session)).unwrap();
        assert!(!public.contains(JWT));
        assert!(!public.to_ascii_lowercase().contains("authorization"));
    }

    #[test]
    fn accepts_upstream_insured_false_but_rejects_true() {
        let request = serde_json::json!({"session_id":"test-insured-session","swap_type":"EXACT_INPUT","origin_asset":"nep141:eth.omft.near","destination_asset":"nep141:sol.omft.near","amount":"1000","recipient":"recipient-on-destination","deadline_seconds":900});

        for (insured, succeeds) in [(false, true), (true, false)] {
            let shared = Rc::new(RefCell::new(Shared {
                response_insured: Some(insured),
                ..Shared::default()
            }));
            let mut host = MockHost(shared);
            write_api_key(&mut host, JWT.as_bytes()).unwrap();
            let result = create_with_verifier(
                &mut host,
                "alice",
                &serde_json::to_vec(&request).unwrap(),
                test_verify,
            );
            assert_eq!(result.is_ok(), succeeds);
            if let Err(error) = result {
                assert!(error.contains("unsupported execution metadata"));
            }
        }
    }

    #[test]
    fn confirm_and_refresh_require_the_whole_session_lock() {
        let shared = Rc::new(RefCell::new(Shared::default()));
        let mut host = MockHost(shared.clone());
        write_api_key(&mut host, JWT.as_bytes()).unwrap();
        let body = serde_json::to_vec(&serde_json::json!({
            "session_id":"test-session-lock",
            "swap_type":"EXACT_INPUT",
            "origin_asset":"nep141:eth.omft.near",
            "destination_asset":"dest",
            "amount":"1000",
            "recipient":"recipient",
            "deadline_seconds":900
        }))
        .unwrap();
        let id = create_with_verifier(&mut host, "alice", &body, test_verify).unwrap();
        confirm_with_verifier(&mut host, "alice", &id, b"confirm", test_verify).unwrap();
        let lock_key = format!("locks/swaps/alice/{id}");
        shared.borrow_mut().store.insert(
            lock_key,
            br#"{"owner":"concurrent-route","expires_ms":1800000120000}"#.to_vec(),
        );

        assert!(confirm_with_verifier(&mut host, "alice", &id, b"confirm", test_verify).is_err());
        assert!(refresh_with_verifier(&mut host, "alice", &id, b"refresh", test_verify).is_err());
        assert_eq!(load(&mut host, "alice", &id).unwrap().state, "prepared");
        let shared = shared.borrow();
        assert_eq!(shared.stage_calls, 0);
        assert_eq!(shared.status_calls, 0);
    }

    #[test]
    fn erc20_workflow_prepares_transfer_to_validated_contract() {
        let shared = Rc::new(RefCell::new(Shared {
            erc20: true,
            ..Shared::default()
        }));
        let mut host = MockHost(shared);
        write_api_key(&mut host, JWT.as_bytes()).unwrap();
        let body=serde_json::to_vec(&serde_json::json!({"session_id":"test-erc20-session","swap_type":"EXACT_INPUT","origin_asset":"nep141:eth.omft.near","destination_asset":"dest","amount":"1000","recipient":"recipient","deadline_seconds":900})).unwrap();
        let id = create_with_verifier(&mut host, "alice", &body, test_verify).unwrap();
        confirm_with_verifier(&mut host, "alice", &id, b"confirm", test_verify).unwrap();
        let tx = load(&mut host, "alice", &id)
            .unwrap()
            .prepared_transaction
            .unwrap();
        assert_eq!(tx.to, "0x3333333333333333333333333333333333333333");
        assert_eq!(tx.value_wei, "0");
        assert!(tx.data_hex.starts_with("0xa9059cbb"));
    }

    #[test]
    fn ambiguous_stage_is_durable_and_never_retried() {
        let shared = Rc::new(RefCell::new(Shared {
            stage_fails: true,
            ..Shared::default()
        }));
        let mut host = MockHost(shared.clone());
        write_api_key(&mut host, JWT.as_bytes()).unwrap();
        let body=serde_json::to_vec(&serde_json::json!({"session_id":"test-ambiguous-session","swap_type":"EXACT_INPUT","origin_asset":"nep141:eth.omft.near","destination_asset":"dest","amount":"1000","recipient":"recipient","deadline_seconds":900})).unwrap();
        let id = create_with_verifier(&mut host, "alice", &body, test_verify).unwrap();
        confirm_with_verifier(&mut host, "alice", &id, b"confirm", test_verify).unwrap();
        assert!(confirm_with_verifier(&mut host, "alice", &id, b"confirm", test_verify).is_err());
        assert_eq!(
            load(&mut host, "alice", &id).unwrap().state,
            "staging_ambiguous"
        );
        assert!(confirm_with_verifier(&mut host, "alice", &id, b"confirm", test_verify).is_err());
        assert_eq!(shared.borrow().stage_calls, 1);
    }

    #[test]
    fn quote_failure_is_persisted_without_the_credential() {
        let shared = Rc::new(RefCell::new(Shared {
            corrupt_signature: true,
            ..Shared::default()
        }));
        let mut host = MockHost(shared);
        write_api_key(&mut host, JWT.as_bytes()).unwrap();
        let body=serde_json::to_vec(&serde_json::json!({"session_id":"test-failure-session","swap_type":"EXACT_INPUT","origin_asset":"nep141:eth.omft.near","destination_asset":"dest","amount":"1000","recipient":"recipient","deadline_seconds":900})).unwrap();
        assert!(create_with_verifier(&mut host, "alice", &body, test_verify).is_err());
        let id = "test-failure-session";
        let raw = host
            .get(&session::failure_key("alice", id), 65536)
            .unwrap()
            .unwrap();
        let text = String::from_utf8(raw).unwrap();
        assert!(text.contains("quote_failed"));
        assert!(!text.contains(JWT));
    }

    #[test]
    fn caller_session_id_is_reserved_before_quote_side_effects() {
        let shared = Rc::new(RefCell::new(Shared::default()));
        let mut host = MockHost(shared.clone());
        write_api_key(&mut host, JWT.as_bytes()).unwrap();
        let body=serde_json::to_vec(&serde_json::json!({"session_id":"caller-known-session","swap_type":"EXACT_INPUT","origin_asset":"nep141:eth.omft.near","destination_asset":"dest","amount":"1000","recipient":"recipient","deadline_seconds":900})).unwrap();
        assert_eq!(
            create_with_verifier(&mut host, "alice", &body, test_verify).unwrap(),
            "caller-known-session"
        );
        let error = create_with_verifier(&mut host, "alice", &body, test_verify).unwrap_err();
        assert!(error.contains("session_id already exists"));
        assert_eq!(shared.borrow().quote_calls, 1);
    }

    #[test]
    fn refresh_does_not_poll_upstream_before_outbox_has_a_hash() {
        let shared = Rc::new(RefCell::new(Shared {
            inspect_pending: true,
            ..Shared::default()
        }));
        let mut host = MockHost(shared.clone());
        write_api_key(&mut host, JWT.as_bytes()).unwrap();
        let body=serde_json::to_vec(&serde_json::json!({"session_id":"pending-outbox-session","swap_type":"EXACT_INPUT","origin_asset":"nep141:eth.omft.near","destination_asset":"dest","amount":"1000","recipient":"recipient","deadline_seconds":900})).unwrap();
        let id = create_with_verifier(&mut host, "alice", &body, test_verify).unwrap();
        confirm_with_verifier(&mut host, "alice", &id, b"confirm", test_verify).unwrap();
        confirm_with_verifier(&mut host, "alice", &id, b"confirm", test_verify).unwrap();
        refresh_with_verifier(&mut host, "alice", &id, b"refresh", test_verify).unwrap();
        let session = load(&mut host, "alice", &id).unwrap();
        assert_eq!(session.state, "staged");
        assert_eq!(session.outbox_state.as_deref(), Some("pending"));
        assert!(session.origin_tx_hash.is_none());
        assert_eq!(shared.borrow().status_calls, 0);
    }

    #[test]
    fn upstream_status_errors_do_not_strand_executable_state() {
        let shared = Rc::new(RefCell::new(Shared {
            malformed_status: true,
            ..Shared::default()
        }));
        let mut host = MockHost(shared.clone());
        write_api_key(&mut host, JWT.as_bytes()).unwrap();
        let body=serde_json::to_vec(&serde_json::json!({"session_id":"status-error-session","swap_type":"EXACT_INPUT","origin_asset":"nep141:eth.omft.near","destination_asset":"dest","amount":"1000","recipient":"recipient","deadline_seconds":900})).unwrap();
        let id = create_with_verifier(&mut host, "alice", &body, test_verify).unwrap();
        for _ in 0..3 {
            confirm_with_verifier(&mut host, "alice", &id, b"confirm", test_verify).unwrap();
        }
        assert!(refresh_with_verifier(&mut host, "alice", &id, b"refresh", test_verify).is_err());
        let session = load(&mut host, "alice", &id).unwrap();
        assert_eq!(session.state, "deposit_sent");
        assert!(session.last_error.is_some());
        assert_eq!(shared.borrow().status_calls, 1);
    }

    #[test]
    fn migrated_sent_session_does_not_reinspect_old_package_outbox() {
        let shared = Rc::new(RefCell::new(Shared::default()));
        let mut host = MockHost(shared.clone());
        write_api_key(&mut host, JWT.as_bytes()).unwrap();
        let body=serde_json::to_vec(&serde_json::json!({"session_id":"migrated-sent-session","swap_type":"EXACT_INPUT","origin_asset":"nep141:eth.omft.near","destination_asset":"dest","amount":"1000","recipient":"recipient","deadline_seconds":900})).unwrap();
        let id = create_with_verifier(&mut host, "alice", &body, test_verify).unwrap();
        for _ in 0..3 {
            confirm_with_verifier(&mut host, "alice", &id, b"confirm", test_verify).unwrap();
        }
        let mut session = load(&mut host, "alice", &id).unwrap();
        session.origin_tx_hash = Some("0xabc".into());
        session.outbox_state = Some("success".into());
        session.state = "deposit_sent".into();
        save(&mut host, &session).unwrap();
        shared.borrow_mut().inspect_denied = true;

        refresh_with_verifier(&mut host, "alice", &id, b"refresh", test_verify).unwrap();
        assert_eq!(
            load(&mut host, "alice", &id).unwrap().state,
            "settled_success"
        );
        assert_eq!(shared.borrow().status_calls, 1);
    }
}
