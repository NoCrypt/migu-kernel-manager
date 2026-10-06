
adb root
adb shell "mkdir -p /data/adb/modules/kmgr/bin /data/adb/modules/kmgr/webroot"
adb push kmgr/bin/kmgr /data/adb/modules/kmgr/bin/kmgr
adb shell "chmod 755 /data/adb/modules/kmgr/bin/kmgr"
adb push kmgr/module.prop /data/adb/modules/kmgr/module.prop
adb shell "rm -rf /data/adb/modules/kmgr/webroot"
adb push src/webui/dist /data/adb/modules/kmgr/webroot

adb shell "am force-stop com.kowx712.supermanager" 
adb shell "am force-stop me.weishu.kernelsu" 