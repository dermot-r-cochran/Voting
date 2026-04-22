"""Data models for the proportional allocation engine.

Entities
--------
Option  – an item that can receive one or more slots.
Group   – a logical grouping of options (optional).
Voter   – holds a non‑negative score for each option it wishes to support.

Invariants
----------
* All scores in ``Voter.scores`` are non‑negative.
* A ``Voter`` with all‑zero scores is invalid and will be rejected at
  normalisation time.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Optional


@dataclass(frozen=True)
class Option:
    """An option (candidate / list item) that can receive discrete slots.

    Parameters
    ----------
    id:
        Unique string identifier.
    group_id:
        Optional identifier of the ``Group`` this option belongs to.
        ``None`` means the option is ungrouped.
    """

    id: str
    group_id: Optional[str] = None


@dataclass(frozen=True)
class Group:
    """A logical grouping of options (e.g. a political party).

    Parameters
    ----------
    id:
        Unique string identifier.
    """

    id: str


@dataclass
class Voter:
    """A voter with graded preferences over a set of options.

    Parameters
    ----------
    id:
        Unique string identifier.
    scores:
        Mapping from ``option_id`` to a raw non‑negative score.
        Missing option keys are treated as a score of 0.

    Raises
    ------
    ValueError
        If any score is strictly negative.
    """

    id: str
    scores: dict[str, float] = field(default_factory=dict)

    def __post_init__(self) -> None:
        for opt_id, score in self.scores.items():
            if score < 0:
                raise ValueError(
                    f"Voter {self.id!r}: score for option {opt_id!r} "
                    f"must be non‑negative, got {score}"
                )
