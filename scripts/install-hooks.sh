#!/usr/bin/env bash
# Install deepseek-pet hooks for Claude Code (~/.claude/settings.json) and Z.ai ZCode (~/.zcode/cli/config.json).
# Idempotent: repeat runs replace our entries, foreign hooks are untouched, a one-time .bak is written.
# Extra flags pass through (--only claude|zai, --claude-settings PATH, --zcode-config PATH).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
HOOK="$ROOT/desktop/cross-platform/hooks/pet-hook.sh"
chmod +x "$HOOK"
exec python3 "$ROOT/tools/install_hooks.py" install --hook-script "$HOOK" "$@"
