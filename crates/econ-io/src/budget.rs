//! The starting budget (spec `economy/budget-2021`): the budget in force on the
//! start date as scenario data. What the state spends on each of thirteen
//! ministry lines in each of twelve economic types, the social benefits by
//! programme, the occupied public posts by sector and the five public firm
//! units that employ them, and the published national totals the table must
//! add up to.
//!
//! Amounts are whole lei a year in the file and bani in code (ADR-0007). The
//! loader adds no behaviour: it parses, checks (AC-BUD-01 to 04, 13), splits
//! annual amounts into months (AC-BUD-10) and hashes (AC-BUD-11).
//!
//! API skeleton from the spec's sketch (ADR-0014): the acceptance tests in
//! `tests/acceptance/budget.rs` were written against it before the code, and
//! every body is `unimplemented!()` until then. Compiled only with the crate
//! feature `budget`.

use econ_types::{Bani, Date};

/// The twelve economic types, in file order: the order of every amount vector.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EconType {
    /// `wages`: compensation of employees, wages and salaries (ESA D11).
    Wages,
    /// `employer_contributions`: employers' social contributions (D12).
    EmployerContributions,
    /// `goods_services`: intermediate consumption (P2).
    GoodsServices,
    /// `social_benefits_cash`: social benefits other than in kind (D62).
    SocialBenefitsCash,
    /// `social_benefits_in_kind`: social transfers in kind bought from market
    /// producers (D632).
    SocialBenefitsInKind,
    /// `subsidies`: subsidies (D3).
    Subsidies,
    /// `interest`: property income paid, interest (D4).
    Interest,
    /// `investment`: gross capital formation (P5).
    Investment,
    /// `non_produced_assets`: acquisitions less disposals of non-produced
    /// assets (NP); the only type that may be negative.
    NonProducedAssets,
    /// `current_transfers`: other current transfers (D7).
    CurrentTransfers,
    /// `capital_transfers`: capital transfers (D9).
    CapitalTransfers,
    /// `other_taxes`: other taxes on production, current taxes on income and
    /// wealth, capital taxes paid (D29 + D5 + D8).
    OtherTaxes,
}

/// Number of economic types.
pub const N_TYPES: usize = 12;

/// One ministry line of the game: a set of COFOG subgroups and its spending.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BudgetLine {
    /// Line id, for example `"education"`.
    pub id: String,
    /// Its COFOG subgroups (`"GF0901"`, ...); the lines are a partition of the
    /// subgroups.
    pub cofog: Vec<String>,
    /// Spending a year on each economic type, in [`EconType`] order: whole lei
    /// of the file, as bani.
    pub per_year: [Bani; N_TYPES],
}

/// Social benefits of one programme (cash, D62, and in kind, D632), a year.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Programme {
    /// Programme id, for example `"old_age"`.
    pub id: String,
    /// Its COFOG subgroups or divisions.
    pub cofog: Vec<String>,
    /// Cash benefits a year.
    pub cash: Bani,
    /// Benefits in kind a year.
    pub in_kind: Bani,
}

/// Occupied public posts of one sector and the public firm unit they sit in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicSector {
    /// Sector id, for example `"education"`.
    pub id: String,
    /// Occupied posts in December 2021.
    pub posts: u32,
    /// The public firm unit whose employees they are, for example
    /// `"education_services.public"`.
    pub unit: String,
}

/// Posts, wages and employers' contributions of one public firm unit, a year.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicUnitBudget {
    /// The public firm unit's id.
    pub unit: String,
    /// Occupied posts: `public_posts[f]`, its headcount target at the start.
    pub posts: u32,
    /// Wages (D11) a year.
    pub wages: Bani,
    /// Employers' contributions (D12) a year.
    pub employer_contributions: Bani,
}

/// Published national totals by ESA code, as bani (`TE`, `D1`, `D11`, `D12`,
/// `P2`, `D62`, `D632`, `D3`, `D4`, `P5`, `NP`, `D7`, `D9`, `D29_D5_D8`, ...),
/// sorted by code. A vector, not a map (ADR-0006).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishedTotals(pub Vec<(String, Bani)>);

/// The starting budget, checked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Budget {
    /// The budget year, 2021.
    pub year: i32,
    /// The scenario's start date, 1 December 2021.
    pub start_date: Date,
    /// Nominal GDP of the year.
    pub gdp: Bani,
    /// The thirteen ministry lines, in file order.
    pub lines: Vec<BudgetLine>,
    /// Social benefits by programme, in file order.
    pub programmes: Vec<Programme>,
    /// Occupied public posts by sector, in file order.
    pub sectors: Vec<PublicSector>,
    /// The five public firm units, in file order.
    pub public_units: Vec<PublicUnitBudget>,
    /// The published national totals.
    pub published: PublishedTotals,
}

/// Why a budget file was rejected. Nothing is loaded in that case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BudgetError {
    /// A line with the wrong number of amounts.
    Shape {
        /// The line's id.
        line: String,
    },
    /// A type that is not one of the twelve economic types.
    UnknownType(String),
    /// A line id given twice.
    DuplicateLine(String),
    /// A type (or total expenditure) whose lines do not add up to its
    /// published total.
    TotalMismatch {
        /// The published total's ESA code.
        code: String,
        /// What the lines add up to.
        lines: Bani,
        /// The published total.
        published: Bani,
    },
    /// A COFOG division whose amounts do not add up to its published total.
    DivisionMismatch {
        /// The division, for example `"GF07"`.
        cofog: String,
    },
    /// A sector or public unit whose unit is not one of the five public firm
    /// units.
    UnknownUnit(String),
    /// Posts that do not add up (sectors, public units, the total).
    PostsMismatch,
    /// A public unit with wages and no posts.
    WagesWithoutPosts(String),
}

impl Budget {
    /// Parse a budget file (`budget2021_ro.json`'s shape) and run
    /// [`Budget::check`].
    ///
    /// # Errors
    /// The first [`BudgetError`] found; nothing is loaded.
    pub fn from_json(_bytes: &[u8]) -> Result<Budget, BudgetError> {
        unimplemented!("budget loader (spec economy/budget-2021)")
    }

    /// The table adds up, exactly, to the published totals it carries
    /// (AC-BUD-01 to 04, 13).
    ///
    /// # Errors
    /// The first inconsistency found.
    pub fn check(&self) -> Result<(), BudgetError> {
        unimplemented!("budget check (spec economy/budget-2021)")
    }

    /// The amount of month `month` (0 to 11, counted from the start month) of
    /// an amount a year: whole bani, the remainder to the first months, the
    /// sign of a negative amount kept (spec, "Monthly amounts"; AC-BUD-10).
    pub fn monthly(_per_year: Bani, _month: u8) -> Bani {
        unimplemented!("monthly amounts (spec economy/budget-2021)")
    }

    /// Spending a year of type `t` over all lines.
    pub fn total(&self, _t: EconType) -> Bani {
        unimplemented!("budget total (spec economy/budget-2021)")
    }

    /// The budget hash (FNV-1a 64; AC-BUD-11).
    #[must_use]
    pub fn hash(&self) -> u64 {
        unimplemented!("budget hash (spec economy/budget-2021)")
    }
}
