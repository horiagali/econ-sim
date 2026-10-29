//! The real `World` (spec `game/world-loop`, roadmap M2): a scenario loaded
//! into column tables, advanced one day per [`World::step`], the phases of
//! the day run in their fixed order when their period is due
//! ([`crate::schedule`]), every leu moved through the ledger and the
//! invariants checked every day.
//!
//! Beyond the spec's sketch, for the tests: the read-only column tables
//! ([`HouseholdColumns`], [`PersonColumns`], [`FirmUnitColumns`],
//! [`IndustryColumns`]), [`WorldParams::spec_defaults`],
//! [`World::inflation_target_ppm`], and [`StartEdit`], the edits of a
//! starting state by which two worlds are compared where a criterion needs a
//! change that M2 has no lever for.
//!
//! Money columns are whole bani per record (a household's amounts before its
//! `hh_weight`; a firm unit's are already totals). Quantities are real units:
//! bani at starting prices. Prices and shares are millionths.
//!
//! Choices of this first version where the specs leave room:
//! - `employer` is employment by a firm unit as an employee. Employers,
//!   own-account and family workers keep their status (labour market, M2
//!   simplification) and earn their business's self-employment income; the
//!   unit stage J gave them is not an employer in the `World`.
//! - The normal tightness of the January wage round (θ*) is the tightness of
//!   normal turnover in the starting economy: vacancies of one month of quits
//!   over the qualified seekers at load.
//! - Households buy the imported part of each good (its share in the
//!   input-output table's household column) from the rest of the world at the
//!   starting price, through the seller industry's clearing account; the
//!   domestic part is served from inventories.
//! - Households pay the legal VAT rates less Romania's VAT gap (36.7%): the
//!   input-output table's taxes on household consumption are what was
//!   collected. Who evades is the informal economy's (M15).
//! - Fixed investment includes the table's change in inventories: capital
//!   formation, bought by firms.
//! - A unit holds stocks of its essential inputs, a target of 60 days of use
//!   (the firm units' materials and goods for resale are about 2.4 months of
//!   inputs at the start), ordering its daily use and a thirtieth of the gap
//!   to the target; an essential input is short, and cuts output, only when
//!   its stock runs out (production, edge cases). Other inputs are bought as
//!   used.
//! - An input is essential when it is at least 1% of the buyer's output.
//! - Staff is counted in whole records, so a unit's staff meets its
//!   requirement when it is within half of one of its records; matching
//!   reaches the job seekers of the regions where the unit has staff.
//! - Cohort units pay the investment goods of the fixed final demand in
//!   proportion to their starting surplus, out of what they can spare beyond
//!   a month of wages; the domestic cohort units pay self-employment and
//!   informal income the same way. Cash takings are paid into the bank the
//!   same day.
//! - The clock starts just after November's pay day: on day 0, before the
//!   first phase, each household is paid the starting month's income of its
//!   members by their payers, so that the income its first decision counts as
//!   received is in its accounts ("Starting the clock").
//! - The government pays from its deposits at the central bank, which may go
//!   below zero in M2 (the central bank finances it until bonds exist, M3).
//! - A public producer short of money is paid the shortfall by the
//!   government, as non-market output valued at cost (ADR-0016). A cohort
//!   unit short of money borrows it from the spare money of the other units
//!   and repays out of later revenue: credit inside the firm sector, which
//!   leaves the sector's balances unchanged (banks lend from M3).

// The phases walk several aligned columns by record index.
#[cfg(feature = "banking")]
#[allow(clippy::needless_range_loop)]
mod banking;
#[cfg(feature = "monetary")]
mod central_bank;
// Ministry budgets and the money levers (spec game/lever-system).
#[cfg(feature = "levers")]
#[allow(clippy::needless_range_loop)]
mod budget;
#[allow(clippy::needless_range_loop)]
mod close;
#[allow(clippy::needless_range_loop)]
mod fiscal;
#[cfg(feature = "trade")]
#[allow(clippy::needless_range_loop)]
mod foreign;
#[allow(clippy::needless_range_loop)]
mod goods;
// Backstop: firms that cannot pay (owner, 2026-10-26).
#[allow(clippy::needless_range_loop)]
mod insolvency;
#[cfg(feature = "investment")]
#[allow(clippy::needless_range_loop)]
mod investment;
#[allow(clippy::needless_range_loop)]
mod labour;
#[allow(clippy::needless_range_loop)]
mod load;
mod money;
#[cfg(feature = "levers")]
mod policy;
// The test writer's additions of M3's investment and revenue (ADR-0014).
#[cfg(any(feature = "investment", feature = "revenue"))]
mod observe;
mod pay_day;
#[cfg(feature = "revenue")]
#[allow(clippy::needless_range_loop)]
mod revenue;
#[cfg(feature = "fiscal")]
mod treasury;
// The test writer's additions of roadmap M5 (ADR-0014): education's first
// version, migration, and the person columns their criteria read.
#[cfg(feature = "education")]
pub mod education;
#[cfg(feature = "migration")]
mod migration;
#[cfg(any(
    feature = "migration",
    feature = "education",
    feature = "approval",
    feature = "poverty"
))]
mod persons_view;

use econ_ledger::{InvariantError, Ledger};
use econ_mech_households::HouseholdParams;
use econ_mech_labour::LabourParams;
use econ_mech_prices::PriceParams;
use econ_mech_production::ProductionParams;
use econ_mech_tax::vat::{VatCategory, VatRate, VatSchedule};
use econ_popgen::EduLevel;
use econ_rng::KeyedRng;
use econ_types::{Bani, Calendar, Date, Day, Month};

use crate::Command;
use crate::schedule::{Due, Period};

#[cfg(feature = "banking")]
pub use banking::BankState;
#[cfg(feature = "monetary")]
pub use central_bank::{CentralBankState, MonetaryParams};
pub use fiscal::{IncomeTaxLaw, TransferLaw};
#[cfg(feature = "trade")]
pub use foreign::{Foreign, TradeMonth, TradeParams};
pub use insolvency::FirmUnpaid;
#[cfg(feature = "investment")]
pub use investment::InvestmentState;
#[cfg(feature = "migration")]
pub use migration::{DiasporaView, MIGRATION_AGE_BANDS, MigrationMonth, StockCell};
#[cfg(feature = "revenue")]
pub use observe::{
    HeldBenefit, HeldPayment, ImputedRentPayment, RevenueDay, RevenueMonth, eu_month,
};
#[cfg(feature = "investment")]
pub use observe::{InvestmentMonth, InvestmentPlans};
pub use pay_day::PaySlips;
#[cfg(feature = "fiscal")]
pub use treasury::{FiscalMonth, FiscalParams, Treasury};

