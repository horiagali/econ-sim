"""Income split: normalisation and the reference stage (spec society/population-incomes)."""
import json
import sys
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(HERE.parent / "reference"))
sys.path.insert(0, str(Path(__file__).resolve().parent))

import normalise_census as nc  # noqa: E402
import normalise_incomes as ni  # noqa: E402
import popgen_reference as ref  # noqa: E402
from test_population import jsonstat  # noqa: E402

F = HERE / "fixtures"
FIXTURE = F / "other_incomes2021_ro.json"
UNIT = 1_000_000
EARNINGS = {"minimum_wage": 2300, "quantile_curve": {"rank": [0, UNIT], "value": [500_000, 1_500_000]}}


def small_docs(quarter=300.0, year=1000.0, value_added=350.0, compensation=100.0):
    return {
        ni.QUARTER_ID: jsonstat({"na_item": ["B3G"]}, {("B3G",): quarter}),
        ni.YEAR_ID: jsonstat({"na_item": ["B3G"]}, {("B3G",): year}),
        ni.AGRICULTURE_ID: jsonstat({"na_item": ["B1G", "D1"]}, {("B1G",): value_added, ("D1",): compensation}),
    }


class TestNormaliseIncomes(unittest.TestCase):
    def test_mixed_income_is_split_in_three_that_sum_to_it(self):
        m = ni.build(small_docs(), EARNINGS)
        # 300 million lei a quarter is 100 million a month.
        self.assertEqual(m["mixed_income"], 100_000_000)
        # Farming: (350 - 100) x 0.8 of the year's 1,000, so a fifth.
        self.assertEqual(m["farm_income"], 20_000_000)
        self.assertEqual(m["farm_in_kind"], 12_000_000)
        self.assertEqual(m["self_employment_income"], 22_000_000)
        self.assertEqual(m["farm_income"] + m["self_employment_income"] + m["informal_income"], m["mixed_income"])
        self.assertEqual(m["minimum_wage"], 2300)
        self.assertEqual(len(m["undeclared_wage_share"]), 10)
        self.assertEqual(m["undeclared_wage_share"][8], 0)                    # the public sector declares everything
        ref.check_income_margins(m)

    def test_farming_that_leaves_nothing_for_the_rest_is_rejected(self):
        with self.assertRaises(nc.CensusError):
            ni.build(small_docs(value_added=1300.0), EARNINGS)
        with self.assertRaises(nc.CensusError):
            ni.build({**small_docs(), ni.YEAR_ID: jsonstat({"na_item": ["D1"]}, {("D1",): 1.0})}, EARNINGS)

    def test_committed_fixture_is_what_the_published_tables_say(self):
        m = json.loads(FIXTURE.read_text(encoding="utf-8"))
        # Mixed income of households, 2021-Q4: 46,066 million lei.
        self.assertEqual(m["mixed_income"], round(46_066_000_000 / 3))
        self.assertEqual(m["farm_income"] + m["self_employment_income"] + m["informal_income"], m["mixed_income"])
        self.assertGreater(m["informal_income"], m["self_employment_income"])
        self.assertIn("guesses", m["provenance"])
        ref.check_income_margins(m)


