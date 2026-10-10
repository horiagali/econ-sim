"""Wages: earnings normalisation and the reference stage (spec society/population-wages)."""
import json
import sys
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(HERE.parent / "reference"))
sys.path.insert(0, str(Path(__file__).resolve().parent))

import normalise_census as nc  # noqa: E402
import normalise_census_jobs as nj  # noqa: E402
import normalise_earnings as ne  # noqa: E402
import popgen_reference as ref  # noqa: E402
from test_population import jsonstat  # noqa: E402

F = HERE / "fixtures"
FIXTURE = F / "earnings2021_ro.json"
SECTIONS = [s for group in nj.GROUP_SECTIONS for s in group if s not in "ATU"]   # the survey has B to S
UNIT = 1_000_000


def survey_docs(dim: str, codes: list[str], drop=()):
    """Earnings and employees by section, sex and category.

    Every section has 100 women and 100 men per category. Earnings are 1000
    for the first category and 2000 for the others; men earn 10% more; section
    C pays double. `drop` removes cells from the earnings table.
    """
    dims = {"sizeclas": ["GE10"], "nace_r2": [ne.ALL, *SECTIONS], "sex": ["T", "F", "M"], dim: ["TOTAL", *codes]}
    earn, emp = {}, {}
    for section in dims["nace_r2"]:
        scale = 2 if section == "C" else 1
        for sex, lift in (("T", 1.05), ("F", 1.0), ("M", 1.1)):
            cells = [1000 if code == codes[0] else 2000 for code in codes]
            for code, value in zip(codes, cells):
                earn[("GE10", section, sex, code)] = value * lift * scale
                emp[("GE10", section, sex, code)] = 100 if sex != "T" else 200
            earn[("GE10", section, sex, "TOTAL")] = sum(cells) / len(cells) * lift * scale
            emp[("GE10", section, sex, "TOTAL")] = (100 if sex != "T" else 200) * len(codes)
    for key in drop:
        del earn[key]
    return jsonstat(dims, earn), jsonstat(dims, emp)


def small_docs(drop=()):
    docs = {}
    for earn_id, emp_id, dim, codes in ne.TABLES.values():
        docs[earn_id], docs[emp_id] = survey_docs(dim, codes, drop if dim == "isco08" else ())
    docs[ne.QUANTILES_ID] = jsonstat(
        {"worktime": ["TOTAL", "FT"], "quant_inc": ["P10", "MED", "P90"]},
        {("TOTAL", "P10"): 900, ("TOTAL", "MED"): 1600, ("TOTAL", "P90"): 3200, ("FT", "MED"): 1})
    docs[ne.WAGES_ID] = jsonstat(
        {"na_item": ["D11", "D1"], "time": [ne.QUARTER], "s_adj": ["NSA", "SCA"], "unit": ["CP_MNAC"],
         "nace_r2": ["TOTAL", *nj.GROUP_CODES]},
        {("D11", ne.QUARTER, "NSA", "CP_MNAC", code): 3.0 * (k + 1) for k, code in enumerate(nj.GROUP_CODES)})
    docs[ne.MINIMUM_WAGE_ID] = jsonstat(
        {"currency": ["NAC", "EUR"], "time": [ne.START_HALF_YEAR, ne.SURVEY_HALF_YEAR]},
        {("NAC", ne.START_HALF_YEAR): 700, ("NAC", ne.SURVEY_HALF_YEAR): 800, ("EUR", ne.START_HALF_YEAR): 1})
    return docs


