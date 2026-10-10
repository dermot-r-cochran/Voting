//! Doc check: the Markdown in this repository agrees with the tree.
//!
//! Standard library only, run by `cargo test` and therefore by CI. It reads
//! `README.md`, `TestingStrategy.md`, `CLAUDE.md` and everything under
//! `docs/`, and fails if:
//!
//! 1. a relative link does not resolve to a file;
//! 2. a file carries more than one front-matter block (a `---` line followed
//!    by `key:` lines and closed by `---`, which is what a stray fragment left
//!    by a merge looks like);
//! 3. a count the prose states does not match the tree: "N scenarios" must be
//!    the number of golden cases in `tests/golden/cases.txt`, and
//!    "N properties" the number of properties in `tests/properties.rs`;
//! 4. a test cited as `tests/<file>.rs::<name>` does not exist.
//!
//! External links (`http`, `https`, `mailto`) and in-page anchors are not
//! checked. Fenced code blocks are skipped throughout; inline code is skipped
//! for links and counts but is exactly where test citations live.

use std::fs;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn markdown_files() -> Vec<PathBuf> {
    let root = root();
    let mut files: Vec<PathBuf> = ["README.md", "TestingStrategy.md", "CLAUDE.md"]
        .iter()
        .map(|name| root.join(name))
        .filter(|path| path.is_file())
        .collect();
    let mut pending = vec![root.join("docs")];
    while let Some(dir) = pending.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|ext| ext == "md") {
                files.push(path);
            }
        }
    }
    files.sort();
    files
}

/// Lines outside fenced code blocks, numbered from 1.
fn prose(text: &str) -> Vec<(usize, &str)> {
    let mut kept = Vec::new();
    let mut fenced = false;
    for (index, line) in text.lines().enumerate() {
        if line.trim_start().starts_with("```") {
            fenced = !fenced;
            continue;
        }
        if !fenced {
            kept.push((index + 1, line));
        }
    }
    kept
}

/// The line with every `inline code` span blanked out.
fn without_inline_code(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut in_code = false;
    for ch in line.chars() {
        if ch == '`' {
            in_code = !in_code;
            out.push(' ');
        } else if in_code {
            out.push(' ');
        } else {
            out.push(ch);
        }
    }
    out
}

/// Targets of every `[text](target)` link on the line.
fn link_targets(line: &str) -> Vec<String> {
    let mut targets = Vec::new();
    let mut rest = line;
    while let Some(start) = rest.find("](") {
        let after = &rest[start + 2..];
        let end = after
            .find(|c: char| c == ')' || c.is_whitespace())
            .unwrap_or(after.len());
        let target = after[..end].trim_start_matches('<').trim_end_matches('>');
        if !target.is_empty() {
            targets.push(target.to_string());
        }
        rest = &after[end..];
    }
    targets
}

fn is_front_matter_key(line: &str) -> bool {
    let mut chars = line.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_alphabetic() || c == '_')
        && line.split_once(':').is_some_and(|(key, _)| {
            key.chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        })
}

fn front_matter_blocks(text: &str) -> usize {
    let lines: Vec<&str> = text.lines().collect();
    let mut count = 0;
    let mut i = 0;
    while i < lines.len() {
        if lines[i].trim() == "---" && i + 1 < lines.len() && is_front_matter_key(lines[i + 1]) {
            if let Some(close) = lines[i + 1..].iter().position(|line| line.trim() == "---") {
                count += 1;
                i += close + 2;
                continue;
            }
        }
        i += 1;
    }
    count
}

fn number(token: &str) -> Option<usize> {
    let word = token.trim_matches(|c: char| !c.is_ascii_alphanumeric());
    if let Ok(n) = word.parse() {
        return Some(n);
    }
    let words = [
        "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten", "eleven",
        "twelve",
    ];
    words
        .iter()
        .position(|w| w.eq_ignore_ascii_case(word))
        .map(|i| i + 1)
}

