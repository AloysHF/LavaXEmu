# LavaXEmu — A LavaX virtual machine written in Rust

<p align="center">
  <img src="res/logo-banner.png" alt="LavaXEmu" width="600">
</p>

<p align="center">
  <a href="https://github.com/AloysHF/LavaXEmu/actions/workflows/ci.yml"><img src="https://github.com/AloysHF/LavaXEmu/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://github.com/AloysHF/LavaXEmu/releases/latest"><img src="https://img.shields.io/github/v/release/AloysHF/LavaXEmu" alt="Release"></a>
  <a href="https://github.com/AloysHF/LavaXEmu/releases"><img src="https://img.shields.io/github/downloads/AloysHF/LavaXEmu/total" alt="Downloads"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/License-GPL%202.0-blue.svg" alt="License: GPL 2.0"></a>
</p>

LavaXEmu is a cross-platform LavaX virtual machine written in Rust. It loads
`.lav` game programs and `.pac` packages and executes them through a pure-Rust bytecode
interpreter, providing:

- Standalone desktop emulator with window display
- Libretro core for RetroArch integration
- Headless, deterministic core library for testing and portability

The project is in early development. LAV file parsing, 117 bytecodes, 87
system API dispatches, indexed-color graphics, input handling, deterministic
clock, and memory virtual file system are implemented. Frontend features are
being progressively improved.

## Features

- **LAV format support** — file header parsing, code/data segment loading
- **Stack-based bytecode VM** — 117 opcodes for arithmetic, control flow,
  memory access, and system calls
- **System API bridge** — 87 firmware services for display, input, files,
  memory, audio, and utilities
- **Indexed-color display** — 160×240 resolution with 256-color palette
- **Keyboard and pointer input** — full keyboard mapping with mouse/touch
  support
- **Virtual file system** — sandboxed file access with selective write-back
- **Headless mode** — run N frames without a window for testing and batch
  processing (`--headless --frames`)
- **Screenshot capture** — automated PNG screenshot generation
- **Save states** — full machine state snapshots through the libretro API
- **Reset** — rebuilds the emulator runtime state (F10 in standalone,
  `retro_reset` in libretro)
- **Libretro integration** — playable libretro core with RGB565 video output,
  RetroPad input, content loading, save states, and reset
