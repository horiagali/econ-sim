#!/usr/bin/env python3
"""Generate ignored test stubs for acceptance criteria (ADR-0014, step 2).

Finds lines like `- [ ] **AC-VAT-01** [unit] text` in a spec and writes a
Rust test file with one `#[ignore]`d test per criterion that does not exist
yet. Existing tests are never overwritten.

Usage: python scripts/ac_stubs.py docs/02-design/economy/taxation.md VAT crates/econ-mech-tax/tests/acceptance/vat.rs
"""
import re
import sys
from pathlib import Path

AC = re.compile(r"\*\*(AC-[A-Z0-9]+-\d+)\*\*\s*`?\[(\w+)\]`?\s*(.*)")


def main() -> int:
    spec, prefix, out = Path(sys.argv[1]), sys.argv[2], Path(sys.argv[3])
    acs = []
    for line in spec.read_text(encoding="utf-8").splitlines():
        m = AC.search(line)
        if m and m.group(1).startswith(f"AC-{prefix}-"):
            acs.append((m.group(1), m.group(2), m.group(3).strip()))
    existing = out.read_text(encoding="utf-8") if out.exists() else ""
    new = []
    for ac_id, kind, text in acs:
        fn = ac_id.lower().replace("-", "_")
        if f"fn {fn}(" in existing:
            continue
        new.append(f'''
/// {ac_id} [{kind}] {text}
#[test]
#[ignore = "stub: test-writer session must implement"]
fn {fn}() {{
    todo!("{ac_id}");
}}
''')
    if not new:
        print(f"OK — all {len(acs)} {prefix} criteria already have tests in {out}")
        return 0
    out.parent.mkdir(parents=True, exist_ok=True)
    header = "" if existing else f"//! Acceptance tests for AC-{prefix}-* (generated stubs, filled by a test-writer session).\n"
    out.write_text(existing + header + "".join(new), encoding="utf-8")
    print(f"Wrote {len(new)} stub(s) to {out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
