//! Inspect the session's own outbox entry through Bloom's package-scoped host call.
use crate::{runtime::Host, session::Session};
use petal::sdk::OutboxInspection;

/// Bloom answers `tx_inspect` only for an entry this package staged from the
/// route's bound account, in whichever state it now is, so a successful
/// inspection proves ownership and sender. The deposit bytes are bound by the
/// ID recorded from this session's own `tx_stage` of its prepared transaction;
/// Bloom does not let a Petal read outbox entries through the VFS.
pub fn inspect_owned<H: Host>(host: &mut H, session: &Session) -> Result<OutboxInspection, String> {
    session.account.as_ref().ok_or("account binding missing")?;
    let id = session.outbox_id.as_deref().ok_or("outbox ID missing")?;
    if id.is_empty()
        || id.len() > 128
        || !id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
    {
        return Err("invalid outbox ID".into());
    }
    session
        .prepared_transaction
        .as_ref()
        .ok_or("prepared transaction missing")?;
    let inspection = host
        .tx_inspect(&session.wallet, &session.origin.bloom_chain, id)
        .map_err(|error| {
            format!(
                "owned outbox entry unavailable in the numbered account; reconcile before retrying: {error}"
            )
        })?;
    if inspection.outbox_id != id {
        return Err("Bloom outbox inspection named a different entry".into());
    }
    Ok(inspection)
}
