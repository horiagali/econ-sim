"""Education and activity: census normalisation and the reference stage (spec society/population-attributes)."""
import json
import sys
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(HERE.parent / "reference"))
sys.path.insert(0, str(Path(__file__).resolve().parent))

import normalise_census as nc  # noqa: E402
import normalise_census_attributes as na  # noqa: E402
import popgen_reference as ref  # noqa: E402
from test_population import jsonstat  # noqa: E402

COUNTY_FIXTURE = HERE / "fixtures" / "census2021_margins_ro.json"
FIXTURE = HERE / "fixtures" / "census2021_edu_activity_ro.json"
REGION, COUNTY = "RO99", "RO999"


def small_tables(suppress_activity=(), suppress_education=(), unknown=0):
    """One region with one county. Per sex and year of age: 3 children under 15;
    from 15 on, 1 employed (upper secondary) and 2 retired (primary)."""
    a_dims = {"geo": [REGION], "sex": ["F", "M"], "age": na.YEAR_CODES, "wstatus": ["TOTAL", *na.ACTIVITY_CODES, "UNK"]}
    a_values = {}
    for sex in "FM":
        for age, y in enumerate(na.YEAR_CODES):
            row = {"B_AGE_MIN": 3} if age < 15 else {"EMP": 1, "INC": 2}
            for code in na.ACTIVITY_CODES:
                a_values[(REGION, sex, y, code)] = row.get(code, 0)
            a_values[(REGION, sex, y, "TOTAL")] = 3
            a_values[(REGION, sex, y, "UNK")] = unknown
    e_dims = {"geo": [REGION], "sex": ["F", "M"], "age": nc.AGE_CODES, "isced11": ["TOTAL", *na.EDU_CODES, "UNK"],
              "wstatus": ["TOTAL", *na.STATUS_CODES, "UNK"]}
    e_values = {}
    for sex in "FM":
        for band, a in enumerate(nc.AGE_CODES):
            years = 5 if band < 20 else 1
            cells = {("NAP", "INAC"): 3 * years} if band < 3 else {("ED3", "EMP"): years, ("ED1", "INAC"): 2 * years}
            for code in na.EDU_CODES:
                for st in na.STATUS_CODES:
                    e_values[(REGION, sex, a, code, st)] = cells.get((code, st), 0)
            e_values[(REGION, sex, a, "TOTAL", "TOTAL")] = 3 * years
    for key in suppress_activity:
        del a_values[key]
    for key in suppress_education:
        del e_values[key]
    persons = {"dimension": {"geo": {"category": {"index": {COUNTY: 0}}}}}
    return jsonstat(a_dims, a_values), jsonstat(e_dims, e_values), persons


