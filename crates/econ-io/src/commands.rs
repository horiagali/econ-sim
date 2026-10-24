//! Commands files of headless play (spec `game/lever-system`, "Headless
//! play"): JSON lines in the form of the command log, `seq` and `effective`
//! optional, values as integers or in display units (`"21%"`, `"2,550 lei"`).
//! The scenario's `law.json` and `world.json` are the same lines, dated the
//! start date. Behind the feature `levers`.

use std::path::Path;

use econ_core::levers::{
    Command, LawInForce, LeverMeta, LeverValue, LogLine, Order, Pending, Registry, RejectCode,
    Rejection, Shape, validate,
};
use econ_core::world::{World, WorldParams};
use econ_types::{Bani, Date};
use serde_json::Value;

use crate::SaveError;
use crate::budget::Budget;

/// Read and validate a whole commands file before the first day runs.
///
/// Each line is validated on the day it was given against the lines before
/// it (a cancel names an earlier line still pending on its day). `seq` is
/// the line's own or, when absent, one more than the last; `effective` the
/// line's own or the earliest the lever allows.
///
/// # Errors
/// The first rejected line: its line number (from 1) and the rejection.
pub fn read_commands(path: &Path, registry: &Registry) -> Result<Vec<LogLine>, (usize, Rejection)> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| (0, malformed(format!("{}: {e}", path.display()))))?;
    let mut lines: Vec<LogLine> = Vec::new();
    let mut pending: Vec<Pending> = Vec::new();
    let mut last_seq = 0;
    for (n, raw) in text.lines().enumerate() {
        if raw.trim().is_empty() {
            continue;
        }
        let at = |r: Rejection| (n + 1, r);
        let mut line = parse_line(raw, registry).map_err(at)?;
        let seq = line.seq.unwrap_or(last_seq + 1);
        last_seq = seq;
        // What is still pending on the day this line was given.
        pending.retain(|p| p.effective > line.given);
        let (effective, _) = validate(registry, &line.order, line.given, &pending).map_err(at)?;
        match &line.order {
            Order::Set(c) => pending.push(Pending {
                seq,
                given: line.given,
                command: c.clone(),
                effective,
            }),
            Order::Cancel { seq: target } => pending.retain(|p| p.seq != *target),
        }
        line.seq = Some(seq);
        line.effective = Some(line.effective.unwrap_or(effective));
        lines.push(line);
    }
    Ok(lines)
}

/// The law of a scenario folder.
#[derive(Debug, Clone)]
pub struct ScenarioLaw {
    /// The registry, its keyed levers' `keys` filled from the scenario's
    /// tables (industries, budget lines, public units, …).
    pub registry: Registry,
    /// The lines of `law.json` then `world.json`, in file order.
    pub lines: Vec<LogLine>,
    /// The law in force they set.
    pub law: LawInForce,
}

/// Read a scenario folder's `law.json` and `world.json` against the registry
/// (spec, "Default"; ADR-0018).
///
/// # Errors
/// A line rejected (its line number from 1 in its file and the rejection), or
/// an active lever or key the files do not set (line 0, the rejection naming
/// it).
pub fn read_scenario_law(
    dir: &Path,
    registry: &Registry,
) -> Result<ScenarioLaw, (usize, Rejection)> {
    let registry = with_scenario_keys(dir, registry).map_err(|m| (0, malformed(m)))?;
    let mut lines = Vec::new();
    for file in ["law.json", "world.json"] {
        let path = dir.join(file);
        let text = std::fs::read_to_string(&path)
            .map_err(|e| (0, malformed(format!("{}: {e}", path.display()))))?;
        for (n, raw) in text.lines().enumerate() {
            if raw.trim().is_empty() {
                continue;
            }
            lines.push(parse_line(raw, &registry).map_err(|r| (n + 1, r))?);
        }
    }
    let law = LawInForce::from_lines(&registry, &lines).map_err(|r| (0, r))?;
    Ok(ScenarioLaw {
        registry,
        lines,
        law,
    })
}

/// The registry with the keys of the levers keyed by a scenario table
/// (industries, budget lines, public units) filled from the folder, unless
/// the registry names a subset.
fn with_scenario_keys(dir: &Path, registry: &Registry) -> Result<Registry, String> {
    let industries = crate::start::industry_ids(dir).map_err(|e| format!("{e:?}"))?;
    let budget_path = dir.join("budget.json");
    let bytes =
        std::fs::read(&budget_path).map_err(|e| format!("{}: {e}", budget_path.display()))?;
    let budget = Budget::from_json(&bytes).map_err(|e| format!("budget.json: {e:?}"))?;
    let lines: Vec<String> = budget.lines.iter().map(|l| l.id.clone()).collect();
    let units: Vec<String> = budget.public_units.iter().map(|u| u.unit.clone()).collect();
    let mut out = registry.clone();
    for meta in &mut out.levers {
        if !meta.keys.is_empty() {
            continue;
        }
        let table = match meta.key.as_deref() {
            Some("industries") => &industries,
            Some("budget_lines") => &lines,
            Some("public_units") => &units,
            _ => continue,
        };
        meta.keys.clone_from(table);
    }
    Ok(out)
}

