#!/usr/bin/env python3
"""Assemble the starting state from the committed fixtures and report whether its parts agree.

    python python/pipeline/build_scenario.py --sample-scale 1000 [--out DIR]

**Interim.** ADR-0018 makes the scenario builder a Rust command (`econ-cli
build-scenario`) that writes Arrow tables. The stages after housing exist only
as Python references until their acceptance tests are written, so this script
runs the references instead. Its lasting part is the consistency report: the
checks the Rust builder must also pass.

With `--out` it writes `scenario.json` (start date, scale, seed, the hash of
every stage and of every input file) and `report.json`. Without, it prints the
report. It exits with 1 if a check fails, except the checks listed in
`KNOWN_GAPS`, which are reported with the reason they are expected to fail.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent / "reference"))
import opening_balance_reference as obs  # noqa: E402
import firm_reference as fref  # noqa: E402
import popgen_reference as ref  # noqa: E402

FIXTURES = HERE / "fixtures"
START_DATE = "2021-12-01"
INPUTS = {
    "margins": "census2021_margins_ro.json", "attributes": "census2021_edu_activity_ro.json",
    "jobs": "census2021_jobs_ro.json", "housing": "census2021_housing_ro.json", "earnings": "earnings2021_ro.json",
    "pensions": "pensions2021_ro.json", "incomes": "other_incomes2021_ro.json", "wealth": "household_wealth2021_ro.json",
    "io": "io_table2021_ro.json", "firms": "firm_units2021_ro.json", "sheets": "balance_sheets2021q4_ro.json",
    "firm_sheets": "firm_balance_sheets2021_ro.json", "details": "sector_details2021_ro.json",
    "firm_extras": "firm_undeclared_inventories2021_ro.json",
}
# Mean disposable income of households by income fifth, Romania 2020, euro a year
# (Eurostat experimental statistics icw_res_02, read 2026-10-10). A check, not an input.
INCOME_BY_FIFTH_2020 = [2483.3, 5438.3, 7764.3, 11155.5, 18997.8]
# Checks that are expected to fail for now, with the reason. They are reported and do not fail the build.
KNOWN_GAPS = {
    "income_distribution": "Benefits other than pensions (child allowance, unemployment benefit, minimum income) and "
                           "taxes are not applied before the simulation runs, so the poorest fifth has under half the "
                           "share the survey gives it and 6% of households have no income at all. Re-check after the "
                           "tax and transfer mechanics exist (roadmap M3); if it still fails, informal income is too "
                           "concentrated (14% of adults share it).",
}
BANI = 100


def load(fixtures: Path) -> tuple[dict, dict]:
    docs, hashes = {}, {}
    for key, name in INPUTS.items():
        raw = (fixtures / name).read_bytes()
        docs[key] = json.loads(raw)
        hashes[name] = hashlib.sha256(raw).hexdigest()
    return docs, hashes


def run_stages(d: dict, sample_scale: int, seed: int) -> dict:
    """Every stage of the generator, in order. Returns the population and each stage's columns."""
    pop = ref.generate(d["margins"], sample_scale, seed)
    counties = d["margins"]["counties"]
    attrs = ref.assign_attributes(pop, counties, d["attributes"], seed)
    jobs = ref.assign_jobs(pop, counties, d["attributes"], d["jobs"], attrs, seed)
    housing = ref.assign_housing(pop, counties, d["housing"], seed)
    housing["urban_from_class"] = d["housing"]["urban_from_class"]
    wage = ref.assign_wages(pop, d["earnings"], attrs, jobs, seed)
    pension = ref.assign_pensions(pop, d["pensions"], attrs, seed)
    incomes = ref.assign_other_incomes(pop, d["incomes"], attrs, jobs, housing, wage, seed)
    wealth = ref.assign_wealth(pop, d["wealth"], housing, wage, pension, seed)
    employers = ref.assign_employers(pop, d["firms"], jobs, wage, seed)
    firm_extras = fref.undeclared_and_inventories(d["firms"], d["firm_extras"])
    return {"pop": pop, "attrs": attrs, "jobs": jobs, "housing": housing, "wage": wage, "pension": pension,
            "incomes": incomes, "wealth": wealth, "employers": employers, "firm_extras": firm_extras}


