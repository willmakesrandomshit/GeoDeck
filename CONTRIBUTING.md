# Contributing to GeodeSync

Thanks for taking the time to report a problem or propose a change. GeodeSync manages game files and saves, so small, reviewable changes and clear test evidence matter.

## Before opening an issue

- Check existing issues and the current [user guide](USER_GUIDE.md).
- Include the GeodeSync version, operating system, relevant Steam/Geometry Dash setup, what you expected, and what happened.
- Remove account names, personal paths, webhook URLs, tokens, and save contents from logs or screenshots.
- For a security issue, follow [SECURITY.md](SECURITY.md) and do not publish exploit details in a public issue.

## Before opening a pull request

1. Open an issue first for larger changes so the scope can be discussed.
2. Keep changes focused and explain any effect on saves, game files, network access, or update behavior.
3. Add or update tests for behavior changes. Tests that create files should use a temporary directory and fixtures, not a user's Steam installation or saves.
4. Run the checks below and include the actual result in the pull request. If a platform is unavailable, say so.
5. Update documentation when commands, supported downloads, behavior, or limitations change.

## Development checks

Use Python 3.14 with Tk available. From the repository root, run:

```sh
python -m unittest discover -s tests -v
```

Platform packaging instructions are in [BUILDING.md](BUILDING.md). A local test run does not replace Windows, macOS, or Linux CI; do not claim a platform build or manual game test that you did not perform.

## Review expectations

- Preserve existing user data and fail safely when validation is uncertain.
- Do not add credentials, private logs, personal save files, or generated build artifacts.
- Explain the reason for the change, how it was checked, and any remaining platform limitation.
- A maintainer will review the change before it is merged or released.