class TestNormaliseEarnings(unittest.TestCase):
    def test_levels_come_from_the_national_accounts(self):
        m = ne.build(small_docs())
        # 3 million lei a quarter is 1 million a month.
        self.assertEqual(m["wage_bill"], [1_000_000 * (k + 1) for k in range(10)])
        self.assertEqual(m["minimum_wage"], 700)

    def test_proportions_are_relative_to_the_group_and_weighted_by_employees(self):
        m = ne.build(small_docs())
        # Women earn 1/1.05 of the group's mean, men 1.1/1.05, in every group.
        self.assertEqual(m["rel_sex"][1], [round(UNIT / 1.05), round(UNIT * 1.1 / 1.05)])
        # Occupations: the first earns 1000, the other nine 2000, so the mean is 1900 (x 1.05 for both sexes).
        self.assertEqual(m["rel_occupation"][2][0][:2], [round(UNIT * 1000 / 1995), round(UNIT * 2000 / 1995)])
        # Industry is B, C, D and E; C pays double, so the group's level is 1.25 times a section's:
        # the proportions inside the group are unchanged.
        self.assertEqual(m["rel_occupation"][1], m["rel_occupation"][2])
        self.assertEqual(len(m["rel_age"][0][0]), 3)
        self.assertEqual(len(m["rel_education"][0][1]), 3)

    def test_agriculture_and_unpublished_cells_use_the_whole_survey(self):
        gone = [("GE10", "F", "F", "OC1"), ("GE10", "F", "M", "OC1")]       # construction, managers
        m = ne.build(small_docs(drop=gone))
        whole = [round(UNIT * 1000 / 1995)] + [round(UNIT * 2000 / 1995)] * 9
        self.assertEqual(m["rel_occupation"][0][0], whole)                  # agriculture is not surveyed
        self.assertEqual(m["rel_occupation"][2][0][1], whole[1])            # the dropped cell
        self.assertEqual(m["rel_occupation"][2][1][1], round(UNIT * 2000 * 1.1 / 1995))

    def test_quantile_curve_runs_from_the_minimum_wage_through_the_deciles(self):
        curve = ne.build(small_docs())["quantile_curve"]
        mean = 1995
        self.assertEqual(curve["rank"][:4], [0, 100_000, 500_000, 900_000])
        self.assertEqual(curve["rank"][-1], UNIT)
        self.assertEqual(curve["value"][:4], [round(UNIT * v / mean) for v in (800, 900, 1600, 3200)])
        self.assertEqual(curve["value"], sorted(curve["value"]))
        # The line through the knots averages close to 1 (the Pareto tail is drawn with straight pieces).
        area = sum((b - a) * (va + vb) / 2 for a, b, va, vb in
                   zip(curve["rank"], curve["rank"][1:], curve["value"], curve["value"][1:])) / UNIT / UNIT
        self.assertAlmostEqual(area, 1.0, delta=0.03)

    def test_missing_inputs_are_rejected(self):
        docs = small_docs()
        docs[ne.WAGES_ID] = jsonstat({"na_item": ["D11"], "time": [ne.QUARTER], "s_adj": ["NSA"], "unit": ["CP_MNAC"],
                                      "nace_r2": ["A"]}, {("D11", ne.QUARTER, "NSA", "CP_MNAC", "A"): 1.0})
        with self.assertRaises(nc.CensusError):
            ne.build(docs)
        docs = small_docs()
        docs[ne.QUANTILES_ID] = jsonstat({"worktime": ["TOTAL"], "quant_inc": ["MED"]}, {("TOTAL", "MED"): 1600})
        with self.assertRaises(nc.CensusError):
            ne.build(docs)

    def test_committed_fixture_is_what_the_published_tables_say(self):
        m = json.loads(FIXTURE.read_text(encoding="utf-8"))
        self.assertEqual(m["industry_groups"], nj.INDUSTRY_GROUPS)
        self.assertEqual(m["minimum_wage"], 2300)
        # Wages and salaries, 2021-Q4: 111,684.5 million lei (the groups sum to it up to rounding).
        self.assertLessEqual(abs(m["totals"]["wage_bill_per_month"] * 3 - 111_684_500_000), 2_000_000)
        self.assertEqual(sum(m["wage_bill"]), m["totals"]["wage_bill_per_month"])
        # Information and communication pays best; managers earn more than elementary occupations everywhere.
        self.assertTrue(all(row[1] > row[9] for group in m["rel_occupation"] for row in group))
        self.assertTrue(all(row[2] > row[0] for group in m["rel_education"] for row in group))
        ref.check_wage_margins(m)


