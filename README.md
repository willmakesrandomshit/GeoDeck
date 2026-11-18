<p align="center">
  <img src="assets/banner.png" alt="GeodeSync" width="100%">
</p>

<p align="center">
  <b>A Geometry Dash and Geode companion for Steam.</b><br>
  Switch supported game versions, protect saves, manage mods, and troubleshoot from one desktop app.
</p>

<p align="center">
  <a href="https://github.com/willmakesrandomshit/GeodeSync/releases/latest"><img src="https://img.shields.io/github/v/release/willmakesrandomshit/GeodeSync?display_name=tag&style=for-the-badge" alt="Latest release"></a>
  <img src="https://img.shields.io/badge/downloads-Windows%20%7C%20Linux-7b6b9e?style=for-the-badge" alt="Current downloads for Windows and Linux">
</p>

## Download and get started

Download the latest published build from [GitHub Releases](https://github.com/willmakesrandomshit/GeodeSync/releases/latest). Choose the package for your system, then follow the [complete user guide](USER_GUIDE.md).

- **Windows:** Extract the entire ZIP to a normal folder and open `GeodeSync.exe` or `Run-GeodeSync.bat`. Keep the `_internal` folder beside the executable; do not move the executable by itself.
- **Linux / Steam Deck:** Use the `GeodeSync-v<version>-Linux.tar.gz` asset. Extract the complete archive to a writable folder and run `./run.sh`. The app manages Steam's Windows Geometry Dash files through Proton; use Steam Deck Desktop Mode. See [Linux setup notes](README-Linux.txt).
- **macOS:** No macOS download is currently published. The build workflow withholds macOS packages until Developer ID signing and notarization are available. Do not use an older, unsigned preview as a substitute. See [macOS installation notes](README-macOS.txt) for status and supported build targets.

### App updates

Packaged builds check the official GitHub Releases API in the background. When a newer stable version is available, GeodeSync downloads the matching platform package, checks its size and SHA-256 digest against GitHub's release metadata, and validates archive paths before staging it beside the current app. It then applies the verified update and restarts automatically; while a Steam game-file download or switch is active, it waits for that operation to finish. Updates never run with administrator privileges. Install the app in a folder your account can write to (for example, `~/Applications` on macOS), or update it manually from the official Releases page if the operating system protects the install folder.

The digest check detects transfer corruption and mismatched release assets. GitHub release metadata and the package are served by the same GitHub account, so this is not an independent publisher signature. Windows Authenticode and Apple signing are used only when release signing credentials are configured; GeodeSync does not bypass operating-system warnings.

GeodeSync manages Geometry Dash installations provided by Steam; it does not replace Steam or Geode. Before switching versions, close the game and review the confirmation. The switch flow backs up detected saves, validates staged game files, and keeps recovery data if a swap is interrupted. If validation fails, the existing installation should be left in place.

## What it can do

- **Dashboard and launch options:** View available player statistics and choose among the launch presets exposed by the installed app.
- **Geode mod tools:** Browse SDK mod listings, view installed mods, and use the app's mod management actions.
- **Demon list and progress:** Browse Pointercrate listings and compare available local save progress.
- **Audio and visual customization:** Manage supported practice music, death effects, and texture options with app-provided restore actions.
- **Save tools:** Create backups, manage save slots, and export or restore GeodeSync bundles. Always keep a separate backup before modifying saves.
- **Diagnostics and shortcuts:** Inspect common setup problems, then review any suggested repair before applying it. Keyboard shortcuts are listed in the user guide.
- **Version management:** Manage supported Steam Geometry Dash versions and related update settings. Close the game and review each confirmation before changing installation files.
- **Discord Rich Presence and tray mode:** Optionally show app and game state through Discord's local client connection.

## Build and verify

GitHub Actions runs the regression suite on Windows, macOS, and Linux and builds Windows and Linux release packages. macOS builds are checked in CI for Apple silicon and Intel, but macOS downloads are withheld unless signing and notarization credentials are configured. Build targets do not establish that every OS version, processor, or Steam installation has been tested.

Run the regression suite with:

```sh
python -m unittest discover -s tests -v
```

See [macOS installation notes](README-macOS.txt), [Linux notes](README-Linux.txt), [release notes](release_notes.md), and [all releases](https://github.com/willmakesrandomshit/GeodeSync/releases).

Security and trust details: [Security policy](SECURITY.md), [Privacy notes](PRIVACY.md), and [Build instructions](BUILDING.md).

## Contributing

Bug reports and pull requests are welcome. Read [CONTRIBUTING.md](CONTRIBUTING.md) before opening a change. Original GeodeSync code and maintainer-created artwork are available under the [MIT licence](LICENSE). The Geode loader logo is excluded; see [third-party notices](THIRD_PARTY_NOTICES.md).

## Project status

See [ROADMAP.md](ROADMAP.md) for the current maintenance priorities. GeodeSync is an independent community project and is not affiliated with Valve or the Geode SDK team.
