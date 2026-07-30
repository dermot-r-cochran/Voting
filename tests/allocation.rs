//! Tests for the proportional allocation engine.
//!
//! A port of `tests/test_allocation.py`, case for case, so the two
//! implementations can be compared on identical scenarios:
//!
//! * Conservation of total weight (normalisation invariant)
//! * Conservation of allocated slots (slot invariant)
//! * Tie cases (equal weights must not violate global totals)
//! * Exact threshold cases (accumulated weight == quota)
//! * Multi-group scenarios
//! * Surplus-transfer weight conservation
//! * Determinism (same input → same output)
//! * Edge cases: single candidate, zero-sum ballot, invalid n_slots

use std::collections::BTreeMap;

use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::Zero;

use voting::allocation::{
    allocate_slots, compute_aggregated_weights, compute_group_weights, normalize_voter,
    order_candidates_by_weight, AllocationError,
};
use voting::models::{Candidate, Score, Voter};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn frac(numerator: i64, denominator: i64) -> BigRational {
    BigRational::new(BigInt::from(numerator), BigInt::from(denominator))
}

fn whole(value: i64) -> BigRational {
    BigRational::from_integer(BigInt::from(value))
}

/// A voter from `(candidate, score)` pairs given as floats, matching the
/// Python tests' ballots exactly.
fn voter(id: &str, scores: &[(&str, f64)]) -> Voter {
    Voter::with_f64_scores(id, scores.iter().map(|(c, s)| (*c, *s)))
        .expect("test ballots are finite and non-negative")
}

fn ids(names: &[&str]) -> Vec<String> {
    names.iter().map(|s| s.to_string()).collect()
}

fn candidates(names: &[&str]) -> Vec<Candidate> {
    names.iter().map(|n| Candidate::new(*n)).collect()
}

fn sum_weights(weights: &BTreeMap<String, BigRational>) -> BigRational {
    weights.values().fold(BigRational::zero(), |acc, w| acc + w)
}

fn total_slots(allocation: &BTreeMap<String, u64>) -> u64 {
    allocation.values().sum()
}

// ---------------------------------------------------------------------------
// Normalisation
// ---------------------------------------------------------------------------

#[test]
fn basic_normalization() {
    let v = voter("v1", &[("A", 3.0), ("B", 1.0)]);
    let weights = normalize_voter(&v, &ids(&["A", "B"])).unwrap();
    assert_eq!(weights["A"], frac(3, 4));
    assert_eq!(weights["B"], frac(1, 4));
    assert_eq!(sum_weights(&weights), whole(1));
}

#[test]
fn three_candidates() {
    let v = voter("v1", &[("A", 2.0), ("B", 3.0), ("C", 1.0)]);
    let weights = normalize_voter(&v, &ids(&["A", "B", "C"])).unwrap();
    assert_eq!(sum_weights(&weights), whole(1));
    assert_eq!(weights["A"], frac(2, 6));
    assert_eq!(weights["B"], frac(3, 6));
    assert_eq!(weights["C"], frac(1, 6));
}

#[test]
fn equal_scores() {
    let v = voter("v1", &[("A", 1.0), ("B", 1.0), ("C", 1.0)]);
    let weights = normalize_voter(&v, &ids(&["A", "B", "C"])).unwrap();
    for candidate in ["A", "B", "C"] {
        assert_eq!(weights[candidate], frac(1, 3));
    }
    assert_eq!(sum_weights(&weights), whole(1));
}

#[test]
fn missing_candidate_gets_zero() {
    let v = voter("v1", &[("A", 2.0)]);
    let weights = normalize_voter(&v, &ids(&["A", "B"])).unwrap();
    assert_eq!(weights["B"], BigRational::zero());
    assert_eq!(weights["A"], whole(1));
    assert_eq!(sum_weights(&weights), whole(1));
}

#[test]
fn zero_sum_ballot_is_rejected() {
    let v = voter("v1", &[("A", 0.0), ("B", 0.0)]);
    let err = normalize_voter(&v, &ids(&["A", "B"])).unwrap_err();
    assert_eq!(err, AllocationError::ZeroSumBallot { voter: "v1".into() });
}

