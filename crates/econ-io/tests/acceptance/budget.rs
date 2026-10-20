//! Acceptance tests for AC-BUD-01..13 (spec: economy/budget-2021). Written in
//! a test-authoring session from the spec and its API sketch, before the code
//! existed (ADR-0014), against the skeleton `econ_io::budget` behind the crate
//! feature `budget`. AC-BUD-14 waits for the proposed flow codes and a posting
//! function, which the sketch does not have.
//!
//! "The fixture" is `python/pipeline/fixtures/budget2021_ro.json`. Amounts in
//! comments are lei; `Bani(n)` is bani.
//!
//! The reconciliations (AC-BUD-05 to 09) belong to the scenario builder's
//! report, which has no budget section yet; they are checked here on the
//! committed fixtures and on the scenario that `econ_scenario::build` makes,
//! with the spec's tolerances.

use std::collections::BTreeSet;

use econ_io::budget::{Budget, BudgetError, EconType};
use econ_types::Bani;
use serde_json::Value;

use super::support_budget::{
    DIVISIONS, POSTS, PUBLIC_UNITS, TE_LEI, TYPES, assert_within, budget, budget_json, built, bump,
    fixture_json, golden, hex, lei, line_index, lines_sum, load_variant, public_employees,
    published, reference, sector_index, unit_index,
};

/// **AC-BUD-01** `[unit]` For each of the twelve types, the amounts of the thirteen lines add up exactly (in bani) to the published national total the fixture carries (D11, D12, P2, D62, D632, D3, D4, P5, NP, D7, D9, D29+D5+D8), and the lines add up to total expenditure, 474,031,500,000 lei, exactly.
#[test]
fn ac_bud_01() {
    let b = budget();
    assert_eq!(b.lines.len(), 13, "thirteen budget lines");
    let fx = budget_json();
    // The published totals are the fixture's, in bani, sorted by code.
    let fx_published = fx["published"].as_object().expect("published totals");
    assert_eq!(
        b.published.0.len(),
        fx_published.len(),
        "every published total is kept"
    );
    for (code, v) in fx_published {
        assert_eq!(published(b, code), lei(v), "published {code} in bani");
    }
    let codes: Vec<&String> = b.published.0.iter().map(|(c, _)| c).collect();
    let mut sorted = codes.clone();
    sorted.sort();
    assert_eq!(codes, sorted, "published totals sorted by code");

    for (t, id, code) in TYPES {
        let sum = lines_sum(b, t);
        assert_eq!(sum, published(b, code), "type {id}: lines against {code}");
        assert_eq!(b.total(t), sum, "Budget::total({id})");
    }
    let te = Bani(
        b.lines
            .iter()
            .flat_map(|l| l.per_year.iter())
            .map(|a| a.0)
            .sum(),
    );
    assert_eq!(te, Bani(TE_LEI * 100), "the lines add up to TE");
    assert_eq!(te, published(b, "TE"), "TE as published");
    assert_eq!(
        Bani(TYPES.iter().map(|&(t, _, _)| b.total(t).0).sum()),
        te,
        "the type totals add up to TE"
    );

    // `check` holds on the loaded table and finds a type that no longer adds
    // up: 1 lei moved from goods and services to interest in one line.
    assert_eq!(b.check(), Ok(()));
    let mut moved = b.clone();
    moved.lines[0].per_year[EconType::GoodsServices as usize].0 -= 100;
    moved.lines[0].per_year[EconType::Interest as usize].0 += 100;
    match moved.check() {
        Err(BudgetError::TotalMismatch {
            code,
            lines,
            published: p,
        }) => {
            let want = match code.as_str() {
                "P2" => published(b, "P2").0 - 100,
                "D4" => published(b, "D4").0 + 100,
                other => panic!("TotalMismatch on {other}, expected P2 or D4"),
            };
            assert_eq!(lines, Bani(want), "what the lines of {code} add up to");
            assert_eq!(p, published(b, &code), "the published {code}");
        }
        other => panic!("expected TotalMismatch on P2 or D4, got {other:?}"),
    }
}

