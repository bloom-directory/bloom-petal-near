//! Verify the account-scoped public outbox before using the canonical SDK operations.
use crate::{runtime::Host, session::Session};
use serde::Deserialize;

#[derive(Deserialize)]
struct Intent {
    id: String,
    wallet: String,
    chain: String,
    chain_id: u64,
    from: String,
    to: String,
    value_wei: String,
    data_hex: String,
}

/// Returns the outbox directory (`pending`, `sent` or `failed`) holding the entry.
pub fn verify_owned<H: Host>(host: &mut H, session: &Session) -> Result<&'static str, String> {
    let account = session.account.as_ref().ok_or("account binding missing")?;
    let id = session.outbox_id.as_deref().ok_or("outbox ID missing")?;
    if id.is_empty()
        || id.len() > 128
        || !id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
    {
        return Err("invalid outbox ID".into());
    }
    let tx = session
        .prepared_transaction
        .as_ref()
        .ok_or("prepared transaction missing")?;
    // The entry may have moved after an ambiguous broadcast. Never use `latest`.
    for state in ["pending", "sent", "failed"] {
        let path = format!(
            "wallets/{}/{}/chains/{}/outbox/{state}/{id}/intent.json",
            session.wallet, account.number, session.origin.bloom_chain
        );
        let Ok(raw) = host.vfs_read(&path, 256 * 1024) else {
            continue;
        };
        let intent: Intent =
            serde_json::from_slice(&raw).map_err(|_| "invalid Bloom outbox intent")?;
        if intent.id != id
            || intent.wallet != session.wallet
            || intent.chain != session.origin.bloom_chain
            || intent.chain_id != session.origin.expected_chain_id
            || !intent.from.eq_ignore_ascii_case(&session.wallet_address)
            || !intent.to.eq_ignore_ascii_case(&tx.to)
            || intent.value_wei != tx.value_wei
            || !intent.data_hex.eq_ignore_ascii_case(&tx.data_hex)
        {
            return Err(
                "Bloom outbox intent does not match the bound account and prepared deposit".into(),
            );
        }
        return Ok(state);
    }
    Err("owned outbox entry unavailable in the numbered account; reconcile before retrying".into())
}

#[derive(Deserialize)]
struct FailedIntent {
    #[serde(default)]
    status: String,
    #[serde(default)]
    policy_checks: Vec<PolicyCheck>,
}

#[derive(Deserialize)]
struct PolicyCheck {
    outcome: String,
}

/// True only when the owned entry in `failed/` carries evidence that nothing
/// was signed: Bloom recorded no broadcast attempt, and the entry either holds
/// a hard policy denial or was cancelled locally by Bloom. An entry the expiry
/// sweep moved after an ambiguous broadcast has neither, so it never counts.
pub fn refused_before_signing<H: Host>(host: &mut H, session: &Session) -> Result<bool, String> {
    let account = session.account.as_ref().ok_or("account binding missing")?;
    let id = session.outbox_id.as_deref().ok_or("outbox ID missing")?;
    let dir = format!(
        "wallets/{}/{}/chains/{}/outbox/failed/{id}",
        session.wallet, account.number, session.origin.bloom_chain
    );
    if host
        .vfs_read(&format!("{dir}/broadcast_attempted.json"), 64 * 1024)
        .is_ok()
    {
        return Ok(false);
    }
    let raw = host.vfs_read(&format!("{dir}/intent.json"), 256 * 1024)?;
    let intent: FailedIntent =
        serde_json::from_slice(&raw).map_err(|_| "invalid Bloom outbox intent")?;
    let denied = intent.policy_checks.iter().any(|c| c.outcome == "deny");
    let cancelled =
        intent.status == "cancelled" && host.vfs_read(&format!("{dir}/cancel.txt"), 4096).is_ok();
    Ok(denied || cancelled)
}
