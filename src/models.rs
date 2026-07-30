//! Data models for the proportional allocation engine.
//!
//! Entities
//! --------
//! [`Candidate`] – an item that can receive one or more slots.
//! [`Group`]     – a logical grouping of candidates (optional).
//! [`Voter`]     – holds a non-negative [`Score`] for each candidate it supports.
//!
//! Invariants
//! ----------
//! * Every score is non-negative. This is enforced by [`Score`]'s constructors
//!   rather than checked after the fact: a negative score has no
//!   representation, so no later stage has to re-check for one.
//! * A voter whose scores are all zero is invalid, and is rejected at
//!   normalisation time rather than at construction — a voter may legitimately
//!   be built with no scores at all, and only becomes a zero-sum ballot once a
//!   candidate set is applied to it.
//!
//! Naming note: the Python original calls a slot recipient `Option`. That name
//! is taken by the prelude here, and shadowing it would make every `Option<T>`
//! in the crate ambiguous to a reader. `Candidate` is the same concept.

use std::collections::BTreeMap;

use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{Signed, Zero};

/// A candidate (option / list item) that can receive discrete slots.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Candidate {
    /// Unique identifier.
    pub id: String,
    /// Identifier of the [`Group`] this candidate belongs to, if any.
    pub group_id: Option<String>,
}

impl Candidate {
    /// A candidate belonging to no group.
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            group_id: None,
        }
    }

    /// A candidate belonging to `group_id`.
    pub fn in_group(id: impl Into<String>, group_id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            group_id: Some(group_id.into()),
        }
    }
}

/// A logical grouping of candidates (e.g. a political party).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Group {
    /// Unique identifier.
    pub id: String,
}

impl Group {
    pub fn new(id: impl Into<String>) -> Self {
        Self { id: id.into() }
    }
}

/// Error returned when a score cannot be constructed.
///
/// Not `Eq`: the `NotFinite` variant carries the offending `f64`, and `f64`
/// has no total equality (NaN != NaN), which is the very case that variant
/// reports.
#[derive(Debug, Clone, PartialEq)]
pub enum ScoreError {
    /// The value was strictly negative.
    Negative(String),
    /// The text was not a decimal number.
    NotADecimal(String),
    /// The value was a float that has no decimal representation (NaN, ±∞).
    NotFinite(f64),
}

impl std::fmt::Display for ScoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Negative(v) => write!(f, "score must be non-negative, got {v}"),
            Self::NotADecimal(s) => write!(f, "score {s:?} is not a decimal number"),
            Self::NotFinite(v) => write!(f, "score must be finite, got {v}"),
        }
    }
}

impl std::error::Error for ScoreError {}

/// A raw, non-negative score for one candidate.
///
/// Held as an exact rational. The type exists so that "non-negative" is a
/// property of the value rather than of having remembered to validate it:
/// once you hold a `Score`, there is nothing left to check.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Score(BigRational);

impl Score {
    /// Zero.
    pub fn zero() -> Self {
        Self(BigRational::zero())
    }

    /// A score from a non-negative integer.
    pub fn from_int(value: u64) -> Self {
        Self(BigRational::from_integer(BigInt::from(value)))
    }

    /// A score from an exact rational, rejecting negatives.
    pub fn from_rational(value: BigRational) -> Result<Self, ScoreError> {
        if value.is_negative() {
            return Err(ScoreError::Negative(value.to_string()));
        }
        Ok(Self(value))
    }

