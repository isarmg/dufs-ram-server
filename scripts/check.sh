#!/usr/bin/env bash
set -euo pipefail

project_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$project_dir"
# shellcheck source=scripts/lib/toolchain.sh
source "$project_dir/scripts/lib/toolchain.sh"

run() {
  printf '\n==> %s\n' "$*"
  "$@"
}

require() {
  command -v "$1" >/dev/null 2>&1 || {
    printf 'required command is unavailable: %s\n' "$1" >&2
    exit 1
  }
}

require cargo
require git
require node
require npm
require stat

IFS= read -r required_node_version < .node-version
dufs_require_exact_node_version "$project_dir" "$required_node_version" node

shell_scripts=(
  scripts/check.sh
  scripts/check-integration.sh
  scripts/check-release.sh
  scripts/check-coverage.sh
  scripts/check-deployment.sh
  scripts/check-formal-release-e2e.sh
  scripts/check-release-runtime.sh
  scripts/package-release.sh
  scripts/lib/package-release-self-test.sh
  scripts/lib/toolchain.sh
  tests/data/generate_tls_certs.sh
)

run rustc --version
run cargo --version
run node --version
run npm --version
run bash -n "${shell_scripts[@]}"
if command -v shellcheck >/dev/null 2>&1; then
  run shellcheck --severity=warning "${shell_scripts[@]}"
else
  printf '\n==> SKIP: 未安装 ShellCheck；CI 会执行固定版本。\n'
fi

run npm run build:platform
run cargo fmt --all --check
run cargo clippy --locked --target x86_64-unknown-linux-gnu --all-targets --all-features -- -D warnings
run cargo test --locked --target x86_64-unknown-linux-gnu --all-targets --all-features
run node scripts/check-release-workflow.mjs
run npm run check:js
run npm run check:types
run npm run check:docs
run npm run check:independence
run npm run test:frontend:unit
run git diff --check
run git diff --cached --check

printf '\n==> development checks passed; uncommitted work is allowed\n'
