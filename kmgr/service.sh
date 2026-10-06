#!/system/bin/sh
# Runs once after boot. Applies saved settings, then exits. No re-assertion.
MODDIR=${0%/*}
STATE=/data/adb/kmgr
BIN="$MODDIR/bin/kmgr"
[ -x "$BIN" ] || BIN="/data/adb/modules/kmgr/bin/kmgr"

until [ "$(getprop sys.boot_completed)" = "1" ]; do sleep 1; done
sleep 5

"$BIN" apply
rc=$?

if [ "$rc" -eq 0 ]; then
  echo 0 > "$STATE/boot_count"
  rm -f "$STATE/boot_failed"
else
  echo 1 > "$STATE/boot_failed"
fi

# Opt-in thermal hold: only if the user enabled it (flag file written by the app).
if [ "$rc" -eq 0 ] && [ -f "$STATE/hold_thermal" ]; then
  "$BIN" hold-thermal --interval 10000 &
fi
