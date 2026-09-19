#!/usr/bin/env bash
set -euo pipefail

project_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$project_dir"

if [[ "${1:-}" == "--deployment-self-test" ]]; then
  shift
  ./scripts/check-deployment.sh --self-test
fi
if (($# != 0)); then
  printf 'usage: %s [--deployment-self-test]\n' "$0" >&2
  exit 2
fi

npm run build:platform
cargo build --locked --target x86_64-unknown-linux-gnu
binary="${CARGO_TARGET_DIR:-$project_dir/target}/x86_64-unknown-linux-gnu/debug/dufs"
./scripts/check-deployment.sh
DUFS_FRONTEND_BINARY="$binary" npm run test:frontend:run
