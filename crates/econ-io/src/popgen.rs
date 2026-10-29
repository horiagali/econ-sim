//! Population generator I/O (spec `society/population-generator`, ADR-0012):
//! the margin file in, Arrow tables and the fit report out. `econ-popgen`
//! itself does no I/O.
//!
//! | File | Content |
//! |---|---|
//! | `households.arrow` | `hh_weight` (u32), `hh_county` (u8, index into `counties`), `hh_collective` (bool), `hh_key` (u64, the index) |
//! | `persons.arrow` | `household_id` (u32), `age` (u16, months), `sex` (u8: F 0, M 1), `role` (u8: head 0, partner 1, child 2, other 3), `person_key` (u64, the index) |
//! | `fit_report.json` | parameters, table sizes, weighted totals, state hash, fit report, county codes, provenance of the margins |
//!
//! The stages after the fit each read one more margin file and add columns
//! (codes as in their specs) and, to the fit report, their hash and the
//! provenance of their file ([`Stages`]):
//!
//! | Stage | Spec | Columns |
//! |---|---|---|
//! | C | `society/population-attributes` | persons: `edu_level`, `activity` (u8) |
//! | D | `society/population-jobs` | persons: `employment_status`, `occupation`, `industry_group` (u8) |
//! | E | `society/population-housing` | households: `hh_locality_size` (u8), `hh_urban` (bool), `hh_tenure` (u8) |
//! | F | `society/population-wages` | persons: `wage` (u64, gross monthly, bani) |
//! | G | `society/population-pensions` | persons: `pension` (u64, gross monthly, bani) |
//! | H | `society/population-wealth` | households: `hh_cash`, `hh_deposits`, `hh_dwelling_value`, `hh_mortgage`, `hh_consumer_debt` (u64, bani) |
//! | I | `society/population-incomes` | persons: `wage_declared`, `self_employment_income`, `self_employment_declared`, `informal_income` (u64, bani a month); households: `hh_subsistence_income` (u64) |
//! | J | `society/population-employers` | persons: `industry` (u8, catalogue index or 255), `employer` (u16, firm unit or 65535) |
//! | K | `society/population-inactivity-enrolment` | persons: `inactive_kind`, `enrolled` (u8) |
//!
//! Stage L (linked emigrants and the diaspora stock) writes tables of its own: [`crate::people`].

use std::path::Path;
use std::sync::Arc;

use arrow_array::{ArrayRef, BooleanArray, UInt8Array, UInt16Array, UInt32Array, UInt64Array};
use arrow_schema::DataType;
use econ_core::scale_spike::SeedPopulation;
use econ_popgen::{
    AttributeMargins, BorrowerRule, Curve, EDU_GROUPS, EDU_LEVELS, FirmUnit, FirmUnits, GenParams,
    HouseholdHousing, HouseholdWealth, HousingMargins, INDUSTRY_GROUPS, IncomeMargins,
    InformalPropensity, JobMargins, KINDS, MARGIN_ACTIVITIES, Margins, OCCUPATIONS, OPEN,
    OtherIncomes, PensionMargins, PersonAttributes, PersonEmployers, PersonInactivityEnrolment,
    PersonJobs, PersonPensions, PersonWages, Population, SIZE_GROUPS, STATUSES, TENURES, UnitSize,
    WageMargins, WealthMargins,
};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::{SaveError, batch, io_err, ipc_bytes};

/// A margin file: the generator's input tables plus where they came from.
#[derive(Debug, Clone, PartialEq)]
pub struct MarginFile {
    /// The tables.
    pub margins: Margins,
    /// The file's `provenance` block, passed through to the fit report.
    pub provenance: Value,
}

/// The JSON layout written by `python/pipeline/normalise_census.py`.
#[derive(Deserialize)]
struct RawMargins {
    counties: Vec<String>,
    sexes: Vec<String>,
    age_bands: Vec<(u16, Option<u16>)>,
    size_classes: Vec<(u16, Option<u16>)>,
    persons_private: Vec<Vec<Vec<u64>>>,
    persons_collective: Vec<Vec<Vec<u64>>>,
    households: Vec<Vec<u64>>,
    #[serde(default)]
    provenance: Value,
}

/// Parse a margin file. Ragged tables are rejected here, so a wrong row
/// cannot hide behind a flat table of the right total length.
///
/// # Errors
/// `SaveError::Parse` if the JSON is not a margin file.
pub fn read_margins(json: &str) -> Result<MarginFile, SaveError> {
    let parse_err = |what: &str| SaveError::Parse(format!("margin file: {what}"));
    let raw: RawMargins = serde_json::from_str(json).map_err(|e| parse_err(&e.to_string()))?;
    if raw.sexes != ["F", "M"] {
        return Err(parse_err("sexes must be [\"F\", \"M\"]"));
    }
    let (n_c, n_b, n_k) = (
        raw.counties.len(),
        raw.age_bands.len(),
        raw.size_classes.len(),
    );
    let persons = |table: Vec<Vec<Vec<u64>>>, name: &str| {
        let regular = table.len() == n_c
            && table
                .iter()
                .all(|c| c.len() == 2 && c.iter().all(|s| s.len() == n_b));
        if regular {
            Ok(table.into_iter().flatten().flatten().collect::<Vec<u64>>())
        } else {
            Err(parse_err(&format!("{name} is not [county][sex][age_band]")))
        }
    };
    let persons_private = persons(raw.persons_private, "persons_private")?;
    let persons_collective = persons(raw.persons_collective, "persons_collective")?;
    if raw.households.len() != n_c || raw.households.iter().any(|c| c.len() != n_k) {
        return Err(parse_err("households is not [county][size_class]"));
    }
    let open = |pairs: Vec<(u16, Option<u16>)>| -> Vec<(u16, u16)> {
        pairs
            .into_iter()
            .map(|(lo, hi)| (lo, hi.unwrap_or(OPEN)))
            .collect()
    };
    Ok(MarginFile {
        margins: Margins {
            counties: raw.counties,
            age_bands: open(raw.age_bands),
            size_classes: open(raw.size_classes),
            persons_private,
            persons_collective,
            households: raw.households.into_iter().flatten().collect(),
        },
        provenance: raw.provenance,
    })
}

/// The margin file of a stage after the fit: its tables plus where they came from.
#[derive(Debug, Clone, PartialEq)]
pub struct StageFile<M> {
    /// The tables.
    pub margins: M,
    /// The file's `provenance` block, passed through to the fit report.
    pub provenance: Value,
}

