#!/usr/bin/env python3
"""Give every non-financial cohort firm unit its opening balance sheet (ADR-0016).

Writes `data/derived/firm_balance_sheets2021_ro.json`, in lei, end of 2021:
for each unit of `firm_units2021_ro.json` that is a cohort outside finance, its
fixed assets and its financial assets and liabilities, for all its firms
together. Each column sums exactly to the sector's total.

Measured: the totals. Financial positions are those of non-financial
corporations in the financial accounts; fixed assets are theirs in the capital
stock accounts, plus the business capital of households (their fixed assets
other than dwellings), which belongs to unincorporated micro firms.

Guessed (`GUESSES`): how each total is spread over the units. No statistic
gives balance sheets by industry and size class here. The keys are simple:
capital and loans follow a unit's operating surplus, money and trade credit
follow its output, and small firms borrow less from banks per leu of capital
than large and foreign-controlled ones.

Not here: public units (their assets and debts are the government's) and the
units of banking and insurance (their balance sheets are the financial
sectors' own).
"""
from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from jsonstat import to_rows  # noqa: E402
from normalise_census import CensusError  # noqa: E402

HERE = Path(__file__).resolve().parent
RAW = HERE / "data" / "raw"
CAPITAL_ID = "ro_capital_stock_2021"
FINANCIAL_INDUSTRIES = {"banking_services", "insurance_pensions"}

GUESSES = {
    # What each position follows: a unit's operating surplus (value added less wages) or its output.
    "key": {"fixed_assets": "surplus", "loans": "surplus", "equity_issued": "surplus", "debt_securities_issued": "surplus",
            "deposits": "output", "currency": "output", "other_receivable": "output", "other_payable": "output",
            "loans_granted": "output", "equity_held": "surplus", "debt_securities_held": "output", "insurance": "output"},
    # Bank and parent-company loans per leu of capital, relative: micro firms borrow least,
    # foreign-controlled firms most (loans from their groups).
    "loan_tilt": {"micro": 0.5, "small": 0.8, "large": 1.2, "foreign": 1.5},
    # Listed debt is issued by large firms only; shares in other firms are held by small and large ones.
    "only_large": ["debt_securities_issued"],
    "not_micro": ["equity_held"],
}
# (name in the output, side, instrument of the sector balance sheets)
POSITIONS = [
    ("currency", "assets", "currency"), ("deposits", "assets", "deposits"),
    ("debt_securities_held", "assets", "debt_securities"), ("loans_granted", "assets", "loans"),
    ("equity_held", "assets", "equity"), ("insurance", "assets", "insurance_and_pensions"),
    ("other_receivable", "assets", "other_accounts"),
    ("debt_securities_issued", "liabilities", "debt_securities"), ("loans", "liabilities", "loans"),
    ("equity_issued", "liabilities", "equity"), ("other_payable", "liabilities", "other_accounts"),
]


def apportion(total: int, weights: list[int]) -> list[int]:
    """Split `total` in proportion to `weights` by largest remainder, ties to the lower index."""
    wsum = sum(weights)
    if wsum <= 0:
        raise CensusError("nothing to spread a total over")
    parts = [total * w // wsum for w in weights]
    order = sorted(range(len(weights)), key=lambda i: (-(total * weights[i] % wsum), i))
    for i in order[: total - sum(parts)]:
        parts[i] += 1
    return parts


def build(firms: dict, sheets: dict, capital_doc: dict) -> dict:
    """`firms` is the firm-unit file, `sheets` the sector balance sheets, `capital_doc` fixed assets by sector."""
    units = [u for u in firms["units"] if u["kind"] == "cohort" and u["industry"] not in FINANCIAL_INDUSTRIES]
    if not units:
        raise CensusError("no non-financial cohort units")
    cells = {(r["sector"], r["asset10"]): float(r["value"]) for r in to_rows(capital_doc)}
    try:
        corporate = round(cells[("S11", "N11N")] * 1_000_000)
        household_business = round((cells[("S14_S15", "N11N")] - cells[("S14_S15", "N111N")]) * 1_000_000)
    except KeyError as e:
        raise CensusError(f"capital stock: {e} is missing") from e
    surplus = [max(u["value_added"] - u["wages"], 0) for u in units]
    output = [u["output"] for u in units]
    tilt = GUESSES["loan_tilt"]

    def weights(name: str) -> list[int]:
        base = surplus if GUESSES["key"][name] == "surplus" else output
        out = []
        for u, b in zip(units, base):
            w = b
            if name == "loans":
                w = round(b * (tilt["foreign"] if u["controller"] == "foreign" else tilt[u["size"]]))
            if name in GUESSES["only_large"] and u["size"] != "large":
                w = 0
            if name in GUESSES["not_micro"] and u["size"] == "micro":
                w = 0
            out.append(w)
        return out

    columns = {name: apportion(sheets[side][instrument]["firms"], weights(name)) for name, side, instrument in POSITIONS}
    # Fixed assets: corporations' over all units; households' business capital over the domestic micro units.
    fixed = apportion(corporate, weights("fixed_assets"))
    micro = [s if u["size"] == "micro" and u["controller"] == "domestic" else 0 for u, s in zip(units, surplus)]
    for k, extra in enumerate(apportion(household_business, micro)):
        fixed[k] += extra
    rows = []
    for k, u in enumerate(units):
        row = {"id": u["id"], "firm_count": u["firm_count"], "fixed_assets": fixed[k]}
        row.update({name: columns[name][k] for name, _, _ in POSITIONS})
        assets = fixed[k] + sum(columns[name][k] for name, side, _ in POSITIONS if side == "assets")
        row["net_worth_after_equity"] = assets - sum(columns[name][k] for name, side, _ in POSITIONS if side == "liabilities")
        rows.append(row)
    totals = {name: sum(columns[name]) for name, _, _ in POSITIONS}
    totals["fixed_assets"] = sum(fixed)
    return {"date": "2021-12-31", "unit": "lei", "units": rows, "totals": totals,
            "fixed_assets_of_corporations": corporate, "business_fixed_assets_of_households": household_business}


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--raw", type=Path, default=RAW)
    ap.add_argument("--firms", type=Path, default=HERE / "data" / "derived" / "firm_units2021_ro.json")
    ap.add_argument("--sheets", type=Path, default=HERE / "data" / "derived" / "balance_sheets2021q4_ro.json")
    ap.add_argument("--out", type=Path, default=HERE / "data" / "derived" / "firm_balance_sheets2021_ro.json")
    a = ap.parse_args()
    raw = (a.raw / f"{CAPITAL_ID}.json").read_bytes()
    side = a.raw / f"{CAPITAL_ID}.provenance.json"
    prov = json.loads(side.read_text(encoding="utf-8")) if side.exists() else {"id": CAPITAL_ID}
    prov["sha256"] = hashlib.sha256(raw).hexdigest()
    m = build(json.loads(a.firms.read_text(encoding="utf-8")), json.loads(a.sheets.read_text(encoding="utf-8")),
              json.loads(raw))
    m["provenance"] = {"sources": [prov], "guesses": GUESSES,
                       "note": "Totals from Eurostat's financial accounts and capital stock accounts; the spread over "
                               "units is an educated guess, listed under `guesses`"}
    a.out.parent.mkdir(parents=True, exist_ok=True)
    a.out.write_text(json.dumps(m, separators=(",", ":")) + "\n", encoding="utf-8")
    shown = {k: round(v / 1e9, 1) for k, v in m["totals"].items()}
    print(f"OK - {len(m['units'])} units, billion lei: {shown} -> {a.out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
