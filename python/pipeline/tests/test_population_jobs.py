"""Jobs: census normalisation and the reference stage (spec society/population-jobs)."""
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
import popgen_reference as ref  # noqa: E402
from test_population import jsonstat  # noqa: E402

COUNTY_FIXTURE = HERE / "fixtures" / "census2021_margins_ro.json"
ACTIVITY_FIXTURE = HERE / "fixtures" / "census2021_edu_activity_ro.json"
FIXTURE = HERE / "fixtures" / "census2021_jobs_ro.json"
REGION = "RO99"


def census_doc(dim: str, codes: list[str], suppress=()):
    """One region. Per sex and age band from 15 on: 10 persons, of whom 6 are
    employed: 4 employees and 2 own-account workers, all in the first category."""
    dims = {"geo": [REGION], "sex": ["F", "M"], "age": nc.AGE_CODES, dim: ["TOTAL", *codes, "NAP", "UNK"],
            "wstatus": ["TOTAL", *nj.KIND_CODES, "NAP", "UNK"]}
    values = {}
    for sex in "FM":
        for band, a in enumerate(nc.AGE_CODES):
            adult = band >= 3
            values[(REGION, sex, a, "TOTAL", "TOTAL")] = 10
            values[(REGION, sex, a, "NAP", "TOTAL")] = 4 if adult else 10
            for code in codes:
                for kind in nj.KIND_CODES:
                    first = adult and code == codes[0]
                    values[(REGION, sex, a, code, kind)] = (4 if kind == "SAL" else 2 if kind == "SELF_NS" else 0) if first else 0
    for key in suppress:
        del values[key]
    return jsonstat(dims, values)


def lfs_doc(row_dim: str, rows: list[str], col_dim: str, cols: list[str], value=1.5):
    dims = {"age": ["Y_GE15", "Y20-64"], "sex": ["T", "M", "F"], row_dim: ["TOTAL", *rows], col_dim: ["TOTAL", *cols]}
    values = {("Y_GE15", s, r, c): value for s in "FM" for r in rows for c in cols}
    return jsonstat(dims, values)


def small_docs(**kw):
    sections = [x for group in nj.GROUP_SECTIONS for x in group]
    return (census_doc("nace_r2", nj.GROUP_CODES, **kw), census_doc("isco08", nj.OCCUPATION_CODES),
            lfs_doc("isco08", nj.OCCUPATION_CODES, "nace_r2", sections),
            lfs_doc("isced11", nj.EDU_GROUP_CODES, "isco08", nj.OCCUPATION_CODES))


