# Game compatibility fixes

These cases exercise the standalone content loader and the shared emulator core.
The local corpus is not distributed with the repository. Set `LAVAX_GAME_ROOT`
to its `lavax_game` directory, then run:

```powershell
cargo test -p lavaxemu --test game_compatibility -- --ignored
```

Only games with a completed display and input regression appear below. Startup
without an execution error alone does not establish playability.

## Dark Chess (暗子象棋)

- Content: `LAVA1 暗子象棋/未打包/暗子象棋.lav`.
- Problem: the game opens `/LavaData/SCC.dat`, but its unpacked companion
  `SCC.dat` sits beside the program. The missing-file screen exits on a key press.
- Fix: when an exact `/LavaData/...` file is missing, resolve an existing
  companion at the content root. Actual device directory files take priority.
  Reads and writes use the same file entry, preserving write-back to the
  original companion rather than making a divergent copy.
- Validation: title, player-name prompts, board, cursor movement, piece flipping,
  and continued execution; 1,800-frame scripted replay and an opt-in regression.

| Before | Board after input |
| --- | --- |
| ![Missing resource](images/fixes/dark-chess-before.png) | ![Playable board](images/fixes/dark-chess-after.png) |

## Pet World 3H (宠物世界3H)

- Content: `LAVA1 宠物世界/GVmaker/NC3000/宠物世界3H.lav`.
- Problems: opcode `0x00` was rejected at `0x72`, although the reference VM
  treats it as a no-op. Importing only `GVmaker/NC3000` also omitted the sibling
  `LavaData/宠世图片.dat` directory.
- Fix: accept zero padding as a no-op; recognize `GVmaker` and `LAVA` device
  layouts with a sibling `LavaData` directory in both frontends. Standalone
  write-back uses the same root. Flat bundles retain their own content root.
- Validation: pet sprites, status information, pet management, detail pages,
  menu navigation and continued execution. Unit tests cover padding and
  nested resource/write-back paths, including unrelated ancestor directories.
  Write-back preserves existing on-disk path casing on case-sensitive systems.
  External multiplayer is outside this basic-control check.

![Pet management and statistics](images/fixes/pet-world-3h-after.png)

## Princess Maker 4 (美少女梦工场4)

- Content: `LAVA2 美少女梦工场/非pac/PM4.lav`.
- Problem: `/LavaData/pm_data*.dat` and `PM4_cfg.dat` could not be opened in
  the flat bundle, leaving a blank screen and exiting in the first frame.
- Fix: the shared flat-companion resolution above restores all four data
  files. This game's separate regression covers the complete introduction,
  character confirmation, main room, and opening the status screen with Enter.
- Validation: 6,666 scripted frames without an error or exit, a 15,000-frame
  exploratory replay, and visual inspection of the room and status pages.

| Main room | Status screen after Enter |
| --- | --- |
| ![Main room](images/fixes/pm4-game.png) | ![Status screen](images/fixes/pm4-status.png) |

## Puzzled Sheep

- Content: `LAVA1 Puzzled Sheep/Psheep.lav`.
- Problem: starting the game opens `/GVMdata/Psheep.dat`, while the bundle
  places `Psheep.dat` beside the program; the game displayed `Error!` and exited.
- Fix: recognize the legacy `/GVMdata` resource prefix for existing flat
  companions, with the same exact-path priority as `/LavaData`.
- Validation: first-level loading, cursor movement, simulation animation,
  Escape to leave simulation, and 1,200 exploratory frames. Checked the
  supplied game source to verify the expected controls and resource name.

| Level selection | Simulation running |
| --- | --- |
| ![Level](images/fixes/sheep-level.png) | ![Simulation](images/fixes/sheep-running.png) |

## Hit (打地鼠)

- Content: `LAVA2 打地鼠/LAVA2/Hit.lav`.
- Problem: the `LAVA2` program folder was not recognized as part of a device
  layout, so `/LavaData/MULogo.dat` could not be loaded and `Read Error` appeared.
- Fix: recognize `LAVA2` alongside `LAVA` and `GVmaker` in both frontends.
- Validation: logo, playfield, nine TYUGHJBNM hit controls, P to pause and
  Enter to resume. The regression checks the framebuffer stays frozen during
  a 120-frame pause and changes again after resuming. Also replayed 1,800 frames.
- Timer follow-up: `GetTime` now advances with the deterministic 60 Hz clock,
  including calendar rollovers. Previously it always returned the initial
  timestamp, preventing the game's countdown from advancing. The regression
  checks the game's time buffer changes over two seconds; clock tests cover
  leap years, year rollover, SetTime/GetTime, and save-state phase restoration.

![Running game](images/fixes/hit-game.png)

## Counter-Strike 1.3 (CS 1.3)

