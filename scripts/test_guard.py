#!/usr/bin/env python3
"""Test-count guard: tests and assertions may not quietly disappear (ADR-0014 layer 3).

Compares the Rust tests of the working tree (in CI: the PR's merge commit)
with a base commit (in CI: the PR base; locally: `git merge-base HEAD
origin/main`) and fails when:

- live tests fall (a test that runs in `cargo test` or `cargo test --release`),
  per group: `acceptance` (the protected `crates/*/tests/acceptance/**`) and
  `unit` (every other test);
- assertions fall, per group (`assert!`, `assert_eq!`, `assert_ne!`,
  `prop_assert*!`, and calls to the project's `assert_*` helpers; in test
  code only: files under a `tests/` directory and `#[cfg(test)]` modules);
- a test is newly ignored or skipped: `#[ignore]`, `#[ignore = "..."]`, a
  `cfg_attr(..., ignore)` other than the release-only form, a `#[cfg(...)]`
  that is never true (`any()`, `not(all())`, `FALSE`), or a body that starts
  with `return;`. A new test that is born ignored counts too;
- a never-true `#[cfg(...)]` or an `if false` appears anywhere in test code;
- a file under `tests/golden/` disappears.

`#[cfg_attr(debug_assertions, ignore = "...")]` makes a test release-only:
it still runs in CI's release jobs, so it stays live, but every move from
live to release-only is reported.

Overrides go only through `tests/golden/guard-approvals.toml`, a protected
path (only the test-writer subagent or an owner's test session may edit it).
An approval counts only in the change that adds it, so the file is also the
audit log. See `APPROVAL_HELP` for the format.

Run with `just test-guard` (part of `just check`).
"""
from __future__ import annotations

import argparse
import datetime as dt
import os
import re
import subprocess
import sys
from collections import Counter
from dataclasses import dataclass, field

try:
    import tomllib
except ModuleNotFoundError:  # Python < 3.11
    tomllib = None

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
SCAN_ROOTS = ["crates", "ui/src-tauri"]
GOLDEN_DIR = "tests/golden"
APPROVALS = "tests/golden/guard-approvals.toml"
ACCEPTANCE_RE = re.compile(r"^crates/[^/]+/tests/acceptance/")
GROUPS = ("acceptance", "unit")
CHECKS = ("live-tests", "assertions", "ignored-test", "skip-marker", "golden-file")

APPROVAL_HELP = f"""\
To approve a decrease, the owner says yes explicitly (AGENTS.md), and the
test-writer subagent (or the owner's test session) adds an entry to
{APPROVALS} in the same pull request:

  [[approval]]
  check = "assertions"        # {" | ".join(CHECKS)}
  group = "acceptance"        # live-tests and assertions: acceptance | unit
  decrease = 2                # live-tests and assertions: the largest fall approved
  # target = "crates/x/tests/acceptance/y.rs::test_name"   # ignored-test; a file path for skip-marker and golden-file
  approved_by = "horia"
  approved_on = 2026-10-26
  reason = "why the owner agreed"

An approval counts only in the change that adds it."""


# --------------------------------------------------------------------------
# Rust source scanning
# --------------------------------------------------------------------------