/// What a `World` starts from: the population and stage columns, the firm-unit
/// table, the opening balances, the input coefficients, the basket shares and
/// the placeholder final demand. Read from a scenario folder by
/// `econ_io::read_start_state`. Money in bani; flows of the firm units and
/// the final demand are a year's; coefficients and shares are millionths.
#[derive(Debug, Clone, Default)]
#[allow(missing_docs)]
pub struct StartState {
    /// The scenario's `sample_scale`.
    pub sample_scale: u32,
    /// The scenario's `rng_seed`.
    pub rng_seed: u64,
    // Households.
    pub hh_weight: Vec<u32>,
    pub hh_county: Vec<u8>,
    /// A collective household (institution).
    pub hh_collective: Vec<bool>,
    /// The region of the household's county.
    pub hh_region: Vec<u8>,
    pub hh_cash: Vec<Bani>,
    pub hh_deposits: Vec<Bani>,
    /// Food grown at home, a month (in kind).
    pub hh_subsistence_income: Vec<Bani>,
    // Persons.
    pub household_id: Vec<u32>,
    /// Age in months.
    pub age: Vec<u16>,
    pub edu_level: Vec<EduLevel>,
    /// Stage C's activity code (4 = unemployed).
    pub activity: Vec<u8>,
    /// Stage D's status in employment (0 none, 1 employee, 2 employer, 3
    /// own-account, 4 family worker).
    pub employment_status: Vec<u8>,
    pub occupation: Vec<u8>,
    pub wage: Vec<Bani>,
    pub wage_declared: Vec<Bani>,
    pub pension: Vec<Bani>,
    /// Self-employment and informal income, a month.
    pub other_income: Vec<Bani>,
    /// The declared part of self-employment income, a month.
    pub self_employment_declared: Vec<Bani>,
    /// Stage J's firm unit, or `econ_popgen::NO_EMPLOYER`.
    pub employer: Vec<u16>,
    /// Stage K's level of enrolment (spec society/education, first version):
    /// the column `enrolled` of `persons.arrow`.
    #[cfg(feature = "education")]
    pub enrolled: Vec<econ_popgen::Enrolled>,
    // Firm units, in the order of the scenario's table.
    pub unit_industry: Vec<u8>,
    /// A cohort unit (not a public producer).
    pub unit_cohort: Vec<bool>,
    /// Size class: micro 0, small 1, large 2, public 3.
    pub unit_size: Vec<u8>,
    /// Real firms the unit stands for.
    pub unit_firm_count: Vec<u32>,
    /// A cohort unit controlled from Romania (not public, not foreign).
    pub unit_domestic_cohort: Vec<bool>,
    pub unit_output: Vec<Bani>,
    pub unit_value_added: Vec<Bani>,
    pub unit_wages: Vec<Bani>,
    pub unit_currency: Vec<Bani>,
    pub unit_deposits: Vec<Bani>,
    /// The opening balance sheet, `[instrument][sector]` in matrix order.
    pub balances: Vec<[Bani; 8]>,
    // Catalogue industries.
    pub industry_id: Vec<String>,
    pub industry_group: Vec<u8>,
    /// Storable goods (one month of inventory); the rest a quarter month.
    pub goods: Vec<bool>,
    /// `price_flexibility`, a month.
    pub flexibility_ppm: Vec<u32>,
    /// `[buyer][supplier]`: home inputs per unit of the buyer's output.
    pub home_coefficient_ppm: Vec<Vec<u32>>,
    /// Imported inputs per unit of output.
    pub import_coefficient_ppm: Vec<u32>,
    /// Other costs (taxes on production and on inputs) per unit of output.
    pub other_cost_ppm: Vec<u32>,
    /// `basket_share[g]`, summing to 1,000,000.
    pub basket_ppm: Vec<u32>,
    /// The imported part of the households' purchases of each good.
    pub basket_import_ppm: Vec<u32>,
    pub gov_domestic: Vec<Bani>,
    pub gov_imported: Vec<Bani>,
    pub investment_domestic: Vec<Bani>,
    pub investment_imported: Vec<Bani>,
    pub exports: Vec<Bani>,
    /// Government bonds by residual maturity at the start, as (from year, to
    /// year, amount) buckets of the debt profile; empty when the scenario has
    /// none (spec `economy/fiscal-policy`, "Starting state").
    pub gov_debt_profile: Vec<(u8, u8, Bani)>,
    /// The budget lines the `World` pays as a twelfth of the year at each
    /// month end until a lever changes them: flow code, the sector paid and
    /// the yearly amount (spec `economy/fiscal-policy`, "In", step 1); empty
    /// when the scenario has no budget.
    pub gov_transfers_year: Vec<(econ_ledger::FlowCode, econ_ledger::Sector, Bani)>,
    /// Each budget line's 2021 wage cell (D11) split over the public units
    /// its COFOG subgroups belong to, by their own D11 totals: (line id,
    /// public unit id, D11 a year), in the budget's line order (spec
    /// `game/lever-system`, "How the `World` carries it"); empty when the
    /// scenario has no budget.
    pub budget_wage_shares: Vec<(String, String, Bani)>,
    /// Each firm unit's loans in the firm-unit table (all lenders).
    pub unit_loans: Vec<Bani>,
    /// Each household's mortgage (stage H).
    pub hh_mortgage: Vec<Bani>,
    /// Each household's consumer debt (stage H).
    pub hh_consumer_debt: Vec<Bani>,
    /// Each firm unit's fixed assets (its firms' balance sheets).
    pub unit_fixed_assets: Vec<Bani>,
    /// Each unit's controller: domestic 0, foreign 1, state 2.
    pub unit_controller: Vec<u8>,
    /// Each unit's ownership shares, millionths: the state, domestic private
    /// owners, foreign owners.
    pub unit_ownership_ppm: Vec<[u32; 3]>,
    /// The government's fixed investment and land a year (the budget's
    /// `investment` and `non_produced_assets` lines).
    pub gov_investment_year: Bani,
    /// Each household's tenure: owner 0, tenant 1, other 2 (stage E).
    pub hh_tenure: Vec<u8>,
    /// Each household's dwelling value (stage H).
    pub hh_dwelling_value: Vec<Bani>,
    /// Each industry's product taxes on inputs over its input purchases,
    /// millionths (spec economy/government-revenue-2021, part 1).
    pub product_tax_ppm: Vec<u32>,
    /// Each industry's other taxes on production over its output, millionths
    /// (part 2; subsidies left out).
    pub production_tax_ppm: Vec<u32>,
    /// The product taxes on investment over its value, millionths.
    pub investment_product_tax_ppm: u32,
    /// Each industry's imported refined petroleum over its imported inputs,
    /// millionths (spec economy/excise).
    pub fuel_import_ppm: Vec<u32>,
    /// Each industry's imported oil (crude and refined petroleum) over its
    /// imported inputs, millionths (`world.fuel_price[oil]`).
    pub oil_import_ppm: Vec<u32>,
    edits: Vec<StartEdit>,
}

