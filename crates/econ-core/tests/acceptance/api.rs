//! Acceptance tests for AC-API-01 to AC-API-05 (spec: game/core-api, draft
//! of 2026-11-01), written by the test writer from the spec and its API
//! sketch before the code (ADR-0014); behind the feature `api`, against the
//! test writer's API skeleton `econ_core::api` (`Query`, `Command`, `Answer`
//! and their DTOs, `ApiError`, `World::query`, `World::command`).
//!
//! "1:1000" is the Romania scenario at that scale, seed 42, under the law of
//! its folder (`law.json`: the 2021 law), every parameter at its spec
//! default. Month 0 is December 2021.
//!
//! Readings shared by the tests (the skeleton's module documentation states
//! the shapes): indicators by dotted name, breakdowns, units, periods,
//! aggregates, views and statuses by the registry files' spellings, levers
//! by dotted id and keys by text id, dates as `DateDto`. `Series { from, to
//! }` covers the months `from..=to`.

use econ_core::api::{
    Answer, ApiError, BlockDto, Command, DateDto, IndicatorDto, KeyDto, LeverDto, LeversDto,
    OrderDto, OrderedDto, PendingDto, Query, SeriesDto, StatusDto, SteppedDto, ValueDto,
};
use econ_core::explain::Node;
use econ_core::groups::{Attribute, Condition, Filter, GroupSummary};
use econ_core::levers::{LeverValue, RejectCode, Status as LeverStatus};
use econ_core::stats::{
    Aggregate, Breakdown, Derive, IndicatorMeta, Key, Period, Status, Unit, View, indicators,
};
use econ_core::world::World;
use econ_popgen::{Activity, EduLevel, Enrolled, Sex};
use econ_types::{Bani, Calendar, Date, Day};

use super::support::repo_root;
use super::support_lev::{date, python, start_date};
use super::support_m6::{
    NATIONAL, computed, feature_on, id, name_of, run_closes, step_checked, world_2021_default,
};

// --- the registry's spellings --------------------------------------------------

fn unit_name(u: Unit) -> &'static str {
    match u {
        Unit::Bani => "bani",
        Unit::RealBani => "real_bani",
        Unit::Ppm => "ppm",
        Unit::Persons => "persons",
        Unit::IndexPpm => "index_ppm",
        Unit::Milli => "milli",
        Unit::Count => "count",
    }
}

fn period_name(p: Period) -> &'static str {
    match p {
        Period::Month => "month",
        Period::Day => "day",
    }
}

fn breakdown_name(b: Breakdown) -> &'static str {
    match b {
        Breakdown::Region => "region",
        Breakdown::County => "county",
        Breakdown::Industry => "industry",
        Breakdown::Class => "class",
        Breakdown::Fifth => "fifth",
        Breakdown::AgeBand => "age_band",
        Breakdown::Sex => "sex",
        Breakdown::Edu => "edu",
        Breakdown::Activity => "activity",
        Breakdown::Level => "level",
        Breakdown::Flow => "flow",
        Breakdown::Line => "line",
        Breakdown::Good => "good",
    }
}

fn aggregate_name(a: Aggregate) -> &'static str {
    match a {
        Aggregate::Sum => "sum",
        Aggregate::Mean => "mean",
        Aggregate::None => "none",
    }
}

fn view_name(v: View) -> &'static str {
    match v {
        View::Overview => "overview",
        View::Budget => "budget",
        View::Prices => "prices",
        View::Finance => "finance",
        View::Trade => "trade",
        View::People => "people",
        View::Informal => "informal",
        View::Map => "map",
    }
}

fn derive_text(d: Derive) -> String {
    match d {
        Derive::Ratio(a, b) => format!("ratio({}, {})", name_of(a), name_of(b)),
        Derive::Real(a) => format!("real({})", name_of(a)),
        Derive::GrowthYoy(a) => format!("growth_yoy({})", name_of(a)),
        Derive::GrowthMom(a) => format!("growth_mom({})", name_of(a)),
        Derive::PerCapita(a) => format!("per_capita({})", name_of(a)),
    }
}

/// A registry entry as the API answers it.
fn indicator_dto(m: &IndicatorMeta) -> IndicatorDto {
    IndicatorDto {
        name: m.name.to_string(),
        glossary: m.glossary.to_string(),
        unit: unit_name(m.unit).to_string(),
        period: period_name(m.period).to_string(),
        breakdowns: m
            .breakdowns
            .iter()
            .map(|&b| breakdown_name(b).to_string())
            .collect(),
        aggregate: aggregate_name(m.aggregate).to_string(),
        derive: m.derive.map(derive_text),
        feature: m.feature.map(str::to_string),
        views: m.views.iter().map(|&v| view_name(v).to_string()).collect(),
        headline: m.headline,
        status: match m.status {
            Status::Active => "active",
            Status::Planned => "planned",
        }
        .to_string(),
    }
}

