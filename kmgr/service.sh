#!/system/bin/sh
# Runs once after boot. Applies saved settings, then exits. No re-assertion.
MODDIR=${0%/*}
STATE=/data/adb/kmgr
BIN="$MODDIR/bin/kmgr"
[ -x "$BIN" ] || BIN="/data/adb/modules/kmgr/bin/kmgr"

until [ "$(getprop sys.boot_completed)" = "1" ]; do sleep 1; done
sleep 5

# Seconds the device must stay up after a successful apply before the boot
# counter is cleared. A setting that crashes the device shortly after boot must
# NOT clear it, or the boot-loop guard never triggers.
SETTLE=60

"$BIN" apply
rc=$?

# Opt-in thermal hold: start as soon as apply succeeded (before the settle wait).
if [ "$rc" -eq 0 ] && [ -f "$STATE/hold_thermal" ]; then
  "$BIN" hold-thermal --interval 10000 &
fi

if [ "$rc" -eq 0 ]; then
  # Still alive after the settle window => the applied settings are safe.
  sleep "$SETTLE"
  echo 0 > "$STATE/boot_count"
  rm -f "$STATE/boot_failed"
else
  echo 1 > "$STATE/boot_failed"
fi
