# kmgr CLI / JSON protocol

A reusable specification of the `kmgr` backend so the same contract can be implemented in another
project. It is self-contained: a caller only needs to run the binary and parse JSON. The registry
schema that `tree` resolves is included as Appendix A.

- **Binary:** `/data/adb/modules/kmgr/bin/kmgr` (static aarch64-linux-android, no runtime deps).
- **Transport:** one process per command. Every command prints **one JSON object to stdout**, or a
  line per tick for the streaming commands. Errors are JSON on stdout too
  (`{"ok":false,"error":"...","code":<exit-code>}`); **stderr is reserved for debug traces and is
  normally empty**. Callers should parse one shape regardless of success or failure.
- **Exit codes:** `0` success, `1` operational error, `2` usage error, `3` `apply` skipped by the
  boot-loop guard (see below).
- **Schema handshake:** `tree` and `info` carry `"schema": <N>` and `kmgr version` reports it
  explicitly. A UI ships an expected value and warns when the binary's differs (a module update
  replaced the binary without the UI being refreshed, or vice versa).

## 1. Conventions

- All writes are validated: the path is resolved with `realpath` and must live under one of the allowed
  roots `/sys`, `/proc/sys`, `/dev/cpuset`, `/dev/stune`, `/dev/cpuctl`; path traversal is rejected.
  The **resolved** path is then opened with `O_NOFOLLOW`, so a symlink swapped in between the check and
  the write is rejected rather than followed (there is no TOCTOU window between validating one string and
  writing another).
- `/sys` is a very wide root, so the allowlist is not a safety boundary. Entries that can destabilise the
  device carry a `risk` of `medium` or `high`; the UI confirms `high` before applying.
- A write is followed by a read-back and reports the **actual** value, so callers can detect clamping:
  `{"ok": bool, "requested": string, "actual": string[, "error": string]}`.
- When the process runs as root, nodes are treated as writable even if their mode is `0444` (many
  `/proc/sys` nodes), matching what root can actually write.
- Values are **strings** in both directions (raw kernel text), including numeric tunables; the numeric
  registry hints (`min`, `max`, `step`, `scale`) are integers. `persist` records the current value string
  verbatim.
- Identifiers are **keys** of the form `<scope>.<id>` (e.g. `cpu.cluster.policy0.max_freq`). Persisted
  state is keyed by that string, never by the raw path, so it survives kernel updates. Submenu/custom
  children are keyed by their **absolute path** instead.
- **First-seen defaults** (`defaults.json`) are recorded the first time a key is observed *before* it can
  change: on the first successful read (`tree`/`get`/`get-many`) **or** immediately before the first write
  that could change it (`set`/`set-path`/`set-many`/`apply`), whichever happens first. `reset` can only
  restore the true pre-change value if this invariant holds, so a write never records the value it is
  about to set as the default.

## 2. Commands

### `kmgr tree [--values]`

Resolves the whole registry and prints it. This is the discovery call a UI makes first. The root
carries `"ok":true` and `"schema":N`.

```json
{
  "ok": true,
  "schema": 2,
  "groups": [
    {
      "id": "cpu",
      "title": "CPU",
      "sections": [
        {
          "id": "cluster",
          "scope": "cpu.cluster.policy0",
          "title": "Little cluster",
          "entries": [ /* Entry */ ]
        }
      ]
    }
  ]
}
```

`tree --values` skips labels/choices/paths and prints only a key→value map, which is far cheaper for a
periodic refresh:

```json
{"ok":true,"schema":2,"values":{"cpu.cluster.policy0.governor":"schedutil","cpu.cluster.policy0.max_freq":"1804800"}}
```

**Entry**

