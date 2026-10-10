"""Census normalisation and the population-generator reference (spec society/population-generator)."""
import json
import sys
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(HERE.parent / "reference"))

import normalise_census as nc  # noqa: E402
import popgen_reference as ref  # noqa: E402

FIXTURE = HERE / "fixtures" / "census2021_margins_ro.json"


def jsonstat(dims: dict, values: dict) -> dict:
    """A JSON-stat document from {dim: [codes]} and {(code, ...): value}."""
    ids = list(dims)
    size = [len(dims[d]) for d in ids]
    doc = {"id": ids, "size": size, "value": {},
           "dimension": {d: {"category": {"index": {c: i for i, c in enumerate(dims[d])}}} for d in ids}}
    for key, v in values.items():
        flat = 0
        for d, code in zip(ids, key):
            flat = flat * len(dims[d]) + dims[d].index(code)
        doc["value"][str(flat)] = v
    return doc


def small_census(suppress=(), households=(22, 30, 20, 15, 5, 7, 1)):
    """One county, 300 persons of whom 6 are not in private households."""
    dims = {"geo": ["RO999"], "hhstatus": ["TOTAL", "PRV", "NPRV"], "sex": ["F", "M"], "age": ["TOTAL", *nc.AGE_CODES]}
    values = {}
    for sex in "FM":
        prv = [7] * 21
        npr = [0] * 21
        npr[4] = 2
        npr[19] = 1
        for i, a in enumerate(nc.AGE_CODES):
            values[("RO999", "PRV", sex, a)] = prv[i]
            values[("RO999", "NPRV", sex, a)] = npr[i]
            values[("RO999", "TOTAL", sex, a)] = prv[i] + npr[i]
        values[("RO999", "PRV", sex, "TOTAL")] = sum(prv)
        values[("RO999", "NPRV", sex, "TOTAL")] = sum(npr)
        values[("RO999", "TOTAL", sex, "TOTAL")] = sum(prv) + sum(npr)
    for key in suppress:
        del values[key]
    hdims = {"geo": ["RO999"], "n_person": ["TOTAL", *nc.SIZE_CODES]}
    hvalues = {("RO999", k): v for k, v in zip(nc.SIZE_CODES, households)}
    hvalues[("RO999", "TOTAL")] = sum(households)
    return jsonstat(dims, values), jsonstat(hdims, hvalues)


class TestNormaliseCensus(unittest.TestCase):
    def test_complete_tables_pass_through(self):
        m = nc.build(*small_census())
        self.assertEqual(nc.totals(m), {"persons_private": 294, "persons_collective": 6, "households": 100})
        self.assertEqual(m["persons_collective"][0][0][4], 2)

    def test_suppressed_not_private_cell_is_total_minus_private(self):
        m = nc.build(*small_census(suppress=[("RO999", "NPRV", "F", "Y20-24")]))
        self.assertEqual(m["persons_collective"][0][0][4], 2)

    def test_cell_suppressed_everywhere_gets_the_county_residual(self):
        gone = [("RO999", s, "F", "Y95-99") for s in ("NPRV", "TOTAL")]
        m = nc.build(*small_census(suppress=gone))
        self.assertEqual(m["persons_collective"][0][0][19], 1)
        self.assertEqual(nc.totals(m)["persons_collective"], 6)

    def test_persons_that_cannot_fit_the_households_are_rejected(self):
        with self.assertRaises(nc.CensusError):
            nc.build(*small_census(households=(100, 0, 0, 0, 0, 0, 0)))

    def test_size_classes_must_sum_to_the_total(self):
        persons, hh = small_census()
        hh["value"]["0"] = 99
        with self.assertRaises(nc.CensusError):
            nc.build(persons, hh)

    def test_committed_fixture_has_the_census_totals(self):
        m = json.loads(FIXTURE.read_text(encoding="utf-8"))
        self.assertEqual(len(m["counties"]), 42)
        self.assertEqual(m["totals"], {"persons_private": 18_935_010, "persons_collective": 118_805, "households": 7_709_139})
        self.assertEqual(nc.totals(m), m["totals"])
        nc.check_consistent(m["counties"], dict(zip(m["counties"], [dict(zip(nc.SEXES, c)) for c in m["persons_private"]])),
                            dict(zip(m["counties"], m["households"])), 15)


class TestPopgenReference(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.m = json.loads(FIXTURE.read_text(encoding="utf-8"))
        cls.pop = ref.generate(cls.m, 2000)

    def test_chacha8_port_matches_the_rust_known_answers(self):
        ref.check_rng()

    def test_county_totals_are_exact(self):
        fit = ref.fit_errors(self.m, self.pop, 2000)
        self.assertTrue(fit["exact_county_totals"])
        weighted_persons = sum(self.pop["hh_weight"][h] for h in self.pop["household_id"])
        self.assertEqual(weighted_persons, 19_053_815)

    def test_large_cells_are_within_one_percent(self):
        fit = ref.fit_errors(self.m, self.pop, 2000)
        for family, buckets in fit["max_relative_error"].items():
            self.assertLessEqual(buckets.get(">=1000 records", 0.0), 0.01, family)

    def test_structure(self):
        pop = self.pop
        members: dict = {}
        for i, h in enumerate(pop["household_id"]):
            members.setdefault(h, []).append(i)
        self.assertEqual(sorted(members), list(range(len(pop["hh_weight"]))))
        self.assertTrue(all(w >= 1 for w in pop["hh_weight"]))
        for h, idx in members.items():
            self.assertEqual(sum(1 for i in idx if pop["role"][i] == ref.ROLE_HEAD), 1)
            if pop["hh_collective"][h]:
                self.assertEqual(len(idx), 1)
            elif any(pop["age"][i] < 15 * 12 for i in idx):
                self.assertTrue(any(pop["age"][i] >= 20 * 12 for i in idx))

    def test_deterministic_and_seed_changes_only_the_assembly(self):
        self.assertEqual(ref.state_hash(ref.generate(self.m, 2000)), ref.state_hash(self.pop))
        other = ref.generate(self.m, 2000, rng_seed=7)
        self.assertNotEqual(ref.state_hash(other), ref.state_hash(self.pop))
        self.assertTrue(ref.fit_errors(self.m, other, 2000)["exact_county_totals"])

    def test_bad_input_is_rejected(self):
        with self.assertRaises(ref.GenError):
            ref.generate(self.m, 0)
        broken = json.loads(json.dumps(self.m))
        broken["households"][0] = [1, 0, 0, 0, 0, 0, 0]
        with self.assertRaises(ref.GenError):
            ref.generate(broken, 2000)


if __name__ == "__main__":
    unittest.main()
