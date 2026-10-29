//! Acceptance tests generated from spec criteria (ADR-0014). Protected path:
//! only a test-writer session (`ECON_TEST_AUTHORING=1`) edits these files.

// The `World` (game/world-loop) and the `[sim]` criteria of the four M2
// mechanics on it, compiled with the crate feature `world`, which the
// implementation adds to `default`.
#[cfg(feature = "world")]
mod households;
#[cfg(feature = "world")]
mod labour;
#[cfg(feature = "world")]
mod prices;
#[cfg(feature = "world")]
mod production;
#[cfg(feature = "world")]
mod support;
#[cfg(feature = "world")]
mod world;
#[cfg(feature = "world")]
mod world_golden;

// The M3 criteria on the `World`, each behind the feature of the test
// writer's additions it reads, which the implementation adds to `default`.
#[cfg(feature = "banking")]
mod banking;
#[cfg(feature = "fiscal")]
mod fiscal;
#[cfg(feature = "monetary")]
mod monetary;
#[cfg(any(
    feature = "pay_day",
    feature = "banking",
    feature = "fiscal",
    feature = "trade"
))]
mod support_m3;
#[cfg(feature = "pay_day")]
mod taxation_world;
#[cfg(feature = "trade")]
mod trade;
#[cfg(feature = "pay_day")]
mod transfers_world;

// M3's investment and revenue (the code came before these tests, by the
// owner's decision of 2026-10-21): each behind the feature its code is
// behind, which the implementation adds to `default` once they pass.
#[cfg(feature = "revenue")]
mod eu_funds;
#[cfg(feature = "revenue")]
mod excise;
#[cfg(feature = "revenue")]
mod imputed_rent;
#[cfg(feature = "investment")]
mod investment;
#[cfg(feature = "revenue")]
mod revenue;
#[cfg(any(feature = "investment", feature = "revenue"))]
mod support_rev;

// M4: the lever system (spec game/lever-system) and the labour market's pay
// and job choice (AC-LM-11, AC-LM-12), written before the code against the
// API skeleton `econ_core::levers` and the test writer's additions, behind
// the feature `levers`, which the implementation adds to `default`.
#[cfg(feature = "levers")]
mod labour_pay;
#[cfg(feature = "levers")]
mod levers;
#[cfg(feature = "levers")]
mod levers_sim;
#[cfg(feature = "levers")]
mod support_lev;

// M5: demographics and the household life cycle (specs economy/demographics,
// society/households-lifecycle), written before the code against the test
// writer's additions, behind the features `demography` and `lifecycle`,
// which the implementation adds to `default`. Runs of years are
// release-only (ignored under debug_assertions, the form of AC-WLD-10).
#[cfg(feature = "demography")]
mod demography;
#[cfg(feature = "lifecycle")]
mod households_lifecycle;
#[cfg(feature = "demography")]
mod support_dem;

// M4: the policy-scenario suite (spec game/policy-scenarios, AC-POL-01 to
// 11), behind the feature `policy`; the scenarios whose lever is not wired
// yet are each behind their lever's feature as well (see `policy/mod.rs`).
// Its tests run in release builds only (ignored under debug_assertions).
#[cfg(feature = "policy")]
mod policy;

// M5: migration, education, approval and poverty (specs
// economy/migration-remittances, society/education, society/opinion-approval,
// society/poverty-inequality), written before the code against the test
// writer's API skeletons, each behind its feature, which the implementation
// adds to `default`. Long runs are release-only (ignored under
// debug_assertions, the form of AC-WLD-10).
#[cfg(feature = "approval")]
mod approval;
#[cfg(feature = "education")]
mod education;
#[cfg(feature = "migration")]
mod migration;
#[cfg(feature = "poverty")]
mod poverty;
#[cfg(any(
    feature = "migration",
    feature = "education",
    feature = "approval",
    feature = "poverty"
))]
mod support_m5;
