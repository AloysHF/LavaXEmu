# RetroArch Core

This guide covers installing and using the LavaXEmu libretro core with
RetroArch.

## Supported Platforms

The core builds as a standard libretro dynamic library and runs on any
platform RetroArch supports:

| Platform | Architecture | Status |
|----------|-------------|--------|
| Windows | x86_64 | ✅ |
| macOS | x86_64, aarch64 | ✅ |
| Linux | x86_64, aarch64 | ✅ |
| Android | aarch64, armv7 | ✅ |
| iOS | aarch64 | ✅ |

## Building

```bash
cargo build --release
```

The binary is produced at:
- Windows: `target/release/lavaxemu.dll`
- Linux: `target/release/liblavaxemu.so`
- macOS: `target/release/liblavaxemu.dylib`

Rename the binary to `lavaxemu_libretro.<ext>` (e.g.,
`lavaxemu_libretro.dll` on Windows) before placing it in RetroArch's `cores/`
directory.

Also copy `crates/lavaxemu-libretro/lavaxemu_libretro.info` to RetroArch's
`info/` directory.

## Installation

1. Build the core or download a pre-built binary from the
   [Releases](https://github.com/AloysHF/LavaXEmu/releases) page.
2. Place the renamed core binary in RetroArch's `cores/` directory.
3. Place `lavaxemu_libretro.info` in RetroArch's `info/` directory.
4. Launch RetroArch and select **Load Content** to load a `.lav` file.

## Input Mapping

The core maps RetroPad inputs to LavaX keys:

| RetroPad Button | LavaX Key |
| --- | --- |
| D-Pad Up | Up (20) |
| D-Pad Down | Down (21) |
| D-Pad Right | Right (22) |
| D-Pad Left | Left (23) |
| A | B |
| B | N |
| X | M |
| Y | G |
| Select | H |
| Start | J |
| L | Page Up (19) |
| R | Page Down (14) |

Keyboard input is also available when using RetroArch with a keyboard.

## Features

- **RGB565 video output** — 160×240 native resolution at 60 Hz
- **Save states** — full machine state snapshots via the libretro API
- **Reset** — `retro_reset` rebuilds the emulator runtime state
- **Pointer input** — mouse or touchscreen maps to the 160×240 guest screen

## Core Info

```
corename = "LavaXEmu"
display_name = "LavaXEmu"
authors = "AloysHF"
supported_extensions = "lav"
license = "GPL-2.0-or-later"
categories = "Emulator"
```

## Android Cross-Compilation

For Android targets, install the Android NDK and configure Rust:

```bash
rustup target add aarch64-linux-android armv7-linux-androideabi
```

Set the `ANDROID_NDK_ROOT` environment variable and use `cargo ndk` or
manual cross-compilation flags. See the
[Rust Android documentation](https://mozilla.github.io/firefox-browser-architecture/experiments/2017-09-21-rust-on-android.html)
for details.

## iOS Cross-Compilation

For iOS targets, install the iOS SDK and configure Rust:

```bash
rustup target add aarch64-apple-ios
```

Use `cargo-lipo` or build manually with the appropriate SDK flags. The
output is a static library that needs to be wrapped in a framework for
RetroArch on iOS.

## Troubleshooting

### Core not loading

- Ensure the core binary is named correctly (`lavaxemu_libretro.<ext>`).
- Check that the `.info` file is in the correct directory.
- Verify the core was built for the correct platform architecture.

### No video output

- Ensure the `.lav` file is a valid LavaX program.
- Check RetroArch's logs for error messages.

### Input not working

- Verify RetroPad is configured correctly in RetroArch's input settings.
- Some games may require specific key combinations.