/// What the stages after the fit produced, each with the `provenance` block
/// of its margin file. A stage that was not run is `None`.
#[derive(Debug, Clone, Copy, Default)]
pub struct Stages<'a> {
    /// Stage C: education and activity.
    pub attributes: Option<(&'a PersonAttributes, &'a Value)>,
    /// Stage D: jobs.
    pub jobs: Option<(&'a PersonJobs, &'a Value)>,
    /// Stage E: locality size and tenure.
    pub housing: Option<(&'a HouseholdHousing, &'a Value)>,
    /// Stage F: wages.
    pub wages: Option<(&'a PersonWages, &'a Value)>,
    /// Stage G: pensions.
    pub pensions: Option<(&'a PersonPensions, &'a Value)>,
    /// Stage H: household wealth.
    pub wealth: Option<(&'a HouseholdWealth, &'a Value)>,
    /// Stage I: declared and undeclared income.
    pub incomes: Option<(&'a OtherIncomes, &'a Value)>,
    /// Stage J: industry and employer.
    pub employers: Option<(&'a PersonEmployers, &'a Value)>,
    /// Stage K: kinds of inactivity and enrolment.
    pub inactivity: Option<(&'a PersonInactivityEnrolment, &'a Value)>,
    /// The region of each county (stage C's margins): with it, the
    /// households table has `hh_region`.
    pub region_of_county: Option<&'a [u8]>,
}

pub(crate) fn stage_err(file: &str, what: &str) -> SaveError {
    SaveError::Parse(format!("{file} margin file: {what}"))
}

/// A nested table of counts as one row-major vector. Ragged tables and tables
/// of another shape are rejected.
pub(crate) fn flat_table(
    v: &Value,
    file: &str,
    name: &str,
    dims: &[usize],
) -> Result<Vec<u64>, SaveError> {
    fn flatten(v: &Value, dims: &[usize], out: &mut Vec<u64>) -> bool {
        match dims.split_first() {
            None => v.as_u64().map(|n| out.push(n)).is_some(),
            Some((&n, rest)) => v
                .as_array()
                .is_some_and(|a| a.len() == n && a.iter().all(|x| flatten(x, rest, out))),
        }
    }
    let mut out = Vec::with_capacity(dims.iter().product());
    if flatten(&v[name], dims, &mut out) {
        Ok(out)
    } else {
        Err(stage_err(
            file,
            &format!("{name} is not a table of counts of shape {dims:?}"),
        ))
    }
}

/// A list of codes or labels.
pub(crate) fn names(v: &Value, file: &str, name: &str) -> Result<Vec<String>, SaveError> {
    serde_json::from_value(v[name].clone())
        .map_err(|_| stage_err(file, &format!("{name} is not a list of names")))
}

/// A list of labels that the stage fixes, in its order.
pub(crate) fn fixed_names(
    v: &Value,
    file: &str,
    name: &str,
    want: &[&str],
) -> Result<(), SaveError> {
    if names(v, file, name)? == want {
        Ok(())
    } else {
        Err(stage_err(file, &format!("{name} must be {want:?}")))
    }
}

/// A list of labels of which the stage fixes only the number.
fn counted_names(v: &Value, file: &str, name: &str, want: usize) -> Result<(), SaveError> {
    if names(v, file, name)?.len() == want {
        Ok(())
    } else {
        Err(stage_err(file, &format!("{name} must have {want} entries")))
    }
}

fn age_bands(v: &Value, file: &str) -> Result<Vec<(u16, u16)>, SaveError> {
    let bands: Vec<(u16, Option<u16>)> = serde_json::from_value(v["age_bands"].clone())
        .map_err(|_| stage_err(file, "age_bands is not a list of [from, to]"))?;
    Ok(bands
        .into_iter()
        .map(|(from, to)| (from, to.unwrap_or(OPEN)))
        .collect())
}

pub(crate) fn parse(json: &str, file: &str) -> Result<Value, SaveError> {
    serde_json::from_str(json).map_err(|e| stage_err(file, &e.to_string()))
}

/// Parse the education-and-activity margin file (stage C), written by
/// `python/pipeline/normalise_census_attributes.py`. `counties` are those of
/// the county margin file: the region of each is looked up by its code.
///
/// # Errors
/// `SaveError::Parse` if the JSON is not such a file or a county has no region.
pub fn read_attribute_margins(
    json: &str,
    counties: &[String],
) -> Result<StageFile<AttributeMargins>, SaveError> {
    const FILE: &str = "education-and-activity";
    let v = parse(json, FILE)?;
    fixed_names(&v, FILE, "sexes", &["F", "M"])?;
    fixed_names(
        &v,
        FILE,
        "activities",
        &[
            "child",
            "in_education",
            "employed",
            "unemployed",
            "retired",
            "inactive_other",
        ],
    )?;
    fixed_names(
        &v,
        FILE,
        "statuses",
        &["employed", "unemployed", "inactive"],
    )?;
    counted_names(&v, FILE, "edu_levels", EDU_LEVELS)?;
    let regions = names(&v, FILE, "regions")?;
    let region_of_county = counties
        .iter()
        .map(|county| {
            v["region_of_county"][county]
                .as_str()
                .and_then(|region| regions.iter().position(|r| r == region))
                .and_then(|index| u8::try_from(index).ok())
                .ok_or_else(|| stage_err(FILE, &format!("county {county} has no region")))
        })
        .collect::<Result<Vec<u8>, SaveError>>()?;
    let age_bands = age_bands(&v, FILE)?;
    let years = v["activity"][0][0].as_array().map_or(0, Vec::len);
    let years_of_age =
        u16::try_from(years).map_err(|_| stage_err(FILE, "too many years of age"))?;
    let (n_r, n_b) = (regions.len(), age_bands.len());
    let activity = flat_table(&v, FILE, "activity", &[n_r, 2, years, MARGIN_ACTIVITIES])?;
    let education = flat_table(&v, FILE, "education", &[n_r, 2, n_b, STATUSES, EDU_LEVELS])?;
    Ok(StageFile {
        margins: AttributeMargins {
            regions,
            region_of_county,
            age_bands,
            years_of_age,
            activity,
            education,
        },
        provenance: v["provenance"].clone(),
    })
}