/// A rejection of a line that is not a command.
fn malformed(message: String) -> Rejection {
    Rejection {
        code: RejectCode::WrongShape,
        lever: None,
        message,
    }
}

/// One JSON line of a commands file, a log, `law.json` or `world.json`.
fn parse_line(raw: &str, registry: &Registry) -> Result<LogLine, Rejection> {
    let v: Value =
        serde_json::from_str(raw).map_err(|e| malformed(format!("Not a command: {e}.")))?;
    let date = |field: &str| -> Result<Option<Date>, Rejection> {
        match v.get(field) {
            None | Some(Value::Null) => Ok(None),
            Some(d) => d
                .as_str()
                .and_then(parse_date)
                .map(Some)
                .ok_or_else(|| malformed(format!("`{field}` is not a date like 2022-03-01."))),
        }
    };
    let given = date("given")?.ok_or_else(|| malformed("A command needs `given`.".to_string()))?;
    let effective = date("effective")?;
    let seq = v.get("seq").and_then(Value::as_u64);
    if let Some(target) = v.get("cancel") {
        let target = target
            .as_u64()
            .ok_or_else(|| malformed("`cancel` is a sequence number.".to_string()))?;
        return Ok(LogLine {
            seq,
            given,
            order: Order::Cancel { seq: target },
            effective,
        });
    }
    let id = v
        .get("lever")
        .and_then(Value::as_str)
        .ok_or_else(|| malformed("A command needs `lever` or `cancel`.".to_string()))?;
    let meta = registry.find(id).ok_or_else(|| Rejection {
        code: RejectCode::UnknownLever,
        lever: None,
        message: format!("There is no lever `{id}`."),
    })?;
    let key = match v.get("key").and_then(Value::as_str) {
        None => None,
        Some(k) => Some(
            meta.keys
                .iter()
                .position(|x| x == k)
                .and_then(|i| u32::try_from(i).ok())
                .ok_or_else(|| Rejection {
                    code: RejectCode::BadKey,
                    lever: Some(meta.id),
                    message: format!(
                        "`{}` has no `{k}`; choose one of {}.",
                        meta.label,
                        meta.keys.join(", ")
                    ),
                })?,
        ),
    };
    let value = value_of(meta, v.get("value").unwrap_or(&Value::Null))?;
    Ok(LogLine {
        seq,
        given,
        order: Order::Set(Command {
            lever: meta.id,
            key,
            value,
            effective,
        }),
        effective,
    })
}

/// A JSON value as the lever's shape: integers, or display units for rates
/// and money, an option's text for a choice.
fn value_of(meta: &LeverMeta, v: &Value) -> Result<LeverValue, Rejection> {
    let wrong = || Rejection {
        code: RejectCode::WrongShape,
        lever: Some(meta.id),
        message: format!("`{}` does not take {v}.", meta.label),
    };
    let int = |v: &Value| v.as_i64();
    Ok(match meta.shape {
        Shape::Rate => {
            let bp = match v {
                Value::String(s) => decimal(s.trim().strip_suffix('%').ok_or_else(wrong)?),
                _ => int(v),
            };
            LeverValue::Rate(bp.and_then(|b| i32::try_from(b).ok()).ok_or_else(wrong)?)
        }
        Shape::Money => {
            let bani = match v {
                Value::String(s) => decimal(
                    &s.trim()
                        .strip_suffix("lei")
                        .ok_or_else(wrong)?
                        .replace(',', ""),
                ),
                _ => int(v),
            };
            LeverValue::Money(Bani(bani.ok_or_else(wrong)?))
        }
        Shape::Number => LeverValue::Number(int(v).ok_or_else(wrong)?),
        Shape::Choice => {
            let i = match v {
                Value::String(s) => {
                    meta.options
                        .iter()
                        .position(|o| o == s)
                        .ok_or_else(|| Rejection {
                            code: RejectCode::BadOption,
                            lever: Some(meta.id),
                            message: format!(
                                "`{}` can be {}.",
                                meta.label,
                                meta.options.join(", ")
                            ),
                        })?
                }
                _ => int(v)
                    .and_then(|i| usize::try_from(i).ok())
                    .ok_or_else(wrong)?,
            };
            LeverValue::Choice(u16::try_from(i).map_err(|_| wrong())?)
        }
        Shape::Flag => LeverValue::Flag(v.as_bool().ok_or_else(wrong)?),
        Shape::Schedule => {
            let rows = v.as_array().ok_or_else(wrong)?;
            let mut out = Vec::with_capacity(rows.len());
            for row in rows {
                let pair = row.as_array().filter(|p| p.len() == 2).ok_or_else(wrong)?;
                let threshold = int(&pair[0]).ok_or_else(wrong)?;
                let rate = int(&pair[1])
                    .and_then(|r| i32::try_from(r).ok())
                    .ok_or_else(wrong)?;
                out.push((Bani(threshold), rate));
            }
            LeverValue::Schedule(out)
        }
    })
}

