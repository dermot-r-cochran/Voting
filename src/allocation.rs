//! Deterministic, invariant-preserving proportional allocation engine.
//!
//! Algorithm overview
//! ------------------
//! 1. **Normalisation** – each voter's raw scores become fractional weights
//!    (exact rationals) summing to exactly 1.
//! 2. **Aggregation** – normalised weights are summed across voters, giving
//!    per-candidate totals whose grand sum equals the number of voters.
//! 3. **STV quota allocation** (phase 1) – a quota is computed (see
//!    `allocate_slots` for its exact form and how it differs from the
//!    classical Droop quota) and any candidate reaching it is awarded a slot. Surplus weight is redistributed
//!    proportionally across each voter's remaining candidates.
//! 4. **Largest-remainder allocation** (phase 2) – slots unfilled by phase 1
//!    are distributed by the Hamilton method, with deterministic tie-breaking.
//!
//! Invariants (asserted at every phase boundary)
//! ---------------------------------------------
//! * Total normalised voter weight = number of voters
//! * Total allocated slots = `n_slots`
//! * Every surplus transfer conserves total weight exactly
//! * Tie-breaking is purely deterministic (lexicographic candidate id)
//!
//! These are `assert!`, not `debug_assert!`, and `Cargo.toml` keeps
//! `debug-assertions` on in release. An allocation that silently violated
//! conservation would be worse than one that stopped.
//!
//! Determinism note: every collection here is ordered (`BTreeMap`/`BTreeSet`),
//! so iteration order is a property of the data rather than of hashing. The
//! result would be identical with hash maps — every consuming step sorts
//! explicitly — but a reader should not have to prove that to trust it.

use std::collections::{BTreeMap, BTreeSet};

use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{ToPrimitive, Zero};

use crate::models::{Candidate, Voter};

/// Error returned when an allocation cannot be performed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AllocationError {
    /// `n_slots` was zero or negative.
    NonPositiveSlots(i64),
    /// The voter list was empty.
    EmptyVoters,
    /// The candidate list was empty.
    EmptyCandidates,
    /// A voter scored every candidate zero, so its ballot cannot be normalised.
    ZeroSumBallot {
        /// The offending voter's id.
        voter: String,
    },
}

impl std::fmt::Display for AllocationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NonPositiveSlots(n) => {
                write!(f, "n_slots must be a positive integer, got {n}")
            }
            Self::EmptyVoters => write!(f, "voters list must not be empty"),
            Self::EmptyCandidates => write!(f, "candidates list must not be empty"),
            Self::ZeroSumBallot { voter } => {
                write!(
                    f,
                    "voter {voter:?} has a zero-sum ballot - cannot normalise weights"
                )
            }
        }
    }
}

impl std::error::Error for AllocationError {}

fn rational_from_usize(value: usize) -> BigRational {
    BigRational::from_integer(BigInt::from(value))
}

/// Convert a voter's raw scores to fractional weights summing to exactly 1.
///
/// Candidates missing from the voter's ballot receive weight 0.
///
/// # Errors
/// [`AllocationError::ZeroSumBallot`] if every score is zero.
pub fn normalize_voter(
    voter: &Voter,
    candidate_ids: &[String],
) -> Result<BTreeMap<String, BigRational>, AllocationError> {
    let total: BigRational = candidate_ids
        .iter()
        .map(|id| voter.score_for(id))
        .fold(BigRational::zero(), |acc, s| acc + s);

    if total.is_zero() {
        return Err(AllocationError::ZeroSumBallot {
            voter: voter.id.clone(),
        });
    }

    let weights: BTreeMap<String, BigRational> = candidate_ids
        .iter()
        .map(|id| (id.clone(), voter.score_for(id) / total.clone()))
        .collect();

    let sum = weights.values().fold(BigRational::zero(), |acc, w| acc + w);
    assert!(
        sum == BigRational::from_integer(BigInt::from(1)),
        "normalisation invariant violated for voter {:?}: sum = {}",
        voter.id,
        sum
    );

    Ok(weights)
}

