"""Employers: the reference stage (spec society/population-employers)."""
import json
import sys
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(HERE.parent / "reference"))

import popgen_reference as ref  # noqa: E402

F = HERE / "fixtures"

SMALL_FIRMS = {
    "sizes": ["micro", "small", "large", "public"],
    "industries": [{"id": "farms", "industry_group": 0}, {"id": "mills", "industry_group": 1}],
    "units": [
        {"id": "farms.micro.domestic", "industry": "farms", "kind": "cohort", "size": "micro", "controller": "domestic",
         "persons_employed": 10, "wages": 100},
        {"id": "mills.large.foreign", "industry": "mills", "kind": "cohort", "size": "large", "controller": "foreign",
         "persons_employed": 30, "wages": 900},                         # pays 30 a head
        {"id": "mills.micro.domestic", "industry": "mills", "kind": "cohort", "size": "micro", "controller": "domestic",
         "persons_employed": 10, "wages": 100},                         # pays 10 a head
    ],
}


def small_population():
    """Eight persons of weight 1: four mill employees with wages 1 to 4, one mill own-account worker,
    one farm employee, two persons without a job."""
    status = [1, 1, 1, 1, 3, 1, 0, 0]
    group = [1, 1, 1, 1, 1, 0, 255, 255]
    wage = [100, 200, 300, 400, 0, 50, 0, 0]
    pop = {"hh_weight": [1] * 8, "household_id": list(range(8))}
    return pop, {"employment_status": status, "industry_group": group}, wage


class TestEmployerReference(unittest.TestCase):
    def test_better_paid_employees_are_in_better_paying_units(self):
        pop, ja, wage = small_population()
        e = ref.assign_employers(pop, SMALL_FIRMS, ja, wage, 42, wage_link_ppm=1_000_000)
        # The micro mill has a quarter of the mills' persons: the lowest-paid of the four employees.
        self.assertEqual(e["employer"][:4], [2, 1, 1, 1])
        self.assertEqual(e["industry"][:4], [1, 1, 1, 1])
        self.assertEqual((e["employer"][4], e["industry"][4]), (2, 1))          # own-account: the domestic micro unit
        self.assertEqual((e["employer"][5], e["industry"][5]), (0, 0))
        self.assertEqual(e["employer"][6:], [ref.NO_EMPLOYER] * 2)
        self.assertEqual(e["industry"][6:], [ref.NO_INDUSTRY] * 2)

    def test_bad_inputs_are_rejected(self):
        pop, ja, wage = small_population()
        with self.assertRaisesRegex(ref.GenError, "ShapeMismatch"):
            ref.assign_employers(pop, SMALL_FIRMS, ja, wage[:-1])
        with self.assertRaisesRegex(ref.GenError, "EmptyTable"):
            ref.assign_employers(pop, dict(SMALL_FIRMS, units=[]), ja, wage)
        no_farms = dict(SMALL_FIRMS, units=SMALL_FIRMS["units"][1:])
        with self.assertRaisesRegex(ref.GenError, "NoMargin"):                  # a farm employee and no farm unit
            ref.assign_employers(pop, no_farms, ja, wage)
        stray = dict(SMALL_FIRMS, units=[dict(SMALL_FIRMS["units"][0], industry="mines")])
        with self.assertRaisesRegex(ref.GenError, "ShapeMismatch"):
            ref.assign_employers(pop, stray, ja, wage)

    def test_fixture_population(self):
        load = lambda name: json.loads((F / name).read_text(encoding="utf-8"))  # noqa: E731
        m, ea, jobs = load("census2021_margins_ro.json"), load("census2021_edu_activity_ro.json"), load("census2021_jobs_ro.json")
        firms = load("firm_units2021_ro.json")
        pop = ref.generate(m, 1000, 42)
        attrs = ref.assign_attributes(pop, m["counties"], ea, 42)
        ja = ref.assign_jobs(pop, m["counties"], ea, jobs, attrs, 42)
        wage = ref.assign_wages(pop, load("earnings2021_ro.json"), attrs, ja, 42)
        e = ref.assign_employers(pop, firms, ja, wage, 42)
        self.assertEqual(f"{ref.employers_hash(e):016x}", "e94a4f492b24c1cb")
        self.assertEqual(e, ref.assign_employers(pop, firms, ja, wage, 42))
        self.assertNotEqual(e, ref.assign_employers(pop, firms, ja, wage, 7))
        group_of = {k: ind["industry_group"] for k, ind in enumerate(firms["industries"])}
        for i, status in enumerate(ja["employment_status"]):
            if status == 0:
                self.assertEqual((e["industry"][i], e["employer"][i]), (ref.NO_INDUSTRY, ref.NO_EMPLOYER))
                continue
            unit = firms["units"][e["employer"][i]]
            self.assertEqual(firms["industries"][e["industry"][i]]["id"], unit["industry"])
            self.assertEqual(group_of[e["industry"][i]], ja["industry_group"][i])   # the jobs stage's group is kept
            if status != 1:
                self.assertEqual(unit["size"], "micro")
        fit = ref.employer_fit(pop, firms, ja, e, wage)
        self.assertLess(fit["largest_gap_of_a_unit_share_in_its_group"], 0.03)
        pay = {size: v["mean_wage_lei"] for size, v in fit["employees_and_pay_by_size"].items()}
        self.assertLess(pay["micro"], pay["small"])
        self.assertLess(pay["small"], pay["large"])
        # Without wages the stage still runs.
        self.assertEqual(len(ref.assign_employers(pop, firms, ja, None, 42)["employer"]), len(wage))


if __name__ == "__main__":
    unittest.main()
