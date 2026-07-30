//! Voting - deterministic proportional allocation engine.
//!
//! A port of this repository's Python engine. The algorithm is unchanged: raw
//! ballot scores are normalised to exact fractional weights, aggregated, then
//! converted into discrete slots by a Droop-quota STV phase followed by a
//! largest-remainder phase, with lexicographic tie-breaking throughout.
//!
//! What the port changes is where the guarantees live. In Python, "scores are
//! non-negative" was a check run after construction, and exactness depended on
//! remembering to write `Fraction(str(score))`. Here a [`models::Score`]
//! cannot hold a negative value and is rational by construction, so those two
//! properties hold for every value in the program rather than at the points
//! someone remembered to assert them.
//!
//! What it does not change: the totals. "Normalised weights sum to one" and
//! "allocated slots sum to `n_slots`" are facts about arithmetic over a whole
//! collection, not about any single value, so they remain runtime assertions -
//! see [`allocation`].
//!
//! ```
//! use voting::models::{Candidate, Score, Voter};
//! use voting::allocation::allocate_slots;
//!
//! let voters = vec![
//!     Voter::with_scores("v1", [("A", Score::from_int(3)), ("B", Score::from_int(1))]),
//!     Voter::with_scores("v2", [("A", Score::from_int(1)), ("B", Score::from_int(3))]),
//! ];
//! let candidates = vec![Candidate::new("A"), Candidate::new("B")];
//!
//! let result = allocate_slots(&voters, &candidates, 2)?;
//! assert_eq!(result.values().sum::<u64>(), 2);
//! # Ok::<(), voting::allocation::AllocationError>(())
//! ```

pub mod allocation;
pub mod models;

pub use allocation::{
    allocate_slots, compute_aggregated_weights, compute_group_weights, normalize_voter,
    order_candidates_by_weight, AllocationError,
};
pub use models::{Candidate, Group, Score, ScoreError, Voter};