fn key_dto(k: Key) -> KeyDto {
    KeyDto {
        breakdown: k.breakdown.map(|b| breakdown_name(b).to_string()),
        index: k.index,
    }
}

fn national() -> KeyDto {
    key_dto(NATIONAL)
}

fn date_dto(d: Date) -> DateDto {
    DateDto {
        year: d.year,
        month: d.month,
        day: d.day,
    }
}

fn value_dto(v: &LeverValue) -> ValueDto {
    match v {
        LeverValue::Rate(r) => ValueDto::Rate(*r),
        LeverValue::Money(b) => ValueDto::Money(b.get()),
        LeverValue::Number(n) => ValueDto::Number(*n),
        LeverValue::Choice(c) => ValueDto::Choice(*c),
        LeverValue::Flag(f) => ValueDto::Flag(*f),
        LeverValue::Schedule(rows) => {
            ValueDto::Schedule(rows.iter().map(|(t, r)| (t.get(), *r)).collect())
        }
    }
}

/// The lever registry, the law in force and the pending orders, as the API
/// answers them.
fn levers_dto(w: &World) -> LeversDto {
    let law = w.law();
    let levers = w
        .registry()
        .levers
        .iter()
        .map(|m| LeverDto {
            id: m.id.as_str().to_string(),
            status: match m.status {
                LeverStatus::Active => "active",
                LeverStatus::Planned => "planned",
                LeverStatus::Retired => "retired",
            }
            .to_string(),
            keys: m.keys.clone(),
            in_force: if m.keys.is_empty() {
                vec![law.get(m.id, None).map(value_dto)]
            } else {
                (0..m.keys.len())
                    .map(|k| {
                        law.get(m.id, Some(u32::try_from(k).expect("a key")))
                            .map(value_dto)
                    })
                    .collect()
            },
        })
        .collect();
    let pending = w
        .pending()
        .iter()
        .map(|p| {
            let meta = w.registry().get(p.command.lever);
            PendingDto {
                seq: p.seq,
                given: date_dto(p.given),
                lever: p.command.lever.as_str().to_string(),
                key: p
                    .command
                    .key
                    .map(|k| meta.keys[usize::try_from(k).expect("a key")].clone()),
                value: value_dto(&p.command.value),
                effective: date_dto(p.effective),
            }
        })
        .collect();
    LeversDto { levers, pending }
}

// --- variants -------------------------------------------------------------------

/// Every query variant's name (an exhaustive match: a new variant must be
/// added here, and so to the round trip and to `api.ts`'s check).
fn query_name(q: &Query) -> &'static str {
    match q {
        Query::Status => "Status",
        Query::Indicators => "Indicators",
        Query::Series { .. } => "Series",
        Query::Keys { .. } => "Keys",
        Query::Group { .. } => "Group",
        Query::GroupBreakdown { .. } => "GroupBreakdown",
        Query::Watched { .. } => "Watched",
        Query::Explain { .. } => "Explain",
        Query::Explained => "Explained",
        Query::Levers => "Levers",
    }
}

/// Every command variant's name.
fn command_name(c: &Command) -> &'static str {
    match c {
        Command::Step { .. } => "Step",
        Command::StepMonths { .. } => "StepMonths",
        Command::Order { .. } => "Order",
        Command::Watch { .. } => "Watch",
        Command::WriteHistory { .. } => "WriteHistory",
    }
}

/// Every answer variant's name.
fn answer_name(a: &Answer) -> &'static str {
    match a {
        Answer::Status(_) => "Status",
        Answer::Indicators(_) => "Indicators",
        Answer::Series(_) => "Series",
        Answer::Keys(_) => "Keys",
        Answer::Group(_) => "Group",
        Answer::GroupBreakdown(_) => "GroupBreakdown",
        Answer::Watched(_) => "Watched",
        Answer::Explain(_) => "Explain",
        Answer::Explained(_) => "Explained",
        Answer::Levers(_) => "Levers",
        Answer::Stepped(_) => "Stepped",
        Answer::Ordered(_) => "Ordered",
        Answer::Watching(_) => "Watching",
        Answer::Written(_) => "Written",
    }
}

/// A filter with a condition on every attribute.
fn every_condition() -> Filter {
    Filter {
        conditions: vec![
            Condition::County(vec![3, 40]),
            Condition::Region(vec![0, 7]),
            Condition::Urban(true),
            Condition::Age { from: 18, to: 65 },
            Condition::Sex(vec![Sex::F]),
            Condition::Edu(vec![EduLevel::Isced3, EduLevel::Isced7To8]),
            Condition::Activity(vec![Activity::Employed, Activity::Unemployed]),
            Condition::Occupation(vec![2, econ_popgen::NOT_EMPLOYED]),
            Condition::Industry(vec![Some(4), None]),
            Condition::Fifth(vec![0, 4]),
            Condition::Class(vec![0]),
            Condition::Enrolled(vec![Enrolled::None, Enrolled::Tertiary]),
        ],
    }
}

