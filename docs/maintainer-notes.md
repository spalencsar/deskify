# Maintainer Notes

This document captures lightweight repo-management conventions for `deskify` during the alpha/MVP phase.

## Issue Labels (Recommended)

Use labels to make project direction visible and to reduce triage friction.

- `architecture/runtime` - runtime model, shared runtime, config-based apps, install model evolution
- `distro-compat` - distro-specific build/runtime issues (WebKitGTK, packaging prerequisites, environment differences)
- `integration` - desktop integration topics (`.desktop`, icons, launchers, window class behavior)
- `icons` - favicon fetching, icon parsing, icon quality/fallback behavior
- `good first issue` - scoped tasks suitable for first-time contributors
- `needs testing` - behavior change needs manual or wider validation across systems
- `design decision` - issue/PR affects long-term architecture or product direction
- `docs` - README, CONTRIBUTING, examples, troubleshooting, release notes
- `bug` - incorrect behavior or regression
- `enhancement` - feature or improvement request
- `ci` - GitHub Actions, release workflows, build checks

Label definitions for GitHub setup are stored in `docs/github-labels.json`.
If you use GitHub CLI, `scripts/setup-github-labels.sh <owner/repo>` will create/update them.

## Triage Heuristics (Alpha)

- Prefer labeling directionally first (`architecture/runtime`, `distro-compat`, `integration`) before priority.
- Use `design decision` early when a discussion can create long-term constraints.
- Add `needs testing` to changes that touch build/install/remove flows or desktop integration behavior.
- Distinguish user-environment issues from code bugs (`distro-compat` + details) to avoid misclassifying support reports.

## Release Notes Categories (Suggested Mapping)

- `feature`, `enhancement` -> user-facing functionality additions
- `fix`, `bug` -> behavior corrections and regressions
- `docs` -> documentation changes
- `chore`, `ci` -> maintenance and pipeline work

## Maintainer Framing (README / Issues)

During alpha, consistently frame Deskify as:

- a Linux-first web app integration tool
- an early MVP with intentional per-app build architecture
- a project with a visible evolution path toward runtime/config-based installs

This helps avoid "Nativefier clone" framing as the default community label.
