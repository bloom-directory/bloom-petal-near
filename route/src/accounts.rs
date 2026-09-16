//! Broker-projected account selection. A request selects identity, never list position.
use crate::runtime::Host;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountBinding {
    pub number: u32,
    pub fingerprint: String,
    pub derivation_path: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrustedAccount {
    pub wallet: String,
    pub number: u32,
    pub fingerprint: String,
}

impl TrustedAccount {
    pub fn from_params(params: &[(String, String)]) -> Option<Self> {
        let unique = |name: &str| {
            let mut values = params.iter().filter(|(key, _)| key == name);
            let value = &values.next()?.1;
            values.next().is_none().then_some(value.clone())
        };
        Some(Self {
            wallet: unique("bloom.wallet")?,
            number: number(&unique("bloom.account")?)?,
            fingerprint: unique("bloom.owner_key_fingerprint")?,
        })
    }
}

fn number(value: &str) -> Option<u32> {
    if value.is_empty()
        || !value.bytes().all(|b| b.is_ascii_digit())
        || (value.len() > 1 && value.starts_with('0'))
    {
        return None;
    }
    value.parse::<u32>().ok().filter(|n| *n < 1 << 31)
}

#[derive(Deserialize)]
struct Inventory {
    wallet_id: String,
    accounts: Vec<Account>,
    accounts_unavailable: Option<String>,
}
#[derive(Deserialize)]
struct Account {
    number: Option<u32>,
    public_key_fingerprint: String,
    path: String,
    derivation_profile: String,
    lifecycle: String,
}

pub fn resolve(
    raw: &[u8],
    wallet: &str,
    fingerprint: &str,
    path: &str,
) -> Result<AccountBinding, String> {
    if fingerprint.len() != 64
        || !fingerprint
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err("account fingerprint must be 64 lowercase hex characters".into());
    }
    let expected = path
        .strip_prefix("m/44'/60'/0'/0/")
        .and_then(number)
        .ok_or("unsupported EVM account derivation path")?;
    let inventory: Inventory = serde_json::from_slice(raw).map_err(|_| "invalid accounts.json")?;
    if inventory.wallet_id != wallet || inventory.accounts_unavailable.is_some() {
        return Err("wallet account inventory is unavailable or mismatched".into());
    }
    let mut matches = inventory
        .accounts
        .iter()
        .filter(|a| a.public_key_fingerprint == fingerprint || a.path == path);
    let account = matches.next().ok_or("intended account not found")?;
    if matches.next().is_some()
        || account.public_key_fingerprint != fingerprint
        || account.path != path
        || account.number != Some(expected)
        || account.lifecycle != "active"
        || account.derivation_profile != "bip44-evm-secp256k1-v1"
    {
        return Err("account fingerprint, path, number or lifecycle mismatch".into());
    }
    Ok(AccountBinding {
        number: expected,
        fingerprint: fingerprint.into(),
        derivation_path: path.into(),
    })
}

pub fn bind<H: Host>(
    host: &mut H,
    wallet: &str,
    fingerprint: &str,
    path: &str,
) -> Result<AccountBinding, String> {
    let trusted = host.trusted_account().ok_or("Bloom account context unavailable at /petals/near-intents; account-scoped host dispatch is required")?;
    if trusted.wallet != wallet {
        return Err("trusted account wallet mismatch".into());
    }
    let raw = host.vfs_read(&format!("wallets/{wallet}/accounts.json"), 2 * 1024 * 1024)?;
    let binding = resolve(&raw, wallet, fingerprint, path)?;
    if trusted.number != binding.number || trusted.fingerprint != binding.fingerprint {
        return Err("trusted account does not match selected fingerprint and path".into());
    }
    Ok(binding)
}

pub fn revalidate<H: Host>(host: &mut H, session: &crate::session::Session) -> Result<(), String> {
    let account = session
        .account
        .as_ref()
        .ok_or("legacy session has no exact account binding; manual recovery required")?;
    if session.request.account_fingerprint != account.fingerprint
        || session.request.derivation_path != account.derivation_path
    {
        return Err("session request and account binding mismatch".into());
    }
    let current = bind(
        host,
        &session.wallet,
        &account.fingerprint,
        &account.derivation_path,
    )?;
    if current != *account || address(host, &session.wallet, account)? != session.wallet_address {
        return Err("persisted account binding changed".into());
    }
    Ok(())
}