class TestNormaliseJobs(unittest.TestCase):
    def test_tables_are_read_by_kind_and_category(self):
        m = nj.build(*small_docs())
        self.assertEqual(m["regions"], [REGION])
        self.assertEqual(m["by_industry_group"][0][0][2], [[0] * 10] * 4)                      # under 15: nobody
        self.assertEqual(m["by_industry_group"][0][1][5][0], [4] + [0] * 9)                    # employees, agriculture
        self.assertEqual(m["by_occupation"][0][0][5][2], [2] + [0] * 9)                # own-account, group 0
        self.assertEqual(nj.totals(m)["kind"], {"employee": 144, "employer": 0, "own_account": 72, "family_worker": 0})

    def test_patterns_are_persons_and_sections_are_grouped(self):
        m = nj.build(*small_docs())
        self.assertEqual(m["pattern_edu_occupation"][0][2], [1500] * 10)
        # Industry groups add their NACE sections: A is one section, B-E four, R-U four.
        self.assertEqual(m["pattern_occupation_industry_group"][1][4], [1500, 6000, 1500, 4500, 1500, 1500, 1500, 3000, 4500, 6000])
        self.assertEqual(m["edu_group_of_level"], [0, 1, 1, 2, 2])

    def test_suppressed_cells_get_what_the_employed_total_leaves_over(self):
        gone = [(REGION, "F", "Y40-44", "A", "SAL"), (REGION, "F", "Y40-44", "A", "SELF_S")]
        m = nj.build(*small_docs(suppress=gone))
        self.assertEqual(m["by_industry_group"][0][0][8][0][0] + m["by_industry_group"][0][0][8][1][0], 4)
        self.assertEqual(sum(sum(row) for row in m["by_industry_group"][0][0][8]), 6)

    def test_tables_must_hold_the_employed_of_the_activity_table(self):
        m = nj.build(*small_docs())
        years = [[0, 0, 0, 0, 0, 0]] * 15 + [[0, 0, 6, 0, 4, 0] if y % 5 == 0 else [0] * 6 for y in range(15, 101)]
        activity = {"regions": [REGION], "activities": ref.MARGIN_ACTIVITIES, "activity": [[years, years]]}
        nj.check_against_activity(m, activity)
        far = [[0, 0, 60, 0, 0, 0] if row[2] else row for row in years]
        with self.assertRaises(nc.CensusError):
            nj.check_against_activity(m, dict(activity, activity=[[far, far]]))

    def test_an_empty_pattern_is_rejected(self):
        docs = list(small_docs())
        docs[3] = lfs_doc("isced11", nj.EDU_GROUP_CODES, "isco08", nj.OCCUPATION_CODES, value=0)
        with self.assertRaises(nc.CensusError):
            nj.build(*docs)

    def test_committed_fixture_matches_the_census_and_the_activity_fixture(self):
        m = json.loads(FIXTURE.read_text(encoding="utf-8"))
        self.assertEqual(nj.totals(m), m["totals"])
        for name in ("kind", "industry_group", "occupation"):
            self.assertEqual(sum(m["totals"][name].values()), 7_689_171, name)
        # Published national totals; the fill of suppressed cells moves a few dozen persons.
        for name, published in (("employee", 6_545_255), ("employer", 101_712), ("own_account", 810_791),
                                ("family_worker", 231_413)):
            self.assertLessEqual(abs(m["totals"]["kind"][name] - published), 100, name)
        self.assertEqual(m["totals"]["occupation"]["armed_forces"], 0)
        nj.check_against_activity(m, json.loads(ACTIVITY_FIXTURE.read_text(encoding="utf-8")))


class TestDealAndBalance(unittest.TestCase):
    def test_deal_follows_the_targets(self):
        carry = [0, 0, 0]
        got = ref.deal([10] * 10, [50, 30, 20], carry)
        self.assertEqual([got.count(k) for k in range(3)], [5, 3, 2])
        self.assertEqual(carry, [0, 0, 0])

    def test_single_person_groups_do_not_starve_a_small_category(self):
        carry = [0, 0]
        got = [ref.deal([10], [9, 1], carry)[0] for _ in range(40)]
        self.assertEqual(got.count(1), 4)                     # one in ten, however small the groups
        self.assertLessEqual(max(abs(c) for c in carry), 10)

    def test_a_zero_target_never_receives_anyone(self):
        carry = [-30, 30, 0]
        # Owed -15, 30, 15: the second category is owed the most but is closed.
        self.assertEqual(ref.deal([10] * 3, [5, 0, 5], carry), [2, 2, 2])
        self.assertEqual(carry[1], 30)

    def test_balance_keeps_rows_exact_and_follows_the_pattern(self):
        pattern = [[9, 1], [1, 9]]
        t = ref.balance(pattern, [100, 300], [200, 200])
        self.assertEqual([sum(row) for row in t], [100, 300])
        self.assertLessEqual(abs(t[0][0] + t[1][0] - 200), 1)
        self.assertGreater(t[0][0], t[0][1])                  # the first row still leans to the first column
        self.assertGreater(t[1][1], t[1][0])
        self.assertEqual(ref.balance(pattern, [0, 40], [30, 10]), [[0, 0], [30, 10]])