/// A change to a starting state, applied by [`World::new`] after it has
/// derived the starting values of the spec ("Starting the clock" and the
/// mechanics' starting states), so that a test can compare two worlds that
/// differ only in it. Industry indices are catalogue indices, as in
/// [`IndustryColumns`]; household indices are those of [`HouseholdColumns`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StartEdit {
    /// A one-off transfer: each household receives this amount (per record,
    /// before its weight) on day 0 before the first phase, paid by the
    /// government as pensions are (the three legs of the spec) and counted in
    /// its `hh_income_received` (AC-HH-05).
    Transfer(Vec<Bani>),
    /// Every unit of the industry starts with no inventory and no capacity:
    /// it neither produces nor sells (AC-HH-07).
    ShutIndustry(usize),
    /// Every unit of the industry starts with its inventory times
    /// `factor_ppm` / 1,000,000; its inventory target ratio is unchanged
    /// (AC-PRICE-07).
    ScaleInventory {
        /// Catalogue index.
        industry: usize,
        /// Millionths.
        factor_ppm: u32,
    },
    /// Every unit's labour requirement per unit of output times
    /// `factor_ppm` / 1,000,000, for the whole run: its desired employment
    /// rises in that proportion (AC-LM-06).
    ScaleLabourRequirement {
        /// Millionths.
        factor_ppm: u32,
    },
    /// The household basket shares by catalogue industry, in millionths
    /// summing to 1,000,000, in place of the input-output table's (AC-PROD-07).
    BasketShares(Vec<u32>),
    /// Every person's wage and declared wage, and every reference wage, times
    /// `factor_ppm` / 1,000,000, after the starting markups and unit costs are
    /// set: a wage rise on day 0 (AC-PRICE-05).
    ScaleWages {
        /// Millionths.
        factor_ppm: u32,
    },
    /// On day 0, before the first phase, every unit of the industry lays off
    /// this share of its employees (rounded per unit), as layoffs: they become
    /// unemployed job seekers and their unemployment claims open on their
    /// wage (AC-ST-08).
    #[cfg(feature = "pay_day")]
    Layoff {
        /// Catalogue index.
        industry: usize,
        /// Millionths.
        share_ppm: u32,
    },
    /// The policy rate becomes `rate_ppm` from the day given, as a command of
    /// the lever system would set it (M4); the banks reprice at the next
    /// monthly close (AC-BANK-05).
    #[cfg(feature = "banking")]
    PolicyRate {
        /// The first day of the new rate.
        from: Day,
        /// Millionths a year.
        rate_ppm: i32,
    },
    /// On day 0, before the first phase, the firm unit borrows this amount
    /// from the banks: its loans and its deposits grow by it (AC-BANK-06).
    #[cfg(feature = "banking")]
    ExtraLoan {
        /// Index into the firm units.
        unit: usize,
        /// Bani.
        amount: Bani,
    },
    /// For the first `months` months the government's deficit is financed by
    /// the central bank (its deposits at the central bank may fall below
    /// zero, or the central bank buys the month's issue) instead of by bonds
    /// sold to the other holders (AC-MP-05).
    #[cfg(feature = "monetary")]
    CentralBankFinancing {
        /// Months from the start.
        months: u32,
    },
    /// Every good's euro price in the outside world times `factor_ppm` /
    /// 1,000,000, for the whole run: a world-constants edit (AC-TFX-04).
    #[cfg(feature = "trade")]
    WorldPrices {
        /// Millionths.
        factor_ppm: u32,
    },
    /// A permanent depreciation of the leu: on day 0, before the first
    /// phase, the exchange rate `e` (lei per euro) becomes its starting value
    /// times `factor_ppm` / 1,000,000, while `e0`, the base of the index
    /// `e / e0`, keeps the scenario's starting rate; from then on `e` moves
    /// by trade's monthly rule from the new level (AC-EUF-05). A test
    /// writer's addition (ADR-0014).
    #[cfg(feature = "trade")]
    Depreciation {
        /// Millionths.
        factor_ppm: u32,
    },
    /// The world runs without one part of the 2021 law's revenue, for the
    /// whole run, so that a test can compare it with the world that has it
    /// (AC-EXC-07, AC-GRV-03, AC-IMP-03 and 04). A test writer's addition
    /// (ADR-0014).
    #[cfg(feature = "revenue")]
    WithoutRevenue(RevenuePart),
    /// The world starts with no diaspora, as a scenario without the
    /// diaspora tables (spec economy/migration-remittances, "Starting
    /// state"): no linked emigrants (the starting population's `abroad`
    /// persons are left out) and an empty stock, so no remittances;
    /// emigration, returns, immigration and moves still run (the
    /// counterfactual of AC-MIG-07 and 14). A test writer's addition
    /// (ADR-0014).
    #[cfg(feature = "migration")]
    NoDiaspora,
    /// A scripted shock (AC-MIG-15): for the whole run, the unemployment
    /// rate the migration rules read for a development region (emigration,
    /// and for moves between counties both the origin's push factor and the
    /// destination choice) is its measured rate plus `points_ppm`. A test
    /// writer's addition (ADR-0014).
    #[cfg(feature = "migration")]
    MigrationUnemploymentShock {
        /// The development region.
        region: u8,
        /// Millionths (50,000 = 5 points).
        points_ppm: u32,
    },
}

/// A part of the 2021 law's revenue that [`StartEdit::WithoutRevenue`] leaves
/// out.
#[cfg(feature = "revenue")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RevenuePart {
    /// Excise duties (spec economy/excise): no duty is charged, and other
    /// costs keep the starting duty as the table has it (the world before
    /// excise was charged).
    Excise,
    /// Product taxes on firms' purchases and other taxes on production
    /// (spec economy/government-revenue-2021, parts 1 and 2).
    ProductTaxes,
    /// The imputed rent paid back to owner-occupiers (spec
    /// economy/imputed-rent).
    ImputedRent,
}