class TestIncomeReference(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        load = lambda name: json.loads((F / name).read_text(encoding="utf-8"))  # noqa: E731
        cls.m, ea, jobs = load("census2021_margins_ro.json"), load("census2021_edu_activity_ro.json"), load("census2021_jobs_ro.json")
        housing, cls.inc = load("census2021_housing_ro.json"), load("other_incomes2021_ro.json")
        cls.pop = ref.generate(cls.m, 1000, 42)
        cls.attrs = ref.assign_attributes(cls.pop, cls.m["counties"], ea, 42)
        cls.ja = ref.assign_jobs(cls.pop, cls.m["counties"], ea, jobs, cls.attrs, 42)
        cls.wage = ref.assign_wages(cls.pop, load("earnings2021_ro.json"), cls.attrs, cls.ja, 42)
        cls.h = ref.assign_housing(cls.pop, cls.m["counties"], housing, 42)
        cls.i = ref.assign_other_incomes(cls.pop, cls.inc, cls.attrs, cls.ja, cls.h, cls.wage, 42)
        cls.fit = ref.income_fit(cls.pop, cls.inc, cls.attrs, cls.ja, cls.h, cls.wage, cls.i)

    def test_take_from_top_stops_when_the_weight_is_reached(self):
        self.assertEqual(ref.take_from_top([0, 1, 2, 3], [5, 5, 5, 5], 8), [3, 2])
        self.assertEqual(ref.take_from_top([0, 1], [5, 5], 0), [])
        self.assertEqual(ref.take_from_top([0, 1], [5, 5], 99), [1, 0])

    def test_structure(self):
        i, status = self.i, self.ja["employment_status"]
        minimum = self.inc["minimum_wage"] * 100
        for p in range(len(self.wage)):
            declared, wage = i["wage_declared"][p], self.wage[p]
            self.assertLessEqual(declared, wage)
            if 0 < declared < wage:
                self.assertGreaterEqual(declared, minimum)                   # an envelope declares at least the minimum wage
            self.assertEqual(i["self_employment_income"][p] > 0, status[p] in (2, 3, 4))
            self.assertLessEqual(i["self_employment_declared"][p], i["self_employment_income"][p])
            if i["informal_income"][p]:
                self.assertTrue(15 * 12 <= self.pop["age"][p] < 75 * 12)
        for hh in range(len(self.pop["hh_weight"])):
            if i["hh_subsistence_income"][hh]:
                self.assertFalse(self.h["hh_urban"][hh] or self.pop["hh_collective"][hh])

    def test_totals_and_shares_are_the_ones_asked_for(self):
        fit = self.fit
        self.assertLess(fit["mixed_income_relative_error"], 1e-5)
        for got, want in zip(fit["undeclared_wage_share_by_group"], fit["target_by_group"]):
            self.assertAlmostEqual(got, want, delta=0.03)
        self.assertEqual(fit["undeclared_wage_share_by_group"][8], 0.0)
        self.assertAlmostEqual(fit["informal_recipients_share_of_15_to_74"], 0.14, delta=0.005)
        self.assertAlmostEqual(fit["rural_households_growing_food"], 0.6, delta=0.01)
        formal, partly, informal = fit["employees_formal_partly_informal"]
        self.assertGreater(formal, 0.65)
        self.assertGreater(partly, informal)

    def test_the_informal_economy_is_the_size_of_the_anchors(self):
        # Between the measured share of labour input (21.7%) and well above it: the national
        # accounts book most of the informal economy as household income.
        self.assertTrue(0.20 < self.fit["undeclared_share_of_private_labour_income"] < 0.45)

    def test_same_seed_same_incomes_and_the_hash_of_the_spec(self):
        again = ref.assign_other_incomes(self.pop, self.inc, self.attrs, self.ja, self.h, self.wage, 42)
        self.assertEqual(again, self.i)
        self.assertEqual(f"{ref.incomes_hash(self.i):016x}", "40c861af66b29b98")
        other = ref.assign_other_incomes(self.pop, self.inc, self.attrs, self.ja, self.h, self.wage, 7)
        self.assertNotEqual(other, self.i)

    def test_with_no_undeclared_share_every_wage_is_declared(self):
        clean = dict(self.inc, undeclared_wage_share=[0] * 10)
        i = ref.assign_other_incomes(self.pop, clean, self.attrs, self.ja, self.h, self.wage, 42)
        self.assertEqual(i["wage_declared"], self.wage)

    def test_bad_inputs_are_rejected(self):
        with self.assertRaisesRegex(ref.GenError, "ShapeMismatch"):
            ref.assign_other_incomes(self.pop, self.inc, self.attrs, self.ja, self.h, self.wage[:-1], 42)
        with self.assertRaisesRegex(ref.GenError, "ShapeMismatch"):
            ref.assign_other_incomes(self.pop, dict(self.inc, undeclared_wage_share=[0]), self.attrs, self.ja, self.h, self.wage)
        with self.assertRaisesRegex(ref.GenError, "ShapeMismatch"):
            ref.assign_other_incomes(self.pop, dict(self.inc, fully_informal_share=2 * UNIT), self.attrs, self.ja, self.h, self.wage)
        with self.assertRaisesRegex(ref.GenError, "EmptyTable"):
            ref.assign_other_incomes(self.pop, dict(self.inc, industry_groups=[]), self.attrs, self.ja, self.h, self.wage)


if __name__ == "__main__":
    unittest.main()