    /// A score from decimal text such as `"2"`, `"0.1"` or `"3.75"`.
    ///
    /// This is the exact constructor: `"0.1"` becomes precisely 1/10, with no
    /// binary floating-point step in between.
    pub fn from_decimal_str(text: &str) -> Result<Self, ScoreError> {
        let t = text.trim();
        let err = || ScoreError::NotADecimal(text.to_string());

        let (sign, digits) = match t.strip_prefix('-') {
            Some(rest) => (-1i8, rest),
            None => (1i8, t.strip_prefix('+').unwrap_or(t)),
        };

        let (int_part, frac_part) = match digits.split_once('.') {
            Some((i, f)) => (i, f),
            None => (digits, ""),
        };
        if int_part.is_empty() && frac_part.is_empty() {
            return Err(err());
        }
        if !int_part.chars().all(|c| c.is_ascii_digit())
            || !frac_part.chars().all(|c| c.is_ascii_digit())
        {
            return Err(err());
        }

        let combined = format!("{int_part}{frac_part}");
        let numerator: BigInt = if combined.is_empty() {
            BigInt::from(0)
        } else {
            combined.parse().map_err(|_| err())?
        };
        let denominator = BigInt::from(10u8).pow(frac_part.len() as u32);
        let value = BigRational::new(numerator, denominator);

        Self::from_rational(if sign < 0 { -value } else { value })
    }

    /// A score from an `f64`, via its shortest decimal representation.
    ///
    /// This reproduces the Python original's `Fraction(str(score))`: `0.1`
    /// becomes 1/10, not the binary double 0.1000000000000000055511151231…
    /// Prefer [`Score::from_decimal_str`] or [`Score::from_int`] in new code —
    /// this constructor exists so ballots already held as floats port across
    /// with identical results.
    pub fn from_f64(value: f64) -> Result<Self, ScoreError> {
        if !value.is_finite() {
            return Err(ScoreError::NotFinite(value));
        }
        Self::from_decimal_str(&format_shortest(value))
    }

    /// The score as an exact rational.
    pub fn as_rational(&self) -> &BigRational {
        &self.0
    }

    /// Whether this score is zero.
    pub fn is_zero(&self) -> bool {
        self.0.is_zero()
    }
}

/// Format an `f64` the way Python's `str()` does for the cases that matter
/// here: shortest representation that round-trips, without exponent notation
/// for the magnitudes a ballot score realistically takes.
fn format_shortest(value: f64) -> String {
    let s = format!("{value}");
    if s.contains(['e', 'E']) {
        // Rust switches to exponent form only for extreme magnitudes. Expand
        // it rather than failing, so from_f64 never rejects a finite value.
        return format!("{value:.*}", 340);
    }
    s
}

/// A voter with graded preferences over a set of candidates.
///
/// A candidate absent from `scores` is treated as scored zero.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Voter {
    /// Unique identifier.
    pub id: String,
    /// Mapping from candidate id to that candidate's score.
    pub scores: BTreeMap<String, Score>,
}

impl Voter {
    /// A voter with no scores yet.
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            scores: BTreeMap::new(),
        }
    }

    /// A voter built from `(candidate_id, score)` pairs.
    pub fn with_scores<I, S>(id: impl Into<String>, scores: I) -> Self
    where
        I: IntoIterator<Item = (S, Score)>,
        S: Into<String>,
    {
        Self {
            id: id.into(),
            scores: scores.into_iter().map(|(k, v)| (k.into(), v)).collect(),
        }
    }

    /// A voter built from `(candidate_id, f64)` pairs, for ballots already
    /// held as floats. See [`Score::from_f64`] for the conversion rule.
    pub fn with_f64_scores<I, S>(id: impl Into<String>, scores: I) -> Result<Self, ScoreError>
    where
        I: IntoIterator<Item = (S, f64)>,
        S: Into<String>,
    {
        let mut map = BTreeMap::new();
        for (candidate, value) in scores {
            map.insert(candidate.into(), Score::from_f64(value)?);
        }
        Ok(Self {
            id: id.into(),
            scores: map,
        })
    }

    /// This voter's score for `candidate_id`, or zero if unscored.
    pub fn score_for(&self, candidate_id: &str) -> BigRational {
        self.scores
            .get(candidate_id)
            .map(|s| s.as_rational().clone())
            .unwrap_or_else(BigRational::zero)
    }
}
