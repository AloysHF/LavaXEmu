# Android Libretro Core

LavaXEmu ships a libretro core for Android devices. It is cross-compiled with
the Android NDK and packaged next to its `.info` and `android.json` profiles.

## Building for Android

```bash
export ANDROID_NDK_HOME="$HOME/Android/Sdk/ndk/<version>"
export RUSTFLAGS="-C link-arg=-Wl,-z,max-page-size=16384"
cargo ndk -t arm64-v8a -t armeabi-v7a -t x86 -t x86_64 --platform 21 \
    build -p lavaxemu-libretro --release
```

Each ABI produces `liblavaxemu.so`; rename it to
`lavaxemu_libretro_android.so` when installing into RetroArch on Android.
The CI workflows perform this packaging automatically.

## RetroDroid

The same core ships inside [RetroDroid](https://github.com/AloysHF/RetroDroid),
the all-in-one Android app for this emulator family. RetroDroid loads
`liblavaxemu.so` directly (no RetroArch required) and reads the neighbouring
`android.json` for the system profile: display names, content extensions,
virtual-controller preset, firmware requirements and orientation.

Rules the core follows for any Android host:

- The core declares `need_fullpath` and always opens the real content path;
  in-memory content is rejected with a message instead of being buffered.
- Saves and in-game files go to the directory answered by
  `RETRO_ENVIRONMENT_GET_SAVE_DIRECTORY` (the content path itself may be a
  disposable cache copy). Firmware and other assets belong to
  `RETRO_ENVIRONMENT_GET_SYSTEM_DIRECTORY`; LavaX needs no firmware because
  its font resource is embedded in the core.
- Load failures are reported with `RETRO_ENVIRONMENT_SET_MESSAGE` and
  `RETRO_ENVIRONMENT_GET_LOG_INTERFACE`, never as a silent `false`.
- Panics are caught at the C ABI boundary, so a bad frame cannot kill the host.
- Save states carry an `LVXS` magic header plus a little-endian `u32` format
  version; states from another version are rejected instead of being misread.
- Cores are linked with 16 KB page alignment for Android 15 devices.
