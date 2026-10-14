"""Kinds of inactivity and enrolment: the reference stage (spec society/population-inactivity-enrolment)."""
import copy
import json
import sys
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(HERE.parent / "reference"))

import popgen_reference as ref  # noqa: E402

F = HERE / "fixtures"


def load(name):
    return json.loads((F / name).read_text(encoding="utf-8"))


class TestInactivityEnrolmentReference(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        m, ea = load("census2021_margins_ro.json"), load("census2021_edu_activity_ro.json")
        cls.ie = load("inactivity_enrolment2021_ro.json")
        cls.pop = ref.generate(m, 1000, 42)
        cls.attrs = ref.assign_attributes(cls.pop, m["counties"], ea, 42)
        cls.k = ref.assign_inactivity_enrolment(cls.pop, cls.attrs, cls.ie, 42)

    def test_fixture_hash_and_determinism(self):
        self.assertEqual(f"{ref.inactivity_enrolment_hash(self.k):016x}", "6bbd2499e40e5b15")
        self.assertEqual(self.k, ref.assign_inactivity_enrolment(self.pop, self.attrs, self.ie, 42))
        self.assertNotEqual(self.k, ref.assign_inactivity_enrolment(self.pop, self.attrs, self.ie, 7))

    def test_structure(self):
        act, edu = self.attrs["activity"], self.attrs["edu_level"]
        for i, (kind, level) in enumerate(zip(self.k["inactive_kind"], self.k["enrolled"])):
            self.assertEqual(kind != ref.INACTIVE_NONE, act[i] == ref.ACT_INACTIVE_OTHER)
            if act[i] == ref.ACT_STUDENT:
                self.assertNotEqual(level, ref.NOT_ENROLLED)
            if act[i] == ref.ACT_RETIRED:
                self.assertEqual(level, ref.NOT_ENROLLED)
            if act[i] == ref.ACT_CHILD:
                self.assertIn(level, (ref.NOT_ENROLLED, 1))
            if level >= ref.FIRST_LEVEL_AFTER_SECONDARY:
                self.assertGreaterEqual(edu[i], ref.STUDENT_MIN_EDU)
            if level and act[i] in (ref.ACT_EMPLOYED, ref.ACT_UNEMPLOYED, ref.ACT_INACTIVE_OTHER):
                self.assertGreaterEqual(self.pop["age"][i], 15 * 12)

    def test_pupils_left_unenrolled_only_where_the_places_are_full(self):
        # A pupil is left not enrolled only in a sex and enrolment age group whose enrolled weight
        # reaches its places (the school statistics at the group's rate), within one person's weight.
        groups = [(g[0], g[1]) for g in self.ie["enrolment_ages"]]
        w = [self.pop["hh_weight"][h] for h in self.pop["household_id"]]
        cell = {}
        for i, level in enumerate(self.k["enrolled"]):
            y = self.pop["age"][i] // 12
            g = next(k for k, (lo, hi) in enumerate(groups) if y >= lo and (hi is None or y < hi))
            c = cell.setdefault((self.pop["sex"][i], g), {"weight": 0, "enrolled": 0, "left": 0, "max": 0})
            c["weight"] += w[i]
            c["max"] = max(c["max"], w[i])
            c["enrolled"] += w[i] if level else 0
            c["left"] += w[i] if not level and self.attrs["activity"][i] == ref.ACT_PUPIL else 0
        left = 0
        for (s, g), c in cell.items():
            if c["left"]:
                counts, pop = self.ie["enrolment"][s][g], self.ie["population"][s][g]
                places = sum(ref.apportion(c["weight"], counts + [max(pop - sum(counts), 0)])[:-1])
                self.assertGreaterEqual(c["enrolled"], places - c["max"])
                left += c["left"]
        self.assertGreater(left, 0)

    def test_fit(self):
        fit = ref.inactivity_enrolment_fit(self.pop, self.attrs, self.ie, self.k, 1000)
        for by_kind in fit["inactive_kind_by_sex"].values():
            for _, _, err in by_kind.values():
                self.assertLess(abs(err), 0.02)
        self.assertLess(fit["largest_gap_of_a_kind_share_in_a_band"], 0.03)
        for level in ("pre_school", "primary", "lower_secondary", "post_secondary", "tertiary", "master_doctorate"):
            self.assertLess(abs(fit["enrolled_by_level"][level][2]), 0.06)
        gaps = {k: v for k, v in fit["largest_gaps_of_a_level_share_in_an_age_group"].items() if k[1:] not in ("19", "20")}
        self.assertLess(max(gaps.values(), default=0), 0.03)

    def test_homemaker_child_link(self):
        shares = []
        for link in (0, 1_000_000):
            k = ref.assign_inactivity_enrolment(self.pop, self.attrs, self.ie, 42, homemaker_child_link_ppm=link)
            shares.append(ref.inactivity_enrolment_fit(self.pop, self.attrs, self.ie, k, 1000)
                          ["share_living_with_a_young_child"]["homemaker"])
        self.assertGreater(shares[1] - shares[0], 0.10)

    def test_bad_inputs_are_rejected(self):
        with self.assertRaisesRegex(ref.GenError, "ShapeMismatch"):
            ref.assign_inactivity_enrolment(self.pop, {k: v[:-1] for k, v in self.attrs.items()}, self.ie)
        with self.assertRaisesRegex(ref.GenError, "ShapeMismatch"):
            ref.assign_inactivity_enrolment(self.pop, self.attrs, dict(self.ie, inactive_kinds=["a", "b", "c"]))
        bad = copy.deepcopy(self.ie)
        bad["enrolment"][0].pop()
        with self.assertRaisesRegex(ref.GenError, "ShapeMismatch"):
            ref.assign_inactivity_enrolment(self.pop, self.attrs, bad)
        with self.assertRaisesRegex(ref.GenError, "EmptyTable"):
            ref.assign_inactivity_enrolment(self.pop, self.attrs, dict(self.ie, age_bands=[]))
        zero = copy.deepcopy(self.ie)
        zero["inactive_kind"] = [[[0, 0, 0] for _ in s] for s in zero["inactive_kind"]]
        with self.assertRaisesRegex(ref.GenError, "EmptyTable"):
            ref.assign_inactivity_enrolment(self.pop, self.attrs, zero)


class TestNormaliser(unittest.TestCase):
    def test_kinds_keep_the_census_rows_and_the_homemakers(self):
        ea, ie = load("census2021_edu_activity_ro.json"), load("inactivity_enrolment2021_ro.json")
        other = ea["activities"].index("inactive_other")
        for s in range(2):
            for b, (lo, hi) in enumerate(ea["age_bands"]):
                years = range(lo, hi if hi is not None else len(ea["activity"][0][0]))
                census = sum(ea["activity"][r][s][y][other] for r in range(len(ea["regions"])) for y in years)
                self.assertEqual(sum(ie["inactive_kind"][s][b]), census)
        self.assertEqual(ie["totals"]["inactive_kind"]["homemaker"], 1_169_829)
        self.assertEqual(sum(ie["totals"]["enrolment"].values()), 3_495_791)


if __name__ == "__main__":
    unittest.main()
