#!/usr/bin/env python3
"""Builds a DEMO environment so the real GeoDeck app has something to show.

Everything here is fixture data laid out exactly as the app expects it on Linux
(Steam folder, appmanifest, PE-stamped GeometryDash.exe, .geode packages, GD save
files). It uses the same formats as the repository's own integration tests
(tests/integration_tests.rs). All mods are clearly labelled "Demo", the player is
"DemoPlayer", and nothing here is a real Geometry Dash binary, mod, or save.

Usage: python3 scripts/make-demo-fixture.py <root> [user]   (home = <root>/home/<user>)
Then run the app with HOME=<home> XDG_CONFIG_HOME=<home>/.config
"""
import base64, gzip, json, struct, subprocess, sys, zipfile, pathlib

root = pathlib.Path(sys.argv[1]).resolve()
user = sys.argv[2] if len(sys.argv) > 2 else "user"
home = root / "home" / user
steam = home / ".local/share/Steam"
game = steam / "steamapps/common/Geometry Dash"
saves = steam / "steamapps/compatdata/322170/pfx/drive_c/users/steamuser/AppData/Local/GeometryDash"
for d in (game / "geode/mods", game / "Resources", saves, home / ".config"):
    d.mkdir(parents=True, exist_ok=True)

# Minimal PE header whose TimeDateStamp maps to GD 2.2081 in the app's table.
pe = bytearray(512)
pe[0:2] = b"MZ"
struct.pack_into("<I", pe, 0x3C, 0x80)
pe[0x80:0x84] = b"PE\0\0"
struct.pack_into("<I", pe, 0x88, 1768959392)
(game / "GeometryDash.exe").write_bytes(bytes(pe))
(game / "steam_appid.txt").write_text("322170")
(game / "Resources/README.txt").write_text("demo resources folder")
# A valid 1x1 PNG so the app's PNG-resource health check has something to verify.
(game / "Resources/GJ_GameSheet-demo.png").write_bytes(base64.b64decode(
    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg=="))

# Geode loader stand-in carrying a VS_FIXEDFILEINFO for 5.11.2 (as in the tests).
ver = bytearray(64)
ver[16:20] = bytes([0xBD, 0x04, 0xEF, 0xFE])
struct.pack_into("<I", ver, 24, (5 << 16) | 11)
struct.pack_into("<I", ver, 28, (2 << 16) | 0)
(game / "Geode.dll").write_bytes(bytes(ver))
(game / "XInput1_4.dll").write_bytes(b"demo proxy")

(steam / "steamapps/appmanifest_322170.acf").write_text(
    '"AppState"\n{\n\t"appid"\t\t"322170"\n\t"buildid"\t\t"20000000"\n'
    '\t"AutoUpdateBehavior"\t\t"1"\n\t"SizeOnDisk"\t\t"98765432"\n}\n')

def mod(fname, meta):
    with zipfile.ZipFile(game / "geode/mods" / fname, "w") as z:
        z.writestr("mod.json", json.dumps(meta))

base = {"geode": "5.0.0", "gd": {"win": "2.2081"}, "developer": "GeoDeck Demo"}
mod("demo.core-api.geode", {**base, "id": "demo.core-api", "name": "Demo Core API", "version": "1.2.0",
    "api": True, "description": "Demo package for the GeoDeck trailer. Not a real mod."})
mod("demo.practice-suite.geode", {**base, "id": "demo.practice-suite", "name": "Demo Practice Suite",
    "version": "2.0.1", "description": "Demo package for the GeoDeck trailer. Not a real mod.",
    "tags": ["gameplay"], "dependencies": {"demo.core-api": ">=1.0.0"}})
mod("demo.hud-toolkit.geode", {**base, "id": "demo.hud-toolkit", "name": "Demo HUD Toolkit",
    "version": "0.9.4", "description": "Demo package for the GeoDeck trailer. Not a real mod.",
    "tags": ["interface"], "dependencies": {"demo.core-api": ">=1.0.0"}})
mod("demo.level-browser.geode", {**base, "id": "demo.level-browser", "name": "Demo Level Browser",
    "version": "1.4.0", "description": "Demo package for the GeoDeck trailer. Not a real mod.",
    "tags": ["online"]})
mod("demo.replay-helper.geode.disabled", {**base, "id": "demo.replay-helper", "name": "Demo Replay Helper",
    "version": "0.3.2", "description": "Demo package for the GeoDeck trailer. Not a real mod.",
    "tags": ["utility"]})

# A demo crash log in Geode's real format, blaming a demo mod, so the crash-log
# parser and "Disable Faulty Mod" UI have something to show.
(game / "geode/crashlogs").mkdir(parents=True, exist_ok=True)
(game / "geode/crashlogs/2026-11-20_21-04-11.log").write_text("""2026-11-20T21:04:11.4210000+0000
Whoopsies! An unhandled exception has occurred.
== Geode Information ==
Loader Version: v5.11.2
== Exception Information ==
Faulty Mod: Demo Practice Suite v2.0.1 (demo.practice-suite)
Exception Code: c0000005 (EXCEPTION_ACCESS_VIOLATION)
Exception Details: Failed to read from memory at 0x0000000000000000
== Stack Trace (the most important part) ==
- demo.practice-suite.dll + 0x1a2b3c (demo::PracticeSuite::onUpdate + 0x4c | demo/practice.cpp:128)
- GeometryDash.exe + 0x1f4a20
== Register States ==
RAX: 0x0
== Installed Mods ==
x | [v5.11.2] geode.loader
x | [v2.0.1] demo.practice-suite
x | [v1.2.0] demo.core-api
""")

def enc(xml: str) -> bytes:
    gz = gzip.compress(xml.encode())
    return bytes(b ^ 11 for b in base64.urlsafe_b64encode(gz))

stats = {"1": 48210, "2": 9650, "5": 12, "6": 3120, "8": 18, "12": 64, "13": 240, "14": 912000, "28": 150}
gs = "".join(f"<k>{k}</k><s>{v}</s>" for k, v in stats.items())
# Demo custom songs: short generated tones named by song ID, described in MDLM_001
# exactly as GD writes it. Titles and artists are fictional and labelled Demo.
demo_songs = [
    ("9000001", "Demo Track: Neon Drive", "Demo Artist"),
    ("9000002", "Demo Track: Glass Circuit", "Demo Artist"),
    ("9000003", "Demo Track: Low Orbit", "Demo Artist B"),
    ("9000004", "Demo Track: Afterimage", "Demo Artist B"),
    ("9000005", "Demo Track: Static Bloom", "Demo Artist C"),
]
mdlm = ""
for sid, title, artist in demo_songs:
    mdlm += (f"<k>{sid}</k><d><k>kCEK</k><i>6</i><k>1</k><i>{sid}</i>"
             f"<k>2</k><s>{title}</s><k>4</k><s>{artist}</s></d>")
    subprocess.run(["ffmpeg", "-v", "error", "-y", "-f", "lavfi", "-i",
                    f"sine=frequency={300 + int(sid[-1]) * 60}:duration=3",
                    "-b:a", "128k", str(saves / f"{sid}.mp3")], check=True)
gm = ('<?xml version="1.0"?><plist version="1.0" gjver="2.0"><dict><k>GJA_001</k><s>DemoPlayer</s>'
      f'<k>GJA_003</k><i>1</i><k>GS_value</k><d>{gs}</d><k>MDLM_001</k><d>{mdlm}</d></dict></plist>')
levels = "".join(
    f'<k>k_{i}</k><d><k>kCEK</k><i>4</i><k>k2</k><s>{n}</s><k>k3</k><s>'
    + base64.b64encode(b"Demo level").decode() + "</s></d>" for i, n in enumerate(["Demo Level A", "Demo Level B", "Demo Level C"]))
ll = f'<?xml version="1.0"?><plist version="1.0"><dict><k>LLM_01</k><d>{levels}</d></dict></plist>'
(saves / "CCGameManager.dat").write_bytes(enc(gm))
(saves / "CCLocalLevels.dat").write_bytes(enc(ll))
print("fixture ready:", root)
