#!/usr/bin/env bash
# Cloud-container setup for codex_somersault — the Rust agent harness in codex-rs/.
#
# Target: Ubuntu 24.04 (the base used by Claude Code cloud containers and by
# .devcontainer/Dockerfile). Idempotent — safe to re-run.
#
# What it provisions:
#   - native build deps (clang, openssl, pkg-config, libcap, musl) — see .devcontainer/Dockerfile
#   - rustup; the toolchain VERSION is pinned by codex-rs/rust-toolchain.toml (1.93.0)
#   - cargo-nextest (`just test`) and cargo-insta (TUI snapshot tests)
#   - uv (required by `just fmt`, which also formats the Python SDK)
#   - prefetched crate dependencies (`just install` -> cargo fetch)
#
# Not installed (install yourself only if you need them):
#   - Bazel: large; the Cargo path is primary. Needed only for `just bazel-*` / argument-comment-lint.
#   - pnpm/Node: only for `pnpm format` (markdown/JSON) and the TypeScript SDK.
set -euo pipefail

SUDO=""
if [ "$(id -u)" -ne 0 ]; then SUDO="sudo"; fi

# Resolve the repo root from this script's location so it works from any cwd.
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

echo "==> [1/4] System build dependencies (Ubuntu/Debian)"
export DEBIAN_FRONTEND=noninteractive
$SUDO apt-get update
$SUDO apt-get install -y --no-install-recommends software-properties-common
$SUDO add-apt-repository --yes universe            # musl-tools & clang live in 'universe'
$SUDO apt-get update
$SUDO apt-get install -y --no-install-recommends \
  build-essential curl git ca-certificates \
  pkg-config libcap-dev clang musl-tools libssl-dev just \
  cmake libclang-dev                               # insurance for native/bindgen crates
$SUDO rm -rf /var/lib/apt/lists/*

echo "==> [2/4] Rust toolchain (rustup; version pinned by codex-rs/rust-toolchain.toml)"
if ! command -v rustup >/dev/null 2>&1; then
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
fi
# shellcheck disable=SC1091
. "$HOME/.cargo/env"

echo "==> [3/4] Workspace test/format tooling"
cargo install --locked cargo-nextest   # `just test`
cargo install --locked cargo-insta     # TUI snapshot tests
curl -LsSf https://astral.sh/uv/install.sh | sh   # required by `just fmt` (Python SDK)
# shellcheck disable=SC1091
. "$HOME/.local/bin/env" 2>/dev/null || true

echo "==> [4/4] Install pinned toolchain + prefetch crate dependencies"
# `just install` == `rustup show active-toolchain` (installs 1.93.0 + components) + `cargo fetch`.
( cd "$REPO_ROOT" && just install )

echo
echo "Setup complete. Verify with:"
echo "  cd $REPO_ROOT/codex-rs && rustc --version && cargo nextest --version && just test -p codex-protocol"