| field | type | meaning |
|---|---|---|
| `id` | string | id within the section |
| `scope` | string | persistence scope |
| `key` | string | `scope.id`, the persistence/command key |
| `label` | string | UI label |
| `type` | string | see §3 |
| `path` | string\|null | resolved node path (null for container/custom entries) |
| `value` | string\|null | effective current value (for `enum_bracket`, the selected token) |
| `display` | string\|null | human value (e.g. `1804 MHz`, `Enabled`) |
| `choices` | `[{value,label}]` | selectable options (enums, freq lists) |
| `labels` | `{value:label}` | value→text map (booleans) |
| `min,max,step` | int\|null | numeric hints |
| `unit,display_unit` | string\|null | raw unit and display unit |
| `scale` | int\|null | display divisor (absent = 1) |
| `write_order` | string\|null | paired-freq write ordering hint |
| `handler` | string\|null | native handler (`zram`, `wm_size`, `wm_density`, `custom_tunables`) |
| `bundle` | string\|null | entries applied together (e.g. `zram`, `kcal`) |
| `help` | string\|null | one-line hint |
| `risk` | `low`\|`medium`\|`high`\|null | `high` → confirm before applying |
| `is_dir` | bool | container/submenu |
| `persisted` | bool | has a set-on-boot record |
| `children` | `[Child]` | for container entries |

**Child** (submenu item, for `dir` / `glob_files` / `auto_glob`): `{label, path, value, type, writable, persisted}`.
Children are written with `set-path` and persisted with `persist-path`.

Entries whose node is missing on the kernel are silently omitted, as is a group/section with no entries.

### `kmgr get <key>`

```json
{"id":"governor","key":"cpu.cluster.policy0.governor","label":"CPU governor",
 "type":"enum","path":"/sys/.../scaling_governor","value":"schedutil","persisted":false}
```

### `kmgr get-many <key>...`

Resolves many keys in one process and prints only `key→value` (or `null` when a key does not resolve on
this kernel). Absolute paths are accepted for custom/submenu children. This is the cheap refresh path
when the UI already has the tree shape.

```json
{"ok":true,"schema":2,"values":{"cpu.cluster.policy0.governor":"schedutil","cpu.cluster.policy0.max_freq":"1804800"}}
```

### `kmgr set <key> <value> [--persist]`

Applies live; `--persist` also records it as set-on-boot. Handler entries (e.g. `zram`) run their native
sequence instead of a plain write.

```json
{"ok":true,"requested":"performance","actual":"performance","key":"cpu.cluster.policy0.governor","persisted":true}
```

### `kmgr set-many <key> <value> ... [--persist]`

Applies several plain nodes as **one atomic unit** (e.g. a KCAL RGB bundle). Each pair is written and
read back in order; if any write fails or is clamped, the already-written nodes are restored to their
previous values and the call returns `{"ok":false,"rolled_back":true,"failed":{...},"applied":[...]}`.
On success it returns `{"ok":true,"applied":[...]}`. Handler entries are not accepted here — use `set`
for those.

### `kmgr persist <key>` / `kmgr unpersist <key>`

Record / drop the current value as set-on-boot without changing it.

```json
{"ok":true,"key":"cpu.cluster.policy0.max_freq","value":"1804800"}
```

### `kmgr set-path <path> <value>` / `kmgr persist-path <path>` / `kmgr unpersist-path <path>`

Same as `set`/`persist` but keyed by an absolute path, for submenu and custom children.

### `kmgr custom list|add|remove <path>`

App-side custom tunables, stored in `/data/adb/kmgr/custom.json` (never in module files). `add` validates
the allowed roots and existence.

```json
{"ok":true,"tunables":[{"path":"/proc/sys/vm/vfs_cache_pressure","type":"int","value":"100","persisted":false}]}
```

### `kmgr apply`

Re-applies every persisted entry once. Intended for boot.

- **Order:** entries are replayed in **registry order** (groups → sections → entries as authored), never
  in `applied.conf` file order. Keys that no longer resolve (and absolute paths) are replayed last in
  stable key order. This guarantees paired min/max frequency nodes are written in the same order the UI
  writes them, so a boot cannot fail by writing `min` above the current `max` (or vice versa).