impl StartState {
    /// Record an edit; [`World::new`] applies the edits in the order given
    /// and refuses an edit that does not fit the tables with
    /// [`WorldError::Shape`].
    pub fn apply(&mut self, edit: StartEdit) {
        self.edits.push(edit);
    }
}

/// The law in force. Until `law.json` exists (M4) the M2 tests pass
/// [`Law::placeholder_2021`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Law {
    /// The VAT rates.
    pub vat: VatSchedule,
    /// Each catalogue industry's good's VAT category.
    pub vat_category: Vec<VatCategory>,
    /// The gross minimum wage a month.
    pub minimum_wage: Bani,
    /// Taxes on income and profits; `None` in M2's placeholder law.
    pub income: Option<IncomeTaxLaw>,
    /// Social transfers; `None` in M2's placeholder law (pensions are then
    /// paid at their starting amounts).
    pub transfers: Option<TransferLaw>,
    /// `excise_duty[g]`: each catalogue industry's excise duty per unit,
    /// millionths of its starting price (0 for a good without one; spec
    /// `economy/excise`). A change takes the place of the indexed duty.
    pub excise_duty_ppm: Vec<u32>,
    /// `property_tax_rate`, basis points of the dwelling value a year. `None`:
    /// M3's rule, 2021's yearly total shared equally by owner households
    /// (spec `economy/government-revenue-2021`); the law of `law.json` has
    /// the rate (spec `game/lever-system`).
    pub property_tax_bp: Option<u32>,
    /// `wealth_tax_threshold`: the household net worth above which the
    /// wealth tax is due.
    pub wealth_tax_threshold: Bani,
    /// `wealth_tax_rate`, basis points a year (0: none, as in 2021).
    pub wealth_tax_bp: u32,
}

/// The excise duties of 1 December 2021, millionths of the starting price per
/// unit (spec `economy/excise`, "Starting state"): fuel, tobacco, alcohol.
pub const EXCISE_2021: [(&str, u32); 3] = [
    ("refined_petroleum", 512_500),
    ("tobacco", 2_030_400),
    ("beverages", 71_200),
];

/// The catalogue industries in the order of the firm-unit file, with the VAT
/// category of their good on 1 December 2021 (spec `economy/vat`: 9% for
/// food, medicines and water; 5% for books and newspapers, cultural events,
/// accommodation and restaurants; financial services, health, education,
/// social care, rents and public services exempt).
pub const CATALOGUE_VAT: [(&str, VatCategory); 90] = {
    use VatCategory::{Exempt as E, Reduced as R, Reduced2 as R2, Standard as S};
    [
        ("admin_support", S),
        ("agri_cereals", R),
        ("agri_construction_machinery", S),
        ("agri_fruit_veg", R),
        ("agri_oilseeds", R),
        ("agri_vineyards", R),
        ("air_transport", S),
        ("aircraft", S),
        ("banking_services", E),
        ("beverages", S),
        ("building_materials", S),
        ("car_parts", S),
        ("cars", S),
        ("chemicals", S),
        ("coal_mining", S),
        ("computers_comms", S),
        ("construction_buildings", S),
        ("construction_civil", S),
        ("consumer_electronics", S),
        ("crude_oil", S),
        ("data_centres", S),
        ("defence_equipment", S),
        ("defence_services", E),
        ("education_services", E),
        ("electrical_equipment", S),
        ("electronic_components", S),
        ("engineering_rnd", S),
        ("fertilisers", S),
        ("fishing", R),
        ("food_bakery_milling", R),
        ("food_meat_dairy", R),
        ("food_other", R),
        ("forestry", S),
        ("furniture", S),
        ("gas_heat_supply", S),
        ("health_services", E),
        ("hotels_tourism", R2),
        ("industrial_machinery", S),
        ("insurance_pensions", E),
        ("it_software", S),
        ("leather_footwear", S),
        ("livestock_dairy", R),
        ("livestock_meat", R),
        ("media_publishing", R2),
        ("medical_devices", S),
        ("metal_ores", S),
        ("metal_products", S),
        ("natural_gas", S),
        ("nonferrous_metals", S),
        ("other_manufacturing", S),
        ("paper_printing", S),
        ("passenger_transport", S),
        ("personal_services", S),
        ("pharmaceuticals", R),
        ("plastics_rubber", S),
        ("post_logistics", S),
        ("power_biomass", S),
        ("power_coal", S),
        ("power_gas", S),
        ("power_grid", S),
        ("power_hydro", S),
        ("power_nuclear", S),
        ("power_solar", S),
        ("power_storage", S),
        ("power_supply", S),
        ("power_wind", S),
        ("professional_services", S),
        ("public_administration", E),
        ("quarrying", S),
        ("rail_equipment", S),
        ("rail_transport", S),
        ("real_estate", E),
        ("recreation_culture", R2),
        ("refined_petroleum", S),
        ("restaurants", R2),
        ("retail_trade", S),
        ("road_freight", S),
        ("shipbuilding", S),
        ("social_care", E),
        ("steel", S),
        ("telecom_services", S),
        ("textiles_apparel", S),
        ("tobacco", S),
        ("trucks_buses", S),
        ("uranium_fuel", S),
        ("vehicle_trade_repair", S),
        ("water_transport", S),
        ("water_waste", R),
        ("wholesale_trade", S),
        ("wood_products", S),
    ]
};

impl Law {
    /// The law of 1 December 2021: VAT 19%, 9% and 5% with each good's category
    /// from the law in force; a minimum wage of 2,300 lei.
    #[must_use]
    pub fn placeholder_2021() -> Law {
        Law {
            vat: VatSchedule {
                standard: VatRate::percent(19),
                reduced: VatRate::percent(9),
                reduced_2: VatRate::percent(5),
            },
            vat_category: CATALOGUE_VAT.iter().map(|&(_, c)| c).collect(),
            minimum_wage: Bani::from_lei(2_300),
            income: None,
            transfers: None,
            excise_duty_ppm: CATALOGUE_VAT
                .iter()
                .map(|&(id, _)| {
                    EXCISE_2021
                        .iter()
                        .find(|&&(g, _)| g == id)
                        .map_or(0, |&(_, d)| d)
                })
                .collect(),
            property_tax_bp: None,
            wealth_tax_threshold: Bani::ZERO,
            wealth_tax_bp: 0,
        }
    }
}

