"""Unit tests for the proportional allocation engine.

Coverage
--------
* Conservation of total weight (normalisation invariant)
* Conservation of allocated slots (slot invariant)
* Tie cases (equal weights must not violate global totals)
* Exact threshold cases (option accumulated weight == quota)
* Multi‑group scenarios
* Surplus‑transfer weight conservation
* Determinism (same input → same output)
* Edge cases: single option, zero‑sum ballot, invalid n_slots
"""

from __future__ import annotations

import pytest
from fractions import Fraction

from voting.models import Option, Voter
from voting.allocation import (
    allocate_slots,
    compute_aggregated_weights,
    compute_group_weights,
    normalize_voter,
    order_options_by_weight,
)


# ── Normalisation ──────────────────────────────────────────────────────────

class TestNormalizeVoter:
    def test_basic_normalization(self) -> None:
        voter = Voter("v1", {"A": 3.0, "B": 1.0})
        weights = normalize_voter(voter, ["A", "B"])
        assert weights["A"] == Fraction(3, 4)
        assert weights["B"] == Fraction(1, 4)
        assert sum(weights.values()) == Fraction(1)

    def test_three_options(self) -> None:
        voter = Voter("v1", {"A": 2.0, "B": 3.0, "C": 1.0})
        weights = normalize_voter(voter, ["A", "B", "C"])
        assert sum(weights.values()) == Fraction(1)
        assert weights["A"] == Fraction(2, 6)
        assert weights["B"] == Fraction(3, 6)
        assert weights["C"] == Fraction(1, 6)

    def test_equal_scores(self) -> None:
        voter = Voter("v1", {"A": 1.0, "B": 1.0, "C": 1.0})
        weights = normalize_voter(voter, ["A", "B", "C"])
        for opt in ["A", "B", "C"]:
            assert weights[opt] == Fraction(1, 3)
        assert sum(weights.values()) == Fraction(1)

    def test_missing_option_gets_zero(self) -> None:
        voter = Voter("v1", {"A": 2.0})
        weights = normalize_voter(voter, ["A", "B"])
        assert weights["B"] == Fraction(0)
        assert weights["A"] == Fraction(1)
        assert sum(weights.values()) == Fraction(1)

    def test_zero_sum_ballot_raises(self) -> None:
        voter = Voter("v1", {"A": 0.0, "B": 0.0})
        with pytest.raises(ValueError, match="zero‑sum"):
            normalize_voter(voter, ["A", "B"])

    def test_single_option(self) -> None:
        voter = Voter("v1", {"A": 5.0})
        weights = normalize_voter(voter, ["A"])
        assert weights["A"] == Fraction(1)


# ── Aggregation ────────────────────────────────────────────────────────────

class TestAggregatedWeights:
    def test_conservation_invariant(self) -> None:
        """Grand total of aggregated weights must equal number of voters."""
        voters = [
            Voter("v1", {"A": 1.0, "B": 3.0}),
            Voter("v2", {"A": 2.0, "B": 2.0}),
            Voter("v3", {"A": 3.0, "B": 1.0}),
        ]
        options = [Option("A"), Option("B")]
        agg = compute_aggregated_weights(voters, options)
        assert sum(agg.values()) == Fraction(len(voters))

    def test_uniform_preferences(self) -> None:
        """With identical preferences each option gets equal share."""
        voters = [Voter(f"v{i}", {"A": 1.0, "B": 1.0}) for i in range(4)]
        options = [Option("A"), Option("B")]
        agg = compute_aggregated_weights(voters, options)
        assert agg["A"] == agg["B"] == Fraction(2)

    def test_one_sided_preferences(self) -> None:
        """Voters who only score one option give it their full weight."""
        voters = [
            Voter("v1", {"A": 1.0, "B": 0.0}),
            Voter("v2", {"A": 1.0, "B": 0.0}),
            Voter("v3", {"A": 0.0, "B": 1.0}),
        ]
        options = [Option("A"), Option("B")]
        agg = compute_aggregated_weights(voters, options)
        assert agg["A"] == Fraction(2)
        assert agg["B"] == Fraction(1)
        assert sum(agg.values()) == Fraction(3)


# ── Slot allocation: invariants ────────────────────────────────────────────

