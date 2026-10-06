#!/usr/bin/env bash
# Smoke test for the kmgr backend against a fake sysfs/procfs tree.
# Usage: bash src/kmgr/tests/smoke.sh
set -uo pipefail
ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"

if [ -z "${KMGR_BIN:-}" ]; then
  for cand in "$ROOT/src/kmgr/target/debug/kmgr" "$ROOT/src/kmgr/target/debug/kmgr.exe"; do
    [ -x "$cand" ] && KMGR_BIN="$cand" && break
  done
fi
[ -x "${KMGR_BIN:-}" ] || { echo "build first: cargo build --manifest-path src/kmgr/Cargo.toml"; exit 1; }

FAKE_POSIX="$(mktemp -d)"
FAKE="$FAKE_POSIX"
if command -v cygpath >/dev/null 2>&1; then FAKE="$(cygpath -m "$FAKE_POSIX")"; fi
trap 'rm -rf "$FAKE_POSIX"' EXIT
export KMGR_ROOT="$FAKE"

w() { mkdir -p "$(dirname "$FAKE/$1")"; printf '%s' "$2" > "$FAKE/$1"; }

# --- CPU: three clusters, ranks assigned by sorted policy number
for p in 0 4 7; do
  base="/sys/devices/system/cpu/cpufreq/policy$p"
  w "$base/scaling_available_frequencies" "300000 576000 1017600 1804800 2419200 3187200"
  w "$base/scaling_available_governors" "performance powersave schedutil"
  w "$base/scaling_governor" "schedutil"
  w "$base/scaling_min_freq" "300000"
  w "$base/scaling_max_freq" "1804800"
  w "$base/scaling_cur_freq" "1804800"
done
w "/sys/devices/system/cpu/cpufreq/policy0/schedutil/up_rate_limit_us" "1000"

# --- GPU
g="/sys/class/kgsl/kgsl-3d0"
w "$g/devfreq/available_frequencies" "180000000 305000000 490000000 670000000"
w "$g/devfreq/available_governors" "msm-adreno-tz simple_ondemand"
w "$g/devfreq/governor" "msm-adreno-tz"
w "$g/devfreq/min_freq" "180000000"
w "$g/devfreq/max_freq" "670000000"
w "$g/max_pwrlevel" "0"
w "$g/min_pwrlevel" "5"
w "$g/idle_timer" "80"
w "$g/throttling" "1"

# --- misc / scheduler / memory
w "/sys/class/thermal/thermal_message/sconfig" "10"
w "/proc/sys/net/ipv4/tcp_congestion_control" "cubic"
w "/proc/sys/net/ipv4/tcp_available_congestion_control" "reno cubic bbr"
w "/proc/sys/kernel/sched_energy_aware" "1"
w "/proc/sys/vm/swappiness" "100"
w "/proc/sys/vm/min_free_kbytes" "8192"
w "/proc/sys/vm/admin_reserve_kbytes" "0"
w "/proc/sys/vm/nr_overcommit_hugepages" "0"
w "/proc/sys/vm/drop_caches" "0"

# --- monitor sources
w "/proc/stat" "cpu  100 0 100 800 0 0 0 0 0 0
cpu0 50 0 50 400 0 0 0 0 0 0
cpu1 50 0 50 400 0 0 0 0 0 0
cpu2 0 0 0 100 0 0 0 0 0 0
cpu3 0 0 0 100 0 0 0 0 0 0
cpu4 0 0 0 50 0 0 0 0 0 0
cpu5 0 0 0 50 0 0 0 0 0 0
cpu6 0 0 0 25 0 0 0 0 0 0
cpu7 0 0 0 25 0 0 0 0 0 0"
w "/proc/meminfo" "MemTotal:        5831000 kB
MemFree:         1234000 kB
MemAvailable:    3000000 kB
Cached:          1000000 kB"
w "/proc/loadavg" "0.50 0.40 0.30 1/500 12345"
w "/proc/uptime" "123456.78 1000.00"
w "/sys/class/power_supply/battery/capacity" "76"
w "/sys/class/power_supply/battery/status" "Discharging"
w "/sys/class/power_supply/battery/temp" "312"
w "/sys/class/power_supply/battery/current_now" "-450000"
w "/sys/class/power_supply/battery/voltage_now" "3900000"
w "/sys/class/power_supply/battery/charge_full" "4200000"
w "/sys/class/power_supply/battery/charge_full_design" "4500000"
w "/sys/class/power_supply/battery/cycle_count" "210"
w "/sys/class/kgsl/kgsl-3d0/gpu_busy_percentage" "12 %"
w "/sys/class/kgsl/kgsl-3d0/devfreq/cur_freq" "305000000"
w "/sys/class/kgsl/kgsl-3d0/gpu_model" "Adreno (TM) 650"
w "/sys/class/thermal/thermal_zone0/temp" "38500"
w "/sys/class/thermal/thermal_zone0/type" "aoss0-usr"
w "/proc/sys/kernel/random/entropy_avail" "256"
w "/proc/sys/kernel/random/poolsize" "4096"
w "/sys/block/zram0/disksize" "2147483648"
w "/sys/block/zram0/mm_stat" "1073741824 536870912 600000000 0 0 0 0 0 0"
w "/sys/power/suspend_stats/total_time" "600000"
w "/sys/power/suspend_stats/success_count" "12"

