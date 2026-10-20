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
