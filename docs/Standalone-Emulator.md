# Standalone Emulator

This guide covers installing and running the standalone `lavaxemu` binary,
loading games, keyboard controls, headless mode, and all command-line options.

## Supported Platforms

| Platform | Architecture | Status |
|----------|-------------|--------|
| Windows | x86_64 | ✅ |
| macOS | x86_64, aarch64 | ✅ |
| Linux | x86_64, aarch64 | ✅ |

## Installation

Download the latest standalone binary for your platform from the
[Releases](https://github.com/AloysHF/LavaXEmu/releases) page.

You can also build it from source:

```bash
cargo build --release -p lavaxemu
```

The binary is produced at `target/release/lavaxemu` (`.exe` on Windows).

## Synopsis

```text
lavaxemu [OPTIONS] <PROGRAM> [-- <ARGUMENTS>...]
```

## Options

| Option | Value | Default | Description |
|---|---|---|---|
| `<PROGRAM>` | path | *required* | Path to the LAV program file. |
| `--info` | flag | off | Parse the program and print its metadata without running it. |
| `--headless` | flag | off | Run without opening a window. |
| `--frames <COUNT>` | integer | `600` | Number of frames to run in headless mode. |
| `--screenshot <PATH>` | path | — | Save the last frame to a PNG file. |
| `--scale <SCALE>` | 1–8 | `3` | Initial integer window scale. |
| `--read-only` | flag | off | Do not write virtual file changes back to disk. |
| `-- <ARGUMENTS>...` | strings | — | Arguments exposed to the guest program. |

## Loading Games

The standalone emulator accepts `.lav` files:

```bash
# Load a game directly
lavaxemu path/to/game.lav

# Load with custom window scale
lavaxemu path/to/game.lav --scale 4

# Print program metadata
lavaxemu path/to/game.lav --info

# Pass arguments to the guest program
lavaxemu path/to/game.lav -- first second
```

## Default Key Mappings

| LavaX Key | Keyboard |
| --- | --- |
| Direction keys | Arrow keys |
| A–Z | A–Z |
| B | Space or 1 |
| N | 2 |
| M | 3 |
| G | 4 |
| H | 5 or Select |
| J | 6 or Start |
| T | 7 |
| Y | 8 |
| U | 9 |
| Page Up | Page Up or Tab |
| Page Down | Page Down or Backspace |
| Enter | Enter |
| Escape | Escape |

| Function Key | Action |
| --- | --- |
| F9 | Pause / Resume |
| F10 | Reset |
| F12 | Exit |

The window also accepts mouse clicks as pointer input on the 160×240 screen.

## Headless Mode

Run the emulator without a window — useful for automated testing and batch
processing:

```bash
# Run 600 frames silently and exit
lavaxemu path/to/game.lav --headless --frames 600

# Run headlessly with read-only mode
lavaxemu path/to/game.lav --headless --frames 600 --read-only
```

## Screenshot Mode

Capture a PNG screenshot after running a number of frames:

```bash
# Take a screenshot after 300 frames
lavaxemu path/to/game.lav --screenshot screenshot.png --frames 300

# Take a screenshot in read-only mode
lavaxemu path/to/game.lav --screenshot screenshot.png --frames 300 --read-only
```

This is used by the batch screenshot script (`scripts/batch-screenshots.ps1`)
to generate screenshots for all games at once.

## Virtual File System

The emulator loads resources from the program's directory into a virtual file
system. By default, modified files are written back to disk when the emulator
exits. Use `--read-only` to prevent any disk writes.

## Batch Screenshots

To capture screenshots for all LAV programs in the local validation directory,
run:

```powershell
pwsh scripts/batch-screenshots.ps1
```

The script builds the standalone `lavaxemu` executable, runs each program
with default or program-specific capture timing, and writes PNG captures
to `docs/images`. Use `-Frames`, `-Binary`, `-GameDirectory`, or
`-OutputDirectory` to override the script defaults where applicable.

## Examples

```bash
# Basic usage
lavaxemu path/to/game.lav

# Custom window scale
lavaxemu path/to/game.lav --scale 5

# Print program info
lavaxemu path/to/game.lav --info

# Take a screenshot and exit
lavaxemu path/to/game.lav --screenshot shot.png --frames 300

# Run 600 frames without a window
lavaxemu path/to/game.lav --headless --frames 600

# Read-only mode
lavaxemu path/to/game.lav --read-only

# Pass arguments to guest
lavaxemu path/to/game.lav -- arg1 arg2
```
