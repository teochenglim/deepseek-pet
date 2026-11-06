#!/usr/bin/env bash
# Remove every deepseek-pet hook entry from Claude Code and Z.ai ZCode configs. Foreign hooks stay.
# Extra flags pass through (--only claude|zai, --claude-settings PATH, --zcode-config PATH).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
HOOK="$ROOT/desktop/cross-platform/hooks/pet-hook.sh"
exec python3 "$ROOT/tools/install_hooks.py" uninstall --hook-script "$HOOK" "$@"
