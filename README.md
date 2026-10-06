# Migu Kernel Manager (kmgr)

A KernelSU module kernel manager as an offline WebUI, for arm64 Android (Linux 4.19). 
Flash the zip, then open the WebUI from your manager's module card (reboot only if your 
manager asks you to apply the module). You never need to edit a file, write JSON, or put
anything into the module directory.

Works with KernelSU, KernelSU Next, APatch and WebUI X managers (anything that injects the `ksu`
WebUI object).

## Install

1. Download `kmgr.zip`.
2. Install it from your KernelSU/APatch manager (Modules → Install from storage).
3. Open the module's WebUI from the module card. No reboot needed.

Nothing is changed until you act. All values are read live from your kernel.

## How it works

- The whole backend is one static arm64 binary, `/data/adb/modules/kmgr/bin/kmgr`, that prints JSON.
- Everything is probed at runtime. Paths that do not exist on your kernel are simply not shown; cluster
  names come from the rank of the CPU policies, never from fixed numbers.
- **Set on boot**: the switch on each row means "set on boot". Changing a value applies it immediately;
  the switch makes it persist. Persisting is off by default. At boot, `service.sh` runs `kmgr apply` once
  to re-apply saved values, then exits.
- Every write is verified by reading back, and reports `{ok, requested, actual}`. If the kernel clamps a
  value, a toast shows the actual value.
- A boot-loop guard counts boots: if applying saved values failed twice in a row, they are skipped and a
  banner explains why (reset to recover).

## Pages

- **Dashboard** — per-core current frequencies, cluster max freqs and governor, GPU usage/freq/renderer/
  OpenGL, RAM and zRAM, battery, and system info (loadavg, entropy, uptime, deep sleep, WireGuard, kernel).
  A **Live monitor** screen shows scrolling charts. Monitoring stops when you leave the page.
- **CPU and GPU** — max/min frequency, governor and governor parameters per cluster; Core control,
  Cpusets, Stune and Uclamp; GPU frequency, governor, power levels and force flags.
- **Kernel settings** — I/O, Memory (including zRAM resize with the full `swapoff → reset → algorithm →
  disksize → mkswap → swapon` sequence), Scheduler, and Miscellaneous (TCP, thermal profile, pstore,
  entropy). "Other vm tunables" and "Kernel-specific scheduler nodes" are listed inline.
  - **Custom tunables** — add any writable node by absolute path (allowed roots: `/sys`, `/proc/sys`,
    `/dev/cpuset`, `/dev/stune`, `/dev/cpuctl`) and tune it like any other row.
- **Display control** — colour calibration sliders (if your kernel ships KCAL), resolution and pixel
  density via `wm` with a 15-second auto-revert confirmation, and reset.
- **Build.prop editor** — search `getprop`, then override a property. Overrides are written to the module's
  `system.prop` (systemless) and take effect after a reboot. Real partitions are never touched.

## Thermal profile

The thermal profile is applied **once** at boot and never re-asserted, and the node is never chmodded or
given a special SELinux context, so other tools can still change it. A separate opt-in **"Hold thermal
profile"** switch (off by default) keeps re-applying your saved value every 10 seconds; it pauses while the
screen is off. Turn it off to let other tools take over.

## `discover.log`

Some rows (scheduler `sched_*` nodes, the keyboard polling interval) are marked as unconfirmed for this
kernel. When the app runs, entries it could not resolve are written to
`/data/adb/kmgr/logs/discover.log`. If a row you expected is missing, check that file (via a file manager
with root, or `adb shell cat /data/adb/kmgr/logs/discover.log`) — a missing node means the kernel does not
expose it, and it is hidden on purpose rather than guessed.

## Known limitations

- Only nodes the kernel actually exposes are shown. `verify` entries that are not found stay hidden.
- Deep sleep is derived from the wall clock minus `/proc/uptime`; if the kernel exposes suspend statistics
  they are used instead. On a freshly booted device it reads near 0%.
- Colour calibration (KCAL) is hidden entirely if your kernel does not provide the nodes.
- I/O stats rely on `/proc/diskstats`; kernels that do not account block I/O (some custom builds) show the
  I/O tab flat at zero
- Some managers stage a module install into `modules_update` and apply it on the next reboot, so the WebUI
  button may not appear until then. That is the manager, not the module.
- Some managers cache the WebUI bundle. If an update does not appear, clear the manager's WebView cache
  (reinstall or force-stop the manager) and reopen the module.
- The module is built for arm64. Other devices will work only if their nodes match.

## Internal state

`/data/adb/kmgr/` holds `applied.conf`, `defaults.json`, `custom.json`, `profiles/`, `backups/` and `logs/`.
It is written by the app only; you never need to touch it.

## Building from source

Requires Rust with the `aarch64-linux-android` target, `cargo-ndk`, the Android NDK, and `bun`.

```
bash build.sh          # builds the binary, builds the WebUI, writes kmgr.zip
bash src/kmgr/tests/smoke.sh   # backend smoke test against a fake sysfs tree
```

The registry in `tunables.d/` is reference data for developers; it is compiled into the binary and is not
shipped in the zip.

<details>
  <summary>Quick push for development</summary>
  
  Make sure to enable ADB root on your root manager.
  Run this only if you already have the module installed.

  ```
  .\push.ps1
  ```
</details>

## License

Bundled Roboto font is Apache-2.0 (see `src/webui/src/fonts/Roboto-LICENSE.txt`). The sample image in the
WebUI is provided by the user.
