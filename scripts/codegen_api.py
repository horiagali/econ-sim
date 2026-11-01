#!/usr/bin/env python3
"""Generate ui/src/generated/api.ts from the core API's serde types (spec
game/core-api, AC-API-04).

Reads the `#[derive(..., Serialize, ...)]` structs and enums of
crates/econ-core/src/api.rs (the API's types and the private mirror types it
serialises `Filter`, `GroupSummary` and `Node` through) and `Attribute` of
crates/econ-core/src/groups.rs, and writes their JSON shapes as TypeScript
types, as serde_json writes them: structs as objects, enums externally tagged
(a unit variant as its name, a newtype variant as `{ Name: value }`, a tuple
variant as `{ Name: [values] }`, a struct variant as `{ Name: { fields } }`).

    python scripts/codegen_api.py           # write
    python scripts/codegen_api.py --check   # fail if the file is stale
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SOURCES = [
    (ROOT / "crates/econ-core/src/api.rs", None),
    (ROOT / "crates/econ-core/src/groups.rs", {"Attribute"}),
]
OUT = ROOT / "ui/src/generated/api.ts"

# The mirror types are the public types' JSON form: named as the public ones.
RENAME = {
    "FilterDto": "Filter",
    "ConditionDto": "Condition",
    "SummaryDto": "GroupSummary",
    "NodeDto": "Node",
    "SexDto": "Sex",
    "EduDto": "EduLevel",
    "ActivityDto": "Activity",
    "EnrolledDto": "Enrolled",
}
NUMBERS = {"u8", "u16", "u32", "u64", "i8", "i16", "i32", "i64", "usize", "f64"}


def strip_comments(text: str) -> str:
    return "\n".join(re.sub(r"//.*$", "", line) for line in text.splitlines())


def block(text: str, start: int) -> tuple[str, int]:
    """The text between the brace at or after `start` and its match."""
    i = text.index("{", start)
    depth = 0
    for j in range(i, len(text)):
        if text[j] == "{":
            depth += 1
        elif text[j] == "}":
            depth -= 1
            if depth == 0:
                return text[i + 1 : j], j + 1
    raise ValueError("unbalanced braces")


def split_top(text: str, sep: str = ",") -> list[str]:
    """Split on `sep` outside brackets."""
    out, depth, cur = [], 0, ""
    for ch in text:
        if ch in "<({[":
            depth += 1
        elif ch in ">)}]":
            depth -= 1
        if ch == sep and depth == 0:
            out.append(cur)
            cur = ""
        else:
            cur += ch
    out.append(cur)
    return [x.strip() for x in out if x.strip()]


def ts_type(rust: str) -> str:
    rust = rust.strip()
    m = re.fullmatch(r"(\w+)\s*<(.*)>", rust, re.S)
    if m:
        outer, inner = m.group(1), m.group(2)
        if outer == "Option":
            return f"{ts_type(inner)} | null"
        if outer == "Vec":
            t = ts_type(inner)
            return f"({t})[]" if "|" in t else f"{t}[]"
        raise ValueError(f"unknown generic {rust}")
    if rust.startswith("("):
        parts = split_top(rust[1:-1])
        return "[" + ", ".join(ts_type(p) for p in parts) + "]"
    if rust in NUMBERS:
        return "number"
    if rust == "String":
        return "string"
    if rust == "bool":
        return "boolean"
    return RENAME.get(rust, rust)


def fields(body: str) -> list[tuple[str, str]]:
    out = []
    for f in split_top(body):
        f = re.sub(r"#\[[^\]]*\]", "", f).strip()
        f = re.sub(r"^pub(\([^)]*\))?\s+", "", f)
        name, _, ty = f.partition(":")
        out.append((name.strip(), ty.strip()))
    return out


def obj(fs: list[tuple[str, str]]) -> str:
    return "{ " + "; ".join(f"{n}: {ts_type(t)}" for n, t in fs) + " }"


def items(path: Path, only: set[str] | None) -> list[str]:
    text = strip_comments(path.read_text(encoding="utf-8"))
    out = []
    derive = re.compile(r"#\[(?:cfg_attr\([^,]*,\s*)?derive\(([^)]*)\)\)?\]")
    for m in derive.finditer(text):
        if "Serialize" not in m.group(1):
            continue
        head = re.compile(r"\s*(?:#\[[^\]]*\]\s*)*(?:pub(?:\([^)]*\))?\s+)?(struct|enum)\s+(\w+)")
        h = head.match(text, m.end())
        if not h:
            continue
        kind, name = h.group(1), h.group(2)
        if only is not None and name not in only:
            continue
        body, _ = block(text, h.end())
        ts_name = RENAME.get(name, name)
        if kind == "struct":
            out.append(f"export interface {ts_name} {obj(fields(body))}")
            continue
        variants = []
        for v in split_top(body):
            v = re.sub(r"#\[[^\]]*\]", "", v).strip()
            vm = re.match(r"(\w+)\s*(.*)$", v, re.S)
            vname, rest = vm.group(1), vm.group(2).strip()
            if not rest:
                variants.append(f'"{vname}"')
            elif rest.startswith("("):
                parts = split_top(rest[1:-1])
                inner = ts_type(parts[0]) if len(parts) == 1 else "[" + ", ".join(ts_type(p) for p in parts) + "]"
                variants.append(f"{{ {vname}: {inner} }}")
            else:
                variants.append(f"{{ {vname}: {obj(fields(rest[1:-1]))} }}")
        out.append(f"export type {ts_name} =\n  | " + "\n  | ".join(variants) + ";")
    return out


def render() -> str:
    parts = [
        "// Generated by scripts/codegen_api.py from the core API's serde types",
        "// (crates/econ-core/src/api.rs; spec game/core-api). Do not edit:",
        "// run `just codegen`. Integers are JSON numbers: money in bani, rates in",
        "// millionths or basis points; i64 values above 2^53 lose precision in",
        "// JavaScript (the series travel as binary arrays instead, ADR-0013).",
        "",
    ]
    for path, only in SOURCES:
        for item in items(path, only):
            parts.append(item)
            parts.append("")
    return "\n".join(parts)


def main() -> int:
    text = render()
    if "--check" in sys.argv[1:]:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} is stale: run `just codegen`", file=sys.stderr)
            return 1
        print("OK — api.ts is in step with the Rust types.")
        return 0
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text, encoding="utf-8", newline="\n")
    print(f"OK — wrote {OUT.relative_to(ROOT)}.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
