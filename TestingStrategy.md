# Testing Strategy

How this crate is tested and why each layer exists. The one-line version: the
invariants are the product, so they are enforced three ways — asserted at
runtime, pinned at 300 fixed points, and searched for counterexamples at
random.

## The governing principle

An allocation engine that silently violated conservation would be worse than
one that stopped. So the invariants declared in `src/allocation.rs`'s module
docs — total normalised weight = number of voters, total allocated slots =
`n_slots`, exact surplus conservation, deterministic tie-breaking — are treated
as the specification, and every layer below checks them from a different angle.

## Layer 0 — runtime asserts that cannot be compiled away

The invariant checks in `allocation.rs` are `assert!`, not `debug_assert!`,
and `Cargo.toml` keeps `debug-assertions = true` in release so a downstream
build profile cannot optimise them out. Tests verify behaviour at development
time; these verify it on every real run.

## Layer 1 — unit and integration tests (`tests/allocation.rs`, `tests/score.rs`)

- `tests/allocation.rs`: hand-built scenarios for each algorithm phase —
  normalisation, aggregation, quota election, surplus transfer, largest
  remainder, group weights, open ordering — plus every rejection path
  (`NonPositiveSlots`, `EmptyVoters`, `EmptyCandidates`, `ZeroSumBallot`).
- `tests/score.rs`: the crate's only parsing surface, tested directly, because
  a parsing bug silently mis-weights a ballot rather than failing anywhere
  visible. Covers `from_decimal_str` exactness (`"0.1"` → exactly 1/10),
  lopsided forms (`"1."`, `".5"`), the rejection table, negative zero,
  `from_f64`'s shortest-decimal contract and its exponent-expansion path
  (`1e300`, `1e-30`), and the error `Display` strings.

## Layer 2 — the golden parity suite (`tests/parity.rs`)

`tests/golden/expected.txt` is the Python engine's own output on all 300
scenarios, captured 2026-07-30 immediately before that engine was deleted. It
is **behaviour-as-specification**: the agreement survives the implementation,
and any change that alters an allocation fails here rather than silently
shipping.

The consequence cuts both ways and is the suite's whole point: **a change that
intentionally alters allocations must regenerate `expected.txt` as a
deliberate, reviewed decision** — never patch individual expected lines to
make a red test green.

One standing example: the quota is `floor(n_voters / n_slots) + 1`, which is
*not* the classical Droop quota (`floor(votes / (seats + 1)) + 1`) — the
divisor differs, so phase 1 elects less eagerly. That is the Python original's
behaviour and the golden suite pins it; the comment at the computation site in
`allocation.rs` records exactly this. Switching to classical Droop would be a
behavioural decision, not a correction.

## Layer 3 — property-based tests (`tests/properties.rs`, proptest)

The golden suite pins fixed points; proptest searches the space between them
(1–6 candidates, 1–10 voters, 1–12 slots, tenth-step decimal scores, every
ballot kept non-zero-sum by construction). Six properties, ~256 random cases
each per run:

1. allocation sums to exactly `n_slots`, every candidate present in the result
2. determinism — same input, same output
3. candidate-list order never influences the result
4. voter-list order never influences the result
5. ballot scale-freeness — multiplying scores by a positive constant changes
   nothing, because normalisation divides it out
6. `normalize_voter` weights sum to exactly 1

On a proptest failure, the shrunken counterexample it prints is the bug
report; add it to `tests/allocation.rs` as a named regression case before
fixing.

## CI (`.github/workflows/ci.yml`)

`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`
(unit + doc + parity + properties) on every PR and push to `main`. There is no
lockfile by design; the floor-pinned dependencies resolve fresh.

## Extending

- New algorithm behaviour → a hand-built case in `tests/allocation.rs` *and*
  ask whether an existing property should now be stronger.
- New input surface → direct tests beside `tests/score.rs`'s pattern: the
  happy path, the rejection table, and the boundary values.
- Intentional behaviour change → regenerate the golden expectations in the
  same PR, with the reasoning in the commit message.
