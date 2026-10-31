//! The starting data of roadmap M5 (people over time) in and out: the margin
//! files of stages K (kinds of inactivity and enrolment) and L (linked
//! emigrants), the demographic tables (`demography.json`), and the tables of
//! the linked emigrants and the diaspora stock.
//!
//! | File | Content |
//! |---|---|
//! | `linked_emigrants.arrow` | `household_id` (u32), `person_key` (u64), `age` (u16, months), `sex`, `role`, `edu_level` (u8), `months_abroad` (u16) |
//! | `diaspora_stock.arrow` | `region` (u8, stage C's region index), `sex` (u8), `birth_year` (u16), `tier` (u8: high 0, mid 1, low 2), `persons` (u64, real persons) |
//! | `demography.json` | the pipeline's `demography2019_ro.json`, as it is ([`read_demography`]) |

use std::sync::Arc;

use arrow_array::{ArrayRef, UInt8Array, UInt16Array, UInt32Array, UInt64Array};
use arrow_schema::DataType;
use econ_mech_demography::{
    Demography, FertilityTable, MORTALITY_AGES, MortalityTable, PPB, Ppb, RetirementSchedule,
};
use econ_popgen::{
    EDU_LEVELS, ENROLMENT_LEVELS, INACTIVE_KINDS, InactivityEnrolmentMargins,
    LinkedEmigrantMargins, LinkedEmigrants, OPEN,
};
use econ_types::Date;
use serde_json::Value;

use crate::popgen::{StageFile, fixed_names, flat_table, parse, stage_err};
use crate::{SaveError, batch, ipc_bytes};

/// `[from, to)` pairs in years, `null` the open end of the last.
fn ranges(v: &Value, file: &str, name: &str) -> Result<Vec<(u16, u16)>, SaveError> {
    let pairs: Vec<(u16, Option<u16>)> = serde_json::from_value(v[name].clone())
        .map_err(|_| stage_err(file, &format!("{name} is not a list of [from, to]")))?;
    Ok(pairs
        .into_iter()
        .map(|(from, to)| (from, to.unwrap_or(OPEN)))
        .collect())
}

/// Parse the inactivity-and-enrolment margin file (stage K), written by
/// `python/pipeline/normalise_inactivity_enrolment.py`.
///
/// # Errors
/// `SaveError::Parse` if the JSON is not such a file.
pub fn read_inactivity_margins(
    json: &str,
) -> Result<StageFile<InactivityEnrolmentMargins>, SaveError> {
    const FILE: &str = "inactivity-and-enrolment";
    let v = parse(json, FILE)?;
    fixed_names(&v, FILE, "sexes", &["F", "M"])?;
    fixed_names(
        &v,
        FILE,
        "inactive_kinds",
        &["homemaker", "disabled", "other"],
    )?;
    fixed_names(
        &v,
        FILE,
        "enrolment_levels",
        &[
            "pre_school",
            "primary",
            "lower_secondary",
            "upper_secondary",
            "post_secondary",
            "tertiary",
            "master_doctorate",
        ],
    )?;
    let age_bands = ranges(&v, FILE, "age_bands")?;
    let enrolment_ages = ranges(&v, FILE, "enrolment_ages")?;
    let (n_b, n_g) = (age_bands.len(), enrolment_ages.len());
    Ok(StageFile {
        margins: InactivityEnrolmentMargins {
            inactive_kind: flat_table(&v, FILE, "inactive_kind", &[2, n_b, INACTIVE_KINDS])?,
            enrolment: flat_table(&v, FILE, "enrolment", &[2, n_g, ENROLMENT_LEVELS])?,
            population: flat_table(&v, FILE, "population", &[2, n_g])?,
            age_bands,
            enrolment_ages,
        },
        provenance: v["provenance"].clone(),
    })
}