/// **AC-BUD-02** `[unit]` The lines are a partition of the COFOG subgroups (no subgroup in two lines); for each of the ten divisions, the amounts of its subgroups add up exactly to Eurostat's published division total.
#[test]
fn ac_bud_02() {
    let b = budget();
    let mut seen = BTreeSet::new();
    for l in &b.lines {
        assert!(!l.cofog.is_empty(), "line {} has subgroups", l.id);
        for s in &l.cofog {
            assert!(seen.insert(s.clone()), "subgroup {s} is in two lines");
            assert!(
                s.len() == 6 && DIVISIONS.iter().any(|d| s.starts_with(d)),
                "line {}: {s} is a subgroup of one of the ten divisions",
                l.id
            );
        }
    }
    for d in DIVISIONS {
        assert!(
            seen.iter().any(|s| s.starts_with(d)),
            "division {d} has subgroups in the lines"
        );
    }

    // The division totals the loader checks: each equals Eurostat's published
    // total, and the divisions and the lines add up to the same amounts by type.
    let fx = budget_json();
    let fns = fx["by_function"].as_array().expect("by_function");
    let names: Vec<&str> = fns
        .iter()
        .map(|f| f["cofog"].as_str().expect("a division"))
        .collect();
    assert_eq!(names, DIVISIONS, "the ten divisions");
    for (k, (t, id, _)) in TYPES.iter().enumerate() {
        let by_fn: i64 = fns
            .iter()
            .map(|f| f["spending"][k].as_i64().expect("lei"))
            .sum();
        assert_eq!(
            Bani(by_fn * 100),
            lines_sum(b, *t),
            "type {id}: divisions against lines"
        );
    }
    for f in fns {
        assert_eq!(f["total"], f["published_total"], "division {}", f["cofog"]);
    }

    // A division that misses its published total by 1 lei is rejected.
    let gf07 = DIVISIONS.iter().position(|&d| d == "GF07").expect("GF07");
    let r = load_variant(|v| bump(&mut v["by_function"][gf07]["published_total"], 1));
    assert_eq!(
        r,
        Err(BudgetError::DivisionMismatch {
            cofog: "GF07".into()
        })
    );
    // So is a division whose amounts do not add up to its total: 1 lei more
    // on one type.
    let r = load_variant(|v| bump(&mut v["by_function"][gf07]["spending"][2], 1));
    assert_eq!(
        r,
        Err(BudgetError::DivisionMismatch {
            cofog: "GF07".into()
        })
    );
    // A subgroup in two lines is not a partition: rejected.
    let r = load_variant(|v| {
        let research = line_index(v, "research");
        v["lines"][research]["cofog"]
            .as_array_mut()
            .expect("cofog")
            .push(Value::from("GF0901"));
    });
    assert!(r.is_err(), "a subgroup in two lines is rejected: {r:?}");
}

/// **AC-BUD-03** `[unit]` The social benefits by programme add up exactly to D62 (cash) and D632 (in kind).
#[test]
fn ac_bud_03() {
    let b = budget();
    assert_eq!(b.programmes.len(), 10, "ten programmes");
    let cash = Bani(b.programmes.iter().map(|p| p.cash.0).sum());
    let kind = Bani(b.programmes.iter().map(|p| p.in_kind.0).sum());
    assert_eq!(cash, published(b, "D62"), "cash benefits against D62");
    assert_eq!(kind, published(b, "D632"), "benefits in kind against D632");
    assert_eq!(cash, b.total(EconType::SocialBenefitsCash));
    assert_eq!(kind, b.total(EconType::SocialBenefitsInKind));
    // 156,499.4 million lei together.
    assert_eq!(cash.0 + kind.0, 156_499_400_000 * 100);
}

