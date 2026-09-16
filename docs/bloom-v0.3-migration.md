# Bloom v0.3 migration

## Target and reference

The implementation follows [Enso #7](https://github.com/bloom-directory/bloom-petal-enso/pull/7)
at `cd9031ad0b2b516c6a514b575204752bbfc163e5`: wallet-scoped routes select the
wallet with `petal::wallet_param(ctx)` and read its canonical account-0 EVM address.
The package does not declare account awareness. Multi-account support is deferred.

Tested Bloom source: release PR #284 at
`e8d8eb331511d81d50ca0ab5308a8e0aa27f067a` (workspace version 0.3.0).
This was an open release candidate at inspection time.

## Supported behavior

The selected wallet's EVM address is `wallets/<wallet>/0/address.evm`; its native
balance is `wallets/<wallet>/0/chains/<chain>/balance.raw`. Outbox artifacts live
beneath that chain directory. Contract reads and chain-ID checks continue through
the canonical mediated chain RPC import, with the persisted account-0 address
in ERC-20 balance calls. Solana recipient discovery uses direct `address`,
`balance`, and `balance.json` leaves beneath `wallets/<wallet>/0/chains/<chain>/`;
there is no additional `solana/` directory. Solana-origin execution is unsupported.

Requests retain their original wallet-scoped shape. They do not accept an account
number, fingerprint or derivation path. Account 0 is the explicit supported
contract, not a fallback after another selection fails. No inventory list position
is used, and missing account-0 identity never falls back to a wallet-root address.

New session schema 2 records account number 0 and the selected wallet's address.
The prepared digest and public review include that binding. Before side effects,
reject nonzero bindings and require the current account-0 address to match the
persisted address. Legacy unbound sessions remain readable but need manual recovery.
Before confirmation or inspection, read the exact account-0 outbox intent and
compare sender, wallet, chain, chain ID, outbox ID and prepared deposit bytes.
Never resolve a session's outbox through `latest`.

Typed SDK outbox imports are retained, with wallet and chain arguments. Bloom's
wallet-scoped host selects account 0 and owns Broker authorization, simulation,
policy and signing. No guest VFS write or direct RPC broadcast replaces those
checks. The root Petal mount does not need account-scoped dispatch for this
wallet-scoped contract, and the guest neither reads nor fabricates reserved
`bloom.*` account parameters. A future multi-account feature will need a supported
host dispatch contract; it is outside this migration.

A durable pending marker precedes confirmation. Timeouts, traps and failed
persistence after that boundary require inspection, including after quote expiry.
A returned approval requirement permits retrying the same outbox through the
host's authorization flow. Staging ambiguity requires manual recovery and never
restages automatically.

## Coverage and limits

Fixtures exercise account 0 without trusted context or selector inputs, multiple
wallet route values, rejection of nonzero sessions, missing canonical addresses,
changed sender addresses, native/ERC-20 staging, Broker approval retries, exact
outbox ownership, moved entries, restart, ambiguous broadcast reconciliation and
settlement submission. These use mocked host imports, so actual RPC simulation
and Broker approval are not claimed as end-to-end tested. The isolated real-Bloom
smoke test validates package installation, route execution and durable settings.
No live-money swap was authorized or performed.

## Validation results (2026-09-16)

- Bloom: built and ran **0.3.0**, PR #284 commit
  `e8d8eb331511d81d50ca0ab5308a8e0aa27f067a`, using its locked dependencies.
- Petal SDK, CLI/builder and reusable release workflow:
  `2beed2ff344ce2b0c112e07096027e1ae0404007` (package version 0.1.0).
  Its WIT is identical to Bloom's pinned contract commit
  `73c5b06a77599368fbc79fb7947a629b5b4c630e`.
  The previous `1af3ba97` pin included a newer private-input interface;
  aligning the toolchain is necessary for the release candidate's contract.
- Rust: `rustc 1.97.1 (8bab26f4f 2026-07-14)`;
  Cargo: `1.97.1 (c980f4866 2026-06-30)`; wasm-tools: `1.257.0`.
- Release-candidate dependency pins: Broker
  `dd2add2b9d41540521d08c77d19fb467a2d8029e`, Signer
  `ccc9adb3866b17b87d2774018dcfa015184b1918`, service runtime
  `5db670e1b7507deabfdcf451be8b5d315c1c9d91`.
  Broker and Signer were not launched for the smoke test.

| Check | Result |
| --- | --- |
| `cargo test --manifest-path route/Cargo.toml --locked` | 41 passed |
| `cargo clippy --manifest-path route/Cargo.toml --locked --all-targets -- -D warnings` | Passed |
| `cargo fmt --manifest-path route/Cargo.toml --all -- --check` | Passed |
| `scripts/check-route-architecture.sh` | 24 controllers passed |
| `scripts/build.sh` | 24 WASM route components built |
| `petal check --root .` (pinned local CLI) | Capability reconciliation passed |
| Bloom `petals build` and `petals install` | Passed through isolated v0.3 daemon |
| `scripts/e2e-cli.sh` with that Bloom binary | Route execution, daemon restart, secret persistence, mode 0600 and public redaction passed |
| `git diff --check`; shell syntax checks | Passed |

WIT digest:
`f52579e1f964eb0302c418f08817593d8f2fc38a434b5f71b483972c56839254`.
The toolchain reports a dependency future-incompatibility notice for
`proc-macro-error2 2.0.1`; current checks pass.

The smoke test starts a fresh temporary daemon and removes it afterward. It
creates no wallet, stages no transaction and performs no broadcast. The prior
script assumed inline CLI execution; v0.3 requires daemon IPC. Package validation
now runs in that isolated daemon instead of requiring the user's running Bloom.
