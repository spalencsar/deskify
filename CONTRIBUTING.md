# Contributing to Deskify

First off, thank you for considering contributing to Deskify! It's people like you that make Deskify such a great tool.

`deskify` is currently in an **Alpha / Early MVP** phase. Contributions that improve reliability, error handling, tests, and Linux compatibility are especially valuable.

## How Can I Contribute?

### Reporting Bugs
This section guides you through submitting a bug report for Deskify. Following these guidelines helps maintainers and the community understand your report, reproduce the behavior, and find related reports.

* Use a clear and descriptive title for the issue to identify the problem.
* Describe the exact steps which reproduce the problem in as many details as possible.
* Provide specific examples to demonstrate the steps.

### Suggesting Enhancements
This section guides you through submitting an enhancement suggestion for Deskify, including completely new features and minor improvements to existing functionality.

* Use a clear and descriptive title for the issue to identify the suggestion.
* Provide a step-by-step description of the suggested enhancement in as many details as possible.
* Explain why this enhancement would be useful to most Deskify users.

### Pull Requests
* Fill in the required template
* Do not include issue numbers in the PR title
* Include screenshots only when UI/desktop integration behavior is relevant.
* Follow the Rust styleguide (`cargo fmt`).
* Document new code based on the existing documentation style.
* End all files with a newline.
* Prefer small, focused PRs (one behavioral change per PR).

## Development Setup
1. Deskify relies on `tauri`. Ensure `tauri-cli` is installed via `cargo install tauri-cli`.
2. Ensure you have the standard Tauri dependencies for Linux (WebKit2GTK, etc.).
3. Clone the repository and run `cargo build` to test your changes.
4. Run `cargo clippy` to check for common mistakes and improve your Rust code.

## Local Checks Before Opening a PR

Run these checks locally before submitting a pull request:

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo check
```

## Maintainer Notes (Repo Meta / Triage)

Maintainers can use `docs/maintainer-notes.md` for the current label strategy and issue triage conventions during the alpha phase.

## Manual Smoke Test (Recommended for Behavior Changes)

If your change affects build/install/list/remove behavior, run a quick smoke test:

```bash
deskify build --url "https://example.com" --name "Example"
deskify list
deskify remove example
```

Also verify invalid IDs are rejected safely:

```bash
deskify remove ../foo
deskify remove FooBar
```