/// **AC-BUD-04** `[unit]` Posts by sector add up to 1,262,318; every sector's unit is one of the five public firm units; the public units' posts, wages and employer contributions add up exactly to the total posts, D11 and D12.
#[test]
fn ac_bud_04() {
    let b = budget();
    let posts: u64 = b.sectors.iter().map(|s| u64::from(s.posts)).sum();
    assert_eq!(posts, POSTS, "posts by sector");
    for s in &b.sectors {
        assert!(
            PUBLIC_UNITS.contains(&s.unit.as_str()),
            "sector {}: {} is not a public firm unit",
            s.id,
            s.unit
        );
    }
    let units: BTreeSet<&str> = b.public_units.iter().map(|u| u.unit.as_str()).collect();
    assert_eq!(b.public_units.len(), 5, "five public units");
    assert_eq!(
        units,
        PUBLIC_UNITS.into_iter().collect::<BTreeSet<_>>(),
        "the five public firm units"
    );
    for u in &b.public_units {
        let of_sectors: u32 = b
            .sectors
            .iter()
            .filter(|s| s.unit == u.unit)
            .map(|s| s.posts)
            .sum();
        assert_eq!(
            u.posts, of_sectors,
            "{}: posts are its sectors' posts",
            u.unit
        );
    }
    let unit_posts: u64 = b.public_units.iter().map(|u| u64::from(u.posts)).sum();
    assert_eq!(unit_posts, POSTS, "the public units' posts");
    let wages = Bani(b.public_units.iter().map(|u| u.wages.0).sum());
    let contributions = Bani(
        b.public_units
            .iter()
            .map(|u| u.employer_contributions.0)
            .sum(),
    );
    assert_eq!(
        wages,
        published(b, "D11"),
        "the public units' wages against D11"
    );
    assert_eq!(wages, b.total(EconType::Wages));
    assert_eq!(
        contributions,
        published(b, "D12"),
        "employer contributions against D12"
    );
    assert_eq!(contributions, b.total(EconType::EmployerContributions));
}

/// The five public units of the firm-unit file, as (id, persons employed,
/// wages a year in bani).
fn firm_file_public_units() -> Vec<(String, i128, i128)> {
    let firms = fixture_json("firm_units2021_ro.json");
    firms["units"]
        .as_array()
        .expect("units")
        .iter()
        .filter(|u| u["size"] == "public")
        .map(|u| {
            (
                u["id"].as_str().expect("an id").to_string(),
                i128::from(u["persons_employed"].as_u64().expect("persons")),
                i128::from(lei(&u["wages"]).0),
            )
        })
        .collect()
}

/// **AC-BUD-05** `[unit]` The public units' wage bill in the firm-unit file is within 2% of the budget's wages (D11). Measured: 0.00% (the units' wages are this table's).
#[test]
fn ac_bud_05() {
    let b = budget();
    let units = firm_file_public_units();
    let ids: BTreeSet<&str> = units.iter().map(|u| u.0.as_str()).collect();
    assert_eq!(
        ids,
        PUBLIC_UNITS.into_iter().collect::<BTreeSet<_>>(),
        "the firm-unit file's public units"
    );
    let wages: i128 = units.iter().map(|u| u.2).sum();
    assert_within(
        "public units' wages in the firm-unit file against D11",
        wages,
        i128::from(b.total(EconType::Wages).0),
        0.02,
    );
}

/// **AC-BUD-06** `[unit]` The public units' persons employed in the firm-unit file are within 2% of the posts, in total and for each unit. Measured: 0.00% (the units are built from the posts; tightened from 25% on 2026-10-15, when it measured +18.9%).
#[test]
fn ac_bud_06() {
    let b = budget();
    let units = firm_file_public_units();
    let persons: i128 = units.iter().map(|u| u.1).sum();
    let posts: i128 = b.public_units.iter().map(|u| i128::from(u.posts)).sum();
    assert_within(
        "public persons employed against posts",
        persons,
        posts,
        0.02,
    );
    for u in &b.public_units {
        let f = units
            .iter()
            .find(|f| f.0 == u.unit)
            .unwrap_or_else(|| panic!("{} is not in the firm-unit file", u.unit));
        assert_within(
            &format!("{}: persons employed against posts", u.unit),
            f.1,
            i128::from(u.posts),
            0.02,
        );
    }
}

