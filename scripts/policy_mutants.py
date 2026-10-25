#!/usr/bin/env python3
"""The policy suite's mutation check (spec game/policy-scenarios, "The mutation
check", AC-POL-11; owner's delegation of 2026-10-25).

Runs cargo-mutants on each named sign of the mutation table against its
scenario's test at 1:1000, in a release build, and fails unless every mutant is
caught (cargo-mutants' own baseline run is the unmutated build passing):

  MUT-1  fx_step's interest-parity term (the unary minus before gamma1)   -> ac_pol_01
  MUT-2  the purchaser price's VAT factor, (p + d)(1 + vat) as /           -> ac_pol_03
  MUT-3  the monetary-financing push added to expected inflation (+= as -=) -> ac_pol_05

Needs `cargo install cargo-mutants`. Mutates the tree in place (restored
after each mutant), so run it on a clean checkout with nothing else building;
on Windows the restored file may come back with LF line endings (`git checkout`
it). Results are in target/policy-mutants/mutants.out.

Usage:
    python scripts/policy_mutants.py            # all three
    python scripts/policy_mutants.py MUT-1      # one
"""
import re
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CLOSE = ROOT / "crates" / "econ-core" / "src" / "world" / "close.rs"
PUSH = "self.mac.expected_inflation_ppm += self.cb_next_push();"


def push_line() -> int:
    """The line of the monetary-financing push in `monthly_close`."""
    for n, line in enumerate(CLOSE.read_text(encoding="utf-8").splitlines(), 1):
        if PUSH in line:
            return n
    sys.exit(f"policy-mutants: `{PUSH}` is not in {CLOSE.relative_to(ROOT)}")


def mutants() -> list[tuple[str, str, str, str, str]]:
    """(name, file, regex, lever feature, test) of each mutant."""
    return [
        (
            "MUT-1",
            "crates/econ-mech-trade/src/lib.rs",
            r"delete - in fx_step$",
            "lever_policy_rate",
            "policy::rate_hike::ac_pol_01",
        ),
        (
            "MUT-2",
            "crates/econ-core/src/world/revenue.rs",
            r"replace \* with / in World::raw_consumer_price$",
            "policy",
            "policy::vat_rise::ac_pol_03",
        ),
        (
            "MUT-3",
            "crates/econ-core/src/world/close.rs",
            rf"close\.rs:{push_line()}:\d+: replace \+= with -= in World::monthly_close$",
            "lever_monetary_financing",
            "policy::monetary_financing::ac_pol_05",
        ),
    ]


def cargo_mutants(file: str, regex: str, feature: str, test: str, list_only: bool) -> subprocess.CompletedProcess:
    cmd = [
        "cargo", "mutants", "--in-place", "--no-shuffle",
        "--file", file, "--re", regex,
        "--profile", "release",
        "--output", str(ROOT / "target" / "policy-mutants"),
    ]
    if list_only:
        cmd.append("--list")
    else:
        # The test package and its features go to cargo test: cargo-mutants
        # tests the mutated package and would apply `--features` to it
        # (econ-mech-trade has no `econ-core/...` feature).
        cmd += [
            "--", "--package", "econ-core", "--features", f"econ-core/{feature}",
            "--test", "acceptance",
            "--", "--exact", "--test-threads=1", test,
        ]
    return subprocess.run(cmd, cwd=ROOT, capture_output=list_only, text=True)


def main() -> int:
    if shutil.which("cargo-mutants") is None:
        print("policy-mutants: needs cargo-mutants (`cargo install cargo-mutants`)")
        return 1
    wanted = set(sys.argv[1:])
    failed = []
    for name, file, regex, feature, test in mutants():
        if wanted and name not in wanted:
            continue
        listed = cargo_mutants(file, regex, feature, test, list_only=True)
        found = [l for l in listed.stdout.splitlines() if re.search(regex, l)]
        if len(found) != 1:
            print(f"{name}: expected exactly one mutant for /{regex}/ in {file}, found {len(found)}:")
            print(listed.stdout + listed.stderr)
            failed.append(f"{name}: not exactly one mutant")
            continue
        print(f"{name}: {found[0]} against {test}", flush=True)
        r = cargo_mutants(file, regex, feature, test, list_only=False)
        # cargo-mutants: 0 every mutant caught, 2 one missed, 3 a timeout,
        # 4 the unmutated baseline failed.
        verdict = {0: "caught", 2: "MISSED", 3: "TIMEOUT", 4: "BASELINE FAILS"}.get(
            r.returncode, f"exit {r.returncode}"
        )
        print(f"{name}: {verdict}", flush=True)
        if r.returncode != 0:
            failed.append(f"{name}: {verdict}")
    if failed:
        print("policy-mutants: " + "; ".join(failed))
        return 1
    print("policy-mutants: every sign mutant is caught")
    return 0


if __name__ == "__main__":
    sys.exit(main())
