#!/usr/bin/env python3
"""Normalise earnings data into the margin file of the population generator's
wage stage (spec `society/population-wages`).

Writes `data/derived/earnings2021_ro.json`:

    wage_bill[industry group]                  lei per month: national accounts, wages and salaries, 2021-Q4 / 3
    minimum_wage                               lei per month, gross, in force on 1 December 2021
    rel_sex[industry group][sex]                      mean earnings of the sex / mean of the group, millionths
    rel_occupation[industry group][sex][occupation]   mean earnings of the cell / mean of the group, millionths
    rel_age[industry group][sex][age group]           the same by age group
    rel_education[industry group][sex][edu group]     the same by education group
    quantile_curve                             earnings at a rank / mean earnings, millionths, as a line through knots

Levels come from the national accounts (the wage bill of each of the ten
industry groups). The Structure of Earnings Survey 2022 gives proportions
only: how much more a manager earns than a clerk in the same industry group.
It covers enterprises with 10 employees or more in NACE sections B to S, so:

- a group is the employee-weighted average of its sections that the survey has;
- agriculture (not surveyed) uses the proportions of the whole survey;
- a cell the survey does not publish uses the proportion of the whole survey
  for that sex and category, then for both sexes, then 1.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from jsonstat import to_rows  # noqa: E402
from normalise_census import SEXES, CensusError  # noqa: E402
from normalise_census_jobs import (  # noqa: E402
    EDU_GROUP_CODES, EDU_GROUP_OF_LEVEL, EDU_GROUPS, GROUP_CODES, GROUP_SECTIONS, INDUSTRY_GROUPS,
    OCCUPATION_CODES, OCCUPATIONS,
)

HERE = Path(__file__).resolve().parent
RAW = HERE / "data" / "raw"
UNIT = 1_000_000
# (dataset of mean earnings, dataset of employees, dimension, codes) per table.
TABLES = {
    "rel_occupation": ("ro_ses2022_earnings_occupation", "ro_ses2022_employees_occupation", "isco08", OCCUPATION_CODES),
    "rel_age": ("ro_ses2022_earnings_age", "ro_ses2022_employees_age", "age", ["Y_LT30", "Y30-49", "Y_GE50"]),
    "rel_education": ("ro_ses2022_earnings_education", "ro_ses2022_employees_education", "isced11", EDU_GROUP_CODES),
}
QUANTILES_ID = "ro_ses2022_earnings_quantiles"
WAGES_ID = "ro_na_wages_2021q4"
MINIMUM_WAGE_ID = "ro_minimum_wage"
AGE_GROUP_FROM = [0, 30, 50]          # first age (years) of the survey's three age groups
ALL = "B-S_X_O"                       # the whole survey: sections B to S without public administration
SURVEY_SIZE = "GE10"
START_HALF_YEAR, SURVEY_HALF_YEAR = "2021-S2", "2022-S2"
QUARTER = "2021-Q4"
# Ranks of the knots of the quantile curve above the ninth decile, in millionths.
TAIL_RANKS = [950_000, 990_000, 999_000, 1_000_000]
TAIL_CAP_RANK = 0.9999                # the curve's last knot has the earnings of this rank


def survey_cells(doc: dict, dim: str) -> dict:
    """{(section or aggregate, sex code, category code): value} of one survey table."""
    out = {}
    for r in to_rows(doc):
        if r.get("sizeclas", SURVEY_SIZE) == SURVEY_SIZE:
            out[(r["nace_r2"], r["sex"], r[dim])] = float(r["value"])
    return out


def mean_of(sections: str, sex: str, code: str, earnings: dict, employees: dict) -> float | None:
    """Employee-weighted mean earnings of a category over the sections that publish it."""
    num = den = 0.0
    for section in sections:
        e, n = earnings.get((section, sex, code)), employees.get((section, sex, code))
        if e is not None and n:
            num += e * n
            den += n
    return num / den if den else None


def relativities(earn_doc: dict, emp_doc: dict, dim: str, codes: list[str]) -> list:
    """[industry group][sex][category]: mean of the cell over the mean of the group, in millionths."""
    earnings, employees = survey_cells(earn_doc, dim), survey_cells(emp_doc, dim)
    overall = earnings.get((ALL, "T", "TOTAL"))
    if not overall:
        raise CensusError(f"earnings by {dim}: the survey total is missing")

    def whole_survey(sex: str, code: str) -> float:
        """The proportion in the whole survey; for both sexes if not published by sex; else 1."""
        cell = earnings.get((ALL, sex, code))
        if cell is not None:
            return cell / overall
        both, of_sex = earnings.get((ALL, "T", code)), earnings.get((ALL, sex, "TOTAL"))
        return (both / overall) * (of_sex / overall) if both is not None and of_sex is not None else 1.0

    table = []
    for sections in GROUP_SECTIONS:
        group_mean = mean_of(sections, "T", "TOTAL", earnings, employees)
        by_sex = []
        for sex in SEXES:
            row = []
            for code in codes:
                cell = mean_of(sections, sex, code, earnings, employees) if group_mean else None
                row.append(round((cell / group_mean if cell is not None else whole_survey(sex, code)) * UNIT))
            by_sex.append(row)
        table.append(by_sex)
    return table


def quantile_curve(doc: dict, overall_mean: float, minimum_wage_at_survey: int) -> dict:
    """Earnings at a rank over mean earnings, as a line through knots (both in millionths).

    Published: the first decile, the median and the ninth decile. The bottom
    of the curve is the minimum wage at the time of the survey. Above the ninth
    decile the curve follows a Pareto tail whose mean makes the whole curve
    average 1: the survey's mean is known, the shape of its top is not.
    """
    q = {r["quant_inc"]: float(r["value"]) for r in to_rows(doc) if r["worktime"] == "TOTAL"}
    try:
        p10, med, p90 = q["P10"] / overall_mean, q["MED"] / overall_mean, q["P90"] / overall_mean
    except KeyError as e:
        raise CensusError(f"earnings quantiles: {e} is missing") from e
    bottom = min(minimum_wage_at_survey / overall_mean, p10)
    below = (bottom + p10) / 2 * 0.1 + (p10 + med) / 2 * 0.4 + (med + p90) / 2 * 0.4
    top_mean = (1.0 - below) / 0.1                     # mean of the top tenth, for the curve to average 1
    if top_mean <= p90:
        raise CensusError("earnings quantiles: the published mean is below what the deciles imply")
    alpha = top_mean / (top_mean - p90)                # Pareto: mean above x is x * alpha / (alpha - 1)
    ranks = [0, 100_000, 500_000, 900_000]
    values = [bottom, p10, med, p90]
    for rank in TAIL_RANKS:
        at = min(rank / UNIT, TAIL_CAP_RANK)
        ranks.append(rank)
        values.append(p90 * (0.1 / (1.0 - at)) ** (1.0 / alpha))
    return {"rank": ranks, "value": [round(v * UNIT) for v in values]}


def wage_bill(doc: dict) -> list[int]:
    """Lei per month for each industry group: the quarter's wages and salaries (million lei) / 3."""
    cells = {r["nace_r2"]: r["value"] for r in to_rows(doc)
             if r["na_item"] == "D11" and r["time"] == QUARTER and r["s_adj"] == "NSA" and r["unit"] == "CP_MNAC"}
    out = []
    for code in GROUP_CODES:
        if code not in cells:
            raise CensusError(f"national accounts: no wages and salaries for {code} in {QUARTER}")
        tenths_of_million = round(float(cells[code]) * 10)   # published with one decimal
        out.append((tenths_of_million * 100_000 + 1) // 3)
    return out


def minimum_wages(doc: dict) -> dict:
    cells = {r["time"]: int(r["value"]) for r in to_rows(doc) if r["currency"] == "NAC"}
    if START_HALF_YEAR not in cells or SURVEY_HALF_YEAR not in cells:
        raise CensusError("minimum wage: a half-year is missing")
    return cells


def build(docs: dict) -> dict:
    m = {
        "industry_groups": INDUSTRY_GROUPS,
        "sexes": SEXES,
        "occupations": OCCUPATIONS,
        "age_group_from": AGE_GROUP_FROM,
        "edu_groups": EDU_GROUPS,
        "edu_group_of_level": EDU_GROUP_OF_LEVEL,
        "wage_bill": wage_bill(docs[WAGES_ID]),
    }
    minimum = minimum_wages(docs[MINIMUM_WAGE_ID])
    m["minimum_wage"] = minimum[START_HALF_YEAR]
    occupation = TABLES["rel_occupation"]
    m["rel_sex"] = [[row[0] for row in group]
                    for group in relativities(docs[occupation[0]], docs[occupation[1]], occupation[2], ["TOTAL"])]
    for name, (earn_id, emp_id, dim, codes) in TABLES.items():
        m[name] = relativities(docs[earn_id], docs[emp_id], dim, codes)
    overall = survey_cells(docs[TABLES["rel_occupation"][0]], "isco08")[(ALL, "T", "TOTAL")]
    m["quantile_curve"] = quantile_curve(docs[QUANTILES_ID], overall, minimum[SURVEY_HALF_YEAR])
    return m


def dataset_ids() -> list[str]:
    ids = [ds for earn_id, emp_id, _, _ in TABLES.values() for ds in (earn_id, emp_id)]
    return [*ids, QUANTILES_ID, WAGES_ID, MINIMUM_WAGE_ID]


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--raw", type=Path, default=RAW)
    ap.add_argument("--out", type=Path, default=HERE / "data" / "derived" / "earnings2021_ro.json")
    a = ap.parse_args()
    docs, prov = {}, []
    for ds in dataset_ids():
        raw = (a.raw / f"{ds}.json").read_bytes()
        docs[ds] = json.loads(raw)
        side = a.raw / f"{ds}.provenance.json"
        p = json.loads(side.read_text(encoding="utf-8")) if side.exists() else {"id": ds}
        p["sha256"] = hashlib.sha256(raw).hexdigest()
        prov.append(p)
    m = build(docs)
    m["totals"] = {"wage_bill_per_month": sum(m["wage_bill"])}
    m["provenance"] = {"sources": prov,
                       "note": "Eurostat: national accounts 2021-Q4 (levels), Structure of Earnings Survey 2022 "
                               "(proportions; enterprises with 10 employees or more, NACE B to S), minimum wage; "
                               "unpublished cells filled by normalise_earnings.py"}
    a.out.parent.mkdir(parents=True, exist_ok=True)
    a.out.write_text(json.dumps(m, separators=(",", ":")) + "\n", encoding="utf-8")
    print(f"OK - wage bill {m['totals']['wage_bill_per_month']:,} lei a month, minimum wage {m['minimum_wage']} -> {a.out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
