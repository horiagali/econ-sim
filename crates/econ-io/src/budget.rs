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
//! API from the spec's sketch (ADR-0014): the acceptance tests in
//! `tests/acceptance/budget.rs` were written against it before the code.
//! Compiled only with the crate feature `budget` (on by default).

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
    /// The COFOG subgroups whose compensation goes to it.
    pub cofog: Vec<String>,
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

// ---------------------------------------------------------------------------
// The spec's fixed vocabulary.
// ---------------------------------------------------------------------------

/// The twelve economic types in file order, with their id in the file and the
/// published national total (ESA code) the lines must add up to.
const TYPES: [(EconType, &str, &str); N_TYPES] = [
    (EconType::Wages, "wages", "D11"),
    (
        EconType::EmployerContributions,
        "employer_contributions",
        "D12",
    ),
    (EconType::GoodsServices, "goods_services", "P2"),
    (EconType::SocialBenefitsCash, "social_benefits_cash", "D62"),
    (
        EconType::SocialBenefitsInKind,
        "social_benefits_in_kind",
        "D632",
    ),
    (EconType::Subsidies, "subsidies", "D3"),
    (EconType::Interest, "interest", "D4"),
    (EconType::Investment, "investment", "P5"),
    (EconType::NonProducedAssets, "non_produced_assets", "NP"),
    (EconType::CurrentTransfers, "current_transfers", "D7"),
    (EconType::CapitalTransfers, "capital_transfers", "D9"),
    (EconType::OtherTaxes, "other_taxes", "D29_D5_D8"),
];

/// The thirteen ministry lines of the spec ("Budget lines"). The file's order
/// is kept (the hash reads it); this list only says which ids exist.
const LINE_IDS: [&str; 13] = [
    "administration",
    "public_debt",
    "defence",
    "police_justice",
    "economic_affairs",
    "transport",
    "environment",
    "housing_community",
    "health",
    "culture",
    "education",
    "social_protection",
    "research",
];

/// The five public firm units (spec, "Public employment").
const PUBLIC_UNITS: [&str; 5] = [
    "defence_services.public",
    "education_services.public",
    "health_services.public",
    "public_administration.public",
    "social_care.public",
];

/// The ten COFOG divisions, in the order of the file's `by_function`.
const DIVISIONS: [&str; 10] = [
    "GF01", "GF02", "GF03", "GF04", "GF05", "GF06", "GF07", "GF08", "GF09", "GF10",
];

/// Published total expenditure, the ESA code all lines add up to.
const TE: &str = "TE";

/// The name [`BudgetError::DivisionMismatch`] carries when the ten divisions
/// together, type by type, do not add up to the lines (no single division can
/// be blamed for it).
const ALL_DIVISIONS: &str = "GF01-GF10";

// ---------------------------------------------------------------------------
// The file as it is written (whole lei). Fields the loader does not need
// (`esa`, `categories`, `totals`, `cash_execution`, `provenance`, names and
// confidences) are derived or descriptive and are ignored: the ESA code of each
// type is fixed by the spec (`TYPES`).
// ---------------------------------------------------------------------------

#[derive(serde::Deserialize)]
struct RawBudget {
    year: i32,
    start_date: String,
    gdp: i64,
    types: Vec<String>,
    lines: Vec<RawLine>,
    by_function: Vec<RawFunction>,
    programmes: Vec<RawProgramme>,
    employment: RawEmployment,
    public_units: Vec<RawUnit>,
    /// A map in the file; a `BTreeMap` here, so it comes out sorted by code.
    published: std::collections::BTreeMap<String, i64>,
}

#[derive(serde::Deserialize)]
struct RawLine {
    id: String,
    cofog: Vec<String>,
    spending: Vec<i64>,
    total: i64,
}

#[derive(serde::Deserialize)]
struct RawFunction {
    cofog: String,
    spending: Vec<i64>,
    total: i64,
    published_total: i64,
}

#[derive(serde::Deserialize)]
struct RawProgramme {
    id: String,
    cofog: Vec<String>,
    cash: i64,
    in_kind: i64,
    total: i64,
}

#[derive(serde::Deserialize)]
struct RawEmployment {
    posts: u32,
    central: u32,
    local: u32,
    sectors: Vec<RawSector>,
}

#[derive(serde::Deserialize)]
struct RawSector {
    id: String,
    posts: u32,
    unit: String,
}

#[derive(serde::Deserialize)]
struct RawUnit {
    unit: String,
    posts: u32,
    wages: i64,
    employer_contributions: i64,
    #[serde(default)]
    cofog: Vec<String>,
}