/// Parse the jobs margin file (stage D), written by
/// `python/pipeline/normalise_census_jobs.py`. The regions, and each county's
/// region, are those of the education-and-activity file.
///
/// # Errors
/// `SaveError::Parse` if the JSON is not such a file or its regions differ.
pub fn read_job_margins(
    json: &str,
    attributes: &AttributeMargins,
) -> Result<StageFile<JobMargins>, SaveError> {
    const FILE: &str = "jobs";
    let v = parse(json, FILE)?;
    fixed_names(&v, FILE, "sexes", &["F", "M"])?;
    counted_names(&v, FILE, "kinds", KINDS)?;
    counted_names(&v, FILE, "occupations", OCCUPATIONS)?;
    counted_names(&v, FILE, "industry_groups", INDUSTRY_GROUPS)?;
    counted_names(&v, FILE, "edu_groups", EDU_GROUPS)?;
    if names(&v, FILE, "regions")? != attributes.regions {
        return Err(stage_err(
            FILE,
            "regions differ from the education-and-activity file",
        ));
    }
    let age_bands = age_bands(&v, FILE)?;
    let (n_r, n_b) = (attributes.regions.len(), age_bands.len());
    let by_industry_group = flat_table(
        &v,
        FILE,
        "by_industry_group",
        &[n_r, 2, n_b, KINDS, INDUSTRY_GROUPS],
    )?;
    let by_occupation = flat_table(
        &v,
        FILE,
        "by_occupation",
        &[n_r, 2, n_b, KINDS, OCCUPATIONS],
    )?;
    let groups: Vec<u8> = serde_json::from_value(v["edu_group_of_level"].clone())
        .map_err(|_| stage_err(FILE, "edu_group_of_level is not a list of indices"))?;
    Ok(StageFile {
        margins: JobMargins {
            regions: attributes.regions.clone(),
            region_of_county: attributes.region_of_county.clone(),
            age_bands,
            by_industry_group,
            by_occupation,
            pattern_edu_occupation: flat_table(
                &v,
                FILE,
                "pattern_edu_occupation",
                &[2, EDU_GROUPS, OCCUPATIONS],
            )?,
            pattern_occupation_industry_group: flat_table(
                &v,
                FILE,
                "pattern_occupation_industry_group",
                &[2, OCCUPATIONS, INDUSTRY_GROUPS],
            )?,
            edu_group_of_level: groups
                .try_into()
                .map_err(|_| stage_err(FILE, "edu_group_of_level must have one entry per level"))?,
        },
        provenance: v["provenance"].clone(),
    })
}

/// Parse the housing margin file (stage E), written by
/// `python/pipeline/normalise_census_housing.py`.
///
/// # Errors
/// `SaveError::Parse` if the JSON is not such a file.
pub fn read_housing_margins(json: &str) -> Result<StageFile<HousingMargins>, SaveError> {
    const FILE: &str = "housing";
    let v = parse(json, FILE)?;
    counted_names(&v, FILE, "size_groups", SIZE_GROUPS)?;
    counted_names(&v, FILE, "tenures", TENURES)?;
    let counties = names(&v, FILE, "counties")?;
    let classes = names(&v, FILE, "locality_classes")?.len();
    let age_group_from: Vec<u16> = serde_json::from_value(v["age_group_from"].clone())
        .map_err(|_| stage_err(FILE, "age_group_from is not a list of ages"))?;
    let urban_from_class = v["urban_from_class"]
        .as_u64()
        .and_then(|class| u8::try_from(class).ok())
        .ok_or_else(|| stage_err(FILE, "urban_from_class is not a class"))?;
    let n_c = counties.len();
    Ok(StageFile {
        margins: HousingMargins {
            locality_classes: u8::try_from(classes)
                .map_err(|_| stage_err(FILE, "too many locality classes"))?,
            urban_from_class,
            persons_by_locality: flat_table(
                &v,
                FILE,
                "persons_by_locality",
                &[n_c, age_group_from.len(), classes],
            )?,
            households_by_tenure: flat_table(
                &v,
                FILE,
                "households_by_tenure",
                &[n_c, SIZE_GROUPS, TENURES],
            )?,
            counties,
            age_group_from,
        },
        provenance: v["provenance"].clone(),
    })
}

/// A list of millionths.
fn ppm_list(v: &Value, file: &str, name: &str) -> Result<Vec<u32>, SaveError> {
    serde_json::from_value(v[name].clone())
        .map_err(|_| stage_err(file, &format!("{name} is not a list of millionths")))
}

/// A table of millionths of the given shape, row-major.
fn ppm_table(v: &Value, file: &str, name: &str, dims: &[usize]) -> Result<Vec<u32>, SaveError> {
    flat_table(v, file, name, dims)?
        .into_iter()
        .map(|x| u32::try_from(x).map_err(|_| stage_err(file, &format!("{name} is too large"))))
        .collect()
}

/// The education group of each education level.
fn edu_group_of_level(v: &Value, file: &str) -> Result<[u8; EDU_LEVELS], SaveError> {
    serde_json::from_value(v["edu_group_of_level"].clone()).map_err(|_| {
        stage_err(
            file,
            &format!("edu_group_of_level must list {EDU_LEVELS} education groups"),
        )
    })
}

/// Parse the earnings margin file (stage F), written by
/// `python/pipeline/normalise_earnings.py`.
///
/// # Errors
/// `SaveError::Parse` if the JSON is not such a file.
pub fn read_wage_margins(json: &str) -> Result<StageFile<WageMargins>, SaveError> {
    const FILE: &str = "earnings";
    let v = parse(json, FILE)?;
    fixed_names(&v, FILE, "sexes", &["F", "M"])?;
    counted_names(&v, FILE, "industry_groups", INDUSTRY_GROUPS)?;
    counted_names(&v, FILE, "occupations", OCCUPATIONS)?;
    counted_names(&v, FILE, "edu_groups", EDU_GROUPS)?;
    let age_group_from: Vec<u16> = serde_json::from_value(v["age_group_from"].clone())
        .map_err(|_| stage_err(FILE, "age_group_from is not a list of ages"))?;
    let n_age = age_group_from.len();
    let g = INDUSTRY_GROUPS;
    Ok(StageFile {
        margins: WageMargins {
            wage_bill: flat_table(&v, FILE, "wage_bill", &[g])?,
            minimum_wage: v["minimum_wage"]
                .as_u64()
                .ok_or_else(|| stage_err(FILE, "minimum_wage is not an amount"))?,
            edu_group_of_level: edu_group_of_level(&v, FILE)?,
            rel_sex: ppm_table(&v, FILE, "rel_sex", &[g, 2])?,
            rel_occupation: ppm_table(&v, FILE, "rel_occupation", &[g, 2, OCCUPATIONS])?,
            rel_age: ppm_table(&v, FILE, "rel_age", &[g, 2, n_age])?,
            rel_education: ppm_table(&v, FILE, "rel_education", &[g, 2, EDU_GROUPS])?,
            curve_rank: ppm_list(&v["quantile_curve"], FILE, "rank")?,
            curve_value: ppm_list(&v["quantile_curve"], FILE, "value")?,
            age_group_from,
        },
        provenance: v["provenance"].clone(),
    })
}

