GeoDeck Alpha-1.0.0 for Linux & Steam Deck (SteamOS)
======================================================

1. Extract this archive anywhere (e.g. ~/Applications/GeoDeck).
2. Run:
     ./run-geodeck.sh
   Or mark `geodedeck` executable (`chmod +x geodedeck`) and launch directly.

Notes for Linux & SteamOS:
- GeoDeck automatically detects Steam libraries across ~/.local/share/Steam,
  ~/.steam/steam, and Flatpak Steam, plus Proton compatdata save folders
  (`.../compatdata/322170/pfx/drive_c/users/steamuser/AppData/Local/GeometryDash`).
- Geometry Dash launches natively through Steam Proton (`steam://rungameid/322170`).
- Local save snapshots use AES-256-GCM encryption with key permissions `0600`.