/// Parse the linked-emigrant margin file (stage L), written by
/// `python/pipeline/normalise_linked_emigrants.py`. `regions` are stage C's
/// region codes: the file's region factors are looked up by code, and a
/// region the file does not name has a factor of one.
///
/// # Errors
/// `SaveError::Parse` if the JSON is not such a file or names an unknown region.
pub fn read_linked_emigrant_margins(
    json: &str,
    regions: &[String],
) -> Result<StageFile<LinkedEmigrantMargins>, SaveError> {
    const FILE: &str = "linked-emigrants";
    let v = parse(json, FILE)?;
    fixed_names(&v, FILE, "sexes", &["F", "M"])?;
    let number = |name: &str| {
        v[name]
            .as_u64()
            .ok_or_else(|| stage_err(FILE, &format!("{name} is not a whole number")))
    };
    let small = |name: &str| {
        number(name).and_then(|n| {
            u16::try_from(n).map_err(|_| stage_err(FILE, &format!("{name} is too large")))
        })
    };
    let ppm = |x: &Value, what: &str| {
        x.as_u64()
            .and_then(|n| u32::try_from(n).ok())
            .ok_or_else(|| stage_err(FILE, &format!("{what} is not in millionths")))
    };
    let ages = v["departure_age_weights"]
        .as_array()
        .and_then(|a| a.first())
        .and_then(Value::as_array)
        .map_or(0, Vec::len);
    let departure_years: Vec<(u16, u64)> = serde_json::from_value(v["departure_years"].clone())
        .map_err(|_| stage_err(FILE, "departure_years is not a list of [year, weight]"))?;
    let edu: Vec<u32> = serde_json::from_value(v["edu_shares_ppm"].clone())
        .map_err(|_| stage_err(FILE, "edu_shares_ppm is not a list of millionths"))?;
    let edu_shares_ppm: [u32; EDU_LEVELS] = edu
        .try_into()
        .map_err(|_| stage_err(FILE, "edu_shares_ppm has one share per education level"))?;
    let mut region_factor_ppm = vec![1_000_000; regions.len()];
    for pair in v["region_factor_ppm"]
        .as_array()
        .ok_or_else(|| stage_err(FILE, "region_factor_ppm is not a list"))?
    {
        let code = pair[0]
            .as_str()
            .ok_or_else(|| stage_err(FILE, "a region code"))?;
        let r = regions
            .iter()
            .position(|c| c == code)
            .ok_or_else(|| stage_err(FILE, &format!("region {code} is not one of stage C's")))?;
        region_factor_ppm[r] = ppm(&pair[1], "a region factor")?;
    }
    let head_age_factor = v["head_age_factor"]
        .as_array()
        .ok_or_else(|| stage_err(FILE, "head_age_factor is not a list"))?
        .iter()
        .map(|band| {
            let from = band[0]
                .as_u64()
                .and_then(|n| u16::try_from(n).ok())
                .ok_or_else(|| stage_err(FILE, "a head age band"))?;
            let to = if band[1].is_null() {
                OPEN
            } else {
                band[1]
                    .as_u64()
                    .and_then(|n| u16::try_from(n).ok())
                    .ok_or_else(|| stage_err(FILE, "a head age band"))?
            };
            Ok((from, to, ppm(&band[2], "a head age factor")?))
        })
        .collect::<Result<_, SaveError>>()?;
    Ok(StageFile {
        margins: LinkedEmigrantMargins {
            start_year: small("start_year")?,
            linked_households: number("linked_households")?,
            diaspora_total: number("diaspora_total")?,
            first_departure_age: small("first_departure_age")?,
            departure_age_weights: flat_table(&v, FILE, "departure_age_weights", &[2, ages])?,
            departure_years,
            edu_shares_ppm,
            region_factor_ppm,
            rural_factor_ppm: ppm(&v["rural_factor_ppm"], "rural_factor_ppm")?,
            head_age_factor,
        },
        provenance: v["provenance"].clone(),
    })
}

/// A date `YYYY-MM-DD`.
fn date(s: &str) -> Option<Date> {
    let mut parts = s.split('-');
    let (y, m, d) = (parts.next()?, parts.next()?, parts.next()?);
    if parts.next().is_some() {
        return None;
    }
    Some(Date {
        year: y.parse().ok()?,
        month: m.parse().ok()?,
        day: d.parse().ok()?,
    })
}

