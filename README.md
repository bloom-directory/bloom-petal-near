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
