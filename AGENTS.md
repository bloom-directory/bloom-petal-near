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

## Venue policy

- Each wallet's swap rules live at `settings/wallets/<wallet>/venue.toml`, in
  `route/src/policy.rs`, with the bundled defaults in
  `route/src/venue-defaults.toml`. A wallet without a file uses those defaults.
- Enforce the policy before the quote request, again against the signed quote,
  and again at confirmation. Store the resulting checks on the session so
  `plan.md` and `policy_check.json` show what held.
- A stored file that no longer parses fails closed. Never fall back to the
  defaults for a policy the owner wrote.
- These rules are guard rails, not custody: anything that can write the mount
  can change them. Bloom's wallet policy and the per-transaction approval remain
  the checks that matter, so never describe the venue policy as an authority
  boundary.
- Do not add a rule the Petal cannot verify from what it sees. The 1Click quote
  carries no solver identity, so solver filtering is not implementable today.

## Wallets and transactions

- The package is wallet-scoped and supports account 0. Resolve wallet route
  parameters with the canonical `petal::wallet_param(ctx)` helper.
- Read EVM identity from `wallets/<wallet>/0/address.evm`. Chain leaves and outbox
  artifacts are under `wallets/<wallet>/0/chains/<chain>/`; Solana address/balance
  leaves are direct children of that directory.
- Preserve the session's account-0 address binding and verify the sender and
  prepared deposit against its exact outbox entry before confirmation or inspection.
- Never construct reserved `bloom.*` parameters. Keep transaction writes on the
  canonical SDK host imports and preserve Broker-mediated authorization.
- Persist ambiguity markers before staging and confirmation. Reconcile ambiguous
  broadcasts through the existing outbox entry without automatically rebroadcasting.
