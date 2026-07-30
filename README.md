# Voting

A deterministic, invariant-preserving proportional allocation engine, in Rust.

Given voters with graded preferences over a set of candidates, it distributes a
fixed number of discrete slots between them — proportionally, reproducibly, and
without ever creating or destroying a slot along the way.

## The algorithm

1. **Normalisation** — each voter's raw scores become fractional weights that
   sum to exactly 1. A voter who scores every candidate zero is rejected rather
   than silently ignored.
2. **Aggregation** — normalised weights are summed per candidate, so the grand
   total equals the number of voters.
3. **STV quota allocation** — a Droop quota, `floor(voters / slots) + 1`, is
   computed; any candidate reaching it takes a slot, and its surplus is
   redistributed proportionally across each voter's remaining candidates.
4. **Largest remainder** — slots that quota allocation could not fill are
   distributed by the Hamilton method.

Ties break lexicographically by candidate id at every stage, so the same input
always produces the same output. Position in the input list never influences
the result: ordering is by accumulated weight alone (*open ordering*).

## The invariants

These are the point of the engine, and they are asserted at every phase
boundary rather than assumed:

- total normalised voter weight equals the number of voters
- total allocated slots equals the number requested
- every surplus transfer conserves total weight exactly
- tie-breaking is deterministic

They are `assert!`, not `debug_assert!`, and `Cargo.toml` keeps
`debug-assertions` enabled in release builds. An allocation that silently
violated conservation would be worse than one that stopped.

## Exactness

All arithmetic is exact rational (`BigRational`), never floating point.
Surplus transfer multiplies fractions repeatedly, so denominators grow — a
fixed-width numerator would overflow precisely where the arithmetic is meant to
be exact.

A `Score` cannot hold a negative value, and is rational by construction:

```rust
use voting::{allocate_slots, Candidate, Score, Voter};

let voters = vec![
    Voter::with_scores("v1", [("A", Score::from_int(3)), ("B", Score::from_int(1))]),
    Voter::with_scores("v2", [("A", Score::from_int(1)), ("B", Score::from_int(3))]),
];
let candidates = vec![Candidate::new("A"), Candidate::new("B")];

let result = allocate_slots(&voters, &candidates, 2)?;
assert_eq!(result.values().sum::<u64>(), 2);
```

`Score::from_decimal_str("0.1")` is exactly one tenth. `Score::from_f64` routes
through the shortest decimal representation, so ballots held as floats convert
the way a reader expects rather than the way binary doubles do.

## Building and testing

```bash
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

CI runs all three on every pull request.

## History

This engine was originally written in Python. The Rust port landed in #3, and
the Python implementation was removed in the change that added this README —
the two produced identical allocations across 300 scenarios at the moment of
removal.

That comparison is preserved rather than discarded. `tests/golden/` holds the
scenarios together with the **Python engine's own outputs**, and
`tests/parity.rs` asserts this implementation still reproduces them. The
behaviour therefore outlived the implementation, and any future change that
alters an allocation fails there rather than shipping quietly.

## Licence

GPL-3.0-or-later. See [`LICENSE`](./LICENSE).
