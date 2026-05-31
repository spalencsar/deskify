# Security Policy

Deskify is currently in an alpha / early MVP phase. Security reports are still welcome and should be handled carefully, especially because Deskify creates local desktop entries, downloads icons, manages browser profiles, and can install launchers into user-level XDG locations.

## Supported Versions

During the alpha phase, security fixes target the latest code on `master` and the most recent published pre-release when practical.

Older alpha releases are not guaranteed to receive backported fixes. Users should update to the latest available release after a security fix is published.

## Reporting a Vulnerability

Please do not open a public issue for a vulnerability before it has been reviewed.

Preferred reporting path:

1. Use GitHub's private vulnerability reporting / security advisory feature for this repository, if available.
2. If private reporting is not available, contact the maintainer through GitHub and request a private disclosure channel.

Include as much detail as possible:

- Affected Deskify version or commit SHA
- Operating system and desktop environment
- Exact command used, if applicable
- Steps to reproduce
- Expected and actual behavior
- Impact assessment
- Any proof-of-concept files or URLs, if safe to share privately

## What Counts as a Security Issue

Security-sensitive areas include, but are not limited to:

- Unsafe `.desktop` entry generation or command injection
- Unsafe handling of app IDs, paths, icons, URLs, or browser binary paths
- Path traversal or accidental deletion outside Deskify-managed locations
- Unsafe profile handling for Chromium-backed apps
- Unexpected execution of downloaded or user-controlled content
- Vulnerabilities in generated Tauri wrapper configuration

General bugs, compatibility issues, build failures, and feature requests should be reported through normal GitHub issues unless they have a security impact.

## Response Expectations

The project is maintained on a best-effort basis during alpha. The maintainer will try to acknowledge valid reports promptly, assess impact, and coordinate a fix or mitigation before public disclosure.

If a report is accepted, the fix will normally be documented in `CHANGELOG.md` and released through the regular GitHub release process.
