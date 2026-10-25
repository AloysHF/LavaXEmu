# EmulatorJS Core (Browser)

LavaXEmu can run in the browser as a standard
[EmulatorJS](https://emulatorjs.org) core. The build follows EmulatorJS's own
pipeline: the Rust libretro core is compiled for `wasm32-unknown-emscripten`,
linked against the `EmulatorJS/RetroArch` fork, and packaged as the
`<core>-wasm.data` bundles that an EmulatorJS page loads at runtime.

The project site ships a ready-to-use player at
<https://AloysHF.github.io/LavaXEmu/play.html>: visitors pick a `.lav` program
from their own machine and it runs from a blob URL, so nothing is uploaded and
the site does not host any game data.

## What you get

Every nightly publishes a single asset:

| File | Contents |
|------|----------|
| `lavaxemu-emulatorjs.zip` | Both core variants, the build report `cores/reports/lavaxemu.json`, `core.json`, `build.json`, `license.txt` and a README |

The two `.data` files stay separate inside the ZIP: EmulatorJS requests
`<core>-wasm.data` or `<core>-legacy-wasm.data` by name depending on WebGL2
support, so one bundle cannot serve both cases.

The bundle is laid out like this:

```
lavaxemu-emulatorjs.zip
├── cores/
│   ├── lavaxemu-wasm.data
│   ├── lavaxemu-legacy-wasm.data
│   └── reports/lavaxemu.json
├── core.json
├── build.json
├── license.txt
└── README.md
```

Each `.data` file is a 7z archive containing `<core>_libretro.js`,
`<core>_libretro.wasm`, `core.json`, `build.json` and `license.txt`.

## Install into a site

EmulatorJS always fetches cores from `<EJS_pathtodata>cores/`, so host the
runtime yourself (or use the pinned CDN files) and drop the core beside it:

```
data/
├── loader.js
├── emulator.min.js
├── emulator.min.css
├── compression/
│   ├── extract7z.js       # extracts the core .data archive
│   └── extractzip.js      # extracts ZIP content
└── cores/
    ├── lavaxemu-wasm.data
    ├── lavaxemu-legacy-wasm.data
    └── reports/lavaxemu.json
```

`emulator.min.zip` only contains `emulator.min.js` and `emulator.min.css`;
the `compression/` files are loaded on demand from the same EmulatorJS data
directory. Without `extract7z.js` the core download ends in a "Network Error".

To upgrade the runtime, replace the runtime files with a newer EmulatorJS
release (≥ 4.2.3) and keep this `cores/` directory.

## Use

```html
<div id="game"></div>
<script>
  EJS_player = "#game";
  EJS_core = "lavaxemu";
  EJS_gameUrl = URL.createObjectURL(programFile);
  EJS_gameName = programFile.name;
  EJS_pathtodata = "data/";
  EJS_startOnLoaded = false;
</script>
<script src="data/loader.js"></script>
```

`site/play.html` in this repository is a complete example.

When the game comes from a blob URL, EmulatorJS writes it into the wasm
filesystem under `EJS_gameName`. Set that to the original file name, extension
included (for example `game.lav` or `game.zip`), otherwise the frontend cannot
infer the content type and the core receives an unknown path.

### Content

A bare `.lav` program loads on its own. Programs that read companion data
files must be zipped with them, because EmulatorJS hands the core one file. On
start the core imports the files unpacked next to the program into its virtual
file system.

Keep the program and its data in a folder inside the archive, for example:

```
game/game.lav
game/data.bin
game/saves/...
```

The extracted layout is preserved and every file below `game/` is imported. A
flat archive (program at the archive root) also works: the root files are
imported without descending into EmulatorJS's own virtual directories. See
[lavax-format.md](lavax-format.md) for the program format and
[Game-Compatibility.md](Game-Compatibility.md) for tested titles.

Game data is not distributed with this repository or required by the site: the
play page only loads files the visitor selects.

## Rendering and performance

The core renders its indexed-color framebuffer as `RGB565` through libretro's
software path, at the program's native resolution (up to 320×240). Both
variants are built because EmulatorJS picks the file name by WebGL2 support.

## Controls

EmulatorJS maps the RetroPad to its default keyboard and gamepad bindings:

| Input | LavaX Key |
|-------|-----------|
| Arrow keys | Directions |
| X / Z | B / N |
| A / S | G / M |
| Q / E | Page Up / Page Down |
| Enter | J |
| Right Shift | H |

Touch controls and physical gamepads are provided by EmulatorJS.

## Saves

The core has no battery-backed RAM, so `core.json` sets `"save": false` to hide
EmulatorJS's save-file buttons. Save states are implemented by the core and
handled by EmulatorJS; they stay in the browser's local storage.

## Building the bundle

The nightly workflow runs this automatically. To build locally you need an
activated [emsdk](https://emscripten.org/docs/getting_started/downloads.html)
(`emmake` in `PATH`), `p7zip`, `zip`, and the pinned nightly toolchain:

```bash
# Rust's prebuilt emscripten stdlib carries wasm exception handling, which
# Binaryen's Asyncify pass rejects. Rebuild std with panic=abort instead.
cargo +nightly build -p lavaxemu-libretro \
  --target wasm32-unknown-emscripten --profile wasm-release \
  -Zbuild-std=std,panic_abort

scripts/build-emulatorjs-core.sh
```

The script clones the pinned `EmulatorJS/RetroArch` revision, links the
staticlib with `Makefile.emulatorjs`, builds the WebGL2 and legacy variants, and
writes `out/lavaxemu-emulatorjs.zip`. `--work-dir` moves the RetroArch checkout
out of `target/`, which matters on CI because `rust-cache` restores `target/`
without the checkout's `.git`.

The Pages deployment fetches the pinned EmulatorJS runtime, downloads the
`lavaxemu-emulatorjs.zip` asset from the rolling `nightly-build` release, and
redeploys after every successful Nightly Build.
