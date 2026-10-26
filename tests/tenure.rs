//! The design note's table is what the toy model prints.
//!
//! `docs/lot-then-vote.md` ("What the toy model shows") states, in words,
//! the share of a 60-seat chamber with fifteen or more years' service under
//! each renewal rule, at two incumbency advantages, as `cargo run --example
//! tenure` reports it. Nothing else holds the note to the model: the example
//! is a binary, the doc check reads prose, and a change to the model's
//! constants or rules would leave the table stale without failing anywhere.
//! This file includes the example as a module and checks the table against
//! `run` directly.
//!
//! The note says to read the ordering and not the digits, so the ordering
//! is the strict claim here (two terms and out: none; a decaying ticket:
//! fewer than always-eligible, more than none), and each worded cell is
//! held to within a quarter of the fraction it names. "About a sixth" is
//! satisfied by 14%, not by 10% or 25%.

#[allow(dead_code)]
#[path = "../examples/tenure.rs"]
mod tenure;

use tenure::{run, Outcome, Rule, SEATS, TERM, YEARS};

/// A worded share from the note's table, as a fraction.
fn about(fraction: f64) -> impl Fn(f64) -> bool {
    move |share| (share - fraction).abs() <= fraction / 4.0
}

fn table(p_inc: f64) -> (Outcome, Outcome, Outcome) {
    (
        run(Rule::Always, p_inc),
        run(Rule::Cap2, p_inc),
        run(Rule::Decay, p_inc),
    )
}

#[test]
fn the_clock_in_the_note_matches_the_constants() {
    // "a 60-seat chamber, a third of the seats contested each cycle, for
    // about 133 elections per seat", "read as five years a term".
    assert_eq!(SEATS, 60);
    assert_eq!(TERM, 5);
    assert_eq!(YEARS / TERM / 3, 133);
}

#[test]
fn the_tenure_table_in_the_note_matches_the_model() {
    // | Rule | Incumbent wins 70% | Incumbent wins 85% |
    // | Always eligible | about half | about three quarters |
    // | Two terms and out | none | none |
    // | One renewal, then a decaying ticket | about a sixth | about a fifth |
    let (always, cap2, decay) = table(0.7);
    assert!(
        about(0.5)(always.share_15_plus),
        "always eligible at 70%: {:.0}% is not about half",
        always.share_15_plus * 100.0
    );
    assert_eq!(
        cap2.share_15_plus, 0.0,
        "two terms and out at 70%: not none"
    );
    assert!(
        about(1.0 / 6.0)(decay.share_15_plus),
        "decaying ticket at 70%: {:.0}% is not about a sixth",
        decay.share_15_plus * 100.0
    );
    assert!(cap2.share_15_plus < decay.share_15_plus);
    assert!(decay.share_15_plus < always.share_15_plus);

    let (always, cap2, decay) = table(0.85);
    assert!(
        about(0.75)(always.share_15_plus),
        "always eligible at 85%: {:.0}% is not about three quarters",
        always.share_15_plus * 100.0
    );
    assert_eq!(
        cap2.share_15_plus, 0.0,
        "two terms and out at 85%: not none"
    );
    assert!(
        about(0.2)(decay.share_15_plus),
        "decaying ticket at 85%: {:.0}% is not about a fifth",
        decay.share_15_plus * 100.0
    );
    assert!(cap2.share_15_plus < decay.share_15_plus);
    assert!(decay.share_15_plus < always.share_15_plus);
}

#[test]
fn two_terms_and_out_never_reaches_a_third_term() {
    // The cap is a hard ceiling, so every longer-service share is exactly
    // zero and the mean cannot exceed two terms.
    for p_inc in [0.7, 0.85] {
        let cap2 = run(Rule::Cap2, p_inc);
        assert_eq!(cap2.share_15_plus, 0.0);
        assert_eq!(cap2.share_20_plus, 0.0);
        assert!(cap2.mean_years <= (2 * TERM) as f64);
        assert!(cap2.share_10_plus > 0.0, "a second term is still possible");
    }
}

#[test]
fn the_fixed_seed_makes_the_table_reproducible() {
    // "The random source is a fixed-seed xorshift so the table is the same
    // on every run."
    for rule in [Rule::Always, Rule::Cap2, Rule::Decay] {
        let first = run(rule, 0.7);
        let second = run(rule, 0.7);
        assert_eq!(first.mean_years, second.mean_years);
        assert_eq!(first.share_10_plus, second.share_10_plus);
        assert_eq!(first.share_15_plus, second.share_15_plus);
        assert_eq!(first.share_20_plus, second.share_20_plus);
    }
}