class TestJobsReference(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.m = json.loads(COUNTY_FIXTURE.read_text(encoding="utf-8"))
        cls.ea = json.loads(ACTIVITY_FIXTURE.read_text(encoding="utf-8"))
        cls.jobs = json.loads(FIXTURE.read_text(encoding="utf-8"))
        cls.pop = ref.generate(cls.m, 1000)
        cls.attrs = ref.assign_attributes(cls.pop, cls.m["counties"], cls.ea)
        cls.before = (ref.state_hash(cls.pop), ref.attributes_hash(cls.attrs))
        cls.ja = ref.assign_jobs(cls.pop, cls.m["counties"], cls.ea, cls.jobs, cls.attrs)

    def test_structure(self):
        ja = self.ja
        for act, st, occ, sec in zip(self.attrs["activity"], ja["employment_status"], ja["occupation"], ja["industry_group"]):
            if act == ref.ACT_EMPLOYED:
                self.assertIn(st, (1, 2, 3, 4))
                self.assertTrue(0 <= occ <= 9 and 0 <= sec <= 9)
            else:
                self.assertEqual((st, occ, sec), (ref.KIND_NONE, ref.NOT_EMPLOYED, ref.NOT_EMPLOYED))

    def test_earlier_stages_are_not_changed(self):
        self.assertEqual((ref.state_hash(self.pop), ref.attributes_hash(self.attrs)), self.before)
        self.assertEqual(f"{self.before[1]:016x}", "8a0509e8ebc90647")

    def test_hash_is_the_one_recorded_in_the_spec(self):
        self.assertEqual(f"{ref.jobs_hash(self.ja):016x}", "7e3499aa1c44e7f1")

    def test_fit_is_within_the_spec_tolerances(self):
        fit = ref.job_fit_errors(self.pop, self.m["counties"], self.ea, self.jobs, self.attrs, self.ja, 1000)
        limits = {">=1000 records": 0.01, ">=100 records": 0.10, ">=30 records": 0.25}
        for family, buckets in fit.items():
            for bucket, error in buckets.items():
                self.assertLessEqual(error, limits[bucket], f"{family} {bucket}")

    def test_links_follow_the_patterns(self):
        tot = ref.job_totals(self.pop, self.m["counties"], self.ea, self.jobs, self.attrs, self.ja)

        def share(select, among) -> float:
            base = sum(v for k, v in tot.items() if among(k))
            return sum(v for k, v in tot.items() if among(k) and select(k)) / base

        # key: (region, sex, band, kind, occupation, industry group, edu_level)
        self.assertGreater(share(lambda k: k[6] >= 3, lambda k: k[4] == 2), 0.70)     # professionals with tertiary
        self.assertLess(share(lambda k: k[6] >= 3, lambda k: k[4] == 9), 0.20)        # elementary with tertiary
        self.assertGreater(share(lambda k: k[5] == 0, lambda k: k[4] == 6), 0.60)     # skilled agricultural in agriculture
        by_industry_group = [share(lambda k, s=s: k[5] == s, lambda k: k[4] == 2) for s in range(10)]
        self.assertEqual(by_industry_group.index(max(by_industry_group)), 8)                          # professionals: public, education, health

    def test_another_seed_changes_persons_not_totals(self):
        other = ref.assign_jobs(self.pop, self.m["counties"], self.ea, self.jobs, self.attrs, rng_seed=7)
        self.assertNotEqual(other, self.ja)
        fit = ref.job_fit_errors(self.pop, self.m["counties"], self.ea, self.jobs, self.attrs, other, 1000)
        for family in ("national kind", "national industry group", "national occupation"):
            self.assertLessEqual(fit[family][">=1000 records"], 0.01, family)

    def test_bad_inputs_are_rejected(self):
        counties = self.m["counties"]
        short = json.loads(json.dumps(self.jobs))
        short["by_industry_group"][0][0][5][0].pop()
        with self.assertRaisesRegex(ref.GenError, "ShapeMismatch"):
            ref.assign_jobs(self.pop, counties, self.ea, short, self.attrs)
        with self.assertRaisesRegex(ref.GenError, "EmptyTable"):
            ref.assign_jobs(self.pop, counties, self.ea, dict(self.jobs, regions=[]), self.attrs)
        fewer = {"edu_level": self.attrs["edu_level"][1:], "activity": self.attrs["activity"][1:]}
        with self.assertRaisesRegex(ref.GenError, "ShapeMismatch"):
            ref.assign_jobs(self.pop, counties, self.ea, self.jobs, fewer)
        nobody = json.loads(json.dumps(self.jobs))
        for name in ("by_industry_group", "by_occupation"):
            nobody[name][0][0] = [[[0] * 10 for _ in range(4)] for _ in range(21)]
        with self.assertRaisesRegex(ref.GenError, "NoMargin"):
            ref.assign_jobs(self.pop, counties, self.ea, nobody, self.attrs)


if __name__ == "__main__":
    unittest.main()
