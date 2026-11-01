//! Acceptance tests generated from spec criteria (ADR-0014). Protected path:
//! only a test-writer session edits these files.

// M7: the calibration report (spec game/calibration, AC-CAL-04 to 06 and
// 08), written before the code against the test writer's skeleton of
// `econ-cli calibrate`, behind the feature `calibration`, which the
// implementation adds to the CI's builds once they pass. The runs of years
// are release-only (ignored under debug_assertions, the form of AC-WLD-10).
#[cfg(feature = "calibration")]
mod calibrate;
