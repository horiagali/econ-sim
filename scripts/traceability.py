#!/usr/bin/env python3
"""Spec ↔ test traceability (ADR-0014, step 5).

Every acceptance criterion ID (`**AC-XXX-NN**`) found in docs/02-design must
have a test somewhere under crates/ that mentions the ID, and tests must not
be ignored stubs. Criteria in `locked` specs without a live test FAIL the
check; in other specs they are reported only. Also fails if a test mentions an
ID that no spec defines (stale test).
"""
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
AC = re.compile(r"\*\*(AC-[A-Z0-9]+-\d+)\*\*")
STATUS = re.compile(r"^status:\s*(\w+)", re.M)


def test_blocks(text: str) -> list[str]:
    """Split a Rust file into one chunk per `#[test]` item.

    A chunk starts at the doc comments / attributes directly above
    `#[test]` and runs until the next such header, so an ID in the doc
    comment and an `#[ignore]` on the item land in the same chunk.
    """
    lines = text.splitlines()
    starts = []
    for i, line in enumerate(lines):
        if line.strip() == "#[test]":
            j = i
            while j > 0 and lines[j - 1].strip().startswith(("///", "#[")):
                j -= 1
            starts.append(j)
    if not starts:
        return []
    return ["\n".join(lines[a:b]) for a, b in zip(starts, starts[1:] + [len(lines)])]


def main() -> int:
    spec_ids = {}
    for p in (ROOT / "docs" / "02-design").rglob("*.md"):
        text = p.read_text(encoding="utf-8")
        m = STATUS.search(text)
        status = m.group(1) if m else "?"
        for ac in AC.findall(text):
            spec_ids[ac] = (p.relative_to(ROOT).as_posix(), status)
    live, stubs = set(), set()
    for p in (ROOT / "crates").rglob("*.rs"):
        if "target" in p.parts:
            continue
        text = p.read_text(encoding="utf-8")
        for block in test_blocks(text):
            for ac in re.findall(r"AC-[A-Z0-9]+-\d+", block):
                (stubs if "#[ignore" in block else live).add(ac)
    errors, notes = [], []
    for ac, (path, status) in sorted(spec_ids.items()):
        if ac not in live:
            msg = f"{ac} ({path}, {status}) has no live test" + (" (stub only)" if ac in stubs else "")
            (errors if status == "locked" else notes).append(msg)
    for ac in sorted((live | stubs) - set(spec_ids)):
        errors.append(f"{ac} is tested but not defined in any spec (stale test?)")
    for n in notes:
        print("note:", n)
    if errors:
        print("TRACEABILITY FAILED:")
        for e in errors:
            print("  -", e)
        return 1
    covered = len([a for a in spec_ids if a in live])
    print(f"OK — {covered}/{len(spec_ids)} acceptance criteria have live tests; no stale test IDs.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