class TestAllocateSlotsInvariants:
    def test_total_slots_conservation_3_options(self) -> None:
        voters = [
            Voter("v1", {"A": 3.0, "B": 1.0, "C": 2.0}),
            Voter("v2", {"A": 1.0, "B": 3.0, "C": 2.0}),
            Voter("v3", {"A": 2.0, "B": 2.0, "C": 2.0}),
        ]
        options = [Option("A"), Option("B"), Option("C")]
        result = allocate_slots(voters, options, n_slots=3)
        assert sum(result.values()) == 3

    def test_total_slots_conservation_various_n(self) -> None:
        """slot conservation holds for several values of n_slots."""
        voters = [
            Voter("v1", {"A": 2.0, "B": 1.0, "C": 3.0}),
            Voter("v2", {"A": 1.0, "B": 3.0, "C": 2.0}),
            Voter("v3", {"A": 3.0, "B": 2.0, "C": 1.0}),
        ]
        options = [Option("A"), Option("B"), Option("C")]
        for n in [1, 2, 3, 5, 10]:
            result = allocate_slots(voters, options, n_slots=n)
            assert sum(result.values()) == n, f"Failed for n_slots={n}"

    def test_all_values_non_negative(self) -> None:
        voters = [Voter("v1", {"A": 1.0, "B": 2.0})]
        options = [Option("A"), Option("B")]
        result = allocate_slots(voters, options, n_slots=3)
        assert all(v >= 0 for v in result.values())


# ── Slot allocation: correctness ───────────────────────────────────────────

class TestAllocateSlotsCorrectness:
    def test_proportional_majority(self) -> None:
        """Option with dominant support receives more slots."""
        voters = [
            Voter("v1", {"A": 10.0, "B": 1.0}),
            Voter("v2", {"A": 10.0, "B": 1.0}),
            Voter("v3", {"A": 1.0, "B": 10.0}),
        ]
        options = [Option("A"), Option("B")]
        result = allocate_slots(voters, options, n_slots=3)
        assert sum(result.values()) == 3
        assert result["A"] > result["B"]

    def test_equal_preferences_tie(self) -> None:
        """Equal weight: each option must get the same number of slots."""
        voters = [
            Voter("v1", {"A": 1.0, "B": 1.0}),
            Voter("v2", {"A": 1.0, "B": 1.0}),
        ]
        options = [Option("A"), Option("B")]
        result = allocate_slots(voters, options, n_slots=2)
        assert result["A"] == 1
        assert result["B"] == 1
        assert sum(result.values()) == 2

    def test_exact_quota_threshold(self) -> None:
        """Option accumulated weight == quota must be allocated correctly.

        4 voters, 2 slots → Droop quota = floor(4/2)+1 = 3.
        Option A is supported by exactly 3 voters (each contributing full
        weight), so it hits quota exactly.
        """
        voters = [
            Voter("v1", {"A": 1.0, "B": 0.0}),
            Voter("v2", {"A": 1.0, "B": 0.0}),
            Voter("v3", {"A": 1.0, "B": 0.0}),
            Voter("v4", {"A": 0.0, "B": 1.0}),
        ]
        options = [Option("A"), Option("B")]
        result = allocate_slots(voters, options, n_slots=2)
        assert sum(result.values()) == 2
        # A accumulates exactly quota (3) and must be allocated a slot
        assert result["A"] >= 1

    def test_single_option_gets_all_slots(self) -> None:
        voters = [Voter(f"v{i}", {"A": 1.0}) for i in range(5)]
        options = [Option("A")]
        result = allocate_slots(voters, options, n_slots=3)
        assert result["A"] == 3

    def test_more_slots_than_voters(self) -> None:
        """n_slots > len(voters) is valid; conservation still holds."""
        voters = [Voter("v1", {"A": 1.0, "B": 1.0})]
        options = [Option("A"), Option("B")]
        result = allocate_slots(voters, options, n_slots=6)
        assert sum(result.values()) == 6

    def test_many_options_few_slots(self) -> None:
        """Slots < options: some options legitimately receive 0."""
        voters = [
            Voter("v1", {"A": 5.0, "B": 1.0, "C": 1.0, "D": 1.0}),
            Voter("v2", {"A": 5.0, "B": 1.0, "C": 1.0, "D": 1.0}),
        ]
        options = [Option("A"), Option("B"), Option("C"), Option("D")]
        result = allocate_slots(voters, options, n_slots=2)
        assert sum(result.values()) == 2


# ── Determinism ────────────────────────────────────────────────────────────