// ---------------------------------------------------------------------------
// Helpers.
// ---------------------------------------------------------------------------

/// A malformed record. The sketch has no parse variant, so everything that is
/// wrong with the file's shape (not JSON, a missing field, a fraction where
/// whole lei are due, an amount too large for bani, an id longer than 255
/// bytes, a record whose parts do not add up to its own `total`) is a
/// [`BudgetError::Shape`] naming the record (a line id, or a field).
fn shape(what: impl Into<String>) -> BudgetError {
    BudgetError::Shape { line: what.into() }
}

/// Whole lei of the file as bani, checked.
fn bani(lei: i64, what: &str) -> Result<Bani, BudgetError> {
    lei.checked_mul(100).map(Bani).ok_or_else(|| shape(what))
}

/// A sum in `i128` (which cannot overflow for any count of `i64` a file holds)
/// back to bani; saturates instead of wrapping in the impossible case.
fn saturate(sum: i128) -> Bani {
    Bani(i64::try_from(sum).unwrap_or(if sum < 0 { i64::MIN } else { i64::MAX }))
}

/// An id that the hash can carry (one length byte).
fn short_id(s: &str) -> Result<(), BudgetError> {
    if s.len() > usize::from(u8::MAX) {
        Err(shape(s))
    } else {
        Ok(())
    }
}

/// `YYYY-MM-DD` as a [`Date`].
fn parse_date(s: &str) -> Option<Date> {
    let mut parts = s.split('-');
    let (y, m, d) = (parts.next()?, parts.next()?, parts.next()?);
    if parts.next().is_some() || y.len() != 4 || m.len() != 2 || d.len() != 2 {
        return None;
    }
    Date::new(y.parse().ok()?, m.parse().ok()?, d.parse().ok()?)
}

/// Feed an id: one length byte, then its UTF-8 bytes. Ids are at most 255
/// bytes when loaded; a longer one (only possible in a hand-built `Budget`)
/// has its length byte saturated at 255.
fn write_id(h: &mut econ_core::StateHasher, s: &str) {
    h.write(&[u8::try_from(s.len()).unwrap_or(u8::MAX)]);
    h.write(s.as_bytes());
}