/// The four mechanics' parameters and the seed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorldParams {
    /// Households.
    pub households: HouseholdParams,
    /// Labour market.
    pub labour: LabourParams,
    /// Production.
    pub production: ProductionParams,
    /// Prices.
    pub prices: PriceParams,
    /// Master seed of the keyed draws (the scenario's `rng_seed`).
    pub rng_seed: u64,
    /// The banks.
    #[cfg(feature = "banking")]
    pub banking: econ_mech_banking::BankParams,
    /// The policy rate at the start, millionths a year (1.75%; constant until
    /// the lever system, M4).
    #[cfg(feature = "banking")]
    pub policy_rate_ppm: i32,
    /// The central bank.
    #[cfg(feature = "monetary")]
    pub monetary: MonetaryParams,
    /// The government's debt.
    #[cfg(feature = "fiscal")]
    pub fiscal: FiscalParams,
    /// Trade and the exchange rate.
    #[cfg(feature = "trade")]
    pub trade: TradeParams,
    /// Approval (spec society/opinion-approval); `None`: approval is not
    /// computed (AC-APP-03). A test writer's addition (ADR-0014).
    #[cfg(feature = "approval")]
    pub approval: Option<crate::approval::ApprovalParams>,
    /// Poverty and inequality indicators (spec society/poverty-inequality);
    /// `None`: they are not computed (AC-POV-07). A test writer's addition
    /// (ADR-0014).
    #[cfg(feature = "poverty")]
    pub poverty: Option<crate::poverty::PovertyParams>,
}

impl WorldParams {
    /// Every parameter at its spec's default, with this seed.
    #[must_use]
    pub fn spec_defaults(rng_seed: u64) -> WorldParams {
        WorldParams {
            households: HouseholdParams {
                mpc: [1_000_000, 1_000_000, 1_000_000, 980_000, 950_000],
                mpc_wealth_ppm: 5_000,
                subsistence_basket: Bani::from_lei(700),
            },
            labour: LabourParams {
                matching_efficiency_ppm: 500_000,
                matching_elasticity_ppm: 500_000,
                max_layoff_share_ppm: 100_000,
                quit_rate_ppm: 10_000,
                wage_tightness_response_ppm: 100_000,
            },
            production: ProductionParams {
                theta_ppm: 300_000,
                phi_ppm: 250_000,
            },
            prices: PriceParams {
                indexation_ppm: 500_000,
                eta_utilisation_ppm: 200_000,
                eta_inventory_ppm: 100_000,
                normal_utilisation_ppm: 800_000,
                max_daily_move_ppm: 20_000,
            },
            rng_seed,
            #[cfg(feature = "banking")]
            banking: econ_mech_banking::BankParams::spec_defaults(),
            #[cfg(feature = "banking")]
            policy_rate_ppm: 17_500,
            #[cfg(feature = "monetary")]
            monetary: MonetaryParams::spec_defaults(),
            #[cfg(feature = "fiscal")]
            fiscal: FiscalParams::spec_defaults(),
            #[cfg(feature = "trade")]
            trade: TradeParams::spec_defaults(),
            #[cfg(feature = "approval")]
            approval: Some(crate::approval::ApprovalParams::spec_defaults()),
            #[cfg(feature = "poverty")]
            poverty: Some(crate::poverty::PovertyParams::spec_defaults()),
        }
    }
}

/// The phases of a day, in the order they run (the spec's table).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[allow(missing_docs)]
pub enum PhaseId {
    Commands,
    Plan,
    Staffing,
    Prices,
    OutputAndInputs,
    FixedDemand,
    Households,
    GoodsMarket,
    Quits,
    Pay,
    WageRound,
    MonthlyClose,
    Checks,
}

/// The phases of a day and their periods: the one list of the spec.
pub const PHASES: [(PhaseId, Period); 13] = [
    (PhaseId::Commands, Period::Daily),
    (PhaseId::Plan, Period::Staggered),
    (PhaseId::Staffing, Period::Staggered),
    (PhaseId::Prices, Period::Daily),
    (PhaseId::OutputAndInputs, Period::Daily),
    (PhaseId::FixedDemand, Period::Daily),
    (PhaseId::Households, Period::Staggered),
    (PhaseId::GoodsMarket, Period::Daily),
    (PhaseId::Quits, Period::MonthEnd),
    (PhaseId::Pay, Period::MonthEnd),
    (PhaseId::WageRound, Period::Yearly { month: 1, day: 1 }),
    (PhaseId::MonthlyClose, Period::MonthEnd),
    (PhaseId::Checks, Period::Daily),
];

/// What a day returns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DayReport {
    /// The day that ran.
    pub day: Day,
    /// Its date.
    pub date: Date,
    /// The phases that ran, in order.
    pub phases: Vec<PhaseId>,
    /// Every check of the checks phase held.
    pub checks_ok: bool,
    /// The checks that failed.
    pub failed: Vec<InvariantError>,
    /// The state hash after the day.
    pub state_hash: u64,
}

/// The statistics of a closed month. Money is national (weighted) bani.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonthStats {
    /// The month.
    pub month: Month,
    /// Consumer price index, millionths (1,000,000 at the start).
    pub cpi_ppm: u32,
    /// Twelve-month inflation (over the starting CPI, annualised, in the first
    /// year), millionths.
    pub inflation_ppm: i32,
    /// Expected inflation after the close, millionths a year.
    pub expected_inflation_ppm: i32,
    /// Employed persons, weighted.
    pub employed: u64,
    /// Unemployed persons, weighted.
    pub unemployed: u64,
    /// Wages paid.
    pub wage_bill: Bani,
    /// Household consumption, VAT included.
    pub consumption: Bani,
    /// GDP by production.
    pub gdp_production: Bani,
    /// GDP by expenditure.
    pub gdp_expenditure: Bani,
    /// GDP by income.
    pub gdp_income: Bani,
    /// Shortages of the month by catalogue industry, real units.
    pub shortages: Vec<i64>,
    /// Households in hardship, weighted.
    pub hardship_households: u64,
    /// Gross wages firm units owed and could not pay this month
    /// (`wages_unpaid`; zero in a normal run).
    pub wages_unpaid: Bani,
    /// Taxes firm units owed and could not pay this month (`taxes_unpaid`;
    /// zero in a normal run).
    pub taxes_unpaid: Bani,
}

