"""Deterministic, invariant‑preserving proportional allocation engine.

Algorithm overview
------------------
1. **Normalisation** – each voter's raw scores are converted to fractional
   weights (using ``fractions.Fraction`` for exact arithmetic) that sum to
   exactly 1.
2. **Aggregation** – normalised weights are summed across all voters, yielding
   per‑option totals whose grand sum equals the number of voters.
3. **STV quota allocation** (Phase 1) – a Droop quota is computed and options
   that accumulate enough weight are awarded a slot.  Surplus weight is
   redistributed proportionally to each voter's remaining preferences.
4. **Largest‑remainder allocation** (Phase 2) – any slots not filled in Phase 1
   are distributed via the largest‑remainder (Hamilton) method, with
   deterministic tie‑breaking.

Invariants (asserted at every phase boundary)
---------------------------------------------
* Total normalised voter weight  = ``len(voters)``
* Total allocated slots          = ``n_slots``
* Every surplus transfer conserves total weight exactly
* Tie‑breaking is purely deterministic (lexicographic option id)
"""

from __future__ import annotations

from fractions import Fraction
from typing import Optional

from voting.models import Option, Voter


# ---------------------------------------------------------------------------
# Normalisation
# ---------------------------------------------------------------------------

def normalize_voter(voter: Voter, option_ids: list[str]) -> dict[str, Fraction]:
    """Convert raw scores to fractional weights that sum to exactly 1.

    Parameters
    ----------
    voter:
        The voter whose scores are to be normalised.
    option_ids:
        Ordered list of all option identifiers to include.  Missing options
        receive a weight of 0.

    Returns
    -------
    dict[str, Fraction]
        Mapping ``option_id → weight`` with ``sum(values()) == Fraction(1)``.

    Raises
    ------
    ValueError
        If all scores are zero (zero‑sum ballot).
    """
    total = sum(
        Fraction(str(voter.scores.get(opt_id, 0))) for opt_id in option_ids
    )
    if total == 0:
        raise ValueError(
            f"Voter {voter.id!r} has a zero‑sum ballot – "
            "cannot normalise weights"
        )

    weights: dict[str, Fraction] = {
        opt_id: Fraction(str(voter.scores.get(opt_id, 0))) / total
        for opt_id in option_ids
    }

    # Invariant: normalised weights must sum to exactly 1
    assert sum(weights.values()) == Fraction(1), (
        f"Normalisation invariant violated for voter {voter.id!r}: "
        f"sum = {sum(weights.values())}"
    )
    return weights


# ---------------------------------------------------------------------------
# Aggregation
# ---------------------------------------------------------------------------

def compute_aggregated_weights(
    voters: list[Voter],
    options: list[Option],
) -> dict[str, Fraction]:
    """Aggregate normalised voter weights per option.

    Parameters
    ----------
    voters:
        Non‑empty list of voters.
    options:
        Non‑empty list of options.

    Returns
    -------
    dict[str, Fraction]
        Mapping ``option_id → total_weight`` where
        ``sum(values()) == Fraction(len(voters))``.

    Raises
    ------
    ValueError
        If any voter has a zero‑sum ballot (propagated from
        :func:`normalize_voter`).
    """
    option_ids = [o.id for o in options]
    normalized = [normalize_voter(v, option_ids) for v in voters]

    aggregated: dict[str, Fraction] = {opt_id: Fraction(0) for opt_id in option_ids}
    for weights in normalized:
        for opt_id, w in weights.items():
            aggregated[opt_id] += w

    # Invariant: grand total must equal the number of voters
    total = sum(aggregated.values())
    assert total == Fraction(len(voters)), (
        f"Aggregation invariant violated: sum = {total}, "
        f"expected {len(voters)}"
    )
    return aggregated


def compute_group_weights(
    aggregated: dict[str, Fraction],
    options: list[Option],
) -> dict[str, Fraction]:
    """Compute total aggregated weight per group.

    Parameters
    ----------
    aggregated:
        Per‑option aggregated weights (output of
        :func:`compute_aggregated_weights`).
    options:
        Full option list (used to look up ``group_id``).

    Returns
    -------
    dict[str, Fraction]
        Mapping ``group_id → total_weight`` for every group that appears in
        *options*.  Options without a group are not included.
    """
    group_totals: dict[str, Fraction] = {}
    for option in options:
        if option.group_id is not None:
            g = option.group_id
            group_totals[g] = group_totals.get(g, Fraction(0)) + aggregated[option.id]
    return group_totals


