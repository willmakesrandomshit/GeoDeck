#!/usr/bin/env bash
# Captures the application screenshots used in the trailer, with NO pixel edits.
# Runs the REAL GeoDeck release build under Xvfb against a demo environment
# (scripts/make-demo-fixture.py, home = /home/deck as on a Steam Deck).
#
# Needs root (writes /home/deck and /etc/hosts) and: xvfb, xdotool, ffmpeg.
# To stop the sandbox's offline "index unreachable" error toast appearing in
# the footage, the Geode/Pointercrate/GitHub hosts resolve to a local listener
# that accepts connections and never answers, so those requests hang instead of
# failing instantly. /etc/hosts is restored at the end.
#
# Usage: scripts/capture-shots.sh
set -euo pipefail
cd "$(dirname "$0")/.."
BIN="${GEODECK_BIN:-$PWD/../target/release/geodedeck}"
H=/home/deck
cp /etc/hosts /tmp/hosts.capture.bak
printf '127.0.0.1 api.geode-sdk.org\n127.0.0.1 pointercrate.com\n127.0.0.1 api.github.com\n' >> /etc/hosts
cat > /tmp/blackhole.py <<'PY'
import socket
s = socket.socket(); s.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
s.bind(("127.0.0.1", 443)); s.listen(50)
keep = []
while True:
    c, _ = s.accept(); keep.append(c)
PY
python3 /tmp/blackhole.py & BH=$!
trap 'cp /tmp/hosts.capture.bak /etc/hosts; kill $BH 2>/dev/null || true' EXIT
sleep 1

rm -rf "$H"; python3 scripts/make-demo-fixture.py / deck >/dev/null
export HOME=$H XDG_CONFIG_HOME=$H/.config XDG_DATA_HOME=$H/.local/share

# Three real encrypted snapshots, made by clicking the app's own "Create Backup Now".
cat > /tmp/mksnap.sh <<SNAP
#!/usr/bin/env bash
"$BIN" --tab backups --size 1600x1000 >/tmp/app_run.log 2>&1 &
APP=\$!
sleep 6; xdotool mousemove 303 237 click 1; sleep 4
kill \$APP 2>/dev/null; wait \$APP 2>/dev/null || true
SNAP
chmod +x /tmp/mksnap.sh
for i in 1 2 3; do timeout 60 xvfb-run -a -s "-screen 0 1920x1200x24" /tmp/mksnap.sh; sleep 1; done

mkdir -p public/shots
for t in home installs mods demonlist backups health textures songs settings; do
  timeout 90 xvfb-run -a -s "-screen 0 1920x1200x24" "$BIN" --tab "$t" --size 1600x1000 \
    --capture "$PWD/public/shots/$t.png" >/tmp/cap_$t.log 2>&1 || echo "capture $t failed"
done

# Sub-tabs reached by clicking them in the real UI (window is 1600x1000 at 0,0).
cat > /tmp/subshot.sh <<SUB
#!/usr/bin/env bash
# args: tab click_x click_y out
"$BIN" --tab "\$1" --size 1600x1000 >/tmp/app_sub.log 2>&1 &
APP=\$!
sleep 6; xdotool mousemove "\$2" "\$3" click 1; sleep 2.5
import -display "\$DISPLAY" -window root -crop 1600x1000+0+0 +repage "\$4"
kill \$APP 2>/dev/null; wait \$APP 2>/dev/null || true
SUB
chmod +x /tmp/subshot.sh
sub() { timeout 60 xvfb-run -a -s "-screen 0 1920x1200x24" /tmp/subshot.sh "$1" "$2" "$3" "$PWD/public/shots/$4.png" || echo "sub $4 failed"; }
sub tools 417 187 crash
sub tools 794 187 levels
sub mods 709 187 profiles
sub mods 556 187 audit
sub backups 452 187 slots
sub backups 630 187 compactor
ls -la public/shots/*.png
