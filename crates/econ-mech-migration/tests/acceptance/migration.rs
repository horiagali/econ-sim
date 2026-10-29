//! Acceptance tests for AC-MIG-01 to AC-MIG-04 (spec:
//! economy/migration-remittances), the `[unit]` criteria, against the API
//! sketch. Written by the test writer before the code existed (ADR-0014).
//!
//! Rates and indices in millionths; wages abroad in basis points of the start
//! (10,000 = 100%, the lever `world.wages_abroad`); the exchange rate in lei
//! per euro, millionths. The parameters are the spec's defaults (ε 1.5, β_u 5,
//! β_net 1, unemployed factor 2, p^max 2% a month, η 0.8, δ 5% a year).
//! Expected values of powers were computed in double precision (Python); the
//! tests allow one unit of the last place (a millionth, a cent) for rounding,
//! which the spec does not fix.

use econ_mech_migration::{
    MigrationParams, Pick, destination_ppm, emigration_ppm, eur_to_lei, pull_ppm,
    remittance_eur_cents, take_whole_records,
};
use econ_num::math;
use econ_types::Bani;

const ONE: u32 = 1_000_000;
/// Wages abroad at the start, basis points.
const START_BP: u32 = 10_000;

fn near(a: u32, b: u32, what: &str) {
    assert!(a.abs_diff(b) <= 1, "{what}: {a} against {b}");
}

fn near_i(a: i64, b: i64, what: &str) {
    assert!((a - b).abs() <= 1, "{what}: {a} against {b}");
}

fn p() -> MigrationParams {
    let p = MigrationParams::spec_defaults();
    assert_eq!(p.emigration_elasticity_ppm, 1_500_000);
    assert_eq!(p.return_elasticity_ppm, 1_000_000);
    assert_eq!(p.unemployment_response_ppm, 5_000_000);
    assert_eq!(p.network_effect_ppm, 1_000_000);
    assert_eq!(p.unemployed_factor_ppm, 2_000_000);
    assert_eq!(p.max_emigration_ppm, 20_000);
    assert_eq!(p.household_share_ppm, 300_000);
    assert_eq!(p.link_return_ppm, 1_500_000);
    assert_eq!(p.remittance_elasticity_ppm, 800_000);
    assert_eq!(p.remittance_decay_ppm_year, 50_000);
    p
}

/// **AC-MIG-01** `[unit]` The pull is ω / H of step 1: 1 at the start; a 10%
/// rise of wages abroad raises it by 10%; a 10% rise of home net wages in lei
/// at a constant exchange rate lowers it by 1/1.1; a 10% depreciation raises
/// it by 10%. The emigration probability is the formula of step 2,
/// increasing in the pull, the unemployment rate and the network flag, never
/// above p^max.
///
/// Readings: `H` is the caller's: home net wages 10% higher in lei at a
/// constant rate are `H` = 1.1; a 10% depreciation (`e` = 1.1 `e0`) at
/// constant lei wages is `H` = 1/1.1 (rounded to the millionth). The
/// probability is checked at the start (every factor 1: the base rate), for
/// each factor of step 2 alone against the formula, and at the cap.
#[test]
fn ac_mig_01() {
    let p = p();
    // The pull.
    assert_eq!(pull_ppm(START_BP, ONE), ONE, "1 at the start");
    near(pull_ppm(11_000, ONE), 1_100_000, "wages abroad +10%");
    near(
        pull_ppm(START_BP, 1_100_000),
        909_091,
        "home net wages +10% in lei",
    );
    let h_depreciated = 909_091; // (W / 1.1 e0) / (W / e0), millionths
    near(
        pull_ppm(START_BP, h_depreciated),
        1_100_000,
        "a 10% depreciation",
    );
    // Wages abroad and home wages up by the same 10%: no pull.
    near(pull_ppm(11_000, 1_100_000), ONE, "both +10%");

    // The probability at the start: the base rate.
    let u0 = 50_000; // 5% unemployment in the region
    for base in [0, 1, 1_000, 10_000] {
        assert_eq!(
            emigration_ppm(base, ONE, u0, u0, false, false, &p),
            base,
            "base {base} with every factor at its start"
        );
    }
    let base = 5_000;
    // The pull to the power ε = 1.5: 1.1^1.5 = 1.153690.
    near(
        emigration_ppm(10_000, 1_100_000, u0, u0, false, false, &p),
        11_537,
        "pull 1.1",
    );
    // Unemployment: (1 + 5 · 0.06) / (1 + 5 · 0.05) = 1.04.
    near(
        emigration_ppm(10_000, ONE, 60_000, u0, false, false, &p),
        10_400,
        "unemployment 5% → 6%",
    );
    // The network: 1 + β_net = 2.
    near(
        emigration_ppm(base, ONE, u0, u0, true, false, &p),
        10_000,
        "a household with a linked emigrant",
    );
    // A job seeker: the unemployed factor, 2.
    near(
        emigration_ppm(base, ONE, u0, u0, false, true, &p),
        10_000,
        "a job seeker",
    );
    // Increasing in each.
    let mut last = 0;
    for pull in [500_000, 800_000, ONE, 1_200_000, 1_500_000] {
        let x = emigration_ppm(base, pull, u0, u0, false, false, &p);
        assert!(x > last, "increasing in the pull: {pull} gives {x}");
        last = x;
    }
    let mut last = 0;
    for u in [0, 20_000, 50_000, 100_000, 200_000] {
        let x = emigration_ppm(base, ONE, u, u0, false, false, &p);
        assert!(x > last, "increasing in unemployment: {u} gives {x}");
        last = x;
    }
    assert!(
        emigration_ppm(base, ONE, u0, u0, true, false, &p)
            > emigration_ppm(base, ONE, u0, u0, false, false, &p)
    );
    // Never above p^max (2% a month).
    for (b, pull, linked, seeker) in [
        (15_000, 2_000_000, false, false),
        (19_000, ONE, true, true),
        (100_000, ONE, false, false),
        (20_000, 4_000_000, true, true),
    ] {
        let x = emigration_ppm(b, pull, 300_000, u0, linked, seeker, &p);
        assert!(
            x <= p.max_emigration_ppm,
            "{x} above the cap for base {b}, pull {pull}"
        );
    }
    assert_eq!(
        emigration_ppm(15_000, 2_000_000, u0, u0, false, false, &p),
        20_000,
        "the cap binds: 15,000 · 2^1.5 = 42,426"
    );
}