/// Why a world was not built or a day was refused. Nothing changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorldError {
    /// The tables disagree with the opening ledger (I-4 at day 0).
    TablesDisagreeWithLedger {
        /// Which balance.
        what: &'static str,
    },
    /// Commands are not accepted before the lever registry (M4).
    CommandsNotYet,
    /// The tables do not fit together.
    Shape(&'static str),
}

/// The households table, aligned with the scenario's.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HouseholdColumns {
    /// Real households represented.
    pub hh_weight: Vec<u32>,
    /// Currency.
    pub hh_cash: Vec<Bani>,
    /// Bank deposits.
    pub hh_deposits: Vec<Bani>,
    /// Money received since the last decision day.
    pub hh_income_received: Vec<Bani>,
    /// The day of the month of the household's decision, 1 to 28.
    pub hh_stagger_day: Vec<u8>,
    /// The household could not pay for the subsistence basket at its last decision.
    pub hh_hardship: Vec<bool>,
    /// Income fifth by last month's income per equivalised adult (0 = poorest).
    pub hh_fifth: Vec<u8>,
    /// The day of its last decision; `None` before the first.
    pub last_decision: Vec<Option<Day>>,
    /// Everything received since day 0 (AC-HH-03).
    pub income_to_date: Vec<Bani>,
    /// Everything spent since day 0, VAT included (AC-HH-03).
    pub spending_to_date: Vec<Bani>,
}

/// The persons table, aligned with the scenario's.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PersonColumns {
    /// Index into the households table.
    pub household_id: Vec<u32>,
    /// Highest completed education.
    pub edu_level: Vec<EduLevel>,
    /// Firm unit, or `econ_popgen::NO_EMPLOYER`.
    pub employer: Vec<u16>,
    /// ISCO-08 major group, or `econ_popgen::NOT_EMPLOYED`.
    pub occupation: Vec<u8>,
    /// Gross wage a month; 0 for a person without a job.
    pub wage: Vec<Bani>,
    /// Its declared part.
    pub wage_declared: Vec<Bani>,
    /// Months since the person lost a job or started looking.
    pub months_unemployed: Vec<u16>,
    /// The day the person's current job started; `None` for a job held since
    /// day 0 or no job.
    pub hired_on: Vec<Option<Day>>,
    /// The wage paid to the person at the latest pay day.
    pub last_pay: Vec<Bani>,
}

/// The firm units, aligned with the scenario's firm-unit table.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FirmUnitColumns {
    /// Catalogue industry.
    pub industry: Vec<u8>,
    /// The day of the month of the unit's plan and staffing, 1 to 28.
    pub unit_stagger_day: Vec<u8>,
    /// Weighted persons whose `employer` is the unit.
    pub staff: Vec<u64>,
    /// Persons wanted by the last plan.
    pub desired_employment: Vec<u64>,
    /// Expected sales a month.
    pub expected_demand: Vec<i64>,
    /// Maximum output a month.
    pub capacity: Vec<i64>,
    /// Finished-goods inventory.
    pub inventory: Vec<i64>,
    /// Months of expected demand the unit aims to hold, millionths.
    pub inventory_target_ppm: Vec<u32>,
    /// Sold since the last plan.
    pub sales_since_plan: Vec<i64>,
    /// Produced on the latest day.
    pub output_today: Vec<i64>,
    /// Sold on the latest day.
    pub sales_today: Vec<i64>,
    /// Received out of its industry's sales clearing account on the latest day.
    pub revenue_today: Vec<Bani>,
    /// Currency.
    pub currency: Vec<Bani>,
    /// Bank deposits.
    pub deposits: Vec<Bani>,
    /// The day of its last plan; `None` before the first.
    pub last_plan: Vec<Option<Day>>,
    /// The day of its last staffing; `None` before the first.
    pub last_staffing: Vec<Option<Day>>,
}

/// The catalogue industries.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IndustryColumns {
    /// Catalogue id (`agri_cereals`, `cars`, …).
    pub id: Vec<String>,
    /// Industry group (the ten NACE groups of the population's jobs).
    pub group: Vec<u8>,
    /// Household basket share, millionths; also the CPI weight.
    pub basket_share_ppm: Vec<u32>,
    /// Price, millionths of the starting price.
    pub price_ppm: Vec<u32>,
    /// Consumer price, VAT included, millionths of the starting consumer price.
    pub consumer_price_ppm: Vec<u32>,
    /// Demand on the latest day, real units.
    pub demand_today: Vec<i64>,
    /// Demand not served on the latest day, real units.
    pub shortage_today: Vec<i64>,
    /// What the latest day's buyers paid for the industry's goods, VAT
    /// excluded, into its sales clearing account.
    pub sales_value_today: Vec<Bani>,
}

/// Per person, what the tables do not show.
#[derive(Debug, Clone, Default)]
struct PersonState {
    /// Status in employment (0 none, 1 employee, 2 to 4 the self-employed).
    status: Vec<u8>,
    /// Looking for work.
    seeking: Vec<bool>,
    /// Region of the person's household.
    region: Vec<u8>,
    pension: Vec<Bani>,
    /// Age in months.
    age: Vec<u16>,
    /// Self-employment and informal income at the start, a month.
    other_income: Vec<Bani>,
}

/// Per household, what the tables do not show.
#[derive(Debug, Clone, Default)]
struct HouseholdState {
    /// Equivalised adults, thousandths.
    eq_adults_milli: Vec<u32>,
    subsistence: Vec<Bani>,
    /// Money received in the current and in the previous calendar month.
    income_this_month: Vec<Bani>,
    income_last_month: Vec<Bani>,
}

