#!/usr/bin/env python3
"""Python reference of the starting budget (spec `economy/budget-2021`).

    python python/reference/budget_reference.py [--fixtures DIR] [--population-scale N]

Loads `budget2021_ro.json`, checks that it is consistent with itself and with
the published totals it carries (AC-BUD-01 to 04), reconciles it with the firm
units, the earnings, the pensions and the input-output table (AC-BUD-05 to 08)
and, with `--population-scale`, with the public employees of the starting
population (AC-BUD-09). Prints one JSON document: the totals, every check, the
monthly amounts of each line in bani and the budget hash (AC-BUD-11), so that
the Rust scenario loader can be diffed against it (AC-BUD-12).

Exit code 1 if a structural check fails or a reconciliation fails outside its
tolerance, except the reconciliations listed in `KNOWN_GAPS`.

Written independently of `python/pipeline/normalise_budget.py`: it reads only
the fixture, never the normaliser.
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
FIXTURES = ROOT / "pipeline" / "fixtures"
BANI = 100
MONTHS = 12
FNV_OFFSET, FNV_PRIME, M64 = 0xCBF29CE484222325, 0x100000001B3, (1 << 64) - 1

# Each economic type and the published national total it must add up to.
TYPE_TOTALS = {"wages": "D11", "employer_contributions": "D12", "goods_services": "P2",
               "social_benefits_cash": "D62", "social_benefits_in_kind": "D632", "subsidies": "D3",
               "interest": "D4", "investment": "P5", "non_produced_assets": "NP", "current_transfers": "D7",
               "capital_transfers": "D9", "other_taxes": "D29_D5_D8"}
PUBLIC_UNITS = ["defence_services.public", "education_services.public", "health_services.public",
                "public_administration.public", "social_care.public"]

# Tolerances of the reconciliations, as relative differences (spec, "Acceptance tests").
TOL = {
    "firm_units_public_wages": 0.02,      # AC-BUD-05
    "firm_units_public_persons": 0.02,    # AC-BUD-06 (the public units are built from the budget's posts)
    "earnings_public_group": 0.10,        # AC-BUD-07
    "pensions": 0.10,                     # AC-BUD-08
    "io_government_consumption": 0.01,    # AC-BUD-08
    "population_public_employees": 0.20,  # AC-BUD-09
    "population_public_wages": 0.10,      # AC-BUD-09
    "firm_units_by_unit": 0.02,           # each public unit against its posts and wages
}
# Reconciliations expected to fail for now, with the reason; reported, they do not fail the check. None since the
# public units were built from the budget's posts and wages (2026-10-15).
KNOWN_GAPS: dict[str, str] = {}


def load(fixtures: Path = FIXTURES) -> dict:
    names = {"budget": "budget2021_ro.json", "firms": "firm_units2021_ro.json", "earnings": "earnings2021_ro.json",
             "pensions": "pensions2021_ro.json", "io": "io_table2021_ro.json"}
    return {k: json.loads((fixtures / v).read_text(encoding="utf-8")) for k, v in names.items()}


def structure_errors(b: dict) -> list[str]:
    """AC-BUD-01 to 04: the table adds up, exactly, to the published totals it carries."""
    err = []
    types, pub = b["types"], b["published"]
    if sorted(types) != sorted(TYPE_TOTALS) or len(set(types)) != len(types):
        err.append("types: not the twelve economic types")
        return err
    n = len(types)
    for ln in b["lines"]:
        if len(ln["spending"]) != n:
            err.append(f"line {ln['id']}: {len(ln['spending'])} amounts for {n} types")
        if sum(ln["spending"]) != ln["total"]:
            err.append(f"line {ln['id']}: amounts do not add up to its total")
        if any(not isinstance(v, int) for v in ln["spending"]):
            err.append(f"line {ln['id']}: an amount is not whole lei")
    ids = [ln["id"] for ln in b["lines"]]
    if len(set(ids)) != len(ids):
        err.append("lines: an id is repeated")
    # AC-BUD-01: each type over the lines equals its published national total; the lines equal total expenditure.
    for k, t in enumerate(types):
        got = sum(ln["spending"][k] for ln in b["lines"])
        if got != pub[TYPE_TOTALS[t]]:
            err.append(f"type {t}: lines add up to {got}, published {TYPE_TOTALS[t]} is {pub[TYPE_TOTALS[t]]}")
    total = sum(ln["total"] for ln in b["lines"])
    if total != pub["TE"] or b["totals"]["total"] != pub["TE"]:
        err.append(f"total: lines add up to {total}, published TE is {pub['TE']}")
    if pub["D11"] + pub["D12"] != pub["D1"]:
        err.append("published: D11 + D12 is not D1")
    # AC-BUD-02: the lines are a partition of the COFOG subgroups, and each division equals its published total.
    subs = [s for ln in b["lines"] for s in ln["cofog"]]
    if len(set(subs)) != len(subs):
        err.append("lines: a COFOG subgroup is in two lines")
    fsubs = set()
    for f in b["by_function"]:
        if f["total"] != f["published_total"]:
            err.append(f"function {f['cofog']}: {f['total']} against published {f['published_total']}")
        if sum(f["spending"]) != f["total"]:
            err.append(f"function {f['cofog']}: amounts do not add up to its total")
        fsubs |= {s for s in subs if s.startswith(f["cofog"])}
    if fsubs != set(subs):
        err.append("lines: a subgroup outside the ten divisions")
    by_fn = [sum(col) for col in zip(*[f["spending"] for f in b["by_function"]])]
    by_ln = [sum(col) for col in zip(*[ln["spending"] for ln in b["lines"]])]
    if by_fn != by_ln:
        err.append("functions and lines do not add up to the same amounts by type")
    # AC-BUD-03: the programmes are the social benefits.
    cash = sum(p["cash"] for p in b["programmes"])
    kind = sum(p["in_kind"] for p in b["programmes"])
    if cash != pub["D62"] or kind != pub["D632"]:
        err.append(f"programmes: cash {cash} and in kind {kind} against D62 {pub['D62']} and D632 {pub['D632']}")
    if any(p["cash"] + p["in_kind"] != p["total"] for p in b["programmes"]):
        err.append("programmes: a total is not cash plus in kind")
    # AC-BUD-04: posts by sector add up to the published total; public units carry every post and every leu of wages.
    emp = b["employment"]
    if sum(s["posts"] for s in emp["sectors"]) != emp["posts"]:
        err.append("employment: sectors do not add up to the total posts")
    if emp["central"] + emp["local"] != emp["posts"]:
        err.append("employment: central and local do not add up to the total posts")
    units = {u["unit"]: u for u in b["public_units"]}
    if sorted(units) != sorted(PUBLIC_UNITS):
        err.append("public units: not the five public firm units")
    if any(s["unit"] not in units for s in emp["sectors"]):
        err.append("employment: a sector's unit is not a public unit")
    for u in b["public_units"]:
        if u["posts"] != sum(s["posts"] for s in emp["sectors"] if s["unit"] == u["unit"]):
            err.append(f"public unit {u['unit']}: posts are not its sectors' posts")
        if u["posts"] <= 0 and u["wages"] > 0:
            err.append(f"public unit {u['unit']}: wages without posts")
    if sum(u["posts"] for u in b["public_units"]) != emp["posts"]:
        err.append("public units: posts do not add up to the total")
    if sum(u["wages"] for u in b["public_units"]) != pub["D11"]:
        err.append("public units: wages do not add up to D11")
    if sum(u["employer_contributions"] for u in b["public_units"]) != pub["D12"]:
        err.append("public units: employer contributions do not add up to D12")
    return err


def monthly(lei_a_year: int) -> list[int]:
    """An amount a year in lei as twelve monthly amounts in bani: whole bani, the remainder to the first months.

    A negative amount is split as its absolute value, with the sign kept. The twelve add up exactly.
    """
    sign = -1 if lei_a_year < 0 else 1
    total = abs(lei_a_year) * BANI
    base, rest = divmod(total, MONTHS)
    return [sign * (base + (1 if m < rest else 0)) for m in range(MONTHS)]


def _fnv(h: int, data: bytes) -> int:
    for byte in data:
        h = ((h ^ byte) * FNV_PRIME) & M64
    return h


def _text(s: str) -> bytes:
    raw = s.encode("utf-8")
    if len(raw) > 255:
        raise ValueError("id longer than 255 bytes")
    return bytes([len(raw)]) + raw


def budget_hash(b: dict) -> int:
    """FNV-1a 64 over: the line count (u32 LE); per line its id (length byte, UTF-8) and each amount in bani
    (i64 LE, two's complement) in the order of `types`; the sector count (u32 LE); per sector its id and posts
    (u32 LE); per public unit, in file order, its id, posts (u32 LE), wages and employer contributions in bani
    (i64 LE)."""
    h = _fnv(FNV_OFFSET, len(b["lines"]).to_bytes(4, "little"))
    for ln in b["lines"]:
        h = _fnv(h, _text(ln["id"]))
        for v in ln["spending"]:
            h = _fnv(h, (v * BANI).to_bytes(8, "little", signed=True))
    sectors = b["employment"]["sectors"]
    h = _fnv(h, len(sectors).to_bytes(4, "little"))
    for s in sectors:
        h = _fnv(h, _text(s["id"]) + s["posts"].to_bytes(4, "little"))
    for u in b["public_units"]:
        h = _fnv(h, _text(u["unit"]) + u["posts"].to_bytes(4, "little")
                 + (u["wages"] * BANI).to_bytes(8, "little", signed=True)
                 + (u["employer_contributions"] * BANI).to_bytes(8, "little", signed=True))
    return h


def _ratio(got: float, want: float) -> float:
    return round(got / want, 4) if want else float("inf")


def _check(name: str, got: float, want: float, **facts) -> dict:
    r = _ratio(got, want)
    return {"name": name, "got": round(got), "want": round(want), "ratio": r, "tolerance": TOL[name],
            "ok": abs(r - 1) <= TOL[name], **facts}


def public_population(fixtures: Path, sample_scale: int, seed: int = 42) -> dict:
    """Weighted public employees and their wage bill (lei a year) by public unit, from the Python references."""
    sys.path.insert(0, str(ROOT / "pipeline"))
    import build_scenario as bs  # noqa: PLC0415

    d, _ = bs.load(fixtures)
    s = bs.run_stages(d, sample_scale, seed)
    units = d["firms"]["units"]
    pw = [s["pop"]["hh_weight"][h] for h in s["pop"]["household_id"]]
    out = {u: {"employees": 0, "wages": 0} for u in PUBLIC_UNITS}
    for i, e in enumerate(s["employers"]["employer"]):
        if e != bs.ref.NO_EMPLOYER and s["jobs"]["employment_status"][i] == bs.ref.KIND_EMPLOYEE \
                and units[e]["id"] in out:
            out[units[e]["id"]]["employees"] += pw[i]
            out[units[e]["id"]]["wages"] += pw[i] * s["wage"][i]
    for v in out.values():
        v["wages"] = v["wages"] * MONTHS // BANI
    return out


def reconcile(d: dict, population: dict | None = None) -> list[dict]:
    """AC-BUD-05 to 09: the budget against the other parts of the starting state."""
    b, firms = d["budget"], d["firms"]
    pub = b["published"]
    fu = {u["id"]: u for u in firms["units"] if u["size"] == "public"}
    bu = {u["unit"]: u for u in b["public_units"]}
    checks = [
        _check("firm_units_public_wages", sum(u["wages"] for u in fu.values()), pub["D11"],
               note="public units' wage bill in the firm-unit file against the budget's wages (D11), lei a year"),
        _check("firm_units_public_persons", sum(u["persons_employed"] for u in fu.values()), b["employment"]["posts"],
               note="public units' persons employed in the firm-unit file against occupied posts, December 2021"),
    ]
    by_unit = {u: {"persons": [fu[u]["persons_employed"], bu[u]["posts"]], "wages": [fu[u]["wages"], bu[u]["wages"]]}
               for u in PUBLIC_UNITS if u in fu}
    worst = max(max(abs(_ratio(*v["persons"]) - 1), abs(_ratio(*v["wages"]) - 1)) for v in by_unit.values())
    checks.append({"name": "firm_units_by_unit", "ok": worst <= TOL["firm_units_by_unit"], "worst": round(worst, 4),
                   "tolerance": TOL["firm_units_by_unit"], "firm_file_against_budget": by_unit})
    groups = d["earnings"]["industry_groups"]
    group_bill = d["earnings"]["wage_bill"][groups.index("public_education_health")] * MONTHS
    checks.append(_check("earnings_public_group", group_bill, pub["D11"],
                         note="wages of the industry group public_education_health (all employers, 2021-Q4 a "
                              "month, times twelve) against the budget's wages (D11)"))
    pensions = sum(p["cash"] for p in b["programmes"] if p["id"] in ("old_age", "survivors"))
    checks.append(_check("pensions", pensions, d["pensions"]["pension_bill"] * MONTHS,
                         note="old-age and survivors' cash benefits against the pension bill of the pension stage"))
    gov = d["io"]["final"]["government"]
    io_gov = sum(sum(gov[k]) for k in ("domestic", "imports_eu", "imports_non_eu"))
    checks.append(_check("io_government_consumption", io_gov, pub["P3"],
                         note="government final consumption in the input-output table against P3"))
    if population is not None:
        emp = sum(v["employees"] for v in population.values())
        wages = sum(v["wages"] for v in population.values())
        checks.append(_check("population_public_employees", emp, b["employment"]["posts"],
                             by_unit={u: [v["employees"], bu[u]["posts"]] for u, v in population.items()}))
        checks.append(_check("population_public_wages", wages, pub["D11"],
                             by_unit={u: [v["wages"], bu[u]["wages"]] for u, v in population.items()}))
    for c in checks:
        if c["name"] in KNOWN_GAPS:
            c["known_gap"] = KNOWN_GAPS[c["name"]]
    return checks


def summary(d: dict, population: dict | None = None) -> dict:
    b = d["budget"]
    errors = structure_errors(b)
    checks = reconcile(d, population)
    ok = not errors and all(c["ok"] for c in checks if c["name"] not in KNOWN_GAPS)
    return {
        "ok": ok,
        "hash": f"{budget_hash(b):016x}",
        "total": b["totals"]["total"],
        "share_of_gdp_ppm": b["totals"]["total"] * 1_000_000 // b["gdp"],
        "by_category": b["totals"]["by_category"],
        "posts": b["employment"]["posts"],
        "structure_errors": errors,
        "reconciliation": checks,
        "lines_first_month_bani": {ln["id"]: [monthly(v)[0] for v in ln["spending"]] for ln in b["lines"]},
    }


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--fixtures", type=Path, default=FIXTURES)
    ap.add_argument("--population-scale", type=int, default=None,
                    help="also reconcile with the public employees of the starting population at this scale")
    ap.add_argument("--seed", type=int, default=42)
    a = ap.parse_args()
    d = load(a.fixtures)
    pop = public_population(a.fixtures, a.population_scale, a.seed) if a.population_scale else None
    out = summary(d, pop)
    print(json.dumps(out, indent=1))
    return 0 if out["ok"] else 1


if __name__ == "__main__":
    sys.exit(main())