/// **AC-BUD-07** `[unit]` The earnings file's wage bill of the industry group `public_education_health`, times twelve, is within 10% of the budget's wages (D11). Measured: −4.2%.
#[test]
fn ac_bud_07() {
    let b = budget();
    let earnings = fixture_json("earnings2021_ro.json");
    let g = earnings["industry_groups"]
        .as_array()
        .expect("industry groups")
        .iter()
        .position(|x| x == "public_education_health")
        .expect("the group public_education_health");
    let bill = i128::from(lei(&earnings["wage_bill"][g]).0) * 12;
    assert_within(
        "earnings of public_education_health a year against D11",
        bill,
        i128::from(b.total(EconType::Wages).0),
        0.10,
    );
}

/// **AC-BUD-08** `[unit]` Old-age and survivors' cash benefits are within 10% of the pension file's pension bill times twelve (measured +7.3%); government final consumption in the input-output table is within 1% of P3 (measured 0.00%).
#[test]
fn ac_bud_08() {
    let b = budget();
    let pensions: i128 = b
        .programmes
        .iter()
        .filter(|p| p.id == "old_age" || p.id == "survivors")
        .map(|p| i128::from(p.cash.0))
        .sum();
    let n = b
        .programmes
        .iter()
        .filter(|p| p.id == "old_age" || p.id == "survivors")
        .count();
    assert_eq!(n, 2, "the programmes old_age and survivors");
    let bill = i128::from(lei(&fixture_json("pensions2021_ro.json")["pension_bill"]).0) * 12;
    assert_within(
        "old-age and survivors' cash benefits against the pension bill",
        pensions,
        bill,
        0.10,
    );

    let io = fixture_json("io_table2021_ro.json");
    let gov = &io["final"]["government"];
    let io_gov: i128 = ["domestic", "imports_eu", "imports_non_eu"]
        .iter()
        .flat_map(|k| gov[k].as_array().expect("a column").iter())
        .map(|v| i128::from(lei(v).0))
        .sum();
    assert_within(
        "government final consumption in the input-output table against P3",
        io_gov,
        i128::from(published(b, "P3").0),
        0.01,
    );
}

/// **AC-BUD-09** `[unit]` In the starting population at 1:1000 and 1:100 (seed 42), the weighted employees of the five public units are within 20% of the posts (measured −16.9% and −17.0%), and their wages times twelve within 10% of D11 (measured −9.1% and −8.9%).
#[test]
fn ac_bud_09() {
    let b = budget();
    let posts: i128 = b.public_units.iter().map(|u| i128::from(u.posts)).sum();
    let d11 = i128::from(b.total(EconType::Wages).0);
    for scale in [1000, 100] {
        let by_unit = public_employees(built(scale));
        let employees: i128 = by_unit.iter().map(|u| u.0).sum();
        let wages: i128 = by_unit.iter().map(|u| u.1).sum();
        assert!(employees > 0, "1:{scale}: the public units have employees");
        assert_within(
            &format!("1:{scale}: public employees against posts"),
            employees,
            posts,
            0.20,
        );
        assert_within(
            &format!("1:{scale}: public employees' wages a year against D11"),
            wages,
            d11,
            0.10,
        );
    }
}

/// The monthly amount the spec's rule gives: `q = |A| / 12`, `r = |A| mod 12`
/// (in bani), month `m` gets `q + 1` if `m < r`, else `q`, with the sign of `A`.
fn rule(per_year: i64, m: u8) -> i64 {
    let (q, r) = (per_year.abs() / 12, per_year.abs() % 12);
    let v = q + i64::from(i64::from(m) < r);
    if per_year < 0 { -v } else { v }
}