impl Budget {
    /// Parse a budget file (`budget2021_ro.json`'s shape) and run
    /// [`Budget::check`].
    ///
    /// Checks, in this order (the first failure is returned, nothing is
    /// loaded):
    /// 1. the file parses ([`BudgetError::Shape`] naming `file` otherwise);
    /// 2. `types` is the twelve type ids in the spec's order: an id that is
    ///    not a type, or a type out of place (repeated or reordered; the
    ///    sketch has no variant of its own for those), is
    ///    [`BudgetError::UnknownType`]; a list of the wrong length is
    ///    [`BudgetError::Shape`] naming `types`;
    /// 3. each line has twelve whole-lei amounts that add up to its `total`,
    ///    and each programme's cash and in kind add up to its `total`
    ///    ([`BudgetError::Shape`] naming the line or programme);
    /// 4. [`Budget::check`] on the loaded table;
    /// 5. the sectors' posts add up to `employment.posts`, and `central` plus
    ///    `local` is that total too ([`BudgetError::PostsMismatch`]);
    /// 6. `by_function` is the ten divisions in order, each with twelve
    ///    amounts adding up to its total, which equals Eurostat's published
    ///    total; then, type by type, the divisions add up to the lines
    ///    ([`BudgetError::DivisionMismatch`]; for the last check, which no
    ///    single division fails, the name is `"GF01-GF10"`).
    ///
    /// The division table and the total posts are checked here, not in
    /// [`Budget::check`], because [`Budget`] does not carry them.
    ///
    /// # Errors
    /// The first [`BudgetError`] found; nothing is loaded.
    pub fn from_json(bytes: &[u8]) -> Result<Budget, BudgetError> {
        let raw: RawBudget =
            serde_json::from_slice(bytes).map_err(|e| shape(format!("file: {e}")))?;

        // 2. The economic types.
        for (k, t) in raw.types.iter().enumerate() {
            let known = TYPES.iter().any(|&(_, name, _)| name == t);
            let in_place = TYPES.get(k).is_some_and(|&(_, name, _)| name == t);
            if !known || !in_place {
                return Err(BudgetError::UnknownType(t.clone()));
            }
        }
        if raw.types.len() != N_TYPES {
            return Err(shape("types"));
        }

        // 3. The lines and programmes, each consistent with its own total.
        let mut lines = Vec::with_capacity(raw.lines.len());
        for l in raw.lines {
            short_id(&l.id)?;
            if l.spending.len() != N_TYPES {
                return Err(shape(l.id));
            }
            let sum: i128 = l.spending.iter().map(|&a| i128::from(a)).sum();
            if sum != i128::from(l.total) {
                return Err(shape(l.id));
            }
            let mut per_year = [Bani(0); N_TYPES];
            for (slot, &lei) in per_year.iter_mut().zip(&l.spending) {
                *slot = bani(lei, &l.id)?;
            }
            lines.push(BudgetLine {
                id: l.id,
                cofog: l.cofog,
                per_year,
            });
        }
        let mut programmes = Vec::with_capacity(raw.programmes.len());
        for p in raw.programmes {
            if i128::from(p.cash) + i128::from(p.in_kind) != i128::from(p.total) {
                return Err(shape(p.id));
            }
            programmes.push(Programme {
                cash: bani(p.cash, &p.id)?,
                in_kind: bani(p.in_kind, &p.id)?,
                id: p.id,
                cofog: p.cofog,
            });
        }
        let mut sectors = Vec::with_capacity(raw.employment.sectors.len());
        for s in &raw.employment.sectors {
            short_id(&s.id)?;
            sectors.push(PublicSector {
                id: s.id.clone(),
                posts: s.posts,
                unit: s.unit.clone(),
            });
        }
        let mut public_units = Vec::with_capacity(raw.public_units.len());
        for u in raw.public_units {
            short_id(&u.unit)?;
            public_units.push(PublicUnitBudget {
                wages: bani(u.wages, &u.unit)?,
                employer_contributions: bani(u.employer_contributions, &u.unit)?,
                unit: u.unit,
                posts: u.posts,
                cofog: u.cofog,
            });
        }
        let mut published = Vec::with_capacity(raw.published.len());
        for (code, lei) in raw.published {
            let b = bani(lei, &code)?;
            published.push((code, b));
        }
        let budget = Budget {
            year: raw.year,
            start_date: parse_date(&raw.start_date).ok_or_else(|| shape("start_date"))?,
            gdp: bani(raw.gdp, "gdp")?,
            lines,
            programmes,
            sectors,
            public_units,
            published: PublishedTotals(published),
        };

        // 4. The table against its published totals.
        budget.check()?;

        // 5. The total posts.
        let emp = &raw.employment;
        let sector_posts: u64 = emp.sectors.iter().map(|s| u64::from(s.posts)).sum();
        if sector_posts != u64::from(emp.posts)
            || u64::from(emp.central) + u64::from(emp.local) != u64::from(emp.posts)
        {
            return Err(BudgetError::PostsMismatch);
        }

        // 6. The COFOG divisions.
        check_divisions(&raw.by_function, &budget)?;
        Ok(budget)
    }

