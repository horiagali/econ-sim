#!/usr/bin/env python3
"""Normalise the data for the incomes that are not wages or pensions into the
margin file of the population generator's income-split stage (spec
`society/population-incomes`).

Writes `data/derived/other_incomes2021_ro.json`:

    mixed_income            lei per month: mixed income of households, 2021-Q4 / 3 (measured)
    farm_income             lei per month: the part of it that is farming (measured share, guessed household part)
    shares and rules        how the rest is split and who gets it (educated guesses)
    undeclared_wage_share[industry group]   share of each group's wage bill paid outside contracts (guesses)
    quantile_curve          the earnings curve, for the spread of self-employment and informal income
    anchors                 the measured sizes of the undeclared economy, for checking

Measured: the total (national accounts). Guessed: every split of it, and
which wages are partly or wholly undeclared; no source gives these for Romania
(research note `informal-economy-romania`). The guesses are the constants
below, each with its reason. The owner's steer of 2026-10-11: plausible
interaction matters, matching 2021 one to one does not.
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
from normalise_census_jobs import INDUSTRY_GROUPS  # noqa: E402

HERE = Path(__file__).resolve().parent
RAW = HERE / "data" / "raw"
UNIT = 1_000_000
QUARTER_ID = "ro_household_mixed_income_2021q4"
YEAR_ID = "ro_household_mixed_income_2021"
AGRICULTURE_ID = "ro_na_agriculture_2021"

# --- educated guesses (confidence low) ------------------------------------------------------
GUESSES = {
    # Of agriculture's value added less its wages, the part earned by households and not by farm companies.
    "household_share_of_farm_surplus": 0.8,
    # Of households' farm income, the part that is food grown and eaten at home; the rest is sold.
    "farm_in_kind_share": 0.6,
    # Rural private households that grow food for themselves; those with a farm worker or a
    # retired member come first; how unequal their harvests are (0 all equal, 1 like earnings).
    "subsistence_household_share": 0.6,
    "subsistence_bonus": {"farm_worker": 0.6, "retired": 0.3},
    "subsistence_dispersion": 0.3,
    # Income of non-farm employers and own-account workers, as a share of mixed income:
    # about half a million people at 1.2 times the mean wage.
    "self_employment_share_of_mixed": 0.22,
    # What they declare of it; farmers declare less.
    "self_employment_declared": 0.6,
    "farm_declared": 0.2,
    # An employer earns this many times an own-account worker; a family worker this share.
    "rel_kind": {"employer": 2.0, "own_account": 1.0, "family_worker": 0.5},
    # How closely a self-employed person's place among the self-employed follows education.
    "self_employment_education_link": 0.3,
    # Share of each industry group's wage bill paid outside contracts: agriculture, industry, construction,
    # trade-transport-hospitality, information, finance, real estate, professional and administrative,
    # public administration-education-health, other services.
    "undeclared_wage_share": [0.40, 0.08, 0.25, 0.18, 0.03, 0.02, 0.15, 0.08, 0.0, 0.20],
    # Of the undeclared wages, the part earned in jobs with no contract at all; the rest is envelopes.
    "fully_informal_share": 0.35,
    # An envelope job declares the minimum wage, or this share of the true wage if that is more.
    "envelope_declared_share": 0.5,
    # How strongly low wages come first when jobs with no contract are picked (0 by chance, 1 strictly lowest first).
    "low_wage_link": 0.5,
    # People aged 15 to 74 with informal income outside a job, as a share of that age group.
    "informal_recipient_share": 0.14,
    # How likely each kind of person is to be one of them, relative to each other.
    "informal_propensity": {
        "unemployed": 0.60, "inactive_other": 0.30, "retired": 0.20, "student": 0.15,
        "own_account": 0.50, "family_worker": 0.40, "employer": 0.30,
        "employee_by_industry_group": [0.30, 0.05, 0.20, 0.15, 0.03, 0.03, 0.05, 0.05, 0.01, 0.18],
    },
}
# Measured sizes of the undeclared economy (research note), for checking only.
ANCHORS = {"undeclared_share_of_private_labour_input_2019": 0.217,
           "undeclared_share_of_private_value_added_2019": 0.271,
           "vat_gap_2021": 0.367}


def ppm(x: float) -> int:
    return round(x * UNIT)


def one(doc: dict, what: str, **where: str) -> float:
    rows = [r for r in to_rows(doc) if all(r.get(k) == v for k, v in where.items())]
    if len(rows) != 1:
        raise CensusError(f"{what}: expected one value, found {len(rows)}")
    return float(rows[0]["value"])


def build(docs: dict, earnings: dict) -> dict:
    """`earnings` is the earnings margin file (its curve and its minimum wage are reused)."""
    quarter = one(docs[QUARTER_ID], "mixed income of the quarter", na_item="B3G")
    year = one(docs[YEAR_ID], "mixed income of the year", na_item="B3G")
    surplus = (one(docs[AGRICULTURE_ID], "value added of agriculture", na_item="B1G")
               - one(docs[AGRICULTURE_ID], "compensation in agriculture", na_item="D1"))
    g = GUESSES
    farm_share = surplus * g["household_share_of_farm_surplus"] / year
    if not 0 < farm_share + g["self_employment_share_of_mixed"] < 1:
        raise CensusError("mixed income: farming and self-employment leave nothing, or less, for informal income")
    mixed = round(quarter * 1_000_000 / 3)
    farm = round(mixed * farm_share)
    self_employment = round(mixed * g["self_employment_share_of_mixed"])
    if len(g["undeclared_wage_share"]) != len(INDUSTRY_GROUPS):
        raise CensusError("one undeclared wage share per industry group")
    p = g["informal_propensity"]
    return {
        "industry_groups": INDUSTRY_GROUPS,
        "mixed_income": mixed,
        "farm_income": farm,
        "farm_in_kind": round(farm * g["farm_in_kind_share"]),
        "self_employment_income": self_employment,
        "informal_income": mixed - farm - self_employment,
        "minimum_wage": earnings["minimum_wage"],
        "quantile_curve": earnings["quantile_curve"],
        "subsistence_household_share": ppm(g["subsistence_household_share"]),
        "subsistence_bonus": {k: ppm(v) for k, v in g["subsistence_bonus"].items()},
        "subsistence_dispersion": ppm(g["subsistence_dispersion"]),
        "self_employment_education_link": ppm(g["self_employment_education_link"]),
        "self_employment_declared": ppm(g["self_employment_declared"]),
        "farm_declared": ppm(g["farm_declared"]),
        "rel_kind": [ppm(g["rel_kind"][k]) for k in ("employer", "own_account", "family_worker")],
        "undeclared_wage_share": [ppm(v) for v in g["undeclared_wage_share"]],
        "fully_informal_share": ppm(g["fully_informal_share"]),
        "envelope_declared_share": ppm(g["envelope_declared_share"]),
        "low_wage_link": ppm(g["low_wage_link"]),
        "informal_recipient_share": ppm(g["informal_recipient_share"]),
        "informal_propensity": {
            **{k: ppm(p[k]) for k in ("unemployed", "inactive_other", "retired", "student",
                                      "own_account", "family_worker", "employer")},
            "employee_by_industry_group": [ppm(v) for v in p["employee_by_industry_group"]],
        },
        "anchors": {k: ppm(v) for k, v in ANCHORS.items()},
    }


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--raw", type=Path, default=RAW)
    ap.add_argument("--earnings", type=Path, default=HERE / "data" / "derived" / "earnings2021_ro.json")
    ap.add_argument("--out", type=Path, default=HERE / "data" / "derived" / "other_incomes2021_ro.json")
    a = ap.parse_args()
    docs, prov = {}, []
    for ds in (QUARTER_ID, YEAR_ID, AGRICULTURE_ID):
        raw = (a.raw / f"{ds}.json").read_bytes()
        docs[ds] = json.loads(raw)
        side = a.raw / f"{ds}.provenance.json"
        p = json.loads(side.read_text(encoding="utf-8")) if side.exists() else {"id": ds}
        p["sha256"] = hashlib.sha256(raw).hexdigest()
        prov.append(p)
    m = build(docs, json.loads(a.earnings.read_text(encoding="utf-8")))
    m["provenance"] = {"sources": prov, "guesses": GUESSES,
                       "note": "Eurostat national accounts (mixed income, agriculture); every split is an educated "
                               "guess, listed under `guesses`; anchors from the research note informal-economy-romania"}
    a.out.parent.mkdir(parents=True, exist_ok=True)
    a.out.write_text(json.dumps(m, separators=(",", ":")) + "\n", encoding="utf-8")
    shown = {k: m[k] for k in ("mixed_income", "farm_income", "farm_in_kind", "self_employment_income", "informal_income")}
    print(f"OK - {shown} -> {a.out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