/// **AC-BUD-10** `[unit]` Every annual amount becomes twelve monthly amounts in bani that add up exactly to it, differ by at most one ban, put the remainder on the first months and keep the sign of a negative amount.
#[test]
fn ac_bud_10() {
    let b = budget();
    let mut amounts: Vec<i64> = b
        .lines
        .iter()
        .flat_map(|l| l.per_year.iter().map(|a| a.0))
        .collect();
    amounts.extend(b.programmes.iter().flat_map(|p| [p.cash.0, p.in_kind.0]));
    amounts.extend(
        b.public_units
            .iter()
            .flat_map(|u| [u.wages.0, u.employer_contributions.0]),
    );
    amounts.extend([
        0,
        1,
        11,
        12,
        13,
        23,
        100,
        -1,
        -11,
        -13,
        -140_000_000 * 100, // NP, 2021: −140.0 million lei
        111_286_200_000 * 100,
        i64::MAX / 2,
        i64::MIN / 2,
    ]);
    assert!(
        amounts.iter().any(|&a| a < 0),
        "a negative amount is tested"
    );
    for a in amounts {
        let months: Vec<i64> = (0..12u8).map(|m| Budget::monthly(Bani(a), m).0).collect();
        assert_eq!(
            months.iter().map(|&x| i128::from(x)).sum::<i128>(),
            i128::from(a),
            "{a}: the twelve add up"
        );
        let (lo, hi) = (
            months.iter().min().expect("twelve"),
            months.iter().max().expect("twelve"),
        );
        assert!(
            hi - lo <= 1,
            "{a}: months differ by at most one ban: {months:?}"
        );
        for (m, &x) in (0..12u8).zip(&months) {
            assert_eq!(x, rule(a, m), "{a}: month {m} by the spec's rule");
            assert!(
                a == 0 || x == 0 || x.signum() == a.signum(),
                "{a}: month {m} keeps the sign"
            );
        }
        // The remainder is on the first months: never more in a later month.
        assert!(
            months.windows(2).all(|w| w[0].abs() >= w[1].abs()),
            "{a}: the remainder goes to the first months: {months:?}"
        );
    }
}

/// **AC-BUD-11** `[golden]` The budget hash of the fixture equals the committed golden value. Hash: FNV-1a 64 over the line count (u32, little-endian); per line its id (one length byte, then UTF-8) and each amount in bani (i64, little-endian) in type order; the sector count (u32); per sector its id and posts (u32); per public unit in file order its id, posts (u32), wages and employer contributions in bani (i64). Python reference: `ffd4772468f8552e`.
#[test]
fn ac_bud_11() {
    assert_eq!(
        hex(budget().hash()),
        golden("fixture"),
        "the budget hash of the fixture (tests/golden/budget_ro_2021.hashes)"
    );
    // The hash reads the amounts: one ban moved between two lines changes it.
    let mut moved = budget().clone();
    moved.lines[0].per_year[0].0 -= 1;
    moved.lines[1].per_year[0].0 += 1;
    assert_ne!(moved.hash(), budget().hash(), "the hash reads every amount");
}

fn as_i64(v: &Value) -> i64 {
    v.as_i64().unwrap_or_else(|| panic!("a whole number: {v}"))
}

fn strings(v: &Value) -> Vec<String> {
    v.as_array()
        .expect("a list")
        .iter()
        .map(|s| s.as_str().expect("a string").to_string())
        .collect()
}

