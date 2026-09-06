#!/usr/bin/env bash
set -euo pipefail

project_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$project_dir"
[[ $# == 1 && "$1" == /* && -f "$1" && -x "$1" && ! -L "$1" ]] || {
  printf 'usage: %s ABSOLUTE_RELEASE_BINARY\n' "$0" >&2
  exit 2
}
release_binary="$1"
# Intentional process crashes must not leave core files containing state or
# credentials. This limit only applies to this isolated acceptance subprocess.
ulimit -c 0
# The fixture is compiled from this checkout, which may not be the isolated
# source tree that produced the supplied formal package binary.
npm run build:platform
cargo build --locked --release --target x86_64-unknown-linux-gnu --example runtime_shutdown_probe
node scripts/check-shutdown-runtime.mjs "${CARGO_TARGET_DIR:-$project_dir/target}/x86_64-unknown-linux-gnu/release/examples/runtime_shutdown_probe"
DUFS_TEST_BINARY="$release_binary" cargo test --locked \
  --target x86_64-unknown-linux-gnu --test crash_recovery -- --nocapture