/// One query of each variant.
fn every_query() -> Vec<Query> {
    vec![
        Query::Status,
        Query::Indicators,
        Query::Series {
            id: "fx.eur_lei".to_string(),
            key: national(),
            from: 0,
            to: 2,
            daily: true,
        },
        Query::Keys {
            id: "people.population".to_string(),
        },
        Query::Group {
            filter: every_condition(),
        },
        Query::GroupBreakdown {
            filter: Filter::default(),
            by: Attribute::AgeBand,
        },
        Query::Watched {
            name: "the young".to_string(),
        },
        Query::Explain {
            id: "prices.cpi".to_string(),
            key: key_dto(Key::of(Breakdown::Region, 2)),
            from: 1,
            to: 2,
        },
        Query::Explained,
        Query::Levers,
    ]
}

/// One command of each variant (both kinds of order).
fn every_command() -> Vec<Command> {
    vec![
        Command::Step { days: 3 },
        Command::StepMonths { months: 2 },
        Command::Order {
            order: OrderDto::Set {
                lever: "world.fuel_price".to_string(),
                key: Some("oil".to_string()),
                value: ValueDto::Money(48_600),
                effective: Some(DateDto {
                    year: 2022,
                    month: 3,
                    day: 1,
                }),
            },
        },
        Command::Order {
            order: OrderDto::Set {
                lever: "tax.income.schedule".to_string(),
                key: None,
                value: ValueDto::Schedule(vec![(0, 1000), (500_000, 1600)]),
                effective: None,
            },
        },
        Command::Order {
            order: OrderDto::Cancel { seq: 17 },
        },
        Command::Watch {
            name: "workers".to_string(),
            filter: every_condition(),
        },
        Command::WriteHistory {
            dir: "out/history".to_string(),
        },
    ]
}

/// A summary with every field set.
fn some_summary(k: u64) -> GroupSummary {
    GroupSummary {
        persons: 1_000 + k,
        households: 400,
        person_records: 31,
        household_records: 12,
        small_sample: false,
        employed: 500,
        unemployed: 30,
        unemployment_rate_ppm: 56_604,
        wages_gross: Bani(123_456_789),
        wages_net: Bani(80_000_000),
        pensions: Bani(-0),
        unemployment_benefits: Bani(7),
        household_transfers: Bani(99),
        taxes_paid: Bani(43_456_789),
        money: Bani(9_000_000_000),
        debt: Bani(-1),
        income_per_adult: Some(Bani(250_000)),
        sol_milli: Some(1_234),
        approval_milli: Some(-12_500),
    }
}

/// One answer of each variant.
fn every_answer() -> Vec<Answer> {
    let node = Node {
        label: "prices.cpi".to_string(),
        value: 12_000,
        indicator: Some((id("prices.cpi"), NATIONAL)),
        children: vec![
            Node {
                label: "refined_petroleum".to_string(),
                value: 9_000,
                indicator: Some((id("prices.cpi_contribution"), Key::of(Breakdown::Good, 5))),
                children: Vec::new(),
            },
            Node {
                label: "the rest".to_string(),
                value: 3_000,
                indicator: None,
                children: Vec::new(),
            },
        ],
    };
    vec![
        Answer::Status(StatusDto {
            date: DateDto {
                year: 2022,
                month: 2,
                day: 1,
            },
            day: 62,
            month: 2,
            sample_scale: 1000,
            law: "2021".to_string(),
            features: vec!["stats".to_string(), "api".to_string()],
        }),
        Answer::Indicators(indicators().iter().take(3).map(indicator_dto).collect()),
        Answer::Series(SeriesDto {
            first_month: 1,
            months: vec![4_950_000, -3, i64::MAX],
            days: Some(vec![1, 2, 3]),
        }),
        Answer::Keys(vec![national(), key_dto(Key::of(Breakdown::County, 41))]),
        Answer::Group(some_summary(0)),
        Answer::GroupBreakdown(vec![(0, some_summary(1)), (1, GroupSummary::default())]),
        Answer::Watched(Some(vec![some_summary(2)])),
        Answer::Watched(None),
        Answer::Explain(Some(node)),
        Answer::Explain(None),
        Answer::Explained(vec!["gdp.nominal".to_string()]),
        Answer::Levers(LeversDto {
            levers: vec![LeverDto {
                id: "world.fuel_price".to_string(),
                status: "active".to_string(),
                keys: vec!["oil".to_string(), "gas".to_string(), "coal".to_string()],
                in_force: vec![Some(ValueDto::Money(32_400)), None, None],
            }],
            pending: vec![PendingDto {
                seq: 3,
                given: DateDto {
                    year: 2021,
                    month: 12,
                    day: 1,
                },
                lever: "tax.vat.rate_standard".to_string(),
                key: None,
                value: ValueDto::Rate(2100),
                effective: DateDto {
                    year: 2022,
                    month: 1,
                    day: 1,
                },
            }],
        }),
        Answer::Stepped(SteppedDto {
            days: 62,
            closes: 2,
        }),
        Answer::Ordered(OrderedDto {
            seq: 4,
            effective: DateDto {
                year: 2022,
                month: 1,
                day: 1,
            },
            warnings: vec!["below the EU minimum".to_string()],
        }),
        Answer::Watching("workers".to_string()),
        Answer::Written(vec![BlockDto {
            year: 2022,
            file: "2022.arrow".to_string(),
            sha256: "ab".repeat(32),
            rows: 12_345,
        }]),
    ]
}