/// A decimal with at most two places, times 100, exactly: `21` → 2100,
/// `3.75` → 375, `-0.5` → -50.
fn decimal(s: &str) -> Option<i64> {
    let s = s.trim();
    let (neg, s) = match s.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, s),
    };
    let (whole, frac) = s.split_once('.').unwrap_or((s, ""));
    if whole.is_empty()
        || frac.len() > 2
        || !whole
            .bytes()
            .chain(frac.bytes())
            .all(|b| b.is_ascii_digit())
    {
        return None;
    }
    let whole: i64 = whole.parse().ok()?;
    let frac: i64 = if frac.is_empty() {
        0
    } else {
        format!("{frac:0<2}").parse().ok()?
    };
    let v = whole.checked_mul(100)?.checked_add(frac)?;
    Some(if neg { -v } else { v })
}

/// `2022-03-01`.
fn parse_date(s: &str) -> Option<Date> {
    let mut parts = s.split('-');
    let year = parts.next()?.parse().ok()?;
    let month = parts.next()?.parse().ok()?;
    let day = parts.next()?.parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Date::new(year, month, day)
}

/// One line of the command log as JSON (spec, "Logging and deterministic
/// replay"): `{"seq":17,"given":"2022-02-14","lever":"…","key":"…",
/// "value":2100,"effective":"2022-03-01"}` or `{"seq":19,"given":"…",
/// "cancel":17}`. Values are the shape's integers, a choice its option's
/// text, a schedule `[[threshold_bani, rate_bp], …]`; a key its text.
#[must_use]
pub fn log_line_json(line: &LogLine, registry: &Registry) -> String {
    let text = |s: &str| serde_json::to_string(s).unwrap_or_else(|_| String::from("\"\""));
    let mut out = String::from("{");
    if let Some(seq) = line.seq {
        out.push_str(&format!("\"seq\":{seq},"));
    }
    out.push_str(&format!("\"given\":\"{}\"", iso(line.given)));
    match &line.order {
        Order::Cancel { seq } => out.push_str(&format!(",\"cancel\":{seq}")),
        Order::Set(c) => {
            let meta = registry.get(c.lever);
            out.push_str(&format!(",\"lever\":{}", text(c.lever.as_str())));
            if let Some(k) = c.key {
                let key = usize::try_from(k)
                    .ok()
                    .and_then(|k| meta.keys.get(k))
                    .map_or_else(|| k.to_string(), Clone::clone);
                out.push_str(&format!(",\"key\":{}", text(&key)));
            }
            let value = value_json(meta, &c.value);
            out.push_str(&format!(",\"value\":{value}"));
            if let Some(e) = line.effective {
                out.push_str(&format!(",\"effective\":\"{}\"", iso(e)));
            }
        }
    }
    out.push('}');
    out
}

/// A lever's value as JSON, as the log writes it: the shape's integers, a
/// choice its option's text, a schedule `[[threshold_bani, rate_bp], …]`.
#[must_use]
pub fn value_json(meta: &LeverMeta, value: &LeverValue) -> String {
    match value {
        LeverValue::Rate(v) => v.to_string(),
        LeverValue::Money(b) => b.get().to_string(),
        LeverValue::Number(n) => n.to_string(),
        LeverValue::Choice(i) => meta.options.get(usize::from(*i)).map_or_else(
            || i.to_string(),
            |o| serde_json::to_string(o).unwrap_or_else(|_| String::from("\"\"")),
        ),
        LeverValue::Flag(f) => f.to_string(),
        LeverValue::Schedule(rows) => format!(
            "[{}]",
            rows.iter()
                .map(|(t, r)| format!("[{},{r}]", t.get()))
                .collect::<Vec<_>>()
                .join(",")
        ),
    }
}