pub fn address<H: Host>(
    host: &mut H,
    wallet: &str,
    account: &AccountBinding,
) -> Result<String, String> {
    let raw = host.vfs_read(
        &format!("wallets/{wallet}/{}/address.evm", account.number),
        128,
    )?;
    let address = std::str::from_utf8(&raw)
        .map_err(|_| "wallet address is not UTF-8")?
        .trim()
        .to_ascii_lowercase();
    crate::assets::validate_address(&address)?;
    Ok(address)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    fn row(n: u32, fingerprint: &str) -> Value {
        json!({"number":n,"public_key_fingerprint":fingerprint,"path":format!("m/44'/60'/0'/0/{n}"),"derivation_profile":"bip44-evm-secp256k1-v1","lifecycle":"active"})
    }
    fn resolve_rows(rows: Value) -> Result<AccountBinding, String> {
        resolve(
            &serde_json::to_vec(&json!({"wallet_id":"alice","accounts":rows})).unwrap(),
            "alice",
            &"aa".repeat(32),
            "m/44'/60'/0'/0/7",
        )
    }

    #[test]
    fn selects_sparse_number_by_fingerprint_and_path_independent_of_order() {
        let target = row(7, &"aa".repeat(32));
        let other = row(0, &"bb".repeat(32));
        for rows in [json!([target, other]), json!([other, target])] {
            let account = resolve_rows(rows).unwrap();
            assert_eq!(account.number, 7);
            assert_eq!(account.fingerprint, "aa".repeat(32));
        }
    }

    #[test]
    fn rejects_missing_duplicate_retired_and_mismatched_identity() {
        let target = row(7, &"aa".repeat(32));
        for rows in [
            json!([]),
            json!([row(0, &"bb".repeat(32))]),
            json!([target, target]),
        ] {
            assert!(resolve_rows(rows).is_err());
        }
        for (field, value) in [
            ("number", json!(0)),
            ("number", Value::Null),
            ("public_key_fingerprint", json!("bb".repeat(32))),
            ("path", json!("m/44'/60'/0'/0/0")),
            (
                "derivation_profile",
                json!("bip44-solana-slip10-ed25519-v1"),
            ),
            ("lifecycle", json!("retired")),
        ] {
            let mut bad = target.clone();
            bad[field] = value;
            assert!(resolve_rows(json!([bad])).is_err(), "{field}");
        }
    }

    #[test]
    fn rejects_unavailable_wrong_wallet_and_noncanonical_path() {
        let mut inventory = json!({"wallet_id":"alice","accounts":[row(7, &"aa".repeat(32))]});
        for path in [
            "m/44'/60'/0'/0/07",
            "m/44'/60'/1'/0/7",
            "m/44'/60'/0'/0/2147483648",
            "m/44'/501'/7'/0'",
        ] {
            assert!(
                resolve(
                    &serde_json::to_vec(&inventory).unwrap(),
                    "alice",
                    &"aa".repeat(32),
                    path
                )
                .is_err()
            );
        }
        assert!(
            resolve(
                &serde_json::to_vec(&inventory).unwrap(),
                "bob",
                &"aa".repeat(32),
                "m/44'/60'/0'/0/7"
            )
            .is_err()
        );
        inventory["accounts_unavailable"] = json!("Broker offline");
        assert!(
            resolve(
                &serde_json::to_vec(&inventory).unwrap(),
                "alice",
                &"aa".repeat(32),
                "m/44'/60'/0'/0/7"
            )
            .is_err()
        );
    }

    #[test]
    fn context_requires_all_unique_host_fields_and_never_defaults_to_zero() {
        let params = vec![
            ("bloom.wallet".into(), "alice".into()),
            ("bloom.account".into(), "7".into()),
            ("bloom.owner_key_fingerprint".into(), "aa".repeat(32)),
        ];
        assert_eq!(TrustedAccount::from_params(&params).unwrap().number, 7);
        for removed in 0..params.len() {
            let mut missing = params.clone();
            missing.remove(removed);
            assert!(TrustedAccount::from_params(&missing).is_none());
        }
        let mut duplicate = params.clone();
        duplicate.push(params[1].clone());
        assert!(TrustedAccount::from_params(&duplicate).is_none());
        for value in ["", "-1", "07", "2147483648"] {
            let mut malformed = params.clone();
            malformed[1].1 = value.into();
            assert!(TrustedAccount::from_params(&malformed).is_none());
        }
        assert!(TrustedAccount::from_params(&[("account".into(), "7".into())]).is_none());
    }
}
