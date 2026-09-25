# NEAR Intents 1Click Petal

This Bloom Petal requests signed 1Click quotes, prepares native-currency or
ERC-20 deposits from supported EVM origins, stages them through Bloom's generic
transaction outbox, and tracks settlement without exposing wallet keys or the
persisted partner JWT.

## Build and test

The route crate and `petal-build.toml` pin the canonical SDK and builder from
[`bloom-directory/petal`](https://github.com/bloom-directory/petal); this
repository does not carry a private WIT, SDK, or builder copy.

```sh
cargo test --manifest-path route/Cargo.toml
scripts/build.sh
BLOOM_REPO=/path/to/bloom scripts/validate.sh
# Or run the isolated daemon/package smoke test with an already built binary:
BLOOM_BIN=/path/to/bloom scripts/e2e-cli.sh
```

After installation, write the 1Click partner JWT once to
`/petals/near-intents/wallets/<wallet>/<account>/settings/api-key`. It is stored in Bloom's persistent
private store. Reads return configuration status only and never echo the key.

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

Select a wallet and numbered account under `/petals/near-intents/wallets/<wallet>/<account>/`. Petal operations and settings live below that directory. Account 0 keeps its existing private records; other accounts have separate stores. The core wallet tree remains `/wallets/<wallet>/<account>/`.
