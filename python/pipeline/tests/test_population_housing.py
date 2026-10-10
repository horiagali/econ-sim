"""Locality size and tenure: census normalisation and the reference stage (spec society/population-housing)."""
import json
import sys
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(HERE.parent / "reference"))
sys.path.insert(0, str(Path(__file__).resolve().parent))

import normalise_census as nc  # noqa: E402
import normalise_census_housing as nh  # noqa: E402
import popgen_reference as ref  # noqa: E402
from test_population import jsonstat  # noqa: E402

COUNTY_FIXTURE = HERE / "fixtures" / "census2021_margins_ro.json"
FIXTURE = HERE / "fixtures" / "census2021_housing_ro.json"
COUNTY = "RO999"


def small_docs(suppress=(), unknown_tenure=6):
    """One county. Per sex and age group: 10 persons in localities of 2,000-4,999
    and 20 in localities of 20,000-49,999. 100 households, 30 of one person."""
    dims = {"geo": [COUNTY], "sex": ["F", "M"], "age": ["TOTAL", *nh.AGE_GROUP_CODES], "n_person": ["TOTAL", *nh.SIZE_CODES]}
    values = {}
    for sex in "FM":
        for a in ["TOTAL", *nh.AGE_GROUP_CODES]:
            times = 6 if a == "TOTAL" else 1
            for code in nh.SIZE_CODES:
                values[(COUNTY, sex, a, code)] = {"2000-4999": 10, "20000-49999": 20}.get(code, 0) * times
            values[(COUNTY, sex, a, "TOTAL")] = 30 * times
    for key in suppress:
        del values[key]
    tdims = {"geo": [COUNTY], "hhcomp": ["TOTAL", "P1"], "tenure": ["TOTAL", "OWN", "RENT", "OTH", "UNK"]}
    tvalues = {(COUNTY, "TOTAL", "OWN"): 80, (COUNTY, "TOTAL", "RENT"): 10, (COUNTY, "TOTAL", "OTH"): 10 - unknown_tenure,
               (COUNTY, "TOTAL", "UNK"): unknown_tenure, (COUNTY, "TOTAL", "TOTAL"): 100,
               (COUNTY, "P1", "OWN"): 20, (COUNTY, "P1", "RENT"): 6, (COUNTY, "P1", "OTH"): 4,
               (COUNTY, "P1", "UNK"): 0, (COUNTY, "P1", "TOTAL"): 30}
    return jsonstat(dims, values), jsonstat(tdims, tvalues)


def small_county_margins():
    half = [9] * 20 + [0]
    return {"counties": [COUNTY], "persons_private": [[half, half]], "persons_collective": [[[0] * 21, [0] * 21]],
            "households": [[30, 40, 20, 10, 0, 0, 0]]}


class TestNormaliseHousing(unittest.TestCase):
    def test_classes_are_grouped_and_sexes_added(self):
        m = nh.build(*small_docs())
        self.assertEqual(m["counties"], [COUNTY])
        self.assertEqual(m["persons_by_locality"][0], [[0, 20, 0, 40, 0, 0]] * 6)
        self.assertEqual(nh.totals(m)["persons_urban"], 240)
        nh.check_against_counties(m, small_county_margins())

    def test_unknown_tenure_is_spread_in_proportion(self):
        m = nh.build(*small_docs())
        self.assertEqual(m["households_by_tenure"][0][0], [20, 6, 4])
        # Larger households: 60 owners, 4 tenants, 0 other known, 6 unknown.
        self.assertEqual(m["households_by_tenure"][0][1], [66, 4, 0])
        self.assertEqual(sum(map(sum, m["households_by_tenure"][0])), 100)

    def test_a_suppressed_cell_gets_what_its_total_leaves_over(self):
        gone = [(COUNTY, "F", "Y_GE85", "2000-4999"), (COUNTY, "F", "Y_GE85", "LT200")]
        m = nh.build(*small_docs(suppress=gone))
        # The class the county has nobody in stays zero; the other hole takes the 10 persons.
        self.assertEqual(m["persons_by_locality"][0][5], [0, 20, 0, 40, 0, 0])

    def test_tables_must_hold_the_persons_and_households_of_the_county_margins(self):
        m = nh.build(*small_docs())
        other = small_county_margins()
        other["households"] = [[31, 39, 20, 10, 0, 0, 0]]
        with self.assertRaises(nc.CensusError):
            nh.check_against_counties(m, other)
        other = small_county_margins()
        other["persons_private"][0][0][3] += 1
        with self.assertRaises(nc.CensusError):
            nh.check_against_counties(m, other)

    def test_committed_fixture_matches_the_census_and_the_county_fixture(self):
        m = json.loads(FIXTURE.read_text(encoding="utf-8"))
        self.assertEqual(len(m["counties"]), 42)
        self.assertEqual(nh.totals(m), m["totals"])
        self.assertEqual(sum(m["totals"]["persons_by_locality"].values()), 19_053_815)
        self.assertEqual(sum(m["totals"]["households_by_tenure"].values()), 7_709_139)
        self.assertEqual(m["totals"]["persons_urban"], 9_650_571)
        nh.check_against_counties(m, json.loads(COUNTY_FIXTURE.read_text(encoding="utf-8")))