#[test]
fn single_candidate_takes_all_weight() {
    let v = voter("v1", &[("A", 5.0)]);
    let weights = normalize_voter(&v, &ids(&["A"])).unwrap();
    assert_eq!(weights["A"], whole(1));
}

#[test]
fn decimal_scores_are_exact() {
    // The property the port exists to keep: 0.1 is one tenth, not the binary
    // double nearest to it. Three tenths sum to exactly one.
    let v = voter("v1", &[("A", 0.1), ("B", 0.1), ("C", 0.1)]);
    let weights = normalize_voter(&v, &ids(&["A", "B", "C"])).unwrap();
    assert_eq!(sum_weights(&weights), whole(1));
    assert_eq!(weights["A"], frac(1, 3));

    let exact = Score::from_decimal_str("0.1").unwrap();
    assert_eq!(exact.as_rational(), &frac(1, 10));
}

#[test]
fn negative_scores_have_no_representation() {
    assert!(Score::from_f64(-1.0).is_err());
    assert!(Score::from_decimal_str("-0.5").is_err());
}

// ---------------------------------------------------------------------------
// Aggregation
// ---------------------------------------------------------------------------

#[test]
fn conservation_invariant() {
    let voters = vec![
        voter("v1", &[("A", 1.0), ("B", 3.0)]),
        voter("v2", &[("A", 2.0), ("B", 2.0)]),
        voter("v3", &[("A", 3.0), ("B", 1.0)]),
    ];
    let agg = compute_aggregated_weights(&voters, &candidates(&["A", "B"])).unwrap();
    assert_eq!(sum_weights(&agg), whole(voters.len() as i64));
}

#[test]
fn uniform_preferences() {
    let voters: Vec<Voter> = (0..4)
        .map(|i| voter(&format!("v{i}"), &[("A", 1.0), ("B", 1.0)]))
        .collect();
    let agg = compute_aggregated_weights(&voters, &candidates(&["A", "B"])).unwrap();
    assert_eq!(agg["A"], whole(2));
    assert_eq!(agg["B"], whole(2));
}

#[test]
fn one_sided_preferences() {
    let voters = vec![
        voter("v1", &[("A", 1.0), ("B", 0.0)]),
        voter("v2", &[("A", 1.0), ("B", 0.0)]),
        voter("v3", &[("A", 0.0), ("B", 1.0)]),
    ];
    let agg = compute_aggregated_weights(&voters, &candidates(&["A", "B"])).unwrap();
    assert_eq!(agg["A"], whole(2));
    assert_eq!(agg["B"], whole(1));
    assert_eq!(sum_weights(&agg), whole(3));
}

// ---------------------------------------------------------------------------
// Slot allocation: invariants
// ---------------------------------------------------------------------------

#[test]
fn total_slots_conservation_three_candidates() {
    let voters = vec![
        voter("v1", &[("A", 3.0), ("B", 1.0), ("C", 2.0)]),
        voter("v2", &[("A", 1.0), ("B", 3.0), ("C", 2.0)]),
        voter("v3", &[("A", 2.0), ("B", 2.0), ("C", 2.0)]),
    ];
    let result = allocate_slots(&voters, &candidates(&["A", "B", "C"]), 3).unwrap();
    assert_eq!(total_slots(&result), 3);
}

#[test]
fn total_slots_conservation_various_n() {
    let voters = vec![
        voter("v1", &[("A", 2.0), ("B", 1.0), ("C", 3.0)]),
        voter("v2", &[("A", 1.0), ("B", 3.0), ("C", 2.0)]),
        voter("v3", &[("A", 3.0), ("B", 2.0), ("C", 1.0)]),
    ];
    let options = candidates(&["A", "B", "C"]);
    for n in [1, 2, 3, 5, 10] {
        let result = allocate_slots(&voters, &options, n).unwrap();
        assert_eq!(total_slots(&result), n as u64, "failed for n_slots={n}");
    }
}

// ---------------------------------------------------------------------------
// Slot allocation: correctness
// ---------------------------------------------------------------------------