/// Parse `county_moves.json` (the pipeline's `county_moves2021_ro.json`):
/// its counties must be `counties`, in that order, and the balances sum to
/// zero.
///
/// # Errors
/// `SaveError::Parse` if the JSON is not such a file.
pub fn read_county_moves(
    json: &str,
    counties: &[String],
) -> Result<econ_popgen::CountyMoves, SaveError> {
    const FILE: &str = "county moves";
    let v = parse(json, FILE)?;
    let names: Vec<&str> = v["counties"]
        .as_array()
        .map(|a| a.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    if names.len() != counties.len() || names.iter().zip(counties).any(|(a, b)| *a != b.as_str()) {
        return Err(stage_err(FILE, "counties differ from the census margins'"));
    }
    let ints = |name: &str| -> Result<Vec<i64>, SaveError> {
        let a = v[name]
            .as_array()
            .filter(|a| a.len() == counties.len())
            .ok_or_else(|| stage_err(FILE, &format!("{name} needs one value per county")))?;
        a.iter()
            .map(|x| {
                x.as_i64()
                    .ok_or_else(|| stage_err(FILE, &format!("{name} holds a non-integer")))
            })
            .collect()
    };
    let net_2021 = ints("net_2021")?;
    if net_2021.iter().sum::<i64>() != 0 {
        return Err(stage_err(FILE, "net_2021 does not sum to zero"));
    }
    let deg = |name: &str| -> Result<Vec<i32>, SaveError> {
        ints(name)?
            .into_iter()
            .map(|x| i32::try_from(x).map_err(|_| stage_err(FILE, &format!("{name} out of range"))))
            .collect()
    };
    Ok(econ_popgen::CountyMoves {
        net_2021,
        seat_lat_milli: deg("seat_lat_milli")?,
        seat_lon_milli: deg("seat_lon_milli")?,
        settlements_2021: v["settlements_2021"]
            .as_u64()
            .ok_or_else(|| stage_err(FILE, "settlements_2021"))?,
        between_county_share_ppm: v["between_county_share_ppm"]
            .as_u64()
            .and_then(|x| u32::try_from(x).ok())
            .filter(|&x| x <= 1_000_000)
            .ok_or_else(|| stage_err(FILE, "between_county_share_ppm"))?,
    })
}

/// Parse `demography.json` (the pipeline's `demography2019_ro.json`): the
/// mortality table (ages 0 to 109 in the file; at 110 the probability is
/// one), the fertility rates, the sex ratio at birth and the dated
/// retirement ages.
///
/// # Errors
/// `SaveError::Parse` if the JSON is not such a file or a table fails
/// [`Demography::check`].
pub fn read_demography(json: &str) -> Result<Demography, SaveError> {
    const FILE: &str = "demography";
    let v = parse(json, FILE)?;
    fixed_names(&v, FILE, "sexes", &["F", "M"])?;
    let m = &v["mortality"];
    let ages = usize::try_from(m["ages"].as_u64().unwrap_or(0)).unwrap_or(0);
    if ages == 0 || ages >= MORTALITY_AGES {
        return Err(stage_err(FILE, "mortality.ages must be 1 to 110"));
    }
    let table = |name: &str| -> Result<[[Ppb; MORTALITY_AGES]; 2], SaveError> {
        let flat = flat_table(m, FILE, name, &[2, ages])?;
        let mut out = [[PPB; MORTALITY_AGES]; 2];
        for (k, &q) in flat.iter().enumerate() {
            out[k / ages][k % ages] = u32::try_from(q)
                .ok()
                .filter(|&q| q <= PPB)
                .ok_or_else(|| stage_err(FILE, &format!("mortality.{name} is not in ppb")))?;
        }
        Ok(out)
    };
    let year = |x: &Value, what: &str| {
        x.as_u64()
            .and_then(|n| u16::try_from(n).ok())
            .ok_or_else(|| stage_err(FILE, &format!("{what} is not a year")))
    };
    let f = &v["fertility"];
    let rates_ppm: Vec<u32> = serde_json::from_value(f["rates_ppm"].clone())
        .map_err(|_| stage_err(FILE, "fertility.rates_ppm is not a list of millionths"))?;
    let mut dated: [Vec<(Date, u16)>; 2] = [Vec::new(), Vec::new()];
    for (sex, name) in ["F", "M"].iter().enumerate() {
        let entries: Vec<(String, u16)> =
            serde_json::from_value(v["retirement_age_schedule"][name].clone()).map_err(|_| {
                stage_err(FILE, "retirement_age_schedule is not [[date, months], ...]")
            })?;
        for (d, months) in entries {
            let d = date(&d).ok_or_else(|| stage_err(FILE, &format!("the date {d}")))?;
            dated[sex].push((d, months));
        }
    }
    let d = Demography {
        mortality: MortalityTable {
            base_year: year(&m["base_year"], "mortality.base_year")?,
            yearly: table("yearly_ppb")?,
            monthly: table("monthly_ppb")?,
        },
        fertility: FertilityTable {
            year: year(&f["year"], "fertility.year")?,
            first_age: year(&f["first_age"], "fertility.first_age")?,
            rates_ppm,
        },
        sex_ratio_at_birth_ppm: v["sex_ratio_at_birth_ppm"]
            .as_u64()
            .and_then(|n| u32::try_from(n).ok())
            .ok_or_else(|| stage_err(FILE, "sex_ratio_at_birth_ppm"))?,
        retirement_age_schedule: RetirementSchedule { dated },
    };
    d.check()
        .map_err(|e| stage_err(FILE, &format!("the tables: {e:?}")))?;
    Ok(d)
}

/// A column of small codes.
fn codes(values: impl Iterator<Item = u8>) -> ArrayRef {
    Arc::new(UInt8Array::from_iter_values(values))
}

/// `linked_emigrants.arrow`: one row per linked emigrant, whose `person_key`s
/// run from `first_person_key` (the residents' count) on.
///
/// # Errors
/// On Arrow encoding failures.
pub fn linked_emigrants_table(
    e: &LinkedEmigrants,
    first_person_key: u64,
) -> Result<Vec<u8>, SaveError> {
    let n = e.household_id.len() as u64;
    ipc_bytes(&batch(vec![
        (
            "household_id",
            DataType::UInt32,
            Arc::new(UInt32Array::from(e.household_id.clone())) as ArrayRef,
        ),
        (
            "person_key",
            DataType::UInt64,
            Arc::new(UInt64Array::from_iter_values(
                first_person_key..first_person_key + n,
            )),
        ),
        (
            "age",
            DataType::UInt16,
            Arc::new(UInt16Array::from(e.age.clone())),
        ),
        (
            "sex",
            DataType::UInt8,
            codes(e.sex.iter().map(|&s| s as u8)),
        ),
        (
            "role",
            DataType::UInt8,
            codes(e.role.iter().map(|&r| r as u8)),
        ),
        (
            "edu_level",
            DataType::UInt8,
            codes(e.edu_level.iter().map(|&x| x as u8)),
        ),
        (
            "months_abroad",
            DataType::UInt16,
            Arc::new(UInt16Array::from(e.months_abroad.clone())),
        ),
    ])?)
}

/// `diaspora_stock.arrow`: one row per cell of `diaspora_stock[k]` with persons.
///
/// # Errors
/// On Arrow encoding failures.
pub fn diaspora_stock_table(e: &LinkedEmigrants) -> Result<Vec<u8>, SaveError> {
    let s = &e.stock;
    ipc_bytes(&batch(vec![
        ("region", DataType::UInt8, codes(s.iter().map(|c| c.region))),
        ("sex", DataType::UInt8, codes(s.iter().map(|c| c.sex as u8))),
        (
            "birth_year",
            DataType::UInt16,
            Arc::new(UInt16Array::from_iter_values(
                s.iter().map(|c| c.birth_year),
            )) as ArrayRef,
        ),
        ("tier", DataType::UInt8, codes(s.iter().map(|c| c.tier))),
        (
            "persons",
            DataType::UInt64,
            Arc::new(UInt64Array::from_iter_values(s.iter().map(|c| c.persons))),
        ),
    ])?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> String {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../python/pipeline/fixtures")
            .join(name);
        std::fs::read_to_string(path).expect("the fixture")
    }

    #[test]
    fn the_demographic_fixture_reads() {
        let d = read_demography(&fixture("demography2019_ro.json")).expect("reads");
        assert_eq!(d.mortality.base_year, 2018);
        // At 110 the monthly probability is one; below it, under one.
        assert_eq!(d.mortality.monthly[0][110], PPB);
        assert!(d.mortality.monthly[1][109] < PPB);
        // Infant mortality 2017-2019: about 5.5 per thousand girls, 6.7 boys.
        assert_eq!(d.mortality.yearly[0][0], 5_480_000);
        assert_eq!(d.mortality.yearly[1][0], 6_703_333);
        assert_eq!((d.fertility.year, d.fertility.first_age), (2019, 15));
        assert_eq!(d.fertility.rates_ppm.len(), 35);
        let tfr = d.fertility.total_ppm();
        assert!((1_760_000..1_780_000).contains(&tfr), "{tfr}");
        assert_eq!(d.sex_ratio_at_birth_ppm, 514_000);
        let start = Date {
            year: 2021,
            month: 12,
            day: 1,
        };
        let s = &d.retirement_age_schedule;
        assert_eq!(s.age_in_force(0, start), Some(741));
        assert_eq!(s.age_in_force(1, start), Some(780));
        let later = Date {
            year: 2030,
            month: 1,
            day: 1,
        };
        assert_eq!(s.age_in_force(0, later), Some(756));
        assert!(read_demography("{}").is_err());
    }

    #[test]
    fn the_margin_files_of_stages_k_and_l_read() {
        let k =
            read_inactivity_margins(&fixture("inactivity_enrolment2021_ro.json")).expect("reads");
        assert_eq!(k.margins.age_bands.len(), 21);
        assert_eq!(k.margins.enrolment.len(), 2 * 37 * ENROLMENT_LEVELS);
        let regions: Vec<String> = [
            "RO11", "RO12", "RO21", "RO22", "RO31", "RO32", "RO41", "RO42",
        ]
        .iter()
        .map(|s| (*s).to_string())
        .collect();
        let l = read_linked_emigrant_margins(&fixture("linked_emigrants2021_ro.json"), &regions)
            .expect("reads");
        assert_eq!(l.margins.linked_households, 1_200_000);
        assert_eq!(l.margins.departure_age_weights.len(), 2 * 52);
        assert_eq!(l.margins.region_factor_ppm[5], 400_000);
        assert_eq!(l.margins.head_age_factor.last().map(|b| b.1), Some(OPEN));
        assert!(
            read_linked_emigrant_margins(&fixture("linked_emigrants2021_ro.json"), &regions[..2])
                .is_err()
        );
    }
}