- Boot-loop guard: if `boot_count >= 2`, prints `{"ok":false,"skipped":true,"reason":"..."}` and exits `3`.
- Otherwise prints `{"ok":bool,"applied":[{"key":... ,"ok":... ,"requested":... ,"actual":... [,"error":...]}]}`.
- `apply` does **not** clear `boot_count`. The boot script clears it only after the device has stayed up
  past a settle window (`sys.boot_completed` + a delay), so settings that crash the device shortly after
  a "successful" apply still trip the guard on the next boot.

### `kmgr reset <scope|all>`

Restores the first-seen defaults (from `defaults.json`, recorded as described in §1) for the scope and
clears its persisted entries.

```json
{"ok":true,"scope":"display.kcal","restored":[{"key":"display.kcal.red","ok":true,"requested":"256","actual":"256"}]}
```

### `kmgr version`

Handshake for a UI that shipped an expected schema. One-shot, no side effects.

```json
{"ok":true,"name":"kmgr","version":"0.1.0","schema":2}
```

### `kmgr monitor --interval <ms> [--seconds N]`

Streams one JSON object per line (NDJSON) to stdout, one per tick. Source files are opened once and
re-read each tick (no per-tick fork). `--seconds N` makes it exit on its own (leak guard); the stream also
ends cleanly when the reader closes the pipe. `monitor` is the dashboard composite and overlaps the
`live` tabs (e.g. its CPU/GPU/RAM fields mirror `live cpu`/`live gpu`/`live ram`); use `live` when only
one tab is on screen.

```json
{
  "t": 1791171328917,
  "load": 1.7,
  "cores": [{"cpu":0,"cur_khz":1171200,"min_khz":1171200,"max_khz":1804800,"load":0.0}],
  "clusters": [{"title":"Little cluster","governor":"schedutil","cur_khz":1171200,"max_khz":1804800}],
  "cpu_temp": 37.6,
  "gpu": {"busy":25.0,"cur_hz":150000000,"max_hz":670000000},
  "mem": {"total":5628.2,"free":121.1,"available":1405.5,"cached":1448.4},
  "zram": {"total":3072.0,"used":2152.3,"compr":736.7,"pct":70.1},
  "battery": {"capacity":54,"status":"Charging","temp":34.0,"current":-230468,"voltage":3908681,
              "charge_counter":2459422,"charge_full":4996000,"charge_full_design":5000000,"cycles":3},
  "loadavg": [1.42,1.40,1.55],
  "loadavg_full": "1.42 1.40 1.55 1/6449 27993",
  "entropy": {"avail":3192,"poolsize":4096},
  "uptime": 3040.09,
  "suspend_count": 16,
  "deep_sleep": 0.0,
  "deep_sleep_ms": 827.7
}
```

