#!/usr/bin/env python3
"""Give the firm units their undeclared sales and their opening inventories
(spec `economy/firm-units-undeclared-inventories`; ADR-0016).

Writes `data/derived/firm_undeclared_inventories2021_ro.json`:

    margins     what the rule of the spec needs besides the firm-unit file: per catalogue industry the days of
                output held as inventories, the finished share and households' purchases (a check); the
                undeclared wage shares and household-business income of the incomes stage; the anchor of the
                undeclared share of private value added; the measured inventories of firms and of households'
                businesses; the guessed tilts
    units       the seven columns of every unit, computed by the Python reference (`firm_reference.py`)
    totals      their sums and the checks of the reference

Measured: inventories of non-financial corporations and of households at the end of 2021 (Eurostat capital
stock accounts); households' purchases by industry (the input-output table). Taken from the incomes stage so
that the two agree: the undeclared share of each industry group's wage bill and the undeclared income of
household businesses. Anchors (research note informal-economy-romania): undeclared work was 27.1% of private
value added (European Labour Authority, 2019), the VAT gap 36.7% (European Commission, 2021).
Guessed (`GUESSES`, confidence low to medium): days of output held as inventories and the finished share by
industry, the tilts by size class and foreign control. The owner's steer of 2026-10-11 applies: plausible
behaviour matters, matching 2021 one to one does not.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(HERE.parent / "reference"))
import firm_reference as ref  # noqa: E402
from jsonstat import to_rows  # noqa: E402
from normalise_census import CensusError  # noqa: E402

RAW = HERE / "data" / "raw"
INVENTORIES_ID = "ro_inventories_2021"
UNIT = 1_000_000

# Measured sizes of the undeclared economy (research note informal-economy-romania). The value-added share is
# the target of the rule; the VAT gap is a check.
ANCHORS = {"undeclared_share_of_private_value_added": 0.271, "vat_gap": 0.367}

GUESSES = {
    # Days of a year's output held as inventories at the end of the year, by table industry (exact code first,
    # then its letter), and the share of them that is the industry's own output (finished goods and work in
    # progress); the rest is materials, supplies and goods bought for resale. Only the relative sizes matter:
    # the totals are measured. Pattern: inventory-to-sales ratios of manufacturing, wholesale and retail in the
    # US Census Bureau's monthly surveys (about 1.3 to 1.5 months of sales), harvest stocks at the year's end in
    # agriculture, work in progress in construction and real estate, almost nothing in services. Confidence low.
    "inventory_days": {
        "A01": [120, 0.75], "A02": [60, 0.6], "A03": [30, 0.5], "B": [40, 0.5],
        "C10-12": [45, 0.5], "C13-15": [70, 0.5], "C16": [60, 0.5], "C17": [45, 0.45], "C18": [25, 0.3],
        "C19": [35, 0.5], "C20": [55, 0.5], "C21": [90, 0.55], "C22": [50, 0.45], "C23": [55, 0.5],
        "C24": [60, 0.5], "C25": [60, 0.5], "C26": [70, 0.5], "C27": [65, 0.5], "C28": [90, 0.55],
        "C29": [35, 0.4], "C30": [120, 0.7], "C31_32": [60, 0.5], "C33": [30, 0.3],
        "D35": [15, 0.1], "E36": [10, 0.1], "E37-39": [10, 0.3], "F": [60, 0.8],
        # Trade: inventories are goods bought for resale; output is the trade margin, about a fifth to a
        # quarter of sales, so 1.3 months of sales is about 150 days of output.
        "G45": [120, 0.05], "G46": [150, 0.05], "G47": [150, 0.05],
        "H": [5, 0.0], "I": [10, 0.1], "J58": [30, 0.5], "J": [5, 0.2], "K": [0, 0.0],
        # Real estate: dwellings built for sale and not yet sold are inventories of developers.
        "L": [30, 0.9], "M": [5, 0.4], "N": [5, 0.2], "O": [0, 0.0], "P": [2, 0.0], "Q": [5, 0.0],
        "R": [5, 0.2], "S": [10, 0.3], "T": [0, 0.0], "U": [0, 0.0],
    },
    # How undeclared wages of an industry group are spread over its units, per leu of wage bill, by class
    # (micro, small, large, foreign-controlled). The incomes stage puts jobs with no contract on the lowest
    # wages first and the employer stage puts the lowest wages in the lowest-paying units (micro first), so
    # micro units carry most. Not a guess about Romania but a fit to the synthetic population: the tilt was set
    # so that the share of each class's wage bill paid undeclared comes out within a point of what Stages I and J
    # give at 1:100 (micro 18.1%, small 14.6%, large 12.6%, foreign-controlled 8.9%; see the spec).
    "wage_tilt": [1.15, 0.95, 0.9, 0.8],
    # Informal income (casual and seasonal work, undeclared second activities) is earned in domestic units of
    # every size, per leu of operating surplus, by class; small units half as much as micro and large ones;
    # foreign-controlled units: none. Day labourers in construction and farming, seasonal hands and cash-paid
    # extra hours are found in large domestic firms too, and nothing measured says they are concentrated in small
    # ones. The owner's decisions of 2026-10-14: first 1, 1, 1, 0 (it was 1, 0.5, 0.2, 0, which left micro and
    # small units hiding 59% and 47% of their sales), then 1, 0.5, 1, 0, which brings small units well below micro
    # ones (micro 50.0%, small 38.7%, large 34.7%). Farm sales and self-employment stay on domestic micro units
    # (they are the sales of household holdings and sole traders, micro by definition). Confidence low.
    "informal_tilt": [1.0, 0.5, 1.0, 0.0],
    # Share of each industry group's operating surplus that is earned on sales and can be hidden. Real estate:
    # most of its surplus is the imputed rent of owner-occupiers (households' operating surplus, 71 billion lei
    # of real estate's 89 billion), which is not a sale. Confidence medium.
    "surplus_from_sales": [1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 0.2, 1.0, 1.0, 1.0],
    # How undeclared profit is spread, per leu of operating surplus, by class: micro firms hide most, foreign-
    # controlled firms least (audited groups; their tax risk is transfer pricing, not hidden sales). Confidence low.
    "profit_tilt": [1.5, 1.5, 0.5, 0.15],
    # Undeclared profit of an industry group is also in proportion to its undeclared wage share, plus this
    # floor, so that groups with no undeclared wages (finance, public services sold on the market) hide a little.
    "profit_group_floor": 0.02,
    # No unit hides more than this share of its value added.
    "max_undeclared_share": 0.9,
    # How sure each guess is (the reasons are in the spec's table of guesses).
    "confidence": {"inventory_days": "low", "wage_tilt": "medium (fitted to the synthetic population)",
                   "informal_tilt": "low (educated guess, owner's decision of 2026-10-14)",
                   "surplus_from_sales": "medium", "profit_tilt": "low", "profit_group_floor": "low",
                   "max_undeclared_share": "low"},
}


def ppm(x: float) -> int:
    return round(x * UNIT)


def days_of(code: str) -> list:
    d = GUESSES["inventory_days"]
    if code in d:
        return d[code]
    if code[0] in d:
        return d[code[0]]
    raise CensusError(f"inventory days: no guess for table industry {code}")


def build(firms: dict, io: dict, incomes: dict, inventories_doc: dict) -> dict:
    """`firms` the firm-unit file, `io` the input-output margin file, `incomes` the margin file of the
    incomes stage, `inventories_doc` Eurostat's inventories by sector (JSON-stat)."""
    cells = {r["sector"]: float(r["value"]) for r in to_rows(inventories_doc) if r.get("asset10") == "N12N"}
    try:
        corporations = round(cells["S11"] * 1_000_000)
        households = round(cells["S14_S15"] * 1_000_000)
    except KeyError as e:
        raise CensusError(f"inventories: sector {e} is missing") from e
    pos = {ind["code"]: k for k, ind in enumerate(io["industries"])}
    table_of: dict[str, tuple[str, int]] = {}
    for entry in io["catalogue"]:
        for part in entry["parts"]:
            table_of.setdefault(part["id"], (entry["industry"], part["share"]))
    hh = io["final"]["households"]["domestic"]
    industries = []
    for ind in firms["industries"]:
        code, share = table_of[ind["id"]]
        days, finished = days_of(code)
        industries.append({"id": ind["id"], "table_industry": code, "inventory_days": days,
                           "finished_share": ppm(finished),
                           "household_purchases": hh[pos[code]] * share // UNIT})
    if incomes["industry_groups"] != io["industry_groups"]:
        raise CensusError("the incomes stage and the input-output table have different industry groups")
    # Undeclared income of households, a year: farm income sold (not eaten at home) less its declared part,
    # self-employment income less its declared part, and informal income.
    farm_cash = incomes["farm_income"] - incomes["farm_in_kind"]
    farm = 12 * (farm_cash * (UNIT - incomes["farm_declared"]) // UNIT)
    self_employment = 12 * (incomes["self_employment_income"] * (UNIT - incomes["self_employment_declared"]) // UNIT)
    g = GUESSES
    if len(g["surplus_from_sales"]) != len(io["industry_groups"]):
        raise CensusError("one surplus-from-sales share per industry group")
    return {
        "year": 2021, "unit": "lei a year (undeclared), lei at the end of 2021 (inventories); shares in millionths",
        "industries": industries,
        "industry_groups": io["industry_groups"],
        "undeclared_wage_share": incomes["undeclared_wage_share"],
        "household_business_propensity": incomes["informal_propensity"]["employee_by_industry_group"],
        "undeclared_farm_income": farm,
        "undeclared_self_employment_income": self_employment,
        "informal_income": 12 * incomes["informal_income"],
        "undeclared_value_added_share": ppm(ANCHORS["undeclared_share_of_private_value_added"]),
        "surplus_from_sales": [ppm(x) for x in g["surplus_from_sales"]],
        "wage_tilt": [ppm(x) for x in g["wage_tilt"]],
        "informal_tilt": [ppm(x) for x in g["informal_tilt"]],
        "profit_tilt": [ppm(x) for x in g["profit_tilt"]],
        "profit_group_floor": ppm(g["profit_group_floor"]),
        "max_undeclared_share": ppm(g["max_undeclared_share"]),
        "inventories": {"corporations": corporations, "household_business": households},
        "anchors": {k: ppm(v) for k, v in ANCHORS.items()},
    }


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--raw", type=Path, default=RAW)
    derived = HERE / "data" / "derived"
    ap.add_argument("--firms", type=Path, default=derived / "firm_units2021_ro.json")
    ap.add_argument("--io", type=Path, default=derived / "io_table2021_ro.json")
    ap.add_argument("--incomes", type=Path, default=derived / "other_incomes2021_ro.json")
    ap.add_argument("--out", type=Path, default=derived / "firm_undeclared_inventories2021_ro.json")
    a = ap.parse_args()
    raw = (a.raw / f"{INVENTORIES_ID}.json").read_bytes()
    side = a.raw / f"{INVENTORIES_ID}.provenance.json"
    prov = json.loads(side.read_text(encoding="utf-8")) if side.exists() else {"id": INVENTORIES_ID}
    prov["sha256"] = hashlib.sha256(raw).hexdigest()
    firms = json.loads(a.firms.read_text(encoding="utf-8"))
    m = build(firms, json.loads(a.io.read_text(encoding="utf-8")), json.loads(a.incomes.read_text(encoding="utf-8")),
              json.loads(raw))
    cols = ref.undeclared_and_inventories(firms, m)
    m["units"] = [{"id": u["id"], **{k: cols[k][f] for k in ref.COLUMNS}} for f, u in enumerate(firms["units"])]
    m["totals"] = {**{k: sum(cols[k]) for k in ref.COLUMNS if not k.endswith("_share")},
                   "hash": f"{ref.columns_hash(cols):016x}", "fit": ref.fit(firms, m, cols)}
    m["provenance"] = {
        "sources": [prov], "guesses": GUESSES, "anchors": ANCHORS,
        "from_other_files": ["firm_units2021_ro.json (units)", "io_table2021_ro.json (households' purchases)",
                             "other_incomes2021_ro.json (undeclared wage shares, household-business income)"],
        "note": "Inventories of non-financial corporations (S11) and households (S14_S15) from Eurostat's capital "
                "stock accounts; the anchor of 27.1% from the European Labour Authority (2023, reference year 2019) "
                "and the VAT gap of 36.7% from the European Commission (2021), both by way of the research note "
                "informal-economy-romania; every spread over units is an educated guess, listed under `guesses`"}
    a.out.parent.mkdir(parents=True, exist_ok=True)
    a.out.write_text(json.dumps(m, separators=(",", ":")) + "\n", encoding="utf-8")
    print(f"OK - {json.dumps(m['totals']['fit']['billion_lei'])} -> {a.out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
