"""Z.ai ZCode pet hook entry.

ZCode sends the same JSON on stdin as Claude Code (hook_event_name, session_id, cwd,
tool_name), so this only differs by the source label. Its failure event is
PostToolUseFailure, mapped in config state_map.
"""

import os
import sys
from pathlib import Path

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from common import (  # noqa: E402, I001
    first_present,
    platform_dir_from_script,
    process_event,
    read_stdin_json,
)


def main():
    platform_dir = platform_dir_from_script(__file__)
    input_data = read_stdin_json()
    log_path = str(Path(platform_dir) / 'runtime' / 'hook-events.log')

    process_event(
        platform_dir=platform_dir,
        source='zai',
        hook_event=first_present(input_data, 'hook_event_name'),
        session_id=first_present(input_data, 'session_id', default='unknown'),
        tool_name=first_present(input_data, 'tool_name'),
        cwd=first_present(input_data, 'cwd'),
        log_path=log_path,
    )


if __name__ == '__main__':
    main()