def strip_rust(src: str) -> str:
    """Blank out comments and the insides of string and char literals.

    Keeps length and newlines, so offsets and line numbers still hold.
    Attribute strings such as `#[ignore = "x"]` keep their quotes.
    """
    out = list(src)
    n = len(src)
    i = 0

    def blank(a: int, b: int) -> None:
        for k in range(a, min(b, n)):
            if out[k] != "\n":
                out[k] = " "

    while i < n:
        c = src[i]
        if c == "/" and i + 1 < n and src[i + 1] == "/":
            j = src.find("\n", i)
            j = n if j < 0 else j
            blank(i, j)
            i = j
        elif c == "/" and i + 1 < n and src[i + 1] == "*":
            depth, j = 1, i + 2
            while j < n and depth:
                if src.startswith("/*", j):
                    depth, j = depth + 1, j + 2
                elif src.startswith("*/", j):
                    depth, j = depth - 1, j + 2
                else:
                    j += 1
            blank(i, j)
            i = j
        elif c == "r" and re.match(r"r#*\"", src[i:i + 300]) and (i == 0 or not (src[i - 1].isalnum() or src[i - 1] == "_")):
            m = re.match(r"r(#*)\"", src[i:])
            close = '"' + m.group(1)
            start = i + m.end()
            j = src.find(close, start)
            j = n if j < 0 else j
            blank(start, j)
            i = j + len(close)
        elif c == '"':
            j = i + 1
            while j < n and src[j] != '"':
                j += 2 if src[j] == "\\" else 1
            blank(i + 1, j)
            i = j + 1
        elif c == "'":
            m = re.match(r"'(\\(x[0-9a-fA-F]{2}|u\{[0-9a-fA-F]+\}|.)|[^\\'\n])'", src[i:i + 16])
            if m:
                blank(i + 1, i + m.end() - 1)
                i += m.end()
            else:
                i += 1  # a lifetime
        else:
            i += 1
    return "".join(out)


def match_close(text: str, start: int, open_c: str, close_c: str) -> int:
    """Index just past the bracket that closes the one at `start`."""
    depth = 0
    for k in range(start, len(text)):
        ch = text[k]
        if ch == open_c:
            depth += 1
        elif ch == close_c:
            depth -= 1
            if depth == 0:
                return k + 1
    return len(text)


TOKEN_RE = re.compile(r"#!?\s*\[|\b(fn|mod)\s+([A-Za-z_]\w*)|\b(?:pub|async|unsafe|const|extern|crate|super|in|self)\b|[A-Za-z_]\w*|\S")
NEVER_CFG_RE = re.compile(r"^cfg\s*\(\s*(any\s*\(\s*\)|not\s*\(\s*all\s*\(\s*\)\s*\)|FALSE|false|never|disabled)\s*\)$")
CFG_TEST_RE = re.compile(r"^cfg\s*\(\s*test\s*\)$")
TEST_ATTR_RE = re.compile(r"^([\w:]*::)?test$")
IGNORE_RE = re.compile(r"^ignore\b")
RELEASE_ONLY_RE = re.compile(r"^cfg_attr\s*\(\s*debug_assertions\s*,\s*ignore\b")
CFG_ATTR_IGNORE_RE = re.compile(r"^cfg_attr\s*\(.*,\s*ignore\b", re.S)
EARLY_RETURN_RE = re.compile(r"^\{\s*return\s*(Ok\s*\(\s*\(\s*\)\s*\))?\s*;")
IF_FALSE_RE = re.compile(r"\bif\s+(false|!\s*true)\b")
ASSERT_MACRO_RE = re.compile(r"\b(prop_)?assert(_[a-z_]+)?\s*!")
ASSERT_HELPER_RE = re.compile(r"\bassert_\w+\s*\(")
ASSERT_DEF_RE = re.compile(r"\bfn\s+assert_\w+")


@dataclass
class TestFn:
    id: str
    path: str
    name: str
    status: str  # live | release-only | ignored | disabled | skipped
    line: int


@dataclass
class FileScan:
    tests: list = field(default_factory=list)
    assertions: int = 0
    skip_markers: int = 0  # never-true cfg and `if false` in test code


def norm_attr(a: str) -> str:
    return re.sub(r"\s+", " ", a.strip())


