# GeodeSync → GeoDeck

GeodeSync was created by Wil (@willmakesrandomshit). Elq (@mynameisjapan26-hue) is the incoming development lead, pending collaborator access. Wil retains repository ownership; this document does not transfer ownership or grant permissions.

## Direction

With native Geode compatibility reducing the need for version switching, GeoDeck is intended to become a comprehensive Geometry Dash launcher and management companion. Preserve useful existing functionality, existing releases, Git history and the MIT licence.

## Getting started

Clone the repository, read BUILDING.md, CONTRIBUTING.md, SECURITY.md and USER_GUIDE.md, and run `python -m unittest discover -s tests -v`. Existing Python/Tk packaging targets Windows and Linux/Steam Deck; macOS release publication requires signing and notarization. Verify current CI and real installations before asserting support.

## Roadmap

1. Agree branding, maintainer responsibilities and migration plan; keep existing download and update endpoints working.
2. Audit installation detection, launch presets, diagnostics and recovery; preserve save backups and rollback.
3. Improve Geode mod discovery, installation and compatibility using supported interfaces.
4. Refine the desktop interface, save management and customization tools. Evaluate architecture before rewriting.
5. Build and test a public alpha, document known limitations and review release assets and signatures before publication.

## Collaboration

Use feature branches and reviewed pull requests. Run regression tests and platform build checks before release; recommend protecting main against force pushes and requiring passing checks. Do not expose secrets or change updater endpoints without a tested migration. Elq leads feature development once access is granted; Wil retains administrative responsibility until a separate ownership decision. Preserve original copyright and third-party notices and credit new contributions.

## Open decisions

Confirm collaborator access, final brand assets, repository rename timing, compatibility migration and release authority. Current runtime behavior and platform builds have not been independently tested as part of this documentation handover.