Units are carried in the field names, so no scaling guesswork is needed: CPU frequencies are
`*_khz`, GPU/DDR frequencies are `*_hz` (mirroring the live tabs' `khz`/`cur_hz`), memory is MiB,
temperature °C, battery current µA, voltage µV, charge µAh, `deep_sleep_ms` ms and `uptime` s. Any field
may be `null` when its source is unavailable — never a fake `0`. Deep sleep prefers the kernel's
`/sys/power/suspend_stats/total_time`, falling back to `CLOCK_BOOTTIME − CLOCK_MONOTONIC`.

### `kmgr live list`

Lists the Live Monitor tabs whose data source exists on this kernel, in display order.

```json
{"ok":true,"tabs":[{"id":"processes","label":"PROCESSES"},{"id":"cpu","label":"CPU"},{"id":"ram","label":"RAM"}]}
```

### `kmgr live <tab> [--interval <ms>] [--seconds N] [--max N] [--cluster <rank>]`

One NDJSON line per tick, like `monitor`. `--seconds N` self-terminates (leak guard). Tabs and their lines:

| tab | extra flags | example line |
|---|---|---|
| `processes` | `--max N` (default 25) | `{"t":1,"procs":[{"pid":9320,"name":"system_server","cpu":3.6,"rss_mb":184}]}` |
| `cpu` | | `{"t":1,"cores":[{"i":0,"on":true,"load":9,"khz":1804800,"cluster":"Little"}]}` |
| `cpustats` | `--cluster <rank>` | `{"t":1,"rows":[{"khz":300000,"ms":173000}],"total_ms":293000}` |
| `gpu` | | `{"t":1,"usage":6,"cur_hz":305000000,"min_hz":180000000,"max_hz":670000000}` |
| `ram` | | `{"t":1,"total_kb":5831000,"free_kb":3000000,"used_kb":2831000,"usage":49}` |
| `zram` | | `{"t":1,"total_b":2147483648,"used_b":1073741824,"usage":50}` |
| `ddr` | | `{"t":1,"usage":0,"cur_hz":150000000,"min_hz":150000000,"max_hz":670000000}` |
| `io` | | `{"t":1,"usage":0,"read_bps":0,"write_bps":0}` |
| `wakelocks` | | `{"t":1,"pm":[{"name":"CaffeineTile","ms":1795707}],"sources":[{"name":"smp2p-sleepstate","ms":23000,"active":12,"wakeup":1,"events":40,"expire":0,"max_ms":8000}]}` |
| `thermal` | | `{"t":1,"zones":[{"i":0,"type":"aoss0-usr","c":39.5}]}` |

Rules: emit the first tick immediately; deltas (`load`, `cpu`, `io`) are `null` until the second tick.
Unknown values are `null`, never `0`. If a source disappears mid-run the process prints `{"gone":true}` and
stops. Cluster rank order follows the sorted cpufreq policies (lowest = Little). `processes` scans
`/proc/*/stat` and reads RSS only for the top N; CPU% is the delta of `utime+stime` over the delta of total
`/proc/stat` jiffies.

`wakelocks` has two groups: `pm` = Android PowerManager wake locks parsed from `dumpsys power` (app-held
locks like `CaffeineTile`; refreshed every 5 s, held locks keep counting between refreshes) and `sources` =
kernel wakeup sources from `/sys/class/wakeup/wakeup*/{name,total_time_ms}` (fallback
`/sys/kernel/debug/wakeup_sources`), sorted descending, capped at 100.

### `kmgr kill <pid>`

One-shot: sends `SIGKILL` to the given pid (used by the Processes tab). The pid must be an integer `> 1`;
kmgr refuses pid 1 and its own pid. No shell is involved - the integer goes straight to `kill(2)`.

```json
{"ok":true,"pid":9320,"name":"com.example.app","signal":9}
```

On failure: `{"ok":false,"pid":9320,"name":"...","error":"Operation not permitted (os error 1)"}`.

### `kmgr dmesg [--follow] [--since <seq>] [--interval <ms>] [--seconds N]` / `kmgr dmesg --export`

Reads `/dev/kmsg` (non-blocking). Without `--follow` it dumps the buffered lines and exits. With `--follow`
it seeds the last 1000 records (dropping those `<= --since`) then emits one JSON object per new line:

```json
{"n":7552,"t":"[  316.209400] KernelSU: ksu fd installed: 4 for pid 18155"}
```

`--export` writes the current buffer to `/sdcard/Download/kernel_log_<YYYYMMDD-HHMMSS>.txt` and prints
`{"ok":true,"path":"..."}`. Clearing the view is done by the UI (baseline `n`), never by clearing the kernel
buffer.

### `kmgr info`

One-shot static info (GPU strings from `dumpsys SurfaceFlinger`).

```json
{"ok":true,"kernel":"Linux version ...",
 "gpu":{"vendor":"Qualcomm","renderer":"Adreno (TM) 650","gl_version":"OpenGL ES 3.2 V@..."},
 "android":"13","sdk":"33","model":"Example","device":"example",
 "wireguard":"1.0.0","selinux":null,"slot":"_a"}
```

### `kmgr wm [status|reset|size <WxH>|size reset|density <N>|density reset]`

Display override via `wm`. All return the status object:

```json
{"ok":true,
 "size":{"current":"1080x2400","physical":"1080x2400","override":null},
 "density":{"current":440,"physical":440,"override":null}}
```

### `kmgr props [list [query]|set <key> <value>|unset <key>|reset]`

Build.prop editor. Overrides are written to the module's `system.prop` (systemless; real partitions are
never touched) and take effect after a reboot.