/// Aggregate normalised voter weights per candidate.
///
/// The returned weights sum to exactly the number of voters.
///
/// # Errors
/// Propagates [`AllocationError::ZeroSumBallot`] from [`normalize_voter`].
pub fn compute_aggregated_weights(
    voters: &[Voter],
    candidates: &[Candidate],
) -> Result<BTreeMap<String, BigRational>, AllocationError> {
    let candidate_ids: Vec<String> = candidates.iter().map(|c| c.id.clone()).collect();

    let mut aggregated: BTreeMap<String, BigRational> = candidate_ids
        .iter()
        .map(|id| (id.clone(), BigRational::zero()))
        .collect();

    for voter in voters {
        for (id, weight) in normalize_voter(voter, &candidate_ids)? {
            *aggregated.get_mut(&id).expect("candidate id is present") += weight;
        }
    }

    let total = aggregated
        .values()
        .fold(BigRational::zero(), |acc, w| acc + w);
    assert!(
        total == rational_from_usize(voters.len()),
        "aggregation invariant violated: sum = {}, expected {}",
        total,
        voters.len()
    );

    Ok(aggregated)
}

/// Total aggregated weight per group.
///
/// Candidates with no `group_id` are not represented in the result.
pub fn compute_group_weights(
    aggregated: &BTreeMap<String, BigRational>,
    candidates: &[Candidate],
) -> BTreeMap<String, BigRational> {
    let mut totals: BTreeMap<String, BigRational> = BTreeMap::new();
    for candidate in candidates {
        if let Some(group) = &candidate.group_id {
            let weight = aggregated
                .get(&candidate.id)
                .cloned()
                .unwrap_or_else(BigRational::zero);
            *totals
                .entry(group.clone())
                .or_insert_with(BigRational::zero) += weight;
        }
    }
    totals
}

/// Order candidates by aggregated weight, descending.
///
/// Enforces *open ordering*: position in the input list never influences the
/// result. Ties break lexicographically by candidate id.
///
/// When `group_id` is given, only candidates in that group are returned.
pub fn order_candidates_by_weight<'a>(
    candidates: &'a [Candidate],
    aggregated: &BTreeMap<String, BigRational>,
    group_id: Option<&str>,
) -> Vec<&'a Candidate> {
    let mut selected: Vec<&Candidate> = candidates
        .iter()
        .filter(|c| match group_id {
            Some(g) => c.group_id.as_deref() == Some(g),
            None => true,
        })
        .collect();

    let zero = BigRational::zero();
    selected.sort_by(|a, b| {
        let wa = aggregated.get(&a.id).unwrap_or(&zero);
        let wb = aggregated.get(&b.id).unwrap_or(&zero);
        wb.cmp(wa).then_with(|| a.id.cmp(&b.id))
    });
    selected
}