/// Parse the pension margin file (stage G), written by
/// `python/pipeline/normalise_pensions.py`.
///
/// # Errors
/// `SaveError::Parse` if the JSON is not such a file.
pub fn read_pension_margins(json: &str) -> Result<StageFile<PensionMargins>, SaveError> {
    const FILE: &str = "pensions";
    let v = parse(json, FILE)?;
    fixed_names(&v, FILE, "sexes", &["F", "M"])?;
    counted_names(&v, FILE, "edu_groups", EDU_GROUPS)?;
    let amount = |name: &str| {
        v[name]
            .as_u64()
            .ok_or_else(|| stage_err(FILE, &format!("{name} is not an amount")))
    };
    let rel_sex = ppm_table(&v, FILE, "rel_sex", &[2])?;
    Ok(StageFile {
        margins: PensionMargins {
            pension_bill: amount("pension_bill")?,
            minimum_pension: amount("minimum_pension")?,
            rel_sex: [rel_sex[0], rel_sex[1]],
            edu_group_of_level: edu_group_of_level(&v, FILE)?,
            rel_education: ppm_table(&v, FILE, "rel_education", &[2, EDU_GROUPS])?,
            curve_rank: ppm_list(&v["quantile_curve"], FILE, "rank")?,
            curve_value: ppm_list(&v["quantile_curve"], FILE, "value")?,
        },
        provenance: v["provenance"].clone(),
    })
}

/// Parse the household wealth margin file (stage H), written by
/// `python/pipeline/normalise_wealth.py`.
///
/// # Errors
/// `SaveError::Parse` if the JSON is not such a file.
pub fn read_wealth_margins(json: &str) -> Result<StageFile<WealthMargins>, SaveError> {
    const FILE: &str = "wealth";
    let v = parse(json, FILE)?;
    let number = |v: &Value, name: &str| {
        v[name]
            .as_u64()
            .ok_or_else(|| stage_err(FILE, &format!("{name} is not a non-negative integer")))
    };
    let small = |v: &Value, name: &str| {
        number(v, name).and_then(|x| {
            u32::try_from(x).map_err(|_| stage_err(FILE, &format!("{name} is too large")))
        })
    };
    let curve = |name: &str| -> Result<Curve, SaveError> {
        Ok(Curve {
            rank: ppm_list(&v[name], FILE, "rank")?,
            value: ppm_list(&v[name], FILE, "value")?,
        })
    };
    let rule = |name: &str| -> Result<BorrowerRule, SaveError> {
        let r = &v[name];
        let age = |field: &str| {
            number(r, field).and_then(|x| {
                u16::try_from(x)
                    .map_err(|_| stage_err(FILE, &format!("{name}.{field} is not an age")))
            })
        };
        Ok(BorrowerRule {
            share_ppm: small(r, "share")?,
            age_from: age("age_from")?,
            age_to: age("age_to")?,
            income_link_ppm: small(r, "income_link")?,
            urban_bonus_ppm: small(r, "urban_bonus")?,
        })
    };
    let totals = &v["totals"];
    Ok(StageFile {
        margins: WealthMargins {
            cash: number(totals, "cash")?,
            deposits: number(totals, "deposits")?,
            housing_loans: number(totals, "housing_loans")?,
            consumer_loans: number(totals, "consumer_loans")?,
            dwellings: number(totals, "dwellings")?,
            rel_locality: ppm_list(&v, FILE, "rel_locality")?,
            liquid_curve: curve("liquid_curve")?,
            dwelling_curve: curve("dwelling_curve")?,
            loan_curve: curve("loan_curve")?,
            mortgage: rule("mortgage")?,
            consumer: rule("consumer")?,
            liquid_income_link_ppm: small(&v["income_link"], "liquid")?,
            dwelling_income_link_ppm: small(&v["income_link"], "dwelling")?,
        },
        provenance: v["provenance"].clone(),
    })
}

/// Parse the income margin file (stage I), written by
/// `python/pipeline/normalise_incomes.py`.
///
/// # Errors
/// `SaveError::Parse` if the JSON is not such a file.
pub fn read_income_margins(json: &str) -> Result<StageFile<IncomeMargins>, SaveError> {
    const FILE: &str = "incomes";
    let v = parse(json, FILE)?;
    counted_names(&v, FILE, "industry_groups", INDUSTRY_GROUPS)?;
    let number = |v: &Value, name: &str| {
        v[name]
            .as_u64()
            .ok_or_else(|| stage_err(FILE, &format!("{name} is not a non-negative integer")))
    };
    let small = |v: &Value, name: &str| {
        number(v, name).and_then(|x| {
            u32::try_from(x).map_err(|_| stage_err(FILE, &format!("{name} is too large")))
        })
    };
    let prop = &v["informal_propensity"];
    let rel_kind = ppm_table(&v, FILE, "rel_kind", &[3])?;
    Ok(StageFile {
        margins: IncomeMargins {
            farm_income: number(&v, "farm_income")?,
            farm_in_kind: number(&v, "farm_in_kind")?,
            self_employment_income: number(&v, "self_employment_income")?,
            informal_income: number(&v, "informal_income")?,
            minimum_wage: number(&v, "minimum_wage")?,
            quantile_curve: Curve {
                rank: ppm_list(&v["quantile_curve"], FILE, "rank")?,
                value: ppm_list(&v["quantile_curve"], FILE, "value")?,
            },
            undeclared_wage_share: ppm_table(
                &v,
                FILE,
                "undeclared_wage_share",
                &[INDUSTRY_GROUPS],
            )?,
            fully_informal_share: small(&v, "fully_informal_share")?,
            envelope_declared_share: small(&v, "envelope_declared_share")?,
            low_wage_link: small(&v, "low_wage_link")?,
            rel_kind: [rel_kind[0], rel_kind[1], rel_kind[2]],
            self_employment_education_link: small(&v, "self_employment_education_link")?,
            self_employment_declared: small(&v, "self_employment_declared")?,
            farm_declared: small(&v, "farm_declared")?,
            subsistence_household_share: small(&v, "subsistence_household_share")?,
            subsistence_bonus_farm_worker: small(&v["subsistence_bonus"], "farm_worker")?,
            subsistence_bonus_retired: small(&v["subsistence_bonus"], "retired")?,
            subsistence_dispersion: small(&v, "subsistence_dispersion")?,
            informal_recipient_share: small(&v, "informal_recipient_share")?,
            informal_propensity: InformalPropensity {
                unemployed: small(prop, "unemployed")?,
                inactive_other: small(prop, "inactive_other")?,
                retired: small(prop, "retired")?,
                student: small(prop, "student")?,
                employer: small(prop, "employer")?,
                own_account: small(prop, "own_account")?,
                family_worker: small(prop, "family_worker")?,
                employee_by_industry_group: ppm_table(
                    prop,
                    FILE,
                    "employee_by_industry_group",
                    &[INDUSTRY_GROUPS],
                )?,
            },
        },
        provenance: v["provenance"].clone(),
    })
}