```json
{"ok":true,"props":[{"key":"ro.product.model","value":"Example","overridden":false}],"total":1}
```
`set`/`unset`/`reset` return `{"ok":true,"reboot_required":true}`.

### `kmgr thermal-hold [on|off]` and `kmgr hold-thermal --interval <ms>`

`thermal-hold` reads/writes the hold flag; with no argument it reports `{"ok":true,"on":bool}`.
`hold-thermal` is a long-lived process: each tick it reads the thermal node and writes only if it differs,
pauses while the screen is off, and **exits when the flag file disappears**. It never changes permissions.

### `kmgr discover`

Prints the unresolved/verify entries log: `{"ok":true,"log":"/data/adb/kmgr/logs/discover.log","content":"..."}`.

### `kmgr settings [list|set <key> <value>|unset <key>]`

App-side UI settings (accent colour, refresh intervals, list sizes, …) stored in
`/data/adb/kmgr/settings.json`. Keys are `[A-Za-z0-9_]+`, values are strings.

```json
{"ok":true,"settings":{"accent":"#1de9b6","interval":"1000","showHelp":"1"}}
```

## 3. Entry types

`int`, `bool`, `enum`, `enum_bracket`, `enum_int`, `freq`, `string`, `rgb`, `dir`, `glob_files`,
`auto_glob`, `custom`.

- `enum_bracket`: kernel lists options with the current one in brackets (`[cfq] mq-deadline none`);
  `value`/`display` are the selected token.
- `dir`: a directory whose writable files become `children`.
- `glob_files`: for every dir matching `glob`, expose the listed `files`.
- `auto_glob`: every writable file matching `roots`/`name_glob` that is not already declared.
- `custom`: no node of its own; driven by a `handler` (`wm_size`, `wm_density`, `custom_tunables`).
- Collections (`dir`, `glob_files`, `auto_glob`) that resolve to no children are hidden.

## 4. Registry schema

The registry that `tree` resolves is authored as JSON in `tunables.d/` and compiled into the binary. The
full field reference is reproduced in Appendix A (`tunables.d/_schema.md` is the authoring copy).

## 5. State files (`/data/adb/kmgr/`)

| file | format |
|---|---|
| `applied.conf` | `key=value` lines; `key` is `scope.id` or an absolute path |
| `defaults.json` | `{"key":"first-seen-value"}`; recorded as described in §1 |
| `custom.json` | `{"tunables":["/abs/path", ...]}` |
| `settings.json` | `{"key":"value"}`; app-side UI settings (`kmgr settings`) |
| `hold_thermal` | presence = thermal hold enabled |
| `boot_count` | integer, incremented by `post-fs-data.sh`; cleared by `service.sh` only after a settle window |
| `boot_failed` | presence = saved settings were skipped by the boot-loop guard |
| `logs/discover.log` | unresolved `verify` entries, one per line |

Every JSON/conf state file is rewritten whole, so it is **written to a temp file and renamed into place**.
A concurrent reader (`set --persist` while `apply` runs) therefore never observes a half-written file.

These are written by the app only; end users never edit them.

## 6. WebUI bridge (KernelSU)

The manager injects a global `ksu` (`WebViewInterface.addJavascriptInterface(..., "ksu")`). The official
`kernelsu` npm library wraps it:

- `exec(cmd) -> Promise<{errno, stdout, stderr}>` runs a **shell string**. Never interpolate a
  user-supplied value (tunable value, custom path, property value, …) into it — a quote or `;` is
  shell injection.