class TestHousingReference(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.m = json.loads(COUNTY_FIXTURE.read_text(encoding="utf-8"))
        cls.housing = json.loads(FIXTURE.read_text(encoding="utf-8"))
        cls.pop = ref.generate(cls.m, 1000)
        cls.before = ref.state_hash(cls.pop)
        cls.ha = ref.assign_housing(cls.pop, cls.m["counties"], cls.housing)

    def test_structure(self):
        ha, pop = self.ha, self.pop
        self.assertEqual(len(ha["hh_tenure"]), len(pop["hh_weight"]))
        for h in range(len(pop["hh_weight"])):
            k = ha["hh_locality_size"][h]
            self.assertTrue(0 <= k <= 5)
            self.assertEqual(ha["hh_urban"][h], int(k >= 3))
            self.assertGreater(sum(row[k] for row in self.housing["persons_by_locality"][pop["hh_county"][h]]), 0)
            if pop["hh_collective"][h]:
                self.assertEqual(ha["hh_tenure"][h], ref.TENURE_NONE)
            else:
                self.assertIn(ha["hh_tenure"][h], (0, 1, 2))

    def test_the_population_is_not_changed(self):
        self.assertEqual(ref.state_hash(self.pop), self.before)

    def test_hash_is_the_one_recorded_in_the_spec(self):
        self.assertEqual(f"{ref.housing_hash(self.ha):016x}", "a4b8ac6c2c2bb24e")

    def test_fit_is_within_the_spec_tolerances(self):
        fit = ref.housing_fit_errors(self.pop, self.housing, self.ha, 1000)
        self.assertLessEqual(fit.pop("share of 65+ by locality class")["largest difference"], 0.04)
        limits = {">=1000 records": 0.01, ">=100 records": 0.05, ">=30 records": 0.15}
        for family, buckets in fit.items():
            for bucket, error in buckets.items():
                self.assertLessEqual(error, limits[bucket], f"{family} {bucket}")

    def test_the_old_live_in_the_smallest_localities(self):
        got, _ = ref.housing_totals(self.pop, self.housing, self.ha)
        shares = ref.old_share_by_class(self.housing, got)
        self.assertEqual(shares.index(max(shares)), 0)

    def test_another_seed_changes_households_not_totals(self):
        other = ref.assign_housing(self.pop, self.m["counties"], self.housing, rng_seed=7)
        self.assertNotEqual(other, self.ha)
        fit = ref.housing_fit_errors(self.pop, self.housing, other, 1000)
        self.assertLessEqual(fit["national locality class"][">=1000 records"], 0.01)
        self.assertLessEqual(fit["national tenure"][">=1000 records"], 0.01)

    def test_bad_margins_are_rejected(self):
        counties = self.m["counties"]
        with self.assertRaisesRegex(ref.GenError, "ShapeMismatch"):
            ref.assign_housing(self.pop, counties, dict(self.housing, counties=counties[1:]))
        short = json.loads(json.dumps(self.housing))
        short["persons_by_locality"][3][2].pop()
        with self.assertRaisesRegex(ref.GenError, "ShapeMismatch"):
            ref.assign_housing(self.pop, counties, short)
        with self.assertRaisesRegex(ref.GenError, "EmptyTable"):
            ref.assign_housing(self.pop, counties, dict(self.housing, locality_classes=[]))
        nobody = json.loads(json.dumps(self.housing))
        nobody["persons_by_locality"][0] = [[0] * 6 for _ in range(6)]
        with self.assertRaisesRegex(ref.GenError, "NoMargin"):
            ref.assign_housing(self.pop, counties, nobody)


if __name__ == "__main__":
    unittest.main()
