#!/usr/bin/env bash
# Local gate, run by the Stop hook. Mirrors CI minus the non-Linux runners.
set -euo pipefail
cd "$(dirname "$0")/.."
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --quiet -- -D warnings
cargo test --workspace --quiet
cargo check --quiet -p rp-core -p rp-io -p rp-ops -p rp-render --target aarch64-linux-android
