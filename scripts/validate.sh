#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BLOOM_REPO="${BLOOM_REPO:-}"

"$ROOT/scripts/check-route-architecture.sh"
cargo test --manifest-path "$ROOT/route/Cargo.toml" --locked
cargo clippy --manifest-path "$ROOT/route/Cargo.toml" --locked --all-targets -- -D warnings
"$ROOT/scripts/build.sh"

if [[ -n "${PETAL_BIN:-}" ]]; then
  "$PETAL_BIN" check --root "$ROOT"
elif command -v petal >/dev/null 2>&1; then
  petal check --root "$ROOT"
else
  "$ROOT/target/petal-tool/bin/petal" check --root "$ROOT"
fi

if grep -rqsE --exclude-dir=target 'bloom:sign|allowed_intents' "$ROOT/petal.toml" "$ROOT/route"; then
  echo "NEAR Intents package unexpectedly contains the Bloom signing surface" >&2
  exit 1
fi

if grep -rqsE 'test\.jwt\.must-never-appear' \
  "$ROOT/README.md" "$ROOT/petal.toml" "$ROOT/petal/near-intents" "$ROOT/artifacts" "$ROOT/route/files"; then
  echo "test credential leaked into a public package artifact" >&2
  exit 1
fi

if [ -n "$BLOOM_REPO" ]; then
  BLOOM_REPO="$BLOOM_REPO" "$ROOT/scripts/e2e-cli.sh"
else
  echo "set BLOOM_REPO=/path/to/bloom to validate the installed WASM package" >&2
  exit 127
fi
