"""The demographic tables and the linked emigrants of the starting state (specs economy/demographics and
economy/migration-remittances): the normalisers on small inputs and the committed fixtures."""
import json
import sys
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(Path(__file__).resolve().parent))

import normalise_demography as nd  # noqa: E402
import normalise_linked_emigrants as nl  # noqa: E402
from normalise_census import CensusError  # noqa: E402
from test_population import jsonstat  # noqa: E402

FIXTURES = HERE / "fixtures"


def mortality_doc(q=0.01):
    ages = ["Y_LT1"] + [f"Y{a}" for a in range(1, 95)] + ["Y_GE95"]
    values = {}
    for sex in nd.SEXES:
        for t in nd.BASE_YEARS:
            for k, age in enumerate(ages):
                values[("PROBDEATH", sex, age, t)] = 1.0 if age == "Y_GE95" else min(0.5, q * (1 + k / 10))
    return jsonstat({"indic_de": ["PROBDEATH"], "sex": nd.SEXES, "age": ages, "time": nd.BASE_YEARS}, values)


def fertility_doc():
    ages = ["Y10-14"] + [f"Y{a}" for a in range(15, 50)] + ["Y_GE50"]
    values = {("2019", a): (0.001 if a in ("Y10-14", "Y_GE50") else 0.05) for a in ages}
    return jsonstat({"time": ["2019"], "age": ages}, values)


def emigrants_doc():
    ages = [f"Y{a}" for a in range(0, 80)]
    values = {(s, a, "2021"): 100 + int(a[1:]) for s in nl.SEXES for a in ages}
    return jsonstat({"sex": nl.SEXES, "age": ages, "time": ["2021"]}, values)


class TestNormaliseDemography(unittest.TestCase):
    def test_tables_have_their_shape_and_rules(self):
        m = nd.build({nd.MORTALITY_ID: mortality_doc(), nd.FERTILITY_ID: fertility_doc()})
        for sex in range(2):
            self.assertEqual(len(m["mortality"]["yearly_ppb"][sex]), nd.AGES)
            self.assertEqual(len(m["mortality"]["monthly_ppb"][sex]), nd.AGES)
            # The monthly rule m = 1 - (1 - q)^(1/12), in parts per billion.
            q = m["mortality"]["yearly_ppb"][sex][40] / nd.PPB
            self.assertEqual(m["mortality"]["monthly_ppb"][sex][40], round((1 - (1 - q) ** (1 / 12)) * nd.PPB))
            # Extended above 94: below one and rising.
            tail = m["mortality"]["yearly_ppb"][sex][nd.LAST_MEASURED:]
            self.assertTrue(all(a < b for a, b in zip(tail, tail[1:])))
            self.assertLess(tail[-1], nd.PPB)
        # The tails of fertility are folded into 15 and 49, so the total is kept.
        rates = m["fertility"]["rates_ppm"]
        self.assertEqual(len(rates), 35)
        self.assertEqual(rates[0], 55_000)
        self.assertEqual(rates[-1], 51_000)
        self.assertAlmostEqual(m["totals"]["tfr"], 33 * 0.05 + 0.055 + 0.051, places=4)

    def test_missing_cells_are_refused(self):
        doc = fertility_doc()
        doc["value"].pop(str(doc["dimension"]["age"]["category"]["index"]["Y30"]))
        with self.assertRaises(CensusError):
            nd.build({nd.MORTALITY_ID: mortality_doc(), nd.FERTILITY_ID: doc})

    def test_retirement_schedule_rises_one_month_at_a_time(self):
        s = nd.retirement_schedule()
        women = s["F"]
        self.assertEqual(women[0], ["2021-12-01", 741])
        self.assertEqual(women[-1], ["2030-01-01", 756])
        self.assertEqual([m for _, m in women], list(range(741, 757)))
        self.assertEqual([d for d, _ in women], sorted(d for d, _ in women))
        self.assertEqual(s["M"], [["2021-12-01", 780]])

    def test_fixture_is_pre_pandemic_and_checks_out(self):
        m = json.loads((FIXTURES / "demography2019_ro.json").read_text(encoding="utf-8"))
        self.assertEqual(m["mortality"]["base_years"], [2017, 2018, 2019])
        self.assertEqual(m["fertility"]["year"], 2019)
        # TFR 1.77 (2019) and life expectancy 79.1 (women) and 71.7 (men) from the 2017-2019 table.
        self.assertAlmostEqual(m["totals"]["tfr"], 1.77, delta=0.01)
        f, mm = m["totals"]["life_expectancy_at_birth"]
        self.assertAlmostEqual(f, 79.1, delta=0.2)
        self.assertAlmostEqual(mm, 71.7, delta=0.2)
        self.assertEqual(m["sex_ratio_at_birth_ppm"], 514_000)


class TestNormaliseLinkedEmigrants(unittest.TestCase):
    def test_departure_ages_are_single_years_18_to_69(self):
        m = nl.build({nl.EMIGRANTS_ID: emigrants_doc()})
        self.assertEqual(m["start_year"], 2021)
        for row in m["departure_age_weights"]:
            self.assertEqual(len(row), 52)
            self.assertEqual(row[0], 118)
            self.assertEqual(row[-1], 169)
        self.assertEqual(sum(m["edu_shares_ppm"]), 1_000_000)

    def test_fixture_matches_2021_emigrants(self):
        m = json.loads((FIXTURES / "linked_emigrants2021_ro.json").read_text(encoding="utf-8"))
        self.assertEqual(m["linked_households"], 1_200_000)
        self.assertEqual(m["diaspora_total"], 4_000_000)
        # Eurostat migr_emi2, 2021: men are about 60% of the emigrants aged 18 to 69.
        women, men = (sum(row) for row in m["departure_age_weights"])
        self.assertGreater(men, women)
        self.assertEqual([r for r, _ in m["region_factor_ppm"]],
                         ["RO11", "RO12", "RO21", "RO22", "RO31", "RO32", "RO41", "RO42"])


if __name__ == "__main__":
    unittest.main()
