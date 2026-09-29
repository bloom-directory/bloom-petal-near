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

pub fn verify_owned<H: Host>(host: &mut H, session: &Session) -> Result<(), String> {
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
        return Ok(());
    }
    Err("owned outbox entry unavailable in the numbered account; reconcile before retrying".into())
}