/// The first day index of month `m`, counting days from the start.
fn first_day(m: u32) -> usize {
    let cal = Calendar::new(start_date());
    (0..)
        .find(|&d| cal.month(Day(d)).0 == m)
        .map(|d| usize::try_from(d).expect("a day"))
        .expect("a day of the month")
}

/// A query's answer; panics on an error.
fn ask(w: &World, q: &Query) -> Answer {
    w.query(q)
        .unwrap_or_else(|e| panic!("{}: {e:?}", query_name(q)))
}

/// A command's answer; panics on an error.
fn tell(w: &mut World, c: &Command) -> Answer {
    w.command(c)
        .unwrap_or_else(|e| panic!("{}: {e:?}", command_name(c)))
}

fn vat_order(rate: i32) -> OrderDto {
    OrderDto::Set {
        lever: "tax.vat.rate_standard".to_string(),
        key: None,
        value: ValueDto::Rate(rate),
        effective: None,
    }
}

// --- AC-API-01 ------------------------------------------------------------------

/// **AC-API-01** `[sim]` Every query's answer equals the direct method's
/// (`Series` = `stats().series`, `Group` = `group`, `Explain` = `explain`,
/// `Levers` = the registry and the law in force) at 1:1000 after 3 closes,
/// and no query changes the state hash.
///
/// Readings: a 1:1000 world with a group watched and a VAT order pending
/// (given on day 0, effective 1 January 2023), after 3 closes. Each query
/// against its direct method:
/// - `Status`: today's date, day index and month index, `sample_scale`
///   1000, and the features compiled: exactly econ-core's features this
///   build turned on, of those the test knows (the law's name is not
///   checked);
/// - `Indicators`: every registry entry in order, in the registry's
///   spellings;
/// - `Series`: for every computed indicator and key, months 0 to 2 with
///   daily points (`days` for a day indicator, else `None`), and months 1 to
///   2 with and without them (a day indicator's days from month 1's first);
/// - `Keys`: `stats().keys` of every computed indicator;
/// - `Group` and `GroupBreakdown` (by sex): `group` and `group_breakdown`;
///   `Watched`: `watched` (a name never watched: `None`);
/// - `Explain`: `explain` of every indicator of `explained()` over (1, 2)
///   (the year-on-year rates, without points yet, as `None` or not, as
///   `explain` answers), and of `people.gini`; `Explained`: the names of
///   `explained()`;
/// - `Levers`: every lever of the world's registry with its keys and value
///   in force per key, and the pending orders.
///
/// The state hash is the same before and after all the queries.
#[test]
fn ac_api_01() {
    use super::support_lev::{give, set_on};
    let mut w = world_2021_default(1000);
    let region0 = Filter {
        conditions: vec![Condition::Region(vec![0])],
    };
    w.watch_group("region 0", region0.clone());
    let vat = w.registry().find("tax.vat.rate_standard").expect("VAT").id;
    give(
        &mut w,
        set_on(vat, None, LeverValue::Rate(2000), date(2023, 1, 1)),
    );
    run_closes(&mut w, 3);
    let hash = w.state_hash();

    let cal = Calendar::new(start_date());
    let Answer::Status(s) = ask(&w, &Query::Status) else {
        panic!("Status: another answer")
    };
    assert_eq!(s.date, date_dto(w.date()), "Status: the date");
    assert_eq!(s.day, w.day().0, "Status: the day");
    assert_eq!(s.month, cal.month(w.day()).0, "Status: the month");
    assert_eq!(s.sample_scale, 1000, "Status: sample_scale");
    let known = [
        "world",
        "pay_day",
        "banking",
        "fiscal",
        "monetary",
        "trade",
        "investment",
        "revenue",
        "demography",
        "lifecycle",
        "levers",
        "policy",
        "lever_policy_rate",
        "lever_monetary_financing",
        "lever_fx_devaluation",
        "lever_fuel_price",
        "migration",
        "education",
        "approval",
        "poverty",
        "wellbeing",
        "home_quality",
        "stats",
        "groups",
        "explain",
        "api",
    ];
    for f in known {
        assert_eq!(
            s.features.iter().any(|x| x == f),
            feature_on(f),
            "Status: feature {f} listed exactly when compiled: {:?}",
            s.features
        );
    }

    assert_eq!(
        ask(&w, &Query::Indicators),
        Answer::Indicators(indicators().iter().map(indicator_dto).collect()),
        "Indicators"
    );

    let stats = w.stats();
    let mut series = 0usize;
    for m in indicators().iter().filter(|m| computed(m)) {
        let keys = stats.keys(m.id);
        assert_eq!(
            ask(
                &w,
                &Query::Keys {
                    id: m.name.to_string()
                }
            ),
            Answer::Keys(keys.iter().copied().map(key_dto).collect()),
            "{}: Keys",
            m.name
        );
        for &k in keys {
            let direct = stats.series(m.id, k).expect("a key's series");
            let q = |from, to, daily| Query::Series {
                id: m.name.to_string(),
                key: key_dto(k),
                from,
                to,
                daily,
            };
            let first = direct.first_month.0;
            let all = SeriesDto {
                first_month: first,
                months: direct.months.to_vec(),
                days: direct.days.map(<[i64]>::to_vec),
            };
            assert_eq!(
                ask(&w, &q(0, 2, true)),
                Answer::Series(all),
                "{} {k:?}: months 0 to 2",
                m.name
            );
            if first == 0 {
                let from_one = |daily: bool| SeriesDto {
                    first_month: 1,
                    months: direct.months[1..].to_vec(),
                    days: if daily {
                        direct.days.map(|d| d[first_day(1)..].to_vec())
                    } else {
                        None
                    },
                };
                assert_eq!(
                    ask(&w, &q(1, 2, true)),
                    Answer::Series(from_one(true)),
                    "{} {k:?}: months 1 to 2, daily",
                    m.name
                );
                assert_eq!(
                    ask(&w, &q(1, 2, false)),
                    Answer::Series(from_one(false)),
                    "{} {k:?}: months 1 to 2",
                    m.name
                );
            }
            series += 1;
        }
    }
    assert!(series > 0, "some series");

    for f in [Filter::default(), region0.clone()] {
        assert_eq!(
            ask(&w, &Query::Group { filter: f.clone() }),
            Answer::Group(w.group(&f)),
            "Group {f:?}"
        );
        assert_eq!(
            ask(
                &w,
                &Query::GroupBreakdown {
                    filter: f.clone(),
                    by: Attribute::Sex,
                }
            ),
            Answer::GroupBreakdown(w.group_breakdown(&f, Attribute::Sex)),
            "GroupBreakdown {f:?}"
        );
    }
    assert_eq!(
        ask(
            &w,
            &Query::Watched {
                name: "region 0".to_string()
            }
        ),
        Answer::Watched(w.watched("region 0").map(<[GroupSummary]>::to_vec)),
        "Watched"
    );
    assert!(
        w.watched("region 0").is_some_and(|s| s.len() == 3),
        "watched from day 0"
    );
    assert_eq!(
        ask(
            &w,
            &Query::Watched {
                name: "nobody".to_string()
            }
        ),
        Answer::Watched(None),
        "Watched, a name never watched"
    );

    let mut explained: Vec<String> = Vec::new();
    for &i in w.explained() {
        explained.push(name_of(i).to_string());
        assert_eq!(
            ask(
                &w,
                &Query::Explain {
                    id: name_of(i).to_string(),
                    key: national(),
                    from: 1,
                    to: 2,
                }
            ),
            Answer::Explain(w.explain(i, NATIONAL, econ_types::Month(1), econ_types::Month(2))),
            "Explain {}",
            name_of(i)
        );
    }
    assert_eq!(
        ask(&w, &Query::Explained),
        Answer::Explained(explained),
        "Explained"
    );
    assert_eq!(
        ask(
            &w,
            &Query::Explain {
                id: "people.gini".to_string(),
                key: national(),
                from: 1,
                to: 2,
            }
        ),
        Answer::Explain(None),
        "Explain people.gini"
    );

    let levers = levers_dto(&w);
    assert!(!levers.pending.is_empty(), "the VAT order is pending");
    assert_eq!(ask(&w, &Query::Levers), Answer::Levers(levers), "Levers");

    assert_eq!(w.state_hash(), hash, "the queries changed the state");
}