class TestDeterminism:
    def test_same_input_same_output(self) -> None:
        voters = [
            Voter("v1", {"A": 2.0, "B": 1.0, "C": 3.0}),
            Voter("v2", {"A": 1.0, "B": 3.0, "C": 2.0}),
        ]
        options = [Option("A"), Option("B"), Option("C")]
        result1 = allocate_slots(voters, options, n_slots=3)
        result2 = allocate_slots(voters, options, n_slots=3)
        assert result1 == result2

    def test_option_order_independent(self) -> None:
        """Reversing the options list must not change allocations."""
        voters = [
            Voter("v1", {"A": 3.0, "B": 2.0, "C": 1.0}),
            Voter("v2", {"A": 1.0, "B": 2.0, "C": 3.0}),
            Voter("v3", {"A": 2.0, "B": 3.0, "C": 1.0}),
        ]
        options_fwd = [Option("A"), Option("B"), Option("C")]
        options_rev = [Option("C"), Option("B"), Option("A")]
        r1 = allocate_slots(voters, options_fwd, n_slots=3)
        r2 = allocate_slots(voters, options_rev, n_slots=3)
        for opt_id in ["A", "B", "C"]:
            assert r1[opt_id] == r2[opt_id]


# ── Multi‑group scenarios ──────────────────────────────────────────────────

class TestMultiGroupScenarios:
    def test_group_weight_aggregation(self) -> None:
        """Group weights must sum to match the sum of member option weights."""
        voters = [
            Voter("v1", {"A": 2.0, "B": 1.0, "C": 1.0}),
            Voter("v2", {"A": 1.0, "B": 2.0, "C": 1.0}),
        ]
        options = [
            Option("A", group_id="G1"),
            Option("B", group_id="G1"),
            Option("C", group_id="G2"),
        ]
        agg = compute_aggregated_weights(voters, options)
        groups = compute_group_weights(agg, options)
        assert "G1" in groups
        assert "G2" in groups
        # G1 total + G2 total == grand total
        assert groups["G1"] + groups["G2"] == sum(agg.values())

    def test_open_ordering_within_group(self) -> None:
        """Options within a group are ordered by weight, not list position."""
        voters = [
            Voter("v1", {"A": 1.0, "B": 3.0, "C": 2.0}),
            Voter("v2", {"A": 1.0, "B": 3.0, "C": 2.0}),
        ]
        # Listed in order A, B, C but B has the most weight
        options = [
            Option("A", group_id="G1"),
            Option("B", group_id="G1"),
            Option("C", group_id="G1"),
        ]
        agg = compute_aggregated_weights(voters, options)
        ordered = order_options_by_weight(options, agg, group_id="G1")
        assert ordered[0].id == "B"
        assert ordered[1].id == "C"
        assert ordered[2].id == "A"

    def test_multi_group_slot_allocation_conserved(self) -> None:
        """Slot conservation holds across a two‑group scenario."""
        voters = [
            Voter("v1", {"A": 3.0, "B": 1.0, "C": 1.0, "D": 1.0}),
            Voter("v2", {"A": 1.0, "B": 3.0, "C": 1.0, "D": 1.0}),
            Voter("v3", {"A": 1.0, "B": 1.0, "C": 3.0, "D": 1.0}),
            Voter("v4", {"A": 1.0, "B": 1.0, "C": 1.0, "D": 3.0}),
        ]
        options = [
            Option("A", group_id="G1"),
            Option("B", group_id="G1"),
            Option("C", group_id="G2"),
            Option("D", group_id="G2"),
        ]
        result = allocate_slots(voters, options, n_slots=4)
        assert sum(result.values()) == 4

    def test_ungrouped_options_excluded_from_group_weights(self) -> None:
        """Options without a group_id must not appear in group_weights."""
        voters = [Voter("v1", {"A": 1.0, "B": 1.0, "C": 1.0})]
        options = [
            Option("A", group_id="G1"),
            Option("B"),           # no group
            Option("C", group_id="G1"),
        ]
        agg = compute_aggregated_weights(voters, options)
        groups = compute_group_weights(agg, options)
        assert "G1" in groups
        # Ungrouped option B is not counted in any group
        assert groups.get(None) is None  # type: ignore[arg-type]


# ── Surplus‑transfer weight conservation ─────────────────────────────────

class TestWeightConservation:
    def test_surplus_transfer_conserves_slots(self) -> None:
        """If STV triggers surplus transfers, total slots remain n_slots."""
        voters = [
            Voter("v1", {"A": 5.0, "B": 3.0, "C": 2.0}),
            Voter("v2", {"A": 5.0, "B": 2.0, "C": 3.0}),
            Voter("v3", {"A": 1.0, "B": 5.0, "C": 4.0}),
            Voter("v4", {"A": 1.0, "B": 4.0, "C": 5.0}),
            Voter("v5", {"A": 1.0, "B": 3.0, "C": 6.0}),
        ]
        options = [Option("A"), Option("B"), Option("C")]
        result = allocate_slots(voters, options, n_slots=3)
        assert sum(result.values()) == 3
        assert all(v >= 0 for v in result.values())

    def test_no_slots_created_or_destroyed_by_rounding(self) -> None:
        """Largest‑remainder rounding must never over‑ or under‑allocate."""
        voters = [
            Voter("v1", {"A": 1.0, "B": 1.0, "C": 1.0}),
            Voter("v2", {"A": 1.0, "B": 1.0, "C": 1.0}),
            Voter("v3", {"A": 1.0, "B": 1.0, "C": 1.0}),
        ]
        options = [Option("A"), Option("B"), Option("C")]
        for n in [1, 2, 4, 7, 10]:
            result = allocate_slots(voters, options, n_slots=n)
            assert sum(result.values()) == n, f"Failed for n_slots={n}"


