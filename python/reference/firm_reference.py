#!/usr/bin/env python3
"""Independent reference for the undeclared sales and the opening inventories of
the firm units (spec `economy/firm-units-undeclared-inventories`).

Written from the spec, standard library only, integer arithmetic only, no
randomness, so a Rust implementation can be compared with it unit for unit.

    python python/reference/firm_reference.py FIRM_UNITS.json MARGINS.json [--units]

prints a JSON summary (totals, checks against the anchors, the hash); with
`--units` it also prints every unit's columns.
"""
from __future__ import annotations

import argparse
import json
import sys

M64 = 0xFFFFFFFFFFFFFFFF
UNIT = 1_000_000  # shares and tilts are in millionths
CLASSES = ["micro", "small", "large", "foreign"]  # a cohort unit's class: its size, or foreign if foreign-controlled
COLUMNS = ["undeclared_wages", "undeclared_value_added", "undeclared_sales", "undeclared_wage_share",
           "undeclared_sales_share", "inventory", "inventory_materials"]


class FirmError(Exception):
    """A named error: EmptyTable, ShapeMismatch, ShareAboveOne, AnchorsInconsistent, NoRoom."""


def apportion(total: int, weights: list[int]) -> list[int]:
    """Split `total` in proportion to `weights` by largest remainder, ties to the lower index."""
    if total == 0:
        return [0] * len(weights)
    wsum = sum(weights)
    if wsum <= 0:
        raise FirmError("NoRoom")
    parts = [total * w // wsum for w in weights]
    order = sorted(range(len(weights)), key=lambda i: (-(total * weights[i] % wsum), i))
    for i in order[: total - sum(parts)]:
        parts[i] += 1
    return parts


def apportion_capped(total: int, weights: list[int], caps: list[int]) -> list[int]:
    """Split `total` by `weights`, no part above its cap: apportion over the members with a weight, fix every
    member that would pass its cap at the cap, and apportion what is left over the others again."""
    if total < 0:
        raise FirmError("AnchorsInconsistent")
    parts = [0] * len(weights)
    active = [i for i, w in enumerate(weights) if w > 0 and caps[i] > 0]
    left = total
    while left > 0:
        if not active:
            raise FirmError("NoRoom")
        share = apportion(left, [weights[i] for i in active])
        over = [i for i, s in zip(active, share) if parts[i] + s > caps[i]]
        if not over:
            for i, s in zip(active, share):
                parts[i] += s
            break
        for i in over:
            left -= caps[i] - parts[i]
            parts[i] = caps[i]
        active = [i for i in active if i not in set(over)]
    return parts


def unit_class(u: dict) -> int:
    return 3 if u["controller"] == "foreign" else CLASSES.index(u["size"])


def check_inputs(firms: dict, m: dict) -> None:
    units, industries = firms.get("units", []), firms.get("industries", [])
    if not units or not industries:
        raise FirmError("EmptyTable")
    ids = [i["id"] for i in industries]
    if [i["id"] for i in m["industries"]] != ids or any(u["industry"] not in ids for u in units):
        raise FirmError("ShapeMismatch")
    groups = 1 + max(i["industry_group"] for i in industries)
    for key in ("undeclared_wage_share", "household_business_propensity", "surplus_from_sales"):
        if len(m[key]) != groups:
            raise FirmError("ShapeMismatch")
    for key in ("wage_tilt", "informal_tilt", "profit_tilt"):
        if len(m[key]) != len(CLASSES):
            raise FirmError("ShapeMismatch")
    shares = [*m["undeclared_wage_share"], *m["surplus_from_sales"], m["undeclared_value_added_share"],
              m["max_undeclared_share"], *(i["finished_share"] for i in m["industries"])]
    if any(s < 0 or s > UNIT for s in shares):
        raise FirmError("ShareAboveOne")


def undeclared_and_inventories(firms: dict, m: dict) -> dict:
    """The seven columns of the spec for every unit of the firm-unit file, as lists aligned with `units`.
    Amounts are lei a year (undeclared) and lei (inventories); shares are millionths."""
    check_inputs(firms, m)
    units = firms["units"]
    n = len(units)
    ind = {i["id"]: k for k, i in enumerate(firms["industries"])}
    group = [firms["industries"][ind[u["industry"]]]["industry_group"] for u in units]
    margin = [m["industries"][ind[u["industry"]]] for u in units]
    cohort = [u["kind"] == "cohort" for u in units]
    cls = [unit_class(u) if c else -1 for u, c in zip(units, cohort)]
    va = [max(u["value_added"], 0) for u in units]
    # Surplus earned on sales: the rest (imputed rent of owner-occupiers, in real estate) cannot be hidden.
    surplus = [max(u["value_added"] - u["wages"], 0) * m["surplus_from_sales"][group[f]] // UNIT
               for f, u in enumerate(units)]
    hideable = [max(va[f] - max(u["value_added"] - u["wages"], 0) + surplus[f], 0) for f, u in enumerate(units)]
    room = [hideable[f] * m["max_undeclared_share"] // UNIT if cohort[f] else 0 for f in range(n)]

    # Step 1: undeclared wages, per industry group, by wage bill times the class's tilt.
    wages_u = [0] * n
    for g in sorted(set(group)):
        members = [f for f in range(n) if cohort[f] and group[f] == g]
        target = sum(units[f]["wages"] for f in members) * m["undeclared_wage_share"][g] // UNIT
        if target == 0:
            continue
        got = apportion_capped(target, [units[f]["wages"] * m["wage_tilt"][cls[f]] for f in members],
                               [min(units[f]["wages"], room[f]) for f in members])
        for f, x in zip(members, got):
            wages_u[f] = x

    # Step 2: undeclared income of households: farm income in the domestic micro units of agriculture,
    # self-employment income in the domestic micro units, informal income in the domestic cohort units.
    house = [0] * n
    domestic = [cohort[f] and units[f]["controller"] == "domestic" for f in range(n)]
    micro = [domestic[f] and units[f]["size"] == "micro" for f in range(n)]
    prop = m["household_business_propensity"]
    for amount, weight in (
            (m["undeclared_farm_income"], [surplus[f] if micro[f] and group[f] == 0 else 0 for f in range(n)]),
            (m["undeclared_self_employment_income"], [surplus[f] * prop[group[f]] if micro[f] else 0 for f in range(n)]),
            (m["informal_income"], [surplus[f] * m["informal_tilt"][cls[f]] if domestic[f] else 0
                                    for f in range(n)])):
        got = apportion_capped(amount, weight, [room[f] - wages_u[f] - house[f] for f in range(n)])
        house = [a + b for a, b in zip(house, got)]

    # Step 3: undeclared profit of firms: what the anchor leaves, by surplus, industry group and class.
    target = sum(va[f] for f in range(n) if cohort[f]) * m["undeclared_value_added_share"] // UNIT
    rest = target - sum(wages_u) - sum(house)
    if rest < 0:
        raise FirmError("AnchorsInconsistent")
    weight = [surplus[f] * (m["undeclared_wage_share"][group[f]] + m["profit_group_floor"]) * m["profit_tilt"][cls[f]]
              if cohort[f] else 0 for f in range(n)]
    profit = apportion_capped(rest, weight, [room[f] - wages_u[f] - house[f] for f in range(n)])

    out = {k: [0] * n for k in COLUMNS}
    for f, u in enumerate(units):
        uva = wages_u[f] + house[f] + profit[f]
        sales = min(u["output"], uva * u["output"] // va[f]) if va[f] > 0 else 0
        out["undeclared_wages"][f] = wages_u[f]
        out["undeclared_value_added"][f] = uva
        out["undeclared_sales"][f] = sales
        out["undeclared_wage_share"][f] = wages_u[f] * UNIT // u["wages"] if u["wages"] > 0 else 0
        out["undeclared_sales_share"][f] = sales * UNIT // u["output"] if u["output"] > 0 else 0

    # Inventories: corporations' over every cohort unit, households' business inventories over the domestic
    # micro units, both by output times the industry's days; then finished goods and materials.
    key = [u["output"] * margin[f]["inventory_days"] if cohort[f] else 0 for f, u in enumerate(units)]
    corp = apportion(m["inventories"]["corporations"], key)
    hh = apportion(m["inventories"]["household_business"], [k if micro[f] else 0 for f, k in enumerate(key)])
    for f in range(n):
        total = corp[f] + hh[f]
        finished = total * margin[f]["finished_share"] // UNIT
        out["inventory"][f] = finished
        out["inventory_materials"][f] = total - finished
    return out


def columns_hash(cols: dict) -> int:
    """FNV-1a 64 over the unit count (u32, little-endian), then for each unit the seven columns
    (u64, little-endian) in the order of COLUMNS."""
    n = len(cols[COLUMNS[0]])
    data = bytearray(n.to_bytes(4, "little"))
    for f in range(n):
        for k in COLUMNS:
            data += cols[k][f].to_bytes(8, "little")
    h = 0xCBF29CE484222325
    for b in data:
        h = ((h ^ b) * 0x100000001B3) & M64
    return h


def fit(firms: dict, m: dict, cols: dict) -> dict:
    """The totals and the checks of the spec's acceptance tests."""
    units = firms["units"]
    ind = {i["id"]: k for k, i in enumerate(firms["industries"])}
    group = [firms["industries"][ind[u["industry"]]]["industry_group"] for u in units]
    cohort = [u["kind"] == "cohort" for u in units]
    tot = {k: sum(cols[k]) for k in ("undeclared_wages", "undeclared_value_added", "undeclared_sales",
                                     "inventory", "inventory_materials")}
    private_va = sum(max(u["value_added"], 0) for u, c in zip(units, cohort) if c)
    private_out = sum(u["output"] for u, c in zip(units, cohort) if c)
    wages_by_group, undeclared_by_group = {}, {}
    for f, u in enumerate(units):
        if cohort[f]:
            wages_by_group[group[f]] = wages_by_group.get(group[f], 0) + u["wages"]
            undeclared_by_group[group[f]] = undeclared_by_group.get(group[f], 0) + cols["undeclared_wages"][f]
    gap = {g: round(undeclared_by_group[g] / wages_by_group[g] - m["undeclared_wage_share"][g] / UNIT, 6)
           for g in sorted(wages_by_group) if wages_by_group[g] > 0}
    by_class = {}
    for k, name in enumerate(CLASSES):
        fs = [f for f, u in enumerate(units) if cohort[f] and unit_class(u) == k]
        by_class[name] = {
            "undeclared_wage_share": round(sum(cols["undeclared_wages"][f] for f in fs)
                                           / max(sum(units[f]["wages"] for f in fs), 1), 4),
            "undeclared_sales_share": round(sum(cols["undeclared_sales"][f] for f in fs)
                                            / max(sum(units[f]["output"] for f in fs), 1), 4)}
    # VAT gap proxy: the share of households' purchases (at basic prices, by the industry that makes them) bought
    # undeclared, each unit taking its industry's purchases in proportion to its output.
    out_by_ind: dict[str, int] = {}
    for u in units:
        out_by_ind[u["industry"]] = out_by_ind.get(u["industry"], 0) + u["output"]
    lost = 0.0
    for f, u in enumerate(units):
        if out_by_ind[u["industry"]] > 0 and u["output"] > 0:
            hh = m["industries"][ind[u["industry"]]]["household_purchases"] * u["output"] / out_by_ind[u["industry"]]
            lost += hh * cols["undeclared_sales"][f] / u["output"]
    hh_total = sum(i["household_purchases"] for i in m["industries"])
    inv_days = {}
    for g in sorted(set(group)):
        fs = [f for f in range(len(units)) if group[f] == g]
        o = sum(units[f]["output"] for f in fs)
        inv_days[g] = round((sum(cols["inventory"][f] + cols["inventory_materials"][f] for f in fs)) * 365 / o, 1) if o else 0
    return {
        "billion_lei": {k: round(v / 1e9, 2) for k, v in tot.items()},
        "inventories_total": tot["inventory"] + tot["inventory_materials"],
        "inventories_target": m["inventories"]["corporations"] + m["inventories"]["household_business"],
        "undeclared_value_added_total": tot["undeclared_value_added"],
        "undeclared_value_added_target": private_va * m["undeclared_value_added_share"] // UNIT,
        "undeclared_share_of_private_value_added": round(tot["undeclared_value_added"] / private_va, 4),
        "undeclared_share_of_private_sales": round(tot["undeclared_sales"] / private_out, 4),
        "undeclared_wage_share_gap_by_group": gap,
        "household_part": m["undeclared_farm_income"] + m["undeclared_self_employment_income"] + m["informal_income"],
        "by_class": by_class,
        "vat_gap_proxy": round(lost / hh_total, 4) if hh_total else 0.0,
        "vat_gap_anchor": m["anchors"]["vat_gap"] / UNIT,
        "inventory_days_by_group": inv_days,
    }


def population_wage_shares(firms: dict, employer: list[int], wage: list[int], wage_declared: list[int],
                           weight: list[int]) -> dict:
    """The undeclared share of each class's wage bill in the synthetic population: employees' `wage` less
    `wage_declared` (Stage I), by the class of their employer (Stage J), weighted. Persons without an employer
    (65535) or a wage are left out."""
    units = firms["units"]
    paid, hidden = [0] * (len(CLASSES) + 1), [0] * (len(CLASSES) + 1)
    for e, w, d, p in zip(employer, wage, wage_declared, weight):
        if e == 65535 or w == 0:
            continue
        k = len(CLASSES) if units[e]["kind"] != "cohort" else unit_class(units[e])
        paid[k] += p * w
        hidden[k] += p * (w - d)
    return {name: round(hidden[k] / paid[k], 4) for k, name in enumerate([*CLASSES, "public"]) if paid[k]}


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("firms", help="the firm-unit file (firm_units2021_ro.json)")
    ap.add_argument("margins", help="the margin file (firm_undeclared_inventories2021_ro.json)")
    ap.add_argument("--units", action="store_true", help="also print every unit's columns")
    a = ap.parse_args()
    with open(a.firms, encoding="utf-8") as f:
        firms = json.load(f)
    with open(a.margins, encoding="utf-8") as f:
        margins = json.load(f)
    cols = undeclared_and_inventories(firms, margins)
    out = {"units": len(firms["units"]), "hash": f"{columns_hash(cols):016x}", "fit": fit(firms, margins, cols)}
    if a.units:
        out["columns"] = [{"id": u["id"], **{k: cols[k][f] for k in COLUMNS}} for f, u in enumerate(firms["units"])]
    print(json.dumps(out, indent=1))
    return 0


if __name__ == "__main__":
    sys.exit(main())
