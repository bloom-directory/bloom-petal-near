//! Wallet-scoped EVM identity bound to the selected numbered account.
use crate::runtime::Host;
use petal::WalletAccount;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountBinding {
    pub number: u32,
}

/// Re-check a session against the account Bloom selected for this route: the
/// session must be bound to that wallet account, and the account's address
/// must still be the one the session recorded.
pub fn revalidate<H: Host>(
    host: &mut H,
    account: &WalletAccount,
    session: &crate::session::Session,
) -> Result<(), String> {
    let bound = session
        .account
        .as_ref()
        .ok_or("legacy session has no account binding; manual recovery required")?;
    if session.wallet != account.wallet() || bound.number != account.number() {
        return Err("session is bound to another wallet account".into());
    }
    if evm_address(host, account)? != session.wallet_address {
        return Err("persisted account address changed".into());
    }
    Ok(())
}

/// Lifecycle/key-presence preflight for a new executable quote. This does not
/// grant signing approval; the host independently authorizes staging/signing.
pub fn require_active_evm_key<H: Host>(
    host: &mut H,
    account: &WalletAccount,
) -> Result<(), String> {
    let raw = host
        .vfs_read(&format!("{}/account.json", account.dir()), 16 * 1024)
        .map_err(|error| error.message())?;
    let projection: serde_json::Value =
        serde_json::from_slice(&raw).map_err(|_| "invalid numbered account projection")?;
    if projection["schema"] != "bloom.account.v1"
        || projection["wallet"] != account.wallet()
        || projection["number"].as_u64() != Some(u64::from(account.number()))
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

/// The account's EVM address as Bloom projects it, read through the SDK and
/// lowercased to the form sessions persist.
pub fn evm_address<H: Host>(host: &mut H, account: &WalletAccount) -> Result<String, String> {
    let address = account
        .address_evm_from(&mut HostVfs(host))
        .map_err(|response| match response {
            petal::DispatchResponse::Error { message, .. } => message,
            _ => "cannot read the account address".into(),
        })?
        .to_ascii_lowercase();
    crate::assets::validate_address(&address)?;
    Ok(address)
}

/// Lets the SDK read an account's address through [`Host`].
struct HostVfs<'a, H>(&'a mut H);

impl<H: Host> petal::VfsRead for HostVfs<'_, H> {
    fn read(&mut self, path: &str, max_bytes: usize) -> Result<Vec<u8>, petal::SdkError> {
        self.0.vfs_read(path, max_bytes)
    }
}