/// A deterministic priority for the tests (SplitMix64).
fn mix(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^ (x >> 31)
}

/// Candidates `0..n` with weights 900 to 1,100 and keyed priorities.
fn candidates(n: u32, month: u64) -> Vec<Pick> {
    (0..n)
        .map(|r| Pick {
            record: r,
            weight: 900 + u32::try_from(mix(u64::from(r)) % 201).expect("fits"),
            priority: mix(month << 32 | u64::from(r)),
        })
        .collect()
}

/// The records in ascending (priority, record) order.
fn by_priority(c: &[Pick]) -> Vec<Pick> {
    let mut v = c.to_vec();
    v.sort_by_key(|x| (x.priority, x.record));
    v
}

/// The total weight of records of a candidate list.
fn weight_of(c: &[Pick], records: &[u32]) -> i64 {
    records
        .iter()
        .map(|&r| {
            i64::from(
                c.iter()
                    .find(|x| x.record == r)
                    .unwrap_or_else(|| panic!("record {r} was not a candidate"))
                    .weight,
            )
        })
        .sum()
}

/// One month's alignment, checked: the records taken are the first ones in
/// priority order, each once; the carry is the target less the weight taken,
/// within one record's weight of zero. Returns the records and the weight.
fn align_checked(target: i64, carry: &mut i64, c: &[Pick], what: &str) -> (Vec<u32>, i64) {
    let goal = target + *carry;
    let mut cands = c.to_vec();
    let taken = take_whole_records(target, carry, &mut cands);
    let order = by_priority(c);
    let prefix: Vec<u32> = order.iter().take(taken.len()).map(|x| x.record).collect();
    assert_eq!(
        taken, prefix,
        "{what}: the records taken are the first in priority order"
    );
    let w = weight_of(c, &taken);
    assert_eq!(
        *carry,
        goal - w,
        "{what}: the carry is the target less the weight taken"
    );
    let max_w = i64::from(c.iter().map(|x| x.weight).max().unwrap_or(0));
    assert!(
        carry.abs() < max_w,
        "{what}: carry {} is not within one record's weight ({max_w})",
        *carry
    );
    (taken, w)
}

