"""Pet hook entry for agents that need only event-name normalisation:
Gemini CLI, Qwen Code, Kimi Code, GitHub Copilot CLI, Cursor CLI.

    pet-hook.sh <source> [event]

`event` is passed by agents whose payload does not name the event (Copilot, Cursor); otherwise it
is read from the payload. Event names are normalised through hooks/agents.json.
"""

import hashlib
import os
import sys
from pathlib import Path

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from common import (  # noqa: E402, I001
    first_present,
    normalize_event,
    platform_dir_from_script,
    process_event,
    read_stdin_json,
)


def pick_event(source, input_data, argv_event):
    raw = first_present(input_data, 'hook_event_name', 'hookEventName', 'event') or argv_event
    event = normalize_event(source, raw)
    # Cursor reports how a turn ended in `status`; an error is the failed state, not "done"
    if source == 'cursor' and event == 'Stop' and input_data.get('status') == 'error':
        return 'StopFailure'
    return event


def session_id_for(source, input_data, cwd):
    sid = first_present(input_data, 'session_id', 'sessionId', 'conversation_id', 'conversationId')
    if not sid:
        sid = os.environ.get('GEMINI_SESSION_ID', '') if source == 'gemini' else ''
    if not sid:  # Copilot sends none: one session per working folder
        sid = f'{source}-' + hashlib.sha1(cwd.encode('utf-8')).hexdigest()[:10]
    return sid


def main():
    source = sys.argv[1] if len(sys.argv) > 1 else ''
    argv_event = sys.argv[2] if len(sys.argv) > 2 else ''
    platform_dir = platform_dir_from_script(__file__)
    input_data = read_stdin_json()
    log_path = str(Path(platform_dir) / 'runtime' / 'hook-events.log')
    cwd = first_present(input_data, 'cwd', 'workingDirectory')

    event = pick_event(source, input_data, argv_event)
    if not source or not event:
        return

    process_event(
        platform_dir=platform_dir,
        source=source,
        hook_event=event,
        session_id=session_id_for(source, input_data, cwd),
        tool_name=first_present(input_data, 'tool_name', 'toolName', 'tool'),
        cwd=cwd,
        log_path=log_path,
    )


if __name__ == '__main__':
    main()
