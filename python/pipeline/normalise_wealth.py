#!/usr/bin/env python3
"""Normalise household wealth data into the margin file of the population
generator's wealth stage (spec `society/population-wealth`).

Writes `data/derived/household_wealth2021_ro.json`:

    totals of the household sector at the end of 2021, in lei: cash, deposits,
        housing loans, consumer loans, market value of owned dwellings
    liquid_curve, dwelling_curve, loan_curve   value at a rank / mean, millionths, lines through knots
    rel_locality[locality class]               mean dwelling value of the class, as proportions, millionths
    mortgage, consumer                         who borrows: share of households, ages, links
    income_link                                how closely liquid wealth and dwelling value follow income
    pattern                                    Hungary's shares of net wealth by quintile (a check only)

The totals of money and loans are measured (financial accounts). Everything
about who holds them is an educated guess, because no survey of household
wealth covers Romania (research note `household-wealth-romania`). The guesses
are the constants below; each carries its reason. The owner's steer of
2026-10-11: plausible interaction matters, matching 2021 one to one does not.
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
UNIT = 1_000_000
BALANCE_ID = "ro_household_balance_sheet_2021q4"
DWELLINGS_ID = "ro_household_dwellings_2021"
PATTERN_ID = "hu_wealth_pattern_2020"
QUINTILES = ["QU1", "QU2", "QU3", "QU4", "QU5"]

# --- educated guesses (confidence low unless said otherwise) -----------------------------
GUESSES = {
    # Banks' books split household loans roughly three to two between housing and consumer loans.
    "housing_loan_share": 0.58,
    # The capital stock values dwellings at replacement cost without land. What homes would sell
    # for is higher: about 6.7 million owned homes at an average near 170,000 lei.
    "market_value_over_replacement": 2.7,
    # Mean value of an owned dwelling by locality class (under 2,000 inhabitants ... 200,000 or more),
    # as proportions: a village house against a flat in a large city.
    "rel_locality": [0.45, 0.60, 0.85, 1.20, 1.60, 2.30],
    # Money is far more concentrated than homes: the top tenth of households holds about two
    # thirds of cash and deposits, the bottom half about 3%.
    "liquid_curve": {"rank": [0.0, 0.2, 0.4, 0.6, 0.8, 0.9, 0.99, 1.0],
                     "value": [0.01, 0.05, 0.16, 0.36, 0.90, 2.30, 9.0, 40.0]},
    # Homes: the cheapest tenth is worth under half the mean, the dearest tenth over twice.
    "dwelling_curve": {"rank": [0.0, 0.1, 0.5, 0.9, 0.99, 1.0],
                       "value": [0.30, 0.45, 0.80, 1.90, 4.0, 9.0]},
    # Loans outstanding: many small balances near the end of their term, few large new ones.
    "loan_curve": {"rank": [0.0, 0.25, 0.5, 0.75, 0.9, 1.0],
                   "value": [0.05, 0.35, 0.75, 1.30, 2.20, 5.0]},
    # Who has a housing loan: 6% of private households, owners with a head aged 25 to 59,
    # more often with a higher income and in towns.
    "mortgage": {"share": 0.06, "age_from": 25, "age_to": 60, "income_link": 0.6, "urban_bonus": 0.3},
    # Who has a consumer loan: a quarter of private households, head aged 20 to 64, loosely tied to income.
    "consumer": {"share": 0.25, "age_from": 20, "age_to": 65, "income_link": 0.3, "urban_bonus": 0.0},
    # How closely a household's place in each distribution follows its place by income (0 none, 1 exactly).
    "income_link": {"liquid": 0.6, "dwelling": 0.3},
}


def ppm(x: float) -> int:
    return round(x * UNIT)


def curve(spec: dict) -> dict:
    ranks, values = [ppm(r) for r in spec["rank"]], [ppm(v) for v in spec["value"]]
    if ranks[0] != 0 or ranks[-1] != UNIT or sorted(set(ranks)) != ranks or sorted(values) != values or values[0] <= 0:
        raise CensusError("a wealth curve must ascend from rank 0 to 1 with rising values above zero")
    return {"rank": ranks, "value": values}


def balance_sheet(doc: dict) -> dict:
    """Million lei by (side, instrument) of the household sector."""
    return {(r["finpos"], r["na_item"]): float(r["value"]) for r in to_rows(doc) if r["unit"] == "MIO_NAC"}


def lei(million: float) -> int:
    return round(million * 1_000_000)


def build(docs: dict) -> dict:
    sheet = balance_sheet(docs[BALANCE_ID])
    try:
        cash, deposits, loans = sheet[("ASS", "F21")], sheet[("ASS", "F22_F29")], sheet[("LIAB", "F4")]
    except KeyError as e:
        raise CensusError(f"household balance sheet: {e} is missing") from e
    dwellings = [float(r["value"]) for r in to_rows(docs[DWELLINGS_ID]) if r["asset10"] == "N111N"]
    if len(dwellings) != 1:
        raise CensusError("capital stock: expected one value for the dwellings of households")
    pattern = {(r["quant_inc"], r["quant_wlth"]): float(r["value"]) for r in to_rows(docs[PATTERN_ID])
               if r["indic_ewb"] == "WLTH_NET"}
    g = GUESSES
    housing = lei(loans * g["housing_loan_share"])
    borrowers = {}
    for kind in ("mortgage", "consumer"):
        b = g[kind]
        borrowers[kind] = {"share": ppm(b["share"]), "age_from": b["age_from"], "age_to": b["age_to"],
                           "income_link": ppm(b["income_link"]), "urban_bonus": ppm(b["urban_bonus"])}
    return {
        "totals": {
            "cash": lei(cash),
            "deposits": lei(deposits),
            "housing_loans": housing,
            "consumer_loans": lei(loans) - housing,
            "dwellings": lei(dwellings[0] * g["market_value_over_replacement"]),
        },
        "rel_locality": [ppm(v) for v in g["rel_locality"]],
        "liquid_curve": curve(g["liquid_curve"]),
        "dwelling_curve": curve(g["dwelling_curve"]),
        "loan_curve": curve(g["loan_curve"]),
        "mortgage": borrowers["mortgage"],
        "consumer": borrowers["consumer"],
        "income_link": {k: ppm(v) for k, v in g["income_link"].items()},
        "pattern": {
            "country": "HU", "year": 2020,
            "net_wealth_share_by_wealth_quintile": [ppm(pattern[("TOTAL", q)] / 100) for q in QUINTILES],
            "net_wealth_share_by_income_quintile": [ppm(pattern[(q, "TOTAL")] / 100) for q in QUINTILES],
        },
    }


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--raw", type=Path, default=RAW)
    ap.add_argument("--out", type=Path, default=HERE / "data" / "derived" / "household_wealth2021_ro.json")
    a = ap.parse_args()
    docs, prov = {}, []
    for ds in (BALANCE_ID, DWELLINGS_ID, PATTERN_ID):
        raw = (a.raw / f"{ds}.json").read_bytes()
        docs[ds] = json.loads(raw)
        side = a.raw / f"{ds}.provenance.json"
        p = json.loads(side.read_text(encoding="utf-8")) if side.exists() else {"id": ds}
        p["sha256"] = hashlib.sha256(raw).hexdigest()
        prov.append(p)
    m = build(docs)
    m["provenance"] = {"sources": prov, "guesses": GUESSES,
                       "note": "Eurostat financial accounts and capital stock (totals); Hungary's wealth shares "
                               "from Eurostat's experimental statistics (a check only); every distribution is an "
                               "educated guess, listed under `guesses`"}
    a.out.parent.mkdir(parents=True, exist_ok=True)
    a.out.write_text(json.dumps(m, separators=(",", ":")) + "\n", encoding="utf-8")
    print(f"OK - {m['totals']} -> {a.out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