    /// The table adds up, exactly, to the published totals it carries
    /// (AC-BUD-01 to 04, 13).
    ///
    /// Checks, in this order:
    /// 1. the lines are the spec's thirteen, each once (a repeated id is
    ///    [`BudgetError::DuplicateLine`]; an id that is not a line of the
    ///    spec, or a line missing, is [`BudgetError::Shape`] naming it, since
    ///    the sketch has no variant of its own);
    /// 2. no amount is negative except `non_produced_assets`
    ///    ([`BudgetError::Shape`] naming the line);
    /// 3. the lines' COFOG subgroups are a partition: each line has some, each
    ///    is a six-character subgroup of one of the ten divisions and none is
    ///    in two lines ([`BudgetError::DivisionMismatch`] naming the subgroup,
    ///    or [`BudgetError::Shape`] naming a line without subgroups);
    /// 4. each type over the lines equals its published total, in type order,
    ///    then all lines equal `TE` ([`BudgetError::TotalMismatch`]; a
    ///    published total that is missing counts as zero);
    /// 5. the programmes' cash and in kind equal `D62` and `D632`
    ///    ([`BudgetError::TotalMismatch`], `lines` being the programmes' sum);
    /// 6. every sector's unit is a public firm unit, and the public units are
    ///    the five, each once ([`BudgetError::UnknownUnit`]: a unit that is not
    ///    one of the five, a repeated one or a missing one);
    /// 7. each public unit's posts are its sectors' posts
    ///    ([`BudgetError::PostsMismatch`]);
    /// 8. no public unit has wages or employers' contributions without posts
    ///    ([`BudgetError::WagesWithoutPosts`]);
    /// 9. the public units' wages and employers' contributions equal `D11` and
    ///    `D12` ([`BudgetError::TotalMismatch`], `lines` being the units' sum).
    ///
    /// Sums are taken in `i128`, so no amount `i64` can hold overflows them.
    ///
    /// # Errors
    /// The first inconsistency found.
    pub fn check(&self) -> Result<(), BudgetError> {
        // 1. The thirteen lines, each once.
        let mut seen: Vec<&str> = Vec::with_capacity(self.lines.len());
        for l in &self.lines {
            if seen.contains(&l.id.as_str()) {
                return Err(BudgetError::DuplicateLine(l.id.clone()));
            }
            if !LINE_IDS.contains(&l.id.as_str()) {
                return Err(shape(l.id.clone()));
            }
            seen.push(&l.id);
        }
        if let Some(missing) = LINE_IDS.iter().find(|id| !seen.contains(id)) {
            return Err(shape(*missing));
        }

        // 2. Signs.
        for l in &self.lines {
            for (&(t, _, _), a) in TYPES.iter().zip(&l.per_year) {
                if a.0 < 0 && t != EconType::NonProducedAssets {
                    return Err(shape(l.id.clone()));
                }
            }
        }

        // 3. The partition of the COFOG subgroups.
        let mut subgroups = std::collections::BTreeSet::new();
        for l in &self.lines {
            if l.cofog.is_empty() {
                return Err(shape(l.id.clone()));
            }
            for s in &l.cofog {
                let well_formed = s.len() == 6 && DIVISIONS.iter().any(|d| s.starts_with(d));
                if !well_formed || !subgroups.insert(s.as_str()) {
                    return Err(BudgetError::DivisionMismatch { cofog: s.clone() });
                }
            }
        }

        // 4. Each type, then the total, against the published totals.
        let mut te: i128 = 0;
        for &(t, _, code) in &TYPES {
            let sum = i128::from(self.total(t).0);
            self.against(code, sum)?;
            te += sum;
        }
        self.against(TE, te)?;

        // 5. The programmes against the social benefits.
        let cash: i128 = self.programmes.iter().map(|p| i128::from(p.cash.0)).sum();
        let kind: i128 = self
            .programmes
            .iter()
            .map(|p| i128::from(p.in_kind.0))
            .sum();
        self.against(TYPES[EconType::SocialBenefitsCash as usize].2, cash)?;
        self.against(TYPES[EconType::SocialBenefitsInKind as usize].2, kind)?;

        // 6. The public firm units.
        if let Some(s) = self
            .sectors
            .iter()
            .find(|s| !PUBLIC_UNITS.contains(&s.unit.as_str()))
        {
            return Err(BudgetError::UnknownUnit(s.unit.clone()));
        }
        let mut units: Vec<&str> = Vec::with_capacity(self.public_units.len());
        for u in &self.public_units {
            if !PUBLIC_UNITS.contains(&u.unit.as_str()) || units.contains(&u.unit.as_str()) {
                return Err(BudgetError::UnknownUnit(u.unit.clone()));
            }
            units.push(&u.unit);
        }
        if let Some(missing) = PUBLIC_UNITS.iter().find(|u| !units.contains(u)) {
            return Err(BudgetError::UnknownUnit((*missing).to_string()));
        }

        // 7. Posts: every sector's unit is one of the units, so the units'
        // posts then also add up to the sectors' posts.
        for u in &self.public_units {
            let of_sectors: u64 = self
                .sectors
                .iter()
                .filter(|s| s.unit == u.unit)
                .map(|s| u64::from(s.posts))
                .sum();
            if of_sectors != u64::from(u.posts) {
                return Err(BudgetError::PostsMismatch);
            }
        }

        // 8. Wages without posts.
        if let Some(u) = self
            .public_units
            .iter()
            .find(|u| u.posts == 0 && (u.wages != Bani(0) || u.employer_contributions != Bani(0)))
        {
            return Err(BudgetError::WagesWithoutPosts(u.unit.clone()));
        }

        // 9. The units' wages and contributions against D11 and D12.
        let wages: i128 = self
            .public_units
            .iter()
            .map(|u| i128::from(u.wages.0))
            .sum();
        let contributions: i128 = self
            .public_units
            .iter()
            .map(|u| i128::from(u.employer_contributions.0))
            .sum();
        self.against(TYPES[EconType::Wages as usize].2, wages)?;
        self.against(
            TYPES[EconType::EmployerContributions as usize].2,
            contributions,
        )?;
        Ok(())
    }

