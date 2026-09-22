#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# grep, not rg: ripgrep is not installed on the CI runner, and a missing
# command inside `if` reads as "no match", so the check would pass unrun.
if grep -rnE \
  'session_view|session_route|pub fn [a-zA-Z0-9_]+_route[[:space:]]*\(' \
  "$ROOT/route/src" "$ROOT/route/files"; then
  echo "route-facing dispatch facade found; route composition belongs in route/files" >&2
  exit 1
fi

if grep -rnE \
  'crate::(workflow::)?[a-zA-Z0-9_]+_route[[:space:]]*\(' \
  "$ROOT/route/files"; then
  echo "route file delegates to a route-mirroring facade" >&2
  exit 1
fi

route_count="$(
  find "$ROOT/route/files" -type f -name '*.rs' | wc -l | tr -d ' '
)"
if [[ "$route_count" != "27" ]]; then
  echo "expected 27 file-based route controllers, found $route_count" >&2
  exit 1
fi

echo "checked 27 file-based route controllers"
