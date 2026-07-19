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
