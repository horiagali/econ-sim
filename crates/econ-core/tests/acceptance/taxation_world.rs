//! Acceptance tests for AC-TAX-08 and AC-TAX-09 (spec: economy/taxation), the
//! M3 `[sim]` criteria, on the `World`'s pay day under `Law::law_2021()`.
//! Written in a test-authoring session (ADR-0014) against the `World` and the
//! test writer's read-only record of the pay day (`World::pay_slips`,
//! feature `pay_day`).

use econ_core::world::{Law, World};
use econ_ledger::{FlowCode, Sector};
use econ_mech_tax::income::tax_wage;
use econ_popgen::NO_EMPLOYER;
use econ_types::Bani;

use super::support::{person_weights, start_state, step};
use super::support_m3::{FlowSum, flow, within, world_2021};

/// Children under 18 in each household at load.
fn children(w: &World) -> Vec<u8> {
    let start = start_state(1000);
    let mut c = vec![0u8; w.households().hh_weight.len()];
    for (p, &h) in start.household_id.iter().enumerate() {
        if start.age[p] < 18 * 12 {
            c[h as usize] = c[h as usize].saturating_add(1);
        }
    }
    c
}

fn sum_w(weights: &[u32], column: &[Bani]) -> Bani {
    super::support::weighted(weights, column)
}

/// **AC-TAX-08** `[sim]` On every pay day, for every person and summed with
/// weights, the employer's payment = net wage + employee contributions +
/// income tax + CAM, and the government and the pension funds receive exactly
/// their parts in the ledger under their flow codes.
#[test]
fn ac_tax_08() {
    let mut w = world_2021(1000);
    let law = Law::law_2021().income.expect("the 2021 law taxes income");
    let kids = children(&w);
    let mut pay_days = 0;
    let mut checked_against_the_rules = 0usize;
    while pay_days < 3 {
        let r = step(&mut w);
        if !r.date.is_month_end() {
            continue;
        }
        pay_days += 1;
        let s = w.pay_slips();
        assert_eq!(s.day, Some(r.day), "the record is of today's pay day");
        let weights = person_weights(&w);
        let n = weights.len();
        for col in [
            &s.gross_wage,
            &s.declared_wage,
            &s.cas_public,
            &s.cas_pillar2,
            &s.cass,
            &s.income_tax,
            &s.cam,
            &s.net_wage,
        ] {
            assert_eq!(col.len(), n, "a column per person");
        }
        let persons = w.persons();
        let units = w.firm_units();
        for p in 0..n {
            let gross = s.gross_wage[p];
            let withheld = s.cas_public[p] + s.cas_pillar2[p] + s.cass[p] + s.income_tax[p];
            // The employer pays gross + CAM = net + contributions + tax + CAM.
            assert_eq!(
                gross + s.cam[p],
                s.net_wage[p] + withheld + s.cam[p],
                "{:?}, person {p}: the employer's payment",
                r.date
            );
            assert!(
                s.declared_wage[p] <= gross,
                "person {p}: declared above gross"
            );
            for (name, v) in [
                ("CAS", s.cas_public[p]),
                ("Pillar II", s.cas_pillar2[p]),
                ("CASS", s.cass[p]),
                ("income tax", s.income_tax[p]),
                ("CAM", s.cam[p]),
                ("net", s.net_wage[p]),
            ] {
                assert!(v >= Bani::ZERO, "person {p}: {name} {v:?}");
            }
            if s.declared_wage[p] == Bani::ZERO {
                assert_eq!(
                    withheld + s.cam[p],
                    Bani::ZERO,
                    "person {p}: nothing declared"
                );
            }
            // A job held all month since day 0: the slip is the law's
            // `tax_wage` on the person's wage and declared wage, with the
            // household's children as dependants (at most four) or none.
            let e = persons.employer[p];
            if e != NO_EMPLOYER && persons.hired_on[p].is_none() && gross == persons.wage[p] {
                let j = usize::from(units.industry[usize::from(e)]);
                let over = law.overrides.iter().find(|&&(k, _)| k == j).map(|(_, o)| o);
                let h = persons.household_id[p] as usize;
                let most = kids[h].min(4);
                let fits = (0..=most).any(|d| {
                    let t = tax_wage(gross, persons.wage_declared[p], d, &law.labour, over);
                    t.cas_public == s.cas_public[p]
                        && t.cas_pillar2 == s.cas_pillar2[p]
                        && t.cass == s.cass[p]
                        && t.income_tax == s.income_tax[p]
                        && t.cam == s.cam[p]
                        && t.net == s.net_wage[p]
                });
                assert!(
                    fits,
                    "{:?}, person {p}: the slip is not the law's tax on a wage of {gross:?}",
                    r.date
                );
                checked_against_the_rules += 1;
            }
        }
        // Summed with weights, and in the ledger under each flow code.
        let gross = sum_w(&weights, &s.gross_wage);
        let net = sum_w(&weights, &s.net_wage);
        let cas = sum_w(&weights, &s.cas_public);
        let pillar2 = sum_w(&weights, &s.cas_pillar2);
        let cass = sum_w(&weights, &s.cass);
        let tax = sum_w(&weights, &s.income_tax);
        let cam = sum_w(&weights, &s.cam);
        assert!(gross > Bani::ZERO, "wages were paid");
        assert_eq!(gross + cam, net + cas + pillar2 + cass + tax + cam);
        let d = r.date;
        assert_eq!(
            flow(&w, FlowCode::WAGES, Sector::Households),
            net,
            "{d:?}: net wages"
        );
        assert_eq!(flow(&w, FlowCode::WAGES, Sector::Firms), Bani::ZERO - net);
        assert_eq!(
            flow(&w, FlowCode::CONTRIB_PENSION, Sector::Government),
            cas,
            "{d:?}: CAS"
        );
        assert_eq!(
            flow(&w, FlowCode::CONTRIB_PENSION, Sector::Firms),
            Bani::ZERO - cas
        );
        assert_eq!(
            flow(&w, FlowCode::CONTRIB_PENSION_PILLAR2, Sector::PensionFunds),
            pillar2,
            "{d:?}: Pillar II"
        );
        assert_eq!(
            flow(&w, FlowCode::CONTRIB_PENSION_PILLAR2, Sector::Government),
            Bani::ZERO,
            "{d:?}: Pillar II is not the government's"
        );
        assert_eq!(
            flow(&w, FlowCode::CONTRIB_HEALTH, Sector::Government),
            cass,
            "{d:?}: CASS"
        );
        assert_eq!(
            flow(&w, FlowCode::TAX_EMPLOYER_CONTRIBUTION, Sector::Government),
            cam,
            "{d:?}: CAM"
        );
        assert_eq!(
            flow(&w, FlowCode::TAX_INCOME, Sector::Firms),
            Bani::ZERO - tax,
            "{d:?}: income tax withheld on wages"
        );
        // The rest of the income tax is the households' own (pensions and
        // declared self-employment income).
        let hh_weights = &w.households().hh_weight;
        let own = sum_w(&weights, &s.pension_tax) + sum_w(hh_weights, &s.self_employment_tax);
        assert_eq!(
            flow(&w, FlowCode::TAX_INCOME, Sector::Government),
            tax + own,
            "{d:?}: income tax received"
        );
    }
    assert!(
        checked_against_the_rules > 0,
        "some slips were checked against the law's rules"
    );
}