class TestWageReference(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.m = json.loads((F / "census2021_margins_ro.json").read_text(encoding="utf-8"))
        cls.ea = json.loads((F / "census2021_edu_activity_ro.json").read_text(encoding="utf-8"))
        cls.jobs = json.loads((F / "census2021_jobs_ro.json").read_text(encoding="utf-8"))
        cls.earn = json.loads(FIXTURE.read_text(encoding="utf-8"))
        cls.scale = 1000
        cls.pop = ref.generate(cls.m, cls.scale, 42)
        cls.attrs = ref.assign_attributes(cls.pop, cls.m["counties"], cls.ea, 42)
        cls.ja = ref.assign_jobs(cls.pop, cls.m["counties"], cls.ea, cls.jobs, cls.attrs, 42)
        cls.wage = ref.assign_wages(cls.pop, cls.earn, cls.attrs, cls.ja, 42)

    def test_curve_is_a_line_through_its_knots(self):
        curve = {"rank": [0, 500_000, UNIT], "value": [400_000, 800_000, 2_000_000]}
        self.assertEqual(ref.curve_at(curve, 0), 400_000)
        self.assertEqual(ref.curve_at(curve, 250_000), 600_000)
        self.assertEqual(ref.curve_at(curve, 500_000), 800_000)
        self.assertEqual(ref.curve_at(curve, 999_999), 800_000 + 1_200_000 * 499_999 // 500_000)

    def test_only_employees_have_a_wage_and_none_is_below_the_floor(self):
        status = self.ja["employment_status"]
        self.assertTrue(all((w > 0) == (k == ref.KIND_EMPLOYEE) for w, k in zip(self.wage, status)))
        minimum = self.earn["minimum_wage"] * 100
        outside_agriculture = [w for w, k, g in zip(self.wage, status, self.ja["industry_group"])
                               if k == ref.KIND_EMPLOYEE and g != 0]
        self.assertEqual(min(outside_agriculture), minimum)

    def test_every_group_pays_its_wage_bill(self):
        fit = ref.wage_fit(self.pop, self.earn, self.attrs, self.ja, self.wage, self.scale)
        self.assertLess(fit["wage_bill"]["largest relative error"], 1e-5)
        self.assertEqual(fit["groups_with_lowered_floor"], ["agriculture"])
        for name, by_bucket in fit["proportions"].items():
            self.assertLess(by_bucket.get(">=100 records", 0), 0.06, name)
        national = fit["national"]
        self.assertAlmostEqual(national["p10_over_median"], national["survey_p10_over_median"], delta=0.06)
        self.assertAlmostEqual(national["p90_over_median"], national["survey_p90_over_median"], delta=0.25)

    def test_same_seed_same_wages_and_another_seed_moves_them(self):
        again = ref.assign_wages(self.pop, self.earn, self.attrs, self.ja, 42)
        self.assertEqual(ref.wages_hash(again), ref.wages_hash(self.wage))
        other = ref.assign_wages(self.pop, self.earn, self.attrs, self.ja, 7)
        self.assertNotEqual(ref.wages_hash(other), ref.wages_hash(self.wage))
        fit = ref.wage_fit(self.pop, self.earn, self.attrs, self.ja, other, self.scale)
        self.assertLess(fit["wage_bill"]["largest relative error"], 1e-5)

    def test_hash_is_the_one_of_the_spec(self):
        self.assertEqual(f"{ref.wages_hash(self.wage):016x}", "e5c524e4e785f24f")

    def test_bad_inputs_are_rejected(self):
        short = dict(self.earn, wage_bill=self.earn["wage_bill"][:-1])
        with self.assertRaisesRegex(ref.GenError, "ShapeMismatch"):
            ref.assign_wages(self.pop, short, self.attrs, self.ja, 42)
        fewer = {k: v[:-1] for k, v in self.ja.items()}
        with self.assertRaisesRegex(ref.GenError, "ShapeMismatch"):
            ref.assign_wages(self.pop, self.earn, self.attrs, fewer, 42)
        unpaid = dict(self.earn, wage_bill=[0] + self.earn["wage_bill"][1:])
        with self.assertRaisesRegex(ref.GenError, "NoMargin: agriculture"):
            ref.assign_wages(self.pop, unpaid, self.attrs, self.ja, 42)
        empty = dict(self.earn, industry_groups=[], wage_bill=[], rel_sex=[], rel_occupation=[], rel_age=[],
                     rel_education=[])
        with self.assertRaisesRegex(ref.GenError, "EmptyTable"):
            ref.assign_wages(self.pop, empty, self.attrs, self.ja, 42)


if __name__ == "__main__":
    unittest.main()