def stage_hashes(s: dict) -> dict:
    housing = {k: v for k, v in s["housing"].items() if k != "urban_from_class"}
    return {
        "population": f"{ref.state_hash(s['pop']):016x}", "attributes": f"{ref.attributes_hash(s['attrs']):016x}",
        "jobs": f"{ref.jobs_hash(s['jobs']):016x}", "housing": f"{ref.housing_hash(housing):016x}",
        "wages": f"{ref.wages_hash(s['wage']):016x}", "pensions": f"{ref.pensions_hash(s['pension']):016x}",
        "incomes": f"{ref.incomes_hash(s['incomes']):016x}", "wealth": f"{ref.wealth_hash(s['wealth']):016x}",
        "employers": f"{ref.employers_hash(s['employers']):016x}",
        "firm_undeclared_inventories": f"{fref.columns_hash(s['firm_extras']):016x}",
    }


def report(d: dict, s: dict, sample_scale: int) -> dict:
    """Checks across the parts. Each has `ok`; amounts are billion lei unless said otherwise."""
    pop = s["pop"]
    hh_w, hh_of = pop["hh_weight"], pop["household_id"]
    pw = [hh_w[h] for h in hh_of]
    n, n_hh = len(pw), len(hh_w)
    bn = lambda bani: round(bani / BANI / 1e9, 2)  # noqa: E731
    checks: dict[str, dict] = {}

    def check(name: str, ok: bool, **facts) -> None:
        checks[name] = {"ok": bool(ok), **facts}

    # 1. People and households are the census's.
    persons = sum(pw)
    check("persons", persons == 19_053_815, weighted_persons=persons, synthetic_persons=n, synthetic_households=n_hh)

    # 2. Monthly incomes of households against the national accounts.
    wages = sum(w * v for w, v in zip(pw, s["wage"]))
    pensions = sum(w * v for w, v in zip(pw, s["pension"]))
    inc = s["incomes"]
    self_emp = sum(w * v for w, v in zip(pw, inc["self_employment_income"]))
    informal = sum(w * v for w, v in zip(pw, inc["informal_income"]))
    in_kind = sum(hh_w[h] * inc["hh_subsistence_income"][h] for h in range(n_hh))
    want_wages = sum(d["earnings"]["wage_bill"]) * BANI
    check("wage_bill", abs(wages - want_wages) <= 1e-4 * want_wages, got=bn(wages), want=bn(want_wages))
    check("pension_bill", abs(pensions - d["pensions"]["pension_bill"] * BANI) <= 1e-4 * pensions,
          got=bn(pensions), want=bn(d["pensions"]["pension_bill"] * BANI))
    mixed = self_emp + informal + in_kind
    check("mixed_income", abs(mixed - d["incomes"]["mixed_income"] * BANI) <= 1e-4 * mixed,
          got=bn(mixed), want=bn(d["incomes"]["mixed_income"] * BANI))
    declared_wages = sum(w * v for w, v in zip(pw, inc["wage_declared"]))
    income_month = wages + pensions + mixed

    # 3. The wage bill of the population against the input-output table's compensation, by industry group.
    #    The table is the year 2021 and includes employers' contributions; the population is a month of 2021-Q4.
    io = d["io"]
    comp = [0] * len(io["industry_groups"])
    for j, g in enumerate(io["industry_group_of"]):
        comp[g] += io["compensation"][j]
    ratios = []
    for g, name in enumerate(io["industry_groups"]):
        paid = sum(w * v for w, v, st, gr in zip(pw, s["wage"], s["jobs"]["employment_status"], s["jobs"]["industry_group"])
                   if st == ref.KIND_EMPLOYEE and gr == g)
        ratios.append(round(paid * 12 / BANI / comp[g], 3))
    check("wages_against_input_output_table", all(0.85 <= r <= 1.25 for r in ratios),
          year_of_population_wages_over_table_compensation=dict(zip(io["industry_groups"], ratios)),
          note="1.0 would mean a month of 2021-Q4 times twelve equals the year 2021 including employers' contributions")

    # 4. Household wealth against the sector balance sheets.
    sheets, wealth = d["sheets"], s["wealth"]
    total = lambda col: sum(hh_w[h] * wealth[col][h] for h in range(n_hh))  # noqa: E731
    pairs = {"hh_cash": sheets["assets"]["currency"]["households"], "hh_deposits": sheets["assets"]["deposits"]["households"]}
    loans = total("hh_mortgage") + total("hh_consumer_debt")
    ok = all(abs(total(c) - want * BANI) <= 1e-4 * want * BANI for c, want in pairs.items())
    ok = ok and abs(loans - sheets["liabilities"]["loans"]["households"] * BANI) <= 1e-4 * loans
    check("household_balance_sheet", ok, cash=bn(total("hh_cash")), deposits=bn(total("hh_deposits")), loans=bn(loans),
          dwellings=bn(total("hh_dwelling_value")))

    # 5. Deposits: what households and firms hold against what banks owe (the rest is other sectors').
    held = sheets["assets"]["deposits"]["households"] + sheets["assets"]["deposits"]["firms"]
    owed = sheets["liabilities"]["deposits"]["banks"]
    firm_deposits = sum(u["deposits"] for u in d["firm_sheets"]["units"])
    check("deposits", firm_deposits == sheets["assets"]["deposits"]["firms"] and held <= owed,
          households_and_firms=round(held / 1e9, 1), owed_by_banks=round(owed / 1e9, 1))

    # 6. Every employed person has an employer of their industry group; units' employees against the firm table.
    emp, units = s["employers"], d["firms"]["units"]
    group_of = {ind["id"]: ind["industry_group"] for ind in d["firms"]["industries"]}
    placed = all((e == ref.NO_EMPLOYER) == (st == ref.KIND_NONE) and
                 (e == ref.NO_EMPLOYER or group_of[units[e]["industry"]] == g)
                 for e, st, g in zip(emp["employer"], s["jobs"]["employment_status"], s["jobs"]["industry_group"]))
    staffed = len({e for e in emp["employer"] if e != ref.NO_EMPLOYER})
    employees = sum(w for w, st in zip(pw, s["jobs"]["employment_status"]) if st == ref.KIND_EMPLOYEE)
    check("employers", placed, units_with_workers=staffed, units=len(units), employees=employees,
          persons_employed_in_firm_table=d["firms"]["totals"]["persons_employed"])

    # 7. Firm units against the input-output table.
    check("firm_units_against_input_output_table",
          abs(d["firms"]["totals"]["output"] - io["totals"]["output"]) <= 1e-4 * io["totals"]["output"],
          output=round(d["firms"]["totals"]["output"] / 1e9, 1), table=round(io["totals"]["output"] / 1e9, 1))

    # 8. What households earn in a year against what they consume in the table (before taxes and saving).
    consumption = io["totals"]["household_consumption"]
    ratio = income_month * 12 / BANI / consumption
    check("income_against_consumption", 0.8 <= ratio <= 1.4, income_a_year=bn(income_month * 12),
          consumption_in_table=round(consumption / 1e9, 1), ratio=round(ratio, 3),
          note="gross money and in-kind income of a month of 2021-Q4, times twelve, over household consumption of 2021")

    # 9. The distribution of household income by fifth of households, against the survey's shape (2020, disposable).
    income = [0] * n_hh
    for i, h in enumerate(hh_of):
        income[h] += s["wage"][i] + s["pension"][i] + inc["self_employment_income"][i] + inc["informal_income"][i]
    for h in range(n_hh):
        income[h] += inc["hh_subsistence_income"][h]
    private = [h for h in range(n_hh) if not pop["hh_collective"][h]]
    shares = ref.quintile_shares([(income[h], hh_w[h], income[h]) for h in private])
    survey = [v / sum(INCOME_BY_FIFTH_2020) for v in INCOME_BY_FIFTH_2020]
    check("income_distribution", all(abs(a - b) <= 0.08 for a, b in zip(shares, survey)),
          share_of_income_by_fifth=shares, survey_2020_disposable=[round(v, 4) for v in survey],
          households_without_income=round(sum(hh_w[h] for h in private if income[h] == 0)
                                          / sum(hh_w[h] for h in private), 4),
          note="gross income here, disposable income in the survey: taxes and benefits are not applied yet")

    # 10. The declared economy.
    check("declared_wages", 0.8 <= declared_wages / wages <= 0.97, declared_share=round(declared_wages / wages, 4))

    # 11 to 14. The opening ledger (spec economy/opening-balance-sheet) and the tables against it.
    opening = obs.reconcile(obs.opening_balances(sheets), {"cash": total("hh_cash"), "deposits": total("hh_deposits")})
    b = opening["balances"]
    rows = {k: sum(row.values()) for k, row in b.items()}
    check("ledger_invariants", all(v == 0 for v in rows.values()), rows_not_zero={k: v for k, v in rows.items() if v})
    check("households_against_ledger",
          total("hh_cash") == b["cash"]["households"] and total("hh_deposits") == b["deposits"]["households"],
          cash=bn(b["cash"]["households"]), deposits=bn(b["deposits"]["households"]))
    firm_cash = sum(u["currency"] for u in d["firm_sheets"]["units"]) * BANI
    check("firms_against_ledger",
          firm_cash == b["cash"]["firms"] and firm_deposits * BANI == b["deposits"]["firms"],
          cash=bn(b["cash"]["firms"]), deposits=bn(b["deposits"]["firms"]))
    check("left_out", True, **{k: {s: bn(v) for s, v in row.items()} for k, row in opening["left_out"].items()})
    # 15. Undeclared sales and inventories of the firm units: the committed columns are the reference's, the
    #     totals are the anchor's and the measured inventories (spec economy/firm-units-undeclared-inventories).
    extras, cols = d["firm_extras"], s["firm_extras"]
    fit = fref.fit(d["firms"], extras, cols)
    committed = len(extras["units"]) == len(units) and all(
        row["id"] == units[f]["id"] and all(row[k] == cols[k][f] for k in fref.COLUMNS)
        for f, row in enumerate(extras["units"]))
    check("firm_undeclared_and_inventories",
          committed and fit["undeclared_value_added_total"] == fit["undeclared_value_added_target"]
          and fit["inventories_total"] == fit["inventories_target"],
          undeclared_value_added=round(fit["undeclared_value_added_total"] / 1e9, 1),
          undeclared_sales=fit["billion_lei"]["undeclared_sales"], inventories=round(fit["inventories_total"] / 1e9, 1),
          vat_gap_proxy=fit["vat_gap_proxy"])

    # 16. The undeclared wages of each class of firm unit against those of the population (Stages I and J).
    population = fref.population_wage_shares(d["firms"], emp["employer"], s["wage"], inc["wage_declared"], pw)
    gaps = {c: round(population[c] - fit["by_class"][c]["undeclared_wage_share"], 4) for c in fref.CLASSES
            if c in population}
    check("firm_undeclared_wages_against_population", all(abs(g) <= 0.02 for g in gaps.values()),
          population=population, gap=gaps)
    for name, reason in KNOWN_GAPS.items():
        checks[name]["known_gap"] = reason
    return {"sample_scale": sample_scale, "opening": b, "ok": all(c["ok"] for name, c in checks.items() if name not in KNOWN_GAPS),
            "checks": checks,
            "household_income_a_month": {"wages": bn(wages), "pensions": bn(pensions), "self_employment": bn(self_emp),
                                         "informal": bn(informal), "grown_at_home": bn(in_kind)}}


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--sample-scale", type=int, required=True)
    ap.add_argument("--seed", type=int, default=42)
    ap.add_argument("--fixtures", type=Path, default=FIXTURES)
    ap.add_argument("--out", type=Path)
    a = ap.parse_args()
    ref.check_rng()
    docs, input_hashes = load(a.fixtures)
    stages = run_stages(docs, a.sample_scale, a.seed)
    rep = report(docs, stages, a.sample_scale)
    scenario = {"format": 0, "name": "Romania 2021 (interim, from Python references)", "start_date": START_DATE,
                "sample_scale": a.sample_scale, "rng_seed": a.seed, "stage_hashes": stage_hashes(stages),
                "opening_hash": f"{obs.opening_hash(rep['opening']):016x}",
                "inputs": input_hashes, "consistent": rep["ok"]}
    rep.pop("opening")
    if a.out:
        a.out.mkdir(parents=True, exist_ok=True)
        (a.out / "scenario.json").write_text(json.dumps(scenario, indent=2) + "\n", encoding="utf-8")
        (a.out / "report.json").write_text(json.dumps(rep, indent=2) + "\n", encoding="utf-8")
        print(f"{'OK' if rep['ok'] else 'FAILED'} - wrote {a.out / 'scenario.json'} and report.json")
    else:
        print(json.dumps({"scenario": scenario, "report": rep}, indent=2))
    return 0 if rep["ok"] else 1


if __name__ == "__main__":
    sys.exit(main())