// --- AC-API-02 ------------------------------------------------------------------

fn round_trip<T>(v: &T, what: &str)
where
    T: serde::Serialize + serde::de::DeserializeOwned + PartialEq + std::fmt::Debug,
{
    let json = serde_json::to_string(v).unwrap_or_else(|e| panic!("{what}: serialises: {e}"));
    let back: T =
        serde_json::from_str(&json).unwrap_or_else(|e| panic!("{what}: {json} reads back: {e}"));
    assert_eq!(&back, v, "{what}: {json}");
}

/// **AC-API-02** `[unit]` Every query, command and answer survives a JSON
/// round trip unchanged; an unknown indicator name, a key it lacks and a
/// malformed query are errors, never panics.
///
/// Readings: one value of every variant of `Query`, `Command` (orders of
/// both kinds, a schedule value) and `Answer` (`Watched` and `Explain` with
/// and without a value), built by hand (exhaustive matches name them all),
/// to JSON with `serde_json` and back, equal. The errors, on a 1:1000 world
/// at day 0: `Series`, `Keys` and `Explain` of `no.such` are
/// `UnknownIndicator("no.such")`; `Series` of `gdp.nominal` (national only)
/// at region 0, a breakdown spelling that does not exist, and `Explain` of
/// `people.population` at a county index past the last are `BadKey`;
/// `Series` with `from > to` is `Malformed`; JSON that is not a query or a
/// command fails to parse (an error of `serde_json`, not a panic).
#[test]
fn ac_api_02() {
    let queries = every_query();
    let mut names: Vec<&str> = queries.iter().map(query_name).collect();
    names.dedup();
    assert_eq!(names.len(), 10, "every query variant: {names:?}");
    for q in &queries {
        round_trip(q, query_name(q));
    }
    let commands = every_command();
    let mut names: Vec<&str> = commands.iter().map(command_name).collect();
    names.dedup();
    assert_eq!(names.len(), 5, "every command variant: {names:?}");
    for c in &commands {
        round_trip(c, command_name(c));
    }
    let answers = every_answer();
    let mut names: Vec<&str> = answers.iter().map(answer_name).collect();
    names.dedup();
    assert_eq!(names.len(), 14, "every answer variant: {names:?}");
    for a in &answers {
        round_trip(a, answer_name(a));
    }

    let w = world_2021_default(1000);
    let unknown = "no.such".to_string();
    let errs = [
        (
            Query::Series {
                id: unknown.clone(),
                key: national(),
                from: 0,
                to: 0,
                daily: false,
            },
            "Series of no.such",
        ),
        (
            Query::Keys {
                id: unknown.clone(),
            },
            "Keys of no.such",
        ),
        (
            Query::Explain {
                id: unknown.clone(),
                key: national(),
                from: 0,
                to: 1,
            },
            "Explain of no.such",
        ),
    ];
    for (q, what) in errs {
        assert_eq!(
            w.query(&q),
            Err(ApiError::UnknownIndicator(unknown.clone())),
            "{what}"
        );
    }
    let bad_keys = [
        (
            Query::Series {
                id: "gdp.nominal".to_string(),
                key: key_dto(Key::of(Breakdown::Region, 0)),
                from: 0,
                to: 0,
                daily: false,
            },
            "gdp.nominal at region 0",
        ),
        (
            Query::Series {
                id: "people.population".to_string(),
                key: KeyDto {
                    breakdown: Some("planet".to_string()),
                    index: 0,
                },
                from: 0,
                to: 0,
                daily: false,
            },
            "a breakdown that does not exist",
        ),
        (
            Query::Explain {
                id: "people.population".to_string(),
                key: key_dto(Key::of(Breakdown::County, 10_000)),
                from: 0,
                to: 1,
            },
            "people.population at county 10,000",
        ),
    ];
    for (q, what) in bad_keys {
        assert_eq!(w.query(&q), Err(ApiError::BadKey), "{what}");
    }
    let backwards = Query::Series {
        id: "gdp.nominal".to_string(),
        key: national(),
        from: 3,
        to: 1,
        daily: false,
    };
    assert!(
        matches!(w.query(&backwards), Err(ApiError::Malformed(_))),
        "from > to: {:?}",
        w.query(&backwards)
    );
    for text in ["{", "{\"Series\":3}", "\"NoSuchQuery\"", "[]"] {
        assert!(
            serde_json::from_str::<Query>(text).is_err(),
            "{text}: not a query"
        );
        assert!(
            serde_json::from_str::<Command>(text).is_err(),
            "{text}: not a command"
        );
    }
}