/// Parse the firm-unit file (stage J), written by
/// `python/pipeline/normalise_firms.py`: the catalogue industries with their
/// industry group, and the units with what the stage reads of them.
///
/// # Errors
/// `SaveError::Parse` if the JSON is not such a file or a unit's industry is
/// not listed.
pub fn read_firm_units(json: &str) -> Result<StageFile<FirmUnits>, SaveError> {
    const FILE: &str = "firm units";
    let v = parse(json, FILE)?;
    fixed_names(&v, FILE, "sizes", &["micro", "small", "large", "public"])?;
    let list = |name: &str| {
        v[name]
            .as_array()
            .ok_or_else(|| stage_err(FILE, &format!("{name} is not a list")))
    };
    let mut ids = Vec::new();
    let mut industry_group = Vec::new();
    for ind in list("industries")? {
        let id = ind["id"]
            .as_str()
            .ok_or_else(|| stage_err(FILE, "an industry has no id"))?;
        let g = ind["industry_group"]
            .as_u64()
            .and_then(|g| u8::try_from(g).ok())
            .ok_or_else(|| stage_err(FILE, &format!("industry {id} has no industry group")))?;
        ids.push(id);
        industry_group.push(g);
    }
    let mut units = Vec::new();
    for u in list("units")? {
        let id = u["industry"].as_str().unwrap_or_default();
        let industry = ids
            .iter()
            .position(|&i| i == id)
            .and_then(|k| u8::try_from(k).ok())
            .ok_or_else(|| stage_err(FILE, &format!("industry {id} of a unit is not listed")))?;
        let size = match u["size"].as_str() {
            Some("micro") => UnitSize::Micro,
            Some("small") => UnitSize::Small,
            Some("large") => UnitSize::Large,
            Some("public") => UnitSize::Public,
            _ => return Err(stage_err(FILE, "a unit has an unknown size")),
        };
        let number = |name: &str| {
            u[name]
                .as_u64()
                .ok_or_else(|| stage_err(FILE, &format!("{name} of a unit is not a count")))
        };
        units.push(FirmUnit {
            industry,
            size,
            domestic: u["kind"] == "cohort" && u["controller"] == "domestic",
            persons_employed: number("persons_employed")?,
            wages: number("wages")?,
        });
    }
    Ok(StageFile {
        margins: FirmUnits {
            industry_group,
            units,
        },
        provenance: v["provenance"].clone(),
    })
}

/// The columns of a generated population that the simulation starts from.
#[must_use]
pub fn seed_population(pop: &Population) -> SeedPopulation<'_> {
    SeedPopulation {
        hh_weight: &pop.households.hh_weight,
        hh_county: &pop.households.hh_county,
        household_id: &pop.persons.household_id,
        age_months: &pop.persons.age,
    }
}

/// A column of small codes.
fn codes(values: impl Iterator<Item = u8>) -> ArrayRef {
    Arc::new(UInt8Array::from_iter_values(values))
}

/// The households table as Arrow IPC bytes, with the columns of stages E, H
/// and I if they were run.
///
/// # Errors
/// On Arrow encoding failures.
pub fn households_table(pop: &Population, stages: &Stages<'_>) -> Result<Vec<u8>, SaveError> {
    let hh = &pop.households;
    let mut columns = vec![
        (
            "hh_weight",
            DataType::UInt32,
            Arc::new(UInt32Array::from(hh.hh_weight.clone())) as ArrayRef,
        ),
        (
            "hh_county",
            DataType::UInt8,
            Arc::new(UInt8Array::from(hh.hh_county.clone())),
        ),
        (
            "hh_collective",
            DataType::Boolean,
            Arc::new(BooleanArray::from(hh.hh_collective.clone())),
        ),
        // `hh_key`: the stable identity, the start's index (spec
        // society/households-lifecycle).
        (
            "hh_key",
            DataType::UInt64,
            Arc::new(UInt64Array::from_iter_values(0..hh.hh_weight.len() as u64)),
        ),
    ];
    if let Some(regions) = stages.region_of_county {
        columns.push((
            "hh_region",
            DataType::UInt8,
            codes(
                hh.hh_county
                    .iter()
                    .map(|&c| regions.get(usize::from(c)).copied().unwrap_or(u8::MAX)),
            ),
        ));
    }
    if let Some((housing, _)) = stages.housing {
        columns.extend([
            (
                "hh_locality_size",
                DataType::UInt8,
                codes(housing.hh_locality_size.iter().copied()),
            ),
            (
                "hh_urban",
                DataType::Boolean,
                Arc::new(BooleanArray::from(housing.hh_urban.clone())) as ArrayRef,
            ),
            (
                "hh_tenure",
                DataType::UInt8,
                codes(housing.hh_tenure.iter().copied()),
            ),
        ]);
    }
    if let Some((wealth, _)) = stages.wealth {
        for (name, column) in [
            ("hh_cash", &wealth.hh_cash),
            ("hh_deposits", &wealth.hh_deposits),
            ("hh_dwelling_value", &wealth.hh_dwelling_value),
            ("hh_mortgage", &wealth.hh_mortgage),
            ("hh_consumer_debt", &wealth.hh_consumer_debt),
        ] {
            columns.push((
                name,
                DataType::UInt64,
                Arc::new(UInt64Array::from(column.clone())),
            ));
        }
    }
    if let Some((incomes, _)) = stages.incomes {
        columns.push((
            "hh_subsistence_income",
            DataType::UInt64,
            Arc::new(UInt64Array::from(incomes.hh_subsistence_income.clone())),
        ));
    }
    ipc_bytes(&batch(columns)?)
}

