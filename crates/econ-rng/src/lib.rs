//! Keyed random numbers (ADR-0006).
//!
//! Every draw is a pure function of `(seed, stream, tick, entity)`:
//!
//! * the ChaCha8 **key** comes from the master seed and a [`Stream`] id
//!   (one per purpose, e.g. job separations, births);
//! * the ChaCha **stream** number is the entity id (person, household, firm);
//! * the **word position** is derived from the tick.
//!
//! So results never depend on the order in which entities are processed or on
//! the number of threads. Distributions are implemented here on top of
//! `econ_num::math` instead of `rand_distr`, so they are pinned too.

use econ_num::math;
use rand_chacha::ChaCha8Rng;
use rand_core::{Rng, SeedableRng};

/// Purpose of a random draw. Append-only: never renumber (saves and golden
/// runs depend on these values).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u32)]
pub enum Stream {
    /// Test-only stream.
    Test = 0,
    /// Births and deaths.
    Demography = 1,
    /// Job separations and hiring order.
    Labour = 2,
    /// Household consumption noise.
    Consumption = 3,
    /// Synthetic population generation.
    PopulationGen = 4,
    /// Firm entry and exit.
    FirmDynamics = 5,
    /// Weather shocks.
    Weather = 6,
}

/// Number of 32-bit words reserved per (tick, entity, stream).
/// A single draw context may consume at most this many words.
pub const WORDS_PER_TICK: u128 = 1 << 20;

/// Factory for keyed generators. Cheap to copy.
#[derive(Debug, Clone, Copy)]
pub struct KeyedRng {
    seed: u64,
}

impl KeyedRng {
    /// Create from the master seed of a save game.
    #[must_use]
    pub fn new(seed: u64) -> Self {
        KeyedRng { seed }
    }

    /// One fast counter-based draw: a pure function of
    /// `(seed, stream, tick, entity, k)` built from SplitMix64 mixing.
    ///
    /// The entity id is multiplied by the odd constant [`GAMMA`] before the
    /// last mix (the "gamma" mixer, ADR-0006 Amendment 1). Without it,
    /// consecutive entity ids at one tick fail a binary-rank test
    /// (`fast_draws_binary_rank_over_consecutive_entities`).
    ///
    /// Use for hot single draws (one Bernoulli per person per tick); it is
    /// ~100× cheaper than setting up a ChaCha context. Use [`KeyedRng::draw`]
    /// when a context needs many draws. Finding of Spike 4 (ADR-0006
    /// amendment proposed).
    #[must_use]
    pub fn fast_u64(&self, stream: Stream, tick: u32, entity: u64, k: u32) -> u64 {
        let mut st = self.seed ^ 0xD1B5_4A32_D192_ED03;
        st = mix64(st ^ u64::from(stream as u32));
        st = mix64(st ^ (u64::from(tick) << 32 | u64::from(k)));
        mix64(st ^ entity.wrapping_mul(GAMMA))
    }

    /// Fast uniform in `[0, 1)` (see [`KeyedRng::fast_u64`]).
    #[must_use]
    pub fn fast_uniform(&self, stream: Stream, tick: u32, entity: u64, k: u32) -> f64 {
        #[allow(clippy::cast_precision_loss)]
        let v = (self.fast_u64(stream, tick, entity, k) >> 11) as f64 * (1.0 / (1u64 << 53) as f64);
        v
    }

    /// Fast Bernoulli draw (see [`KeyedRng::fast_u64`]).
    #[must_use]
    pub fn fast_bernoulli(&self, stream: Stream, tick: u32, entity: u64, k: u32, p: f64) -> bool {
        self.fast_uniform(stream, tick, entity, k) < p.clamp(0.0, 1.0)
    }

    /// The generator for one `(stream, tick, entity)` context.
    #[must_use]
    pub fn draw(&self, stream: Stream, tick: u32, entity: u64) -> Draw {
        let key = derive_key(self.seed, stream as u32);
        let mut rng = ChaCha8Rng::from_seed(key);
        rng.set_stream(entity);
        rng.set_word_pos(u128::from(tick) * WORDS_PER_TICK);
        Draw { rng }
    }
}

/// The SplitMix64 increment (2^64 / golden ratio, odd).
const GAMMA: u64 = 0x9E37_79B9_7F4A_7C15;

