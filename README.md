# GeoDeck (Alpha-1.0.0)

All-in-one **Geometry Dash Launcher, Version Slot Manager, Geode Mod Studio, NONG Song Library, and Encrypted Backup Vault** written in **Rust**.

## Key Capabilities

- **Geometry Dash Launcher & Version Slot Manager**:
  - Automatic Steam library & `appmanifest_322170.acf` discovery.
  - Windows PE `TimeDateStamp` Geometry Dash version detection (`1.900` – `2.2091`) and PE `VS_FIXEDFILEINFO` Geode Loader detection.
  - Core binary & `Resources/` integrity verification + PE signature validation.
  - Isolated Version Slots (clone active installs or import Steam `download_depot` builds side-by-side without clobbering primary Steam files).
- **Automatic Maximum Performance Tuning**:
  - Automatically applies Maximum Performance optimizations on startup and install switch (Win32 `HIGH_PRIORITY_CLASS`, performance core affinity, DirectX High-Perf GPU binding, fullscreen optimizations override, and Click Between Frames precision/mouse tuning) backed by `optimization_rollback.json`.
- **Geode Mod Manager & `.gdpack` Studio**:
  - Parses `.geode` ZIP archives & `mod.json` (v3–v6 schemas) + decodes `logo.png` icons.
  - Toggles mods natively via Geode's `geode.loader/saved.json` (`should-load-*`) and legacy `.geode.disabled`.
  - Live Geode Index v1 API browser, GD-version-aware updates, SHA-256 verification, dependency/conflict auditor, Mod Profiles, and `.gdpack.zip` portable export/import.
- **NONG Song Library & Offline Metadata Cache**:
  - Resolves human-readable song titles, artists, and associated Geometry Dash levels from `CCGameManager.dat` (`MDLM_001` / `GLM_03`), `CCLocalLevels.dat`, Jukebox manifests, embedded ID3/Vorbis tags, and official GD servers (`musiclibrary_02.dat`, `sfxlibrary.dat`, `getGJSongInfo.php`).
  - Persistent offline cache (`song_metadata_cache.json`), search/sort/format/status filtering, manual song renaming, and NONG track replacement with automatic original track backup & restoration.
- **Encrypted Save Vault & Diagnostics**:
  - Windows DPAPI master key (`vault.key.dpapi`) + AES-256-GCM encryption + SHA-256 & XML integrity verification (`CCGameManager.dat` / `CCLocalLevels.dat`) with strict `GJP`/`GJP2` privacy filtering.
  - Automatic pre-launch, post-exit, and periodic backup scheduler + legacy GeodeSync/GDLauncher backup migration.
  - Geode crash log parser with stack trace viewer, 1-click Disable Faulty Mod, and redacted summary export.
- **Automatic GitHub Updates**:
  - Checks `https://github.com/willmakesrandomshit/GeoDeck` automatically on every launch with SHA-256 verified downloads.

## Build & Run

```powershell
cargo test
cargo run -- --self-check
cargo build --release
```
