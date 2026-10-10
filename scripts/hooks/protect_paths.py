#!/usr/bin/env python3
"""Claude Code PreToolUse hook: block agent edits to protected test paths.

ADR-0014 layer 1. Golden runs, acceptance tests and schemas may only be
changed in a deliberate test-authoring session. To allow it for one session,
start Claude Code with the environment variable ECON_TEST_AUTHORING=1.

Exit code 2 blocks the tool call and shows the message to the agent.
"""
import fnmatch
import json
import os
import re
import sys

PROTECTED = [
    "tests/golden/*",
    "tests/golden/**",
    "crates/*/tests/acceptance/*",
    "crates/*/tests/acceptance/**",
    "schema/*",
    "schema/**",
]


# Shell commands that write to a protected path (heuristic, Spike 9 finding:
# an agent could otherwise bypass the Edit/Write hook with `sed -i`, `>`, …).
PROTECTED_RE = re.compile(r"(tests[/\\]golden|crates[/\\][^\s/\\]+[/\\]tests[/\\]acceptance|(^|[\s/\\])schema[/\\])")
WRITE_RE = re.compile(
    r"(>|\btee\b|\bsed\s+-i|\bperl\s+-[a-z]*i|\bmv\b|\bcp\b|\brm\b|\btruncate\b|"
    r"Set-Content|Add-Content|Out-File|Remove-Item|Move-Item|Copy-Item|write_text|open\()"
)


def blocked(what: str) -> int:
    print(
        f"BLOCKED: {what} touches a protected test/golden/schema path (ADR-0014). "
        "Implementation sessions may not change tests. If this change is "
        "intended, stop and ask the owner to run a test-authoring session "
        "(ECON_TEST_AUTHORING=1).",
        file=sys.stderr,
    )
    return 2


def main() -> int:
    try:
        payload = json.load(sys.stdin)
    except json.JSONDecodeError:
        return 0
    if os.environ.get("ECON_TEST_AUTHORING") == "1":
        return 0
    tool_input = payload.get("tool_input") or {}
    command = tool_input.get("command")
    if isinstance(command, str):
        if PROTECTED_RE.search(command) and WRITE_RE.search(command):
            return blocked("this shell command")
        return 0
    path = tool_input.get("file_path") or tool_input.get("notebook_path") or ""
    if not path:
        return 0
    root = os.environ.get("CLAUDE_PROJECT_DIR") or payload.get("cwd") or os.getcwd()
    try:
        rel = os.path.relpath(os.path.abspath(path), os.path.abspath(root))
    except ValueError:  # different drive on Windows
        return 0
    rel = rel.replace("\\", "/")
    if not any(fnmatch.fnmatch(rel, pat) for pat in PROTECTED):
        return 0
    return blocked(rel)


if __name__ == "__main__":
    sys.exit(main())