def small_county_margins(per_band=30):
    """County margins holding the persons of `small_tables`: 15 per sex and band, 3 in the last."""
    bands = [per_band // 2] * 20 + [3]
    return {"counties": [COUNTY], "persons_private": [[bands, bands]],
            "persons_collective": [[[0] * 21, [0] * 21]]}


class TestNormaliseAttributes(unittest.TestCase):
    def test_complete_tables_pass_through(self):
        m = na.build(*small_tables())
        self.assertEqual(m["regions"], [REGION])
        self.assertEqual(m["region_of_county"], {COUNTY: REGION})
        self.assertEqual(m["activity"][0][0][14], [3, 0, 0, 0, 0, 0])
        self.assertEqual(m["activity"][0][1][15], [0, 0, 1, 0, 2, 0])
        self.assertEqual(m["education"][0][0][2], [[0] * 5, [0] * 5, [15, 0, 0, 0, 0]])
        self.assertEqual(m["education"][0][0][3], [[0, 5, 0, 0, 0], [0] * 5, [10, 0, 0, 0, 0]])
        self.assertEqual(na.totals(m)["activity"]["employed"], 2 * 86)
        na.check_against_counties(m, small_county_margins())

    def test_suppressed_cells_get_what_the_total_leaves_over(self):
        gone_a = [(REGION, "F", "Y40", "EMP"), (REGION, "F", "Y40", "UNE")]
        gone_e = [(REGION, "M", "Y40-44", "ED3", "EMP")]
        m = na.build(*small_tables(suppress_activity=gone_a, suppress_education=gone_e))
        self.assertEqual(m["activity"][0][0][40], [0, 0, 1, 0, 2, 0])   # the one person goes to the first hole
        self.assertEqual(m["education"][0][1][8][0], [0, 5, 0, 0, 0])

    def test_suppression_never_fills_a_cell_that_must_be_zero(self):
        # A child cell suppressed for an adult, an activity suppressed for a child:
        # both are zero, so the residual goes to the cell that can hold it.
        gone = [(REGION, "F", "Y40", "B_AGE_MIN"), (REGION, "F", "Y40", "INC"),
                (REGION, "F", "Y3", "EMP"), (REGION, "F", "Y3", "B_AGE_MIN")]
        m = na.build(*small_tables(suppress_activity=gone))
        self.assertEqual(m["activity"][0][0][40], [0, 0, 1, 0, 2, 0])
        self.assertEqual(m["activity"][0][0][3], [3, 0, 0, 0, 0, 0])

    def test_cells_that_contradict_their_total_are_rejected(self):
        activity, education, persons = small_tables()
        activity["value"]["2"] = 99   # F, under 1, EDUC
        with self.assertRaises(nc.CensusError):
            na.build(activity, education, persons)

    def test_persons_of_unknown_status_are_rejected(self):
        with self.assertRaises(nc.CensusError):
            na.build(*small_tables(unknown=1))

    def test_tables_must_hold_the_persons_of_the_county_margins(self):
        m = na.build(*small_tables())
        with self.assertRaises(nc.CensusError):
            na.check_against_counties(m, small_county_margins(per_band=32))

    def test_committed_fixture_matches_the_census_and_the_county_fixture(self):
        m = json.loads(FIXTURE.read_text(encoding="utf-8"))
        self.assertEqual(len(m["regions"]), 8)
        self.assertEqual(len(m["region_of_county"]), 42)
        self.assertEqual(na.totals(m), m["totals"])
        self.assertEqual(sum(m["totals"]["activity"].values()), 19_053_815)
        self.assertEqual(sum(m["totals"]["edu_level"].values()), 19_053_815)
        self.assertEqual(m["totals"]["activity"]["child"], 3_073_902)
        # Published national totals; the fill of suppressed cells moves a handful of persons.
        for name, published in (("employed", 7_689_171), ("unemployed", 495_848), ("in_education", 1_132_190),
                                ("retired", 4_410_077), ("inactive_other", 2_252_627)):
            self.assertLessEqual(abs(m["totals"]["activity"][name] - published), 10, name)
        na.check_against_counties(m, json.loads(COUNTY_FIXTURE.read_text(encoding="utf-8")))


class TestSplit(unittest.TestCase):
    def test_weighted_sizes_follow_the_targets(self):
        carry = [0, 0, 0]
        got = ref.split([10] * 10, [50, 30, 20], carry)
        self.assertEqual(got, [0] * 5 + [1] * 3 + [2] * 2)
        self.assertEqual(carry, [0, 0, 0])

    def test_what_a_group_cannot_hold_is_carried_to_the_next(self):
        # Three groups of one person of weight 10, each owing 4 to the second category.
        carry = [0, 0]
        got = [ref.split([10], [6, 4], carry)[0] for _ in range(5)]
        self.assertEqual(sum(got), 2)            # 5 x 4 = 20 = two persons
        self.assertEqual(sum(carry), 0)
        self.assertLessEqual(max(abs(c) for c in carry), 5)

    def test_a_zero_target_never_receives_anyone(self):
        carry = [-30, 30]                        # the second category is owed three persons
        self.assertEqual(ref.split([10] * 4, [7, 0], carry), [0] * 4)
        self.assertEqual(carry, [-30, 30])
        # Owed 5 and 35 of the 40: the first midpoint (5) sits on the boundary and goes to the later category.
        self.assertEqual(ref.split([10] * 4, [7, 1], carry), [1, 1, 1, 1])
        self.assertEqual(carry, [5, -5])

    def test_unequal_weights_use_the_midpoint(self):
        carry = [0, 0]
        self.assertEqual(ref.split([30, 10, 60], [35, 65], carry), [0, 1, 1])
        self.assertEqual(carry, [5, -5])


class TestBandTargets(unittest.TestCase):
    def test_rows_are_exact_and_the_band_keeps_its_census_totals(self):
        census = [[90, 10], [50, 50], [10, 90]]          # year of age x activity; band totals 150, 150
        weights = [300, 100, 200]                        # synthetic persons are spread differently
        t = ref.band_activity_targets(census, weights)
        self.assertEqual([sum(row) for row in t], weights)
        cols = [sum(row[k] for row in t) for k in range(2)]
        self.assertLessEqual(abs(cols[0] - 300), 1)
        # Each year keeps its direction: the first is mostly the first activity, the last mostly the second.
        self.assertGreater(t[0][0], t[0][1])
        self.assertLess(t[2][0], t[2][1])

    def test_a_year_the_census_has_nobody_in_uses_the_band(self):
        t = ref.band_activity_targets([[0, 0], [30, 10]], [40, 40])
        self.assertEqual(t, [[30, 10], [30, 10]])

    def test_years_without_synthetic_persons_get_nothing(self):
        # The band's census totals (35, 15) still count the year nobody stands for.
        t = ref.band_activity_targets([[5, 5], [30, 10]], [0, 40])
        self.assertEqual(t, [[0, 0], [28, 12]])


class TestAttributesReference(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.m = json.loads(COUNTY_FIXTURE.read_text(encoding="utf-8"))
        cls.ea = json.loads(FIXTURE.read_text(encoding="utf-8"))
        cls.pop = ref.generate(cls.m, 1000)
        cls.before = ref.state_hash(cls.pop)
        cls.attrs = ref.assign_attributes(cls.pop, cls.m["counties"], cls.ea)

    def test_structure(self):
        edu, act = self.attrs["edu_level"], self.attrs["activity"]
        self.assertEqual(len(edu), len(self.pop["age"]))
        self.assertEqual(len(act), len(self.pop["age"]))
        for age, e, a in zip(self.pop["age"], edu, act):
            if age < 15 * 12:
                self.assertEqual((a, e), (ref.ACT_CHILD if age < 6 * 12 else ref.ACT_PUPIL, 0))
            else:
                self.assertNotEqual(a, ref.ACT_CHILD)
                if a == ref.ACT_STUDENT:
                    self.assertGreaterEqual(e, 1)
                if a == ref.ACT_PUPIL:
                    self.assertEqual(e, 0)

    def test_the_population_is_not_changed(self):
        self.assertEqual(ref.state_hash(self.pop), self.before)
        self.assertEqual(f"{self.before:016x}", "247112c87ae60cf2")

    def test_hash_is_the_one_recorded_in_the_spec(self):
        self.assertEqual(f"{ref.attributes_hash(self.attrs):016x}", "8a0509e8ebc90647")

    def test_fit_is_within_the_spec_tolerances(self):
        fit = ref.attribute_fit_errors(self.pop, self.m["counties"], self.ea, self.attrs, 1000)
        share = fit.pop("national year of age: activity share")
        self.assertLessEqual(share[">=100 records at that age"], 0.05)
        limits = {">=1000 records": 0.01, ">=100 records": 0.06, ">=30 records": 0.15}
        for family, buckets in fit.items():
            for bucket, error in buckets.items():
                self.assertLessEqual(error, limits[bucket], f"{family} {bucket}")

    def test_activity_follows_age(self):
        got_a, _ = ref.attribute_totals(self.pop, self.m["counties"], self.ea, self.attrs)

        def share(year: int, k: int) -> float:
            cells = {key[3]: 0 for key in got_a}
            for (_r, _s, y, kk), v in got_a.items():
                if y == year:
                    cells[kk] += v
            return cells[k] / sum(cells.values())

        in_education, retired = 1, 4
        self.assertGreater(share(15, in_education), share(19, in_education))
        self.assertGreater(share(19, in_education), share(24, in_education))
        self.assertGreater(share(69, retired), share(64, retired))
        self.assertGreater(share(64, retired), share(59, retired))

    def test_another_seed_changes_persons_not_totals(self):
        other = ref.assign_attributes(self.pop, self.m["counties"], self.ea, rng_seed=7)
        self.assertNotEqual(other, self.attrs)
        fit = ref.attribute_fit_errors(self.pop, self.m["counties"], self.ea, other, 1000)
        self.assertLessEqual(fit["national activity"][">=1000 records"], 0.01)
        self.assertLessEqual(fit["national education"][">=1000 records"], 0.01)

    def test_bad_margins_are_rejected(self):
        counties = self.m["counties"]
        short = json.loads(json.dumps(self.ea))
        short["activity"][0][0].pop()
        with self.assertRaisesRegex(ref.GenError, "ShapeMismatch"):
            ref.assign_attributes(self.pop, counties, short)
        empty = dict(self.ea, regions=[])
        with self.assertRaisesRegex(ref.GenError, "EmptyTable"):
            ref.assign_attributes(self.pop, counties, empty)
        lost = dict(self.ea, region_of_county={k: v for k, v in self.ea["region_of_county"].items() if k != counties[0]})
        with self.assertRaisesRegex(ref.GenError, "UnknownCounty"):
            ref.assign_attributes(self.pop, counties, lost)
        nobody = json.loads(json.dumps(self.ea))
        for year in range(40, 45):
            nobody["activity"][0][0][year] = [0] * 6
        with self.assertRaisesRegex(ref.GenError, "NoMargin"):
            ref.assign_attributes(self.pop, counties, nobody)


if __name__ == "__main__":
    unittest.main()
