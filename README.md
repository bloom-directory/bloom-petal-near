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

This migration targets the v0.3.0 release candidate in
[Bloom #284](https://github.com/bloom-directory/bloom/pull/284), commit
`e8d8eb331511d81d50ca0ab5308a8e0aa27f067a`. Execution is currently blocked:
its `/petals/near-intents` mount supplies no trusted account context. The Petal
refuses to quote, prepare, stage, confirm, or refresh an executable swap without
that context. `[account] aware = true` declares support; it does not create a
host dispatch mechanism. Settings, token discovery, and saved-session reads
remain available.

New requests require `account_fingerprint` and `derivation_path`. Copy the
intended active EVM account's `public_key_fingerprint` and `path` from
`/wallets/<wallet>/accounts.json`. The Petal resolves and verifies its `number`
from that exact pair, cross-checks the host account context, and persists the
binding. It never chooses an account by array order or defaults to account 0.
Every wallet route uses the SDK's `wallet_param` helper.

Current public paths:

- EVM address: `/wallets/<wallet>/<n>/address.evm`.
- EVM native balance: `/wallets/<wallet>/<n>/chains/<chain>/balance.raw`.
- Outbox staging: `/wallets/<wallet>/<n>/chains/<chain>/outbox/new.tx`.
- Outbox entries: `/wallets/<wallet>/<n>/chains/<chain>/outbox/<pending|sent|failed>/<id>/`.
- Solana recipient address and balances: direct `address`, `balance`, and
  `balance.json` leaves under `/wallets/<wallet>/<n>/chains/<solana-chain>/`.
  Resolve the Solana account using its fingerprint and `m/44'/501'/<n>'/0'`
  path in `accounts.json`. Solana origins remain unsupported.

The Petal uses canonical `tx_stage`, `tx_confirm`, and `tx_inspect` host imports;
these receive wallet and chain names, not VFS paths. Bloom must supply their
trusted account context. The Petal also verifies `intent.json` through the
numbered account outbox before confirmation or inspection. Broker authorization,
Bloom simulation, policy, and signing remain host responsibilities.

After an ambiguous confirmation, retry `confirm` or `refresh` to inspect the
same outbox entry. The Petal never automatically rebroadcasts it. If inspection
remains pending without a transaction hash, use Bloom's reconciliation and
approval surfaces; do not create another swap to work around the ambiguity.
Legacy sessions without an exact account binding remain readable but require
manual recovery; they are never assigned account 0.

See [the migration contract and Bloom blocker](docs/bloom-v0.3-migration.md).