/// Every "N <noun>" the prose states, with up to one adjective between.
fn stated_counts(prose_text: &str, noun: &str) -> Vec<usize> {
    let tokens: Vec<&str> = prose_text.split_whitespace().collect();
    let mut found = Vec::new();
    for (i, token) in tokens.iter().enumerate() {
        if token.trim_matches(|c: char| !c.is_ascii_alphanumeric()) != noun {
            continue;
        }
        for back in 1..=2 {
            let Some(previous) = i.checked_sub(back).map(|j| tokens[j]) else {
                break;
            };
            if let Some(n) = number(previous) {
                found.push(n);
                break;
            }
            if !previous.chars().all(|c| c.is_ascii_alphabetic()) {
                break;
            }
        }
    }
    found
}

/// Every `tests/<file>.rs::<name>` citation on the line.
fn citations(line: &str) -> Vec<(String, String)> {
    let mut found = Vec::new();
    let mut rest = line;
    while let Some(start) = rest.find("tests/") {
        let after = &rest[start + "tests/".len()..];
        let end = after
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '.' || c == '/'))
            .unwrap_or(after.len());
        let file = &after[..end];
        let tail = &after[end..];
        if file.ends_with(".rs") && tail.starts_with("::") {
            let name_end = tail[2..]
                .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
                .unwrap_or(tail.len() - 2);
            found.push((file.to_string(), tail[2..2 + name_end].to_string()));
        }
        rest = after;
    }
    found
}

fn golden_case_count() -> usize {
    fs::read_to_string(root().join("tests/golden/cases.txt"))
        .expect("tests/golden/cases.txt")
        .lines()
        .filter(|line| line.starts_with("C "))
        .count()
}

fn property_count() -> usize {
    let text = fs::read_to_string(root().join("tests/properties.rs")).expect("tests/properties.rs");
    let Some(start) = text.find("proptest! {") else {
        return 0;
    };
    text[start..]
        .lines()
        .filter(|line| line.trim_start().starts_with("fn "))
        .count()
}

#[test]
fn readme_and_docs_agree_with_the_tree() {
    let root = root();
    let expected = [
        ("scenarios", golden_case_count()),
        ("properties", property_count()),
    ];
    let mut errors = Vec::new();

    for path in markdown_files() {
        let rel = path
            .strip_prefix(&root)
            .unwrap_or(&path)
            .display()
            .to_string()
            .replace('\\', "/");
        let text = fs::read_to_string(&path).expect("read markdown");
        let dir = path.parent().unwrap_or(Path::new("."));

        let blocks = front_matter_blocks(&text);
        if blocks > 1 {
            errors.push(format!("{rel}: {blocks} front-matter blocks"));
        }

        let lines = prose(&text);
        let mut joined = String::new();
        for (number, line) in &lines {
            let plain = without_inline_code(line);
            joined.push_str(&plain);
            joined.push(' ');
            for target in link_targets(&plain) {
                if target.starts_with('#')
                    || target.contains("://")
                    || target.starts_with("mailto:")
                {
                    continue;
                }
                let file = target
                    .split('#')
                    .next()
                    .unwrap_or("")
                    .split('?')
                    .next()
                    .unwrap_or("");
                if !dir.join(file).is_file() {
                    errors.push(format!(
                        "{rel}:{number}: link {target:?} resolves to no file"
                    ));
                }
            }
            for (file, name) in citations(line) {
                let test_file = root.join("tests").join(&file);
                let has_test = fs::read_to_string(&test_file)
                    .map(|source| source.contains(&format!("fn {name}(")))
                    .unwrap_or(false);
                if !has_test {
                    errors.push(format!(
                        "{rel}:{number}: cites tests/{file}::{name}, which does not exist"
                    ));
                }
            }
        }

        for (noun, actual) in expected {
            for stated in stated_counts(&joined, noun) {
                if stated != actual {
                    errors.push(format!(
                        "{rel}: says {stated} {noun}, the tree has {actual}"
                    ));
                }
            }
        }
    }

    assert!(
        errors.is_empty(),
        "doc check found {} problem(s):\n{}",
        errors.len(),
        errors.join("\n")
    );
}