# --- live monitor sources
w "/sys/devices/system/cpu/present" "0-7"
w "/sys/devices/system/cpu/cpufreq/policy0/related_cpus" "0 1 2 3"
w "/sys/devices/system/cpu/cpufreq/policy4/related_cpus" "4 5"
w "/sys/devices/system/cpu/cpufreq/policy7/related_cpus" "6 7"
w "/sys/devices/system/cpu/cpufreq/policy0/stats/time_in_state" "300000 100
1804800 20"
w "/proc/diskstats" "   8       0 sda 1000 0 20000 40 500 0 10000 20 0 60 0"
w "/sys/class/devfreq/ddrqos/cur_freq" "150000000"
w "/sys/class/devfreq/ddrqos/min_freq" "150000000"
w "/sys/class/devfreq/ddrqos/max_freq" "670000000"
w "/sys/class/wakeup/wakeup0/name" "smp2p-sleepstate"
w "/sys/class/wakeup/wakeup0/total_time_ms" "23000"

pass=0; fail=0
ok()    { if [ "$2" = "$3" ]; then pass=$((pass+1)); else fail=$((fail+1)); echo "FAIL: $1 (got '$2' want '$3')"; fi; }
has()   { if echo "$2" | grep -q "$3"; then pass=$((pass+1)); else fail=$((fail+1)); echo "FAIL: $1 (missing '$3')"; fi; }
hasnt() { if echo "$2" | grep -q "$3"; then fail=$((fail+1)); echo "FAIL: $1 (unexpected '$3')"; else pass=$((pass+1)); fi; }

TREE="$("$KMGR_BIN" tree)"

# 1. clusters ranked by policy number, not fixed number
has "little title" "$TREE" '"title":"Little cluster"'
has "big title"    "$TREE" '"title":"Big cluster"'
has "prime title"  "$TREE" '"title":"Prime cluster"'
has "policy0 scope" "$TREE" '"scope":"cpu.cluster.policy0"'
has "freq display" "$TREE" '"display":"1804 MHz"'
has "governor choices" "$TREE" '"label":"schedutil"'
has "governor dir child" "$TREE" '"up_rate_limit_us"'
has "gpu max display" "$TREE" '"display":"670 MHz"'
has "thermal label" "$TREE" '"Dynamic"'
has "vm zero label" "$TREE" '"display":"Disabled"'
has "auto_glob other vm" "$TREE" 'Other vm tunables'
has "auto_glob child" "$TREE" '"nr_overcommit_hugepages"'
hasnt "drop_caches hidden" "$TREE" '"drop_caches"'

# 2. get
G="$("$KMGR_BIN" get cpu.cluster.policy0.governor)"
has "get value" "$G" '"value":"schedutil"'

# 3. set + read-back
S="$("$KMGR_BIN" set cpu.cluster.policy0.governor performance 2>/dev/null || true)"
has "set ok" "$S" '"ok":true'
has "set actual" "$S" '"actual":"performance"'

# 4. set --persist then apply reapplies
"$KMGR_BIN" set cpu.cluster.policy0.max_freq 1017600 --persist >/dev/null 2>&1
w "/sys/devices/system/cpu/cpufreq/policy0/scaling_max_freq" "300000"
A="$("$KMGR_BIN" apply)"
has "apply ok" "$A" '"ok":true'
ok "apply wrote" "$(cat "$FAKE/sys/devices/system/cpu/cpufreq/policy0/scaling_max_freq")" "1017600"

# 5. boot-loop guard
w "/data/adb/kmgr/boot_count" "2"
BG="$("$KMGR_BIN" apply)"; rc=$?
has "guard skipped" "$BG" '"skipped":true'
ok "guard exit code" "$rc" "3"

# 6. monitor stream (NDJSON, one line per tick)
MON="$("$KMGR_BIN" monitor --interval 200 2>/dev/null | head -2)"
has "monitor cores" "$MON" '"cores"'
has "monitor cluster title" "$MON" '"Little cluster"'
has "monitor gpu busy" "$MON" '"busy":12'
has "monitor gpu model not in sample" "$MON" '"cur_hz":305000000'
has "monitor battery" "$MON" '"capacity":76'
has "monitor mem" "$MON" '"total":5694'
has "monitor loadavg" "$MON" '"loadavg":\[0.5,0.4,0.3\]'
has "monitor cpu_temp" "$MON" '"cpu_temp":38.5'
has "monitor cluster cur" "$MON" '"cur_khz":1804800'
has "monitor zram total" "$MON" '"total":2048'
has "monitor zram used" "$MON" '"used":1024'
has "monitor entropy avail" "$MON" '"avail":256'
has "monitor entropy pool" "$MON" '"poolsize":4096'
has "monitor deep sleep ms" "$MON" '"deep_sleep_ms":600000'
ok "monitor two lines" "$(printf '%s\n' "$MON" | grep -c '"t":')" "2"