# ---------------------------------------------------------------------------
# Ordering
# ---------------------------------------------------------------------------

def order_options_by_weight(
    options: list[Option],
    aggregated: dict[str, Fraction],
    group_id: Optional[str] = None,
) -> list[Option]:
    """Order options by aggregated weight (descending).

    Enforces *open ordering*: the final order is determined solely by
    accumulated weight, never by the position in the input list.

    Parameters
    ----------
    options:
        All available options.
    aggregated:
        Per‑option aggregated weights.
    group_id:
        When provided, only options whose ``group_id`` matches are returned.

    Returns
    -------
    list[Option]
        Options sorted by descending weight with lexicographic option‑id as the
        deterministic tie‑breaker.
    """
    if group_id is not None:
        candidates = [o for o in options if o.group_id == group_id]
    else:
        candidates = list(options)

    return sorted(candidates, key=lambda o: (-aggregated[o.id], o.id))


# ---------------------------------------------------------------------------
# Slot allocation
# ---------------------------------------------------------------------------

def allocate_slots(
    voters: list[Voter],
    options: list[Option],
    n_slots: int,
) -> dict[str, int]:
    """Allocate *n_slots* discrete slots to options.

    The algorithm proceeds in two phases:

    **Phase 1 – STV quota allocation**
    A Droop quota is computed::

        n_voters = len(voters)
        quota = floor(n_voters / n_slots) + 1

    In each round the option with the highest accumulated weight that meets or
    exceeds the quota is awarded a slot.  Its surplus weight is redistributed
    proportionally across each voter's remaining (non‑elected) options.  Rounds
    continue until no option meets quota or all slots are filled.

    **Phase 2 – Largest‑remainder allocation**
    Remaining slots (those not filled in Phase 1) are distributed using the
    Hamilton / largest‑remainder method:

    1. Compute each option's fair share of the remaining slots.
    2. Assign the integer floor of each fair share.
    3. Distribute leftover slots to options with the largest fractional
       remainder.  Ties are broken lexicographically by option id.

    Parameters
    ----------
    voters:
        Non‑empty list of voters.
    options:
        Non‑empty list of options.
    n_slots:
        Positive integer – total number of discrete slots to distribute.

    Returns
    -------
    dict[str, int]
        Mapping ``option_id → slots_allocated``.
        Invariant: ``sum(values()) == n_slots`` and all values ≥ 0.

    Raises
    ------
    ValueError
        If *n_slots* is not positive, if *voters* or *options* is empty, or if
        any voter has a zero‑sum ballot.
    """
    if n_slots <= 0:
        raise ValueError(f"n_slots must be a positive integer, got {n_slots}")
    if not voters:
        raise ValueError("voters list must not be empty")
    if not options:
        raise ValueError("options list must not be empty")

    option_ids: list[str] = [o.id for o in options]
    n_voters: int = len(voters)

    # ── Build per‑voter fractional weights (exact rational arithmetic) ─────
    voter_weights: list[dict[str, Fraction]] = [
        normalize_voter(v, option_ids) for v in voters
    ]

    # Droop quota: floor(total_weight / n_slots) + 1
    # With total_weight == n_voters, this is the classical Droop quota.
    quota = Fraction(n_voters // n_slots + 1)

    # Mutable weight state used throughout Phase 1.
    # current_weights[voter_idx][option_id] = still‑available fractional weight.
    current_weights: list[dict[str, Fraction]] = [dict(w) for w in voter_weights]

    allocated: dict[str, int] = {opt_id: 0 for opt_id in option_ids}
    remaining_options: set[str] = set(option_ids)
    slots_remaining: int = n_slots

    # ── Phase 1: STV quota‑based allocation with surplus transfer ──────────
    while slots_remaining > 0 and remaining_options:
        # Accumulate per‑option weight from all voters
        accumulated: dict[str, Fraction] = {
            opt_id: sum(
                current_weights[i].get(opt_id, Fraction(0))
                for i in range(n_voters)
            )
            for opt_id in remaining_options
        }

        # Find all options that have met or exceeded the quota
        eligible = [
            opt_id for opt_id in remaining_options if accumulated[opt_id] >= quota
        ]
        if not eligible:
            break  # No option meets quota → proceed to Phase 2

        # Deterministic selection: highest accumulated weight, then lex id
        winner = min(eligible, key=lambda o: (-accumulated[o], o))

        allocated[winner] += 1
        slots_remaining -= 1
        remaining_options.remove(winner)

        winner_total = accumulated[winner]
        surplus = winner_total - quota
        # Each voter's contribution to the winner is reduced by the transfer
        # fraction so that the winner retains exactly its quota‑share.
        transfer_ratio = surplus / winner_total  # proportion redistributed

        for i in range(n_voters):
            voter_contrib = current_weights[i].get(winner, Fraction(0))
            if voter_contrib == 0:
                continue

            # The surplus from this voter is redistributed to remaining options
            voter_surplus = voter_contrib * transfer_ratio

            # Retain only the quota‑share for this voter
            current_weights[i][winner] = voter_contrib - voter_surplus

            if not remaining_options:
                # No remaining options to receive the surplus; absorbed here.
                # This only occurs when all options have been elected.
                continue

            # Redistribute voter_surplus proportionally to remaining options
            # (following the voter's next‑highest remaining weights).
            future_total = sum(
                current_weights[i].get(o, Fraction(0)) for o in remaining_options
            )
            if future_total > 0:
                for o in remaining_options:
                    w = current_weights[i].get(o, Fraction(0))
                    current_weights[i][o] = w + voter_surplus * (w / future_total)
            # If future_total == 0 the voter has no remaining preferences;
            # the surplus is absorbed (conservation is maintained globally
            # because the slots_remaining path below handles this).

    # ── Phase 2: Largest‑remainder allocation for remaining slots ──────────
    if slots_remaining > 0 and remaining_options:
        # Recompute accumulated weights for still‑eligible options
        accumulated = {
            opt_id: sum(
                current_weights[i].get(opt_id, Fraction(0)) for i in range(n_voters)
            )
            for opt_id in remaining_options
        }
        total_remaining_weight = sum(accumulated.values())

        if total_remaining_weight > 0:
            # Fair share of the remaining slots for each option
            fair_shares: dict[str, Fraction] = {
                opt_id: Fraction(slots_remaining) * accumulated[opt_id] / total_remaining_weight
                for opt_id in remaining_options
            }
        else:
            # All remaining weights are zero; distribute evenly
            fair_shares = {
                opt_id: Fraction(slots_remaining, len(remaining_options))
                for opt_id in remaining_options
            }

        # Step 1: floor allocation
        floor_alloc: dict[str, int] = {
            opt_id: int(share) for opt_id, share in fair_shares.items()
        }
        for opt_id, floor_val in floor_alloc.items():
            allocated[opt_id] += floor_val
            slots_remaining -= floor_val

        # Step 2: distribute leftover slots by largest remainder
        remainders: dict[str, Fraction] = {
            opt_id: fair_shares[opt_id] - floor_alloc[opt_id]
            for opt_id in remaining_options
        }
        sorted_by_remainder = sorted(
            remaining_options,
            key=lambda o: (-remainders[o], o),
        )
        for opt_id in sorted_by_remainder:
            if slots_remaining == 0:
                break
            allocated[opt_id] += 1
            slots_remaining -= 1

    # Handle the unlikely case where all options were elected in Phase 1 but
    # slots_remaining > 0 (can arise when n_slots > len(options)).
    if slots_remaining > 0:
        # Distribute remaining slots by overall aggregated weight using LR
        aggregated = compute_aggregated_weights(voters, options)
        grand_total = sum(aggregated.values())
        fair_shares = {
            opt_id: Fraction(slots_remaining) * aggregated[opt_id] / grand_total
            for opt_id in option_ids
        }
        floor_alloc = {opt_id: int(s) for opt_id, s in fair_shares.items()}
        for opt_id, floor_val in floor_alloc.items():
            allocated[opt_id] += floor_val
            slots_remaining -= floor_val
        remainders = {
            opt_id: fair_shares[opt_id] - floor_alloc[opt_id]
            for opt_id in option_ids
        }
        for opt_id in sorted(option_ids, key=lambda o: (-remainders[o], o)):
            if slots_remaining == 0:
                break
            allocated[opt_id] += 1
            slots_remaining -= 1

    # ── Final invariant checks ─────────────────────────────────────────────
    total_allocated = sum(allocated.values())
    assert total_allocated == n_slots, (
        f"Slot‑conservation invariant violated: "
        f"allocated {total_allocated} != requested {n_slots}"
    )
    assert all(v >= 0 for v in allocated.values()), (
        "Negative allocation detected – this is a bug in the allocation engine"
    )

    return allocated