/// The persons table as Arrow IPC bytes, with the columns of stages C, D, F,
/// G, I and J if they were run.
///
/// # Errors
/// On Arrow encoding failures.
pub fn persons_table(pop: &Population, stages: &Stages<'_>) -> Result<Vec<u8>, SaveError> {
    let p = &pop.persons;
    let mut columns = vec![
        (
            "household_id",
            DataType::UInt32,
            Arc::new(UInt32Array::from(p.household_id.clone())) as ArrayRef,
        ),
        (
            "age",
            DataType::UInt16,
            Arc::new(UInt16Array::from(p.age.clone())),
        ),
        (
            "sex",
            DataType::UInt8,
            codes(p.sex.iter().map(|&s| s as u8)),
        ),
        (
            "role",
            DataType::UInt8,
            codes(p.role.iter().map(|&r| r as u8)),
        ),
        // `person_key`: the stable identity, the start's index (spec
        // economy/demographics); linked emigrants continue the count.
        (
            "person_key",
            DataType::UInt64,
            Arc::new(UInt64Array::from_iter_values(
                0..p.household_id.len() as u64,
            )),
        ),
    ];
    if let Some((attrs, _)) = stages.attributes {
        columns.extend([
            (
                "edu_level",
                DataType::UInt8,
                codes(attrs.edu_level.iter().map(|&e| e as u8)),
            ),
            (
                "activity",
                DataType::UInt8,
                codes(attrs.activity.iter().map(|&a| a as u8)),
            ),
        ]);
    }
    if let Some((jobs, _)) = stages.jobs {
        columns.extend([
            (
                "employment_status",
                DataType::UInt8,
                codes(jobs.employment_status.iter().copied()),
            ),
            (
                "occupation",
                DataType::UInt8,
                codes(jobs.occupation.iter().copied()),
            ),
            (
                "industry_group",
                DataType::UInt8,
                codes(jobs.industry_group.iter().copied()),
            ),
        ]);
    }
    if let Some((wages, _)) = stages.wages {
        columns.push((
            "wage",
            DataType::UInt64,
            Arc::new(UInt64Array::from(wages.wage.clone())),
        ));
    }
    if let Some((pensions, _)) = stages.pensions {
        columns.push((
            "pension",
            DataType::UInt64,
            Arc::new(UInt64Array::from(pensions.pension.clone())),
        ));
    }
    if let Some((incomes, _)) = stages.incomes {
        for (name, column) in [
            ("wage_declared", &incomes.wage_declared),
            ("self_employment_income", &incomes.self_employment_income),
            (
                "self_employment_declared",
                &incomes.self_employment_declared,
            ),
            ("informal_income", &incomes.informal_income),
        ] {
            columns.push((
                name,
                DataType::UInt64,
                Arc::new(UInt64Array::from(column.clone())),
            ));
        }
    }
    if let Some((employers, _)) = stages.employers {
        columns.extend([
            (
                "industry",
                DataType::UInt8,
                codes(employers.industry.iter().copied()),
            ),
            (
                "employer",
                DataType::UInt16,
                Arc::new(UInt16Array::from(employers.employer.clone())),
            ),
        ]);
    }
    if let Some((k, _)) = stages.inactivity {
        columns.extend([
            (
                "inactive_kind",
                DataType::UInt8,
                codes(k.inactive_kind.iter().map(|&x| x as u8)),
            ),
            (
                "enrolled",
                DataType::UInt8,
                codes(k.enrolled.iter().map(|&x| x as u8)),
            ),
        ]);
    }
    ipc_bytes(&batch(columns)?)
}

/// The fit report: what was generated, from what, and how well it fits. Each
/// stage that was run adds its hash and the provenance of its margin file.
#[must_use]
pub fn fit_report(
    file: &MarginFile,
    params: &GenParams,
    pop: &Population,
    stages: &Stages<'_>,
) -> Value {
    let hh = &pop.households;
    let weight = |h: usize| u64::from(hh.hh_weight[h]);
    let weighted_households: u64 = (0..hh.hh_weight.len())
        .filter(|&h| !hh.hh_collective[h])
        .map(weight)
        .sum();
    let weighted_persons: u64 = pop
        .persons
        .household_id
        .iter()
        .map(|&h| weight(h as usize))
        .sum();
    let mut report = json!({
        "sample_scale": params.sample_scale,
        "rng_seed": params.rng_seed,
        "max_household_size": params.max_household_size,
        "raking_tolerance_ppm": params.raking_tolerance_ppm,
        "max_sweeps": params.max_sweeps,
        "min_cell_records": params.min_cell_records,
        "households": hh.hh_weight.len(),
        "persons": pop.persons.household_id.len(),
        "weighted_households": weighted_households,
        "weighted_persons": weighted_persons,
        "state_hash": format!("{:016x}", pop.state_hash()),
        "report": {
            "sweeps": pop.report.sweeps,
            "converged": pop.report.converged,
            "max_error_ppm": pop.report.max_error_ppm,
            "unfitted_cells": pop.report.unfitted_cells,
        },
        "counties": file.margins.counties,
        "provenance": file.provenance,
    });
    let ran = [
        (
            "attributes",
            stages.attributes.map(|(a, p)| (a.state_hash(), p)),
        ),
        ("jobs", stages.jobs.map(|(j, p)| (j.state_hash(), p))),
        ("housing", stages.housing.map(|(h, p)| (h.state_hash(), p))),
        ("wages", stages.wages.map(|(w, p)| (w.state_hash(), p))),
        (
            "pensions",
            stages.pensions.map(|(g, p)| (g.state_hash(), p)),
        ),
        ("wealth", stages.wealth.map(|(w, p)| (w.state_hash(), p))),
        ("incomes", stages.incomes.map(|(i, p)| (i.state_hash(), p))),
        (
            "employers",
            stages.employers.map(|(e, p)| (e.state_hash(), p)),
        ),
        (
            "inactivity_enrolment",
            stages.inactivity.map(|(k, p)| (k.state_hash(), p)),
        ),
    ];
    for (stage, result) in ran {
        if let Some((hash, provenance)) = result {
            report[format!("{stage}_hash")] = json!(format!("{hash:016x}"));
            report[format!("{stage}_provenance")] = provenance.clone();
        }
    }
    report
}

