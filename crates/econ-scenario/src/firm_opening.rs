//! Undeclared sales and opening inventories of the firm units (spec
//! `economy/firm-units-undeclared-inventories`): seven columns per unit, a pure
//! function of the firm-unit file and its margin file, in integer arithmetic,
//! with no randomness.
//!
//! Every split is by largest remainder with ties to the lower index, as in
//! the reference (`python/reference/firm_reference.py`), so both agree to the
//! leu.

use econ_popgen::{FirmUnits, UnitSize};
use serde_json::Value;

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

/// One million: shares and tilts are millionths.
const UNIT: i128 = 1_000_000;
/// Classes of a cohort unit: micro, small, large (its size), foreign-controlled.
const CLASSES: usize = 4;

/// Split `total` in proportion to `weights` by largest remainder, ties to the
/// lower index. The reference's `apportion`.
// Amounts are lei below 10^13 and weights below 10^24, so every product
// stays inside i128.
#[allow(clippy::arithmetic_side_effects)]
fn apportion(total: i128, weights: &[i128]) -> Result<Vec<i128>, FirmOpeningError> {
    if total == 0 {
        return Ok(vec![0; weights.len()]);
    }
    let wsum: i128 = weights.iter().sum();
    if wsum <= 0 {
        return Err(FirmOpeningError::NoRoom);
    }
    let mut parts: Vec<i128> = weights
        .iter()
        .map(|&w| (total * w).div_euclid(wsum))
        .collect();
    let mut order: Vec<usize> = (0..weights.len()).collect();
    order.sort_by_key(|&i| (-(total * weights[i]).rem_euclid(wsum), i));
    let left = usize::try_from(total - parts.iter().sum::<i128>()).unwrap_or(0);
    for &i in order.iter().take(left) {
        parts[i] += 1;
    }
    Ok(parts)
}

/// Split `total` by `weights`, no part above its cap: apportion over the
/// members with a weight and room, fix every member that would pass its cap
/// at the cap, and apportion what is left over the others again.
#[allow(clippy::arithmetic_side_effects)]
fn apportion_capped(
    total: i128,
    weights: &[i128],
    caps: &[i128],
) -> Result<Vec<i128>, FirmOpeningError> {
    if total < 0 {
        return Err(FirmOpeningError::AnchorsInconsistent);
    }
    let mut parts = vec![0i128; weights.len()];
    let mut active: Vec<usize> = (0..weights.len())
        .filter(|&i| weights[i] > 0 && caps[i] > 0)
        .collect();
    let mut left = total;
    while left > 0 {
        if active.is_empty() {
            return Err(FirmOpeningError::NoRoom);
        }
        let share = apportion(
            left,
            &active.iter().map(|&i| weights[i]).collect::<Vec<_>>(),
        )?;
        let over: Vec<usize> = active
            .iter()
            .zip(&share)
            .filter(|&(&i, &s)| parts[i] + s > caps[i])
            .map(|(&i, _)| i)
            .collect();
        if over.is_empty() {
            for (&i, &s) in active.iter().zip(&share) {
                parts[i] += s;
            }
            break;
        }
        for &i in &over {
            left -= caps[i] - parts[i];
            parts[i] = caps[i];
        }
        active.retain(|i| !over.contains(i));
    }
    Ok(parts)
}

fn check_inputs(
    firms: &FirmUnits,
    amounts: &[FirmUnitAmounts],
    m: &FirmOpeningMargins,
) -> Result<usize, FirmOpeningError> {
    if firms.units.is_empty() || firms.industry_group.is_empty() {
        return Err(FirmOpeningError::EmptyTable);
    }
    let groups = usize::from(firms.industry_group.iter().copied().max().unwrap_or(0)) + 1;
    let shape_ok = m.industries.len() == firms.industry_group.len()
        && amounts.len() == firms.units.len()
        && firms
            .units
            .iter()
            .all(|u| usize::from(u.industry) < firms.industry_group.len())
        && [
            &m.undeclared_wage_share_ppm,
            &m.household_business_propensity_ppm,
            &m.surplus_from_sales_ppm,
        ]
        .iter()
        .all(|t| t.len() == groups);
    if !shape_ok {
        return Err(FirmOpeningError::ShapeMismatch);
    }
    let one = 1_000_000;
    let shares_ok = m
        .undeclared_wage_share_ppm
        .iter()
        .chain(&m.surplus_from_sales_ppm)
        .chain([
            &m.undeclared_value_added_share_ppm,
            &m.max_undeclared_share_ppm,
        ])
        .chain(m.industries.iter().map(|i| &i.finished_share_ppm))
        .all(|&s| s <= one);
    if !shares_ok {
        return Err(FirmOpeningError::ShareAboveOne);
    }
    Ok(groups)
}

