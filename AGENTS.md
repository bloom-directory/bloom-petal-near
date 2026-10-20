# NEAR Intents Petal

Implement and review this package against `docs/2026-07-14-near-intents-petal-design.md`.
Never run a live-money swap without explicit user authorization.

## Route/controller/module shape

- Treat `route/files/**/*.rs` as controllers. Each route file owns its route
  parameters, list/read/write selection, endpoint hint, small response
  projection, and route-specific error conversion.
- Do not add a catch-all router, service layer, or route-facing façade under
  `route/src/`. In particular, do not dispatch endpoint behavior through string
  values such as `session_view(ctx, "quote")` or helpers named after one route.
- Keep route-local facts in the route file: one-off store paths, child lists,
  public JSON projections, and single-endpoint Markdown rendering.
- Route controllers may call typed domain operations for substantial reusable
  behavior. Quote acquisition and verification, swap creation, outbox
  confirmation, settlement refresh, locking, persistence, protocol DTOs, and
  public-data sanitization belong in focused modules under `route/src/`.
- Keep the Bloom host implementation in `route/src/runtime.rs`. Domain modules
  depend on its `Host` trait so workflow behavior remains unit-testable.
- Route files should use the canonical `petal` SDK helpers directly. Do not copy
  framework, WIT, SDK, or builder code into this repository.
- If a module begins to look like an index of route handlers, move the
  endpoint-specific composition back into `route/files/`.
- After changing route or domain code, run
  `scripts/check-route-architecture.sh`, route tests and Clippy,
  `scripts/build.sh`, and `petal check --root .`.

## Wallets and transactions

- The package is wallet-scoped and supports numbered accounts. Routes that act on the
  account (create, confirm, refresh, abandon, latest) resolve Bloom's trusted selection
  with `crate::wallet::account(ctx)`, which wraps the SDK's `petal::wallet_account`;
  read-only session routes use `crate::wallet::param(ctx)`. Both apply the SDK's
  wallet-name grammar. Domain functions that act on the account take the
  `petal::WalletAccount` rather than a wallet string and account number.
- Read EVM identity through `crate::accounts::evm_address`, which uses the SDK's
  `WalletAccount::address_evm_from` on `wallets/<wallet>/<account>/address.evm`. Chain leaves are under
  `wallets/<wallet>/<account>/chains/<chain>/`; Solana address/balance leaves are direct
  children of that directory. Bloom denies Petal VFS access to outboxes.
- Preserve the session's account address binding and inspect its exact outbox entry
  through `tx_inspect` before confirmation or inspection. Bloom answers only for an
  entry this package staged from the route's bound account; the deposit bytes are
  bound by the outbox ID recorded from the session's own `tx_stage`.
- Never construct reserved `bloom.*` parameters. Keep transaction writes on the
  canonical SDK host imports and preserve Broker-mediated authorization.
- Persist ambiguity markers before staging and confirmation. Reconcile ambiguous
  broadcasts through the existing outbox entry without automatically rebroadcasting.

## Account-scoped routes

Operations use `/petals/near-intents/swaps/<wallet>/<index>/`; service credentials use the global `/petals/near-intents/settings/` routes. Bloom resolves the explicit adjacent wallet and canonical numbered index from its live authenticated account projection, then supplies trusted `bloom.wallet` and `bloom.account`. Every index, including 0, has a uniform private store for account state. The manifest shares only the exact service credential keys through the package-global store. Public metadata and documentation remain unscoped. Old packages and custom packages require a separate update; no legacy account-0 storage or old-host fallback is supported. The core wallet tree remains `/wallets/<wallet>/<index>/`.
