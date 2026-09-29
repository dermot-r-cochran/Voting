# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

A deterministic, invariant-preserving proportional allocation engine — a small Rust crate (`voting`, GPL-3.0-or-later). Given voters with graded preferences over candidates, it distributes a fixed number of discrete slots proportionally: normalisation → aggregation → STV quota phase → largest-remainder (Hamilton) phase, with lexicographic tie-breaking by candidate id throughout so the same input always produces the same output.

It is a port of a Python engine that no longer exists in the repo; the Python engine's behaviour survives as the golden test suite (see below).

## Commands

```bash
cargo test                                   # everything: unit + integration + parity + property + doc tests
cargo test --test parity                     # one integration test file (allocation, score, parity, properties)
cargo test surplus_transfer                  # tests whose names match a substring
cargo clippy --all-targets -- -D warnings    # lint; CI fails on any warning
cargo fmt --check                            # formatting; CI enforces
```

CI (`.github/workflows/ci.yml`) runs exactly those three checks — fmt, clippy, test — on every PR and push to `main`. There is **no `Cargo.lock` by design**; the floor-pinned dependencies resolve fresh.

## Architecture

Two modules, re-exported flat from `src/lib.rs`:

- **`src/models.rs`** — `Candidate`, `Group`, `Voter`, and `Score`. `Score` is the crate's guarantee-by-construction layer: it cannot hold a negative value and is an exact rational (`BigRational`) from the moment it exists, so downstream code never re-checks either property. `Score::from_decimal_str("0.1")` is exactly 1/10; `Score::from_f64` goes through the shortest decimal representation, not the binary expansion. All arithmetic in the crate is exact rational, never floating point — surplus transfer multiplies fractions repeatedly, so denominators grow, which is why it's `BigRational` and not `Ratio<i64>`.
- **`src/allocation.rs`** — the four-phase algorithm and the invariants, which are the point of the crate: total normalised weight = number of voters, total allocated slots = `n_slots`, exact surplus conservation wherever a surplus has a recipient (a voter with no remaining preferences has their share absorbed, so weight can shrink in phase 1, never grow — which is why slot conservation is the asserted invariant), deterministic tie-breaking. They are `assert!`, not `debug_assert!`, and `Cargo.toml` sets `debug-assertions = true` for release — do not weaken either; an allocation that silently violated conservation would be worse than one that stopped. All collections are `BTreeMap`/`BTreeSet` so iteration order is a property of the data, not of hashing.

Two things to know before "fixing" anything:

- **The quota is deliberately not classical Droop.** It is `floor(n_voters / n_slots) + 1`, ruled by the author (2026-08-24) as the engine's own formula — settled, not a bug. The comment at the computation site in `allocation.rs` records the ruling.
- **`tests/golden/` is behaviour-as-specification.** `expected.txt` holds the removed Python engine's own outputs on all 300 scenarios in `cases.txt`, and `tests/parity.rs` asserts this implementation still reproduces them. Any change that alters an allocation fails there rather than shipping quietly. An *intentional* behaviour change must regenerate `expected.txt` as a deliberate, reviewed decision in the same PR — never patch individual expected lines to make a red test green.

The other test layers: `tests/allocation.rs` (hand-built per-phase scenarios plus every rejection path) and `tests/score.rs` (the crate's only parsing surface) are the unit/integration layer; `tests/properties.rs` (proptest) searches the space between the golden fixed points for counterexamples to six invariant properties. On a proptest failure, the shrunken counterexample is the bug report — add it to `tests/allocation.rs` as a named regression case before fixing. Full rationale and extension rules: `TestingStrategy.md`.

## Related repositories

The map of Dermot's public repositories and what crosses between them is
`RELATED-REPOSITORIES.md` in `dermot-r-cochran/star-rangers`; this section
names only this repository's own neighbours (added 2026-09-29 at his
direction).

- **`dermot-r-cochran/star-rangers`** describes this crate in public. Its About
  page (*The engineering behind the record*) names Voting as lineage — exact
  arithmetic, conservation asserted at every phase boundary, a loud stop
  preferred to a silent error — and its codex entry *Three Disciplines of the
  Record* mirrors that creed in-world as a seat-allocation protocol. A change
  to the invariants, the quota ruling, a rename or a retirement here makes
  that description false; say so in the pull request and expect a follow-up
  there.
- **`dermot-r-cochran/foundation-model`** has an expertise-weighted voting
  module. It shares a word with this crate and nothing else; neither is a
  reference for the other.
- **Siblings by convention:** the account's engineering repositories all
  carry a `TestingStrategy.md` in the same shape as this one's. This is the
  only Rust repository among them, so the fmt/clippy/test gate has no sibling
  to compare against; the golden-suite idea (behaviour as specification) is
  this crate's own.
