#!/usr/bin/env bash
# Print the Claude Code status line exactly as the terminal will show it, using the app binary
# built from this checkout (so the avatar compiled in is the one in hooks/statusline-logo.ansi).
#   scripts/statusline-preview.sh                 # sample numbers
#   echo '{...}' | scripts/statusline-preview.sh -   # your own Claude Code payload
set -eu
cd "$(dirname "$0")/.."
cargo build --quiet --manifest-path desktop/cross-platform/src-tauri/Cargo.toml
bin=desktop/cross-platform/src-tauri/target/debug/deepseek-pet
# a scratch home so the preview never overwrites the real ~/.deepseek-pet/context.json
scratch="$(mktemp -d)"
trap 'rm -rf "$scratch"' EXIT
if [ "${1:-}" = "-" ]; then
  payload="$(cat)"
else
  payload='{"session_id":"preview","model":{"display_name":"Sonnet 5.5"},"workspace":{"current_dir":"'"$PWD"'"},"context_window":{"context_window_size":200000,"used_percentage":42,"current_usage":{"input_tokens":1000,"cache_read_input_tokens":83000}},"cost":{"total_cost_usd":0.42}}'
fi
printf '%s' "$payload" | DEEPSEEK_PET_HOME="$scratch" "$bin" --statusline
