//! Wallet-scoped EVM identity bound to the selected numbered account.
use crate::runtime::Host;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountBinding {
    pub number: u32,
}

pub fn revalidate<H: Host>(host: &mut H, session: &crate::session::Session) -> Result<(), String> {
    let account = session
        .account
        .as_ref()
        .ok_or("legacy session has no account binding; manual recovery required")?;
    if address_for_account(host, &session.wallet, account.number)? != session.wallet_address {
        return Err("persisted account address changed".into());
    }
    Ok(())
}

/// Lifecycle/key-presence preflight for a new executable quote. This does not
/// grant signing approval; the host independently authorizes staging/signing.
pub fn require_active_evm_key<H: Host>(
    host: &mut H,
    wallet: &str,
    account: u32,
) -> Result<(), String> {
    let raw = host.vfs_read(
        &format!("wallets/{wallet}/{account}/account.json"),
        16 * 1024,
    )?;
    let projection: serde_json::Value =
        serde_json::from_slice(&raw).map_err(|_| "invalid numbered account projection")?;
    if projection["schema"] != "bloom.account.v1"
        || projection["wallet"] != wallet
        || projection["number"].as_u64() != Some(u64::from(account))
    {
        return Err("numbered account projection identity mismatch".into());
    }
    let evm = &projection["evm"];
    if evm["state"] != "active"
        || !evm["key_ref"].as_object().is_some_and(|key| {
            key.get("key_spec").and_then(serde_json::Value::as_str) == Some("secp256k1")
        })
    {
        return Err("new executable quotes require an active EVM account key".into());
    }
    Ok(())
}

pub fn address<H: Host>(host: &mut H, wallet: &str) -> Result<String, String> {
    address_for_account(host, wallet, 0)
}

pub fn address_for_account<H: Host>(
    host: &mut H,
    wallet: &str,
    account: u32,
) -> Result<String, String> {
    let raw = host.vfs_read(&format!("wallets/{wallet}/{account}/address.evm"), 128)?;
    let address = std::str::from_utf8(&raw)
        .map_err(|_| "wallet address is not UTF-8")?
        .trim()
        .to_ascii_lowercase();
    crate::assets::validate_address(&address)?;
    Ok(address)
}
