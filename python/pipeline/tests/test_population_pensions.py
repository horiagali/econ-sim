"""Pensions: normalisation and the reference stage (spec society/population-pensions)."""
import json
import sys
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(HERE.parent / "reference"))
sys.path.insert(0, str(Path(__file__).resolve().parent))

import normalise_census as nc  # noqa: E402
import normalise_earnings as ne  # noqa: E402
import normalise_pensions as npn  # noqa: E402
import popgen_reference as ref  # noqa: E402
from test_population import jsonstat  # noqa: E402

F = HERE / "fixtures"
FIXTURE = F / "pensions2021_ro.json"
UNIT = 1_000_000
EARNINGS_CURVE = {"rank": [0, 500_000, UNIT], "value": [500_000, 900_000, 1_700_000]}


def small_docs(gap=20.0, total=120.0):
    edu = {("GE10", ne.ALL, sex, code): value * lift
           for sex, lift in (("F", 1.0), ("M", 1.2))
           for code, value in (("TOTAL", 2000), ("ED0-2", 1000), ("ED3_4", 2000), ("ED5-8", 4000))}
    return {
        npn.EXPENDITURE_ID: jsonstat(
            {"spdepb": ["TOTAL", "OLD"], "spdepm": ["TOTAL"], "unit": ["MIO_NAC"], "time": ["2021"]},
            {("TOTAL", "TOTAL", "MIO_NAC", "2021"): total, ("OLD", "TOTAL", "MIO_NAC", "2021"): 100.0}),
        npn.GAP_ID: jsonstat({"age": ["Y_GE65", "Y65-74"], "time": ["2021"]},
                             {("Y_GE65", "2021"): gap, ("Y65-74", "2021"): 1.0}),
        npn.REPLACEMENT_ID: jsonstat({"sex": ["T", "F"], "time": ["2021"]}, {("T", "2021"): 0.43, ("F", "2021"): 0.41}),
        npn.EDUCATION_ID: jsonstat(
            {"sizeclas": ["GE10"], "nace_r2": [ne.ALL], "sex": ["T", "F", "M"],
             "isced11": ["TOTAL", "ED0-2", "ED3_4", "ED5-8"]}, edu),
    }


class TestNormalisePensions(unittest.TestCase):
    def test_level_gap_and_education_proportions(self):
        # 120 billion lei a year for 5 million retired: 10 billion a month, 2,000 lei each.
        m = npn.build(small_docs(total=120_000.0), {"quantile_curve": EARNINGS_CURVE}, 5_000_000)
        self.assertEqual(m["pension_bill"], 10_000_000_000)
        self.assertEqual(m["minimum_pension"], 800)
        self.assertEqual(m["rel_sex"], [800_000, UNIT])
        self.assertEqual(m["rel_education"], [[500_000, UNIT, 2 * UNIT]] * 2)
        self.assertEqual(m["replacement_ratio"], 430_000)
        self.assertEqual(m["totals"]["mean_pension"], 2000.0)

    def test_curve_is_flat_at_the_minimum_then_shaped_like_earnings_and_averages_one(self):
        retired = 4 * npn.manual("ro_minimum_pension_beneficiaries_2021")     # a quarter at the minimum
        docs = small_docs(total=12 * 2000 * retired / 1_000_000)             # mean pension 2,000 lei
        curve = npn.build(docs, {"quantile_curve": EARNINGS_CURVE}, retired)["quantile_curve"]
        self.assertEqual(curve["rank"], [0, 250_000, 625_000, UNIT])
        self.assertEqual(curve["value"][:2], [400_000, 400_000])              # 800 lei over 2,000
        self.assertEqual(curve["value"], sorted(curve["value"]))
        area = sum((b - a) * (va + vb) / 2 for a, b, va, vb in
                   zip(curve["rank"], curve["rank"][1:], curve["value"], curve["value"][1:])) / UNIT / UNIT
        self.assertAlmostEqual(area, 1.0, places=4)
        # Above the minimum the steps keep the earnings curve's proportions: 0.4 and 0.8 of rise.
        rise = [v - curve["value"][0] for v in curve["value"]]
        self.assertAlmostEqual(rise[2] / rise[3], 400_000 / 1_200_000, places=4)

    def test_impossible_inputs_are_rejected(self):
        with self.assertRaises(nc.CensusError):
            npn.build(small_docs(), {"quantile_curve": EARNINGS_CURVE}, 0)
        with self.assertRaises(nc.CensusError):                               # mean pension below the minimum
            npn.build(small_docs(total=1.0), {"quantile_curve": EARNINGS_CURVE}, 5_000_000)
        with self.assertRaises(nc.CensusError):                               # more at the minimum than retired
            npn.build(small_docs(), {"quantile_curve": EARNINGS_CURVE}, 5_000)
        with self.assertRaises(nc.CensusError):
            npn.build(small_docs(total=120_000.0, gap=120.0), {"quantile_curve": EARNINGS_CURVE}, 5_000_000)

    def test_committed_fixture_is_what_the_published_tables_say(self):
        m = json.loads(FIXTURE.read_text(encoding="utf-8"))
        # 104,664.36 million lei in 2021.
        self.assertEqual(m["pension_bill"], 8_722_030_000)
        self.assertEqual(m["rel_sex"], [769_000, UNIT])                       # gap of 23.1%
        self.assertEqual(m["totals"]["retired"], 4_410_077)
        self.assertEqual(len(m["provenance"]["manual"]), 2)
        ref.check_pension_margins(m)


