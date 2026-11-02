# NEAR Intents 1Click Petal

This Bloom Petal requests signed 1Click quotes, prepares native-currency or
ERC-20 deposits from supported EVM origins, stages them through Bloom's generic
transaction outbox, and tracks settlement without exposing wallet keys or the
persisted partner JWT.

## Swapping

Write a JSON request to `/petals/near-intents/swaps/<wallet>/<index>/new`;
reading that file shows an example. `session_id` is 8 to 64 ASCII letters,
digits, `-` or `_`, and is one-use: every attempt needs a fresh `session_id`.
`amount` is a positive integer string with no leading zeros, sign, decimal
point or exponent. `amount` is in base units of
`origin_asset` for `EXACT_INPUT` (the amount sent) and of `destination_asset`
for `EXACT_OUTPUT` (the amount received). USDC has 6 decimals, so 1 USDC is
`"1000000"`; to receive exactly 0.01 ETH (18 decimals), use `EXACT_OUTPUT`
with `"10000000000000000"`. Asset IDs and decimals are in
`/petals/near-intents/tokens.json`. The origin must be an asset that
`tokens.json` marks `"execution": "executable"` (one of the EVM chains this
Petal maps) on a chain configured in Bloom, because Bloom stages the deposit
there; a `quote_only` origin fails with `origin blockchain is not executable in
Bloom`. The destination can be any chain 1Click supports. Optional fields:
`slippage_bps` (default 100, at most 1000), `deadline_seconds` (300 to 3600,
default 900), `quote_waiting_time_ms` (default 3000, at most 10000) and
`refund_to` (defaults to, and must equal, the wallet address). If the session's `status.json` shows
`quote_failed`, read `last_error` and retry with a new `session_id`.

Each write of `confirm` to `swaps/<wallet>/<index>/<session_id>/confirm`
advances one step:

1. `quoted` -> `prepared`: builds the deposit transaction.
2. `prepared` -> `staged`: stages it in the wallet outbox. The owner has not
   been asked anything yet; this is where the user decides, and an agent stops
   here unless the user asked to submit.
3. `staged` -> `approval_required` or `deposit_broadcast_pending`: asks the
   owner to approve, or broadcasts if no approval is needed.
4. `approval_required` -> `deposit_broadcast_pending`: after the owner
   approves, write `confirm` again. Repeat while the state is
   `approval_required`: on Bloom 0.4.1 a wallet policy approval can come before
   the transaction approval.

Write `confirm`, `y` or `{"confirm":true}`. To accept warnings shown in the
staged plan, write `{"confirm":true,"acknowledge_warnings":true}`. Steps 1-4
are refused once the quote deadline (`quote.deadline` in the session's
`quote.json`) is less than two minutes away. In `deposit_broadcast_pending` or
`deposit_sent`, `confirm` only inspects the existing outbox entry, even after
the deadline.

## Build and test

The route crate and `petal-build.toml` pin the canonical SDK and builder from
[`bloom-directory/petal`](https://github.com/bloom-directory/petal); this
repository does not carry a private WIT, SDK, or builder copy.

```sh
cargo test --manifest-path route/Cargo.toml
scripts/build.sh
BLOOM_REPO=/path/to/bloom scripts/validate.sh
# Or run the installed WASM package with synthetic accounts in the VM fixture:
BLOOM_REPO=/path/to/bloom scripts/e2e-cli.sh
```

Use a Bloom checkout supporting explicit wallet/index routes and exact
`[store].shared_keys` declarations for the WASM fixture.

After installation, write the 1Click partner JWT once to
`/petals/near-intents/settings/api-key`. It is stored in Bloom's persistent
package-global secret store shared by all accounts. Reads return configuration status only and never echo the key.

The implementation contract and security invariants are in
[`docs/2026-07-14-near-intents-petal-design.md`](docs/2026-07-14-near-intents-petal-design.md).

NEAR Intents has no testnet. Normal tests use a mocked 1Click endpoint and do
not broadcast funds. Never run a live-money acceptance test without explicit
authorization and a deliberately funded low-value wallet.

## Releases

Installable Petal packages are built and published by this repository using
immutable SemVer tags. Create a tag from a reviewed commit on `master` and push
it to GitHub. The tag workflow calls the canonical reusable release workflow from
[`bloom-directory/petal`](https://github.com/bloom-directory/petal), which
builds and validates the package before attaching these files to the GitHub
release:

- `near-intents-v<version>.petal.tar.gz`
- `SHA256SUMS`
- `petal-release.json`

Consumers, including Bloom's preinstalled Petal catalog, must pin an exact
release tag, source commit, asset name, and SHA-256 digest. They must not use
`releases/latest` for installation.

The two references in `.github/workflows/release.yml` pin the same full commit
SHA containing the canonical reusable workflow and Petal tooling. Keep both
references immutable and update them together when changing release machinery.

## Wallets and transactions

Swaps use the **selected account** of the wallet selected by the route. Supported origins
are EVM chains; Solana addresses can be used as destination recipients.

Wallet paths:

- EVM address: `/wallets/<wallet>/<account>/address.evm`.
- EVM native balance: `/wallets/<wallet>/<account>/chains/<chain>/balance.raw`.
- Outbox staging: `/wallets/<wallet>/<account>/chains/<chain>/outbox/new.tx`.
- Outbox entries: `/wallets/<wallet>/<account>/chains/<chain>/outbox/<pending|sent|failed>/<id>/`.
- Solana recipient discovery: direct `address`, `balance`, and `balance.json`
  leaves under `/wallets/<wallet>/<account>/chains/<solana-chain>/`.

Before requesting a new executable quote, the Petal requires a matching live
`account.json` projection with an active EVM state and present EVM KeyRef.
Retired, missing, and address-only identities are refused before upstream work.
This lifecycle/key-presence preflight does not grant signing approval.

Each session records the selected wallet's account address and checks it before
subsequent operations. Before confirmation or inspection, the Petal verifies the
sender and deposit bytes in the session's exact outbox entry. Bloom handles
Broker authorization, simulation, policy, signing, and broadcast through its
canonical transaction host interfaces.

After an ambiguous confirmation, retry `confirm` or `refresh` to inspect the same
outbox entry. The Petal never automatically rebroadcasts it. If inspection remains
pending without a hash, use Bloom's reconciliation and approval surfaces; do not
create another swap to work around the ambiguity.

## Account-scoped routes

Operations use `/petals/near-intents/swaps/<wallet>/<index>/`; service credentials use the global `/petals/near-intents/settings/` routes. Bloom resolves the explicit adjacent wallet and canonical numbered index from its live authenticated account projection, then supplies trusted `bloom.wallet` and `bloom.account`. Every index, including 0, has a uniform private store for account state. The manifest shares only the exact service credential keys through the package-global store. Public metadata and documentation remain unscoped. Old packages and custom packages require a separate update; no legacy account-0 storage or old-host fallback is supported. The core wallet tree remains `/wallets/<wallet>/<index>/`.

Existing installed state must be retained through the storage cutover. Pending
sessions contain exact outbox and settlement correlation data needed for manual
reconciliation. This package does not migrate or remove that state. Inspect
pending sessions before replacing an installation; automatic recovery across
the old storage layout is unsupported.
