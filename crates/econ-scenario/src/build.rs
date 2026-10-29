//! Steps 1 to 4 of the spec: read the inputs, run stages A to J, build the
//! firm-unit table and the opening balances, and post them. Roadmap M5's
//! starting data rides along: stages K (kinds of inactivity and enrolment)
//! and L (linked emigrants and the diaspora stock) run after J, and the
//! demographic tables are read and checked; none of them changes stages A to
//! J or the opening balances.

use std::collections::BTreeMap;
use std::path::Path;

use econ_io::people::{read_demography, read_inactivity_margins, read_linked_emigrant_margins};
use econ_io::popgen::{
    MarginFile, StageFile, read_attribute_margins, read_firm_units, read_housing_margins,
    read_income_margins, read_job_margins, read_margins, read_pension_margins, read_wage_margins,
    read_wealth_margins,
};
use econ_ledger::{Instrument, InvariantError, Ledger};
use econ_popgen::{
    AttrParams, AttributeMargins, EmployerParams, FirmUnits, GenParams, HousingMargins,
    HousingParams, InactivityEnrolmentMargins, InactivityParams, IncomeMargins, IncomeParams,
    JobMargins, JobParams, LinkedEmigrantMargins, LinkedEmigrantParams, PensionMargins,
    PensionParams, WageMargins, WageParams, WealthMargins, WealthParams, assign_attributes,
    assign_employers, assign_housing, assign_inactivity_enrolment, assign_jobs,
    assign_linked_emigrants, assign_other_incomes, assign_pensions, assign_wages, assign_wealth,
    generate,
};
use econ_types::Bani;
use serde_json::Value;

use crate::opening::{opening_balances, read_sector_accounts, read_sector_details, reconcile};
use crate::report::report;
use crate::{
    BuildError, BuildParams, FirmUnitTable, Inputs, OpeningBalances, Scenario, StageColumns,
};

/// The input files of the spec's table, in its order.
pub(crate) const INPUT_FILES: [&str; 18] = [
    MARGINS,
    ATTRIBUTES,
    JOBS,
    HOUSING,
    EARNINGS,
    PENSIONS,
    WEALTH,
    INCOMES,
    FIRMS,
    FIRM_SHEETS,
    SHEETS,
    DETAILS,
    IO,
    BUDGET,
    LAW,
    INACTIVITY,
    EMIGRANTS,
    DEMOGRAPHY,
];
const MARGINS: &str = "census2021_margins_ro.json";
const ATTRIBUTES: &str = "census2021_edu_activity_ro.json";
const JOBS: &str = "census2021_jobs_ro.json";
const HOUSING: &str = "census2021_housing_ro.json";
const EARNINGS: &str = "earnings2021_ro.json";
const PENSIONS: &str = "pensions2021_ro.json";
const WEALTH: &str = "household_wealth2021_ro.json";
const INCOMES: &str = "other_incomes2021_ro.json";
pub(crate) const FIRMS: &str = "firm_units2021_ro.json";
pub(crate) const FIRM_SHEETS: &str = "firm_balance_sheets2021_ro.json";
pub(crate) const SHEETS: &str = "balance_sheets2021q4_ro.json";
const DETAILS: &str = "sector_details2021_ro.json";
pub(crate) const IO: &str = "io_table2021_ro.json";
/// The starting budget (spec `economy/budget-2021`), written to the scenario
/// as `budget.json` as it is (ADR-0018 Amendment 1).
pub(crate) const BUDGET: &str = "budget2021_ro.json";
/// The law in force on the start date, by lever id (spec
/// `game/lever-system`, "Default"), written to the scenario as `law.json`
/// and `world.json`.
pub(crate) const LAW: &str = "law2021_ro.json";
/// Stage K (spec `society/population-inactivity-enrolment`).
const INACTIVITY: &str = "inactivity_enrolment2021_ro.json";
/// Stage L (spec `economy/migration-remittances`, "Starting state").
const EMIGRANTS: &str = "linked_emigrants2021_ro.json";
/// The demographic tables (spec `economy/demographics`), written to the
/// scenario as `demography.json` as they are.
pub(crate) const DEMOGRAPHY: &str = "demography2019_ro.json";

/// The margin files of the stages, parsed.
#[derive(Debug, Clone)]
pub struct StageInputs {
    /// Stages A and B.
    pub margins: MarginFile,
    /// Stage C.
    pub attributes: StageFile<AttributeMargins>,
    /// Stage D.
    pub jobs: StageFile<JobMargins>,
    /// Stage E.
    pub housing: StageFile<HousingMargins>,
    /// Stage F.
    pub wages: StageFile<WageMargins>,
    /// Stage G.
    pub pensions: StageFile<PensionMargins>,
    /// Stage H.
    pub wealth: StageFile<WealthMargins>,
    /// Stage I.
    pub incomes: StageFile<IncomeMargins>,
    /// Stage J: the firm units.
    pub firms: StageFile<FirmUnits>,
    /// Stage K.
    pub inactivity: StageFile<InactivityEnrolmentMargins>,
    /// Stage L.
    pub emigrants: StageFile<LinkedEmigrantMargins>,
}