/// Steps 1 to 5 of the spec.
///
/// # Errors
/// See [`FirmOpeningError`]; nothing is produced.
// Integer arithmetic in i128 on lei below 10^13: nothing overflows.
#[allow(clippy::arithmetic_side_effects, clippy::too_many_lines)]
pub fn firm_opening(
    firms: &FirmUnits,
    amounts: &[FirmUnitAmounts],
    m: &FirmOpeningMargins,
) -> Result<FirmOpening, FirmOpeningError> {
    let groups = check_inputs(firms, amounts, m)?;
    let n = firms.units.len();
    let units = &firms.units;
    let group: Vec<usize> = units
        .iter()
        .map(|u| usize::from(firms.industry_group[usize::from(u.industry)]))
        .collect();
    let margin: Vec<&IndustryMargins> = units
        .iter()
        .map(|u| &m.industries[usize::from(u.industry)])
        .collect();
    let cohort: Vec<bool> = units.iter().map(|u| u.size != UnitSize::Public).collect();
    // A cohort unit's class: its size, or foreign if it is controlled from abroad.
    let class: Vec<usize> = units
        .iter()
        .map(|u| match u.size {
            _ if !u.domestic => 3,
            UnitSize::Micro => 0,
            UnitSize::Small => 1,
            _ => 2,
        })
        .collect();
    let output: Vec<i128> = amounts.iter().map(|a| i128::from(a.output)).collect();
    let wages: Vec<i128> = amounts.iter().map(|a| i128::from(a.wages)).collect();
    let raw_va: Vec<i128> = amounts.iter().map(|a| i128::from(a.value_added)).collect();
    let va: Vec<i128> = raw_va.iter().map(|&v| v.max(0)).collect();
    let ppm = |x: u32| i128::from(x);
    // Surplus earned on sales: the rest (imputed rent of owner-occupiers, in
    // real estate) cannot be hidden.
    let surplus: Vec<i128> = (0..n)
        .map(|f| (raw_va[f] - wages[f]).max(0) * ppm(m.surplus_from_sales_ppm[group[f]]) / UNIT)
        .collect();
    let room: Vec<i128> = (0..n)
        .map(|f| {
            let hideable = (va[f] - (raw_va[f] - wages[f]).max(0) + surplus[f]).max(0);
            if cohort[f] {
                hideable * ppm(m.max_undeclared_share_ppm) / UNIT
            } else {
                0
            }
        })
        .collect();

    // Step 1: undeclared wages, per industry group, by wage bill times the class's tilt.
    let mut wages_u = vec![0i128; n];
    for g in 0..groups {
        let members: Vec<usize> = (0..n).filter(|&f| cohort[f] && group[f] == g).collect();
        if members.is_empty() {
            continue;
        }
        let target = members.iter().map(|&f| wages[f]).sum::<i128>()
            * ppm(m.undeclared_wage_share_ppm[g])
            / UNIT;
        if target == 0 {
            continue;
        }
        let weights: Vec<i128> = members
            .iter()
            .map(|&f| wages[f] * ppm(m.wage_tilt_ppm[class[f]]))
            .collect();
        let caps: Vec<i128> = members.iter().map(|&f| wages[f].min(room[f])).collect();
        for (&f, x) in members
            .iter()
            .zip(apportion_capped(target, &weights, &caps)?)
        {
            wages_u[f] = x;
        }
    }

    // Step 2: households' undeclared income: farm income in the domestic micro
    // units of agriculture, self-employment income in the domestic micro
    // units, informal income in the domestic cohort units.
    let domestic: Vec<bool> = (0..n).map(|f| cohort[f] && units[f].domestic).collect();
    let micro: Vec<bool> = (0..n)
        .map(|f| domestic[f] && units[f].size == UnitSize::Micro)
        .collect();
    let prop = &m.household_business_propensity_ppm;
    let mut house = vec![0i128; n];
    let placements: [(u64, Vec<i128>); 3] = [
        (
            m.undeclared_farm_income,
            (0..n)
                .map(|f| {
                    if micro[f] && group[f] == 0 {
                        surplus[f]
                    } else {
                        0
                    }
                })
                .collect(),
        ),
        (
            m.undeclared_self_employment_income,
            (0..n)
                .map(|f| {
                    if micro[f] {
                        surplus[f] * ppm(prop[group[f]])
                    } else {
                        0
                    }
                })
                .collect(),
        ),
        (
            m.informal_income,
            (0..n)
                .map(|f| {
                    if domestic[f] {
                        surplus[f] * ppm(m.informal_tilt_ppm[class[f]])
                    } else {
                        0
                    }
                })
                .collect(),
        ),
    ];
    for (amount, weights) in placements {
        let caps: Vec<i128> = (0..n).map(|f| room[f] - wages_u[f] - house[f]).collect();
        let got = apportion_capped(i128::from(amount), &weights, &caps)?;
        for (h, g) in house.iter_mut().zip(got) {
            *h += g;
        }
    }

    // Step 3: undeclared profit of firms: what the anchor leaves, by surplus,
    // industry group and class.
    let cohort_va: i128 = (0..n).filter(|&f| cohort[f]).map(|f| va[f]).sum();
    let target = cohort_va * ppm(m.undeclared_value_added_share_ppm) / UNIT;
    let rest = target - wages_u.iter().sum::<i128>() - house.iter().sum::<i128>();
    if rest < 0 {
        return Err(FirmOpeningError::AnchorsInconsistent);
    }
    let weights: Vec<i128> = (0..n)
        .map(|f| {
            if cohort[f] {
                surplus[f]
                    * (ppm(m.undeclared_wage_share_ppm[group[f]]) + ppm(m.profit_group_floor_ppm))
                    * ppm(m.profit_tilt_ppm[class[f]])
            } else {
                0
            }
        })
        .collect();
    let caps: Vec<i128> = (0..n).map(|f| room[f] - wages_u[f] - house[f]).collect();
    let profit = apportion_capped(rest, &weights, &caps)?;

    let lei = |x: i128| u64::try_from(x).expect("a non-negative amount");
    let share = |x: i128| u32::try_from(x).expect("a share");
    let mut out = FirmOpening {
        undeclared_wages: Vec::with_capacity(n),
        undeclared_value_added: Vec::with_capacity(n),
        undeclared_sales: Vec::with_capacity(n),
        undeclared_wage_share_ppm: Vec::with_capacity(n),
        undeclared_sales_share_ppm: Vec::with_capacity(n),
        inventory: Vec::with_capacity(n),
        inventory_materials: Vec::with_capacity(n),
    };
    for f in 0..n {
        let uva = wages_u[f] + house[f] + profit[f];
        let sales = if va[f] > 0 {
            output[f].min(uva * output[f] / va[f])
        } else {
            0
        };
        out.undeclared_wages.push(lei(wages_u[f]));
        out.undeclared_value_added.push(lei(uva));
        out.undeclared_sales.push(lei(sales));
        out.undeclared_wage_share_ppm.push(if wages[f] > 0 {
            share(wages_u[f] * UNIT / wages[f])
        } else {
            0
        });
        out.undeclared_sales_share_ppm.push(if output[f] > 0 {
            share(sales * UNIT / output[f])
        } else {
            0
        });
    }

    // Inventories: corporations' over every cohort unit, households' business
    // inventories over the domestic micro units, both by output times the
    // industry's days; then finished goods and materials.
    let key: Vec<i128> = (0..n)
        .map(|f| {
            if cohort[f] {
                output[f] * i128::from(margin[f].inventory_days)
            } else {
                0
            }
        })
        .collect();
    let corp = apportion(i128::from(m.inventories_corporations), &key)?;
    let micro_key: Vec<i128> = (0..n).map(|f| if micro[f] { key[f] } else { 0 }).collect();
    let hh = apportion(i128::from(m.inventories_household_business), &micro_key)?;
    for f in 0..n {
        let total = corp[f] + hh[f];
        let finished = total * ppm(margin[f].finished_share_ppm) / UNIT;
        out.inventory.push(lei(finished));
        out.inventory_materials.push(lei(total - finished));
    }
    Ok(out)
}

