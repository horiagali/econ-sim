//! Acceptance tests for AC-POP-* (spec: society/population-generator).
//! Written in a test-authoring session from the spec and the Python reference,
//! before the Rust generator existed (ADR-0014).
//!
//! "The fixture" is the normalised Census 2021 margin file of Romania.

// Column tables are indexed in parallel; index loops are the clear way to read them.
#![allow(clippy::needless_range_loop)]

use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;

use econ_popgen::{GenError, GenParams, Margins, OPEN, Population, Role, Sex, generate};
use serde_json::Value;

const SEED: u64 = 42;
const SCALES: [u32; 3] = [1000, 100, 10];
/// Tolerance in percent by expected synthetic records behind a cell.
const TOLERANCES: [(u64, u64); 3] = [(1000, 1), (100, 4), (30, 5)];

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn fixture_path() -> PathBuf {
    repo_root().join("python/pipeline/fixtures/census2021_margins_ro.json")
}

fn as_u64(v: &Value) -> u64 {
    v.as_u64().expect("non-negative integer")
}

fn flatten(v: &Value, out: &mut Vec<u64>) {
    match v {
        Value::Array(items) => items.iter().for_each(|i| flatten(i, out)),
        other => out.push(as_u64(other)),
    }
}

fn pairs(v: &Value) -> Vec<(u16, u16)> {
    v.as_array()
        .expect("array of pairs")
        .iter()
        .map(|p| {
            let lo = u16::try_from(as_u64(&p[0])).expect("small");
            let hi = if p[1].is_null() {
                OPEN
            } else {
                u16::try_from(as_u64(&p[1])).expect("small")
            };
            (lo, hi)
        })
        .collect()
}

fn margins() -> &'static Margins {
    static M: OnceLock<Margins> = OnceLock::new();
    M.get_or_init(|| {
        let text = std::fs::read_to_string(fixture_path()).expect("margin fixture");
        let v: Value = serde_json::from_str(&text).expect("valid JSON");
        assert_eq!(v["sexes"], serde_json::json!(["F", "M"]), "sex order");
        let table = |name: &str| {
            let mut out = Vec::new();
            flatten(&v[name], &mut out);
            out
        };
        Margins {
            counties: v["counties"]
                .as_array()
                .expect("counties")
                .iter()
                .map(|c| c.as_str().expect("code").to_string())
                .collect(),
            age_bands: pairs(&v["age_bands"]),
            size_classes: pairs(&v["size_classes"]),
            persons_private: table("persons_private"),
            persons_collective: table("persons_collective"),
            households: table("households"),
        }
    })
}

/// The fixture population at one of the three scales (generated once).
fn population(scale: u32) -> &'static Population {
    static POPS: [OnceLock<Population>; 3] = [OnceLock::new(), OnceLock::new(), OnceLock::new()];
    let i = SCALES
        .iter()
        .position(|&s| s == scale)
        .expect("one of the three scales");
    POPS[i].get_or_init(|| {
        generate(margins(), &GenParams::new(scale, SEED)).expect("fixture generates")
    })
}

/// Weighted totals of a population, in the shape of the margin tables.
struct Totals {
    n_bands: usize,
    n_classes: usize,
    private: Vec<u64>,
    collective: Vec<u64>,
    households: Vec<u64>,
}

fn band_of(m: &Margins, age_months: u16) -> usize {
    let years = age_months / 12;
    m.age_bands
        .iter()
        .position(|&(lo, hi)| years >= lo && (hi == OPEN || years < hi))
        .expect("age inside a band")
}

fn sizes(pop: &Population) -> Vec<u32> {
    let mut size = vec![0u32; pop.households.hh_weight.len()];
    for &h in &pop.persons.household_id {
        size[h as usize] += 1;
    }
    size
}

fn totals(m: &Margins, pop: &Population) -> Totals {
    let (n_c, n_b, n_k) = (m.counties.len(), m.age_bands.len(), m.size_classes.len());
    let mut t = Totals {
        n_bands: n_b,
        n_classes: n_k,
        private: vec![0; n_c * 2 * n_b],
        collective: vec![0; n_c * 2 * n_b],
        households: vec![0; n_c * n_k],
    };
    let size = sizes(pop);
    let hh = &pop.households;
    for (i, &h) in pop.persons.household_id.iter().enumerate() {
        let h = h as usize;
        let c = usize::from(hh.hh_county[h]);
        let cell = (c * 2 + pop.persons.sex[i] as usize) * n_b + band_of(m, pop.persons.age[i]);
        let w = u64::from(hh.hh_weight[h]);
        if hh.hh_collective[h] {
            t.collective[cell] += w;
        } else {
            t.private[cell] += w;
        }
    }
    for h in 0..hh.hh_weight.len() {
        if !hh.hh_collective[h] {
            let members = u16::try_from(size[h]).expect("household size");
            let k = m
                .size_classes
                .iter()
                .position(|&(lo, hi)| members >= lo && (hi == OPEN || members <= hi))
                .expect("size inside a class");
            t.households[usize::from(hh.hh_county[h]) * n_k + k] += u64::from(hh.hh_weight[h]);
        }
    }
    t
}