fn input_err(file: &str, why: impl std::fmt::Display) -> BuildError {
    BuildError::Input {
        file: file.to_string(),
        why: why.to_string(),
    }
}

/// Read and check every input of the spec's table.
pub(crate) fn read_inputs(dir: &Path) -> Result<Inputs, BuildError> {
    let mut files = Vec::with_capacity(INPUT_FILES.len());
    let mut docs = BTreeMap::new();
    for name in INPUT_FILES {
        let path = dir.join(name);
        let bytes = std::fs::read(&path).map_err(|e| input_err(name, e))?;
        let text = std::str::from_utf8(&bytes).map_err(|e| input_err(name, e))?;
        let doc: Value = serde_json::from_str(text).map_err(|e| input_err(name, e))?;
        docs.insert(name, doc);
        files.push((name.to_string(), bytes));
    }
    let text = |name: &str| -> &str {
        let bytes = &files.iter().find(|(n, _)| n == name).expect("read above").1;
        std::str::from_utf8(bytes).expect("checked above")
    };
    let margins = read_margins(text(MARGINS)).map_err(|e| input_err(MARGINS, format!("{e:?}")))?;
    let attributes = read_attribute_margins(text(ATTRIBUTES), &margins.margins.counties)
        .map_err(|e| input_err(ATTRIBUTES, format!("{e:?}")))?;
    let jobs = read_job_margins(text(JOBS), &attributes.margins)
        .map_err(|e| input_err(JOBS, format!("{e:?}")))?;
    read_demography(text(DEMOGRAPHY)).map_err(|e| input_err(DEMOGRAPHY, format!("{e:?}")))?;
    let stages = StageInputs {
        inactivity: read_inactivity_margins(text(INACTIVITY))
            .map_err(|e| input_err(INACTIVITY, format!("{e:?}")))?,
        emigrants: read_linked_emigrant_margins(text(EMIGRANTS), &attributes.margins.regions)
            .map_err(|e| input_err(EMIGRANTS, format!("{e:?}")))?,
        housing: read_housing_margins(text(HOUSING))
            .map_err(|e| input_err(HOUSING, format!("{e:?}")))?,
        wages: read_wage_margins(text(EARNINGS))
            .map_err(|e| input_err(EARNINGS, format!("{e:?}")))?,
        pensions: read_pension_margins(text(PENSIONS))
            .map_err(|e| input_err(PENSIONS, format!("{e:?}")))?,
        wealth: read_wealth_margins(text(WEALTH))
            .map_err(|e| input_err(WEALTH, format!("{e:?}")))?,
        incomes: read_income_margins(text(INCOMES))
            .map_err(|e| input_err(INCOMES, format!("{e:?}")))?,
        firms: read_firm_units(text(FIRMS)).map_err(|e| input_err(FIRMS, format!("{e:?}")))?,
        margins,
        attributes,
        jobs,
    };
    let accounts =
        read_sector_accounts(text(SHEETS)).map_err(|e| input_err(SHEETS, format!("{e:?}")))?;
    let details =
        read_sector_details(text(DETAILS)).map_err(|e| input_err(DETAILS, format!("{e:?}")))?;
    let firm_units = firm_unit_rows(&docs[FIRMS], &docs[FIRM_SHEETS])?;
    Ok(Inputs {
        files,
        accounts,
        details,
        stages,
        docs,
        firm_units,
    })
}

