#!/usr/bin/env bash
set -euo pipefail

project_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$project_dir"

deployment_self_test=false
if (($# == 1)) && [[ "$1" == "--deployment-self-test" ]]; then
  deployment_self_test=true
elif (($# != 0)); then
  printf 'usage: %s [--deployment-self-test]\n' "$0" >&2
  exit 2
fi

if [[ "$deployment_self_test" == true ]]; then
  ./scripts/check-deployment.sh --self-test
fi

npm run build:platform
binary="$(node scripts/build-frontend-candidate.mjs --print-path)"
./scripts/check-deployment.sh
DUFS_FRONTEND_BINARY="$binary" npm run test:frontend:run