def scan_rust(path: str, src: str) -> FileScan:
    text = strip_rust(src)
    whole_file_is_test = "/tests/" in "/" + path or path.endswith("/tests.rs")
    res = FileScan()
    test_regions = []  # (start, end) of #[cfg(test)] modules
    pending = []  # attributes waiting for their item
    names = Counter()
    pos = 0
    while True:
        m = TOKEN_RE.search(text, pos)
        if not m:
            break
        pos = m.end()
        tok = m.group(0)
        if tok.startswith("#") and tok.endswith("["):
            open_at = m.end() - 1
            end = match_close(text, open_at, "[", "]")
            pos = end
            attr = norm_attr(text[open_at + 1:end - 1])
            if NEVER_CFG_RE.match(attr):
                res.skip_markers += 1
            if not tok.startswith("#!"):
                pending.append(attr)
            continue
        if m.group(1) == "mod":
            if any(CFG_TEST_RE.match(a) for a in pending):
                brace = re.compile(r"\s*\{").match(text, m.end())
                if brace:
                    b = brace.end() - 1
                    test_regions.append((b, match_close(text, b, "{", "}")))
            pending = []
            continue
        if m.group(1) == "fn":
            if any(TEST_ATTR_RE.match(a) for a in pending):
                name = m.group(2)
                names[name] += 1
                tid = f"{path}::{name}" + (f"#{names[name]}" if names[name] > 1 else "")
                status = "live"
                if any(NEVER_CFG_RE.match(a) for a in pending):
                    status = "disabled"
                elif any(IGNORE_RE.match(a) for a in pending):
                    status = "ignored"
                elif any(RELEASE_ONLY_RE.match(a) for a in pending):
                    status = "release-only"
                elif any(CFG_ATTR_IGNORE_RE.match(a) for a in pending):
                    status = "ignored"
                else:
                    b = text.find("{", m.end())
                    if b >= 0 and EARLY_RETURN_RE.match(text, b):
                        status = "skipped"
                line = text.count("\n", 0, m.start()) + 1
                res.tests.append(TestFn(tid, path, name, status, line))
            pending = []
            continue
        if m.group(0) in ("pub", "async", "unsafe", "const", "extern", "crate", "super", "in", "self", "(", ")"):
            continue  # `pub(crate) async fn` keeps its attributes
        pending = []

    if whole_file_is_test:
        regions = [(0, len(text))]
    else:
        regions = test_regions
    for a, b in regions:
        part = text[a:b]
        res.assertions += len(ASSERT_MACRO_RE.findall(part))
        res.assertions += len(ASSERT_HELPER_RE.findall(part)) - len(ASSERT_DEF_RE.findall(part))
        res.skip_markers += len(IF_FALSE_RE.findall(part))
    return res


# --------------------------------------------------------------------------
# Snapshots of a tree
# --------------------------------------------------------------------------

@dataclass
class Snapshot:
    tests: dict = field(default_factory=dict)  # id -> TestFn
    assertions: Counter = field(default_factory=Counter)  # group -> n
    skip_markers: Counter = field(default_factory=Counter)  # path -> n
    golden: set = field(default_factory=set)
    approvals_text: str | None = None

    def live(self, group: str) -> int:
        return sum(1 for t in self.tests.values() if group_of(t.path) == group and t.status in ("live", "release-only"))

    def count(self, status: str) -> int:
        return sum(1 for t in self.tests.values() if t.status == status)


def group_of(path: str) -> str:
    return "acceptance" if ACCEPTANCE_RE.match(path) else "unit"


def build_snapshot(files: dict, golden: set, approvals_text) -> Snapshot:
    snap = Snapshot(golden=golden, approvals_text=approvals_text)
    for path in sorted(files):
        scan = scan_rust(path, files[path])
        for t in scan.tests:
            snap.tests[t.id] = t
        snap.assertions[group_of(path)] += scan.assertions
        if scan.skip_markers:
            snap.skip_markers[path] = scan.skip_markers
    return snap


def git(*args: str, input_bytes: bytes | None = None) -> bytes:
    return subprocess.run(["git", *args], cwd=ROOT, input=input_bytes, capture_output=True, check=True).stdout


def snapshot_of_worktree() -> Snapshot:
    listed = git("ls-files", "--cached", "--others", "--exclude-standard", "-z", "--", *SCAN_ROOTS, GOLDEN_DIR)
    files, golden = {}, set()
    for p in sorted(set(listed.decode("utf-8").split("\0"))):
        full = os.path.join(ROOT, p)
        if not p or not os.path.isfile(full):
            continue
        if p.startswith(GOLDEN_DIR + "/"):
            golden.add(p)
        elif p.endswith(".rs"):
            with open(full, encoding="utf-8", errors="replace") as f:
                files[p] = f.read()
    approvals = None
    if os.path.isfile(os.path.join(ROOT, APPROVALS)):
        with open(os.path.join(ROOT, APPROVALS), encoding="utf-8") as f:
            approvals = f.read()
    return build_snapshot(files, golden, approvals)


