#!/bin/bash
# 南琴梨（Kotori Minami）宠物状态 hook 入口。
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
SOURCE="${1:-claude-code}"
PYTHON_BIN="${KOTORI_PET_PYTHON:-/usr/bin/python3}"

run_hook() {
    local script_name="$1"
    shift
    "$PYTHON_BIN" "$SCRIPT_DIR/scripts/$script_name" "$@" || true
}

case "$SOURCE" in
    claude-code)
        run_hook "claude_hook.py"
        ;;
    codex)
        run_hook "codex_hook.py"
        ;;
    zai)
        run_hook "zai_hook.py"
        ;;
    gemini|qwen|kimi|copilot|cursor)
        # Copilot and Cursor pass their event name as the second argument
        run_hook "generic_hook.py" "$SOURCE" "${2:-}"
        ;;
    *)
        echo "Unsupported hook source: $SOURCE" >&2
        exit 1
        ;;
esac