/// (weighted households, weighted private persons, weighted collective persons) per county.
fn county_totals(
    n_c: usize,
    n_b: usize,
    n_k: usize,
    private: &[u64],
    collective: &[u64],
    households: &[u64],
) -> Vec<(u64, u64, u64)> {
    (0..n_c)
        .map(|c| {
            (
                households[c * n_k..(c + 1) * n_k].iter().sum(),
                private[c * 2 * n_b..(c + 1) * 2 * n_b].iter().sum(),
                collective[c * 2 * n_b..(c + 1) * 2 * n_b].iter().sum(),
            )
        })
        .collect()
}

fn assert_exact_county_totals(m: &Margins, pop: &Population, what: &str) {
    let t = totals(m, pop);
    let n_c = m.counties.len();
    let got = county_totals(
        n_c,
        t.n_bands,
        t.n_classes,
        &t.private,
        &t.collective,
        &t.households,
    );
    let want = county_totals(
        n_c,
        t.n_bands,
        t.n_classes,
        &m.persons_private,
        &m.persons_collective,
        &m.households,
    );
    for c in 0..n_c {
        assert_eq!(
            got[c], want[c],
            "{what}: county {} (households, private persons, collective persons)",
            m.counties[c]
        );
    }
}

/// AC-POP-01 [unit] At each of the three scales the number of synthetic persons is within 0.5% of census persons ÷ `sample_scale`, and no table is sized by a constant.
#[test]
fn ac_pop_01() {
    let m = margins();
    let census: u64 = m.persons_private.iter().chain(&m.persons_collective).sum();
    let mut counts = Vec::new();
    for scale in SCALES {
        let pop = population(scale);
        let p = &pop.persons;
        let h = &pop.households;
        assert!(
            p.household_id.len() == p.age.len()
                && p.age.len() == p.sex.len()
                && p.sex.len() == p.role.len()
        );
        assert!(
            h.hh_weight.len() == h.hh_county.len() && h.hh_county.len() == h.hh_collective.len()
        );
        let represented = p.household_id.len() as u64 * u64::from(scale);
        assert!(
            represented.abs_diff(census) * 1000 <= census * 5,
            "1:{scale}: {} synthetic persons for {census} real ones",
            p.household_id.len()
        );
        counts.push(p.household_id.len());
    }
    // Table sizes follow the scale: ten times finer means about ten times more records.
    assert!(counts[0] < counts[1] && counts[1] < counts[2]);
    assert!(counts[1].abs_diff(counts[0] * 10) * 100 <= counts[1]);
    assert!(counts[2].abs_diff(counts[1] * 10) * 100 <= counts[2]);
}

/// AC-POP-02 [unit] At each of the three scales, in every county: Σ `hh_weight` over private households equals the census household count exactly; Σ `hh_weight` × members equals census persons in private households exactly; Σ weights of collective records equals census persons not in private households exactly. The national totals are therefore the same at every scale.
#[test]
fn ac_pop_02() {
    let m = margins();
    for scale in SCALES {
        assert_exact_county_totals(m, population(scale), &format!("1:{scale}"));
    }
}

fn check(family: &str, scale: u32, got: u64, want: u64, worst: &mut Vec<String>) {
    let records = want / u64::from(scale);
    let Some(&(_, pct)) = TOLERANCES
        .iter()
        .find(|&&(min_records, _)| records >= min_records)
    else {
        return; // fewer than 30 expected records: not checked
    };
    if got.abs_diff(want) * 100 > want * pct {
        worst.push(format!(
            "1:{scale} {family}: got {got}, census {want} (tolerance {pct}%)"
        ));
    }
}