/// Write `households.arrow`, `persons.arrow` and `fit_report.json` into `dir`
/// (created if missing), with the columns of the stages that were run.
///
/// # Errors
/// On encoding or file-system failures.
pub fn write_population(
    dir: &Path,
    file: &MarginFile,
    params: &GenParams,
    pop: &Population,
    stages: &Stages<'_>,
) -> Result<(), SaveError> {
    let report = serde_json::to_vec_pretty(&fit_report(file, params, pop, stages))
        .map_err(|e| SaveError::Parse(e.to_string()))?;
    let households = households_table(pop, stages)?;
    let persons = persons_table(pop, stages)?;
    std::fs::create_dir_all(dir).map_err(io_err)?;
    std::fs::write(dir.join("households.arrow"), households).map_err(io_err)?;
    std::fs::write(dir.join("persons.arrow"), persons).map_err(io_err)?;
    std::fs::write(dir.join("fit_report.json"), report).map_err(io_err)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::read_batch;
    use arrow_array::Array;

    /// Two counties, three age bands, two size classes.
    const TINY: &str = r#"{
        "counties": ["A", "B"], "sexes": ["F", "M"],
        "age_bands": [[0, 20], [20, 65], [65, null]],
        "size_classes": [[1, 1], [2, null]],
        "persons_private": [[[30, 90, 30], [30, 90, 30]], [[20, 60, 20], [20, 60, 20]]],
        "persons_collective": [[[0, 4, 6], [0, 5, 5]], [[0, 0, 0], [0, 0, 0]]],
        "households": [[60, 100], [40, 60]],
        "provenance": {"note": "test"}
    }"#;

    /// The scale world on the Romanian starting population (Census 2021
    /// fixture, 1:1000, seed 42), 12 ticks. A change here is a re-golden
    /// event (DETERMINISM.md): explain it in CHANGELOG-sim.md.
    #[test]
    fn golden_romania_12_ticks_at_1_in_1000() {
        use econ_core::scale_spike::ScaleWorld;
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../python/pipeline/fixtures/census2021_margins_ro.json"
        );
        let file = read_margins(&std::fs::read_to_string(path).unwrap()).unwrap();
        let pop = econ_popgen::generate(&file.margins, &GenParams::new(1000, 42)).unwrap();
        let mut w = ScaleWorld::from_population(1000, 42, &seed_population(&pop)).unwrap();
        // Every resident of the census is in the world, at any scale.
        assert_eq!(w.real_persons(), 19_053_815);
        assert_eq!(w.n_households(), pop.households.hh_weight.len());
        let mut last = None;
        for _ in 0..12 {
            let r = w.step();
            assert!(r.ledger_ok && r.clearing_ok);
            last = Some(r);
        }
        let r = last.unwrap();
        let got = (w.state_hash(), r.employed, r.unemployed, r.vat.get());
        assert_eq!(
            got, GOLDEN_ROMANIA_12,
            "Romanian scale-world golden changed: {got:?}"
        );
    }

    /// (state hash, employed, unemployed, VAT in tick 12).
    const GOLDEN_ROMANIA_12: (u64, u64, u64, i64) =
        (0xdf92_e313_fa51_cb01, 10_577_737, 641_162, 881_083_907_188);

    fn fixture(name: &str) -> String {
        let dir = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../python/pipeline/fixtures"
        );
        std::fs::read_to_string(format!("{dir}/{name}")).unwrap()
    }

    /// The hash of 1:1000 in a golden file of the stages (`tests/golden`).
    fn golden_1_in_1000(file: &str) -> String {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests/golden");
        let text = std::fs::read_to_string(format!("{path}/{file}")).unwrap();
        let line = text.lines().find(|l| l.starts_with("1000 ")).unwrap();
        line.split_whitespace().nth(1).unwrap().to_string()
    }

    /// The stage margin fixtures, read by this crate, give the golden hashes
    /// of the stages; their columns and hashes reach the tables and the report.
    #[test]
    fn stage_files_give_the_golden_columns() -> Result<(), SaveError> {
        use econ_popgen::{
            AttrParams, EmployerParams, HousingParams, IncomeParams, JobParams, PensionParams,
            WageParams, WealthParams, assign_attributes, assign_employers, assign_housing,
            assign_jobs, assign_other_incomes, assign_pensions, assign_wages, assign_wealth,
        };
        let file = read_margins(&fixture("census2021_margins_ro.json")).unwrap();
        let ea = read_attribute_margins(
            &fixture("census2021_edu_activity_ro.json"),
            &file.margins.counties,
        )
        .unwrap();
        let job_file = read_job_margins(&fixture("census2021_jobs_ro.json"), &ea.margins).unwrap();
        let housing_file = read_housing_margins(&fixture("census2021_housing_ro.json")).unwrap();
        let wage_file = read_wage_margins(&fixture("earnings2021_ro.json")).unwrap();
        let pension_file = read_pension_margins(&fixture("pensions2021_ro.json")).unwrap();
        let wealth_file = read_wealth_margins(&fixture("household_wealth2021_ro.json")).unwrap();
        let income_file = read_income_margins(&fixture("other_incomes2021_ro.json")).unwrap();
        let firm_file = read_firm_units(&fixture("firm_units2021_ro.json")).unwrap();
        assert_eq!(ea.margins.years_of_age, 101);
        assert_eq!(housing_file.margins.counties, file.margins.counties);

        let params = GenParams::new(1000, 42);
        let pop = econ_popgen::generate(&file.margins, &params).unwrap();
        let attrs = assign_attributes(&pop, &ea.margins, &AttrParams::new(42)).unwrap();
        let jobs = assign_jobs(&pop, &attrs, &job_file.margins, &JobParams::new(42)).unwrap();
        let housing = assign_housing(&pop, &housing_file.margins, &HousingParams::new(42)).unwrap();
        let wages = assign_wages(
            &pop,
            &attrs,
            &jobs,
            &wage_file.margins,
            &WageParams::new(42),
        )
        .unwrap();
        let pensions =
            assign_pensions(&pop, &attrs, &pension_file.margins, &PensionParams::new(42)).unwrap();
        let wealth = assign_wealth(
            &pop,
            &housing,
            Some(&wages),
            Some(&pensions),
            &wealth_file.margins,
            &WealthParams::new(42),
        )
        .unwrap();
        let incomes = assign_other_incomes(
            &pop,
            &attrs,
            &jobs,
            &housing,
            &wages,
            &income_file.margins,
            &IncomeParams::new(42),
        )
        .unwrap();
        let employers = assign_employers(
            &pop,
            &jobs,
            Some(&wages),
            &firm_file.margins,
            &EmployerParams::new(42),
        )
        .unwrap();
        let stages = Stages {
            attributes: Some((&attrs, &ea.provenance)),
            jobs: Some((&jobs, &job_file.provenance)),
            housing: Some((&housing, &housing_file.provenance)),
            wages: Some((&wages, &wage_file.provenance)),
            pensions: Some((&pensions, &pension_file.provenance)),
            wealth: Some((&wealth, &wealth_file.provenance)),
            incomes: Some((&incomes, &income_file.provenance)),
            employers: Some((&employers, &firm_file.provenance)),
            inactivity: None,
            region_of_county: None,
        };

        let r = fit_report(&file, &params, &pop, &stages);
        for (key, golden) in [
            ("attributes_hash", "popgen_ro_census2021_attributes.hashes"),
            ("jobs_hash", "popgen_ro_census2021_jobs.hashes"),
            ("housing_hash", "popgen_ro_census2021_housing.hashes"),
            ("wages_hash", "popgen_ro_census2021_wages.hashes"),
            ("pensions_hash", "popgen_ro_census2021_pensions.hashes"),
            ("wealth_hash", "popgen_ro_census2021_wealth.hashes"),
            ("incomes_hash", "popgen_ro_census2021_incomes.hashes"),
            ("employers_hash", "popgen_ro_census2021_employers.hashes"),
        ] {
            assert_eq!(r[key], golden_1_in_1000(golden), "{key}");
        }
        assert!(r["jobs_provenance"]["sources"].is_array());

        let h = read_batch(households_table(&pop, &stages).unwrap()).unwrap();
        let p = read_batch(persons_table(&pop, &stages).unwrap()).unwrap();
        assert_eq!((h.num_columns(), p.num_columns()), (13, 18));
        assert_eq!(col!(p, "employer", UInt16Array), employers.employer);
        assert_eq!(
            col!(p, "informal_income", UInt64Array),
            incomes.informal_income
        );
        assert_eq!(col!(h, "hh_mortgage", UInt64Array), wealth.hh_mortgage);
        assert_eq!(col!(p, "wage", UInt64Array), wages.wage);
        assert_eq!(col!(p, "pension", UInt64Array), pensions.pension);
        assert_eq!(
            col!(h, "hh_locality_size", UInt8Array),
            housing.hh_locality_size
        );
        assert_eq!(col!(h, "hh_tenure", UInt8Array), housing.hh_tenure);
        let urban = h.column_by_name("hh_urban").unwrap();
        let urban = urban.as_any().downcast_ref::<BooleanArray>().unwrap();
        assert!((0..urban.len()).all(|i| urban.value(i) == housing.hh_urban[i]));
        let level: Vec<u8> = attrs.edu_level.iter().map(|&e| e as u8).collect();
        let activity: Vec<u8> = attrs.activity.iter().map(|&a| a as u8).collect();
        assert_eq!(col!(p, "edu_level", UInt8Array), level);
        assert_eq!(col!(p, "activity", UInt8Array), activity);
        assert_eq!(
            col!(p, "employment_status", UInt8Array),
            jobs.employment_status
        );
        assert_eq!(col!(p, "occupation", UInt8Array), jobs.occupation);
        assert_eq!(col!(p, "industry_group", UInt8Array), jobs.industry_group);
        Ok(())
    }

    #[test]
    fn bad_stage_files_are_rejected() {
        let counties = vec!["RO111".to_string()];
        let ea = fixture("census2021_edu_activity_ro.json");
        // A county the file does not know.
        let unknown = vec!["XX000".to_string()];
        assert!(matches!(
            read_attribute_margins(&ea, &unknown),
            Err(SaveError::Parse(_))
        ));
        let mut swapped: Value = serde_json::from_str(&ea).unwrap();
        swapped["sexes"] = json!(["M", "F"]);
        assert!(matches!(
            read_attribute_margins(&swapped.to_string(), &counties),
            Err(SaveError::Parse(_))
        ));
        // Jobs for other regions than the education-and-activity file's.
        let mut other = read_attribute_margins(&ea, &counties).unwrap().margins;
        other.regions.pop();
        assert!(matches!(
            read_job_margins(&fixture("census2021_jobs_ro.json"), &other),
            Err(SaveError::Parse(_))
        ));
        // A ragged table.
        let housing = fixture("census2021_housing_ro.json");
        let mut v: Value = serde_json::from_str(&housing).unwrap();
        v["households_by_tenure"][0][1]
            .as_array_mut()
            .unwrap()
            .pop();
        assert!(matches!(
            read_housing_margins(&v.to_string()),
            Err(SaveError::Parse(_))
        ));
        assert!(matches!(
            read_housing_margins("{}"),
            Err(SaveError::Parse(_))
        ));
    }

    #[test]
    fn margin_file_is_parsed_row_major() {
        let f = read_margins(TINY).unwrap();
        let m = &f.margins;
        assert_eq!(m.counties, ["A", "B"]);
        assert_eq!(m.age_bands, [(0, 20), (20, 65), (65, OPEN)]);
        assert_eq!(m.size_classes, [(1, 1), (2, OPEN)]);
        assert_eq!(m.persons_private[..6], [30, 90, 30, 30, 90, 30]);
        assert_eq!(m.persons_collective[..6], [0, 4, 6, 0, 5, 5]);
        assert_eq!(m.households, [60, 100, 40, 60]);
        assert_eq!(f.provenance["note"], "test");
    }

    #[test]
    fn bad_margin_files_are_rejected() {
        let ragged = TINY.replace(
            "[[20, 60, 20], [20, 60, 20]]",
            "[[20, 60], [20, 60, 20, 20]]",
        );
        assert!(matches!(read_margins(&ragged), Err(SaveError::Parse(_))));
        let sexes = TINY.replace(r#"["F", "M"]"#, r#"["M", "F"]"#);
        assert!(matches!(read_margins(&sexes), Err(SaveError::Parse(_))));
        assert!(matches!(read_margins("{}"), Err(SaveError::Parse(_))));
    }

    #[test]
    fn tables_round_trip_and_the_report_is_exact() -> Result<(), SaveError> {
        let file = read_margins(TINY).unwrap();
        let params = GenParams::new(10, 3);
        let pop = econ_popgen::generate(&file.margins, &params).unwrap();

        let none = Stages::default();
        let h = read_batch(households_table(&pop, &none).unwrap()).unwrap();
        let p = read_batch(persons_table(&pop, &none).unwrap()).unwrap();
        assert_eq!((h.num_columns(), p.num_columns()), (4, 5));
        assert_eq!(col!(h, "hh_weight", UInt32Array), pop.households.hh_weight);
        assert_eq!(col!(h, "hh_county", UInt8Array), pop.households.hh_county);
        let collective = h.column_by_name("hh_collective").unwrap();
        let collective = collective.as_any().downcast_ref::<BooleanArray>().unwrap();
        assert_eq!(
            (0..collective.len())
                .map(|i| collective.value(i))
                .collect::<Vec<_>>(),
            pop.households.hh_collective
        );
        assert_eq!(
            col!(p, "household_id", UInt32Array),
            pop.persons.household_id
        );
        assert_eq!(col!(p, "age", UInt16Array), pop.persons.age);
        let sex: Vec<u8> = pop.persons.sex.iter().map(|&s| s as u8).collect();
        let role: Vec<u8> = pop.persons.role.iter().map(|&r| r as u8).collect();
        assert_eq!(col!(p, "sex", UInt8Array), sex);
        assert_eq!(col!(p, "role", UInt8Array), role);

        // The report carries the census totals whatever the scale.
        let r = fit_report(&file, &params, &pop, &none);
        assert!(r.get("attributes_hash").is_none());
        assert_eq!(r["weighted_households"], 260);
        assert_eq!(r["weighted_persons"], 520);
        assert_eq!(r["sample_scale"], 10);
        assert_eq!(r["state_hash"], format!("{:016x}", pop.state_hash()));
        assert_eq!(r["provenance"]["note"], "test");
        Ok(())
    }
}