- `spawn(cmd, args, options) -> ChildProcess` runs an **argv array with no shell**. Use it for anything
  user-supplied, and for streaming commands. Listen with `child.stdout.on('data', ...)`,
  `child.on('exit', ...)`.
- **There is no kill.** Terminate a long-lived child by passing a unique tag as an argv entry and running
  `pkill -f '[t]ag'`. The bracket makes the pkill command line contain `[t]ag` while the regex matches the
  literal `tag`, so pkill cannot match (and kill) its own shell. A pidfile is an equivalent alternative.

The shipped WebUI wraps this as `kmgr(args)` (string or argv array), `execArgs(cmd, args)`, and
`spawn(cmd, args, onLine, onExit)` in `src/webui/src/lib/ksu.js`; argv-array calls are used for every
user-supplied value.

A browser mock (`src/webui/src/lib/mock.js`) returns canned JSON for `tree`/`info`/`set` and emits fake
`monitor` samples when `window.ksu` is absent, so the UI runs off-device.

## Appendix A — registry schema

Each file in `tunables.d/` is one UI tab/group; files load in filename order. Nothing is a hardcoded
device assumption: every path is probed at runtime, and entries whose node is missing (or not writable)
are hidden.

```
File:    { "group", "title", "sections":[ Section ] }
Section: { "id", "title", "foreach"?: Foreach, "entries":[ Entry ] }

Foreach (repeat a section per match):
  { "var":"policy", "glob":"/sys/devices/system/cpu/cpufreq/policy*",
    "order":"numeric", "titles":["Little cluster","Big cluster","Prime cluster"],
    "title_fallback":"Cluster {i}" }
  Placeholders usable in strings: {dir} (matched path), {name} (basename), {i} (rank, 0-based),
  {cur:<entry id suffix>} (current value of a sibling entry, e.g. {cur:governor}).
  Titles are assigned by RANK of the sorted matches, never by a fixed policy number.

Entry fields:
  id          unique within the section scope; persisted as "<scope>.<id>=<value>" in applied.conf
  label       UI label
  path        single node path
  paths       candidate list, first existing wins
  locate      {"names":[...],"roots":[...],"max_depth":N} search fallback; log misses to discover.log
  type        int | bool | enum | enum_bracket | enum_int | freq | string | dir | glob_files | auto_glob | rgb | custom
  labels      value->text map (e.g. {"0":"Disabled","1":"Enabled"})
  zero_label  text shown when value is 0
  choices_from node containing a space-separated list of options
  choices     static options
  min,max,step   integers (NULL when unset)
  unit,display_unit strings (NULL when unset)
  scale       integer display divisor (NULL/absent = 1); divides the raw value for display
  handler     named native handler in kmgr for multi-step writes
  bundle      entries sharing a bundle are applied together by the handler (one persisted record)
  write_order "max_first_when_raising" | "min_first_when_lowering" (paired nodes, interactive sets)
  verify      true = path/semantics not confirmed; keep behind discovery + read-back check
  dir         type dir: expandable submenu listing every writable regular file under path
  glob_files  type glob_files: for each dir matching "glob", expose the listed "files"
  auto_glob   type auto_glob: expose every writable file matching roots/name_glob that is not
              already declared; value type inferred (int if numeric, else string)
  risk        "low" | "medium" | "high"; "high" nodes are confirmed by the UI before applying
  help        one-line hint
  optional    true = hide silently if no candidate exists (no discover.log noise)
  apply       "once" (default) = write at boot only; never re-assert unless hold is enabled
  never_touch_mode_or_context   true = never chmod/chcon this node, never write it from post-fs-data

Additions
  enum_int    int-valued enum: "values":[{"value":N,"label":"..."}], "unknown_label":"Unknown ({v})"
  Files 05-sources.json and 80-features.json use "sources"/"sections" shapes: sources are read-only
  candidate lists for the monitor; features are ordinary entries.
```
