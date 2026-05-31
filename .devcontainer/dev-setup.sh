#!/usr/bin/env bash
# Dev-container setup for codex_somersault — tuned for a RAM-constrained cloud
# container (e.g. the Claude Code cloud container: ~15GB RAM, ample disk).
#
# Target: Ubuntu 24.04 (the base of .devcontainer/Dockerfile and Claude Code
# cloud containers). Idempotent — safe to re-run.
#
# Provisions (mirrors .devcontainer/Dockerfile, plus RAM tuning):
#   - native build deps (clang, openssl, pkg-config, libcap, musl, cmake)
#   - rustup; toolchain VERSION is pinned by codex-rs/rust-toolchain.toml (1.95.0)
#   - cargo-nextest (`just test`) + cargo-insta (TUI snapshot tests) + uv (`just fmt`)
#   - prefetched crate deps (`just install` -> cargo fetch) + host-arch musl target
#   - mold linker (faster links, lower link-time RAM) wired into ~/.cargo/config.toml
#   - RAM safety on a 15GB box: a CARGO_BUILD_JOBS cap (~2GB per concurrent rustc)
#     and an optional swapfile that the kernel only touches under real RAM pressure
#   - RUST_MIN_STACK=8388608 baked into the env so bare `cargo nextest` matches
#     `just test` (some tests stack-overflow on the 2 MiB default)
#
# Disk is assumed ample, so incremental compilation is left ON (faster rebuilds).
#
# Tunables (env):
#   SWAP_GB=16     size of the safety swapfile; set 0 to disable swap entirely
#   SWAPPINESS=10  lower = swap only when RAM is genuinely low (1 = only to avoid OOM)
#   JOBS=<n>       override the computed CARGO_BUILD_JOBS cap
set -euo pipefail

SWAP_GB="${SWAP_GB:-16}"
SWAPPINESS="${SWAPPINESS:-10}"

SUDO=""
if [ "$(id -u)" -ne 0 ]; then SUDO="sudo"; fi

# Resolve repo root from this script's location so it works from any cwd.
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

ARCH="$(uname -m)"
CORES="$(nproc)"
case "$ARCH" in
  x86_64)  GNU_TRIPLE=x86_64-unknown-linux-gnu  ;;
  aarch64) GNU_TRIPLE=aarch64-unknown-linux-gnu ;;
  *)       GNU_TRIPLE="" ;;
esac
RAM_GB=$(( $(awk '/MemTotal/{print $2}' /proc/meminfo) / 1024 / 1024 ))
# ~2GB headroom per concurrent rustc; never exceed core count, never below 1.
JOBS="${JOBS:-$(( RAM_GB / 2 ))}"
[ "$JOBS" -lt 1 ] && JOBS=1
[ "$JOBS" -gt "$CORES" ] && JOBS="$CORES"
export CARGO_BUILD_JOBS="$JOBS"   # also applies to the cargo installs below

echo "==> Host: ${ARCH}, ${CORES} cores, ${RAM_GB}GB RAM -> CARGO_BUILD_JOBS=${JOBS}, SWAP_GB=${SWAP_GB}"

echo "==> [1/6] System build dependencies (Ubuntu/Debian)"
export DEBIAN_FRONTEND=noninteractive
$SUDO apt-get update
# 'universe' (musl-tools, clang, mold) is already enabled on most dev images.
# Only try to add it when missing, and never let a broken add-apt-repository
# (e.g. apt_pkg/python mismatch from extra PPAs) abort setup.
if ! apt-cache policy 2>/dev/null | grep -qi universe; then
  $SUDO apt-get install -y --no-install-recommends software-properties-common || true
  $SUDO add-apt-repository --yes universe \
    || echo "    add-apt-repository unavailable; assuming universe is already enabled"
  $SUDO apt-get update || true
fi
# cmake/libclang-dev: bindgen/native-crate insurance; mold: fast linker (24.04 universe)
$SUDO apt-get install -y --no-install-recommends \
  build-essential curl git ca-certificates \
  pkg-config libcap-dev clang musl-tools libssl-dev just \
  cmake libclang-dev mold
