"""Wealth: normalisation and the reference stage (spec society/population-wealth)."""
import json
import sys
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(HERE.parent / "reference"))
sys.path.insert(0, str(Path(__file__).resolve().parent))

import normalise_census as nc  # noqa: E402
import normalise_wealth as nw  # noqa: E402
import popgen_reference as ref  # noqa: E402
from test_population import jsonstat  # noqa: E402

F = HERE / "fixtures"
FIXTURE = F / "household_wealth2021_ro.json"
UNIT = 1_000_000


def small_docs(loans=100.0):
    sheet = jsonstat({"finpos": ["ASS", "LIAB"], "na_item": ["F21", "F22_F29", "F4"], "unit": ["MIO_NAC"]},
                     {("ASS", "F21", "MIO_NAC"): 50.0, ("ASS", "F22_F29", "MIO_NAC"): 250.0,
                      ("ASS", "F4", "MIO_NAC"): 9.0, ("LIAB", "F4", "MIO_NAC"): loans})
    dwellings = jsonstat({"asset10": ["N111N"]}, {("N111N",): 1000.0})
    quintiles = ["TOTAL", *nw.QUINTILES]
    cells = {("TOTAL", q, "WLTH_NET"): v for q, v in zip(nw.QUINTILES, (1.0, 4.0, 9.0, 16.0, 70.0))}
    cells.update({(q, "TOTAL", "WLTH_NET"): v for q, v in zip(nw.QUINTILES, (10.0, 12.0, 15.0, 21.0, 42.0))})
    pattern = jsonstat({"quant_inc": quintiles, "quant_wlth": quintiles, "indic_ewb": ["WLTH_NET"]}, cells)
    return {nw.BALANCE_ID: sheet, nw.DWELLINGS_ID: dwellings, nw.PATTERN_ID: pattern}


class TestNormaliseWealth(unittest.TestCase):
    def test_totals_are_the_published_ones_and_the_two_guessed_splits(self):
        m = nw.build(small_docs())
        t = m["totals"]
        self.assertEqual((t["cash"], t["deposits"]), (50_000_000, 250_000_000))
        self.assertEqual(t["housing_loans"] + t["consumer_loans"], 100_000_000)       # liabilities, not assets
        self.assertEqual(t["housing_loans"], 58_000_000)
        self.assertEqual(t["dwellings"], 2_700_000_000)                              # 2.7 times replacement value
        self.assertEqual(m["pattern"]["net_wealth_share_by_wealth_quintile"][4], 700_000)
        self.assertEqual(m["pattern"]["net_wealth_share_by_income_quintile"][0], 100_000)

    def test_curves_and_rules_are_in_millionths_and_well_formed(self):
        m = nw.build(small_docs())
        for name in ("liquid_curve", "dwelling_curve", "loan_curve"):
            c = m[name]
            self.assertEqual((c["rank"][0], c["rank"][-1]), (0, UNIT))
            self.assertEqual(c["value"], sorted(c["value"]))
        self.assertEqual(len(m["rel_locality"]), 6)
        self.assertEqual(m["mortgage"]["share"], 60_000)
        self.assertEqual(m["consumer"]["age_to"], 65)
        ref.check_wealth_margins(m)
        with self.assertRaises(nc.CensusError):
            nw.curve({"rank": [0.0, 0.5], "value": [1.0, 2.0]})                      # does not reach rank 1

    def test_missing_inputs_are_rejected(self):
        docs = small_docs()
        docs[nw.BALANCE_ID] = jsonstat({"finpos": ["ASS"], "na_item": ["F21"], "unit": ["MIO_NAC"]},
                                       {("ASS", "F21", "MIO_NAC"): 1.0})
        with self.assertRaises(nc.CensusError):
            nw.build(docs)

    def test_committed_fixture_is_what_the_published_tables_say(self):
        m = json.loads(FIXTURE.read_text(encoding="utf-8"))
        t = m["totals"]
        self.assertEqual((t["cash"], t["deposits"]), (57_326_000_000, 294_173_000_000))
        self.assertEqual(t["housing_loans"] + t["consumer_loans"], 187_300_600_000)
        self.assertEqual(m["pattern"]["country"], "HU")
        self.assertIn("guesses", m["provenance"])
        ref.check_wealth_margins(m)


