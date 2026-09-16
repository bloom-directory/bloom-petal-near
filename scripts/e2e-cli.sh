#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BLOOM_BIN="${BLOOM_BIN:-bloom}"
TEST_HOME="$(mktemp -d "${TMPDIR:-/tmp}/bloom-near-intents-e2e.XXXXXX")"
TOKEN="near-intents-e2e.jwt.persistence"
daemon_pid=""
cleanup() {
  if [[ -n "$daemon_pid" ]]; then
    kill "$daemon_pid" 2>/dev/null || true
    wait "$daemon_pid" 2>/dev/null || true
  fi
  rm -rf "$TEST_HOME"
}
trap cleanup EXIT

# v0.3 CLI commands use daemon IPC. Serve creates an isolated default home;
# avoid `init`, which also downloads unrelated preinstalled Petals.
start_daemon() {
  "$BLOOM_BIN" -q --home "$TEST_HOME" serve >"$TEST_HOME/daemon.log" 2>&1 &
  daemon_pid=$!
  ready=false
  for _ in {1..100}; do
    if "$BLOOM_BIN" -q --home "$TEST_HOME" vfs ls / >/dev/null 2>&1; then
      ready=true
      break
    fi
    if ! kill -0 "$daemon_pid" 2>/dev/null; then
      cat "$TEST_HOME/daemon.log" >&2
      exit 1
    fi
    sleep 0.1
  done
  if [[ "$ready" != true ]]; then
    cat "$TEST_HOME/daemon.log" >&2
    echo "isolated Bloom daemon did not become ready" >&2
    exit 1
  fi
}
start_daemon
"$BLOOM_BIN" -q --home "$TEST_HOME" petals build "$ROOT" >/dev/null
"$BLOOM_BIN" -q --home "$TEST_HOME" petals install "$ROOT" >/dev/null

root_listing="$("$BLOOM_BIN" -q --home "$TEST_HOME" vfs ls /petals/near-intents)"
grep -q $'settings\tDir' <<<"$root_listing"
grep -q $'swaps\tDir' <<<"$root_listing"

before="$("$BLOOM_BIN" -q --home "$TEST_HOME" vfs cat /petals/near-intents/settings/api-key)"
grep -q '"configured": false' <<<"$before"
grep -q '"source": "unconfigured"' <<<"$before"
grep -q '"storage": "none"' <<<"$before"
"$BLOOM_BIN" -q --home "$TEST_HOME" vfs write /petals/near-intents/settings/api-key --data "$TOKEN" >/dev/null

# Restart the daemon as well as the CLI to prove durable private storage.
kill "$daemon_pid"
wait "$daemon_pid" || true
daemon_pid=""
start_daemon
after="$("$BLOOM_BIN" -q --home "$TEST_HOME" vfs cat /petals/near-intents/settings/api-key)"
status="$("$BLOOM_BIN" -q --home "$TEST_HOME" vfs cat /petals/near-intents/settings/status.json)"
grep -q '"configured": true' <<<"$after"
grep -q '"configured": true' <<<"$status"
grep -q '"source": "private_store"' <<<"$after$status"
grep -q '"storage": "persistent_private_store"' <<<"$after$status"
if grep -q "$TOKEN" <<<"$after$status"; then
  echo "credential was echoed by a public VFS route" >&2
  exit 1
fi

secret_file="$(find "$TEST_HOME" -type f -path '*/secrets/credentials/partner-jwt' | head -n 1)"
test -n "$secret_file"
grep -q --fixed-strings "$TOKEN" "$secret_file"
permissions="$(stat -f '%Lp' "$secret_file" 2>/dev/null || stat -c '%a' "$secret_file")"
test "$permissions" = "600"

echo "Bloom CLI install, route execution, persistent secret, restart, and redaction E2E passed"
