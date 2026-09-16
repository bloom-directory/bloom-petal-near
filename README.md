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
`/petals/near-intents/settings/api-key`. It is stored in Bloom's persistent
private store. Reads return configuration status only and never echo the key.

The implementation contract and security invariants are in
[`docs/2026-07-14-near-intents-petal-design.md`](docs/2026-07-14-near-intents-petal-design.md).

NEAR Intents has no testnet. Normal tests use a mocked 1Click endpoint and do
not broadcast funds. Never run a live-money acceptance test without explicit
authorization and a deliberately funded low-value wallet.

## Releases

Installable Petal packages are built and published by this repository. The
first release is `v0.1.0`; later releases use immutable SemVer tags. Create the
tag from a reviewed commit on `master` and push it to GitHub. The tag workflow
calls the canonical reusable release workflow from
[`bloom-directory/petal`](https://github.com/bloom-directory/petal), which
builds and validates the package before attaching these files to the GitHub
release:

- `near-intents-v0.1.0.petal.tar.gz`
- `SHA256SUMS`
- `petal-release.json`

Consumers, including Bloom's preinstalled Petal catalog, must pin an exact
release tag, source commit, asset name, and SHA-256 digest. They must not use
`releases/latest` for installation.

The two references in `.github/workflows/release.yml` pin the same full commit
SHA containing the canonical reusable workflow and Petal tooling. Keep both
references immutable and update them together when changing release machinery.

## Bloom v0.3 account contract

This migration follows [Enso #7](https://github.com/bloom-directory/bloom-petal-enso/pull/7):
NEAR Intents remains wallet-scoped and supports **account 0 only** within the
wallet selected by the route. There is no account-awareness manifest declaration,
account selector in requests, or requirement for trusted account-dispatch fields.
Multi-account support is deferred.

Every wallet route uses the SDK's `wallet_param` helper. Current public paths:

- EVM address: `/wallets/<wallet>/0/address.evm`.
- EVM native balance: `/wallets/<wallet>/0/chains/<chain>/balance.raw`.
- Outbox staging: `/wallets/<wallet>/0/chains/<chain>/outbox/new.tx`.
- Outbox entries: `/wallets/<wallet>/0/chains/<chain>/outbox/<pending|sent|failed>/<id>/`.
- Solana recipient discovery: direct `address`, `balance`, and `balance.json`
  leaves under `/wallets/<wallet>/0/chains/<solana-chain>/`.
  Solana origins remain unsupported.

The account-0 address is persisted with each new session and checked before
subsequent operations. Missing account-0 addresses never fall back to wallet-root
addresses. Nonzero session bindings and account-selector request fields are
rejected. Older sessions without an account binding remain readable but require
manual recovery before execution.

The Petal retains canonical `tx_stage`, `tx_confirm`, and `tx_inspect` host imports
with wallet and chain names. Bloom selects the wallet's account 0 and retains
Broker authorization, simulation, policy, and signing. Before confirmation or
inspection, the Petal verifies the sender and deposit bytes in the exact account-0
outbox entry. No reserved `bloom.*` context fields are fabricated.

After an ambiguous confirmation, retry `confirm` or `refresh` to inspect the same
outbox entry. The Petal never automatically rebroadcasts it. If inspection remains
pending without a hash, use Bloom's reconciliation and approval surfaces; do not
create another swap to work around the ambiguity.

See [the migration contract and validation](docs/bloom-v0.3-migration.md).