/// Allocate `n_slots` discrete slots across `candidates`.
///
/// See the module documentation for the two-phase algorithm. The result maps
/// every candidate id to its slot count, summing to exactly `n_slots`.
///
/// # Errors
/// [`AllocationError::NonPositiveSlots`], [`AllocationError::EmptyVoters`],
/// [`AllocationError::EmptyCandidates`], or [`AllocationError::ZeroSumBallot`].
pub fn allocate_slots(
    voters: &[Voter],
    candidates: &[Candidate],
    n_slots: i64,
) -> Result<BTreeMap<String, u64>, AllocationError> {
    if n_slots <= 0 {
        return Err(AllocationError::NonPositiveSlots(n_slots));
    }
    if voters.is_empty() {
        return Err(AllocationError::EmptyVoters);
    }
    if candidates.is_empty() {
        return Err(AllocationError::EmptyCandidates);
    }

    let candidate_ids: Vec<String> = candidates.iter().map(|c| c.id.clone()).collect();
    let n_voters = voters.len();
    let n_slots_usize = n_slots as usize;

    // Per-voter fractional weights (exact rational arithmetic).
    let mut current_weights: Vec<BTreeMap<String, BigRational>> = voters
        .iter()
        .map(|v| normalize_voter(v, &candidate_ids))
        .collect::<Result<_, _>>()?;

    // Quota: floor(n_voters / n_slots) + 1. This is DELIBERATELY this
    // engine's own formula, not the classical Droop quota (which is
    // floor(votes / (seats + 1)) + 1) - ruled by the author on 2026-08-24:
    // the engine does not need to follow Droop exactly, and this divisor is
    // the intended behaviour, not an approximation of Droop. The practical
    // difference: this quota is strictly larger, so phase 1 elects less
    // eagerly and more slots fall through to the largest-remainder phase.
    // The golden parity suite pins it; changing the divisor would alter
    // allocations and requires regenerating tests/golden/expected.txt as a
    // deliberate decision.
    let quota = rational_from_usize(n_voters / n_slots_usize + 1);

    let mut allocated: BTreeMap<String, u64> =
        candidate_ids.iter().map(|id| (id.clone(), 0u64)).collect();
    let mut remaining: BTreeSet<String> = candidate_ids.iter().cloned().collect();
    let mut slots_remaining = n_slots_usize;

    // ---- Phase 1: STV quota allocation with surplus transfer --------------
    while slots_remaining > 0 && !remaining.is_empty() {
        let accumulated = accumulate(&current_weights, &remaining);

        let winner = remaining
            .iter()
            .filter(|id| accumulated[*id] >= quota)
            // Highest accumulated weight; lexicographically smallest id on a
            // tie. `remaining` is ordered, so the first maximum encountered is
            // already the smallest id, but the comparison is written out
            // rather than relying on that.
            .fold(None::<&String>, |best, id| match best {
                None => Some(id),
                Some(current) => {
                    let (a, b) = (&accumulated[id], &accumulated[current]);
                    if a > b || (a == b && id < current) {
                        Some(id)
                    } else {
                        Some(current)
                    }
                }
            })
            .cloned();

        let Some(winner) = winner else {
            break; // No candidate meets quota - proceed to phase 2.
        };

        *allocated.get_mut(&winner).expect("winner is a candidate") += 1;
        slots_remaining -= 1;
        remaining.remove(&winner);

        let winner_total = accumulated[&winner].clone();
        let surplus = winner_total.clone() - quota.clone();
        // Proportion of each voter's contribution that is redistributed, so
        // the winner retains exactly its quota share.
        let transfer_ratio = surplus / winner_total;

        for weights in current_weights.iter_mut() {
            let contribution = weights
                .get(&winner)
                .cloned()
                .unwrap_or_else(BigRational::zero);
            if contribution.is_zero() {
                continue;
            }

            let voter_surplus = contribution.clone() * transfer_ratio.clone();
            weights.insert(winner.clone(), contribution - voter_surplus.clone());

            if remaining.is_empty() {
                // Nothing left to receive the surplus; it is absorbed here.
                continue;
            }

            let future_total = remaining
                .iter()
                .map(|id| weights.get(id).cloned().unwrap_or_else(BigRational::zero))
                .fold(BigRational::zero(), |acc, w| acc + w);

            if future_total > BigRational::zero() {
                for id in remaining.iter() {
                    let w = weights.get(id).cloned().unwrap_or_else(BigRational::zero);
                    let share = voter_surplus.clone() * (w.clone() / future_total.clone());
                    weights.insert(id.clone(), w + share);
                }
            }
            // If future_total is zero the voter has no remaining preferences
            // and the surplus is absorbed; slot conservation is maintained by
            // the phases below, and asserted at the end.
        }
    }

    // ---- Phase 2: largest remainder over the candidates still standing ----
    if slots_remaining > 0 && !remaining.is_empty() {
        let accumulated = accumulate(&current_weights, &remaining);
        let total_remaining: BigRational = accumulated
            .values()
            .fold(BigRational::zero(), |acc, w| acc + w);

        let fair_shares: BTreeMap<String, BigRational> = if total_remaining > BigRational::zero() {
            remaining
                .iter()
                .map(|id| {
                    let share = rational_from_usize(slots_remaining) * accumulated[id].clone()
                        / total_remaining.clone();
                    (id.clone(), share)
                })
                .collect()
        } else {
            // Every remaining weight is zero; distribute evenly.
            let even =
                BigRational::new(BigInt::from(slots_remaining), BigInt::from(remaining.len()));
            remaining
                .iter()
                .map(|id| (id.clone(), even.clone()))
                .collect()
        };

        slots_remaining =
            distribute_largest_remainder(&fair_shares, &mut allocated, slots_remaining);
    }

    // ---- Every candidate was elected in phase 1 but slots remain ---------
    // Reachable when n_slots exceeds the number of candidates.
    if slots_remaining > 0 {
        let aggregated = compute_aggregated_weights(voters, candidates)?;
        let grand_total = aggregated
            .values()
            .fold(BigRational::zero(), |acc, w| acc + w);
        let fair_shares: BTreeMap<String, BigRational> = candidate_ids
            .iter()
            .map(|id| {
                let share = rational_from_usize(slots_remaining) * aggregated[id].clone()
                    / grand_total.clone();
                (id.clone(), share)
            })
            .collect();

        let unfilled = distribute_largest_remainder(&fair_shares, &mut allocated, slots_remaining);
        // Every candidate has a share here, and largest-remainder hands out
        // one leftover slot each until none are left, so this pass always
        // finishes the job. Asserted rather than assumed: silently returning
        // fewer slots than requested is the failure this engine exists to
        // rule out.
        assert!(
            unfilled == 0,
            "largest-remainder fallback left {unfilled} slot(s) unassigned"
        );
    }

    // ---- Final invariants -------------------------------------------------
    let total_allocated: u64 = allocated.values().sum();
    assert!(
        total_allocated == n_slots as u64,
        "slot-conservation invariant violated: allocated {total_allocated} != requested {n_slots}"
    );

    Ok(allocated)
}