def snapshot_of_commit(rev: str) -> Snapshot:
    listed = git("ls-tree", "-r", "-z", "--name-only", rev, "--", *SCAN_ROOTS, GOLDEN_DIR).decode("utf-8")
    paths = [p for p in listed.split("\0") if p]
    golden = {p for p in paths if p.startswith(GOLDEN_DIR + "/")}
    wanted = [p for p in paths if p.endswith(".rs") and not p.startswith(GOLDEN_DIR + "/")]
    if APPROVALS in golden:
        wanted.append(APPROVALS)
    blobs = read_blobs(rev, wanted)
    approvals = blobs.pop(APPROVALS, None)
    return build_snapshot(blobs, golden, approvals)


def read_blobs(rev: str, paths: list) -> dict:
    if not paths:
        return {}
    req = "".join(f"{rev}:{p}\n" for p in paths).encode("utf-8")
    out = git("cat-file", "--batch", input_bytes=req)
    files, pos = {}, 0
    for p in paths:
        nl = out.index(b"\n", pos)
        header = out[pos:nl].decode()
        pos = nl + 1
        if header.endswith("missing"):
            continue
        size = int(header.split()[2])
        files[p] = out[pos:pos + size].decode("utf-8", errors="replace")
        pos += size + 1
    return files


# --------------------------------------------------------------------------
# Approvals
# --------------------------------------------------------------------------

def parse_approvals(text) -> list:
    if not text:
        return []
    if tomllib is None:
        raise SystemExit("test guard: Python 3.11+ is needed to read " + APPROVALS)
    data = tomllib.loads(text)
    return list(data.get("approval", []))


def approval_key(a: dict) -> tuple:
    return tuple(sorted((k, str(v)) for k, v in a.items()))


def new_approvals(base_text, head_text) -> tuple:
    """Approvals added by this change, and problems with them."""
    base_keys = {approval_key(a) for a in parse_approvals(base_text)}
    fresh, problems = [], []
    for a in parse_approvals(head_text):
        if approval_key(a) in base_keys:
            continue
        missing = [k for k in ("check", "approved_by", "approved_on", "reason") if not str(a.get(k, "")).strip()]
        when = a.get("approved_on")
        if isinstance(when, str):
            try:
                dt.date.fromisoformat(when)
            except ValueError:
                missing.append("approved_on (a YYYY-MM-DD date)")
        elif when is not None and not isinstance(when, dt.date):
            missing.append("approved_on (a YYYY-MM-DD date)")
        if a.get("check") not in CHECKS:
            missing.append(f"check (one of {', '.join(CHECKS)})")
        if a.get("check") in ("live-tests", "assertions"):
            if a.get("group") not in GROUPS:
                missing.append("group (acceptance or unit)")
            if not isinstance(a.get("decrease"), int) or a["decrease"] <= 0:
                missing.append("decrease (a positive integer)")
        elif a.get("check") in CHECKS and not str(a.get("target", "")).strip():
            missing.append("target")
        if missing:
            problems.append(f"approval {dict(a)} lacks or has a bad: {', '.join(missing)}")
        else:
            fresh.append(a)
    return fresh, problems


def approved(approvals: list, check: str, *, group=None, decrease=0, target=None) -> dict | None:
    for a in approvals:
        if a["check"] != check:
            continue
        if group is not None and (a.get("group") != group or a.get("decrease", 0) < decrease):
            continue
        if target is not None and a.get("target") != target:
            continue
        return a
    return None


# --------------------------------------------------------------------------
# Comparison
# --------------------------------------------------------------------------

