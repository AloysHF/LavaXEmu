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

## Bytecode Execution

The LavaX VM uses a stack-based bytecode architecture with 117 opcodes:

- **Arithmetic**: add, sub, mul, div, mod, and bitwise operations
- **Control flow**: jump, conditional branches, function calls/returns
- **Stack manipulation**: push, pop, dup, swap
- **Memory access**: local/global variable load/store
- **System calls**: 87 system API dispatches for display, input, audio, etc.

The interpreter processes one instruction per step and maintains deterministic
execution timing.

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
