#!/usr/bin/env bash
# Builds the kmgr native binary (aarch64), the WebUI, and packs kmgr.zip.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")" && pwd)"

echo "==> building kmgr (aarch64-linux-android)"
cd "$ROOT/src/kmgr"
cargo ndk -t arm64-v8a -P 26 build --release
BIN="target/aarch64-linux-android/release/kmgr"
mkdir -p "$ROOT/kmgr/bin"
cp "$BIN" "$ROOT/kmgr/bin/kmgr"

if [ -d "$ROOT/src/webui" ] && [ -f "$ROOT/src/webui/package.json" ]; then
  echo "==> building WebUI"
  cd "$ROOT/src/webui"
  bun install --frozen-lockfile 2>/dev/null || bun install
  bun run build
  rm -rf "$ROOT/kmgr/webroot"
  mkdir -p "$ROOT/kmgr/webroot"
  cp -r dist/* "$ROOT/kmgr/webroot/"
fi

echo "==> packing kmgr.zip"
cd "$ROOT"
rm -f ./out/kmgr.zip
if command -v zip >/dev/null 2>&1; then
  (cd kmgr && zip -r -X ../out/kmgr.zip . -x '.*' >/dev/null)
elif command -v 7z >/dev/null 2>&1; then
  (cd kmgr && 7z a -tzip ../out/kmgr.zip . >/dev/null)
else
  # bsdtar (Windows 10+/macOS) writes zip based on the extension
  (cd kmgr && tar -a -c -f ../out/kmgr.zip .)
fi

echo "==> done: $ROOT/out/kmgr.zip"