/// **AC-MIG-02** `[unit]` Whole-record alignment: given weighted candidates
/// with priorities and a target plus carry, records are taken in priority
/// order, each with its whole weight, until the target is met; the new carry
/// is the target less the weight taken; over 120 months of constant
/// expectations the total moved differs from the expected total by less
/// than one record's weight; reruns give the same records.
///
/// Readings: "priority order" is ascending (keyed priority, then record),
/// the order of the other keyed queues of the specs (education's places, the
/// labour market's hires). The target and the carry are weighted persons,
/// the unit of the weights. "Until the target is met" leaves the stopping
/// rule's last record open (stop at or just before the target): the test
/// asks that the records taken are a prefix of the priority order and that
/// the new carry is within one record's weight of zero. When the target
/// exceeds every candidate's weight, all are taken and the rest is dropped,
/// not carried (edge cases): the carry becomes 0.
#[test]
fn ac_mig_02() {
    // A hand-sized case: weights 1,000, target 2,500.
    let c: Vec<Pick> = [(0, 5), (1, 3), (2, 9), (3, 1), (4, 7)]
        .iter()
        .map(|&(record, priority)| Pick {
            record,
            weight: 1_000,
            priority,
        })
        .collect();
    let mut carry = 0;
    let (taken, w) = align_checked(2_500, &mut carry, &c, "2,500 of five records");
    assert!(
        taken == vec![3, 1] || taken == vec![3, 1, 0],
        "lowest priorities first: {taken:?}"
    );
    assert!(w == 2_000 || w == 3_000);

    // The carry counts: 1,000 owed and a target of 1,000 take two records.
    let mut carry = 1_000;
    let (taken, _) = align_checked(1_000, &mut carry, &c, "a positive carry");
    assert_eq!(taken, vec![3, 1]);
    assert_eq!(carry, 0);

    // A negative carry larger than the target takes nothing and stays.
    let mut carry = -700;
    let mut cands = c.clone();
    assert!(take_whole_records(500, &mut carry, &mut cands).is_empty());
    assert_eq!(carry, -200, "the target less nothing taken");

    // A target above every candidate: all taken, the rest dropped.
    let mut carry = 0;
    let mut cands = c.clone();
    let mut all = take_whole_records(9_000, &mut carry, &mut cands);
    all.sort_unstable();
    assert_eq!(all, vec![0, 1, 2, 3, 4]);
    assert_eq!(carry, 0, "the rest of the target is dropped, not carried");

    // 120 months of constant expectations: 2,345 persons a month out of 50
    // records of 900 to 1,100 persons, priorities keyed by the month.
    let run = || {
        let mut carry = 0;
        let mut moved = 0i64;
        let mut records = Vec::new();
        for month in 0..120u64 {
            let c = candidates(50, month);
            let (taken, w) = align_checked(2_345, &mut carry, &c, &format!("month {month}"));
            moved += w;
            records.push(taken);
        }
        (moved, records)
    };
    let (moved, records) = run();
    let expected = 120 * 2_345;
    assert!(
        (moved - expected).abs() < 1_100,
        "moved {moved} against {expected} expected"
    );
    let (moved2, records2) = run();
    assert_eq!(
        (moved, &records),
        (moved2, &records2),
        "a rerun takes the same records"
    );
}

/// **AC-MIG-03** `[unit]` The remittance amount is the formula of step 3: in
/// euro it rises with wages abroad by the elasticity and falls by the decay
/// per year abroad; in lei it is the euro amount times the exchange rate, to
/// the ban.
///
/// Readings: "to the ban" is rounded half away from zero, the rule of
/// ADR-0007 for money. The euro amounts allow one cent for the rounding of
/// the powers.
#[test]
fn ac_mig_03() {
    let p = p();
    let base = 20_000; // 200 euro a month
    assert_eq!(
        remittance_eur_cents(base, START_BP, 0, &p),
        base,
        "at the start, before any decay"
    );
    // ω^η: 1.2^0.8 = 1.157031.
    near_i(
        remittance_eur_cents(base, 12_000, 0, &p),
        23_141,
        "wages abroad +20%",
    );
    assert!(
        remittance_eur_cents(base, 8_000, 0, &p) < base,
        "wages abroad −20%"
    );
    // (1 − δ)^(months / 12).
    near_i(
        remittance_eur_cents(base, START_BP, 12, &p),
        19_000,
        "a year abroad",
    );
    near_i(
        remittance_eur_cents(base, START_BP, 24, &p),
        18_050,
        "two years abroad",
    );
    near_i(
        remittance_eur_cents(base, START_BP, 6, &p),
        19_494,
        "six months abroad",
    );
    // The formula, for a grid.
    for (bp, months) in [(5_000, 3), (13_000, 40), (40_000, 300), (2_500, 0)] {
        let omega = f64::from(bp) / 10_000.0;
        let years = f64::from(months) / 12.0;
        let x = 20_000.0 * math::powf(omega, 0.8) * math::powf(0.95, years);
        #[allow(clippy::cast_possible_truncation)]
        let want = x.round() as i64;
        near_i(
            remittance_eur_cents(base, bp, months, &p),
            want,
            &format!("ω {bp} bp, {months} months abroad"),
        );
    }
    // Decreasing with time abroad.
    let mut last = i64::MAX;
    for months in [0, 12, 60, 120, 240] {
        let x = remittance_eur_cents(base, START_BP, months, &p);
        assert!(x < last || months == 0, "decay: {months} months gives {x}");
        last = x;
    }

    // In lei: euro times the rate, to the ban (4.9475 lei per euro).
    let rate = 4_947_500;
    assert_eq!(eur_to_lei(20_000, rate), Bani(98_950));
    assert_eq!(eur_to_lei(333, rate), Bani(1_648), "1,647.5175 bani");
    assert_eq!(eur_to_lei(0, rate), Bani::ZERO);
    assert_eq!(eur_to_lei(1, 5_000_000), Bani(5));
    // A weaker leu: more lei for the same euro.
    assert!(eur_to_lei(20_000, 5_442_250) > eur_to_lei(20_000, rate));
    // Large amounts and an extreme rate do not overflow.
    assert_eq!(
        eur_to_lei(1_000_000_000_000, 100_000_000),
        Bani(100_000_000_000_000)
    );
}