class TestWealthReference(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        load = lambda name: json.loads((F / name).read_text(encoding="utf-8"))  # noqa: E731
        cls.m, ea, jobs = load("census2021_margins_ro.json"), load("census2021_edu_activity_ro.json"), load("census2021_jobs_ro.json")
        cls.housing, cls.wealth = load("census2021_housing_ro.json"), load("household_wealth2021_ro.json")
        cls.pop = ref.generate(cls.m, 1000, 42)
        attrs = ref.assign_attributes(cls.pop, cls.m["counties"], ea, 42)
        ja = ref.assign_jobs(cls.pop, cls.m["counties"], ea, jobs, attrs, 42)
        cls.wage = ref.assign_wages(cls.pop, load("earnings2021_ro.json"), attrs, ja, 42)
        cls.pension = ref.assign_pensions(cls.pop, load("pensions2021_ro.json"), attrs, 42)
        h = ref.assign_housing(cls.pop, cls.m["counties"], cls.housing, 42)
        cls.h_attrs = dict(h, urban_from_class=cls.housing["urban_from_class"])
        cls.w = ref.assign_wealth(cls.pop, cls.wealth, cls.h_attrs, cls.wage, cls.pension, 42)
        cls.fit = ref.wealth_fit(cls.pop, cls.wealth, cls.h_attrs, cls.w, cls.wage, cls.pension)

    def test_mixed_order_follows_income_draw_or_both(self):
        weight, income = [1, 1, 1, 1], [40, 10, 30, 20]
        draw = {0: 0, 1: 900_000, 2: 300_000, 3: 600_000}
        self.assertEqual(ref.mixed_order([0, 1, 2, 3], weight, income, draw, UNIT), [1, 3, 2, 0])      # by income
        self.assertEqual(ref.mixed_order([0, 1, 2, 3], weight, income, draw, 0), [0, 2, 3, 1])         # by draw
        # A bonus larger than any key puts its holder last.
        self.assertEqual(ref.mixed_order([0, 1, 2, 3], weight, income, draw, UNIT, {0: 0, 1: 2 * UNIT * UNIT, 2: 0, 3: 0})[-1], 1)

    def test_structure(self):
        w, pop = self.w, self.pop
        for h in range(len(pop["hh_weight"])):
            owner = self.h_attrs["hh_tenure"][h] == ref.TENURE_OWNER
            if pop["hh_collective"][h]:
                self.assertEqual([w[name][h] for name in ref.WEALTH_COLUMNS], [0] * 5)
                continue
            self.assertEqual(w["hh_dwelling_value"][h] > 0, owner)
            self.assertGreater(w["hh_cash"][h] + w["hh_deposits"][h], 0)
            if w["hh_mortgage"][h]:
                self.assertTrue(owner)

    def test_totals_are_met_and_shares_are_the_guessed_ones(self):
        self.assertLess(max(self.fit["total_relative_error"].values()), 1e-5)
        with_ = self.fit["share_of_households_with"]
        self.assertAlmostEqual(with_["hh_mortgage"], 0.06, delta=0.005)
        self.assertAlmostEqual(with_["hh_consumer_debt"], 0.25, delta=0.005)
        self.assertAlmostEqual(self.fit["liquid_share_of_top_tenth"], 0.67, delta=0.05)
        values = self.fit["mean_dwelling_lei_by_locality_class"]
        self.assertEqual(values, sorted(values))
        self.assertAlmostEqual(values[5] / values[0], 2.30 / 0.45, delta=0.3)

    def test_net_wealth_is_concentrated_and_loosely_tied_to_income(self):
        by_wealth, by_income = self.fit["net_wealth_share_by_wealth_fifth"], self.fit["net_wealth_share_by_income_fifth"]
        self.assertGreater(by_wealth[4], 0.45)
        self.assertLess(by_wealth[0], 0.05)
        for got, want in zip(by_income, self.fit["pattern_by_income_fifth"]):
            self.assertAlmostEqual(got, want, delta=0.06)
        self.assertLess(self.fit["share_with_negative_net_wealth"], 0.06)

    def test_same_seed_same_wealth_and_the_hash_of_the_spec(self):
        again = ref.assign_wealth(self.pop, self.wealth, self.h_attrs, self.wage, self.pension, 42)
        self.assertEqual(again, self.w)
        self.assertEqual(f"{ref.wealth_hash(self.w):016x}", "8b7d9a0ea0d3c373")
        other = ref.assign_wealth(self.pop, self.wealth, self.h_attrs, self.wage, self.pension, 7)
        self.assertNotEqual(other, self.w)

    def test_without_incomes_places_are_by_draw_alone_and_totals_still_hold(self):
        w = ref.assign_wealth(self.pop, self.wealth, self.h_attrs, None, None, 42)
        fit = ref.wealth_fit(self.pop, self.wealth, self.h_attrs, w)
        self.assertLess(max(fit["total_relative_error"].values()), 1e-5)

    def test_bad_inputs_are_rejected(self):
        with self.assertRaisesRegex(ref.GenError, "ShapeMismatch"):
            ref.assign_wealth(self.pop, self.wealth, {k: v[:-1] if isinstance(v, list) else v for k, v in self.h_attrs.items()})
        with self.assertRaisesRegex(ref.GenError, "ShapeMismatch"):
            ref.assign_wealth(self.pop, self.wealth, self.h_attrs, self.wage[:-1], self.pension)
        with self.assertRaisesRegex(ref.GenError, "EmptyTable"):
            ref.assign_wealth(self.pop, dict(self.wealth, rel_locality=[]), self.h_attrs)
        broken = dict(self.wealth, liquid_curve={"rank": [0, 500_000], "value": [1, 2]})
        with self.assertRaisesRegex(ref.GenError, "ShapeMismatch"):
            ref.assign_wealth(self.pop, broken, self.h_attrs)


if __name__ == "__main__":
    unittest.main()
