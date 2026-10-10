"""Minimal JSON-stat 2.0 reader (Eurostat API responses) → list of row dicts."""
from __future__ import annotations

from itertools import product


def to_rows(doc: dict) -> list[dict]:
    """Flatten a JSON-stat dataset into rows {dim: label_code, ..., 'value': v}."""
    ids: list[str] = doc["id"]
    sizes: list[int] = doc["size"]
    dims = doc["dimension"]
    # category codes ordered by their index
    codes = []
    for d in ids:
        idx = dims[d]["category"]["index"]
        if isinstance(idx, list):
            codes.append(idx)
        else:
            codes.append([c for c, _ in sorted(idx.items(), key=lambda kv: kv[1])])
    values = doc["value"]
    rows = []
    for flat, combo in enumerate(product(*[range(s) for s in sizes])):
        key = str(flat)
        v = values.get(key) if isinstance(values, dict) else (values[flat] if flat < len(values) else None)
        if v is None:
            continue
        row = {d: codes[i][combo[i]] for i, d in enumerate(ids)}
        row["value"] = v
        rows.append(row)
    return rows