def compare(base: Snapshot, head: Snapshot) -> tuple:
    """Return (failures, notes)."""
    approvals, failures = new_approvals(base.approvals_text, head.approvals_text)
    notes = []

    def gate(msg: str, a) -> None:
        if a:
            notes.append(f"APPROVED ({a['approved_by']}, {a['approved_on']}: {a['reason']}): {msg}")
        else:
            failures.append(msg)

    for g in GROUPS:
        b, h = base.live(g), head.live(g)
        if h < b:
            gate(f"live {g} tests fell from {b} to {h}", approved(approvals, "live-tests", group=g, decrease=b - h))
        b, h = base.assertions[g], head.assertions[g]
        if h < b:
            gate(f"{g} assertions fell from {b} to {h}", approved(approvals, "assertions", group=g, decrease=b - h))

    base_by_name = {}
    for t in base.tests.values():
        base_by_name.setdefault(t.name, set()).add(t.status)
    removed_live = [t for tid, t in base.tests.items() if tid not in head.tests and t.status in ("live", "release-only")]
    for tid, t in sorted(head.tests.items()):
        old = base.tests.get(tid)
        if t.status in ("ignored", "disabled", "skipped"):
            if old is not None and old.status == t.status:
                continue
            if old is None and t.status in base_by_name.get(t.name, set()):
                continue  # moved to another file, already in that state
            was = f"was {old.status}" if old else "new"
            gate(f"{t.status} test ({was}): {tid} (line {t.line})", approved(approvals, "ignored-test", target=tid))
        elif t.status == "release-only" and (old is None or old.status != "release-only"):
            was = f"was {old.status}" if old else "new"
            notes.append(f"release-only now ({was}; runs only in release builds): {tid}")
    for t in removed_live:
        notes.append(f"removed or renamed: {t.id}")

    for path, n in sorted(head.skip_markers.items()):
        b = base.skip_markers.get(path, 0)
        if n > b:
            gate(f"{n - b} new skip marker(s) (never-true #[cfg] or `if false`) in {path}",
                 approved(approvals, "skip-marker", target=path))
    for p in sorted(base.golden - head.golden):
        if p == APPROVALS:
            failures.append(f"{APPROVALS} was removed; it is the audit log of approvals")
            continue
        gate(f"golden file removed: {p}", approved(approvals, "golden-file", target=p))
    return failures, notes


def summary(s: Snapshot) -> str:
    parts = []
    for g in GROUPS:
        parts.append(f"{g}: {s.live(g)} live tests, {s.assertions[g]} assertions")
    parts.append(
        f"release-only {s.count('release-only')}, ignored {s.count('ignored')}, "
        f"disabled {s.count('disabled')}, skipped {s.count('skipped')}, "
        f"skip markers {sum(s.skip_markers.values())}, golden files {len(s.golden)}"
    )
    return "; ".join(parts)


def resolve_base(arg) -> str | None:
    if arg:
        if set(arg) == {"0"}:
            return None  # a new branch pushed: nothing to compare with
        return arg
    try:
        return git("merge-base", "HEAD", "origin/main").decode().strip()
    except subprocess.CalledProcessError:
        return None


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--base", help="commit to compare with (default: merge-base of HEAD and origin/main)")
    args = ap.parse_args(argv)
    base_rev = resolve_base(args.base)
    if base_rev is None:
        print("test guard: no base commit to compare with; skipped")
        return 0
    try:
        base = snapshot_of_commit(base_rev)
    except subprocess.CalledProcessError as e:
        print(f"test guard: cannot read base {base_rev}: {e.stderr.decode(errors='replace').strip()}", file=sys.stderr)
        return 1
    head = snapshot_of_worktree()
    print(f"test guard: base {base_rev[:12]}: {summary(base)}")
    print(f"test guard: this tree:      {summary(head)}")
    failures, notes = compare(base, head)
    for n in notes:
        print(f"  note: {n}")
    if failures:
        print("\nTEST GUARD FAILED (ADR-0014 layer 3): tests may not be weakened without the owner's explicit yes.", file=sys.stderr)
        for f in failures:
            print(f"  - {f}", file=sys.stderr)
        print("\n" + APPROVAL_HELP, file=sys.stderr)
        return 1
    print("test guard: ok")
    return 0


if __name__ == "__main__":
    sys.exit(main())