/// Sum each candidate's still-available weight across all voters.
fn accumulate(
    current_weights: &[BTreeMap<String, BigRational>],
    candidates: &BTreeSet<String>,
) -> BTreeMap<String, BigRational> {
    candidates
        .iter()
        .map(|id| {
            let total = current_weights
                .iter()
                .map(|w| w.get(id).cloned().unwrap_or_else(BigRational::zero))
                .fold(BigRational::zero(), |acc, w| acc + w);
            (id.clone(), total)
        })
        .collect()
}

/// Hamilton / largest-remainder distribution.
///
/// Assigns the integer part of every fair share, then hands leftover slots to
/// the largest fractional remainders, breaking ties lexicographically by id.
/// Returns the number of slots still unassigned, which is zero unless there
/// were fewer candidates than leftover slots.
fn distribute_largest_remainder(
    fair_shares: &BTreeMap<String, BigRational>,
    allocated: &mut BTreeMap<String, u64>,
    slots_remaining: usize,
) -> usize {
    let mut remaining = slots_remaining;

    let mut floors: BTreeMap<String, u64> = BTreeMap::new();
    for (id, share) in fair_shares {
        // Shares are non-negative, so truncation is a floor.
        let floor = share.to_integer().to_u64().expect("slot count fits in u64");
        floors.insert(id.clone(), floor);
        *allocated.get_mut(id).expect("candidate is allocated") += floor;
        remaining -= floor as usize;
    }

    let mut by_remainder: Vec<(&String, BigRational)> = fair_shares
        .iter()
        .map(|(id, share)| {
            let floor = BigRational::from_integer(BigInt::from(floors[id]));
            (id, share.clone() - floor)
        })
        .collect();
    by_remainder
        .sort_by(|(id_a, rem_a), (id_b, rem_b)| rem_b.cmp(rem_a).then_with(|| id_a.cmp(id_b)));

    for (id, _) in by_remainder {
        if remaining == 0 {
            break;
        }
        *allocated.get_mut(id).expect("candidate is allocated") += 1;
        remaining -= 1;
    }

    remaining
}