/// Per firm unit, what the tables do not show.
#[derive(Debug, Clone, Default)]
struct UnitState {
    /// Staff and a month's output at the start: the labour requirement.
    staff0: Vec<u64>,
    output0: Vec<i64>,
    /// `ScaleLabourRequirement`, millionths.
    labour_factor_ppm: u32,
    /// Weight for paying investment goods (cohort units' starting surplus).
    invest_weight: Vec<u64>,
    /// A public producer, paid at cost by the government.
    public: Vec<bool>,
    /// A month of wages at the start of the day.
    wage_reserve: Vec<i128>,
    /// Credit inside the firm sector: what a unit owes the others, and what
    /// it has lent them (the two columns sum to the same).
    owes: Vec<Bani>,
    lent: Vec<Bani>,
    /// The current plan: a month's output, spread over `plan_days` days.
    plan_output: Vec<i64>,
    plan_days: Vec<u8>,
    /// Days of the current plan already produced.
    plan_done: Vec<u8>,
    /// Worst share of yesterday's input orders served, millionths.
    inputs_served_ppm: Vec<u32>,
    /// Stocks of its essential inputs, in the order of its industry's
    /// essential inputs (real units).
    input_stock: Vec<Vec<i64>>,
    /// Its employees, ascending.
    members: Vec<Vec<u32>>,
    /// Employees by occupation at the start (a fallback mix for vacancies).
    start_mix: Vec<[u64; 10]>,
    /// The fraction of a hire carried to next month, by tier.
    hire_carry: Vec<[u32; 3]>,
    /// Vacancies opened this month, by tier.
    vacancies_month: Vec<[u64; 3]>,
    /// Weight for paying self-employment and informal income.
    mixed_weight: Vec<u64>,
    /// The firm sector's deposits still free today for payments out of the
    /// sector (imports, and the product taxes and duties on purchases, which
    /// are posted at the end of phase 8): set at phase 5, spent by phases 5,
    /// 6 and 8. Purchases stop where it runs out (no lever may overdraw the
    /// sector); in a normal run it never binds.
    firm_free: i128,
    /// The share of its wage bill each unit paid on the last pay day,
    /// millionths; empty while every unit paid in full (the firm-payment
    /// backstop).
    pay_share_ppm: Vec<u32>,
    /// What firm units could not pay, by kind (the backstop's report).
    unpaid: insolvency::UnpaidBook,
}

/// Per industry, what the tables do not show.
#[derive(Debug, Clone, Default)]
struct IndustryState {
    /// Home inputs of the industry's output: (supplier, coefficient ppm).
    inputs: Vec<Vec<(usize, u32)>>,
    /// Its essential inputs (suppliers), ascending.
    essential: Vec<Vec<usize>>,
    /// Their coefficients, ppm, aligned with `essential`.
    essential_ppm: Vec<Vec<u32>>,
    /// For each of `inputs`, its place in `essential`, if it is one.
    essential_slot: Vec<Vec<Option<usize>>>,
    import_ppm: Vec<u32>,
    other_ppm: Vec<u32>,
    markup0_ppm: Vec<i32>,
    flex_daily_ppm: Vec<u32>,
    /// Wage cost per unit of output, of the last month.
    wage_cost_ppm: Vec<u32>,
    basket_import_ppm: Vec<u32>,
    /// Fixed final demand a day: quantities and the imported parts in money.
    gov_q: Vec<i64>,
    inv_q: Vec<i64>,
    exp_q: Vec<i64>,
    gov_imp: Vec<Bani>,
    inv_imp: Vec<Bani>,
    /// The month so far.
    output_month: Vec<i64>,
    wages_month: Vec<Bani>,
    shortage_month: Vec<i64>,
}

/// The month so far, for its statistics.
#[derive(Debug, Clone, Default)]
struct MonthAcc {
    output_value: i128,
    inputs_home: i128,
    inputs_imported: i128,
    vat: i128,
    consumption: i128,
    government: i128,
    investment: i128,
    exports: i128,
    imports: i128,
    sales_value: i128,
    wages: i128,
    employer_contributions: i128,
}

/// Economy-wide state.
#[derive(Debug, Clone, Default)]
struct MacroState {
    expected_inflation_ppm: i32,
    target_ppm: i32,
    /// CPI at each close, millionths.
    cpi: Vec<u32>,
    /// Bounds of the income fifths (income per equivalised adult, bani ×
    /// 1,000), from the latest close.
    fifth_bounds: [i128; 4],
    /// Qualified seekers and vacancies by tier at the latest close.
    tier_seekers: [u64; 3],
    tier_vacancies: [u64; 3],
    /// Tightness by industry group at each close, millionths (latest last).
    tightness: Vec<Vec<u32>>,
    normal_tightness: Vec<u32>,
    /// Reference wage `[group][occupation]`.
    reference_wage: Vec<[Bani; 10]>,
    /// Consumption of the first and of the latest closed month.
    first_consumption: Bani,
    last_consumption: Bani,
    /// Separations of the month still to be paid: (person, unit, pay, its
    /// declared part).
    owed: Vec<(u32, u16, Bani, Bani)>,
    /// The one-off transfer of a [`StartEdit::Transfer`], paid on day 0.
    transfer: Vec<Bani>,
    /// The starting month's income is still to be paid (day 0).
    start_pay: bool,
    /// Clearing accounts left non-zero today (I-8): flow, industry, amount.
    clearing: Vec<(econ_ledger::FlowCode, usize, Bani)>,
    month: MonthAcc,
}

/// The day's demand on the industries, before the goods market.
#[derive(Debug, Clone, Default)]
struct DayDemand {
    /// `[industry]`: per buying unit (unit, quantity).
    units: Vec<Vec<(usize, i64)>>,
    /// `[industry]`: per public unit buying investment goods for the
    /// government (unit, quantity; ministry budgets, feature `levers`).
    agents: Vec<Vec<(usize, i64)>>,
    /// Per household buying today: its month's spending, VAT included.
    households: Vec<(usize, Bani)>,
}

/// The simulated economy.
#[derive(Debug, Clone)]
pub struct World {
    sample_scale: u32,
    law: Law,
    params: WorldParams,
    seed: KeyedRng,
    calendar: Calendar,
    today: Day,
    ledger: Ledger,
    households: HouseholdColumns,
    persons: PersonColumns,
    units: FirmUnitColumns,
    industries: IndustryColumns,
    hh: HouseholdState,
    pp: PersonState,
    uu: UnitState,
    ii: IndustryState,
    mac: MacroState,
    fs: fiscal::FiscalState,
    #[cfg(feature = "fiscal")]
    tr: treasury::TreasuryState,
    #[cfg(feature = "banking")]
    bk: banking::Banks,
    #[cfg(feature = "trade")]
    fx: foreign::ForeignState,
    #[cfg(feature = "monetary")]
    cb: central_bank::CentralBank,
    #[cfg(feature = "investment")]
    inv: investment::Investment,
    #[cfg(feature = "revenue")]
    rev: revenue::Revenue,
    slips: PaySlips,
    demand: DayDemand,
    months: Vec<MonthStats>,
    /// The lever system's state: the registry, the law in force, pending
    /// commands, the log and the events (`None` for a world built without
    /// `law.json`).
    #[cfg(feature = "levers")]
    lev: Option<Box<crate::levers::LeverState>>,
    /// The ministry budgets.
    #[cfg(feature = "levers")]
    bud: budget::BudgetState,
}