/// 2021's revenue (Eurostat, government revenue by tax, `gov_10a_taxag`,
/// Romania, general government S13, 2021, million lei, read 2026-10-20:
/// <https://ec.europa.eu/eurostat/api/dissemination/statistics/1.0/data/gov_10a_taxag?format=JSON&lang=EN&geo=RO&time=2021&sector=S13&unit=MIO_NAC>):
/// - taxes on individual or household income (D51A): 28,493.9;
/// - taxes on the income or profits of corporations (D51B): 26,285.3 (the
///   micro-enterprise tax is a tax on income, D51; medium confidence that it
///   is all in D51B);
/// - actual social contributions: employers' (D611) 11,289.1 + households'
///   (D613) 113,044.9 = 124,334.0. Imputed contributions (D612, 9,598.1) are
///   left out: the `World` has none. Pillar II is not government revenue in
///   ESA 2010, as in the `World`.
///
/// In tenths of a million lei.
const REVENUE_2021: [(&str, i64); 3] = [
    ("income tax", 284_939),
    ("contributions", 1_243_340),
    ("corporate tax", 262_853),
];

/// **AC-TAX-09** `[sim]` On the Romania scenario with the 2021 law, the first
/// twelve months' income tax, contributions and corporate tax are each within
/// 20% of 2021's revenue in budget-2021 (the gap is what the undeclared
/// economy and the simplifications explain; reported, with the tolerance the
/// owner sets).
///
/// The budget fixture holds spending only; 2021's revenue is Eurostat's (the
/// source of the budget's spending), above.
#[test]
fn ac_tax_09() {
    let mut w = world_2021(1000);
    let mut sum = FlowSum::default();
    while w.months().len() < 12 {
        step(&mut w);
        sum.add_day(&w);
    }
    let gov = |c| sum.get(c, Sector::Government);
    let simulated = [
        gov(FlowCode::TAX_INCOME),
        gov(FlowCode::CONTRIB_PENSION)
            + gov(FlowCode::CONTRIB_HEALTH)
            + gov(FlowCode::TAX_EMPLOYER_CONTRIBUTION),
        gov(FlowCode::TAX_PROFIT) + gov(FlowCode::TAX_MICRO),
    ];
    let mut failures = Vec::new();
    for ((name, tenths_of_million), sim) in REVENUE_2021.iter().zip(simulated) {
        let published = Bani::from_lei(tenths_of_million * 100_000);
        let gap_ppm = (i128::from(sim.get()) - i128::from(published.get())) * 1_000_000
            / i128::from(published.get());
        // Reported whatever the outcome (the criterion asks for the report).
        eprintln!(
            "AC-TAX-09 {name}: first twelve months {} million lei, 2021 {} million lei, gap {gap_ppm} ppm",
            sim.get() / 100_000_000,
            published.get() / 100_000_000,
        );
        if !within(sim, published, 200_000) {
            failures.push(format!("{name}: gap {gap_ppm} ppm"));
        }
    }
    assert!(failures.is_empty(), "outside 20% of 2021: {failures:?}");
}
