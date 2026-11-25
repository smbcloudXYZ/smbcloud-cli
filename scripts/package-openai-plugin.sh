#!/usr/bin/env bash
# Builds the ZIP that is uploaded at platform.openai.com/plugins.
#
#   scripts/package-openai-plugin.sh
#
# Writes dist/smbcloud-openai-plugin-<version>.zip with plugin.json, mcp.json
# and assets/ at the root of the archive. Refuses to build when the package
# holds anything the portal rejects or that must never ship: client secrets,
# reviewer credentials, server headers, or an instruction to add another
# MCP server.
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
src="$root/packaging/openai/smbcloud"
command -v jq >/dev/null || { echo "jq is required" >&2; exit 1; }
command -v zip >/dev/null || { echo "zip is required" >&2; exit 1; }

version="$(jq -r .version "$src/plugin.json")"
[ -n "$version" ] && [ "$version" != null ] || { echo "plugin.json has no version" >&2; exit 1; }

fail() { echo "refusing to package: $1" >&2; exit 1; }
for key in clientSecret client_secret test_credentials reviewer_instructions; do
  grep -rq "\"$key\"" "$src" && fail "found \"$key\"; credentials go in the portal, never in the package"
done
jq -e '[.mcpServers[] | select((.headers // {}) | length > 0)] | length == 0' "$src/mcp.json" >/dev/null \
  || fail "an MCP server sets headers; public submissions reject them"
grep -rqiE 'mcp add|--mcp|claude_desktop_config' "$src" \
  && fail "the package tells users to add another MCP server"

mkdir -p "$root/dist"
out="$root/dist/smbcloud-openai-plugin-$version.zip"
rm -f "$out"
(cd "$src" && zip -qrX "$out" plugin.json mcp.json assets -x '*.DS_Store')
echo "$out"
unzip -l "$out" | tail -n +2
