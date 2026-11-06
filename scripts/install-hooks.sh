#!/usr/bin/env bash
# Install deepseek-pet hooks for every supported agent found on this Mac (see desktop/cross-platform/hooks/agents.json).
# Idempotent: repeat runs replace our entries, foreign hooks are untouched, a one-time .bak is written.
# Extra flags pass through (--only claude-code,gemini  --home DIR  --claude-settings PATH  --zcode-config PATH).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
HOOK="$ROOT/desktop/cross-platform/hooks/pet-hook.sh"
chmod +x "$HOOK"
exec python3 "$ROOT/tools/install_hooks.py" install --hook-script "$HOOK" "$@"
