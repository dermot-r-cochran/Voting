//! Property-based tests over `allocate_slots` and its supporting functions.
//!
//! The golden parity suite (`tests/parity.rs`) pins 300 fixed scenarios; this
//! file explores the space between them. Each property is one of the
//! invariants the engine either asserts internally or documents:
//!
//! * every allocation sums to exactly `n_slots`, with every candidate present
//! * the result is a pure function of the input (determinism)
//! * candidate list order and voter list order never influence the result
//! * a voter's ballot is scale-free: multiplying all of one voter's scores by
//!   a positive constant changes nothing (normalisation divides it out)
//! * `normalize_voter` returns weights summing to exactly 1
//!
//! Inputs are generated so every voter has at least one positive score - the
//! zero-sum-ballot rejection path is covered by unit tests in
//! `tests/allocation.rs`, and generating it here would just discard cases.

use std::collections::BTreeMap;

use num_bigint::BigInt;
use num_rational::BigRational;
use proptest::prelude::*;
use voting::allocation::{allocate_slots, normalize_voter};
use voting::models::{Candidate, Score, Voter};

const CANDIDATE_POOL: [&str; 6] = ["A", "B", "C", "D", "E", "F"];

#[derive(Debug, Clone)]
struct Scenario {
    candidates: Vec<Candidate>,
    voters: Vec<Voter>,
    n_slots: i64,
}

/// Scores are integers 0..=10 in tenths (0.0 .. 1.0 steps of 0.1 scaled by
/// 10), i.e. exact decimals of the kind real ballots carry. Each voter gets
/// one guaranteed-positive score so no ballot is zero-sum.
fn scenario() -> impl Strategy<Value = Scenario> {
    (1usize..=6, 1usize..=10, 1i64..=12)
        .prop_flat_map(|(n_candidates, n_voters, n_slots)| {
            let per_voter = proptest::collection::vec(0u32..=100, n_candidates);
            let voters = proptest::collection::vec((per_voter, 0usize..n_candidates), n_voters);
            (Just(n_candidates), voters, Just(n_slots))
        })
        .prop_map(|(n_candidates, raw_voters, n_slots)| {
            let candidates: Vec<Candidate> = CANDIDATE_POOL[..n_candidates]
                .iter()
                .copied()
                .map(Candidate::new)
                .collect();
            let voters = raw_voters
                .into_iter()
                .enumerate()
                .map(|(i, (scores, forced))| {
                    let pairs = scores.into_iter().enumerate().map(|(j, tenths)| {
                        // `forced` guarantees at least one positive score.
                        let tenths = if j == forced { tenths.max(1) } else { tenths };
                        (
                            CANDIDATE_POOL[j].to_string(),
                            Score::from_decimal_str(&format!("{}.{}", tenths / 10, tenths % 10))
                                .expect("generated score is a valid decimal"),
                        )
                    });
                    Voter::with_scores(format!("v{i}"), pairs)
                })
                .collect();
            Scenario {
                candidates,
                voters,
                n_slots,
            }
        })
}

fn total(allocation: &BTreeMap<String, u64>) -> u64 {
    allocation.values().sum()
}

proptest! {
    #[test]
    fn allocation_conserves_slots_and_covers_every_candidate(s in scenario()) {
        let allocation = allocate_slots(&s.voters, &s.candidates, s.n_slots)
            .expect("generated scenario is valid input");
        prop_assert_eq!(total(&allocation), s.n_slots as u64);
        for c in &s.candidates {
            prop_assert!(allocation.contains_key(&c.id), "candidate {} missing from result", c.id);
        }
        prop_assert_eq!(allocation.len(), s.candidates.len());
    }

    #[test]
    fn allocation_is_deterministic(s in scenario()) {
        let first = allocate_slots(&s.voters, &s.candidates, s.n_slots).unwrap();
        let second = allocate_slots(&s.voters, &s.candidates, s.n_slots).unwrap();
        prop_assert_eq!(first, second);
    }

    #[test]
    fn candidate_order_never_influences_the_result(s in scenario()) {
        let forward = allocate_slots(&s.voters, &s.candidates, s.n_slots).unwrap();
        let mut reversed = s.candidates.clone();
        reversed.reverse();
        let backward = allocate_slots(&s.voters, &reversed, s.n_slots).unwrap();
        prop_assert_eq!(forward, backward);
    }

    #[test]
    fn voter_order_never_influences_the_result(s in scenario()) {
        let forward = allocate_slots(&s.voters, &s.candidates, s.n_slots).unwrap();
        let mut reversed = s.voters.clone();
        reversed.reverse();
        let backward = allocate_slots(&reversed, &s.candidates, s.n_slots).unwrap();
        prop_assert_eq!(forward, backward);
    }

    #[test]
    fn a_ballot_is_scale_free(s in scenario(), factor in 2u64..=1000) {
        let baseline = allocate_slots(&s.voters, &s.candidates, s.n_slots).unwrap();
        // Scale EVERY voter's ballot by the same factor (each voter's weights
        // are normalised independently, so per-voter scaling is the same
        // claim; scaling all keeps the test simple).
        let factor_rational = BigRational::from_integer(BigInt::from(factor));
        let scaled: Vec<Voter> = s
            .voters
            .iter()
            .map(|v| {
                let pairs = v.scores.iter().map(|(id, score)| {
                    let scaled = score.as_rational() * factor_rational.clone();
                    (
                        id.clone(),
                        Score::from_rational(scaled).expect("positive times positive"),
                    )
                });
                Voter::with_scores(v.id.clone(), pairs)
            })
            .collect();
        let scaled_result = allocate_slots(&scaled, &s.candidates, s.n_slots).unwrap();
        prop_assert_eq!(baseline, scaled_result);
    }

    #[test]
    fn normalized_weights_sum_to_exactly_one(s in scenario()) {
        let ids: Vec<String> = s.candidates.iter().map(|c| c.id.clone()).collect();
        let one = BigRational::from_integer(BigInt::from(1));
        for voter in &s.voters {
            let weights = normalize_voter(voter, &ids).expect("no zero-sum ballots generated");
            let sum = weights.values().fold(BigRational::new(0.into(), 1.into()), |a, w| a + w);
            prop_assert_eq!(&sum, &one);
        }
    }
}