$SUDO rm -rf /var/lib/apt/lists/*

echo "==> [2/6] Rust toolchain (rustup; version pinned by codex-rs/rust-toolchain.toml)"
if ! command -v rustup >/dev/null 2>&1; then
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
fi
# shellcheck disable=SC1091
. "$HOME/.cargo/env" 2>/dev/null || true   # no-op if rust is installed system-wide (already on PATH)
command -v cargo >/dev/null 2>&1 || { echo "cargo not on PATH after rustup setup"; exit 1; }

echo "==> [3/6] Workspace test/format tooling (skipped if already present)"
command -v cargo-nextest >/dev/null 2>&1 || cargo install --locked cargo-nextest
command -v cargo-insta   >/dev/null 2>&1 || cargo install --locked cargo-insta
if ! command -v uv >/dev/null 2>&1; then
  curl -LsSf https://astral.sh/uv/install.sh | sh   # required by `just fmt` (Python SDK)
fi
# shellcheck disable=SC1091
. "$HOME/.local/bin/env" 2>/dev/null || true

# The repo justfile uses `set working-directory`, which needs just >= 1.33.
# apt's just is older. Install from crates.io (cargo install) rather than the
# just.systems script: crates.io is reachable here, but egress allowlists/
# firewalls often 403 arbitrary install-script domains.
if ! just --justfile "$REPO_ROOT/justfile" --list >/dev/null 2>&1; then
  echo "    installing a modern just from crates.io (apt's is too old)"
  cargo install --locked just
  hash -r 2>/dev/null || true
fi

echo "==> [4/6] Pinned toolchain + prefetch crate deps + host-arch musl target"
# `just install` == `rustup show active-toolchain` (installs the pin + components) + `cargo fetch`.
( cd "$REPO_ROOT" && just install )
# musl target is only needed for static/release dist builds — make it non-fatal
# in case static.rust-lang.org is blocked by an egress allowlist.
MUSL_MSG="    (musl target add failed — skipping; only needed for static release builds)"
case "$ARCH" in
  aarch64) rustup target add aarch64-unknown-linux-musl || echo "$MUSL_MSG" ;;
  x86_64)  rustup target add x86_64-unknown-linux-musl  || echo "$MUSL_MSG" ;;
  *) echo "    (unknown arch '$ARCH' — skipping musl target)" ;;
esac

echo "==> [5/6] Build-speed + RAM tuning: env (jobs/stack), mold linker, optional swap"
ENV_FILE="$HOME/.codex-dev-env.sh"
cat > "$ENV_FILE" <<EOF
# Generated by .devcontainer/dev-setup.sh — codex-rs dev tuning (RAM-constrained box).
. "\$HOME/.cargo/env" 2>/dev/null || true
[ -f "\$HOME/.local/bin/env" ] && . "\$HOME/.local/bin/env" 2>/dev/null || true
export RUST_BACKTRACE=1
export CARGO_BUILD_JOBS=${JOBS}   # cap parallel rustc (~2GB/job); override: CARGO_BUILD_JOBS=N just ...
export RUST_MIN_STACK=8388608     # 8 MiB; matches \`just test\` so bare \`cargo nextest\` won't stack-overflow
EOF
MARK="# >>> codex-dev-env >>>"
if ! grep -qF "$MARK" "$HOME/.bashrc" 2>/dev/null; then
  printf '\n%s\n. "%s"\n# <<< codex-dev-env <<<\n' "$MARK" "$ENV_FILE" >> "$HOME/.bashrc"
fi

# mold linker via user-level cargo config. Merges cleanly with the repo's Windows-only
# codex-rs/.cargo/config.toml (different target table), and avoids editing that
# upstream-owned file. Idempotent (guarded on the fuse-ld=mold marker).
CARGO_CFG="${CARGO_HOME:-$HOME/.cargo}/config.toml"
if [ -n "$GNU_TRIPLE" ] && ! grep -q 'fuse-ld=mold' "$CARGO_CFG" 2>/dev/null; then
  mkdir -p "$(dirname "$CARGO_CFG")"
  cat >> "$CARGO_CFG" <<EOF

# >>> codex-dev mold linker >>> (added by .devcontainer/dev-setup.sh)
[target.${GNU_TRIPLE}]
rustflags = ["-C", "link-arg=-fuse-ld=mold"]
# <<< codex-dev mold linker <<<
EOF
  echo "    mold wired into ${CARGO_CFG} for ${GNU_TRIPLE}"
fi

if [ "$SWAP_GB" -gt 0 ]; then
  if swapon --show 2>/dev/null | grep -q .; then
    echo "    swap already active — leaving as-is"
  else
    echo "    creating ${SWAP_GB}G swapfile (used only under RAM pressure; vm.swappiness=${SWAPPINESS})"
    if { $SUDO fallocate -l "${SWAP_GB}G" /swapfile 2>/dev/null \
         || $SUDO dd if=/dev/zero of=/swapfile bs=1M count=$((SWAP_GB*1024)) status=none 2>/dev/null; }; then
      $SUDO chmod 600 /swapfile
      $SUDO mkswap /swapfile >/dev/null
      if $SUDO swapon /swapfile 2>/dev/null; then
        $SUDO sysctl -w "vm.swappiness=${SWAPPINESS}" >/dev/null 2>&1 || true
        echo "    swap enabled."
      else
        echo "    swapon not permitted in this container — removing file, relying on CARGO_BUILD_JOBS=${JOBS}"
        $SUDO rm -f /swapfile
      fi
    else
      echo "    could not allocate swapfile — relying on CARGO_BUILD_JOBS=${JOBS}"
    fi
  fi
else
  echo "    SWAP_GB=0 — swap disabled; relying on CARGO_BUILD_JOBS=${JOBS}"
fi

echo "==> [6/6] Verify"
rustc --version
cargo --version
just --version 2>/dev/null || true
cargo nextest --version 2>/dev/null || true
mold --version 2>/dev/null | head -1 || true
echo "--- memory ---"; free -h 2>/dev/null | sed -n '1,3p'; swapon --show 2>/dev/null || true
echo
echo "Setup complete. Open a new shell (or 'source $ENV_FILE'), then smoke-test:"
echo "  cd $REPO_ROOT && just test -p codex-protocol"
