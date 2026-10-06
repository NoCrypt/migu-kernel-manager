#!/system/bin/sh
# Install checks only. No system writes happen here.
SKIPUNZIP=0

ui_print "- Migu Kernel Manager (kmgr)"
ui_print "- Target: arm64 Android (Linux 4.19)"

if [ -f "$MODPATH/bin/kmgr" ]; then
  set_perm "$MODPATH/bin/kmgr" 0 0 0755
fi
# KernelSU sets the webroot permissions and SELinux context itself on install.

# KernelSU stages an install (files in modules_update + an 'update' marker) and
# only applies it on reboot. The manager's WebUI button appears once 'webroot'
# exists in the ACTIVE module dir, so mirror the runtime pieces there to make
# the WebUI usable right away. The staged update replaces them (same files) at
# the next reboot. On managers that install directly, MODPATH == ACTIVE and
# this is skipped.
ACTIVE=/data/adb/modules/kmgr
if [ -n "$MODPATH" ] && [ "$MODPATH" != "$ACTIVE" ]; then
  mkdir -p "$ACTIVE" 2>/dev/null
  cp -af "$MODPATH/module.prop" "$ACTIVE/module.prop" 2>/dev/null
  rm -rf "$ACTIVE/bin" "$ACTIVE/webroot" 2>/dev/null
  cp -af "$MODPATH/bin" "$ACTIVE/" 2>/dev/null
  cp -af "$MODPATH/webroot" "$ACTIVE/" 2>/dev/null
  [ -x "$ACTIVE/bin/kmgr" ] || chmod 755 "$ACTIVE/bin/kmgr" 2>/dev/null
fi

ui_print "- Open the WebUI from your manager's module card (no reboot needed)"
ui_print "- Done."