/// **AC-BUD-12** `[diff]` On the fixture, the Rust loader and the Python reference (`python/reference/budget_reference.py`) give the same lines, programmes, sectors, public units, first-month amounts in bani and hash.
#[test]
fn ac_bud_12() {
    let b = budget();
    let r = reference();
    assert_eq!(
        r["structure_errors"],
        Value::Array(vec![]),
        "the reference finds the fixture consistent"
    );
    let t = &r["tables"];

    let lines = t["lines"].as_array().expect("lines");
    assert_eq!(lines.len(), b.lines.len(), "line count");
    for (l, p) in b.lines.iter().zip(lines) {
        assert_eq!(l.id, p["id"].as_str().expect("an id"), "line order");
        assert_eq!(l.cofog, strings(&p["cofog"]), "line {}: subgroups", l.id);
        let amounts: Vec<i64> = l.per_year.iter().map(|a| a.0).collect();
        let want: Vec<i64> = p["per_year_bani"]
            .as_array()
            .expect("amounts")
            .iter()
            .map(as_i64)
            .collect();
        assert_eq!(amounts, want, "line {}: amounts a year in bani", l.id);
        let first: Vec<i64> = l
            .per_year
            .iter()
            .map(|&a| Budget::monthly(a, 0).0)
            .collect();
        let want_first: Vec<i64> = r["lines_first_month_bani"][&l.id]
            .as_array()
            .expect("first-month amounts")
            .iter()
            .map(as_i64)
            .collect();
        assert_eq!(
            first, want_first,
            "line {}: first-month amounts in bani",
            l.id
        );
    }

    let progs = t["programmes"].as_array().expect("programmes");
    assert_eq!(progs.len(), b.programmes.len(), "programme count");
    for (g, p) in b.programmes.iter().zip(progs) {
        assert_eq!(g.id, p["id"].as_str().expect("an id"), "programme order");
        assert_eq!(g.cofog, strings(&p["cofog"]), "programme {}: COFOG", g.id);
        assert_eq!(
            g.cash.0,
            as_i64(&p["cash_bani"]),
            "programme {}: cash",
            g.id
        );
        assert_eq!(
            g.in_kind.0,
            as_i64(&p["in_kind_bani"]),
            "programme {}: in kind",
            g.id
        );
    }

    let sectors = t["sectors"].as_array().expect("sectors");
    assert_eq!(sectors.len(), b.sectors.len(), "sector count");
    for (s, p) in b.sectors.iter().zip(sectors) {
        assert_eq!(s.id, p["id"].as_str().expect("an id"), "sector order");
        assert_eq!(
            i64::from(s.posts),
            as_i64(&p["posts"]),
            "sector {}: posts",
            s.id
        );
        assert_eq!(
            s.unit,
            p["unit"].as_str().expect("a unit"),
            "sector {}: unit",
            s.id
        );
    }

    let units = t["public_units"].as_array().expect("public units");
    assert_eq!(units.len(), b.public_units.len(), "public unit count");
    for (u, p) in b.public_units.iter().zip(units) {
        assert_eq!(
            u.unit,
            p["unit"].as_str().expect("a unit"),
            "public unit order"
        );
        assert_eq!(i64::from(u.posts), as_i64(&p["posts"]), "{}: posts", u.unit);
        assert_eq!(u.wages.0, as_i64(&p["wages_bani"]), "{}: wages", u.unit);
        assert_eq!(
            u.employer_contributions.0,
            as_i64(&p["employer_contributions_bani"]),
            "{}: employer contributions",
            u.unit
        );
    }

    assert_eq!(
        hex(b.hash()),
        r["hash"].as_str().expect("the reference's hash"),
        "budget hash"
    );
}

