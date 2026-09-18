//! The venue policy: this Petal's own per-wallet rules for a swap.
//!
//! These rules are guard rails the Petal enforces before it quotes, stages, or
//! confirms anything. They are not custody: Bloom's wallet policy decides which
//! package may act and every deposit transaction is approved by the owner. A
//! wallet without a file gets the bundled defaults in `venue-defaults.toml`,
//! which pay out only to the wallet itself, cap slippage, and cap the input.
//!
//! Anything stored but unparseable fails closed, so a damaged file refuses
//! swaps rather than falling back to something more permissive.

use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};

/// The bundled policy a wallet gets until it writes its own.
pub const DEFAULT_VENUE_POLICY: &str = include_str!("venue-defaults.toml");

/// Largest venue policy this Petal will store or read.
pub const MAX_POLICY_BYTES: usize = 64 * 1024;

/// The recipient class meaning "this wallet's own address".
pub const WALLET_ADDRESS_CLASS: &str = "class:wallet_address";
/// Reserved prefix for named classes, so a typo cannot become a literal address.
const CLASS_PREFIX: &str = "class:";

pub fn venue_policy_key(wallet: &str) -> String {
    format!("settings/wallets/{wallet}/venue.toml")
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct VenuePolicy {
    #[serde(default)]
    pub venue: VenueSection,
    #[serde(default)]
    pub limits: Limits,
    #[serde(default)]
    pub recipients: Recipients,
    #[serde(default)]
    pub chains: Chains,
    #[serde(default)]
    pub solvers: Solvers,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct VenueSection {
    #[serde(default = "default_true")]
    pub enabled: bool,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Limits {
    #[serde(default = "default_max_slippage_bps")]
    pub max_slippage_bps: u16,
    /// Decimal USD string, or absent for no USD cap.
    #[serde(default)]
    pub max_input_usd: Option<String>,
    /// Caps in an asset's smallest units, keyed by 1Click asset id.
    #[serde(default)]
    pub max_input_units: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Recipients {
    #[serde(default)]
    pub allowed: BTreeSet<String>,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Chains {
    #[serde(default)]
    pub allowed_origin: BTreeSet<String>,
    #[serde(default)]
    pub allowed_destination: BTreeSet<String>,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Solvers {
    #[serde(default)]
    pub allowed_deposit_addresses: BTreeSet<String>,
}

fn default_true() -> bool {
    true
}

fn default_max_slippage_bps() -> u16 {
    100
}

impl Default for VenueSection {
    fn default() -> Self {
        Self { enabled: true }
    }
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_slippage_bps: default_max_slippage_bps(),
            max_input_usd: None,
            max_input_units: BTreeMap::new(),
        }
    }
}

impl Default for Recipients {
    fn default() -> Self {
        Self {
            allowed: [WALLET_ADDRESS_CLASS.to_string()].into_iter().collect(),
        }
    }
}

/// Parse a venue policy. Used for stored files and for the bundled defaults.
pub fn parse(bytes: &[u8]) -> Result<VenuePolicy, String> {
    if bytes.is_empty() || bytes.len() > MAX_POLICY_BYTES {
        return Err(format!("venue policy must be 1..={MAX_POLICY_BYTES} bytes"));
    }
    let text = std::str::from_utf8(bytes).map_err(|_| "venue policy must be UTF-8")?;
    let policy: VenuePolicy =
        toml::from_str(text).map_err(|error| format!("venue.toml is invalid: {error}"))?;
    policy.validate()?;
    Ok(policy)
}

/// The defaults, which are part of the package and must always parse.
pub fn defaults() -> VenuePolicy {
    parse(DEFAULT_VENUE_POLICY.as_bytes()).expect("bundled venue policy parses")
}

impl VenuePolicy {
    fn validate(&self) -> Result<(), String> {
        if self.limits.max_slippage_bps > 10_000 {
            return Err("limits.max_slippage_bps must be at most 10000".into());
        }
        if let Some(cap) = &self.limits.max_input_usd
            && parse_decimal(cap).is_none()
        {
            return Err("limits.max_input_usd must be a positive decimal amount".into());
        }
        for (asset, cap) in &self.limits.max_input_units {
            if asset.is_empty() || !crate::input::canonical_amount(cap) {
                return Err(format!(
                    "limits.max_input_units.{asset:?} must be a positive canonical integer"
                ));
            }
        }
        for entry in &self.recipients.allowed {
            check_address_entry("recipients.allowed", entry)?;
        }
        for entry in &self.solvers.allowed_deposit_addresses {
            check_address_entry("solvers.allowed_deposit_addresses", entry)?;
        }
        Ok(())
    }

    fn recipient_allowed(&self, recipient: &str, wallet_address: &str) -> bool {
        self.recipients.allowed.iter().any(|entry| {
            if entry == WALLET_ADDRESS_CLASS {
                recipient.eq_ignore_ascii_case(wallet_address)
            } else {
                entry.eq_ignore_ascii_case(recipient)
            }
        })
    }
}

/// An address entry is matched literally, so a typo silently matches nothing
/// and denies every swap. Addresses are not parsed here: a payout may settle on
/// a chain whose address form is not EVM, such as a NEAR account id or a Solana
/// address. What is checked is the shape a typo actually takes — stray
/// whitespace, and a near-miss of a reserved `class:` name, which would
/// otherwise be stored as a literal address that can never match.
fn check_address_entry(field: &str, entry: &str) -> Result<(), String> {
    if entry.trim().is_empty() || entry.len() > 1024 {
        return Err(format!("{field} entries must be 1..=1024 characters"));
    }
    if entry.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err(format!(
            "{field} entry {entry:?} contains whitespace; addresses are matched literally"
        ));
    }
    if entry.len() >= CLASS_PREFIX.len()
        && entry[..CLASS_PREFIX.len()].eq_ignore_ascii_case(CLASS_PREFIX)
        && entry != WALLET_ADDRESS_CLASS
    {
        return Err(format!(
            "{field} entry {entry:?} is not a known class; the only one is {WALLET_ADDRESS_CLASS:?}"
        ));
    }
    Ok(())
}

/// Read a wallet's stored policy, falling back to the bundled defaults. A
/// stored file that no longer parses fails closed.
pub fn load<H: crate::runtime::Host>(host: &mut H, wallet: &str) -> Result<VenuePolicy, String> {
    match host.get(&venue_policy_key(wallet), MAX_POLICY_BYTES)? {
        Some(bytes) => parse(&bytes),
        None => Ok(defaults()),
    }
}

/// The exact bytes a wallet's policy route serves: its stored file, or the
/// bundled defaults when it has none.
pub fn read_bytes<H: crate::runtime::Host>(host: &mut H, wallet: &str) -> Result<Vec<u8>, String> {
    Ok(host
        .get(&venue_policy_key(wallet), MAX_POLICY_BYTES)?
        .unwrap_or_else(|| DEFAULT_VENUE_POLICY.as_bytes().to_vec()))
}

/// Store a wallet's policy, refusing anything this Petal cannot enforce.
pub fn write<H: crate::runtime::Host>(
    host: &mut H,
    wallet: &str,
    body: &[u8],
) -> Result<(), String> {
    parse(body)?;
    host.put(&venue_policy_key(wallet), body, false)
}

/// Wallets with a stored policy, for the settings directory listing.
pub fn configured_wallets<H: crate::runtime::Host>(host: &mut H) -> Result<Vec<String>, String> {
    let keys = host.list("settings/wallets/", 1024)?;
    let mut wallets: Vec<String> = keys
        .iter()
        .filter_map(|key| {
            let rest = key.strip_prefix("settings/wallets/")?;
            let (wallet, leaf) = rest.split_once('/')?;
            (leaf == "venue.toml" && !wallet.is_empty()).then(|| wallet.to_owned())
        })
        .collect();
    wallets.sort();
    wallets.dedup();
    Ok(wallets)
}

/// What a swap asks for, as far as the venue policy is concerned.
#[derive(Debug, Clone)]
pub struct SwapContext<'a> {
    pub wallet_address: &'a str,
    pub recipient: &'a str,
    pub slippage_bps: u16,
    /// Bloom chain name the input is spent from.
    pub origin_chain: &'a str,
    /// 1Click blockchain code the output lands on.
    pub destination_chain: &'a str,
    pub origin_asset: &'a str,
    /// Input in the asset's smallest units, once the quote is known.
    pub amount_in: Option<&'a str>,
    /// The venue's USD valuation of the input, once the quote is known.
    pub amount_in_usd: Option<&'a str>,
    /// The deposit address the signed quote names, once the quote is known.
    pub deposit_address: Option<&'a str>,
}

fn check(rule: &str, outcome: &str, message: impl Into<String>) -> serde_json::Value {
    serde_json::json!({ "rule": rule, "outcome": outcome, "message": message.into() })
}

/// Evaluate the policy. Checks whose input is not known yet are reported as
/// `pending` and evaluated again once the quote exists.
pub fn evaluate(policy: &VenuePolicy, ctx: &SwapContext<'_>) -> serde_json::Value {
    let mut checks = Vec::new();

    checks.push(if policy.venue.enabled {
        check("venue.enabled", "pass", "NEAR Intents swaps are enabled")
    } else {
        check(
            "venue.enabled",
            "deny",
            "NEAR Intents swaps are disabled in this wallet's venue policy",
        )
    });

    checks.push(if ctx.slippage_bps <= policy.limits.max_slippage_bps {
        check(
            "limits.slippage",
            "pass",
            format!(
                "requested slippage {} bps; wallet maximum {} bps",
                ctx.slippage_bps, policy.limits.max_slippage_bps
            ),
        )
    } else {
        check(
            "limits.slippage",
            "deny",
            format!(
                "requested slippage {} bps exceeds the wallet maximum of {} bps",
                ctx.slippage_bps, policy.limits.max_slippage_bps
            ),
        )
    });

    checks.push(
        if policy.chains.allowed_origin.is_empty()
            || contains_ignore_case(&policy.chains.allowed_origin, ctx.origin_chain)
        {
            check(
                "chains.origin",
                "pass",
                format!("origin chain {} allowed", ctx.origin_chain),
            )
        } else {
            check(
                "chains.origin",
                "deny",
                format!(
                    "origin chain {} is not in chains.allowed_origin",
                    ctx.origin_chain
                ),
            )
        },
    );

    checks.push(
        if policy.chains.allowed_destination.is_empty()
            || contains_ignore_case(&policy.chains.allowed_destination, ctx.destination_chain)
        {
            check(
                "chains.destination",
                "pass",
                format!("destination chain {} allowed", ctx.destination_chain),
            )
        } else {
            check(
                "chains.destination",
                "deny",
                format!(
                    "destination chain {} is not in chains.allowed_destination",
                    ctx.destination_chain
                ),
            )
        },
    );

    checks.push(
        if policy.recipient_allowed(ctx.recipient, ctx.wallet_address) {
            check(
                "recipients.allowed",
                "pass",
                format!("recipient {} permitted", ctx.recipient),
            )
        } else {
            check(
                "recipients.allowed",
                "deny",
                format!(
                    "recipient {} is not allowed; this wallet pays out to {}",
                    ctx.recipient,
                    describe_recipients(policy, ctx.wallet_address)
                ),
            )
        },
    );

    checks.push(match (&policy.limits.max_input_usd, ctx.amount_in_usd) {
        (None, _) => check("limits.input_usd", "pass", "no USD cap is configured"),
        (Some(cap), None) => check(
            "limits.input_usd",
            "pending",
            format!("input value is checked against the {cap} USD cap once quoted"),
        ),
        (Some(cap), Some(value)) => match (parse_decimal(cap), parse_decimal(value)) {
            (Some(cap_units), Some(value_units)) if value_units <= cap_units => check(
                "limits.input_usd",
                "pass",
                format!("input valued at {value} USD; wallet cap {cap} USD"),
            ),
            (Some(_), Some(_)) => check(
                "limits.input_usd",
                "deny",
                format!("input valued at {value} USD exceeds the wallet cap of {cap} USD"),
            ),
            _ => check(
                "limits.input_usd",
                "deny",
                format!("the venue reported an unusable input value {value:?}"),
            ),
        },
    });

    let unit_cap = policy.limits.max_input_units.get(ctx.origin_asset);
    checks.push(match (unit_cap, ctx.amount_in) {
        (None, _) => check(
            "limits.input_units",
            "pass",
            "no unit cap is configured for this asset",
        ),
        (Some(cap), None) => check(
            "limits.input_units",
            "pending",
            format!("input amount is checked against the {cap} unit cap once quoted"),
        ),
        (Some(cap), Some(amount)) => {
            if !crate::input::canonical_amount(amount) {
                check(
                    "limits.input_units",
                    "deny",
                    format!("the venue reported an unusable input amount {amount:?}"),
                )
            } else if less_or_equal_decimal_digits(amount, cap) {
                check(
                    "limits.input_units",
                    "pass",
                    format!("input {amount} units; wallet cap {cap} units"),
                )
            } else {
                check(
                    "limits.input_units",
                    "deny",
                    format!("input {amount} units exceeds the wallet cap of {cap} units"),
                )
            }
        }
    });

    checks.push(if policy.solvers.allowed_deposit_addresses.is_empty() {
        check(
            "solvers.deposit_address",
            "pass",
            "any deposit address from a signed quote is accepted",
        )
    } else {
        match ctx.deposit_address {
            None => check(
                "solvers.deposit_address",
                "pending",
                "the deposit address is checked against the allowlist once quoted",
            ),
            Some(address)
                if contains_ignore_case(&policy.solvers.allowed_deposit_addresses, address) =>
            {
                check(
                    "solvers.deposit_address",
                    "pass",
                    format!("deposit address {address} is allowlisted"),
                )
            }
            Some(address) => check(
                "solvers.deposit_address",
                "deny",
                format!("deposit address {address} is not in solvers.allowed_deposit_addresses"),
            ),
        }
    });

    serde_json::Value::Array(checks)
}

/// The first denial in a set of checks, if any.
pub fn deny_reason(checks: &serde_json::Value) -> Option<String> {
    checks.as_array()?.iter().find_map(|entry| {
        (entry.get("outcome")?.as_str()? == "deny").then(|| {
            format!(
                "venue policy denied [{}]: {}",
                entry.get("rule").and_then(|v| v.as_str()).unwrap_or("?"),
                entry.get("message").and_then(|v| v.as_str()).unwrap_or("")
            )
        })
    })
}

fn describe_recipients(policy: &VenuePolicy, wallet_address: &str) -> String {
    let mut described: Vec<String> = policy
        .recipients
        .allowed
        .iter()
        .map(|entry| {
            if entry == WALLET_ADDRESS_CLASS {
                format!("its own address {wallet_address}")
            } else {
                entry.clone()
            }
        })
        .collect();
    if described.is_empty() {
        described.push("nothing".into());
    }
    described.join(", ")
}

fn contains_ignore_case(set: &BTreeSet<String>, needle: &str) -> bool {
    set.iter().any(|entry| entry.eq_ignore_ascii_case(needle))
}

/// Compare two canonical integers of any length.
fn less_or_equal_decimal_digits(left: &str, right: &str) -> bool {
    if left.len() != right.len() {
        return left.len() < right.len();
    }
    left <= right
}

/// Parse a positive decimal amount such as "250" or "12.50" into hundredths of
/// a unit. Returns `None` for anything else, including negatives.
fn parse_decimal(value: &str) -> Option<u128> {
    let trimmed = value.trim();
    let (whole, fraction) = trimmed.split_once('.').unwrap_or((trimmed, ""));
    if whole.is_empty()
        || whole.len() > 24
        || fraction.len() > 18
        || !whole.bytes().all(|byte| byte.is_ascii_digit())
        || !fraction.bytes().all(|byte| byte.is_ascii_digit())
    {
        return None;
    }
    // Two decimal places is enough to compare money. Both the cap and the value
    // being checked truncate here, so the comparison can admit a value that
    // exceeds the cap by less than a cent. That slack is bounded and immaterial
    // against a USD ceiling, and it is the only rounding the check allows:
    // a value this cannot read at all is refused rather than rounded.
    let mut cents = fraction.chars().chain("00".chars());
    let tenths = cents.next()?.to_digit(10)? as u128;
    let hundredths = cents.next()?.to_digit(10)? as u128;
    let whole: u128 = whole.parse().ok()?;
    whole
        .checked_mul(100)?
        .checked_add(tenths * 10 + hundredths)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context<'a>(recipient: &'a str, wallet: &'a str) -> SwapContext<'a> {
        SwapContext {
            wallet_address: wallet,
            recipient,
            slippage_bps: 50,
            origin_chain: "arbitrum",
            destination_chain: "arb",
            origin_asset: "nep141:arb-0xaaaa.omft.near",
            amount_in: Some("1000000"),
            amount_in_usd: Some("1.00"),
            deposit_address: Some("0xdead00000000000000000000000000000000beef"),
        }
    }

    const WALLET: &str = "0x1111111111111111111111111111111111111111";

    #[test]
    fn bundled_defaults_allow_a_swap_back_to_the_wallet() {
        let policy = defaults();
        assert!(policy.venue.enabled);
        assert_eq!(policy.limits.max_slippage_bps, 100);
        assert_eq!(policy.limits.max_input_usd.as_deref(), Some("250"));
        assert_eq!(
            policy.recipients.allowed,
            [WALLET_ADDRESS_CLASS.to_string()].into_iter().collect()
        );
        assert!(policy.solvers.allowed_deposit_addresses.is_empty());
        let checks = evaluate(&policy, &context(WALLET, WALLET));
        assert_eq!(deny_reason(&checks), None);
    }

    #[test]
    fn defaults_refuse_paying_someone_else() {
        let other = "0x2222222222222222222222222222222222222222";
        let checks = evaluate(&defaults(), &context(other, WALLET));
        let reason = deny_reason(&checks).expect("a foreign recipient is denied");
        assert!(reason.contains("recipients.allowed"), "{reason}");
        assert!(reason.contains(WALLET), "{reason}");
    }

    #[test]
    fn a_custodian_can_pin_payouts_to_its_own_addresses() {
        let treasury = "0x3333333333333333333333333333333333333333";
        let policy = parse(
            format!(
                "[recipients]\nallowed = [\"{treasury}\"]\n[chains]\nallowed_origin = [\"arbitrum\"]\n"
            )
            .as_bytes(),
        )
        .unwrap();
        assert_eq!(
            deny_reason(&evaluate(&policy, &context(treasury, WALLET))),
            None
        );
        // With the class dropped, even the wallet's own address is refused.
        let reason = deny_reason(&evaluate(&policy, &context(WALLET, WALLET))).unwrap();
        assert!(reason.contains("recipients.allowed"), "{reason}");
        // And an origin chain outside the list is refused.
        let mut elsewhere = context(treasury, WALLET);
        elsewhere.origin_chain = "base";
        let reason = deny_reason(&evaluate(&policy, &elsewhere)).unwrap();
        assert!(reason.contains("chains.origin"), "{reason}");
    }

    #[test]
    fn caps_refuse_an_oversized_or_unpriced_input() {
        let policy = defaults();
        let mut over = context(WALLET, WALLET);
        over.amount_in_usd = Some("250.01");
        let reason = deny_reason(&evaluate(&policy, &over)).unwrap();
        assert!(reason.contains("limits.input_usd"), "{reason}");

        let mut unusable = context(WALLET, WALLET);
        unusable.amount_in_usd = Some("lots");
        assert!(
            deny_reason(&evaluate(&policy, &unusable))
                .unwrap()
                .contains("limits.input_usd")
        );

        let mut at_cap = context(WALLET, WALLET);
        at_cap.amount_in_usd = Some("250");
        assert_eq!(deny_reason(&evaluate(&policy, &at_cap)), None);
    }

    #[test]
    fn unit_caps_hold_whatever_the_venue_prices() {
        let asset = "nep141:arb-0xaaaa.omft.near";
        let policy = parse(
            format!("[limits]\n[limits.max_input_units]\n\"{asset}\" = \"1000000\"\n").as_bytes(),
        )
        .unwrap();
        let mut ctx = context(WALLET, WALLET);
        ctx.amount_in = Some("1000000");
        assert_eq!(deny_reason(&evaluate(&policy, &ctx)), None);
        ctx.amount_in = Some("1000001");
        assert!(
            deny_reason(&evaluate(&policy, &ctx))
                .unwrap()
                .contains("limits.input_units")
        );
        // A longer number is larger whatever its digits.
        ctx.amount_in = Some("10000000");
        assert!(deny_reason(&evaluate(&policy, &ctx)).is_some());
    }

    #[test]
    fn slippage_above_the_cap_is_denied() {
        let mut ctx = context(WALLET, WALLET);
        ctx.slippage_bps = 101;
        let reason = deny_reason(&evaluate(&defaults(), &ctx)).unwrap();
        assert!(reason.contains("limits.slippage"), "{reason}");
    }

    #[test]
    fn a_deposit_allowlist_refuses_other_addresses() {
        let listed = "0xdead00000000000000000000000000000000beef";
        let policy =
            parse(format!("[solvers]\nallowed_deposit_addresses = [\"{listed}\"]\n").as_bytes())
                .unwrap();
        let mut ctx = context(WALLET, WALLET);
        assert_eq!(deny_reason(&evaluate(&policy, &ctx)), None);
        ctx.deposit_address = Some("0x4444444444444444444444444444444444444444");
        assert!(
            deny_reason(&evaluate(&policy, &ctx))
                .unwrap()
                .contains("solvers.deposit_address")
        );
        // Before the quote exists the check is pending, not a denial.
        ctx.deposit_address = None;
        assert_eq!(deny_reason(&evaluate(&policy, &ctx)), None);
    }

    #[test]
    fn a_disabled_venue_denies_every_swap() {
        let policy = parse(b"[venue]\nenabled = false\n").unwrap();
        assert!(
            deny_reason(&evaluate(&policy, &context(WALLET, WALLET)))
                .unwrap()
                .contains("venue.enabled")
        );
    }

    #[test]
    fn malformed_policies_are_refused_rather_than_ignored() {
        assert!(parse(b"").is_err());
        assert!(parse(b"[venue]\nenabled = \"yes\"\n").is_err());
        assert!(parse(b"[unknown]\nx = 1\n").is_err());
        assert!(parse(b"[limits]\nmax_slippage_bps = 10001\n").is_err());
        assert!(parse(b"[limits]\nmax_input_usd = \"-5\"\n").is_err());
        assert!(parse(b"[recipients]\nallowed = [\"\"]\n").is_err());
    }

    #[test]
    fn money_parses_without_floating_point() {
        assert_eq!(parse_decimal("250"), Some(25_000));
        assert_eq!(parse_decimal("12.5"), Some(1_250));
        assert_eq!(parse_decimal("0.019"), Some(1));
        assert_eq!(parse_decimal(""), None);
        assert_eq!(parse_decimal("-1"), None);
        assert_eq!(parse_decimal("1e6"), None);
    }

    #[test]
    fn a_class_typo_is_refused_rather_than_stored_as_an_address() {
        // "class:" is reserved, so a near-miss cannot be quietly kept as a
        // literal address that then matches nothing and denies every swap.
        for entry in [
            "class:wallet-address",
            "class:wallet_addresses",
            "CLASS:WALLET",
            "class:",
        ] {
            let body = format!(
                "[venue]\nenabled = true\n[limits]\nmax_slippage_bps = 100\n[limits.max_input_units]\n[recipients]\nallowed = [{entry:?}]\n[chains]\nallowed_origin = []\nallowed_destination = []\n[solvers]\nallowed_deposit_addresses = []\n"
            );
            let error = parse(body.as_bytes()).unwrap_err();
            assert!(error.contains("not a known class"), "{entry}: {error}");
        }
    }

    #[test]
    fn an_entry_with_whitespace_is_refused() {
        let body = "[venue]\nenabled = true\n[limits]\nmax_slippage_bps = 100\n[limits.max_input_units]\n[recipients]\nallowed = [\"0xabc 0xdef\"]\n[chains]\nallowed_origin = []\nallowed_destination = []\n[solvers]\nallowed_deposit_addresses = []\n";
        let error = parse(body.as_bytes()).unwrap_err();
        assert!(error.contains("whitespace"), "{error}");
    }

    #[test]
    fn a_non_evm_payout_address_is_still_allowed() {
        // A destination chain may not use EVM addresses, so entries are not
        // parsed as 0x addresses.
        for entry in [
            "treasury.near",
            "7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU",
            "0x48aae23db69acae2da2f6bf43ec5eb1996cb1245",
        ] {
            let body = format!(
                "[venue]\nenabled = true\n[limits]\nmax_slippage_bps = 100\n[limits.max_input_units]\n[recipients]\nallowed = [{entry:?}]\n[chains]\nallowed_origin = []\nallowed_destination = []\n[solvers]\nallowed_deposit_addresses = []\n"
            );
            let policy = parse(body.as_bytes()).unwrap_or_else(|e| panic!("{entry}: {e}"));
            assert!(policy.recipient_allowed(entry, "0x0000000000000000000000000000000000000000"));
        }
    }
}