/// **AC-MIG-04** `[unit]` Destination shares of step 6 sum to one million
/// ppm; a destination with higher wages, lower unemployment or a shorter
/// travel time gets a larger share; ties are broken by county order.
///
/// Readings: the utilities `V_cd` are the caller's; the test builds them with
/// the spec's formula and defaults (β_w 1, β_u 5, β_t 0.1 an hour, α 0) so
/// that each of wages, unemployment and travel time moves one destination's
/// utility. "Ties broken by county order": destinations of equal utility get
/// shares that differ by at most one millionth, the earlier county not the
/// smaller. The origin gets no share.
#[test]
fn ac_mig_04() {
    // Hand-checked logit: utilities 0, 0, 0.5, −0.5, 0 from county 0.
    let shares = destination_ppm(0, &[0, 0, 500_000, -500_000, 0]);
    assert_eq!(shares.len(), 5);
    assert_eq!(shares[0], 0, "the origin");
    assert_eq!(shares.iter().map(|&s| u64::from(s)).sum::<u64>(), 1_000_000);
    near(shares[1], 235_004, "utility 0");
    near(shares[2], 387_456, "utility 0.5");
    near(shares[3], 142_537, "utility −0.5");
    near(shares[4], 235_004, "utility 0");
    assert!(
        shares[1] >= shares[4] && shares[1] - shares[4] <= 1,
        "a tie"
    );

    // V_cd = α_d + β_w ln(W_d / W_c) − β_u (u_d − u_c) − β_t t_cd, millionths.
    let utility = |w_d: f64, w_c: f64, u_d: f64, u_c: f64, hours: f64| -> i64 {
        let v = math::ln(w_d / w_c) - 5.0 * (u_d - u_c) - 0.1 * hours;
        #[allow(clippy::cast_possible_truncation)]
        let x = (v * 1e6).round() as i64;
        x
    };
    // Origin 0 (wage 4,000, 6% unemployment); four destinations alike
    // (wage 4,000, 6%, 3 hours), then one of them changed.
    let alike = |c: usize| -> Vec<i64> {
        (0..5)
            .map(|d| {
                if d == c {
                    0
                } else {
                    utility(4_000.0, 4_000.0, 0.06, 0.06, 3.0)
                }
            })
            .collect()
    };
    let base = destination_ppm(0, &alike(0));
    assert_eq!(base.iter().map(|&s| u64::from(s)).sum::<u64>(), 1_000_000);
    for d in 1..4 {
        assert!(
            base[d] >= base[d + 1] && base[d] - base[d + 1] <= 1,
            "ties by county order: {base:?}"
        );
    }
    for (what, v) in [
        ("higher wages", utility(4_400.0, 4_000.0, 0.06, 0.06, 3.0)),
        (
            "lower unemployment",
            utility(4_000.0, 4_000.0, 0.04, 0.06, 3.0),
        ),
        ("a shorter trip", utility(4_000.0, 4_000.0, 0.06, 0.06, 1.0)),
    ] {
        let mut u = alike(0);
        u[3] = v;
        let s = destination_ppm(0, &u);
        assert_eq!(
            s.iter().map(|&x| u64::from(x)).sum::<u64>(),
            1_000_000,
            "{what}"
        );
        assert_eq!(s[0], 0, "{what}: the origin");
        assert!(s[3] > base[3], "{what}: {} against {}", s[3], base[3]);
        assert!(
            s[3] > s[1] && s[3] > s[4],
            "{what}: larger than the others {s:?}"
        );
    }
    // Any origin; large and negative utilities.
    let u = [-3_000_000, 2_000_000, 0, 7_000_000, -1];
    for origin in 0..5u8 {
        let s = destination_ppm(origin, &u);
        assert_eq!(
            s.iter().map(|&x| u64::from(x)).sum::<u64>(),
            1_000_000,
            "origin {origin}"
        );
        assert_eq!(s[usize::from(origin)], 0, "origin {origin}");
    }
}