/// The firm units of the firm file joined with their balance sheets; every
/// column but `employees`, which needs the population.
fn firm_unit_rows(firms: &Value, sheets: &Value) -> Result<FirmUnitTable, BuildError> {
    let bad = |why: String| input_err(FIRMS, why);
    let industries: Vec<(&str, u8)> = firms["industries"]
        .as_array()
        .ok_or_else(|| bad("no industries".into()))?
        .iter()
        .map(|i| {
            let id = i["id"]
                .as_str()
                .ok_or_else(|| bad("an industry id".into()))?;
            let group = i["industry_group"]
                .as_u64()
                .and_then(|g| u8::try_from(g).ok())
                .ok_or_else(|| bad(format!("the group of {id}")))?;
            Ok((id, group))
        })
        .collect::<Result<_, BuildError>>()?;
    let units = firms["units"]
        .as_array()
        .ok_or_else(|| bad("no units".into()))?;
    let code = |v: &Value, names: &[&str], what: &str| -> Result<u8, BuildError> {
        v.as_str()
            .and_then(|s| names.iter().position(|&n| n == s))
            .and_then(|k| u8::try_from(k).ok())
            .ok_or_else(|| bad(format!("{what} {v}")))
    };
    let lei = |v: &Value, what: &str| -> Result<u64, BuildError> {
        v.as_u64()
            .and_then(|x| x.checked_mul(100))
            .ok_or_else(|| bad(format!("{what} is not whole lei")))
    };
    let ppm = |v: &Value| -> Result<u32, BuildError> {
        v.as_u64()
            .and_then(|x| u32::try_from(x).ok())
            .ok_or_else(|| bad("an ownership share".into()))
    };
    let mut t = FirmUnitTable::default();
    for u in units {
        let id = u["id"].as_str().ok_or_else(|| bad("a unit id".into()))?;
        let industry = u["industry"]
            .as_str()
            .and_then(|s| industries.iter().position(|&(i, _)| i == s))
            .ok_or_else(|| bad(format!("the industry of {id}")))?;
        t.unit_id.push(id.to_string());
        t.industry
            .push(u8::try_from(industry).map_err(|_| bad("too many industries".into()))?);
        t.industry_group.push(industries[industry].1);
        t.size.push(code(
            &u["size"],
            &["micro", "small", "large", "public"],
            "size",
        )?);
        t.kind
            .push(code(&u["kind"], &["cohort", "public"], "kind")?);
        t.controller.push(code(
            &u["controller"],
            &["domestic", "foreign", "state"],
            "controller",
        )?);
        t.firm_count.push(
            u["firm_count"]
                .as_u64()
                .and_then(|x| u32::try_from(x).ok())
                .ok_or_else(|| bad(format!("the firm count of {id}")))?,
        );
        t.persons_employed.push(
            u["persons_employed"]
                .as_u64()
                .ok_or_else(|| bad(format!("the persons employed of {id}")))?,
        );
        t.output.push(lei(&u["output"], "output")?);
        t.value_added.push(lei(&u["value_added"], "value added")?);
        t.wages.push(lei(&u["wages"], "wages")?);
        t.own_state.push(ppm(&u["ownership"]["state"])?);
        t.own_domestic_private
            .push(ppm(&u["ownership"]["domestic_private"])?);
        t.own_foreign.push(ppm(&u["ownership"]["foreign"])?);
    }
    let n = t.unit_id.len();
    let mut stocks: [Vec<u64>; 12] = std::array::from_fn(|_| vec![0; n]);
    let sheet_units = sheets["units"]
        .as_array()
        .ok_or_else(|| input_err(FIRM_SHEETS, "no units"))?;
    for s in sheet_units {
        let id = s["id"]
            .as_str()
            .ok_or_else(|| input_err(FIRM_SHEETS, "a unit id"))?;
        let f = t
            .unit_id
            .iter()
            .position(|u| u == id)
            .ok_or_else(|| input_err(FIRM_SHEETS, format!("{id} is not a firm unit")))?;
        for (column, name) in stocks.iter_mut().zip(STOCKS) {
            column[f] = s[name]
                .as_u64()
                .and_then(|x| x.checked_mul(100))
                .ok_or_else(|| input_err(FIRM_SHEETS, format!("{name} of {id}")))?;
        }
    }
    let [
        fixed_assets,
        currency,
        deposits,
        debt_securities_held,
        loans_granted,
        equity_held,
        insurance,
        other_receivable,
        debt_securities_issued,
        loans,
        equity_issued,
        other_payable,
    ] = stocks;
    Ok(FirmUnitTable {
        fixed_assets,
        currency,
        deposits,
        debt_securities_held,
        loans_granted,
        equity_held,
        insurance,
        other_receivable,
        debt_securities_issued,
        loans,
        equity_issued,
        other_payable,
        ..t
    })
}

/// The stock columns, named as in `firm_balance_sheets2021_ro.json`.
pub(crate) const STOCKS: [&str; 12] = [
    "fixed_assets",
    "currency",
    "deposits",
    "debt_securities_held",
    "loans_granted",
    "equity_held",
    "insurance",
    "other_receivable",
    "debt_securities_issued",
    "loans",
    "equity_issued",
    "other_payable",
];

/// The weighted sum of a household column, in bani.
pub(crate) fn weighted(hh_weight: &[u32], column: &[u64]) -> i128 {
    hh_weight
        .iter()
        .zip(column)
        .map(|(&w, &v)| i128::from(w).saturating_mul(i128::from(v)))
        .sum()
}

fn stage_err(stage: &str, e: impl std::fmt::Debug) -> BuildError {
    BuildError::Stage(format!("{stage}: {e:?}"))
}

fn bani(v: i128) -> Result<Bani, BuildError> {
    i64::try_from(v)
        .map(Bani)
        .map_err(|_| BuildError::Stage("a household total does not fit i64 bani".into()))
}

