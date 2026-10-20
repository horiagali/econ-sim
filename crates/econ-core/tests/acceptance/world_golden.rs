//! Acceptance test for AC-WLD-05 (spec: game/world-loop): the golden state
//! hash of day 31. The value comes from the first implementation
//! (2026-10-17) and was approved by the owner on 2026-10-20 (ADR-0010,
//! CHANGELOG-sim.md); it lives in `tests/golden/world_ro_2021.hashes`.

use econ_types::Day;

use super::support::{repo_root, step, world};

/// The golden file: one line per scale, `<sample_scale> <hash in hex>`.
const GOLDEN: &str = "tests/golden/world_ro_2021.hashes";

/// The committed state hash of day 31 at a scale.
fn golden(scale: u32) -> u64 {
    let path = repo_root().join(GOLDEN);
    let text =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("golden {}: {e}", path.display()));
    let line = text
        .lines()
        .map(str::trim)
        .find(|l| l.split_whitespace().next() == Some(&scale.to_string()))
        .unwrap_or_else(|| panic!("no 1:{scale} line in {GOLDEN}"));
    let hex = line
        .split_whitespace()
        .nth(1)
        .expect("a hash after the scale");
    u64::from_str_radix(hex, 16).expect("a hexadecimal hash")
}

/// **AC-WLD-05** `[golden]` The state hash on day 31 on the Romania scenario
/// at 1:1000, seed 42, equals the committed golden value.
#[test]
fn ac_wld_05() {
    assert_eq!(golden(1000), 0x407f_56fb_8ccd_24ea, "the approved value");
    let mut w = world(1000);
    // 31 days run (days 0 to 30): the world stands on day 31.
    while w.day() < Day(31) {
        step(&mut w);
    }
    assert_eq!(w.day(), Day(31));
    assert_eq!(w.state_hash(), golden(1000), "the state hash on day 31");
}