- Content: `LAVA1 CS 1.3/非pac/Cs(lav)1_3.lav`.
- Problem: the flat bundle's `Csgun.dat` and other resources could not be
  opened through `/LavaData`, preventing normal startup.
- Fix: shared resource resolution restores startup, animation and mission
  assets. This game's separate regression exercises the title, setup menu,
  mission loading and direction controls after the calendar-clock fix.
- Validation: the replay selects Start, confirms Ready, and reaches combat.
  Direction input changes the actual camera coordinate, and the mission
  framebuffer changes as well. Stage checks follow guest execution rather
  than assuming a fixed animation duration.

| Title | Mission |
| --- | --- |
| ![Title](images/fixes/cs13-title.png) | ![Mission](images/fixes/cs13-mission.png) |

## Crazy F2 (疯狂F2)

- Content: `LAVA1 疯狂F2/F2.lav`.
- Problem: its five-second F2-press round polls the seconds returned by
  `GetTime`; the fixed timestamp previously prevented completion.
- Fix: the advancing guest calendar restores the round timer. This game's
  regression selects a platform, starts with F2, presses F2 five times, and
  waits for the score screen without injecting further input.
- Validation: the round completes and reports a positive press count. The
  initial title intentionally waits for F2, rather than Enter, to begin.

![Completed round](images/fixes/f2-score.png)

## Lost Fantasy

- Content: `LAVA1 lost fantasy/LF_demo.lav` and `LF_demo(PC).lav`.
- Problem: the device version continuously captures and redraws the full
  monochrome screen. Per-pixel buffer access made its 1,800-frame corpus
  replay exceed the 45-second scan limit. The PC version adds a 30 ms delay.
- Fix: copy in-bounds monochrome bitmaps in groups of eight pixels, and pack
  captured rows directly. Clipped, mirrored, inverted, and other color modes
  retain their existing paths. The bitmap fast paths preserve pixel results;
  presentation pacing is handled separately below.
- Validation: the same 1,800-frame replay completes in about 2.9 seconds when
  run individually on the development machine; frame 1,199 is identical to the
  previous capture. Both versions respond to D movement, with horizontal
  position and framebuffer differences checked against an idle clone.
  Bitmap tests exercise all 256 byte patterns and neighboring-pixel boundaries.

![PC version after movement](images/fixes/lost-fantasy.png)

## Speed Racing corpus variant (极速狂飙 / 生死时速II)

- Content: `LAVA1 极速狂飙/极速狂飙.lav`, whose in-game title is 生死时速II.
- Problem: `Refresh` copied the framebuffer but kept executing guest code in
  the same host frame. Unthrottled draw loops consumed CPU and advanced past
  the moment when the frontend could deliver steering input.
- Fix: `Refresh` now returns `Presented`, yielding to the host after each
  visible frame. Unit tests check that successive presented frames remain
  distinct. The instruction budget remains unchanged.
- Validation: enter the main menu, select a difficulty, and start the game.
  Right steering changes horizontal position and the rendered track compared
  with an idle clone. All previously completed game regressions are rerun;
  the CS 1.3 replay now explicitly handles its Ready confirmation.

![Track after steering](images/fixes/speed-racing.png)

## Modified Speed Racing II (生死时速Ⅱ修改版)

- Content: `LAVA1 生死时速II/生死时速Ⅱ（修改版）.lav`.
- Problem: its score loop repeatedly calls Locate, printf and Inkey without
  Refresh. Profiling found 6,250 identical score prints in one host frame,
  making the full replay exceed the 45-second scan limit.
- Fix: printf yields `Presented` after rendering its console output, giving
  the frontend a display/input boundary for console-driven loops as well.
- Validation: choose easy mode, steer right, reach a stable score screen, and
  return to the mode menu with Escape. Steering position is checked against
  an idle clone. A 3,600-frame replay completes in about 0.4 seconds locally.
  A core test confirms console output is presented before the next input poll.

| Score | Menu after Escape |
| --- | --- |
| ![Score](images/fixes/swr-score.png) | ![Mode menu](images/fixes/swr-menu.png) |

## Resource Manager (文件夹助手 3.7)

- Content: `LAVA1 百战天虫即时版/资源管理器.lav`.
- Problem: FileList returned cancellation immediately, so the program exited
  without showing its directory picker.
- Fix: implement the blocking file picker, GBK names, cursor and page navigation,
  selection/scroll return values, cancellation, and SetList help/delete flags.
  The caller handles directory changes and file operations, as in the reference VM.
- Validation: dismiss the splash, move the selection, open the file action menu,
  restore a pending picker save state, cancel, and confirm normal exit. Core tests
  cover Chinese names, paging, special keys, reset, and version 2 state migration.
  Save states now use version 3 and retain version 2 loading support.

| Directory picker | File actions |
| --- | --- |
| ![Directory](images/fixes/filelist-directory.png) | ![Actions](images/fixes/filelist-actions.png) |