/// Steps 2 to 5.
pub(crate) fn build_with_hook(
    inputs: &Inputs,
    params: &BuildParams,
    hook: &dyn Fn(&mut OpeningBalances),
) -> Result<Scenario, BuildError> {
    let (scale, seed) = (params.sample_scale, params.rng_seed);
    let s = &inputs.stages;
    let population = generate(&s.margins.margins, &GenParams::new(scale, seed))
        .map_err(|e| stage_err("population", e))?;
    let pop = &population;
    let attributes = assign_attributes(pop, &s.attributes.margins, &AttrParams::new(seed))
        .map_err(|e| stage_err("attributes", e))?;
    let jobs = assign_jobs(pop, &attributes, &s.jobs.margins, &JobParams::new(seed))
        .map_err(|e| stage_err("jobs", e))?;
    let housing = assign_housing(pop, &s.housing.margins, &HousingParams::new(seed))
        .map_err(|e| stage_err("housing", e))?;
    let wages = assign_wages(
        pop,
        &attributes,
        &jobs,
        &s.wages.margins,
        &WageParams::new(seed),
    )
    .map_err(|e| stage_err("wages", e))?;
    let pensions = assign_pensions(
        pop,
        &attributes,
        &s.pensions.margins,
        &PensionParams::new(seed),
    )
    .map_err(|e| stage_err("pensions", e))?;
    let wealth = assign_wealth(
        pop,
        &housing,
        Some(&wages),
        Some(&pensions),
        &s.wealth.margins,
        &WealthParams::new(seed),
    )
    .map_err(|e| stage_err("wealth", e))?;
    let incomes = assign_other_incomes(
        pop,
        &attributes,
        &jobs,
        &housing,
        &wages,
        &s.incomes.margins,
        &IncomeParams::new(seed),
    )
    .map_err(|e| stage_err("incomes", e))?;
    let employers = assign_employers(
        pop,
        &jobs,
        Some(&wages),
        &s.firms.margins,
        &EmployerParams::new(seed),
    )
    .map_err(|e| stage_err("employers", e))?;
    // Roadmap M5's starting data, read by no mechanic yet.
    let inactivity = assign_inactivity_enrolment(
        pop,
        &attributes,
        &s.inactivity.margins,
        &InactivityParams::new(seed),
    )
    .map_err(|e| stage_err("inactivity_enrolment", e))?;
    let emigrants = assign_linked_emigrants(
        pop,
        &s.attributes.margins.region_of_county,
        s.attributes.margins.regions.len(),
        &housing,
        &s.emigrants.margins,
        &LinkedEmigrantParams::new(seed),
    )
    .map_err(|e| stage_err("linked_emigrants", e))?;

    // Step 3: the firm units' employees.
    let mut firm_units = inputs.firm_units.clone();
    firm_units.employees = vec![0; firm_units.unit_id.len()];
    let hh_weight = &pop.households.hh_weight;
    for (&h, &e) in pop.persons.household_id.iter().zip(&employers.employer) {
        if let Some(n) = firm_units.employees.get_mut(usize::from(e)) {
            *n = n.saturating_add(u64::from(hh_weight[h as usize]));
        }
    }

    // Step 4: the opening balances, reconciled with the households, posted.
    let opening = opening_balances(&inputs.accounts).map_err(BuildError::Opening)?;
    let mut opening = reconcile(
        &opening,
        bani(weighted(hh_weight, &wealth.hh_cash))?,
        bani(weighted(hh_weight, &wealth.hh_deposits))?,
    )
    .map_err(BuildError::Opening)?;
    hook(&mut opening);
    let rows: Vec<InvariantError> = Instrument::ALL
        .into_iter()
        .zip(&opening.balances)
        .filter_map(|(instrument, row)| {
            let sum: Bani = row.iter().copied().sum();
            (!sum.is_zero()).then_some(InvariantError::RowNotZero { instrument, sum })
        })
        .collect();
    if !rows.is_empty() {
        return Err(BuildError::Invariants(rows));
    }
    let mut ledger = Ledger::new();
    opening
        .post(&mut ledger)
        .map_err(|e| BuildError::Stage(format!("posting the opening balances: {e:?}")))?;
    ledger.check_invariants().map_err(BuildError::Invariants)?;

    let stages = StageColumns {
        attributes,
        jobs,
        housing,
        wages,
        pensions,
        wealth,
        incomes,
        employers,
        inactivity,
        emigrants,
    };
    let mut scenario = Scenario {
        params: *params,
        population,
        stages,
        firm_units,
        opening,
        ledger,
        report: crate::Report::default(),
    };
    scenario.report = report(inputs, &scenario);
    Ok(scenario)
}
