#!/usr/bin/env bash
set -euo pipefail

project_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$project_dir"
required_cargo_audit_version="0.22.2"
IFS= read -r required_node_version < .node-version
# shellcheck source=scripts/lib/toolchain.sh
source "$project_dir/scripts/lib/toolchain.sh"

run() {
  printf '\n==> %s\n' "$*"
  "$@"
}

require() {
  if ! command -v "$1" >/dev/null 2>&1; then
    printf 'required command is unavailable: %s\n' "$1" >&2
    exit 1
  fi
}

require cargo
require git
require node
require npm
require nginx
require stat
require systemd-analyze

# npm only warns for an engine mismatch. Reject it here before audits, builds,
# dependency code, or browser tooling can create a false-green quality result.
dufs_require_exact_node_version "$project_dir" "$required_node_version" node

run rustc --version
run cargo --version
run node --version
run npm --version
run ./scripts/check-shell.sh
run cargo fmt --all --check

# Install the locked packages without lifecycle scripts before executing the
# repository's JavaScript checks, then reject vulnerable dependencies first.
run npm ci --ignore-scripts --no-audit --no-fund
run npm audit --audit-level=high
run node scripts/check-release-workflow.mjs
run npm run check:js
run npm run check:docs
run npm run check:independence
run npm run build:platform
run npm run check:types
run npm run test:frontend:unit

cargo_audit_version="$(cargo audit --version 2>/dev/null)" || {
  printf 'required Cargo subcommand is unavailable: cargo audit\n' >&2
  exit 1
}
expected_cargo_audit_version="cargo-audit-audit $required_cargo_audit_version"
[[ "$cargo_audit_version" == "$expected_cargo_audit_version" ]] || {
  printf 'cargo-audit %s is required; found: %s\n' \
    "$required_cargo_audit_version" \
    "$cargo_audit_version" >&2
  exit 1
}
printf '\n==> %s\n' "$cargo_audit_version"
if [[ "${DUFS_ISOLATED_QUALITY_GATE:-}" == "1" ]]; then
  [[ -n "${DUFS_QUALITY_AUDIT_DB:-}" ]] || {
    printf 'isolated release gate requires its sealed RustSec database\n' >&2
    exit 1
  }
  run cargo audit \
    --db "$DUFS_QUALITY_AUDIT_DB" \
    --no-fetch \
    --no-yanked
elif [[ -n "${DUFS_QUALITY_AUDIT_DB:-}" ]]; then
  printf 'DUFS_QUALITY_AUDIT_DB is reserved for the isolated release gate\n' >&2
  exit 1
else
  run cargo fetch --locked
  run cargo audit --deny yanked
fi

# 发布自测统一聚合 normalize-sbom、third-party notices 与 npm cache seed，
# 避免在总检查入口重复执行或让三者的验证范围发生漂移。
run ./scripts/package-release.sh --self-test
# Deployment smoke compiles the real binary and therefore needs embedded Web
# assets even in a freshly extracted, isolated quality-gate source tree.
if [[ "${DUFS_ISOLATED_QUALITY_GATE:-}" == "1" ]]; then
  # 发布输出路径可以包含 shell 元字符，但 Nginx/sed 部署夹具的临时路径
  # 只接受安全字符。部署脚本仍会在 /tmp 下创建并清理私有随机目录。
  run env TMPDIR=/tmp ./scripts/check-deployment.sh
else
  run ./scripts/check-deployment.sh
fi

run cargo clippy --locked --target x86_64-unknown-linux-gnu --all-targets --all-features -- -D warnings
run cargo test --locked --target x86_64-unknown-linux-gnu --all-targets --all-features
run ./scripts/check-coverage.sh
run cargo build --locked --release --target x86_64-unknown-linux-gnu
release_binary="${CARGO_TARGET_DIR:-$project_dir/target}/x86_64-unknown-linux-gnu/release/dufs"
run bash scripts/check-release-runtime.sh "$release_binary"

run ./node_modules/.bin/tsc --version
run env DUFS_FRONTEND_BINARY="$release_binary" npm run test:frontend:run
if [[ "${DUFS_ISOLATED_QUALITY_GATE:-}" == "1" ]]; then
  printf '\n==> SKIP: 隔离正式发布门不运行未固定的宿主 Microsoft Edge；Chromium 与 Firefox 已作为必需矩阵执行。\n'
elif command -v microsoft-edge >/dev/null 2>&1 || command -v microsoft-edge-stable >/dev/null 2>&1; then
  run env DUFS_FRONTEND_BINARY="$release_binary" npm run test:frontend:run -- --edge --project=edge
else
  printf '\n==> SKIP: 未安装 Microsoft Edge；Chromium 与 Firefox 已作为必需矩阵执行。\n'
fi

if [[ "${DUFS_ISOLATED_QUALITY_GATE:-}" == "1" ]]; then
  [[ "${DUFS_BUILD_GIT_SHA:-}" =~ ^([0-9a-f]{40}|[0-9a-f]{64})$ ]] || {
    printf 'isolated quality gate requires the verified full source commit\n' >&2
    exit 1
  }
  [[ ! -e .git && ! -L .git ]] || {
    printf 'isolated quality source unexpectedly contains Git metadata\n' >&2
    exit 1
  }
  printf \
    '\n==> isolated checks passed; the release packager will re-verify source tree %s\n' \
    "$DUFS_BUILD_GIT_SHA"
else
  run git diff --check
  run git diff --cached --check
  if [[ -n "$(git status --porcelain)" ]]; then
    printf 'working tree is not clean:\n' >&2
    git status --short >&2
    exit 1
  fi
  printf '\n==> all release checks passed; working tree is clean\n'
fi
