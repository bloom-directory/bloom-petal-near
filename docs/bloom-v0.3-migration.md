# Bloom v0.3 migration

## Target and references

Contract inspected: Bloom release PR #284 at
`e8d8eb331511d81d50ca0ab5308a8e0aa27f067a` (workspace version 0.3.0).
At inspection time this was an open release candidate, not a released v0.3.0 tag.

Reference migrations:
[Tolly #1](https://github.com/josh-richardson/bloom-petal-tolly/pull/1) at
`389c736dd10cc32f3b3939239639d7152dc74f00` and
[Enso #7](https://github.com/bloom-directory/bloom-petal-enso/pull/7) at
`cd9031ad0b2b516c6a514b575204752bbfc163e5`.
Those migrations explicitly use account 0. This package instead requires the
intended fingerprint and derivation path, as required for exact account binding.

## Contract changes

The public inventory is `wallets/<wallet>/accounts.json`: `wallet_id`, `accounts`,
and optional `accounts_unavailable`. Each row's `public_key_fingerprint`, `path`,
`derivation_profile`, `lifecycle`, and `number` identify the selectable account.
For EVM, `m/44'/60'/0'/0/<n>` encodes the number. Resolve a unique active matching
row and require its declared number to agree with its canonical path. Missing,
retired, ambiguous, malformed, unavailable, or mismatched identities fail closed.

The EVM address is `wallets/<wallet>/<n>/address.evm`; native balance is
`wallets/<wallet>/<n>/chains/<chain>/balance.raw`. Contract reads and chain-ID
checks continue through the canonical mediated chain RPC import, with the exact
bound EVM address in ERC-20 balance calls. Outbox artifacts live beneath the
same numbered chain directory. Direct Solana `address`, `balance`, and
`balance.json` leaves live beneath its numbered chain directory, with no nested
`solana/` component. This package does not add Solana-origin execution.

Session schema 2 persists the account number, fingerprint and path; the prepared
artifact digest and public review include them. Revalidate against current
inventory and host context before side effects. Prior sessions remain readable
but cannot execute without a binding. Confirm/inspect also open the exact
account-scoped outbox intent and compare sender, wallet, chain, chain ID,
outbox ID and prepared deposit bytes. Never resolve an outbox via `latest`.

Typed SDK outbox imports are retained. No guest VFS write or direct RPC broadcast
replaces Broker authorization. A durable pending marker precedes confirmation;
timeouts, traps and failed persistence after that boundary require inspection,
including after quote expiry. A returned approval requirement permits retrying
the same outbox only after the host's authorization flow. Staging ambiguity still
requires manual recovery and never restages automatically.

## Blocking Bloom-owned change

The release candidate's
[`PetalRouter::dispatch_petal`](https://github.com/bloom-directory/bloom/blob/e8d8eb331511d81d50ca0ab5308a8e0aa27f067a/crates/bloom-petals/src/router.rs)
dispatches the root `/petals` mount with no account parameters or account wallet.
`dispatch_for_account` exists internally, but its `for_account` documentation
explicitly says no VFS mount uses it. The
[runner](https://github.com/bloom-directory/bloom/blob/e8d8eb331511d81d50ca0ab5308a8e0aa27f067a/crates/bloom-petals/src/runner.rs)
rejects caller context beginning with `bloom.` and injects the owner fingerprint
only for a trusted account dispatch. Route `[wallet]` parameters and JSON input
are not trusted account context.

**Owner: Bloom (`bloom-petals` router/runner, daemon account/outbox host).** Supply
a supported account-selecting entry point for `/petals/<mount>` that resolves the
Broker account, enforces account-aware package eligibility, and injects trusted
wallet, number and owner fingerprint. Route the same context into stage,
confirmation, inspection and Broker authorization. The daemon's confirmation
and inspection handlers currently check package origin but do not independently
compare the requested account with the staged sender; add that host-side fence
and mismatch tests as part of the change. The guest's numbered intent read is
additional protection, not a substitute for the host enforcement boundary.

This PR consumes host-injected context only. It never writes reserved `bloom.*`
fields and offers no context override setting. Root-mounted execution remains
blocked, including for account 0. Do not advertise complete v0.3 swap support
until the Bloom dispatch change has been integrated and tested.

## Validation scope

Unit/workflow fixtures exercise sparse/reordered inventories, mismatch rejection,
nonzero account address and balance paths, native/ERC-20 staging, approval retries,
outbox ownership, restart, ambiguous broadcast reconciliation, and settlement
submission. These use mocked host imports and do not prove real Broker approval
or RPC simulation. Actual account-scoped staging, simulation, authorization and
submission must be exercised after the Bloom dispatch blocker is resolved.
No live-money swap is authorized or performed by this migration.

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
| `cargo test --manifest-path route/Cargo.toml --locked` | 42 passed |
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
