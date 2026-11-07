# Architecture

LavaXEmu is a cross-platform LavaX virtual machine that runs `.lav` game
programs. The core is platform-independent and exposes a framebuffer plus
keyboard/pointer input to frontends.

## Crate Structure

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

## Boot Flow

1. `Program` parses the LAV file header, validates segment bounds, and loads
   code and data segments.
2. `Vm` initializes the evaluation stack, local variables, and program counter.
3. `Emulator` sets up the display, input state, and virtual file system.
4. The bytecode interpreter executes the program entry point.
5. Each 60 Hz frame invokes the guest program's update and render logic.

`Program` also applies narrowly scoped repairs to identified legacy releases
in the loaded image. Length and CRC checks protect unrelated programs. The
monochrome Tetris v1.3 repair initializes its name-entry buffer from the current
name before the guest reads reused local storage. The Battle on Namek repair
initializes round counters when starting a game and fixes the unreachable
five-stage ending predicate. The Volley Shooting v1.5 repair pads missing record
slots in the guest file and rewinds before updating the shooting record. The
Power Ski repair directs its ending pictures to the front buffer before the
guest waits for confirmation. The Pokemon PK repair separates move metadata
from party setup, reads P2's own roster values, counts P1 defeats correctly and
clears defeated members' HP. The Tunshi v0.29 repair stops enemy targeting when
the player's party is defeated, redraws the field after closing menus and
returns to the title after defeat or Escape from exploration.
The Sword and Foreign Land prologue repairs reserve both 2,000-byte native save
slots before seeking,
preserving existing bytes when the second slot is saved first.
The Four-player Mahjong repair checks the wall before each normal draw and
rejects a kong without a replacement tile, preventing the remaining count from wrapping.
The Zelda Minigames chest timer repair expires elapsed deadlines when an input
wait resumes after the remaining time has become negative.
The Space Battleship repair adds an 80 ms delay to each battle-loop return
for one exact release, including movement, firing and pause resume paths.
The Power Driller repairs share a 40 ms wait across all gameplay drawing
returns, replacing shorter waits in the first two levels of two identified
device releases. The paced release and general VM timing remain unchanged.
The Cosmos repair initializes ten record bytes and returns from its exit menu
through the existing score-save epilogue, protected by length and CRC checks.
The Pet Picture Converter repair subtracts the 18-byte album header before
reading and writing the imported picture payload for one identified release.
The Empire v4.1 repair aligns stored and current timer timestamps, reads the
resource timestamp value instead of its address literal, and retains construction
costs in the resource snapshot. It is restricted to two length/CRC fingerprints.
The Sky and Land Helper repair returns from monster browsing to the menu redraw
path, preventing its Escape key from also triggering exit, for one fingerprint.
The Ghost House repair matches the first selected save slot to its first native
file, correcting reversed load-menu selection for one length/CRC fingerprint.
These repairs do not modify the content file or
change the VM's general stack, local variables or file-seek bounds.

## Bytecode Execution

The LavaX VM uses a stack-based bytecode architecture with 117 opcodes:

- **Arithmetic**: add, sub, mul, div, mod, and bitwise operations
- **Control flow**: jump, conditional branches, function calls/returns
- **Stack manipulation**: push, pop, dup, swap
- **Memory access**: local/global variable load/store
- **System calls**: 87 system API dispatches for display, input, audio, etc.

The interpreter processes one instruction per step and maintains deterministic
execution timing.

Opcode `0x1a` returns the guest text buffer at `0x1400`. Text output and
LCD updates share those cells with direct guest writes; NUL cells render as
blank glyphs. TextOut mode 9 honors the source-inversion flag (`0x08`) for
both font sizes, preserving light text on dark menu backgrounds.
Save-state version 5 keeps that shared memory, while loading
versions 2 through 4 migrates their separate console cells into guest RAM.

## Display

The guest owns a 160×240 indexed-color screen with a 256-color palette. The
display module manages:

- Indexed framebuffer (1 byte per pixel)
- RGB palette (256 entries × 3 bytes)
- Conversion to XRGB8888 for window display
- Conversion to RGB888 for PNG screenshots

The desktop frontend converts the completed screen to 32-bit XRGB for `minifb`.
The libretro core outputs RGB565 to the frontend.

## Input

Input state includes:

- **Keyboard**: array of currently pressed key codes (0–255)
- **Pointer**: optional position (x, y) and pressed state

The standalone frontend maps physical keyboard keys and mouse input to LavaX
key codes. The libretro core maps RetroPad buttons to LavaX keys.

## Virtual File System

The VFS provides sandboxed file access for guest programs:

- Loads resources from the program's directory at startup
- Provides file read/write operations through system APIs
- Tracks modified files for selective write-back
- Uses unbounded in-memory guest storage. Extended `DiskCheck` reports a
  positive capacity capped at 32,767 for signed 16-bit callers; it does not query
  host disk capacity. `DiskReclaim` is a no-op because flash compaction is unnecessary.
- `--read-only` mode prevents any disk writes

## System API

The system API dispatch handles 87 firmware services including:

- Display control (screen mode, palette, drawing)
- Input queries (keyboard state, pointer position)
- File operations (open, read, write, close)
- Memory management (allocate, free, copy)
- Audio control (sound playback, volume)
- Timer and clock operations
- String and math utilities

Unsupported service entries return neutral values. Service usage counters
make missing behavior observable during compatibility work.

## Headless Execution

The standalone binary supports headless mode for deterministic testing:

```bash
lavaxemu path/to/game.lav --headless --frames 600 --read-only
```

This runs the same machine core without opening a window. The `--screenshot`
option captures the last frame as a PNG file. This is the preferred path for
automated regression testing.

## Save States

Save states capture the complete machine state:

- Program counter and evaluation stack
- Local and global variables
- Display framebuffer and palette
- Input state
- Virtual file system contents

Save states are versioned and checksummed for compatibility. The libretro
core exposes save states through the standard retro API.
