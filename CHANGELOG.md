# Changelog

This file records changes made after the latest published release. For published version history, see [release notes](release_notes.md) and [GitHub Releases](https://github.com/willmakesrandomshit/GeodeSync/releases).

## Unreleased

- Add consistent editor defaults and narrow credential ignore patterns.
- Remove duplicate copies of the GeodeSync brand image and obsolete one-off thumbnail renderers.
- Make artwork generation and Geode logo extraction use portable command-line paths instead of machine-specific folders.
- Align game and mod services with their domain models; bound mod metadata reads and prevent toggles from overwriting files.
- Create save snapshots in a staging directory, sanitize snapshot labels, and avoid partial or relative-path backups.
- Use strict shared version parsing for update checks, omit ambient GitHub credentials, and bound process detection waits.
- Correct README and macOS installation guidance to match the currently published Windows and Linux packages.
- Replace absolute and unsupported feature claims with concise descriptions of the app's available tools.
- Add contributor instructions and a maintenance roadmap.
- Require a version tag for automated release publication; pushing a `release/v*` branch no longer publishes a release.
- Add issue forms and a pull request template.
