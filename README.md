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

## Venue policy

Each wallet has its own swap rules at
`/petals/near-intents/settings/wallets/<wallet>/venue.toml`. Read the route to
see the rules in force; write it to replace them. A wallet without a file gets
the bundled defaults, so a new wallet can swap without any setup:

- payouts go to the wallet's own address (`class:wallet_address`);
- requested slippage may not exceed 100 bps;
- an input worth more than 250 USD, as the venue values it, is refused;
- any deposit address from a signed quote is accepted;
- every chain this Petal maps may be used.

The rules are checked before the quote is requested, again against the signed
quote, and again at confirmation, so tightening them stops a swap that was
already quoted. They appear in `plan.md` and in `policy_check.json`. A stored
file that no longer parses refuses swaps rather than falling back to the
defaults.

Changing the file needs no ceremony, so these are guard rails, not custody.
Bloom's wallet policy still decides which package may act and every deposit
transaction is approved by the owner.

Which check stops what:

| Risk | What stops it |
| --- | --- |
| Another Petal, or a hand-staged transaction, sending to a 1Click deposit address | Bloom's wallet policy. Its `petal:near-intents` destination covers only transactions this Petal stages |
| This Petal staging a deposit the owner did not intend | The owner's approval of each deposit transaction, against Bloom's outbox plan |
| A swap paying out somewhere other than the wallet, or exceeding a limit | This venue policy. Bloom cannot check it: the payout happens on the venue's side, not in the transaction Bloom signs |
| The venue naming a deposit address of its choosing | The signed-quote check. The venue policy can only pin deposit addresses for a venue that issues stable ones |

The venue policy is the only check on the payout address, so it matters most
for a custodian. It is also the weakest: whoever can write to the Petal can
change it without a ceremony.

A custodian pins payouts and tightens the limits, for example:

```toml
[limits]
max_slippage_bps = 50
max_input_usd = "5000"

[limits.max_input_units]
# Exact caps that do not depend on the venue's pricing.
"nep141:eth.omft.near" = "1000000000000000000"

[recipients]
allowed = ["0xTREASURY..."]

[chains]
allowed_origin = ["ethereum", "arbitrum"]
allowed_destination = ["arb", "sol"]

[solvers]
# 1Click issues a fresh deposit address per quote and does not name the solver
# behind it, so this list only works with a venue that issues stable addresses.
# Empty means any address a signed quote names.
allowed_deposit_addresses = []
```

Filtering by solver is not possible today: the 1Click quote carries no solver
identity, so no client can enforce it.

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

Swaps use **account 0** of the wallet selected by the route. Supported origins
are EVM chains; Solana addresses can be used as destination recipients.

Wallet paths:

- EVM address: `/wallets/<wallet>/0/address.evm`.
- EVM native balance: `/wallets/<wallet>/0/chains/<chain>/balance.raw`.
- Outbox staging: `/wallets/<wallet>/0/chains/<chain>/outbox/new.tx`.
- Outbox entries: `/wallets/<wallet>/0/chains/<chain>/outbox/<pending|sent|failed>/<id>/`.
- Solana recipient discovery: direct `address`, `balance`, and `balance.json`
  leaves under `/wallets/<wallet>/0/chains/<solana-chain>/`.

Each session records the selected wallet's account-0 address and checks it before
subsequent operations. Before confirmation or inspection, the Petal verifies the
sender and deposit bytes in the session's exact outbox entry. Bloom handles
Broker authorization, simulation, policy, signing, and broadcast through its
canonical transaction host interfaces.

After an ambiguous confirmation, retry `confirm` or `refresh` to inspect the same
outbox entry. The Petal never automatically rebroadcasts it. If inspection remains
pending without a hash, use Bloom's reconciliation and approval surfaces; do not
create another swap to work around the ambiguity.