#[test]
fn proportional_majority() {
    let voters = vec![
        voter("v1", &[("A", 10.0), ("B", 1.0)]),
        voter("v2", &[("A", 10.0), ("B", 1.0)]),
        voter("v3", &[("A", 1.0), ("B", 10.0)]),
    ];
    let result = allocate_slots(&voters, &candidates(&["A", "B"]), 3).unwrap();
    assert_eq!(total_slots(&result), 3);
    assert!(result["A"] > result["B"]);
}

#[test]
fn equal_preferences_tie() {
    let voters = vec![
        voter("v1", &[("A", 1.0), ("B", 1.0)]),
        voter("v2", &[("A", 1.0), ("B", 1.0)]),
    ];
    let result = allocate_slots(&voters, &candidates(&["A", "B"]), 2).unwrap();
    assert_eq!(result["A"], 1);
    assert_eq!(result["B"], 1);
    assert_eq!(total_slots(&result), 2);
}

#[test]
fn exact_quota_threshold() {
    // 4 voters, 2 slots → Droop quota = floor(4/2)+1 = 3. A is supported by
    // exactly three voters at full weight, so it hits quota exactly.
    let voters = vec![
        voter("v1", &[("A", 1.0), ("B", 0.0)]),
        voter("v2", &[("A", 1.0), ("B", 0.0)]),
        voter("v3", &[("A", 1.0), ("B", 0.0)]),
        voter("v4", &[("A", 0.0), ("B", 1.0)]),
    ];
    let result = allocate_slots(&voters, &candidates(&["A", "B"]), 2).unwrap();
    assert_eq!(total_slots(&result), 2);
    assert!(result["A"] >= 1);
}

#[test]
fn single_candidate_gets_all_slots() {
    let voters: Vec<Voter> = (0..5)
        .map(|i| voter(&format!("v{i}"), &[("A", 1.0)]))
        .collect();
    let result = allocate_slots(&voters, &candidates(&["A"]), 3).unwrap();
    assert_eq!(result["A"], 3);
}

#[test]
fn more_slots_than_voters() {
    let voters = vec![voter("v1", &[("A", 1.0), ("B", 1.0)])];
    let result = allocate_slots(&voters, &candidates(&["A", "B"]), 6).unwrap();
    assert_eq!(total_slots(&result), 6);
}

#[test]
fn many_candidates_few_slots() {
    let voters = vec![
        voter("v1", &[("A", 5.0), ("B", 1.0), ("C", 1.0), ("D", 1.0)]),
        voter("v2", &[("A", 5.0), ("B", 1.0), ("C", 1.0), ("D", 1.0)]),
    ];
    let result = allocate_slots(&voters, &candidates(&["A", "B", "C", "D"]), 2).unwrap();
    assert_eq!(total_slots(&result), 2);
}

// ---------------------------------------------------------------------------
// Determinism
// ---------------------------------------------------------------------------

#[test]
fn same_input_same_output() {
    let voters = vec![
        voter("v1", &[("A", 2.0), ("B", 1.0), ("C", 3.0)]),
        voter("v2", &[("A", 1.0), ("B", 3.0), ("C", 2.0)]),
    ];
    let options = candidates(&["A", "B", "C"]);
    let first = allocate_slots(&voters, &options, 3).unwrap();
    let second = allocate_slots(&voters, &options, 3).unwrap();
    assert_eq!(first, second);
}

#[test]
fn candidate_order_independent() {
    let voters = vec![
        voter("v1", &[("A", 3.0), ("B", 2.0), ("C", 1.0)]),
        voter("v2", &[("A", 1.0), ("B", 2.0), ("C", 3.0)]),
        voter("v3", &[("A", 2.0), ("B", 3.0), ("C", 1.0)]),
    ];
    let forward = allocate_slots(&voters, &candidates(&["A", "B", "C"]), 3).unwrap();
    let reversed = allocate_slots(&voters, &candidates(&["C", "B", "A"]), 3).unwrap();
    for candidate in ["A", "B", "C"] {
        assert_eq!(forward[candidate], reversed[candidate]);
    }
}

// ---------------------------------------------------------------------------
// Multi-group scenarios
// ---------------------------------------------------------------------------

