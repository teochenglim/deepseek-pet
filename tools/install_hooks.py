#!/usr/bin/env python3
"""Install or remove deepseek-pet hooks in Claude Code and Z.ai ZCode configs.

Idempotent: managed entries carry the marker `# deepseek-pet` at the end of the command, so
repeat runs replace them instead of duplicating, and foreign hooks are never touched.
A one-time backup `<file>.deepseek-pet.bak` is written before the first change.
"""
import argparse
import json
import os
import sys
import tempfile
from pathlib import Path

MARKER = "# deepseek-pet"
CLAUDE_EVENTS = [
    "Notification", "PermissionRequest", "PostToolUse", "PreCompact", "PreToolUse",
    "SessionEnd", "SessionStart", "Stop", "StopFailure", "SubagentStop", "UserPromptSubmit",
]
# ZCode events (zcode.z.ai/en/docs/hooks)
ZAI_EVENTS = [
    "SessionStart", "UserPromptSubmit", "PreToolUse", "PermissionRequest",
    "PostToolUse", "PostToolUseFailure", "Stop",
]


def is_managed(entry):
    return isinstance(entry, dict) and str(entry.get("command", "")).rstrip().endswith(MARKER)


def load(path):
    if not path.exists():
        return {}
    text = path.read_text(encoding="utf-8")
    if not text.strip():
        return {}
    try:
        data = json.loads(text)
    except json.JSONDecodeError as e:
        raise SystemExit(f"{path} is not valid JSON ({e}); fix it first, nothing was changed")
    if not isinstance(data, dict):
        raise SystemExit(f"{path} must hold a JSON object")
    return data


def save(path, data):
    path.parent.mkdir(parents=True, exist_ok=True)
    if path.exists():
        bak = path.with_name(path.name + ".deepseek-pet.bak")
        if not bak.exists():
            bak.write_bytes(path.read_bytes())
    fd, tmp = tempfile.mkstemp(dir=path.parent, prefix=path.name + ".")
    with os.fdopen(fd, "w", encoding="utf-8") as f:
        json.dump(data, f, indent=2, ensure_ascii=False)
        f.write("\n")
    os.replace(tmp, path)


def strip_managed(groups):
    """Drop managed entries from matcher groups; drop groups left empty."""
    out = []
    for g in groups if isinstance(groups, list) else []:
        inner = [h for h in g.get("hooks", []) if not is_managed(h)] if isinstance(g, dict) else None
        if inner is None:
            out.append(g)
        elif inner:
            out.append({**g, "hooks": inner})
    return out


def apply(events_container, events, entry, install):
    for ev in events:
        groups = strip_managed(events_container.get(ev, []))
        if install:
            groups.append({"hooks": [entry]})
        if groups:
            events_container[ev] = groups
        else:
            events_container.pop(ev, None)


def claude(path, hook, install):
    data = load(path)
    hooks = data.setdefault("hooks", {})
    cmd = f"{hook} claude-code {MARKER}"
    apply(hooks, CLAUDE_EVENTS, {"type": "command", "command": cmd}, install)
    if not hooks:
        data.pop("hooks")
    save(path, data)


def zai(path, hook, install):
    data = load(path)
    hooks = data.setdefault("hooks", {})
    if install:
        hooks["enabled"] = True
    events = hooks.setdefault("events", {})
    cmd = f"{hook} zai {MARKER}"
    apply(events, ZAI_EVENTS, {"type": "command", "command": cmd, "enabled": True, "timeout": 10}, install)
    if not events:
        hooks.pop("events")
    save(path, data)


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("action", choices=["install", "uninstall"])
    ap.add_argument("--hook-script", required=True, type=Path, help="absolute path to pet-hook.sh")
    ap.add_argument("--claude-settings", type=Path, default=Path.home() / ".claude" / "settings.json")
    ap.add_argument("--zcode-config", type=Path, default=Path.home() / ".zcode" / "cli" / "config.json")
    ap.add_argument("--only", choices=["claude", "zai"], help="touch just one agent")
    args = ap.parse_args(argv)

    hook = args.hook_script.resolve()
    if args.action == "install" and not hook.is_file():
        raise SystemExit(f"hook script not found: {hook}")
    install = args.action == "install"
    if args.only != "zai":
        claude(args.claude_settings, str(hook), install)
        print(f"{args.action}: Claude Code -> {args.claude_settings}")
    if args.only != "claude":
        if args.zcode_config.parent.exists() or args.zcode_config.exists():
            zai(args.zcode_config, str(hook), install)
            print(f"{args.action}: Z.ai ZCode -> {args.zcode_config}")
        else:
            print(f"skip: ZCode config dir {args.zcode_config.parent} not found")
    return 0


if __name__ == "__main__":
    sys.exit(main())
