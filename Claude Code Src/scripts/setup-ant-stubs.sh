#!/bin/bash
# Somersault: recreate ANT-internal package stubs in node_modules.
# Run after `bun install` if @ant/* import resolution fails at runtime.
#
# These packages (@ant/claude-for-chrome-mcp, @ant/computer-use-mcp, etc.)
# are Anthropic-internal and not publishable to npm. They're imported at
# module top-level by files in src/utils/claudeInChrome/ and src/utils/computerUse/
# but those subsystems are gated by feature('CHICAGO_MCP') = false (see
# src/bundle-shim.ts) and dead at runtime. The stubs exist only so module
# resolution doesn't fail at import time.

set -e
cd "$(dirname "$0")/.."

create_stub() {
  local pkg="$1"
  local dir="node_modules/$pkg"
  mkdir -p "$dir"
  cat > "$dir/package.json" <<EOF
{
  "name": "$pkg",
  "version": "0.0.0-stub",
  "type": "module",
  "main": "index.mjs",
  "exports": {
    ".": "./index.mjs",
    "./types": "./types.mjs",
    "./sentinelApps": "./sentinelApps.mjs"
  }
}
EOF
  cat > "$dir/index.mjs" <<'EOF2'
// ANT-internal package stub for Somersault. Runtime-dead (gated by
// feature('CHICAGO_MCP') = false). See scripts/setup-ant-stubs.sh.
export const BROWSER_TOOLS = []
export const DEFAULT_GRANT_FLAGS = {}
export const API_RESIZE_PARAMS = {}
export const targetImageSize = () => null
export const buildComputerUseTools = () => []
export const bindSessionContext = () => null
export const getSentinelCategory = () => null
export default {}
EOF2
  cp "$dir/index.mjs" "$dir/types.mjs"
  cp "$dir/index.mjs" "$dir/sentinelApps.mjs"
  echo "stubbed: $pkg"
}

create_stub "@ant/claude-for-chrome-mcp"
create_stub "@ant/computer-use-mcp"
create_stub "@ant/computer-use-swift"
create_stub "@ant/computer-use-input"