# 7. live monitor tabs
LL="$("$KMGR_BIN" live list)"
for id in processes cpu cpustats gpu ram zram ddr io wakelocks thermal; do
  has "live list $id" "$LL" "\"id\":\"$id\""
done
hasnt "live list no kernellog" "$LL" '"id":"kernellog"'

LRAM="$("$KMGR_BIN" live ram --interval 200 2>/dev/null | head -1)"
has "live ram total" "$LRAM" '"total_kb":5831000'
has "live ram free" "$LRAM" '"free_kb":3000000'
has "live ram usage" "$LRAM" '"usage":49'

LZRAM="$("$KMGR_BIN" live zram --interval 200 2>/dev/null | head -1)"
has "live zram total" "$LZRAM" '"total_b":2147483648'
has "live zram used" "$LZRAM" '"used_b":1073741824'

LGPU="$("$KMGR_BIN" live gpu --interval 200 2>/dev/null | head -1)"
has "live gpu usage" "$LGPU" '"usage":12'
has "live gpu min" "$LGPU" '"min_hz":180000000'

LCPU="$("$KMGR_BIN" live cpu --interval 200 2>/dev/null | head -2)"
has "live cpu core" "$LCPU" '"i":0'
has "live cpu little" "$LCPU" '"cluster":"Little"'
has "live cpu prime" "$LCPU" '"cluster":"Prime"'
ok "live cpu two lines" "$(printf '%s\n' "$LCPU" | grep -c '"t":')" "2"

LSTATS="$("$KMGR_BIN" live cpustats --cluster 0 --interval 200 2>/dev/null | head -1)"
has "live cpustats row" "$LSTATS" '"khz":300000'
has "live cpustats total" "$LSTATS" '"total_ms":1200'

LIO="$("$KMGR_BIN" live io --interval 200 2>/dev/null | head -2 | tail -1)"
has "live io usage" "$LIO" '"usage"'
has "live io read" "$LIO" '"read_bps"'

LDDRE="$("$KMGR_BIN" live ddr --interval 200 2>/dev/null | head -1)"
has "live ddr cur" "$LDDRE" '"cur_hz":150000000'

LWAKE="$("$KMGR_BIN" live wakelocks --interval 200 2>/dev/null | head -1)"
has "live wake name" "$LWAKE" '"name":"smp2p-sleepstate"'
has "live wake ms" "$LWAKE" '"ms":23000'

LTH="$("$KMGR_BIN" live thermal --interval 200 2>/dev/null | head -1)"
has "live thermal type" "$LTH" '"type":"aoss0-usr"'
has "live thermal c" "$LTH" '"c":38.5'

LPROC="$("$KMGR_BIN" live processes --interval 200 2>/dev/null | head -1)"
has "live processes key" "$LPROC" '"procs"'

# 8. app settings file
SET="$("$KMGR_BIN" settings set accent '#2196f3')"
has "settings set" "$SET" '"ok":true'
SG="$("$KMGR_BIN" settings)"
has "settings get" "$SG" '"accent":"#2196f3"'
"$KMGR_BIN" settings unset accent >/dev/null 2>&1
hasnt "settings unset" "$("$KMGR_BIN" settings)" '"accent"'

# 9. schema / version handshake
V="$("$KMGR_BIN" version)"
has "version ok"     "$V" '"ok":true'
has "version name"   "$V" '"name":"kmgr"'
has "version schema" "$V" '"schema":2'
has "tree schema"    "$TREE" '"schema":2'

# 10. get-many (cheap refresh)
GM="$("$KMGR_BIN" get-many cpu.cluster.policy0.governor cpu.cluster.policy0.max_freq)"
has "get-many governor" "$GM" '"cpu.cluster.policy0.governor":"performance"'
has "get-many max"      "$GM" '"cpu.cluster.policy0.max_freq":"1017600"'

# 11. tree --values (values only, no labels/choices/paths)
TV="$("$KMGR_BIN" tree --values)"
has "tree values map"      "$TV" '"values"'
has "tree values governor" "$TV" '"cpu.cluster.policy0.governor":"performance"'
hasnt "tree values no groups" "$TV" '"groups"'

# 12. set-many is atomic and rolls back on the first failure
SM="$("$KMGR_BIN" set-many cpu.cluster.policy0.min_freq 576000 cpu.cluster.policy0.max_freq 1017600) || true"
has "set-many ok" "$SM" '"ok":true'
has "set-many rollback" "$("$KMGR_BIN" set-many cpu.cluster.policy0.min_freq 710400 /sys/devices/system/cpu/cpufreq/policy0/nope 1)" '"rolled_back":true'
ok "set-many restored min" "$(cat "$FAKE/sys/devices/system/cpu/cpufreq/policy0/scaling_min_freq")" "576000"

# 13. usage errors are JSON on stdout (exit 2), never bare stderr text
EU="$("$KMGR_BIN" get 2>/dev/null)"; rc=$?
has "usage error is json" "$EU" '"ok":false'
ok "usage error exit code" "$rc" "2"

echo "---"
echo "pass=$pass fail=$fail"
[ "$fail" -eq 0 ]
