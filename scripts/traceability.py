#!/usr/bin/env python3
"""Spec ↔ test traceability (ADR-0014, step 5 and Amendment 1).

Every acceptance criterion ID (`**AC-XXX-NN**`) found in docs/02-design must
have a test somewhere under crates/ that mentions the ID, and tests must not
be ignored stubs. In a `locked` spec a criterion without a live test FAILS the
check unless the spec lists it under a "Tests owed" heading, one line per
criterion (`- AC-XXX-NN: what it waits for`); in other specs it is reported
only. Also fails if a test mentions an ID that no spec defines (stale test),
or if a criterion listed as owed has a live test (stale list).
"""
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
AC = re.compile(r"\*\*(AC-[A-Z0-9]+-\d+)\*\*")
STATUS = re.compile(r"^status:\s*(\w+)", re.M)
OWED_HEADING = re.compile(r"^#{2,4}\s+Tests owed\s*$", re.M)
OWED_LINE = re.compile(r"^- (AC-[A-Z0-9]+-\d+):\s*\S")


def owed_ids(text: str) -> set[str]:
    """IDs listed under a "Tests owed" heading, up to the next heading.

    A line counts only if it gives a reason after the ID.
    """
    m = OWED_HEADING.search(text)
    if not m:
        return set()
    owed = set()
    for line in text[m.end():].splitlines():
        if line.startswith("#"):
            break
        hit = OWED_LINE.match(line.strip())
        if hit:
            owed.add(hit.group(1))
    return owed


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


def check(specs: dict[str, str], live: set[str], stubs: set[str]) -> tuple[list[str], list[str], int, int]:
    """Errors, notes, criteria with a live test, criteria in all.

    `specs` maps a spec's path to its text; `live` and `stubs` are the IDs
    cited by live and by ignored tests.
    """
    spec_ids: dict[str, tuple[str, str]] = {}
    owed: dict[str, str] = {}
    for path, text in specs.items():
        m = STATUS.search(text)
        status = m.group(1) if m else "?"
        for ac in AC.findall(text):
            spec_ids[ac] = (path, status)
        for ac in owed_ids(text):
            owed[ac] = path
    errors, notes = [], []
    for ac, (path, status) in sorted(spec_ids.items()):
        if ac in live:
            continue
        msg = f"{ac} ({path}, {status}) has no live test" + (" (stub only)" if ac in stubs else "")
        if status != "locked":
            notes.append(msg)
        elif owed.get(ac) == path:
            notes.append(msg + " (owed)")
        else:
            errors.append(msg + ' and is not listed under "Tests owed"')
    for ac, path in sorted(owed.items()):
        if ac in live:
            errors.append(f'{ac} is listed under "Tests owed" in {path} but has a live test: remove it from the list')
        elif spec_ids.get(ac, ("", ""))[0] != path:
            errors.append(f'{ac} is listed under "Tests owed" in {path} but is not a criterion of that spec')
    for ac in sorted((live | stubs) - set(spec_ids)):
        errors.append(f"{ac} is tested but not defined in any spec (stale test?)")
    covered = len([a for a in spec_ids if a in live])
    return errors, notes, covered, len(spec_ids)


def main() -> int:
    specs = {}
    for p in (ROOT / "docs" / "02-design").rglob("*.md"):
        specs[p.relative_to(ROOT).as_posix()] = p.read_text(encoding="utf-8")
    live, stubs = set(), set()
    for p in (ROOT / "crates").rglob("*.rs"):
        if "target" in p.parts:
            continue
        text = p.read_text(encoding="utf-8")
        for block in test_blocks(text):
            for ac in re.findall(r"AC-[A-Z0-9]+-\d+", block):
                (stubs if "#[ignore" in block else live).add(ac)
    errors, notes, covered, total = check(specs, live, stubs)
    for n in notes:
        print("note:", n)
    if errors:
        print("TRACEABILITY FAILED:")
        for e in errors:
            print("  -", e)
        return 1
    print(f"OK — {covered}/{total} acceptance criteria have live tests; no stale test IDs.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