#[test]
fn group_weight_aggregation() {
    let voters = vec![
        voter("v1", &[("A", 2.0), ("B", 1.0), ("C", 1.0)]),
        voter("v2", &[("A", 1.0), ("B", 2.0), ("C", 1.0)]),
    ];
    let options = vec![
        Candidate::in_group("A", "G1"),
        Candidate::in_group("B", "G1"),
        Candidate::in_group("C", "G2"),
    ];
    let agg = compute_aggregated_weights(&voters, &options).unwrap();
    let groups = compute_group_weights(&agg, &options);
    assert!(groups.contains_key("G1"));
    assert!(groups.contains_key("G2"));
    assert_eq!(
        groups["G1"].clone() + groups["G2"].clone(),
        sum_weights(&agg)
    );
}

#[test]
fn open_ordering_within_group() {
    let voters = vec![
        voter("v1", &[("A", 1.0), ("B", 3.0), ("C", 2.0)]),
        voter("v2", &[("A", 1.0), ("B", 3.0), ("C", 2.0)]),
    ];
    // Listed A, B, C but B carries the most weight.
    let options = vec![
        Candidate::in_group("A", "G1"),
        Candidate::in_group("B", "G1"),
        Candidate::in_group("C", "G1"),
    ];
    let agg = compute_aggregated_weights(&voters, &options).unwrap();
    let ordered = order_candidates_by_weight(&options, &agg, Some("G1"));
    assert_eq!(ordered[0].id, "B");
    assert_eq!(ordered[1].id, "C");
    assert_eq!(ordered[2].id, "A");
}

#[test]
fn multi_group_slot_allocation_conserved() {
    let voters = vec![
        voter("v1", &[("A", 3.0), ("B", 1.0), ("C", 1.0), ("D", 1.0)]),
        voter("v2", &[("A", 1.0), ("B", 3.0), ("C", 1.0), ("D", 1.0)]),
        voter("v3", &[("A", 1.0), ("B", 1.0), ("C", 3.0), ("D", 1.0)]),
        voter("v4", &[("A", 1.0), ("B", 1.0), ("C", 1.0), ("D", 3.0)]),
    ];
    let options = vec![
        Candidate::in_group("A", "G1"),
        Candidate::in_group("B", "G1"),
        Candidate::in_group("C", "G2"),
        Candidate::in_group("D", "G2"),
    ];
    let result = allocate_slots(&voters, &options, 4).unwrap();
    assert_eq!(total_slots(&result), 4);
}

#[test]
fn ungrouped_candidates_excluded_from_group_weights() {
    let voters = vec![voter("v1", &[("A", 1.0), ("B", 1.0), ("C", 1.0)])];
    let options = vec![
        Candidate::in_group("A", "G1"),
        Candidate::new("B"), // no group
        Candidate::in_group("C", "G1"),
    ];
    let agg = compute_aggregated_weights(&voters, &options).unwrap();
    let groups = compute_group_weights(&agg, &options);
    assert!(groups.contains_key("G1"));
    assert_eq!(groups.len(), 1);
    // B's weight is real but belongs to no group.
    assert_eq!(groups["G1"], agg["A"].clone() + agg["C"].clone());
}

// ---------------------------------------------------------------------------
// Surplus-transfer weight conservation
// ---------------------------------------------------------------------------

#[test]
fn surplus_transfer_conserves_slots() {
    let voters = vec![
        voter("v1", &[("A", 5.0), ("B", 3.0), ("C", 2.0)]),
        voter("v2", &[("A", 5.0), ("B", 2.0), ("C", 3.0)]),
        voter("v3", &[("A", 1.0), ("B", 5.0), ("C", 4.0)]),
        voter("v4", &[("A", 1.0), ("B", 4.0), ("C", 5.0)]),
        voter("v5", &[("A", 1.0), ("B", 3.0), ("C", 6.0)]),
    ];
    let result = allocate_slots(&voters, &candidates(&["A", "B", "C"]), 3).unwrap();
    assert_eq!(total_slots(&result), 3);
}

#[test]
fn no_slots_created_or_destroyed_by_rounding() {
    let voters: Vec<Voter> = (1..=3)
        .map(|i| voter(&format!("v{i}"), &[("A", 1.0), ("B", 1.0), ("C", 1.0)]))
        .collect();
    let options = candidates(&["A", "B", "C"]);
    for n in [1, 2, 4, 7, 10] {
        let result = allocate_slots(&voters, &options, n).unwrap();
        assert_eq!(total_slots(&result), n as u64, "failed for n_slots={n}");
    }
}

