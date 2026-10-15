#!/usr/bin/env python3
"""Build the firm units of the starting economy (ADR-0016; spec `economy/industries`).

Writes `data/derived/firm_units2021_ro.json`: for each of the 90 industries of
the catalogue, up to seven units:

    cohort firms by size class (micro: under 10 persons employed; small: 10 to 49; large: 50 or more),
        each split into a domestically controlled and a foreign-controlled cohort where foreign control matters
    one public unit where the industry has non-market public producers

Each unit has a firm count, its share of the industry's output, value added
and wage bill (from the input-output table), persons employed and an
ownership vector.

Measured: enterprises, employment, value added and wages by size class and
two-digit activity (structural business statistics 2021); value added under
foreign control by activity (foreign affiliates statistics 2021); the
industries' output, value added and wages (input-output table); each public
unit's persons employed (occupied posts) and wage bill (D11), from the budget
(`budget2021_ro.json`, spec `economy/budget-2021`).
Guessed (`GUESSES`): the size structure of the industries the business
statistics do not cover (agriculture, public administration, households as
employers, membership organisations); the public share of output and value
added where the business statistics leave none; that foreign control sits in
the largest firms first. A catalogue
industry inherits the size structure of the table industry it is part of.
Named firms are not here yet (roadmap M9): every unit is a cohort or public.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from jsonstat import to_rows  # noqa: E402
from normalise_census import CensusError  # noqa: E402

HERE = Path(__file__).resolve().parent
RAW = HERE / "data" / "raw"
SIZES_ID = "ro_enterprises_by_size_2021"
FOREIGN_ID = "ro_foreign_controlled_2021"
UNIT = 1_000_000
SIZES = ["micro", "small", "large"]
SIZE_OF = {"0-9": 0, "10-19": 1, "20-49": 1, "50-249": 2, "GE250": 2}
MEASURES = {"firms": "ENT_NR", "persons": "EMP_NR", "value_added": "AV_MEUR", "wages": "WAGE_MEUR"}
MIN_FOREIGN_SHARE = 0.10          # a cohort is split when foreign control is at least this share of it (ADR-0016)

GUESSES = {
    # Industries the business statistics do not cover: firms, and shares of persons, value added and wages,
    # for micro, small and large.
    "structure": {
        "A01": {"firms": [28_000, 3_500, 500], "persons": [0.55, 0.20, 0.25], "value_added": [0.45, 0.25, 0.30]},
        "A02": {"firms": [3_500, 500, 60], "persons": [0.35, 0.25, 0.40], "value_added": [0.30, 0.25, 0.45]},
        "A03": {"firms": [700, 80, 5], "persons": [0.55, 0.30, 0.15], "value_added": [0.50, 0.30, 0.20]},
        # Published figures are suppressed (few firms): refining is three large refineries and some small blenders.
        "C19": {"firms": [20, 8, 6], "persons": [0.01, 0.04, 0.95], "value_added": [0.01, 0.03, 0.96]},
        # Any other industry whose figures are not published.
        "DEFAULT": {"firms": [60, 20, 8], "persons": [0.10, 0.20, 0.70], "value_added": [0.08, 0.17, 0.75]},
        "S94": {"firms": [9_000, 600, 40], "persons": [0.50, 0.30, 0.20], "value_added": [0.50, 0.30, 0.20]},
        "T": {"firms": [20_000, 0, 0], "persons": [1.0, 0.0, 0.0], "value_added": [1.0, 0.0, 0.0]},
        "U": {"firms": [1, 0, 0], "persons": [1.0, 0.0, 0.0], "value_added": [1.0, 0.0, 0.0]},
        # Market activity inside public administration is negligible; the cohorts below are a formality.
        "O84": {"firms": [1, 0, 0], "persons": [1.0, 0.0, 0.0], "value_added": [1.0, 0.0, 0.0]},
    },
    # Industries with non-market public producers (ministries, town halls, public schools, public hospitals,
    # public care homes), and the public share of output to use if the business statistics have no wages for
    # the market part. Where they have, the share is measured: what those wages leave of the industry's wage bill.
    "public_share": {"O84": 1.0, "P85": 0.85, "Q86": 0.70, "Q87_88": 0.70},
    # Persons employed per million lei of wage bill where the business statistics have none (the guessed
    # structures above; public units only when no budget is given): one person per 80,000 lei a year.
    "public_persons_per_million_lei_of_wages": 12.5,
}
# The foreign-affiliates table is coarser than the input-output table: its code for each table industry.
FOREIGN_CODE = {
    "C10-12": "C10-C12", "C13-15": "C13-C15", "C16": "C16-C18", "C17": "C16-C18", "C18": "C16-C18",
    "C22": "C22_C23", "C23": "C22_C23", "C24": "C24_C25", "C25": "C24_C25", "C29": "C29_C30", "C30": "C29_C30",
    "C31_32": "C31_C32", "D35": "D", "E36": "E", "E37-39": "E", "G45": "G", "G46": "G", "G47": "G",
    "H52": "H52_H53", "H53": "H52_H53", "J59_60": "J59_J60", "J62_63": "J62_J63", "K64": "K", "K65": "K", "K66": "K",
    "M69_70": "M69-M71", "M71": "M69-M71", "M73": "M73-M75", "M74_75": "M73-M75", "N78": "N78-N82", "N79": "N78-N82",
    "N80-82": "N78-N82", "P85": "P", "Q87_88": "Q87_Q88", "R90-92": "R", "R93": "R", "S95": "S95_S96", "S96": "S95_S96",
}


def divisions(code: str) -> list[str]:
    """The two-digit activities of a table industry: 'C10-12' is C10, C11, C12; 'J59_60' is J59, J60; 'L' is L68."""
    whole = {"B": ["B"], "F": ["F"], "I": ["I"], "L": ["L68"]}
    if code in whole:
        return whole[code]
    m = re.fullmatch(r"([A-Z])(\d\d)(?:([-_])(\d\d))?", code)
    if not m:
        return [code]
    letter, first, sep, last = m.groups()
    if not sep:
        return [f"{letter}{first}"]
    return [f"{letter}{n:02d}" for n in range(int(first), int(last) + 1)]


def size_structure(sizes_doc: dict) -> dict:
    """{two-digit activity: {measure: [micro, small, large]}}, with unpublished size classes filled from the
    total; a measure published only as a total is None. Activities without firms, persons or value added
    by size are left out."""
    cells = {(r["nace_r2"], r["indic_sbs"], r["size_emp"]): float(r["value"]) for r in to_rows(sizes_doc)}
    out = {}
    for activity in {k[0] for k in cells}:
        row = {}
        for name, code in MEASURES.items():
            total = cells.get((activity, code, "TOTAL"))
            if total is None:
                break
            known = {s: cells.get((activity, code, s)) for s in SIZE_OF}
            missing = [s for s, v in known.items() if v is None]
            if len(missing) == len(SIZE_OF):
                row[name] = None            # published as a total only (wages): the caller uses another measure's shares
                continue
            left = max(total - sum(v for v in known.values() if v is not None), 0.0)
            by_size = [0.0, 0.0, 0.0]
            for s, k in SIZE_OF.items():
                by_size[k] += known[s] if known[s] is not None else left / len(missing)
            row[name] = by_size
        else:
            if all(row[k] is not None for k in ("firms", "persons", "value_added")):
                row["wages_total"] = cells.get((activity, MEASURES["wages"], "TOTAL"), 0.0)   # million euro
                out[activity] = row
    return out


def foreign_shares(foreign_doc: dict) -> dict:
    """{activity code of the foreign-affiliates table: share of value added under foreign control}."""
    cells = {(r["nace_r2"], r["c_ctrl"]): float(r["value"]) for r in to_rows(foreign_doc) if r["indic_sbs"] == "AV_MEUR"}
    out = {}
    for (activity, who), world in cells.items():
        home = cells.get((activity, "DOM"))
        if who == "WORLD" and world > 0 and home is not None:
            out[activity] = min(max(1.0 - home / world, 0.0), 1.0)
    return out


def shares(values: list[float]) -> list[float]:
    """Shares of a total; a negative part (a size class that lost money) counts as nothing."""
    values = [max(v, 0.0) for v in values]
    total = sum(values)
    return [v / total for v in values] if total > 0 else [1.0, 0.0, 0.0]


def build(sizes_doc: dict, foreign_doc: dict, io: dict, public_units: list[dict] | None = None) -> dict:
    """`io` is the input-output margin file (its industries, their output, value added, wages and the catalogue).

    `public_units` is the budget's `public_units` (`budget2021_ro.json`; spec `economy/budget-2021`): when given,
    each public unit's persons employed are its occupied posts and its wage bill is the budget's wages (D11), and
    every public unit built must be one of them. Without it, a public unit's persons follow its wage bill at the
    guessed persons per million lei. Output and value added follow the input-output table either way."""
    structure, foreign = size_structure(sizes_doc), foreign_shares(foreign_doc)
    pos = {ind["code"]: k for k, ind in enumerate(io["industries"])}
    units: dict[tuple, dict] = {}
    guessed: list[str] = []         # table industries whose size structure is a guess

    def add(industry: str, size: str, controller: str, **amounts: float) -> None:
        u = units.setdefault((industry, size, controller),
                             {"firm_count": 0.0, "persons": 0.0, "output": 0.0, "value_added": 0.0, "wages": 0.0})
        for k, v in amounts.items():
            u[k] += v

    for entry in io["catalogue"]:
        code = entry["industry"]
        j = pos[code]
        output = io["output"][j]
        wages = io["compensation"][j]
        value_added = wages + io["operating_surplus"][j] + io["other_taxes_less_subsidies"][j]
        rows = [structure[d] for d in divisions(code) if d in structure]
        guess = None if rows else GUESSES["structure"].get(code, GUESSES["structure"]["DEFAULT"])
        if not rows:
            guessed.append(code)
        if rows:
            firms = [sum(r["firms"][k] for r in rows) for k in range(3)]
            persons = [sum(r["persons"][k] for r in rows) for k in range(3)]
            va_share = shares([sum(r["value_added"][k] for r in rows) for k in range(3)])
            # Wages by size class are not published: a size class's share of wages is the mean of its
            # shares of persons employed and of value added (small firms pay less per head, more per leu of value).
            with_wages = [r for r in rows if r["wages"] is not None]
            person_share = shares(persons)
            wage_share = (shares([sum(r["wages"][k] for r in with_wages) for k in range(3)]) if len(with_wages) == len(rows)
                          else [(a + b) / 2 for a, b in zip(person_share, va_share)])
        else:
            firms, va_share = guess["firms"], guess["value_added"]
            wage_share = guess["persons"]
            persons = [p * wages / 1e6 * GUESSES["public_persons_per_million_lei_of_wages"] for p in guess["persons"]]
        # Where an industry has public producers, the business statistics hold its market part only:
        # the public share is what their wages leave of the industry's wage bill (the guess is the fallback).
        public = GUESSES["public_share"].get(code, 0.0)
        if public > 0 and rows and wages > 0:
            surveyed = sum(r["wages_total"] for r in rows) * io["exchange_rate"] * 1e6
            public = min(max(1.0 - surveyed / wages, 0.0), 1.0)
        market = 1.0 - public
        # Firms and persons of the statistics are those of the market part already.
        head_scale = 1.0 if rows else market
        # Foreign control: the industry's foreign share of value added, in the largest size class first.
        foreign_left = foreign.get(FOREIGN_CODE.get(code, code), 0.0)
        foreign_of = [0.0, 0.0, 0.0]
        for k in (2, 1, 0):
            take = min(foreign_left, va_share[k])
            foreign_of[k] = take / va_share[k] if va_share[k] > 0 else 0.0
            foreign_left -= take
        for part in entry["parts"]:
            s = part["share"] / UNIT
            if public > 0:
                add(part["id"], "public", "state", firm_count=1.0 if s == max(p["share"] for p in entry["parts"]) / UNIT else 0.0,
                    persons=s * public * wages / 1e6 * GUESSES["public_persons_per_million_lei_of_wages"],
                    output=s * public * output, value_added=s * public * value_added, wages=s * public * wages)
            for k, size in enumerate(SIZES):
                if va_share[k] <= 0 and firms[k] <= 0:
                    continue
                f = foreign_of[k] if foreign_of[k] >= MIN_FOREIGN_SHARE else 0.0
                for controller, w in (("domestic", 1.0 - f), ("foreign", f)):
                    if w <= 0:
                        continue
                    # Foreign-controlled firms are few and large: they take their share of value, a tenth of it in number.
                    count_w = w if controller == "domestic" and f == 0 else (f / 10 if controller == "foreign" else 1 - f / 10)
                    add(part["id"], size, controller, firm_count=s * head_scale * firms[k] * count_w,
                        persons=s * head_scale * persons[k] * w, output=s * market * output * va_share[k] * w,
                        value_added=s * market * value_added * va_share[k] * w, wages=s * market * wages * wage_share[k] * w)

    if public_units is not None:
        budget = {p["unit"]: p for p in public_units}
        built = sorted(f"{k[0]}.public" for k in units if k[1] == "public")
        if sorted(budget) != built:
            raise CensusError(f"budget public units {sorted(budget)} are not the public units built {built}")
        for (industry, size, _), u in units.items():
            if size == "public":
                p = budget[f"{industry}.public"]
                u["persons"], u["wages"] = float(p["posts"]), float(p["wages"])

    out_units = []
    for (industry, size, controller), u in sorted(units.items()):
        if u["output"] < 1:
            continue
        state = UNIT if controller == "state" else 0
        foreign_ppm = UNIT if controller == "foreign" else 0
        out_units.append({
            "id": f"{industry}.{size}" + ("" if size == "public" else f".{controller}"),
            "industry": industry, "kind": "public" if size == "public" else "cohort", "size": size,
            "controller": controller, "firm_count": max(1, round(u["firm_count"])),
            "persons_employed": round(u["persons"]), "output": round(u["output"]),
            "value_added": round(u["value_added"]), "wages": round(u["wages"]),
            "ownership": {"state": state, "domestic_private": UNIT - state - foreign_ppm, "foreign": foreign_ppm},
        })
    # The catalogue's industries in a fixed order, each with its industry group (that of its table industries).
    group_of: dict[str, int] = {}
    for entry in io["catalogue"]:
        for part in entry["parts"]:
            group_of.setdefault(part["id"], io["industry_group_of"][pos[entry["industry"]]])
    industries = [{"id": cid, "industry_group": group_of[cid]} for cid in sorted(group_of)]
    total = lambda key: sum(u[key] for u in out_units)  # noqa: E731
    by_controller = {c: sum(u["value_added"] for u in out_units if u["controller"] == c)
                     for c in ("domestic", "foreign", "state")}
    return {
        "year": 2021, "unit": "lei a year", "sizes": [*SIZES, "public"],
        "industries": industries,
        "units": out_units,
        "size_structure_guessed_for": guessed,
        "totals": {"units": len(out_units), "firms": total("firm_count"), "persons_employed": total("persons_employed"),
                   "output": total("output"), "value_added": total("value_added"), "wages": total("wages"),
                   "value_added_by_controller": by_controller},
    }


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--raw", type=Path, default=RAW)
    ap.add_argument("--io", type=Path, default=HERE / "data" / "derived" / "io_table2021_ro.json")
    ap.add_argument("--budget", type=Path, default=HERE / "data" / "derived" / "budget2021_ro.json")
    ap.add_argument("--out", type=Path, default=HERE / "data" / "derived" / "firm_units2021_ro.json")
    a = ap.parse_args()
    docs, prov = {}, []
    for ds in (SIZES_ID, FOREIGN_ID):
        raw = (a.raw / f"{ds}.json").read_bytes()
        docs[ds] = json.loads(raw)
        side = a.raw / f"{ds}.provenance.json"
        p = json.loads(side.read_text(encoding="utf-8")) if side.exists() else {"id": ds}
        p["sha256"] = hashlib.sha256(raw).hexdigest()
        prov.append(p)
    budget = json.loads(a.budget.read_text(encoding="utf-8"))
    m = build(docs[SIZES_ID], docs[FOREIGN_ID], json.loads(a.io.read_text(encoding="utf-8")), budget["public_units"])
    m["provenance"] = {"sources": prov, "guesses": GUESSES,
                       "note": "Eurostat structural business statistics and foreign affiliates statistics 2021, on the "
                               "input-output table's totals; the public units' persons employed and wages are the "
                               "budget's occupied posts and wages (D11), from budget2021_ro.json; guesses listed "
                               "under `guesses`"}
    a.out.parent.mkdir(parents=True, exist_ok=True)
    a.out.write_text(json.dumps(m, separators=(",", ":")) + "\n", encoding="utf-8")
    print(f"OK - {m['totals']} -> {a.out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
