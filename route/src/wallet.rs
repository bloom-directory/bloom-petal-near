//! The wallet and account Bloom selected for a route.

/// Validate a wallet name with the SDK's Bloom wallet-name grammar.
///
/// An on-chain address gets its own message because it is the mistake an
/// agent most often makes here.
pub fn validate_id(value: &str) -> Result<&str, String> {
    if value.len() == 42
        && value.starts_with("0x")
        && value[2..].bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(
            "wallet must be a Bloom wallet id, not an on-chain address; use the id under /wallets/<id>"
                .into(),
        );
    }
    petal::validate_wallet_segment(value)
}

/// Read and validate the generated `[wallet]` route capture.
///
/// The capture is positional, so a caller-supplied `wallet` parameter cannot
/// shadow it as it could through `petal::wallet_param`. This is the wallet
/// [`account`] checks Bloom's trusted selection against.
pub fn param(ctx: &petal::Ctx) -> Result<&str, petal::DispatchResponse> {
    let value = petal::route_generated_param(ctx, "wallet")
        .ok_or_else(|| petal::route_invalid("missing wallet"))?;
    validate_id(value).map_err(petal::route_invalid)
}

/// The wallet and account Bloom selected for this route.
///
/// The SDK reads the pair from trusted context, checks the wallet against the
/// route capture, and has no account-0 default. The wallet-name grammar is
/// checked on top, so this refuses the same names as [`param`].
pub fn account(ctx: &petal::Ctx) -> Result<petal::WalletAccount, petal::DispatchResponse> {
    let account = petal::wallet_account(ctx)?;
    validate_id(account.wallet()).map_err(petal::route_invalid)?;
    Ok(account)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct NewSwap;
    impl petal::RouteIdentity for NewSwap {
        const PATH: &'static str = "swaps/[wallet]/[index]/new";
        const CANONICAL_PATH: &'static str = Self::PATH;
        const PARAMS: &'static [(&'static str, usize)] = &[("wallet", 1), ("index", 2)];
    }

    struct Confirm;
    impl petal::RouteIdentity for Confirm {
        const PATH: &'static str = "swaps/[wallet]/[index]/[id]/confirm";
        const CANONICAL_PATH: &'static str = Self::PATH;
        const PARAMS: &'static [(&'static str, usize)] = &[("wallet", 1), ("index", 2), ("id", 3)];
    }

    fn selected(
        wallet: &str,
        number: u32,
    ) -> Result<petal::WalletAccount, petal::DispatchResponse> {
        let ctx = petal::testing::RouteCtx::new(&format!("swaps/{wallet}/{number}/new"))
            .account(wallet, number)
            .bind::<NewSwap>();
        account(&ctx)
    }

    #[test]
    fn account_selection_requires_trusted_context() {
        let untrusted = petal::testing::RouteCtx::new("swaps/alice/1/new").bind::<NewSwap>();
        assert!(matches!(
            account(&untrusted),
            Err(petal::DispatchResponse::Error { code: -2, .. })
        ));
        for number in [0, 1] {
            let alice = selected("alice", number).expect("a trusted selection");
            assert_eq!((alice.wallet(), alice.number()), ("alice", number));
        }
    }

    #[test]
    fn a_route_refuses_a_wallet_bloom_would_never_select() {
        // Each is refused before any read, store key or upstream request, by
        // the SDK (empty, `..`) or by the wallet-name grammar on top of it.
        for wallet in ["", "..", "test wallet", "wallet.name", "Alice"] {
            assert!(
                matches!(
                    selected(wallet, 0),
                    Err(petal::DispatchResponse::Error { code: -3, .. })
                ),
                "{wallet:?}"
            );
        }
    }

    #[test]
    fn a_caller_wallet_param_cannot_shadow_the_route_capture() {
        // Bloom's router never builds such a context, but if one carried a
        // caller `wallet`, every route here must still act for the capture.
        let new_swap = petal::testing::RouteCtx::new("swaps/alice/0/new")
            .param("wallet", "mallory")
            .account("alice", 0)
            .bind::<NewSwap>();
        let confirm = petal::testing::RouteCtx::new("swaps/alice/0/swap-0001/confirm")
            .param("wallet", "mallory")
            .account("alice", 0)
            .bind::<Confirm>();
        assert_eq!(account(&new_swap).unwrap().wallet(), "alice");
        assert_eq!(account(&confirm).unwrap().wallet(), "alice");
        assert_eq!(param(&confirm), Ok("alice"));
    }

    #[test]
    fn an_on_chain_address_is_pointed_at_the_wallet_tree() {
        let address = "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb1";
        assert!(validate_id(address).unwrap_err().contains("/wallets/<id>"));
    }

    #[test]
    fn matches_bloom_wallet_name_grammar() {
        for valid in ["a", "alice", "release-test", "release_test", "a1"] {
            assert_eq!(validate_id(valid), Ok(valid));
        }
        for invalid in ["", "../etc", "test wallet", "wallet.name", "Alice", "1a"] {
            assert!(validate_id(invalid).is_err(), "{invalid}");
        }
    }
}
