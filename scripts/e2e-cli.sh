#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BLOOM_BIN="${BLOOM_BIN:-bloom}"
HOME_DIR="$(mktemp -d "${TMPDIR:-/tmp}/bloom-near-intents-e2e.XXXXXX")"
TOKEN="near-intents-e2e.jwt.persistence"
trap 'rm -rf "$HOME_DIR"' EXIT

"$BLOOM_BIN" -q --home "$HOME_DIR" init >/dev/null
"$BLOOM_BIN" -q --home "$HOME_DIR" petals install "$ROOT" >/dev/null

root_listing="$("$BLOOM_BIN" -q --home "$HOME_DIR" vfs ls /petals/near-intents)"
grep -q $'settings\tDir' <<<"$root_listing"
grep -q $'swaps\tDir' <<<"$root_listing"

before="$("$BLOOM_BIN" -q --home "$HOME_DIR" vfs cat /petals/near-intents/settings/api-key)"
grep -q '"configured": false' <<<"$before"
"$BLOOM_BIN" -q --home "$HOME_DIR" vfs write /petals/near-intents/settings/api-key --data "$TOKEN" >/dev/null

# A new CLI process proves the private store survives daemon/process lifetime.
after="$("$BLOOM_BIN" -q --home "$HOME_DIR" vfs cat /petals/near-intents/settings/api-key)"
status="$("$BLOOM_BIN" -q --home "$HOME_DIR" vfs cat /petals/near-intents/settings/status.json)"
grep -q '"configured": true' <<<"$after"
grep -q '"configured": true' <<<"$status"
if grep -q "$TOKEN" <<<"$after$status"; then
  echo "credential was echoed by a public VFS route" >&2
  exit 1
fi

secret_file="$(find "$HOME_DIR" -type f -path '*/secrets/credentials/partner-jwt' | head -n 1)"
test -n "$secret_file"
grep -q --fixed-strings "$TOKEN" "$secret_file"
permissions="$(stat -f '%Lp' "$secret_file" 2>/dev/null || stat -c '%a' "$secret_file")"
test "$permissions" = "600"

echo "Bloom CLI install, route execution, persistent secret, restart, and redaction E2E passed"