impl FirmOpening {
    /// FNV-1a 64 over the unit count (u32, little-endian), then for each unit
    /// the seven columns as u64 little-endian, in the order of the spec's
    /// Outputs table.
    #[must_use]
    pub fn state_hash(&self) -> u64 {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        let mut write = |bytes: &[u8]| {
            for &byte in bytes {
                h = (h ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3);
            }
        };
        let n = self.undeclared_wages.len();
        write(&u32::try_from(n).expect("few units").to_le_bytes());
        for f in 0..n {
            for v in [
                self.undeclared_wages[f],
                self.undeclared_value_added[f],
                self.undeclared_sales[f],
                u64::from(self.undeclared_wage_share_ppm[f]),
                u64::from(self.undeclared_sales_share_ppm[f]),
                self.inventory[f],
                self.inventory_materials[f],
            ] {
                write(&v.to_le_bytes());
            }
        }
        h
    }
}

fn parse(why: impl Into<String>) -> FirmOpeningError {
    FirmOpeningError::Parse(why.into())
}

fn field<'a>(v: &'a Value, key: &str) -> Result<&'a Value, FirmOpeningError> {
    v.get(key).ok_or_else(|| parse(format!("no `{key}`")))
}

fn as_u64(v: &Value, key: &str) -> Result<u64, FirmOpeningError> {
    field(v, key)?
        .as_u64()
        .ok_or_else(|| parse(format!("`{key}` is not a whole non-negative number")))
}