/// **AC-BUD-13** `[unit]` A line with the wrong number of amounts, an unknown or repeated type or line, a type whose lines do not add up to its published total, a sector whose unit is not a public firm unit, posts that do not add up, and a public unit with wages and no posts are each rejected with a named error, and nothing is loaded.
#[test]
fn ac_bud_13() {
    // The unchanged fixture loads, so each rejection below is the edit's.
    assert!(load_variant(|_| ()).is_ok(), "the fixture loads");

    // A line with eleven amounts (its total follows).
    let r = load_variant(|v| {
        let i = line_index(v, "education");
        let last = v["lines"][i]["spending"]
            .as_array_mut()
            .expect("amounts")
            .pop()
            .expect("an amount");
        bump(&mut v["lines"][i]["total"], -as_i64(&last));
    });
    assert_eq!(
        r,
        Err(BudgetError::Shape {
            line: "education".into()
        }),
        "a line with the wrong number of amounts"
    );

    // An unknown type: `subsidies` renamed wherever the file names it.
    let r = load_variant(|v| {
        v["types"][5] = Value::from("subsidy_payments");
        let esa = v["esa"].as_object_mut().expect("esa");
        let code = esa.remove("subsidies").expect("subsidies");
        esa.insert("subsidy_payments".into(), code);
        v["categories"]["subsidies"] = Value::from(vec!["subsidy_payments"]);
        let by_type = v["totals"]["by_type"].as_object_mut().expect("by_type");
        let total = by_type.remove("subsidies").expect("subsidies");
        by_type.insert("subsidy_payments".into(), total);
    });
    assert_eq!(
        r,
        Err(BudgetError::UnknownType("subsidy_payments".into())),
        "an unknown type"
    );

    // A repeated type: `wages` twice in the type list. The sketch names no
    // variant of its own for it; any named error rejects it.
    let r = load_variant(|v| v["types"][5] = Value::from("wages"));
    assert!(r.is_err(), "a repeated type is rejected: {r:?}");

    // A repeated line: `culture` renamed `education` (no amount changes).
    let r = load_variant(|v| {
        let i = line_index(v, "culture");
        v["lines"][i]["id"] = Value::from("education");
    });
    assert_eq!(
        r,
        Err(BudgetError::DuplicateLine("education".into())),
        "a repeated line"
    );

    // An unknown line: `culture` renamed `sport`, not a line of the spec. The
    // sketch names no variant of its own for it; any named error rejects it.
    let r = load_variant(|v| {
        let i = line_index(v, "culture");
        v["lines"][i]["id"] = Value::from("sport");
    });
    assert!(r.is_err(), "an unknown line is rejected: {r:?}");

    // A type whose lines do not add up to its published total: P2 published
    // 1 lei higher.
    let p2 = budget_json()["published"]["P2"].as_i64().expect("P2");
    let r = load_variant(|v| bump(&mut v["published"]["P2"], 1));
    assert_eq!(
        r,
        Err(BudgetError::TotalMismatch {
            code: "P2".into(),
            lines: Bani(p2 * 100),
            published: Bani((p2 + 1) * 100),
        }),
        "a type whose lines do not add up to its published total"
    );

    // A sector whose unit is not a public firm unit (the public unit is
    // renamed with it, so posts still add up).
    let r = load_variant(|v| {
        let s = sector_index(v, "social_care");
        v["employment"]["sectors"][s]["unit"] = Value::from("social_care.private");
        let u = unit_index(v, "social_care.public");
        v["public_units"][u]["unit"] = Value::from("social_care.private");
    });
    assert_eq!(
        r,
        Err(BudgetError::UnknownUnit("social_care.private".into())),
        "a sector whose unit is not a public firm unit"
    );

    // Posts that do not add up: one sector has one post more than its unit
    // and the total.
    let r = load_variant(|v| {
        let s = sector_index(v, "education");
        bump(&mut v["employment"]["sectors"][s]["posts"], 1);
    });
    assert_eq!(r, Err(BudgetError::PostsMismatch), "a sector's posts");
    // Or the total is one post more than the sectors and the units.
    let r = load_variant(|v| bump(&mut v["employment"]["posts"], 1));
    assert_eq!(r, Err(BudgetError::PostsMismatch), "the total posts");

    // A public unit with wages and no posts: social care's 130,000 posts
    // taken out of its sector, its unit and the totals, its wages kept.
    let r = load_variant(|v| {
        let s = sector_index(v, "social_care");
        let posts = as_i64(&v["employment"]["sectors"][s]["posts"]);
        v["employment"]["sectors"][s]["posts"] = Value::from(0);
        let u = unit_index(v, "social_care.public");
        v["public_units"][u]["posts"] = Value::from(0);
        bump(&mut v["employment"]["posts"], -posts);
        bump(&mut v["employment"]["local"], -posts);
        bump(&mut v["totals"]["posts"], -posts);
    });
    assert_eq!(
        r,
        Err(BudgetError::WagesWithoutPosts("social_care.public".into())),
        "a public unit with wages and no posts"
    );
}
