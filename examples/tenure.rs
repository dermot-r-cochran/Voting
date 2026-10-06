//! Lot, then vote: how much of a chamber ends up long-serving?
//!
//! A toy model for `docs/lot-then-vote.md`. Each seat is contested every
//! `TERM` years. The candidates are a fresh lottery pool plus, if the rule
//! allows, the sitting member. The sitting member, when on the ballot, wins
//! with probability `p_inc` (the incumbency advantage); otherwise a pool
//! candidate takes the seat. Terms are staggered so a third of the seats fall
//! vacant each cycle.
//!
//! Rules compared:
//! - `always`: the sitting member may always stand (the original proposal)
//! - `cap2`: two terms and out
//! - `decay`: one automatic renewal, then a ticket that halves each term
//!
//! Nothing here is calibrated. It shows the shape of the problem, not its
//! size. The random source is a fixed-seed xorshift so the table is the same
//! on every run; this example has no bearing on the engine, which is
//! deterministic by construction and contains no randomness at all.
//!
//! Run with `cargo run --example tenure`.

use std::collections::BTreeMap;

const SEATS: usize = 60;
const TERM: usize = 5;
const YEARS: usize = 2000;
const WARM_UP_YEARS: usize = 50;

#[derive(Clone, Copy)]
enum Rule {
    Always,
    Cap2,
    Decay,
}

impl Rule {
    fn name(self) -> &'static str {
        match self {
            Rule::Always => "always",
            Rule::Cap2 => "cap2",
            Rule::Decay => "decay",
        }
    }
}

/// A xorshift64* generator: tiny, seedable, and not a dependency.
struct Rng(u64);

impl Rng {
    fn next_unit(&mut self) -> f64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        let bits = x.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 11;
        bits as f64 / (1u64 << 53) as f64
    }
}

struct Outcome {
    mean_years: f64,
    share_10_plus: f64,
    share_15_plus: f64,
    share_20_plus: f64,
}

fn run(rule: Rule, p_inc: f64) -> Outcome {
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    // Terms served so far, per seat. 1 is a member in a first term.
    let mut tenure = [1usize; SEATS];
    // Tenure distribution, sampled every cycle after the warm-up.
    let mut history: BTreeMap<usize, usize> = BTreeMap::new();
    let cycles = YEARS / TERM;
    for cycle in 0..cycles {
        for (seat, served) in tenure.iter_mut().enumerate() {
            if cycle % 3 != seat % 3 {
                continue; // not this seat's turn: staggered thirds
            }
            let t = *served;
            let on_ballot = match rule {
                Rule::Always => true,
                Rule::Cap2 => t < 2,
                Rule::Decay => t < 2 || rng.next_unit() < 0.5f64.powi(t as i32 - 1),
            };
            *served = if on_ballot && rng.next_unit() < p_inc {
                t + 1
            } else {
                1
            };
        }
        if cycle * TERM >= WARM_UP_YEARS {
            for &t in &tenure {
                *history.entry(t).or_insert(0) += 1;
            }
        }
    }
    let total: usize = history.values().sum();
    let share = |min_years: usize| -> f64 {
        let n: usize = history
            .iter()
            .filter(|(&t, _)| t * TERM >= min_years)
            .map(|(_, &n)| n)
            .sum();
        n as f64 / total as f64
    };
    let mean_years = history
        .iter()
        .map(|(&t, &n)| (t * TERM * n) as f64)
        .sum::<f64>()
        / total as f64;
    Outcome {
        mean_years,
        share_10_plus: share(10),
        share_15_plus: share(15),
        share_20_plus: share(20),
    }
}

fn main() {
    for p_inc in [0.7, 0.85] {
        println!("incumbency advantage p_inc={p_inc}, term={TERM} years, seats={SEATS}\n");
        println!(
            "{:8} {:>9} {:>8} {:>8} {:>8}",
            "rule", "mean yrs", "10+ yrs", "15+ yrs", "20+ yrs"
        );
        for rule in [Rule::Always, Rule::Cap2, Rule::Decay] {
            let o = run(rule, p_inc);
            println!(
                "{:8} {:9.1} {:7.0}% {:7.0}% {:7.0}%",
                rule.name(),
                o.mean_years,
                o.share_10_plus * 100.0,
                o.share_15_plus * 100.0,
                o.share_20_plus * 100.0
            );
        }
        println!();
    }
}
