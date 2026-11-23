//! Layer 0 held to its word: the invariant checks are `assert!`, in every build.
//!
//! The README, `TestingStrategy.md` and the module docs of
//! `src/allocation.rs` all say the checks are `assert!`, not `debug_assert!`,
//! and that `Cargo.toml` keeps `debug-assertions` on in release. Nothing held
//! either statement. `cargo test` builds with debug assertions on, so a check
//! downgraded to `debug_assert!` passes every other test in this repository
//! and then vanishes from a release build; a check deleted outright passes
//! them too, because the checks exist to catch the engine misbehaving and no
//! input reaches them while it behaves.
//!
//! The `assert!` carries more than the setting does. Cargo reads profiles
//! from the root manifest of a build and ignores a dependency's, so in a crate
//! that depends on this one the `[profile.release]` here does nothing, and
//! the `assert!` is the whole guarantee.
//!
//! No run can observe the difference from outside, so this reads the source,
//! the way `tests/docs.rs` reads the prose. Standard library only.

use std::fs;
use std::path::PathBuf;

/// The checks `src/allocation.rs` makes, each named by the words its panic
/// message opens with. The first two are the weight invariant (per voter,
/// then in aggregate), the third is surplus conservation, and the last two
/// are slot conservation (the fallback pass, then the final total).
const CHECKS: [&str; 5] = [
    "normalisation invariant violated",
    "aggregation invariant violated",
    "surplus-conservation invariant violated",
    "largest-remainder fallback left",
    "slot-conservation invariant violated",
];

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(relative: &str) -> String {
    fs::read_to_string(root().join(relative)).unwrap_or_else(|_| panic!("read {relative}"))
}

/// Every `.rs` file under `src/`, as `(path relative to the root, text)`.
fn sources() -> Vec<(String, String)> {
    let mut found = Vec::new();
    let mut pending = vec![root().join("src")];
    while let Some(dir) = pending.pop() {
        for entry in fs::read_dir(&dir).expect("read src").flatten() {
            let path = entry.path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                let relative = path
                    .strip_prefix(root())
                    .unwrap_or(&path)
                    .display()
                    .to_string()
                    .replace('\\', "/");
                let text = fs::read_to_string(&path).expect("read source");
                found.push((relative, text));
            }
        }
    }
    found.sort();
    found
}

/// The source with every `//` comment removed, line for line, so that prose
/// which names `debug_assert!` (the module docs do) is not mistaken for code.
fn code(source: &str) -> String {
    source
        .lines()
        .map(|line| line.split("//").next().unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Line numbers, from 1, on which the code uses any `debug_assert` macro.
fn debug_assert_lines(source: &str) -> Vec<usize> {
    code(source)
        .lines()
        .enumerate()
        .filter(|(_, line)| line.contains("debug_assert"))
        .map(|(index, _)| index + 1)
        .collect()
}

/// Every plain `assert!( ... );` statement in the code. A `debug_assert!(`
/// is not one: the character before `assert!(` must not belong to a name.
fn assert_statements(source: &str) -> Vec<String> {
    let code = code(source);
    let mut statements = Vec::new();
    let mut from = 0;
    while let Some(offset) = code[from..].find("assert!(") {
        let start = from + offset;
        let part_of_a_longer_name = code[..start]
            .chars()
            .next_back()
            .is_some_and(|c| c.is_alphanumeric() || c == '_');
        let end = code[start..].find(';').map_or(code.len(), |i| start + i);
        if !part_of_a_longer_name {
            statements.push(code[start..end].to_string());
        }
        from = start + "assert!(".len();
    }
    statements
}

/// The checks in `CHECKS` that no plain `assert!` in the source carries.
fn missing_checks(source: &str) -> Vec<&'static str> {
    let statements = assert_statements(source);
    CHECKS
        .iter()
        .copied()
        .filter(|message| !statements.iter().any(|s| s.contains(message)))
        .collect()
}

/// Whether the manifest's `[profile.release]` sets `debug-assertions = true`.
fn release_keeps_debug_assertions(manifest: &str) -> bool {
    manifest
        .lines()
        .map(|line| line.split('#').next().unwrap_or("").trim())
        .skip_while(|line| *line != "[profile.release]")
        .skip(1)
        .take_while(|line| !line.starts_with('['))
        .any(|line| line.split_whitespace().collect::<String>() == "debug-assertions=true")
}

#[test]
fn no_invariant_check_is_a_debug_assert() {
    let mut found = Vec::new();
    for (path, text) in sources() {
        for line in debug_assert_lines(&text) {
            found.push(format!("{path}:{line}"));
        }
    }
    assert!(
        found.is_empty(),
        "debug_assert in src/, which a release build compiles out: {}. The \
         invariant checks are assert! so that no build profile can remove them.",
        found.join(", ")
    );
}

#[test]
fn every_invariant_check_keeps_its_assert() {
    let missing = missing_checks(&read("src/allocation.rs"));
    assert!(
        missing.is_empty(),
        "src/allocation.rs no longer carries a plain assert! for: {}. If a \
         check was reworded or replaced on purpose, change CHECKS in this \
         file in the same pull request, so that the change is a reviewed \
         decision; a check that was dropped or downgraded goes back.",
        missing.join("; ")
    );
}

#[test]
fn the_release_profile_keeps_debug_assertions() {
    assert!(
        release_keeps_debug_assertions(&read("Cargo.toml")),
        "Cargo.toml no longer sets debug-assertions = true under \
         [profile.release], which the README and TestingStrategy.md say it does"
    );
}

/// The three checks above are known to be able to fail: each is run here on
/// a copy of the real file with exactly the weakening it exists to catch.
#[test]
fn each_check_fails_on_a_weakened_copy() {
    let allocation = read("src/allocation.rs");
    assert!(debug_assert_lines(&allocation).is_empty());
    assert!(missing_checks(&allocation).is_empty());

    // One check downgraded: caught twice, as a debug_assert in the code and
    // as a check with no plain assert! left.
    let downgraded = allocation.replacen("    assert!(", "    debug_assert!(", 1);
    assert_ne!(downgraded, allocation);
    assert_eq!(debug_assert_lines(&downgraded).len(), 1);
    assert_eq!(
        missing_checks(&downgraded),
        ["normalisation invariant violated"]
    );

    // One check deleted: its message is gone from every assert! statement.
    let deleted = allocation.replace("slot-conservation invariant violated", "");
    assert_eq!(
        missing_checks(&deleted),
        ["slot-conservation invariant violated"]
    );

    // The same words in a comment are not a check.
    let commented = "// assert!(false, \"aggregation invariant violated\");";
    assert!(missing_checks(commented).contains(&"aggregation invariant violated"));

    let manifest = read("Cargo.toml");
    assert!(release_keeps_debug_assertions(&manifest));
    for weakened in [
        manifest.replace("debug-assertions = true", "debug-assertions = false"),
        manifest.replace("debug-assertions = true", "# debug-assertions = true"),
        manifest.replace("[profile.release]", "[profile.dev]"),
    ] {
        assert_ne!(weakened, manifest);
        assert!(!release_keeps_debug_assertions(&weakened));
    }
}