// --- AC-API-03 ------------------------------------------------------------------

/// **AC-API-03** `[sim]` Commands: `StepMonths { 2 }` from day 0 makes two
/// closes and answers the days stepped; an `Order` the lever system accepts
/// is pending after it, one it rejects answers `Rejected` with the lever
/// system's code; `Watch` then `StepMonths { 1 }` gives one watched summary.
///
/// Readings: a 1:1000 world. `StepMonths { 2 }` from day 0 answers
/// `Stepped { days, closes: 2 }` with `days` the days it ran (the world's
/// day index after it, 62: December and January) and two closed months.
/// The accepted order: standard VAT at 21% as soon as allowed, answered
/// `Ordered` with a sequence number and effective date that the pending
/// list holds. The rejected ones, each against `give` of the same order on
/// a copy of the world: VAT at 400% (`out_of_range`) and a lever id that
/// does not exist (`unknown_lever`); neither is pending after. `Watch`
/// answers `Watching(name)`, and after `StepMonths { 1 }` (one close)
/// `watched(name)` has one summary. `Step { days: 5 }` answers five days.
#[test]
fn ac_api_03() {
    use econ_core::levers::Order as LeverOrder;
    let mut w = world_2021_default(1000);
    assert_eq!(w.day(), Day(0));
    let a = tell(&mut w, &Command::StepMonths { months: 2 });
    assert_eq!(w.months().len(), 2, "two closes");
    assert_eq!(w.day(), Day(62), "December and January");
    assert_eq!(
        a,
        Answer::Stepped(SteppedDto {
            days: 62,
            closes: 2
        }),
        "StepMonths 2"
    );

    let a = tell(
        &mut w,
        &Command::Order {
            order: vat_order(2100),
        },
    );
    let Answer::Ordered(o) = a else {
        panic!("Order: another answer: {a:?}")
    };
    let p = w
        .pending()
        .iter()
        .find(|p| p.seq == o.seq)
        .unwrap_or_else(|| panic!("the accepted order {} is pending", o.seq));
    assert_eq!(date_dto(p.effective), o.effective, "its effective date");
    assert_eq!(
        value_dto(&p.command.value),
        ValueDto::Rate(2100),
        "its value"
    );

    let vat = w.registry().find("tax.vat.rate_standard").expect("VAT").id;
    let pending = w.pending().len();
    let direct = w
        .clone()
        .give(LeverOrder::Set(econ_core::levers::Command {
            lever: vat,
            key: None,
            value: LeverValue::Rate(40_000),
            effective: None,
        }))
        .expect_err("the lever system rejects 400%");
    assert_eq!(direct.code, RejectCode::OutOfRange);
    match w.command(&Command::Order {
        order: vat_order(40_000),
    }) {
        Err(ApiError::Rejected(r)) => assert_eq!(r.code, direct.code, "the lever system's code"),
        other => panic!("VAT at 400%: {other:?}"),
    }
    let unknown = OrderDto::Set {
        lever: "tax.nonsense".to_string(),
        key: None,
        value: ValueDto::Rate(100),
        effective: None,
    };
    match w.command(&Command::Order { order: unknown }) {
        Err(ApiError::Rejected(r)) => assert_eq!(r.code, RejectCode::UnknownLever, "unknown lever"),
        other => panic!("an unknown lever: {other:?}"),
    }
    assert_eq!(w.pending().len(), pending, "nothing rejected is pending");

    let f = Filter {
        conditions: vec![Condition::Activity(vec![Activity::Retired])],
    };
    assert_eq!(
        tell(
            &mut w,
            &Command::Watch {
                name: "retired".to_string(),
                filter: f
            }
        ),
        Answer::Watching("retired".to_string())
    );
    let a = tell(&mut w, &Command::StepMonths { months: 1 });
    assert!(
        matches!(a, Answer::Stepped(SteppedDto { closes: 1, .. })),
        "StepMonths 1: {a:?}"
    );
    assert_eq!(w.months().len(), 3);
    assert_eq!(
        w.watched("retired").map(<[GroupSummary]>::len),
        Some(1),
        "one watched summary"
    );

    let day = w.day().0;
    let a = tell(&mut w, &Command::Step { days: 5 });
    assert_eq!(w.day().0, day + 5);
    assert!(
        matches!(a, Answer::Stepped(SteppedDto { days: 5, .. })),
        "Step 5: {a:?}"
    );
}