// ---------------------------------------------------------------------------
// Edge cases and validation
// ---------------------------------------------------------------------------

#[test]
fn zero_sum_ballot_rejected_in_allocate() {
    let voters = vec![voter("v1", &[("A", 0.0), ("B", 0.0)])];
    let err = allocate_slots(&voters, &candidates(&["A", "B"]), 1).unwrap_err();
    assert_eq!(err, AllocationError::ZeroSumBallot { voter: "v1".into() });
}

#[test]
fn invalid_n_slots_rejected() {
    let voters = vec![voter("v1", &[("A", 1.0)])];
    let options = candidates(&["A"]);
    assert_eq!(
        allocate_slots(&voters, &options, 0).unwrap_err(),
        AllocationError::NonPositiveSlots(0)
    );
    assert_eq!(
        allocate_slots(&voters, &options, -1).unwrap_err(),
        AllocationError::NonPositiveSlots(-1)
    );
}

#[test]
fn empty_voters_rejected() {
    let err = allocate_slots(&[], &candidates(&["A"]), 1).unwrap_err();
    assert_eq!(err, AllocationError::EmptyVoters);
}

#[test]
fn empty_candidates_rejected() {
    let voters = vec![Voter::new("v1")];
    let err = allocate_slots(&voters, &[], 1).unwrap_err();
    assert_eq!(err, AllocationError::EmptyCandidates);
}

#[test]
fn single_voter_single_candidate() {
    let voters = vec![voter("v1", &[("A", 7.0)])];
    let result = allocate_slots(&voters, &candidates(&["A"]), 5).unwrap();
    assert_eq!(result["A"], 5);
}

#[test]
fn exact_half_tie_does_not_double_count() {
    let voters = vec![voter("v1", &[("A", 1.0), ("B", 1.0)])];
    let result = allocate_slots(&voters, &candidates(&["A", "B"]), 1).unwrap();
    assert_eq!(total_slots(&result), 1);
}

#[test]
fn large_n_slots_with_two_candidates() {
    let voters = vec![
        voter("v1", &[("A", 3.0), ("B", 1.0)]),
        voter("v2", &[("A", 3.0), ("B", 1.0)]),
    ];
    let result = allocate_slots(&voters, &candidates(&["A", "B"]), 100).unwrap();
    assert_eq!(total_slots(&result), 100);
    assert!(result["A"] > result["B"]);
}

// ---------------------------------------------------------------------------
// Ordering
// ---------------------------------------------------------------------------

#[test]
fn order_all_candidates() {
    let voters = vec![voter("v1", &[("A", 1.0), ("B", 2.0), ("C", 3.0)])];
    let options = candidates(&["A", "B", "C"]);
    let agg = compute_aggregated_weights(&voters, &options).unwrap();
    let ordered = order_candidates_by_weight(&options, &agg, None);
    let order: Vec<&str> = ordered.iter().map(|c| c.id.as_str()).collect();
    assert_eq!(order, ["C", "B", "A"]);
}

#[test]
fn order_filtered_by_group() {
    let voters = vec![voter("v1", &[("A", 3.0), ("B", 1.0), ("C", 2.0)])];
    let options = vec![
        Candidate::in_group("A", "G1"),
        Candidate::in_group("B", "G1"),
        Candidate::in_group("C", "G2"),
    ];
    let agg = compute_aggregated_weights(&voters, &options).unwrap();
    let ordered = order_candidates_by_weight(&options, &agg, Some("G1"));
    assert_eq!(ordered.len(), 2);
    assert_eq!(ordered[0].id, "A");
    assert_eq!(ordered[1].id, "B");
}

#[test]
fn tie_broken_by_id() {
    let voters = vec![voter("v1", &[("X", 1.0), ("Y", 1.0)])];
    let options = candidates(&["X", "Y"]);
    let agg = compute_aggregated_weights(&voters, &options).unwrap();
    let ordered = order_candidates_by_weight(&options, &agg, None);
    assert_eq!(ordered[0].id, "X");
    assert_eq!(ordered[1].id, "Y");
}