- **Browser play** — EmulatorJS core compiled to WebAssembly, with a ready-to-use
  [online player](https://AloysHF.github.io/LavaXEmu/play.html) that runs `.lav`
  programs without uploading them

Compatibility evidence is tracked in the [workflow audit](docs/Game-Corpus-Audit.md).
Five in a Row v0.80 has reproducible monochrome and grayscale workflows for
winning, undo/redo, record persistence, restarting and confirmed exit.
Five Tigers has complete-round workflows for alternating play, all five
scoring categories, restarting and exit. Yahtzee has a full-game workflow
covering held dice, reroll limits, all 14 scoring choices and final settlement.
Fairy Fate has LAV and PAC workflows covering map movement, saved progress
restoration, combat damage and confirmed defeat exit.
Arbitrary Picture Puzzle has a workflow completing all four pictures with
tile-swap checks, reconstructed image checks and normal completion exit.
Russian Poker has a workflow for movement boundaries, accelerated falling,
card locking, sampled scoring, stack overflow and Escape termination.
Tetris v1.3 has monochrome and grayscale workflows for rotation, pause/resume,
line clearing, scoring and ranking persistence. A fingerprinted compatibility
repair prevents corrupted ranking names in the monochrome release.
Adventure Island IV has monochrome and grayscale LAV/PAC workflows completing
the first stage, exercising damage and respawn, and restoring custom controls.
Battle on Namek has a workflow covering incorrect guesses, restarting after
three misses, all five stages and normal victory exit. A fingerprinted repair
restores new-game counters and makes its ending reachable.
Volley Shooting v1.5 has a workflow for shooting goals, goalkeeper actions and
record restoration. A fingerprinted repair preserves the separate record slots.

## Usage

### Standalone Mode

Download the latest binary from the
[Releases](https://github.com/AloysHF/LavaXEmu/releases) page and run:

```bash
lavaxemu path/to/game.lav
```

See the [Standalone Emulator](docs/Standalone-Emulator.md) guide for
installation, keyboard controls, headless mode, and all command-line options.

### RetroArch Mode

Install the core and load a game through RetroArch's **Load Content** menu.

See the [RetroArch Core](docs/RetroArch-Core.md) guide for installation,
supported platforms, RetroPad mapping, and features.

### Browser (EmulatorJS)

Open <https://AloysHF.github.io/LavaXEmu/play.html>, then pick a `.lav` program, a `.pac` package,
or a `.zip` that bundles it with its companion data files. It runs locally in
the browser through the LavaXEmu WebAssembly core; nothing is uploaded. See the
[EmulatorJS Core](docs/EmulatorJS-Core.md) guide for embedding the core in your
own EmulatorJS deployment, bundle contents, and controls.

## Building

Requires [Rust](https://www.rust-lang.org/tools/install) (stable).

### Standalone Mode

```bash
cargo build --release -p lavaxemu
cargo run --release -p lavaxemu -- path/to/game.lav
```

### Libretro Core (for RetroArch)

```bash
cargo build --release
```

The binary is produced at `target/release/lavaxemu.dll`
(`liblavaxemu.so` on Linux, `liblavaxemu.dylib` on macOS). Rename it to
`lavaxemu_libretro.<ext>` before placing it in RetroArch's `cores/`
directory.

### EmulatorJS Core (for the browser)

The browser core links the libretro core into EmulatorJS's RetroArch fork. It
needs an activated emsdk, `p7zip`, and the pinned nightly toolchain:

```bash
cargo +nightly build -p lavaxemu-libretro \
  --target wasm32-unknown-emscripten --profile wasm-release \
  -Zbuild-std=std,panic_abort

scripts/build-emulatorjs-core.sh
```

The bundle is written to `out/lavaxemu-emulatorjs.zip`. See
[EmulatorJS Core](docs/EmulatorJS-Core.md) for details.

## Architecture

```
crates/
├── lavaxemu-core/      # Platform-independent emulator engine (library)
│   └── src/
│       ├── lib.rs          # Crate root and public re-exports
│       ├── program.rs      # LAV file parsing and validation
│       ├── vm.rs           # Virtual machine state and execution
│       ├── emulator.rs     # High-level emulator API
│       ├── display.rs      # Framebuffer and palette management
│       ├── input.rs        # Input state management
│       ├── system.rs       # System API dispatch
│       ├── state.rs        # Save state codec
│       ├── vfs.rs          # Virtual file system
│       └── error.rs        # Error types
├── lavaxemu/           # Standalone binary (→ lavaxemu)
│   └── src/
│       ├── main.rs         # Window loop, CLI, input, screenshot
│       └── standalone/
│           ├── mod.rs      # Standalone module root
│           ├── cli.rs      # Command-line argument parsing
│           ├── content.rs  # Content loading and file management
│           └── input.rs    # Keyboard and mouse input mapping
└── lavaxemu-libretro/  # Libretro cdylib (→ lavaxemu_libretro.{dll,so,dylib})
    ├── lavaxemu_libretro.info  # RetroArch core metadata
    └── src/
        ├── lib.rs          # cdylib crate root
        └── ffi.rs          # libretro FFI bindings
```

See [Architecture](docs/architecture.md) for implementation details.

## Key Mappings (Standalone)

| LavaX Key | Keyboard |
| --- | --- |
| Direction keys | Arrow keys |
| A–Z | A–Z |
| B | B or 1 |
| Space | Space |
| 0 / period | 0 / period |
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

## Game Compatibility

The emulator loads `.lav` programs and uncompressed `.pac` packages. Display
size and graphics mode come from the program header. Compatibility testing
is in progress.

For verified operations and remaining entries, see the [Corpus Workflow Audit](docs/Game-Corpus-Audit.md)
and [Game Fixes](docs/Game-Fixes.md). The [Game Compatibility gallery](docs/Game-Compatibility.md)
contains historical startup screenshots.

## Testing

Run the unit tests:

```bash
cargo test --workspace
```

Game files are not included. Supply legally obtained LAV programs separately.

## Contributing

Contributions are welcome! Whether you're interested in fixing bugs, adding
features, improving documentation, or testing game compatibility, we'd love your
help. See [CONTRIBUTING.md](docs/CONTRIBUTING.md) for details.

## Acknowledgments

- [LavaXVM](https://github.com/leesoft-mirage/LavaXVM) — reference LavaX
  virtual machine implementation

## License

This project is licensed under the [GPL 2.0 License](LICENSE).

Format research notes are in [docs/lavax-format.md](docs/lavax-format.md).
Local test resources are git-ignored and not included in source or release
packages.
