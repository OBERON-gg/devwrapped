# Contributing

Thanks for taking the time to help. Bug reports, fixes, and parsers for new history formats are all welcome.

## Before you open an issue

devwrapped reads private data, so please keep it out of issues and pull requests.

- Do not paste shell history, agent logs, commit messages, file names, or paths.
- The terminal summary from devwrapped is safe to paste. It holds only counts and known names.
- If a file fails to parse, describe its structure instead. Replace every value with `x`, the way [docs/agent-formats.md](docs/agent-formats.md) does.
- If devwrapped shows or writes something private, do not open an issue. Report it privately instead, as [PRIVACY.md](PRIVACY.md#reporting-a-privacy-bug) explains.

## Building

devwrapped needs a recent stable Rust toolchain. CI runs these 5 commands on Ubuntu and Windows, and every pull request has to pass them:

```
cargo build --workspace
cargo test --workspace
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo deny check
```

`cargo deny` comes from `cargo install cargo-deny`.

To try a change without your own data:

```
cargo run -p devwrapped -- --sample --format all --out-dir out
```

The `out` folder is ignored by git.

## Layout

| Crate | Content |
| --- | --- |
| `crates/devwrapped-core` | Collectors, the stats model, and the awards. No output formatting. |
| `crates/devwrapped-card` | The SVG, PNG, and HTML renderers and the themes |
| `crates/devwrapped-cli` | Argument parsing and the terminal output. Its package name is `devwrapped`. |

The cli crate depends on the other 2. The card crate depends on the core crate.

## Privacy rules

These rules matter more than any feature:

- No network access. `deny.toml` bans the network crates, and those bans stay.
- A collector keeps only the fields that PRIVACY.md lists and drops everything else while it parses a record.
- Bad input never causes a panic. Skip the record and count it.
- Raw records never go to the terminal, error messages, logs, or test output.
- Tests and fixtures use only made-up data. Never copy real history into the repository.

A change to what a collector reads or keeps also needs a change to PRIVACY.md, and a test with fake secrets for the new fields.

## Code style

Match the style of the code around your change. Two rules are enforced by the compiler and clippy: no `unsafe` code, and no `unwrap` or `expect` outside tests.

Please open an issue before you add a dependency, so we can talk about it first.

Slide changes need updated `insta` snapshots. Run `cargo insta review`, from `cargo install cargo-insta`, and look at each change before you accept it.

## Commits and pull requests

Commit messages follow [Conventional Commits](https://www.conventionalcommits.org/), with a lowercase subject, such as `feat(core): parse nushell history`. Keep each pull request to one change.

By contributing, you agree that your work is licensed under both the MIT and Apache-2.0 licenses, like the rest of the project.
