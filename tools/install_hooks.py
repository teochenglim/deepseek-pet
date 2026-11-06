#!/usr/bin/env python3
"""Install, remove or report deepseek-pet hooks for the supported coding agents.

What to write where comes from hooks/agents.json (one entry per agent, a `format` each).
Guarantees for every agent:
- idempotent: our entries carry the marker `# deepseek-pet` (comment lines for TOML), so a
  repeat run replaces them instead of duplicating, and foreign hooks are never touched;
- a one-time backup `<file>.deepseek-pet.bak` is written before the first change;
- a file that cannot be parsed is never overwritten;
- uninstall removes only our entries (and a file that only held ours).

    install_hooks.py status    --hook-script PET-HOOK.SH   # JSON: present / installed per agent
    install_hooks.py install   --hook-script PET-HOOK.SH [--only id,id]
    install_hooks.py uninstall --hook-script PET-HOOK.SH [--only id,id]
"""
import argparse
import json
import os
import re
import shlex
import sys
import tempfile
from pathlib import Path

MARKER = "# deepseek-pet"
HERE = Path(__file__).resolve().parent
TOML_BEGIN = "# >>> deepseek-pet (managed block, do not edit)"
TOML_END = "# <<< deepseek-pet"


def agents_file():
    for candidate in (HERE / "agents.json", HERE.parent / "desktop" / "cross-platform" / "hooks" / "agents.json"):
        if candidate.is_file():
            return candidate
    raise SystemExit("agents.json not found next to the installer")


def load_agents():
    return json.loads(agents_file().read_text(encoding="utf-8"))["agents"]


def expand(path, home):
    return Path(str(path).replace("~", str(home), 1)) if str(path).startswith("~") else Path(path)


def is_managed(entry):
    if not isinstance(entry, dict):
        return False
    cmd = str(entry.get("command") or entry.get("bash") or "")
    return cmd.rstrip().endswith(MARKER)


def read_json(path):
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


def backup_once(path):
    if path.exists():
        bak = path.with_name(path.name + ".deepseek-pet.bak")
        if not bak.exists():
            bak.write_bytes(path.read_bytes())


def write_text(path, text):
    path.parent.mkdir(parents=True, exist_ok=True)
    backup_once(path)
    fd, tmp = tempfile.mkstemp(dir=path.parent, prefix=path.name + ".")
    with os.fdopen(fd, "w", encoding="utf-8") as f:
        f.write(text)
    os.replace(tmp, path)


def write_json(path, data):
    write_text(path, json.dumps(data, indent=2, ensure_ascii=False) + "\n")


def native(agent, event):
    return agent.get("native", {}).get(event, event)


def command(hook, agent, event=None):
    """The shell line a hook runs. Agents that do not put the event name in the payload get it as an argument."""
    parts = [shlex.quote(str(hook)), agent["id"]]
    if agent["format"] in ("copilot-file", "cursor-flat"):
        parts.append(native(agent, event))
    return " ".join(parts) + " " + MARKER


# ── JSON "matcher group" shape: hooks.<Event>[] = {hooks: [{type, command}]} ──────────────────

def strip_managed_groups(groups):
    out = []
    for g in groups if isinstance(groups, list) else []:
        inner = [h for h in g.get("hooks", []) if not is_managed(h)] if isinstance(g, dict) else None
        if inner is None:
            out.append(g)
        elif inner:
            out.append({**g, "hooks": inner})
    return out


def apply_groups(container, agent, hook, install, entry_extra=None):
    for ev in agent["events"]:
        name = native(agent, ev)
        groups = strip_managed_groups(container.get(name, []))
        if install:
            entry = {"type": "command", "command": command(hook, agent, ev), **(entry_extra or {})}
            groups.append({"hooks": [entry]})
        if groups:
            container[name] = groups
        else:
            container.pop(name, None)


def do_nested(agent, path, hook, install):
    data = read_json(path)
    hooks = data.setdefault("hooks", {})
    apply_groups(hooks, agent, hook, install)
    if not hooks:
        data.pop("hooks")
    write_json(path, data)


def do_zai(agent, path, hook, install):
    data = read_json(path)
    hooks = data.setdefault("hooks", {})
    if install:
        hooks["enabled"] = True
    events = hooks.setdefault("events", {})
    apply_groups(events, agent, hook, install, {"enabled": True, "timeout": 10})
    if not events:
        hooks.pop("events")
    write_json(path, data)


# ── Cursor: hooks.<event>[] = {command}, version 1 ─────────────────────────────────────────

def do_cursor(agent, path, hook, install):
    data = read_json(path)
    hooks = data.setdefault("hooks", {})
    for ev in agent["events"]:
        name = native(agent, ev)
        flat = [h for h in hooks.get(name, []) if not is_managed(h)] if isinstance(hooks.get(name, []), list) else []
        if install:
            flat.append({"command": command(hook, agent, ev)})
        if flat:
            hooks[name] = flat
        else:
            hooks.pop(name, None)
    if install:
        data.setdefault("version", 1)
    if not hooks:
        data.pop("hooks")
        if set(data) <= {"version"}:
            data = {}
    write_json(path, data)


