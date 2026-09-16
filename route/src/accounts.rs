//! Wallet-scoped EVM identity: this Petal supports only account 0.
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
    if account.number != 0 {
        return Err("only account 0 is supported".into());
    }
    if address(host, &session.wallet)? != session.wallet_address {
        return Err("persisted account 0 address changed".into());
    }
    Ok(())
}

pub fn address<H: Host>(host: &mut H, wallet: &str) -> Result<String, String> {
    let raw = host.vfs_read(&format!("wallets/{wallet}/0/address.evm"), 128)?;
    let address = std::str::from_utf8(&raw)
        .map_err(|_| "wallet address is not UTF-8")?
        .trim()
        .to_ascii_lowercase();
    crate::assets::validate_address(&address)?;
    Ok(address)
}
