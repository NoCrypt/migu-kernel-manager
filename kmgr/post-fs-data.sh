#!/system/bin/sh
# Boot-loop guard ONLY. Never write sysfs/procfs from here.
MODDIR=${0%/*}
STATE=/data/adb/kmgr
mkdir -p "$STATE/logs" 2>/dev/null

c=0
[ -f "$STATE/boot_count" ] && c=$(cat "$STATE/boot_count" 2>/dev/null)
case "$c" in ''|*[!0-9]*) c=0 ;; esac
echo $((c + 1)) > "$STATE/boot_count"
