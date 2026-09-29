#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BLOOM_REPO="${BLOOM_REPO:?set BLOOM_REPO to the Bloom checkout supporting explicit wallet/index routes and exact shared_keys storage}"

# This executes the installed built WASM package in a VM fixture with synthetic
# authenticated numbered accounts. It exercises routing, persistent private
# global service settings, restart and redaction without a daemon, live wallet or network.
export BLOOM_HD_PACKAGE_ROOT="${BLOOM_HD_PACKAGE_ROOT:-$(dirname "$ROOT")}"
export BLOOM_HD_PACKAGE_NAMES="near-intents"
cargo test --manifest-path "$BLOOM_REPO/Cargo.toml" -p bloom-petals \
  --test explicit_account_packages --locked -- --ignored --nocapture
