//! The design note's sizing table follows the rule stated above it.
//!
//! `docs/lot-then-vote.md` ("Sizing") gives a rule in words - the lower
//! house has as many seats as the cube root of the voting population, the
//! upper house about a third of that, the cabinet about a quarter of the
//! upper house again - and then a table of electorates worked through it.
//! Until this file nothing held the table to the rule: the doc check reads
//! links, counts and citations, and `tests/tenure.rs` holds the note's other
//! table. A row added or edited with a slip in it would have shipped.
//!
//! This file reads the table out of the note itself, so an edit to the note
//! is what it tests. Each row is recomputed from its own population:
//!
//! - the lower house is the whole number nearest the cube root;
//! - the upper house is the whole number nearest a third of the lower house;
//! - the cabinet is the whole number nearest a quarter of the upper house
//!   (where a quarter falls exactly half way, either neighbour is "about").
//!
//! The arithmetic is exact, in integers, in keeping with the crate: a
//! population such as "3.5 million" is read as a decimal string, never as a
//! float, and "nearest the cube root" is checked by cubing, not by rooting.
//!
//! Nearest is the table's own practice, not a reading added here: the note's
//! word is "about", and every cell in every row rounds to the nearest whole
//! number (152 for Ireland rules out rounding down, 368 and 621 rule out
//! rounding up).
//!
//! Not checked: the note sets Ireland's real Dáil, Seanad and cabinet
//! ceiling beside the row for its electorate and says they sit close to it.
//! The figures are facts about Ireland, which no tree can check, and how
//! close is close is the author's judgement.

use std::fs;
use std::path::Path;

/// One row of the note's table.
struct Row {
    label: String,
    population: u128,
    lower: u128,
    upper: u128,
    cabinet: u128,
}

/// The note's "Sizing" section, heading excluded, up to the next heading.
fn sizing_section() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("docs/lot-then-vote.md");
    let text = fs::read_to_string(path).expect("docs/lot-then-vote.md");
    let mut lines = text
        .lines()
        .skip_while(|line| !line.starts_with("## Sizing"));
    lines.next().expect("the note has a Sizing section");
    lines
        .take_while(|line| !line.starts_with("## "))
        .collect::<Vec<_>>()
        .join("\n")
}

/// A population cell such as "3.5 million (Ireland)", as an exact count.
fn population(cell: &str) -> u128 {
    let mut words = cell.split_whitespace();
    let figure = words.next().unwrap_or("");
    let zeros = match words.next() {
        Some("thousand") => 3,
        Some("million") => 6,
        Some("billion") => 9,
        _ => panic!("population {cell:?} is not a figure and a unit"),
    };
    let (whole, fraction) = figure.split_once('.').unwrap_or((figure, ""));
    assert!(
        fraction.len() <= zeros,
        "population {cell:?} is not a whole number of people"
    );
    format!("{whole}{fraction}{}", "0".repeat(zeros - fraction.len()))
        .parse()
        .unwrap_or_else(|_| panic!("population {cell:?} is not a decimal figure"))
}

/// The table's rows, header and separator lines excluded.
fn rows(section: &str) -> Vec<Row> {
    section
        .lines()
        .filter(|line| line.trim_start().starts_with('|'))
        .skip(2)
        .map(|line| {
            let cells: Vec<&str> = line
                .trim()
                .trim_matches('|')
                .split('|')
                .map(str::trim)
                .collect();
            assert_eq!(cells.len(), 4, "row {line:?} does not have four cells");
            let seats = |cell: &str| -> u128 {
                cell.parse()
                    .unwrap_or_else(|_| panic!("{cell:?} in row {line:?} is not a seat count"))
            };
            Row {
                label: cells[0].to_string(),
                population: population(cells[0]),
                lower: seats(cells[1]),
                upper: seats(cells[2]),
                cabinet: seats(cells[3]),
            }
        })
        .collect()
}

/// Whether `seats` is the whole number nearest the cube root of
/// `population`: seats - 1/2 <= root <= seats + 1/2, cubed and cleared of
/// halves so that it stays in integers.
fn is_nearest_the_cube_root(seats: u128, population: u128) -> bool {
    let below = (2 * seats).saturating_sub(1).pow(3);
    let above = (2 * seats + 1).pow(3);
    below <= 8 * population && 8 * population <= above
}

/// Whether `part` is the whole number nearest a third of `whole`.
fn is_about_a_third(part: u128, whole: u128) -> bool {
    (3 * part).abs_diff(whole) <= 1
}

/// Whether `part` is a whole number nearest a quarter of `whole`. A quarter
/// that falls exactly half way has two nearest neighbours, and both pass.
fn is_about_a_quarter(part: u128, whole: u128) -> bool {
    (4 * part).abs_diff(whole) <= 2
}

#[test]
fn the_sizing_table_in_the_note_follows_its_rule() {
    let rows = rows(&sizing_section());
    assert!(!rows.is_empty(), "the Sizing section has no table rows");
    for row in &rows {
        assert!(
            is_nearest_the_cube_root(row.lower, row.population),
            "{}: a lower house of {} is not the cube root of {}",
            row.label,
            row.lower,
            row.population
        );
        assert!(
            is_about_a_third(row.upper, row.lower),
            "{}: an upper house of {} is not a third of {}",
            row.label,
            row.upper,
            row.lower
        );
        assert!(
            is_about_a_quarter(row.cabinet, row.upper),
            "{}: a cabinet of {} is not a quarter of {}",
            row.label,
            row.cabinet,
            row.upper
        );
    }
}

#[test]
fn the_checks_reject_a_figure_that_is_one_out() {
    // A check that cannot fail proves nothing, so each is shown failing on
    // the Ireland row with one seat added or taken away.
    assert_eq!(population("3.5 million (Ireland)"), 3_500_000);
    assert_eq!(population("240 million"), 240_000_000);

    assert!(is_nearest_the_cube_root(152, 3_500_000));
    assert!(!is_nearest_the_cube_root(151, 3_500_000));
    assert!(!is_nearest_the_cube_root(153, 3_500_000));

    assert!(is_about_a_third(51, 152));
    assert!(!is_about_a_third(50, 152));
    assert!(!is_about_a_third(52, 152));

    assert!(is_about_a_quarter(13, 51));
    assert!(!is_about_a_quarter(12, 51));
    assert!(!is_about_a_quarter(14, 51));
    // Half way: a quarter of 50 is 12.5, and 12 and 13 are equally near.
    assert!(is_about_a_quarter(12, 50));
    assert!(is_about_a_quarter(13, 50));
    assert!(!is_about_a_quarter(11, 50));
}
