//! Golden test: the Rust engine must reproduce the Python engine's allocations.
//!
//! `tests/golden/expected.txt` was produced by the Python implementation that
//! this crate replaced, on 2026-07-30, immediately before it was deleted. The
//! two agreed on all 300 scenarios at that moment. Committing its output turns
//! that one-off agreement into a permanent specification: the behaviour
//! survives the implementation, and any future change that alters an
//! allocation fails here rather than silently shipping.
//!
//! The scenarios cover 1-6 candidates, 1-12 voters, 1-12 slots, integral and
//! four-decimal scores, optional groups, and five hand-built edge cases (a
//! single candidate, an exact Droop quota, 100 slots over two candidates,
//! decimal thirds, and an all-tied field).
//!
//! What is NOT covered here: rejection paths. Every generated scenario is
//! valid input, so zero-sum ballots and non-positive slot counts are exercised
//! by the unit tests in `tests/allocation.rs` rather than by this file.
//!
//! Format, one case per block:
//!   C <case_id> <n_slots>
//!   O <candidate_id> <group_id or ->
//!   V <voter_id> <candidate_id>:<decimal score> ...
//!   E

use std::collections::BTreeMap;

use voting::allocation::allocate_slots;
use voting::models::{Candidate, Score, Voter};

struct Case {
    id: String,
    n_slots: i64,
    candidates: Vec<Candidate>,
    voters: Vec<Voter>,
}

fn parse_cases(text: &str) -> Vec<Case> {
    let mut cases = Vec::new();
    let mut current: Option<Case> = None;

    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let mut parts = line.split_whitespace();
        let tag = parts.next().expect("non-empty line has a tag");

        match tag {
            "C" => {
                let id = parts.next().expect("case id").to_string();
                let n_slots = parts
                    .next()
                    .expect("slot count")
                    .parse()
                    .expect("slot count is an integer");
                current = Some(Case {
                    id,
                    n_slots,
                    candidates: Vec::new(),
                    voters: Vec::new(),
                });
            }
            "O" => {
                let case = current.as_mut().expect("candidate line inside a case");
                let id = parts.next().expect("candidate id");
                let group = parts.next().expect("group field");
                case.candidates.push(if group == "-" {
                    Candidate::new(id)
                } else {
                    Candidate::in_group(id, group)
                });
            }
            "V" => {
                let case = current.as_mut().expect("voter line inside a case");
                let id = parts.next().expect("voter id");
                let mut scores: BTreeMap<String, Score> = BTreeMap::new();
                for pair in parts {
                    let (candidate, value) =
                        pair.split_once(':').expect("score pair is candidate:value");
                    scores.insert(
                        candidate.to_string(),
                        Score::from_decimal_str(value).expect("non-negative decimal score"),
                    );
                }
                case.voters.push(Voter::with_scores(id, scores));
            }
            "E" => cases.push(current.take().expect("a case to close")),
            other => panic!("unknown tag {other:?} in golden cases"),
        }
    }

    assert!(current.is_none(), "final case is missing its E terminator");
    cases
}

fn parse_expected(text: &str) -> BTreeMap<String, String> {
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|line| {
            let (id, result) = line
                .trim()
                .split_once(' ')
                .expect("expected line is '<case_id> <result>'");
            (id.to_string(), result.to_string())
        })
        .collect()
}

#[test]
fn matches_the_python_engine_case_for_case() {
    let cases = parse_cases(include_str!("golden/cases.txt"));
    let expected = parse_expected(include_str!("golden/expected.txt"));

    assert_eq!(cases.len(), 300, "golden case count changed unexpectedly");
    assert_eq!(expected.len(), cases.len(), "one expectation per case");

    let mut mismatches = Vec::new();

    for case in &cases {
        let actual = match allocate_slots(&case.voters, &case.candidates, case.n_slots) {
            Ok(result) => result
                .iter()
                .map(|(candidate, slots)| format!("{candidate}={slots}"))
                .collect::<Vec<_>>()
                .join(","),
            // The Python side raised ValueError for every rejection, so the
            // recorded expectation is the fact of refusal, not its variant.
            Err(_) => "ERROR:ValueError".to_string(),
        };

        let want = expected
            .get(&case.id)
            .unwrap_or_else(|| panic!("no expectation recorded for case {}", case.id));

        if &actual != want {
            mismatches.push(format!(
                "  {}\n    python: {}\n    rust  : {}",
                case.id, want, actual
            ));
        }
    }

    assert!(
        mismatches.is_empty(),
        "{} case(s) diverge from the Python engine:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}
