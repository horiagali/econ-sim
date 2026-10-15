//! Undeclared sales and opening inventories of the firm units (spec
//! `economy/firm-units-undeclared-inventories`): seven columns per unit, a pure
//! function of the firm-unit file and its margin file, in integer arithmetic,
//! with no randomness.
//!
//! **Skeleton** (ADR-0014): the API of the spec's sketch, written with the
//! acceptance tests; the bodies come with the implementation. Test-writer
//! additions: the two readers (the margin file's tilt tables and industry ids
//! can only be checked where they are parsed) and [`FirmOpeningError::Parse`].

use econ_popgen::FirmUnits;

/// What the rule reads of a unit from the firm-unit file, in lei a year.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FirmUnitAmounts {
    /// Output.
    pub output: u64,
    /// Value added (may be negative).
    pub value_added: i64,
    /// Wage bill.
    pub wages: u64,
}

/// The margin file's entry for one catalogue industry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IndustryMargins {
    /// Days of a year's output held as inventories.
    pub inventory_days: u32,
    /// Share of inventories that is the unit's own output, millionths.
    pub finished_share_ppm: u32,
    /// Households' purchases from the industry, lei a year (a check only).
    pub household_purchases: u64,
}

/// The margin file (`firm_undeclared_inventories2021_ro.json`). Shares and
/// tilts in millionths; amounts in lei. Tilts are by class: micro, small,
/// large, foreign-controlled.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(missing_docs)]
pub struct FirmOpeningMargins {
    /// In the firm file's industry order.
    pub industries: Vec<IndustryMargins>,
    /// Per industry group.
    pub undeclared_wage_share_ppm: Vec<u32>,
    /// Per industry group.
    pub household_business_propensity_ppm: Vec<u32>,
    /// Per industry group.
    pub surplus_from_sales_ppm: Vec<u32>,
    pub undeclared_farm_income: u64,
    pub undeclared_self_employment_income: u64,
    pub informal_income: u64,
    pub undeclared_value_added_share_ppm: u32,
    pub wage_tilt_ppm: [u32; 4],
    pub informal_tilt_ppm: [u32; 4],
    pub profit_tilt_ppm: [u32; 4],
    pub profit_group_floor_ppm: u32,
    pub max_undeclared_share_ppm: u32,
    pub inventories_corporations: u64,
    pub inventories_household_business: u64,
}

/// The seven columns, one entry per unit in the firm file's order.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(missing_docs)]
pub struct FirmOpening {
    /// Lei a year.
    pub undeclared_wages: Vec<u64>,
    /// Lei a year.
    pub undeclared_value_added: Vec<u64>,
    /// Lei a year.
    pub undeclared_sales: Vec<u64>,
    pub undeclared_wage_share_ppm: Vec<u32>,
    pub undeclared_sales_share_ppm: Vec<u32>,
    /// Lei: the unit's own output in stock.
    pub inventory: Vec<u64>,
    /// Lei: materials, supplies and goods bought for resale.
    pub inventory_materials: Vec<u64>,
}

/// Why nothing was produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FirmOpeningError {
    /// No units or no industries.
    EmptyTable,
    /// Industries that differ from the firm file's, a unit of an unlisted
    /// industry, or a table of the wrong length.
    ShapeMismatch,
    /// A share above one million millionths.
    ShareAboveOne,
    /// The anchor is below what Steps 1 and 2 place.
    AnchorsInconsistent,
    /// More to place than the units have room for.
    NoRoom,
    /// The text is not such a file.
    Parse(String),
}

/// Steps 1 to 5 of the spec.
///
/// # Errors
/// See [`FirmOpeningError`]; nothing is produced.
pub fn firm_opening(
    firms: &FirmUnits,
    amounts: &[FirmUnitAmounts],
    m: &FirmOpeningMargins,
) -> Result<FirmOpening, FirmOpeningError> {
    let _ = (firms, amounts, m);
    unimplemented!("economy/firm-units-undeclared-inventories: steps 1 to 5")
}

impl FirmOpening {
    /// FNV-1a 64 over the unit count (u32, little-endian), then for each unit
    /// the seven columns as u64 little-endian, in the order of the spec's
    /// Outputs table.
    #[must_use]
    pub fn state_hash(&self) -> u64 {
        unimplemented!("economy/firm-units-undeclared-inventories: the hash")
    }
}

/// Each unit's output, value added and wages from `firm_units2021_ro.json`,
/// in its order.
///
/// # Errors
/// [`FirmOpeningError::Parse`] if the JSON is not such a file.
pub fn read_firm_unit_amounts(json: &str) -> Result<Vec<FirmUnitAmounts>, FirmOpeningError> {
    let _ = json;
    unimplemented!("economy/firm-units-undeclared-inventories: read the amounts")
}

/// Parse the margin file. `industry_ids` are the firm file's industries, in
/// its order.
///
/// # Errors
/// [`FirmOpeningError::ShapeMismatch`] if its industries are not those ids in
/// that order or a tilt table has not four entries;
/// [`FirmOpeningError::Parse`] if it is not such a file.
pub fn read_firm_opening_margins(
    json: &str,
    industry_ids: &[&str],
) -> Result<FirmOpeningMargins, FirmOpeningError> {
    let _ = (json, industry_ids);
    unimplemented!("economy/firm-units-undeclared-inventories: read the margins")
}