fn as_u32(v: &Value, key: &str) -> Result<u32, FirmOpeningError> {
    u32::try_from(as_u64(v, key)?).map_err(|_| parse(format!("`{key}` is too large")))
}

fn u32_list(v: &Value, key: &str) -> Result<Vec<u32>, FirmOpeningError> {
    field(v, key)?
        .as_array()
        .ok_or_else(|| parse(format!("`{key}` is not a list")))?
        .iter()
        .map(|x| {
            x.as_u64()
                .and_then(|x| u32::try_from(x).ok())
                .ok_or_else(|| parse(format!("`{key}` holds a value that is not a share")))
        })
        .collect()
}

/// Each unit's output, value added and wages from `firm_units2021_ro.json`,
/// in its order.
///
/// # Errors
/// [`FirmOpeningError::Parse`] if the JSON is not such a file.
pub fn read_firm_unit_amounts(json: &str) -> Result<Vec<FirmUnitAmounts>, FirmOpeningError> {
    let v: Value = serde_json::from_str(json).map_err(|e| parse(e.to_string()))?;
    field(&v, "units")?
        .as_array()
        .ok_or_else(|| parse("`units` is not a list"))?
        .iter()
        .map(|u| {
            Ok(FirmUnitAmounts {
                output: as_u64(u, "output")?,
                value_added: field(u, "value_added")?
                    .as_i64()
                    .ok_or_else(|| parse("`value_added` is not whole lei"))?,
                wages: as_u64(u, "wages")?,
            })
        })
        .collect()
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
    let v: Value = serde_json::from_str(json).map_err(|e| parse(e.to_string()))?;
    if !v.is_object() {
        return Err(parse("not a margin file"));
    }
    let rows = field(&v, "industries")?
        .as_array()
        .ok_or_else(|| parse("`industries` is not a list"))?;
    let ids: Vec<&str> = rows
        .iter()
        .map(|r| {
            field(r, "id")?
                .as_str()
                .ok_or_else(|| parse("an industry id is not text"))
        })
        .collect::<Result<_, _>>()?;
    if ids != industry_ids {
        return Err(FirmOpeningError::ShapeMismatch);
    }
    let industries = rows
        .iter()
        .map(|r| {
            Ok(IndustryMargins {
                inventory_days: as_u32(r, "inventory_days")?,
                finished_share_ppm: as_u32(r, "finished_share")?,
                household_purchases: as_u64(r, "household_purchases")?,
            })
        })
        .collect::<Result<_, FirmOpeningError>>()?;
    let tilt = |key: &str| -> Result<[u32; CLASSES], FirmOpeningError> {
        <[u32; CLASSES]>::try_from(u32_list(&v, key)?).map_err(|_| FirmOpeningError::ShapeMismatch)
    };
    let inventories = field(&v, "inventories")?;
    Ok(FirmOpeningMargins {
        industries,
        undeclared_wage_share_ppm: u32_list(&v, "undeclared_wage_share")?,
        household_business_propensity_ppm: u32_list(&v, "household_business_propensity")?,
        surplus_from_sales_ppm: u32_list(&v, "surplus_from_sales")?,
        undeclared_farm_income: as_u64(&v, "undeclared_farm_income")?,
        undeclared_self_employment_income: as_u64(&v, "undeclared_self_employment_income")?,
        informal_income: as_u64(&v, "informal_income")?,
        undeclared_value_added_share_ppm: as_u32(&v, "undeclared_value_added_share")?,
        wage_tilt_ppm: tilt("wage_tilt")?,
        informal_tilt_ppm: tilt("informal_tilt")?,
        profit_tilt_ppm: tilt("profit_tilt")?,
        profit_group_floor_ppm: as_u32(&v, "profit_group_floor")?,
        max_undeclared_share_ppm: as_u32(&v, "max_undeclared_share")?,
        inventories_corporations: as_u64(inventories, "corporations")?,
        inventories_household_business: as_u64(inventories, "household_business")?,
    })
}
