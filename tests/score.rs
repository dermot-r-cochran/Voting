//! Unit tests for `Score`'s constructors - the crate's only parsing surface.
//!
//! `tests/allocation.rs` exercises scores incidentally; this file tests the
//! constructors directly, edge cases included, because a parsing bug here
//! silently mis-weights a ballot rather than failing anywhere visible.

use num_bigint::BigInt;
use num_rational::BigRational;
use voting::models::{Score, ScoreError};

fn rational(numerator: i64, denominator: i64) -> BigRational {
    BigRational::new(BigInt::from(numerator), BigInt::from(denominator))
}

#[test]
fn decimal_str_parses_integers_and_decimals_exactly() {
    assert_eq!(
        *Score::from_decimal_str("2").unwrap().as_rational(),
        rational(2, 1)
    );
    assert_eq!(
        *Score::from_decimal_str("0.1").unwrap().as_rational(),
        rational(1, 10)
    );
    assert_eq!(
        *Score::from_decimal_str("3.75").unwrap().as_rational(),
        rational(15, 4)
    );
    assert_eq!(
        *Score::from_decimal_str("0.0001").unwrap().as_rational(),
        rational(1, 10000)
    );
}

#[test]
fn decimal_str_accepts_lopsided_and_padded_forms() {
    // A trailing or leading dot with digits on the other side is a decimal.
    assert_eq!(
        *Score::from_decimal_str("1.").unwrap().as_rational(),
        rational(1, 1)
    );
    assert_eq!(
        *Score::from_decimal_str(".5").unwrap().as_rational(),
        rational(1, 2)
    );
    // Leading zeros, surrounding whitespace, and an explicit plus sign.
    assert_eq!(
        *Score::from_decimal_str("007").unwrap().as_rational(),
        rational(7, 1)
    );
    assert_eq!(
        *Score::from_decimal_str(" 1.5 ").unwrap().as_rational(),
        rational(3, 2)
    );
    assert_eq!(
        *Score::from_decimal_str("+2").unwrap().as_rational(),
        rational(2, 1)
    );
}

#[test]
fn decimal_str_rejects_non_decimals() {
    for bad in [
        "", ".", "+", "-", "abc", "1e3", "1.2.3", "1,5", "0x10", "1 2",
    ] {
        assert!(
            matches!(
                Score::from_decimal_str(bad),
                Err(ScoreError::NotADecimal(_))
            ),
            "expected NotADecimal for {bad:?}"
        );
    }
}

#[test]
fn decimal_str_rejects_negatives_but_accepts_negative_zero() {
    assert!(matches!(
        Score::from_decimal_str("-1"),
        Err(ScoreError::Negative(_))
    ));
    assert!(matches!(
        Score::from_decimal_str("-0.5"),
        Err(ScoreError::Negative(_))
    ));
    // -0 is zero, and zero is not negative.
    assert!(Score::from_decimal_str("-0").unwrap().is_zero());
    assert!(Score::from_decimal_str("-0.0").unwrap().is_zero());
}

#[test]
fn from_f64_uses_the_shortest_decimal_not_the_binary_expansion() {
    // The whole point of the constructor: 0.1 is 1/10, not
    // 0.1000000000000000055511151231257827021181583404541015625.
    assert_eq!(
        *Score::from_f64(0.1).unwrap().as_rational(),
        rational(1, 10)
    );
    assert_eq!(*Score::from_f64(2.5).unwrap().as_rational(), rational(5, 2));
    assert_eq!(*Score::from_f64(0.0).unwrap().as_rational(), rational(0, 1));
    assert_eq!(*Score::from_f64(3.0).unwrap().as_rational(), rational(3, 1));
}

#[test]
fn from_f64_rejects_non_finite_and_negative_values() {
    assert!(matches!(
        Score::from_f64(f64::NAN),
        Err(ScoreError::NotFinite(_))
    ));
    assert!(matches!(
        Score::from_f64(f64::INFINITY),
        Err(ScoreError::NotFinite(_))
    ));
    assert!(matches!(
        Score::from_f64(f64::NEG_INFINITY),
        Err(ScoreError::NotFinite(_))
    ));
    assert!(matches!(
        Score::from_f64(-1.0),
        Err(ScoreError::Negative(_))
    ));
    // -0.0 formats as "-0" and is zero, not negative.
    assert!(Score::from_f64(-0.0).unwrap().is_zero());
}

#[test]
fn from_f64_expands_exponent_notation_rather_than_rejecting_it() {
    // Rust formats these magnitudes with an exponent; format_shortest expands
    // them so the decimal parser still succeeds.
    let large = Score::from_f64(1e300).unwrap();
    assert_eq!(
        *large.as_rational(),
        BigRational::from_integer(BigInt::from(10u8).pow(300))
    );
    let small = Score::from_f64(1e-30).unwrap();
    assert_eq!(
        *small.as_rational(),
        BigRational::new(BigInt::from(1), BigInt::from(10u8).pow(30))
    );
}

#[test]
fn from_rational_rejects_negatives() {
    assert!(matches!(
        Score::from_rational(rational(-1, 2)),
        Err(ScoreError::Negative(_))
    ));
    assert_eq!(
        *Score::from_rational(rational(1, 3)).unwrap().as_rational(),
        rational(1, 3)
    );
}

#[test]
fn score_error_messages_name_the_offending_value() {
    assert_eq!(
        Score::from_decimal_str("abc").unwrap_err().to_string(),
        "score \"abc\" is not a decimal number"
    );
    assert_eq!(
        Score::from_decimal_str("-2").unwrap_err().to_string(),
        "score must be non-negative, got -2"
    );
    assert_eq!(
        Score::from_f64(f64::INFINITY).unwrap_err().to_string(),
        "score must be finite, got inf"
    );
}