/// AC-POP-03 [unit] At each of the three scales the weighted margins are within the tolerances of the table below.
#[test]
fn ac_pop_03() {
    let m = margins();
    let (n_c, n_b, n_k) = (m.counties.len(), m.age_bands.len(), m.size_classes.len());
    let broad_of = |b: usize| match m.age_bands[b].0 {
        0..=14 => 0,
        15..=64 => 1,
        _ => 2,
    };
    let mut failures = Vec::new();
    for scale in SCALES {
        let t = totals(m, population(scale));
        let cell = |table: &[u64], c: usize, s: usize, b: usize| table[(c * 2 + s) * n_b + b];
        let both = [&t.private[..], &m.persons_private[..]];
        // National margins.
        for s in 0..2 {
            let [g, w] = both.map(|tab| {
                (0..n_c)
                    .flat_map(|c| (0..n_b).map(move |b| (c, b)))
                    .map(|(c, b)| cell(tab, c, s, b))
                    .sum::<u64>()
            });
            check("national sex", scale, g, w, &mut failures);
        }
        for b in 0..n_b {
            let [g, w] = both.map(|tab| {
                (0..n_c)
                    .flat_map(|c| (0..2).map(move |s| (c, s)))
                    .map(|(c, s)| cell(tab, c, s, b))
                    .sum::<u64>()
            });
            check("national age band", scale, g, w, &mut failures);
        }
        for k in 0..n_k {
            let g: u64 = (0..n_c).map(|c| t.households[c * n_k + k]).sum();
            let w: u64 = (0..n_c).map(|c| m.households[c * n_k + k]).sum();
            check("national size class", scale, g, w, &mut failures);
        }
        // County margins.
        for c in 0..n_c {
            for s in 0..2 {
                let [g, w] = both.map(|tab| (0..n_b).map(|b| cell(tab, c, s, b)).sum::<u64>());
                check("county x sex", scale, g, w, &mut failures);
                for b in 0..n_b {
                    check(
                        "county x sex x age band",
                        scale,
                        cell(&t.private, c, s, b),
                        cell(&m.persons_private, c, s, b),
                        &mut failures,
                    );
                }
            }
            for broad in 0..3 {
                let [g, w] = both.map(|tab| {
                    (0..n_b)
                        .filter(|&b| broad_of(b) == broad)
                        .map(|b| cell(tab, c, 0, b) + cell(tab, c, 1, b))
                        .sum::<u64>()
                });
                check("county x broad age", scale, g, w, &mut failures);
            }
            for k in 0..n_k {
                check(
                    "county x size class",
                    scale,
                    t.households[c * n_k + k],
                    m.households[c * n_k + k],
                    &mut failures,
                );
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} margins outside tolerance:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

/// AC-POP-04 [unit] Structure: every person belongs to exactly one household; every weight is an integer ≥ 1; every private household's size is in the range of a size class; every private household has exactly one head, aged at least 15; no child under 15 lives in a household without a member aged 20 or more; collective records have exactly one member; ages are within their band.
#[test]
fn ac_pop_04() {
    let m = margins();
    let top_age_months = (m.age_bands.last().expect("bands").0 + 5) * 12;
    for scale in SCALES {
        let pop = population(scale);
        let (hh, p) = (&pop.households, &pop.persons);
        let n_hh = hh.hh_weight.len();
        let params = GenParams::new(scale, SEED);
        let mut members = vec![0u32; n_hh];
        let mut heads = vec![0u32; n_hh];
        let mut has_adult = vec![false; n_hh];
        let mut has_child = vec![false; n_hh];
        for i in 0..p.household_id.len() {
            let h = p.household_id[i] as usize;
            assert!(
                h < n_hh,
                "1:{scale}: person {i} points at household {h} of {n_hh}"
            );
            assert!(
                p.age[i] < top_age_months,
                "1:{scale}: person {i} is {} months old",
                p.age[i]
            );
            members[h] += 1;
            if p.role[i] == Role::Head {
                heads[h] += 1;
                assert!(
                    hh.hh_collective[h] || p.age[i] >= 15 * 12,
                    "1:{scale}: head of household {h} is under 15"
                );
            }
            has_adult[h] |= p.age[i] >= 20 * 12;
            has_child[h] |= p.age[i] < 15 * 12;
        }
        for h in 0..n_hh {
            assert!(
                hh.hh_weight[h] >= 1,
                "1:{scale}: household {h} has weight 0"
            );
            assert!(usize::from(hh.hh_county[h]) < m.counties.len());
            assert_eq!(
                heads[h], 1,
                "1:{scale}: household {h} has {} heads",
                heads[h]
            );
            if hh.hh_collective[h] {
                assert_eq!(members[h], 1, "1:{scale}: collective record {h}");
            } else {
                assert!(
                    (1..=u32::from(params.max_household_size)).contains(&members[h]),
                    "1:{scale}: household {h} has {} members",
                    members[h]
                );
                assert!(
                    !has_child[h] || has_adult[h],
                    "1:{scale}: household {h} has a child but no adult"
                );
            }
        }
    }
}

/// AC-POP-05 [unit] Same margins, scale and seed give identical tables (equal state hash). A different seed changes who lives with whom but none of the totals in AC-POP-02.
#[test]
fn ac_pop_05() {
    let m = margins();
    let first = population(1000);
    let again = generate(m, &GenParams::new(1000, SEED)).expect("generates");
    assert_eq!(&again, first);
    assert_eq!(again.state_hash(), first.state_hash());
    let other = generate(m, &GenParams::new(1000, 7)).expect("generates");
    assert_ne!(other.state_hash(), first.state_hash());
    assert_ne!(other.persons, first.persons);
    assert_exact_county_totals(m, &other, "seed 7");
}

/// AC-POP-06 [golden] The state hash of the fixture population at each of the three scales equals the committed golden value.
#[test]
fn ac_pop_06() {
    let text =
        std::fs::read_to_string(repo_root().join("tests/golden/popgen_ro_census2021.hashes"))
            .expect("golden file");
    let mut seen = 0;
    for line in text.lines().filter(|l| !l.trim().is_empty()) {
        let mut parts = line.split_whitespace();
        let scale: u32 = parts.next().expect("scale").parse().expect("scale");
        let want = parts.next().expect("hash");
        let got = format!("{:016x}", population(scale).state_hash());
        assert_eq!(got, want, "state hash at 1:{scale}");
        seen += 1;
    }
    assert_eq!(seen, SCALES.len(), "one golden hash per scale");
}

fn reference(scale: u32) -> Value {
    let script = repo_root().join("python/reference/popgen_reference.py");
    for python in ["python3", "python"] {
        let run = Command::new(python)
            .arg(&script)
            .arg(fixture_path())
            .args([
                "--sample-scale",
                &scale.to_string(),
                "--seed",
                &SEED.to_string(),
            ])
            .output();
        if let Ok(out) = run
            && out.status.success()
        {
            return serde_json::from_slice(&out.stdout).expect("reference prints JSON");
        }
    }
    panic!(
        "the differential test needs Python 3 on PATH to run python/reference/popgen_reference.py"
    );
}

fn column(v: &Value, name: &str) -> Vec<u64> {
    v[name]
        .as_array()
        .unwrap_or_else(|| panic!("reference column {name}"))
        .iter()
        .map(as_u64)
        .collect()
}

/// AC-POP-07 [diff] On the fixture at 1:1000 and 1:200 the Rust generator and the independent Python reference produce identical households and persons, record for record.
#[test]
fn ac_pop_07() {
    for scale in [1000u32, 200] {
        let pop = generate(margins(), &GenParams::new(scale, SEED)).expect("generates");
        let r = reference(scale);
        let (hh, p) = (&pop.households, &pop.persons);
        let rust: [(&str, Vec<u64>); 7] = [
            (
                "hh_weight",
                hh.hh_weight.iter().map(|&x| u64::from(x)).collect(),
            ),
            (
                "hh_county",
                hh.hh_county.iter().map(|&x| u64::from(x)).collect(),
            ),
            (
                "hh_collective",
                hh.hh_collective.iter().map(|&x| u64::from(x)).collect(),
            ),
            (
                "household_id",
                p.household_id.iter().map(|&x| u64::from(x)).collect(),
            ),
            ("age", p.age.iter().map(|&x| u64::from(x)).collect()),
            ("sex", p.sex.iter().map(|&x| x as u64).collect()),
            ("role", p.role.iter().map(|&x| x as u64).collect()),
        ];
        for (name, got) in rust {
            let want = column(&r, name);
            assert_eq!(got.len(), want.len(), "1:{scale} {name}: number of records");
            if let Some(i) = (0..got.len()).find(|&i| got[i] != want[i]) {
                panic!(
                    "1:{scale} {name}: first difference at record {i}: Rust {} vs reference {}",
                    got[i], want[i]
                );
            }
        }
        let want_hash = r["summary"]["state_hash"].as_str().expect("reference hash");
        assert_eq!(
            format!("{:016x}", pop.state_hash()),
            want_hash,
            "1:{scale} state hash"
        );
    }
    assert_eq!(Sex::F as u8, 0);
    assert_eq!(Role::Other as u8, 3);
}

/// AC-POP-08 [unit] Inconsistent margins, `sample_scale` = 0, an empty table or a table whose length does not match its dimensions are rejected with a named error and produce no output.
#[test]
fn ac_pop_08() {
    let m = margins();
    assert_eq!(
        generate(m, &GenParams::new(0, SEED)),
        Err(GenError::ZeroScale)
    );

    let mut empty = m.clone();
    empty.counties.clear();
    empty.persons_private.clear();
    empty.persons_collective.clear();
    empty.households.clear();
    assert_eq!(
        generate(&empty, &GenParams::new(1000, SEED)),
        Err(GenError::EmptyTable)
    );

    let mut short = m.clone();
    short.households.pop();
    assert_eq!(
        generate(&short, &GenParams::new(1000, SEED)),
        Err(GenError::ShapeMismatch)
    );

    // The first county's persons cannot live in a single one-person household.
    let mut crowded = m.clone();
    let n_k = m.size_classes.len();
    crowded.households[..n_k].fill(0);
    crowded.households[0] = 1;
    assert_eq!(
        generate(&crowded, &GenParams::new(1000, SEED)),
        Err(GenError::InconsistentMargins {
            county: m.counties[0].clone()
        })
    );
}