    /// The published total `code` (zero if the file has none).
    fn published(&self, code: &str) -> Bani {
        self.published
            .0
            .iter()
            .find(|(c, _)| c == code)
            .map_or(Bani(0), |&(_, b)| b)
    }

    /// `sum` must equal the published total `code`.
    fn against(&self, code: &str, sum: i128) -> Result<(), BudgetError> {
        let published = self.published(code);
        if sum == i128::from(published.0) {
            Ok(())
        } else {
            Err(BudgetError::TotalMismatch {
                code: code.to_string(),
                lines: saturate(sum),
                published,
            })
        }
    }

    /// The amount of month `month` (0 to 11, counted from the start month) of
    /// an amount a year: whole bani, the remainder to the first months, the
    /// sign of a negative amount kept (spec, "Monthly amounts"; AC-BUD-10).
    ///
    /// `per_year` is in bani: `q = |A| / 12`, `r = |A| mod 12`, month `m` gets
    /// `q + 1` if `m < r`, else `q`, with the sign of `A`; the twelve add up to
    /// `A` exactly. A month past 11 is taken modulo 12 (the amount in force
    /// repeats each year until a lever changes it).
    pub fn monthly(per_year: Bani, month: u8) -> Bani {
        let m = u64::from(month % 12);
        let a = per_year.0.unsigned_abs();
        let v = a / 12 + u64::from(m < a % 12);
        // |A| / 12 + 1 is far below i64::MAX, even for i64::MIN.
        let v = i64::try_from(v).unwrap_or(i64::MAX);
        if per_year.0 < 0 { Bani(-v) } else { Bani(v) }
    }

    /// Spending a year of type `t` over all lines (summed in `i128`; a sum
    /// beyond `i64`, impossible for a loaded file, saturates).
    pub fn total(&self, t: EconType) -> Bani {
        saturate(
            self.lines
                .iter()
                .map(|l| i128::from(l.per_year[t as usize].0))
                .sum(),
        )
    }

    /// The budget hash (FNV-1a 64; AC-BUD-11): the line count (u32 LE); per
    /// line its id (one length byte, then UTF-8) and each amount in bani (i64
    /// LE) in type order; the sector count (u32 LE); per sector its id and
    /// posts (u32 LE); per public unit, in file order, its id, posts (u32 LE),
    /// wages and employers' contributions in bani (i64 LE). Programmes and
    /// published totals are not hashed (spec). Counts beyond `u32` saturate.
    #[must_use]
    pub fn hash(&self) -> u64 {
        let count = |n: usize| u32::try_from(n).unwrap_or(u32::MAX).to_le_bytes();
        let mut h = econ_core::StateHasher::default();
        h.write(&count(self.lines.len()));
        for l in &self.lines {
            write_id(&mut h, &l.id);
            for a in &l.per_year {
                h.write(&a.0.to_le_bytes());
            }
        }
        h.write(&count(self.sectors.len()));
        for s in &self.sectors {
            write_id(&mut h, &s.id);
            h.write(&s.posts.to_le_bytes());
        }
        for u in &self.public_units {
            write_id(&mut h, &u.unit);
            h.write(&u.posts.to_le_bytes());
            h.write(&u.wages.0.to_le_bytes());
            h.write(&u.employer_contributions.0.to_le_bytes());
        }
        h.finish()
    }
}

/// Step 6 of [`Budget::from_json`]: the ten COFOG divisions, each against
/// Eurostat's published total, and all of them against the lines by type.
fn check_divisions(fns: &[RawFunction], budget: &Budget) -> Result<(), BudgetError> {
    let mismatch = |cofog: &str| BudgetError::DivisionMismatch {
        cofog: cofog.to_string(),
    };
    for (k, d) in DIVISIONS.iter().enumerate() {
        let Some(f) = fns.get(k) else {
            return Err(mismatch(d));
        };
        if f.cofog != *d {
            return Err(mismatch(&f.cofog));
        }
        let sum: i128 = f.spending.iter().map(|&a| i128::from(a)).sum();
        if f.spending.len() != N_TYPES || sum != i128::from(f.total) || f.total != f.published_total
        {
            return Err(mismatch(d));
        }
    }
    if let Some(extra) = fns.get(DIVISIONS.len()) {
        return Err(mismatch(&extra.cofog));
    }
    for &(t, _, _) in &TYPES {
        let by_fn: i128 = fns
            .iter()
            .map(|f| i128::from(f.spending[t as usize]) * 100)
            .sum();
        if by_fn != i128::from(budget.total(t).0) {
            return Err(mismatch(ALL_DIVISIONS));
        }
    }
    Ok(())
}
