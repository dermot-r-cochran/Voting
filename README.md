# Voting

A deterministic, invariant-preserving proportional allocation engine, in Rust.

Given voters with graded preferences over a set of candidates, it distributes a
fixed number of discrete slots between them — proportionally, reproducibly, and
without ever creating or destroying a slot along the way.

## The algorithm

1. **Normalisation** — each voter's raw scores become fractional weights that
   sum to exactly 1. A voter who scores every candidate zero is rejected rather
   than silently ignored. (`tests/allocation.rs::basic_normalization`,
   `tests/allocation.rs::zero_sum_ballot_is_rejected`)
2. **Aggregation** — normalised weights are summed per candidate, so the grand
   total equals the number of voters.
   (`tests/allocation.rs::conservation_invariant`)
3. **STV quota allocation** — a quota of `floor(voters / slots) + 1` — this
   engine's own formula, deliberately not the classical Droop quota — is
   computed; any candidate reaching it takes a slot, and its surplus is
   redistributed proportionally across each voter's remaining candidates.
   (`tests/allocation.rs::exact_quota_threshold`,
   `tests/allocation.rs::surplus_transfer_conserves_slots`)
4. **Largest remainder** — slots that quota allocation could not fill are
   distributed by the Hamilton method.
   (`tests/allocation.rs::no_slots_created_or_destroyed_by_rounding`)

Ties break lexicographically by candidate id at every stage, so the same input
always produces the same output (`tests/allocation.rs::tie_broken_by_id`).
Position in the input list never influences the result: ordering is by
accumulated weight alone (*open ordering*,
`tests/allocation.rs::open_ordering_within_group`).

Each claim above names the test that proves it, as `file::test`; the same
convention runs through the rest of this README, and `tests/docs.rs` fails if
a named test does not exist.

## The invariants

These are the point of the engine. The first three are asserted at every
phase boundary rather than assumed; the fourth is tested:

- total normalised voter weight equals the number of voters
  (`tests/allocation.rs::conservation_invariant`,
  `tests/properties.rs::normalized_weights_sum_to_exactly_one`)
- total allocated slots equals the number requested
  (`tests/allocation.rs::total_slots_conservation_various_n`,
  `tests/properties.rs::allocation_conserves_slots_and_covers_every_candidate`)
- every surplus transfer conserves total weight exactly, counting surplus
  absorbed where it has no recipient (a voter with no remaining preferences)
  (`tests/allocation.rs::surplus_absorbed_when_only_preference_wins`, which
  reaches its result only if the assert held after each transfer)
- tie-breaking is deterministic, tested by properties 2–4 rather than
  asserted (`tests/properties.rs::allocation_is_deterministic`,
  `tests/properties.rs::candidate_order_never_influences_the_result`,
  `tests/properties.rs::voter_order_never_influences_the_result`)

The asserts are `assert!`, not `debug_assert!`, and `Cargo.toml` keeps
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

`Score::from_decimal_str("0.1")` is exactly one tenth
(`tests/score.rs::decimal_str_parses_integers_and_decimals_exactly`).
`Score::from_f64` routes through the shortest decimal representation, so
ballots held as floats convert the way a reader expects rather than the way
binary doubles do
(`tests/score.rs::from_f64_uses_the_shortest_decimal_not_the_binary_expansion`).
The negative-value rejection is
`tests/score.rs::from_rational_rejects_negatives`.

## Building and testing

```bash
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

CI runs all three on every pull request. `cargo test` also runs
`tests/docs.rs`, a standard-library doc check that fails if a relative link in
this README or `docs/` does not resolve, a Markdown file carries two
front-matter blocks, a count stated here (the 300 scenarios, the six
properties) drifts from the tree, or a test cited above does not exist.

## History

This engine was originally written in Python. The Rust port landed in #3, and
the Python implementation was removed in the change that added this README —
the two produced identical allocations across 300 scenarios at the moment of
removal.

That comparison is preserved rather than discarded. `tests/golden/` holds the
scenarios together with the **Python engine's own outputs**, and
`tests/parity.rs::matches_the_python_engine_case_for_case` asserts this
implementation still reproduces them. The behaviour therefore outlived the
implementation, and any future change that alters an allocation fails there
rather than shipping quietly.

## Licence

GPL-3.0-or-later. See [`LICENSE`](./LICENSE).