# ── Edge cases and validation ──────────────────────────────────────────────

class TestEdgeCases:
    def test_zero_sum_ballot_raises_in_allocate(self) -> None:
        voter = Voter("v1", {"A": 0.0, "B": 0.0})
        options = [Option("A"), Option("B")]
        with pytest.raises(ValueError, match="zero‑sum"):
            allocate_slots([voter], options, n_slots=1)

    def test_invalid_n_slots_raises(self) -> None:
        voters = [Voter("v1", {"A": 1.0})]
        options = [Option("A")]
        with pytest.raises(ValueError):
            allocate_slots(voters, options, n_slots=0)

    def test_negative_n_slots_raises(self) -> None:
        voters = [Voter("v1", {"A": 1.0})]
        options = [Option("A")]
        with pytest.raises(ValueError):
            allocate_slots(voters, options, n_slots=-1)

    def test_empty_voters_raises(self) -> None:
        with pytest.raises(ValueError, match="empty"):
            allocate_slots([], [Option("A")], n_slots=1)

    def test_empty_options_raises(self) -> None:
        with pytest.raises(ValueError, match="empty"):
            allocate_slots([Voter("v1", {})], [], n_slots=1)

    def test_negative_score_raises(self) -> None:
        with pytest.raises(ValueError):
            Voter("v1", {"A": -1.0, "B": 1.0})

    def test_single_voter_single_option(self) -> None:
        voters = [Voter("v1", {"A": 7.0})]
        options = [Option("A")]
        result = allocate_slots(voters, options, n_slots=5)
        assert result["A"] == 5

    def test_exact_half_tie_does_not_double_count(self) -> None:
        """A 0.5/0.5 tie must not produce more than n_slots total slots."""
        voters = [Voter("v1", {"A": 1.0, "B": 1.0})]
        options = [Option("A"), Option("B")]
        result = allocate_slots(voters, options, n_slots=1)
        assert sum(result.values()) == 1

    def test_large_n_slots_with_two_options(self) -> None:
        voters = [
            Voter("v1", {"A": 3.0, "B": 1.0}),
            Voter("v2", {"A": 3.0, "B": 1.0}),
        ]
        options = [Option("A"), Option("B")]
        result = allocate_slots(voters, options, n_slots=100)
        assert sum(result.values()) == 100
        # A should receive ~75 slots, B ~25
        assert result["A"] > result["B"]


# ── Group ordering ────────────────────────────────────────────────────────

class TestOrderOptionsByWeight:
    def test_order_all_options(self) -> None:
        voters = [
            Voter("v1", {"A": 1.0, "B": 2.0, "C": 3.0}),
        ]
        options = [Option("A"), Option("B"), Option("C")]
        agg = compute_aggregated_weights(voters, options)
        ordered = order_options_by_weight(options, agg)
        assert [o.id for o in ordered] == ["C", "B", "A"]

    def test_order_filtered_by_group(self) -> None:
        voters = [Voter("v1", {"A": 3.0, "B": 1.0, "C": 2.0})]
        options = [
            Option("A", group_id="G1"),
            Option("B", group_id="G1"),
            Option("C", group_id="G2"),
        ]
        agg = compute_aggregated_weights(voters, options)
        ordered = order_options_by_weight(options, agg, group_id="G1")
        assert len(ordered) == 2
        assert ordered[0].id == "A"  # highest weight in G1
        assert ordered[1].id == "B"

    def test_tie_broken_by_id(self) -> None:
        """Tied weights must be ordered lexicographically by option id."""
        voters = [Voter("v1", {"X": 1.0, "Y": 1.0})]
        options = [Option("X"), Option("Y")]
        agg = compute_aggregated_weights(voters, options)
        ordered = order_options_by_weight(options, agg)
        # Weights are equal; 'X' < 'Y' lexicographically
        assert ordered[0].id == "X"
        assert ordered[1].id == "Y"
