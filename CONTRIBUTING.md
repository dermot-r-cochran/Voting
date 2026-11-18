# Contributing to Voting

Contributions are welcome: bug reports, fixes, tests, documentation and focused improvements.

## Before you start

- For anything beyond a small fix, open an issue first describing the problem and the change you have in mind, so the approach can be agreed before you spend time on it.
- `tests/golden/` is behaviour as specification: it holds the outputs of the engine this crate was ported from. A change that alters an allocation fails there. An intentional behaviour change regenerates `expected.txt` as a reviewed decision in the same pull request; individual expected lines are never patched.
- The quota, `floor(n_voters / n_slots) + 1`, is deliberate and settled, not a mistaken Droop quota.
- The conservation checks are `assert!`, kept on in release builds. A change that weakens them will not be accepted.

## Making a change

- Keep a pull request to one change, and say in its description what it changes and why.
- Run what CI runs before you push:

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

- A proptest counterexample is added to `tests/allocation.rs` as a named regression case before the fix.

## How a pull request is reviewed

The repository has one maintainer, Dermot Cochran, so a pull request may wait a while before anyone looks at it.

- CI must be green. A failing check is the first thing to fix.
- The review may be assisted by an AI tool, which reads the change and reports to the maintainer. It does not approve or merge anything.
- **Every outside pull request is merged by the maintainer, never by an automated process or an AI tool,** however small.
- A pull request may be declined, or asked to change shape, when it moves the project somewhere it is not meant to go. That is not a judgement on the work.

## Licence

Contributions are accepted under this repository's licence, GPL-3.0-or-later (`LICENSE`), the same terms everyone else receives the code under. No sign-off line and no contributor licence agreement are required. Only contribute work you have the right to license on these terms.

## Security

Please don't describe a security problem in a public issue. Open an issue saying only that you have one, and a private way to send the details will be arranged.