// --- AC-API-04 ------------------------------------------------------------------

/// Whether `text` holds `word` as a whole identifier.
fn has_word(text: &str, word: &str) -> bool {
    text.match_indices(word).any(|(i, _)| {
        let before = text[..i].chars().next_back();
        let after = text[i + word.len()..].chars().next();
        let ident = |c: Option<char>| c.is_some_and(|c| c.is_ascii_alphanumeric() || c == '_');
        !ident(before) && !ident(after)
    })
}

/// **AC-API-04** `[unit]` `ui/src/generated/api.ts` is in step with the Rust
/// types (`just codegen-check`), and the UI's `api.ts` adapter compiles
/// against it (the CI's `ui` job).
///
/// Readings: the generator and its form are the implementation's. Here:
/// `ui/src/generated/api.ts` exists and names `Query`, `Command`, `Answer`
/// and every variant of the three (each as a whole identifier); every line
/// of the justfile's `codegen-check` recipe runs and succeeds (the recipe's
/// `{{python}}` is the working interpreter), and one of the scripts it runs
/// writes or checks `api.ts` (its source names the file), so that the check
/// covers it. The compile of the adapter is the CI's `ui` job (`tsc`); here
/// `ui/src/api.ts` imports the generated types (names `generated/api`).
#[test]
fn ac_api_04() {
    let path = repo_root().join("ui/src/generated/api.ts");
    let ts = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let mut words: Vec<&str> = vec!["Query", "Command", "Answer"];
    words.extend(every_query().iter().map(query_name));
    words.extend(every_command().iter().map(command_name));
    words.extend(every_answer().iter().map(answer_name));
    for w in words {
        assert!(has_word(&ts, w), "api.ts does not name {w}");
    }

    let adapter = repo_root().join("ui/src/api.ts");
    let adapter = std::fs::read_to_string(&adapter).expect("ui/src/api.ts");
    assert!(
        adapter.contains("generated/api"),
        "ui/src/api.ts imports the generated types"
    );

    let justfile = std::fs::read_to_string(repo_root().join("justfile")).expect("the justfile");
    let mut lines = Vec::new();
    let mut inside = false;
    for line in justfile.lines() {
        if line.starts_with("codegen-check:") {
            inside = true;
            continue;
        }
        if inside {
            if line.starts_with(' ') || line.starts_with('\t') {
                let l = line.trim();
                if !l.is_empty() && !l.starts_with('#') {
                    lines.push(l.to_string());
                }
            } else if !line.trim().is_empty() {
                break;
            }
        }
    }
    assert!(!lines.is_empty(), "the justfile has a codegen-check recipe");
    let mut covers_api = false;
    for l in &lines {
        let words: Vec<String> = l
            .replace("{{python}}", python())
            .split_whitespace()
            .map(str::to_string)
            .collect();
        for w in &words[1..] {
            if w.ends_with(".py") {
                let src = std::fs::read_to_string(repo_root().join(w)).unwrap_or_default();
                covers_api |= src.contains("api.ts");
            }
        }
        let out = std::process::Command::new(&words[0])
            .args(&words[1..])
            .current_dir(repo_root())
            .output()
            .unwrap_or_else(|e| panic!("{l}: {e}"));
        assert!(
            out.status.success(),
            "{l} failed: {}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
    }
    assert!(
        covers_api,
        "no script of codegen-check checks api.ts: {lines:?}"
    );
}

// --- AC-API-05 ------------------------------------------------------------------

/// **AC-API-05** `[sim]` A session through the API alone reproduces a direct
/// run: the same orders and steps through `command` give the same state hash
/// after 12 closes as calling the `World`'s methods.
///
/// Readings: two 1:1000 worlds. The direct one: `give` standard VAT at 21%
/// on day 0, step day by day to the second close, `give` the policy rate
/// 100 basis points above that day's, step to the twelfth close. The API
/// one: `Command::Order` of the same VAT order, `StepMonths { 2 }`,
/// `Command::Order` of the same policy rate, `StepMonths { 10 }`. The state
/// hashes after 12 closes are equal, as are the command logs' lengths and
/// the days stepped.
#[test]
fn ac_api_05() {
    use super::support_lev::{give, set};
    let mut direct = world_2021_default(1000);
    let vat = direct
        .registry()
        .find("tax.vat.rate_standard")
        .expect("VAT")
        .id;
    let rate = direct
        .registry()
        .find("money.policy_rate")
        .expect("the rate")
        .id;
    give(&mut direct, set(vat, None, LeverValue::Rate(2100)));
    run_closes(&mut direct, 2);
    let bp = direct.central_bank().policy_rate_ppm / 100 + 100;
    give(&mut direct, set(rate, None, LeverValue::Rate(bp)));
    while direct.months().len() < 12 {
        step_checked(&mut direct);
    }

    let mut api = world_2021_default(1000);
    tell(
        &mut api,
        &Command::Order {
            order: vat_order(2100),
        },
    );
    tell(&mut api, &Command::StepMonths { months: 2 });
    tell(
        &mut api,
        &Command::Order {
            order: OrderDto::Set {
                lever: "money.policy_rate".to_string(),
                key: None,
                value: ValueDto::Rate(bp),
                effective: None,
            },
        },
    );
    tell(&mut api, &Command::StepMonths { months: 10 });
    assert_eq!(api.months().len(), 12);
    assert_eq!(api.day(), direct.day(), "the days stepped");
    assert_eq!(
        api.command_log().len(),
        direct.command_log().len(),
        "the command logs"
    );
    assert_eq!(
        api.state_hash(),
        direct.state_hash(),
        "the state hash after 12 closes"
    );
}