#[cfg(feature = "levers")]
pub(crate) use budget::BudgetStart;

/// What the lever system reaches inside the `World` (spec game/lever-system).
#[cfg(feature = "levers")]
impl World {
    /// The lever system's state, if the world was built under `law.json`.
    pub(crate) fn lever_state(&self) -> Option<&crate::levers::LeverState> {
        self.lev.as_deref()
    }

    /// The lever system's state, mutable.
    pub(crate) fn lever_state_mut(&mut self) -> Option<&mut crate::levers::LeverState> {
        self.lev.as_deref_mut()
    }

    /// Install the lever system's state (at load).
    pub(crate) fn set_lever_state(&mut self, state: crate::levers::LeverState) {
        self.lev = Some(Box::new(state));
    }

    /// Replace the law the mechanics read (a command took effect).
    pub(crate) fn set_law(&mut self, law: Law) {
        let old = std::mem::replace(&mut self.law, law);
        if self.law.minimum_wage > old.minimum_wage {
            self.lift_to_minimum();
        }
        #[cfg(feature = "revenue")]
        self.rev_law_changed(&old);
        #[cfg(not(feature = "revenue"))]
        let _ = old;
    }
}

impl World {
    /// Load a starting state under a law (AC-WLD-08: refuses tables that
    /// disagree with the opening ledger, building nothing).
    ///
    /// # Errors
    /// [`WorldError::TablesDisagreeWithLedger`], [`WorldError::Shape`].
    pub fn new(start: StartState, law: Law, params: WorldParams) -> Result<World, WorldError> {
        load::new(start, law, params)
    }

    /// Run today's phases and move to the next day. `commands` must be empty
    /// in M2 (AC-WLD-09).
    ///
    /// # Errors
    /// [`WorldError::CommandsNotYet`]; nothing changes.
    pub fn step(&mut self, commands: &[Command]) -> Result<DayReport, WorldError> {
        if !commands.is_empty() {
            return Err(WorldError::CommandsNotYet);
        }
        let day = self.today;
        let date = self.calendar.date(day);
        self.ledger.begin_tick(day.0);
        self.begin_day();
        #[cfg(feature = "banking")]
        self.bank_begin_day();
        #[cfg(feature = "monetary")]
        self.cb_begin_day();
        #[cfg(feature = "investment")]
        self.invest_begin_day();
        #[cfg(feature = "revenue")]
        self.rev_begin_day();
        #[cfg(feature = "trade")]
        self.trade_begin_day();
        let mut phases = Vec::with_capacity(PHASES.len());
        let mut failed = Vec::new();
        for (id, period) in PHASES {
            let due = period.due(date);
            if due == Due::No {
                continue;
            }
            phases.push(id);
            let cohort = match due {
                Due::Cohort(d) => d,
                _ => 0,
            };
            match id {
                PhaseId::Commands => {
                    #[cfg(feature = "levers")]
                    crate::levers::apply_due(self, date);
                    self.pay_transfer();
                }
                PhaseId::Plan => self.plan(cohort),
                PhaseId::Staffing => self.staffing(cohort),
                PhaseId::Prices => self.prices(),
                PhaseId::OutputAndInputs => self.output_and_inputs(),
                PhaseId::FixedDemand => self.fixed_demand(),
                PhaseId::Households => self.household_decisions(cohort),
                PhaseId::GoodsMarket => self.goods_market(),
                PhaseId::Quits => self.quits(),
                PhaseId::Pay => self.pay(),
                PhaseId::WageRound => self.wage_round(),
                PhaseId::MonthlyClose => {
                    failed.extend(self.monthly_close());
                    #[cfg(feature = "levers")]
                    crate::levers::mark_prices(self);
                }
                PhaseId::Checks => failed.extend(self.checks()),
            }
        }
        #[cfg(feature = "fiscal")]
        self.treasury_day_end();
        #[cfg(feature = "trade")]
        self.trade_day_end();
        #[cfg(feature = "investment")]
        self.invest_day_end();
        self.today = day.next();
        let state_hash = self.state_hash();
        Ok(DayReport {
            day,
            date,
            phases,
            checks_ok: failed.is_empty(),
            failed,
            state_hash,
        })
    }

    /// The date of [`World::day`].
    #[must_use]
    pub fn date(&self) -> Date {
        self.calendar.date(self.today)
    }

    /// The parameters the world runs with.
    #[must_use]
    pub fn params(&self) -> &WorldParams {
        &self.params
    }

    /// The day the next [`World::step`] runs (day 0 after [`World::new`]).
    #[must_use]
    pub fn day(&self) -> Day {
        self.today
    }

    /// The ledger.
    #[must_use]
    pub fn ledger(&self) -> &Ledger {
        &self.ledger
    }

    /// The statistics of every closed month, in order.
    #[must_use]
    pub fn months(&self) -> &[MonthStats] {
        &self.months
    }

    /// The households table.
    #[must_use]
    pub fn households(&self) -> &HouseholdColumns {
        &self.households
    }

    /// The persons table.
    #[must_use]
    pub fn persons(&self) -> &PersonColumns {
        &self.persons
    }

    /// The firm units.
    #[must_use]
    pub fn firm_units(&self) -> &FirmUnitColumns {
        &self.units
    }

    /// The industries.
    #[must_use]
    pub fn industries(&self) -> &IndustryColumns {
        &self.industries
    }

    /// The inflation target, millionths a year (the starting expected
    /// inflation).
    #[must_use]
    pub fn inflation_target_ppm(&self) -> i32 {
        self.mac.target_ppm
    }

    /// Clear what "today" columns hold from the day before.
    fn begin_day(&mut self) {
        let u = &mut self.units;
        u.output_today.iter_mut().for_each(|v| *v = 0);
        u.sales_today.iter_mut().for_each(|v| *v = 0);
        u.revenue_today.iter_mut().for_each(|v| *v = Bani::ZERO);
        let i = &mut self.industries;
        i.demand_today.iter_mut().for_each(|v| *v = 0);
        i.shortage_today.iter_mut().for_each(|v| *v = 0);
        i.sales_value_today.iter_mut().for_each(|v| *v = Bani::ZERO);
        self.demand.households.clear();
        self.demand.units.iter_mut().for_each(Vec::clear);
        self.uu.wage_reserve = self.wage_reserves();
    }
}