/// SplitMix64: a fixed, documented key-derivation step.
fn splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(GAMMA);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// SplitMix64 finalizer (a bijective 64-bit mixer).
fn mix64(z: u64) -> u64 {
    let mut z = z.wrapping_add(GAMMA);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

fn derive_key(seed: u64, stream: u32) -> [u8; 32] {
    let mut st = seed ^ (u64::from(stream) << 32 | u64::from(stream));
    let mut key = [0u8; 32];
    for chunk in key.chunks_exact_mut(8) {
        chunk.copy_from_slice(&splitmix64(&mut st).to_le_bytes());
    }
    key
}

/// A generator positioned at one `(stream, tick, entity)` context.
#[derive(Debug)]
pub struct Draw {
    rng: ChaCha8Rng,
}

impl Draw {
    /// Uniform `u64`.
    pub fn u64(&mut self) -> u64 {
        self.rng.next_u64()
    }

    /// Uniform in `[0, 1)` with 53 random bits.
    pub fn uniform(&mut self) -> f64 {
        #[allow(clippy::cast_precision_loss)]
        let v = (self.u64() >> 11) as f64 * (1.0 / (1u64 << 53) as f64);
        v
    }

    /// `true` with probability `p` (clamped to `[0, 1]`).
    pub fn bernoulli(&mut self, p: f64) -> bool {
        self.uniform() < p.clamp(0.0, 1.0)
    }

    /// Standard normal via Box–Muller on `econ_num::math`.
    pub fn normal(&mut self) -> f64 {
        // 1 - u ∈ (0, 1] avoids ln(0).
        let u1 = 1.0 - self.uniform();
        let u2 = self.uniform();
        math::sqrt(-2.0 * math::ln(u1)) * math::cos(2.0 * std::f64::consts::PI * u2)
    }

    /// Uniform integer in `[0, n)` without modulo bias (rejection sampling).
    ///
    /// # Panics
    /// If `n == 0`.
    pub fn below(&mut self, n: u64) -> u64 {
        assert!(n > 0, "below(0)");
        let zone = u64::MAX - (u64::MAX % n);
        loop {
            let v = self.u64();
            if v < zone {
                return v % n;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Known-answer vectors. A failure after a dependency or toolchain bump is
    /// a re-golden event (record it in CHANGELOG-sim.md).
    #[test]
    fn known_answer_vectors() {
        let r = KeyedRng::new(42);
        let a = r.draw(Stream::Test, 0, 0).u64();
        let b = r.draw(Stream::Test, 7, 123_456).u64();
        let c = r.draw(Stream::Labour, 7, 123_456).u64();
        let got = [a, b, c];
        let expected: [u64; 3] = KAT;
        assert_eq!(got, expected, "KAT changed: {got:#x?}");
    }

    const KAT: [u64; 3] = [
        0x3115_9ef9_87c9_1afc,
        0x7a16_f9e8_d24c_e604,
        0x013c_2607_2112_efa4,
    ];

    #[test]
    fn fast_draws_known_answer_and_moments() {
        let r = KeyedRng::new(42);
        assert_eq!(r.fast_u64(Stream::Test, 7, 123_456, 0), FAST_KAT);
        let n = 100_000u64;
        let mean: f64 = (0..n)
            .map(|e| r.fast_uniform(Stream::Test, 1, e, 0))
            .sum::<f64>()
            / 100_000.0;
        assert!((mean - 0.5).abs() < 0.005);
        let hits = (0..n)
            .filter(|&e| r.fast_bernoulli(Stream::Test, 2, e, 0, 0.015))
            .count();
        assert!((hits as f64 / 100_000.0 - 0.015).abs() < 0.002);
        assert_ne!(
            r.fast_u64(Stream::Test, 7, 1, 0),
            r.fast_u64(Stream::Test, 7, 1, 1)
        );
    }

    const FAST_KAT: u64 = 16_733_015_585_576_584_871;

    /// Rank over GF(2) of the `n`×`n` bit matrix whose rows are consecutive
    /// `n`-bit slices of `words` (64 bits per word, `n` a multiple of 64).
    fn gf2_rank(words: &[u64], n: usize) -> usize {
        let w = n / 64;
        let mut m: Vec<u64> = words[..n * w].to_vec();
        let mut rank = 0;
        for col in 0..n {
            let (cw, cb) = (col / 64, col % 64);
            let Some(piv) = (rank..n).find(|&r| (m[r * w + cw] >> cb) & 1 == 1) else {
                continue;
            };
            if piv != rank {
                for j in 0..w {
                    m.swap(rank * w + j, piv * w + j);
                }
            }
            let (head, tail) = m.split_at_mut((rank + 1) * w);
            let pivot = &head[rank * w..];
            for row in tail.chunks_exact_mut(w) {
                if (row[cw] >> cb) & 1 == 1 {
                    // bits below `col` are already zero in both rows
                    for j in cw..w {
                        row[j] ^= pivot[j];
                    }
                }
            }
            rank += 1;
        }
        rank
    }

    fn entity_rank_deficiency(seed: u64, first_entity: u64) -> usize {
        const N: usize = 8192;
        let r = KeyedRng::new(seed);
        let words: Vec<u64> = (0..(N * N / 64) as u64)
            .map(|e| r.fast_u64(Stream::Labour, 1, first_entity + e, 0))
            .collect();
        N - gf2_rank(&words, N)
    }

    /// Regression test for the entity-only pattern (ADR-0006 Amendment 1,
    /// revised 2026-10-10): the outputs for entity = 0, 1, 2, … at one tick,
    /// laid out as an 8192×8192 bit matrix, must have (almost) full rank.
    /// The mixer without the gamma multiply loses 32–35 here (and fails
    /// PractRand `BRank` at 16 GB); a random matrix loses 0–2.
    #[test]
    fn fast_draws_binary_rank_over_consecutive_entities() {
        let d = entity_rank_deficiency(42, 0);
        assert!(d <= 4, "rank deficiency {d}");
    }

    /// More seeds and a high starting entity (slow in debug builds):
    /// `cargo test --release -p econ-rng -- --ignored`.
    #[test]
    #[ignore = "slow; the default test covers one seed"]
    fn fast_draws_binary_rank_more_seeds() {
        for (seed, first) in [(7u64, 0u64), (42, 1 << 30), (1, 1 << 40)] {
            let d = entity_rank_deficiency(seed, first);
            assert!(
                d <= 4,
                "seed {seed}, first entity {first}: rank deficiency {d}"
            );
        }
    }

    /// Lag-1 correlation of fast uniforms along one input axis, as a z-score.
    fn lag1_z(f: impl Fn(u64) -> f64, n: u64) -> f64 {
        let u: Vec<f64> = (0..n).map(f).collect();
        let m = u.iter().sum::<f64>() / n as f64;
        let (mut num, mut den) = (0.0, 0.0);
        for w in u.windows(2) {
            num += (w[0] - m) * (w[1] - m);
        }
        for x in &u {
            den += (x - m) * (x - m);
        }
        num / den * (n as f64).sqrt()
    }

    /// ADR-0006 amendment acceptance: neighbouring entities, ticks and draw
    /// indices must look independent. Full battery: `python/reference/rng_quality.py`.
    #[test]
    fn fast_draws_neighbours_independent() {
        let r = KeyedRng::new(42);
        let n = 200_000u64;
        let z_entity = lag1_z(|e| r.fast_uniform(Stream::Labour, 7, e, 0), n);
        let z_tick = lag1_z(|t| r.fast_uniform(Stream::Labour, t as u32, 5, 0), n);
        let z_k = lag1_z(|k| r.fast_uniform(Stream::Labour, 3, 5, k as u32), n);
        for (axis, z) in [("entity", z_entity), ("tick", z_tick), ("k", z_k)] {
            assert!(z.abs() < 4.5, "lag-1 correlation along {axis}: z = {z}");
        }
        // Low-probability events (separations) must not cluster on neighbours.
        let p = 0.015;
        let hits: Vec<bool> = (0..n)
            .map(|e| r.fast_bernoulli(Stream::Labour, 9, e, 0, p))
            .collect();
        let pairs = hits.windows(2).filter(|w| w[0] && w[1]).count() as f64;
        let expected = p * p * (n - 1) as f64;
        assert!(
            (pairs - expected).abs() / expected.sqrt() < 4.5,
            "Bernoulli neighbours: {pairs} vs {expected}"
        );
        // Avalanche: flipping one entity bit flips each output bit about half the time.
        for bit in [0u32, 1, 7, 31, 63] {
            let mut flips = [0u32; 64];
            let m = 20_000u64;
            for e in 0..m {
                let d = r.fast_u64(Stream::Labour, 1, e * 2_654_435_761, 0)
                    ^ r.fast_u64(Stream::Labour, 1, (e * 2_654_435_761) ^ (1 << bit), 0);
                for (j, f) in flips.iter_mut().enumerate() {
                    *f += ((d >> j) & 1) as u32;
                }
            }
            for f in flips {
                let pr = f64::from(f) / m as f64;
                assert!((pr - 0.5).abs() < 0.02, "avalanche bit {bit}: {pr}");
            }
        }
    }

    #[test]
    fn order_independent() {
        let r = KeyedRng::new(7);
        let forward: Vec<u64> = (0..100)
            .map(|e| r.draw(Stream::Labour, 3, e).u64())
            .collect();
        let backward: Vec<u64> = (0..100)
            .rev()
            .map(|e| r.draw(Stream::Labour, 3, e).u64())
            .collect();
        let backward: Vec<u64> = backward.into_iter().rev().collect();
        assert_eq!(forward, backward);
    }

    #[test]
    fn contexts_differ() {
        let r = KeyedRng::new(7);
        let base = r.draw(Stream::Labour, 3, 1).u64();
        assert_ne!(base, r.draw(Stream::Labour, 4, 1).u64());
        assert_ne!(base, r.draw(Stream::Labour, 3, 2).u64());
        assert_ne!(base, r.draw(Stream::Demography, 3, 1).u64());
        assert_ne!(base, KeyedRng::new(8).draw(Stream::Labour, 3, 1).u64());
    }

    #[test]
    fn uniform_and_normal_moments() {
        let r = KeyedRng::new(1);
        let n = 20_000u32;
        let mut su = 0.0;
        let mut sn = 0.0;
        let mut sn2 = 0.0;
        for e in 0..n {
            let mut d = r.draw(Stream::Test, 0, u64::from(e));
            su += d.uniform();
            let z = d.normal();
            sn += z;
            sn2 += z * z;
        }
        let nf = f64::from(n);
        assert!((su / nf - 0.5).abs() < 0.01);
        assert!((sn / nf).abs() < 0.03);
        assert!((sn2 / nf - 1.0).abs() < 0.05);
    }
}
