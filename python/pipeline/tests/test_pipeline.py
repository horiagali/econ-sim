import json
import sys
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(HERE))

from ipf import integerise, ipf  # noqa: E402
from jsonstat import to_rows  # noqa: E402


class TestIPF(unittest.TestCase):
    def test_fits_two_margins(self):
        recs = [{"a": x, "b": y} for x in "PQ" for y in "XYZ" for _ in range(3)]
        w, it = ipf(recs, [1.0] * len(recs), {"a": {"P": 60, "Q": 40}, "b": {"X": 50, "Y": 30, "Z": 20}})
        for cat, tgt in {"P": 60, "Q": 40}.items():
            self.assertAlmostEqual(sum(wi for r, wi in zip(recs, w) if r["a"] == cat), tgt, places=6)
        for cat, tgt in {"X": 50, "Y": 30, "Z": 20}.items():
            self.assertAlmostEqual(sum(wi for r, wi in zip(recs, w) if r["b"] == cat), tgt, places=6)
        self.assertLess(it, 200)

    def test_integerise_keeps_total(self):
        ws = [1.4, 2.6, 3.5, 0.5]
        self.assertEqual(sum(integerise(ws)), round(sum(ws)))


class TestJsonStat(unittest.TestCase):
    def test_flatten(self):
        doc = json.loads((HERE / "fixtures" / "jsonstat_sample.json").read_text())
        rows = to_rows(doc)
        self.assertEqual(len(rows), 4)
        self.assertIn({"geo": "RO113", "time": "2022", "value": 20}, rows)


class TestFetchRetry(unittest.TestCase):
    def test_windows_primes_root_store_then_retries(self):
        import ssl
        import urllib.error
        from unittest import mock

        import fetch_eurostat

        cert_error = urllib.error.URLError(ssl.SSLCertVerificationError("unable to get local issuer certificate"))
        with mock.patch.object(fetch_eurostat, "_read", side_effect=[cert_error, b"{}"]) as read, \
                mock.patch.object(fetch_eurostat.subprocess, "run") as run, \
                mock.patch.object(fetch_eurostat.sys, "platform", "win32"):
            self.assertEqual(fetch_eurostat.download("https://example.invalid/x"), b"{}")
        self.assertEqual(read.call_count, 2)
        self.assertEqual(run.call_args.args[0][0], "curl.exe")

    def test_other_platforms_do_not_retry(self):
        import ssl
        import urllib.error
        from unittest import mock

        import fetch_eurostat

        cert_error = urllib.error.URLError(ssl.SSLCertVerificationError("unable to get local issuer certificate"))
        with mock.patch.object(fetch_eurostat, "_read", side_effect=cert_error), \
                mock.patch.object(fetch_eurostat.sys, "platform", "linux"):
            with self.assertRaises(urllib.error.URLError):
                fetch_eurostat.download("https://example.invalid/x")


class TestScaleIsAParameter(unittest.TestCase):
    def test_total_weight_independent_of_scale(self):
        import subprocess
        totals = []
        for scale in (1000, 200):
            out = subprocess.run([sys.executable, str(HERE / "synth_counties.py"), "--fixture", "--scale", str(scale)],
                                 capture_output=True, text=True, check=True).stdout
            totals.append(int(out.split("total weight ")[1].split(",")[0]))
        self.assertEqual(totals[0], totals[1])


if __name__ == "__main__":
    unittest.main()