# ── Copilot: a hooks file that is ours alone ────────────────────────────────────────────────

def do_copilot(agent, path, hook, install):
    if not install:
        if path.exists():
            path.unlink()
        return
    hooks = {}
    for ev in agent["events"]:
        hooks[native(agent, ev)] = [{"type": "command", "bash": command(hook, agent, ev), "timeoutSec": 10}]
    write_json(path, {"version": 1, "hooks": hooks})


# ── Kimi: [[hooks]] tables inside config.toml, kept between marker comments ────────────────

def strip_toml_block(text):
    pattern = re.compile(r"\n?" + re.escape(TOML_BEGIN) + r".*?" + re.escape(TOML_END) + r"\n?", re.S)
    return pattern.sub("\n", text).rstrip("\n")


def toml_str(value):
    return '"' + value.replace("\\", "\\\\").replace('"', '\\"') + '"'


def do_kimi(agent, path, hook, install):
    text = path.read_text(encoding="utf-8") if path.exists() else ""
    base = strip_toml_block(text)
    if install:
        lines = [TOML_BEGIN]
        for ev in agent["events"]:
            lines += [
                "[[hooks]]",
                f"event = {toml_str(native(agent, ev))}",
                'matcher = ""',
                f"command = {toml_str(command(hook, agent, ev))}",
                "timeout = 10",
                "",
            ]
        lines[-1] = TOML_END
        base = (base + "\n\n" if base else "") + "\n".join(lines)
    new = (base + "\n") if base else ""
    if new != text:
        write_text(path, new)


# ── OpenCode: a one-line plugin that re-exports the real plugin ────────────────────────────

def do_opencode(agent, path, hook, install):
    if not install:
        if path.exists() and MARKER in path.read_text(encoding="utf-8"):
            path.unlink()
        return
    plugin = Path(hook).resolve().parent / "opencode-plugin.ts"
    if not plugin.is_file():
        raise SystemExit(f"OpenCode plugin not found: {plugin}")
    # the plugin finds the pet's config through a companion file next to itself
    (plugin.parent / ".kotori-pet-config-dir").write_text(str(plugin.parent.parent) + "\n", encoding="utf-8")
    write_text(path, f'{MARKER}\nexport {{ PetPlugin }} from "file://{plugin}";\n')


WRITERS = {
    "nested": do_nested,
    "zai": do_zai,
    "cursor-flat": do_cursor,
    "copilot-file": do_copilot,
    "kimi-toml": do_kimi,
    "opencode-plugin": do_opencode,
}


def agent_paths(agent, home, overrides):
    config = overrides.get(agent["id"]) or expand(agent["config"], home)
    detect = Path(config).parent if agent["id"] in overrides else expand(agent["detect"], home)
    return Path(config), detect


def is_installed(agent, config):
    try:
        return config.is_file() and MARKER.lstrip("# ") in config.read_text(encoding="utf-8")
    except OSError:
        return False


def status(agents, home, overrides):
    out = {}
    for agent in agents:
        if agent["format"] == "info":
            continue
        config, detect = agent_paths(agent, home, overrides)
        out[agent["id"]] = {"present": detect.exists(), "installed": is_installed(agent, config), "config": str(config)}
    return out


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("action", choices=["install", "uninstall", "status"])
    ap.add_argument("--hook-script", type=Path, help="absolute path to pet-hook.sh")
    ap.add_argument("--home", type=Path, default=Path.home(), help="where ~ points (tests use a temp folder)")
    ap.add_argument("--claude-settings", type=Path, help="override Claude Code's settings.json")
    ap.add_argument("--zcode-config", type=Path, help="override ZCode's config.json")
    ap.add_argument("--only", help="comma-separated agent ids; installs even when the agent's folder is missing")
    args = ap.parse_args(argv)

    agents = [a for a in load_agents() if a["format"] != "info"]
    overrides = {}
    if args.claude_settings:
        overrides["claude-code"] = args.claude_settings
    if args.zcode_config:
        overrides["zai"] = args.zcode_config

    if args.action == "status":
        print(json.dumps(status(agents, args.home, overrides), indent=2))
        return 0

    if not args.hook_script:
        raise SystemExit("--hook-script is required")
    hook = args.hook_script.resolve()
    install = args.action == "install"
    if install and not hook.is_file():
        raise SystemExit(f"hook script not found: {hook}")

    wanted = None
    if args.only:
        wanted = {{"claude": "claude-code"}.get(x.strip(), x.strip()) for x in args.only.split(",") if x.strip()}
        unknown = wanted - {a["id"] for a in agents}
        if unknown:
            raise SystemExit(f"unknown agent(s): {', '.join(sorted(unknown))}")

    for agent in agents:
        if wanted is not None and agent["id"] not in wanted:
            continue
        config, detect = agent_paths(agent, args.home, overrides)
        if wanted is None and not detect.exists():
            print(f"skip: {agent['label']} not found ({detect})")
            continue
        if not install and not config.exists():
            continue
        WRITERS[agent["format"]](agent, config, str(hook), install)
        print(f"{args.action}: {agent['label']} -> {config}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