class TestPensionReference(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.m = json.loads((F / "census2021_margins_ro.json").read_text(encoding="utf-8"))
        cls.ea = json.loads((F / "census2021_edu_activity_ro.json").read_text(encoding="utf-8"))
        cls.pens = json.loads(FIXTURE.read_text(encoding="utf-8"))
        cls.scale = 1000
        cls.pop = ref.generate(cls.m, cls.scale, 42)
        cls.attrs = ref.assign_attributes(cls.pop, cls.m["counties"], cls.ea, 42)
        cls.pension = ref.assign_pensions(cls.pop, cls.pens, cls.attrs, 42)

    def test_only_the_retired_have_a_pension_and_none_is_below_the_minimum(self):
        retired = [a == ref.ACT_RETIRED for a in self.attrs["activity"]]
        self.assertTrue(all((v > 0) == r for v, r in zip(self.pension, retired)))
        self.assertEqual(min(v for v in self.pension if v), self.pens["minimum_pension"] * 100)

    def test_the_bill_is_paid_and_the_proportions_hold(self):
        fit = ref.pension_fit(self.pop, self.pens, self.attrs, self.pension, self.scale)
        self.assertLess(fit["bill_relative_error"], 1e-5)
        self.assertAlmostEqual(fit["women_over_men"], fit["target_women_over_men"], delta=0.03)
        self.assertLess(fit["education_worst_relative_error_100_records"], 0.04)
        self.assertGreater(fit["tertiary_over_low_education"], 1.3)
        self.assertAlmostEqual(fit["share_at_minimum"], fit["target_share_at_minimum"], delta=0.05)
        self.assertLess(fit["median_lei"], fit["mean_lei"])

    def test_a_weaker_link_to_earnings_flattens_pensions_by_education(self):
        flat = ref.assign_pensions(self.pop, self.pens, self.attrs, 42, earnings_link_ppm=0)
        fit = ref.pension_fit(self.pop, self.pens, self.attrs, flat, self.scale, earnings_link_ppm=0)
        self.assertAlmostEqual(fit["tertiary_over_low_education"], 1.0, delta=0.08)

    def test_same_seed_same_pensions_and_the_hash_of_the_spec(self):
        again = ref.assign_pensions(self.pop, self.pens, self.attrs, 42)
        self.assertEqual(again, self.pension)
        self.assertEqual(f"{ref.pensions_hash(self.pension):016x}", "963e671ef7035b92")
        other = ref.assign_pensions(self.pop, self.pens, self.attrs, 7)
        self.assertNotEqual(other, self.pension)

    def test_bad_inputs_are_rejected(self):
        with self.assertRaisesRegex(ref.GenError, "ShapeMismatch"):
            ref.assign_pensions(self.pop, dict(self.pens, rel_sex=[1]), self.attrs, 42)
        fewer = {k: v[:-1] for k, v in self.attrs.items()}
        with self.assertRaisesRegex(ref.GenError, "ShapeMismatch"):
            ref.assign_pensions(self.pop, self.pens, fewer, 42)
        with self.assertRaisesRegex(ref.GenError, "NoMargin"):
            ref.assign_pensions(self.pop, dict(self.pens, pension_bill=0), self.attrs, 42)
        with self.assertRaisesRegex(ref.GenError, "EmptyTable"):
            ref.assign_pensions(self.pop, dict(self.pens, edu_groups=[], rel_education=[[], []]), self.attrs, 42)


if __name__ == "__main__":
    unittest.main()
