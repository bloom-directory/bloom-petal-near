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
