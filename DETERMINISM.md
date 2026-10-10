# Determinism contract

Same scenario + seed + command log ⇒ bit-identical state on every machine,
every thread count, every run. This is what makes golden tests, replays,
bug reports and calibration possible. Source: [ADR-0006](docs/03-architecture/decisions/0006-determinism-contract.md).

## Rules (enforced where possible)

| # | Rule | Enforced by |
|---|---|---|
| 1 | All transcendental maths (`exp`, `ln`, `pow`, `sin`, …) goes through `econ_num::math` (pinned pure-Rust `libm`) | clippy `disallowed_methods` (deny) |
| 2 | No NaN or infinity may enter state: wrap values with `econ_num::finite(x, "what")` at phase boundaries | review + tests |
| 3 | Float sums over agents use integer `Bani` or `econ_num::det_sum`, never a thread-dependent reduction | clippy + review |
| 4 | Parallel code writes only to disjoint indices, or returns per-chunk partials combined in index order | review |
| 5 | Iterate only over `Vec`, `BTreeMap`, `BTreeSet` or `IndexMap`; never `HashMap`/`HashSet` | clippy `disallowed_types` (deny) |
| 6 | Sort floats with `total_cmp` plus an ID tiebreak | review |
| 7 | 64-bit targets only | CI |
| 8 | Default build flags: no `target-cpu=native`, no fast-math, no `mul_add`; toolchain pinned in `rust-toolchain.toml` | clippy + `rust-toolchain.toml` |
| 9 | Randomness only via `econ_rng::KeyedRng`: `fast_*(stream, tick, entity, k)` for per-agent per-tick draws in hot loops, `draw(stream, tick, entity)` (ChaCha8) for everything else; when unsure, ChaCha8 | clippy bans `thread_rng`, `StdRng`, `SmallRng`; review |
| 10 | No wall-clock time or environment variables in state code | clippy bans `Instant`, `SystemTime`, `env::var` |

`just lint-canary` proves the clippy deny-lists are active.

## Money
Money is `econ_types::Bani(i64)`: checked arithmetic (panics on overflow in
every build), one rounding rule (half away from zero), exact splits by largest
remainder. See [ADR-0007](docs/03-architecture/decisions/0007-money-and-ledger.md).

## Re-golden events
These change results on purpose and must be recorded in `CHANGELOG-sim.md`
with the economic or technical reason:
- bumping `rand_chacha`, `rand_core`, `libm` or the Rust toolchain;
- changing the fast-hash mixer (`fast_u64`, `FAST_KAT`);
- any change to a `Stream` id, `WORDS_PER_TICK` or `det_sum::CHUNK`;
- any change in a behaviour rule or phase order.