/// A date as `2022-03-01`.
#[must_use]
pub fn iso(d: Date) -> String {
    format!("{:04}-{:02}-{:02}", d.year, d.month, d.day)
}

/// The format of a save of [`save_world`].
const SAVE_FORMAT: u64 = 1;

/// A save of a `World` under the lever system (spec, "Logging and
/// deterministic replay"; ADR-0011).
///
/// > **Simplification (M4):** a save is the replay the determinism contract
/// > guarantees (ADR-0006: the scenario, the seed and the log give the same
/// > state every day), not a snapshot of the columns: the engine version,
/// > the seed, the day the world stands on, its state hash and the command
/// > log, as JSON. Loading rebuilds the world from the scenario folder,
/// > replays the log to that day and checks the hash; the law in force and
/// > the pending list follow from the log. Loading takes as long as running
/// > to the saved day. Only worlds with the spec defaults' parameters are
/// > saved until the parameters are a scenario file (M6).
///
/// # Errors
/// A world built without `law.json`, or with parameters other than the spec
/// defaults.
pub fn save_world(world: &World) -> Result<Vec<u8>, SaveError> {
    let seed = world.params().rng_seed;
    if *world.params() != WorldParams::spec_defaults(seed) {
        return Err(SaveError::Io(
            "only worlds with the spec defaults' parameters are saved (M4)".to_string(),
        ));
    }
    let registry = world.registry();
    let log: Vec<Value> = world
        .command_log()
        .iter()
        .map(|l| serde_json::from_str(&log_line_json(l, registry)))
        .collect::<Result<_, _>>()
        .map_err(|e| SaveError::Parse(e.to_string()))?;
    let save = serde_json::json!({
        "format": SAVE_FORMAT,
        "engine_version": env!("CARGO_PKG_VERSION"),
        "seed": seed,
        "date": iso(world.date()),
        "state_hash": format!("{:016x}", world.state_hash()),
        "log": log,
    });
    serde_json::to_vec_pretty(&save).map_err(|e| SaveError::Parse(e.to_string()))
}

/// Load a save of [`save_world`] made on a world of the scenario folder `dir`
/// (`io_table`: the input-output table, as for `read_start_state`).
///
/// # Errors
/// A damaged save, one of another engine, a log the scenario's registry
/// refuses, or a replay whose state hash differs from the saved one (another
/// scenario).
pub fn load_world(bytes: &[u8], dir: &Path, io_table: &Path) -> Result<World, SaveError> {
    let parse = |m: String| SaveError::Parse(m);
    let save: Value = serde_json::from_slice(bytes).map_err(|e| parse(e.to_string()))?;
    let format = save["format"]
        .as_u64()
        .ok_or_else(|| parse("no format".into()))?;
    if format != SAVE_FORMAT {
        return Err(SaveError::Version(
            u32::try_from(format).unwrap_or(u32::MAX),
        ));
    }
    if save["engine_version"].as_str() != Some(env!("CARGO_PKG_VERSION")) {
        return Err(parse("a save of another engine version".into()));
    }
    let seed = save["seed"]
        .as_u64()
        .ok_or_else(|| parse("no seed".into()))?;
    let date = save["date"]
        .as_str()
        .and_then(parse_date)
        .ok_or_else(|| parse("no date".into()))?;
    let expected = save["state_hash"]
        .as_str()
        .ok_or_else(|| parse("no state hash".into()))?
        .to_string();

    let rejected = |(n, r): (usize, Rejection)| parse(format!("line {n}: {}", r.message));
    let law = read_scenario_law(dir, &Registry::committed()).map_err(rejected)?;
    let start = crate::read_start_state(dir, io_table)?;
    let mut world = World::with_law(
        start,
        law.registry,
        law.law,
        WorldParams::spec_defaults(seed),
    )
    .map_err(|e| parse(format!("{e:?}")))?;
    let mut log = Vec::new();
    for (n, line) in save["log"]
        .as_array()
        .map_or(&[][..], Vec::as_slice)
        .iter()
        .enumerate()
    {
        log.push(
            parse_line(&line.to_string(), world.registry()).map_err(|r| rejected((n + 1, r)))?,
        );
    }
    let mut next = 0;
    loop {
        let today = world.date();
        while next < log.len() && log[next].given == today {
            world
                .give(log[next].order.clone())
                .map_err(|r| rejected((next + 1, r)))?;
            next += 1;
        }
        if today >= date {
            break;
        }
        world.step(&[]).map_err(|e| parse(format!("{e:?}")))?;
    }
    let got = format!("{:016x}", world.state_hash());
    if got != expected || world.date() != date {
        return Err(SaveError::StateHash { expected, got });
    }
    Ok(world)
}
