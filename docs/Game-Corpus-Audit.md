# Local Corpus Workflow Audit

This is an in-progress audit of every `.lav` and `.pac` entry point in the
local `tmp/lavax_game` corpus. Versions, duplicate distributions, account tools,
editors and installers are retained as separate entries. The corpus is not
redistributed with this repository.

A **workflow passed** row means the linked regression finished successfully
after loading that exact entry. It covers the operations asserted in that test
and the screenshots reviewed in [Game Fixes](Game-Fixes.md); it does not mean
every level or optional feature has been verified. **Pending** means there is
no completed dedicated workflow for that entry. Startup screenshots alone
do not satisfy this status.

## Current evidence

- Input update checked on 2026-10-31: full rerun of 49 workflows, followed by a corrected Dragon Kiss intro replay and the dedicated WarCraft workflow. Those 50 workflows passed with the updated input behavior; later entries have separate incremental checks.
- Shared guest text memory checked on 2026-11-01: all 75 dedicated release workflows passed after the text-buffer repair and state migration.
- Inverted TextOut checked on 2026-11-01: all 77 dedicated release workflows passed after honoring the source-inversion flag.
- Virtual storage capacity checked on 2026-11-05: all 95 existing dedicated release workflows passed after the DiskCheck repair.
- Spot the Difference checked on 2026-11-07: one dedicated release workflow covers the unpacked and packaged entries, finds all three level-1 differences, and advances to level 2.
- Battleship checked on 2026-11-07: one dedicated release workflow covers the unpacked and packaged entries, finishes the intro chain, checks both directions of menu navigation and briefing cancel, fires and scores in battle, moves the ship both ways, and returns to the menu after game over.
- Boxing checked on 2026-11-07: one dedicated release workflow plays thirteen punch exchanges with a fixed key cycle, verifies the enemy counter-moves and both stamina values, reaches a knockout that returns to the title, and starts a rematch.
- Matchsticks checked on 2026-11-07: one dedicated release workflow rejects an over-limit take, plays a full deterministic game, checks the computer's remainder strategy through the last match, and verifies restart, quit and escape behavior.
- Pipe Connection checked on 2026-11-07: one dedicated release workflow starts a round, checks the source cell and three-minute timer, moves the cursor in all four directions, places and removes a pipe, pauses and resumes the countdown, returns to the title, and quits through the credits screen.
- Sokoban checked on 2026-11-07: one dedicated release workflow covers the packed and unpacked entries, moves and blocks against walls, slides and pairs glyphs across two levels, verifies the one-tile type-9 behaviour, advances with the clear screen and `N`, replays with `F1`, and exits with Escape.
- Billiards checked on 2026-11-07: one dedicated release workflow enters the deterministic first rack, checks all seven ball coordinates, turns the aim with arrows and WASD, fires a shot, verifies the cue displacement and power fields, finishes the animation, and quits from the title.
- Sudoku checked on 2026-11-07: one dedicated release workflow checks the title menu, all 81 given cells of level 1, cursor movement and wrapping, digit entry, the hint toggle, the unfinished-board dialog, the answer prompt and overlay, level switching both ways, the help screen and the exit path.
- Circuit Simulator checked on 2026-11-07: one dedicated release workflow covers the unpacked and packaged entries, checks menu row/column navigation and wrapping, browses all seven chips in the auto-scrolling viewer, opens the notes and the five-entry trigger list, edits the waveform cursors and input cells, simulates a latch, returns from the result and exit prompts, and halts from the credits.
- New Hero checked on 2026-11-07: the dedicated village workflow now covers the packaged entry as well; both entries reach byte-identical memory and frames after the 7,200-frame startup route and respond identically to up/down movement.
- Star Legend checked on 2026-11-07: the packaged entry runs its embedded registration program and writes the account record, and that record logs into the unpacked main program and moves on the map.
- Down 100 Floors checked on 2026-11-07: the title and two credit screens gate the run, Left and Right steer the character while Up and Down leave the frame unchanged at that point, and an unattended run reaches the game-over screen.
- Hold 20 Seconds checked on 2026-11-07: the title menu starts a survival run, all four arrow keys move the player to distinct positions, steering into the falling field ends the run with a result overlay, and Enter starts the next run.
- Fly 10000 Meters checked on 2026-11-07: the title starts the flight, F4 opens the speed adjustment, Up and Down change and restore the speed value, holding a key lifts the plane, and an unattended flight ends on credits that Escape closes.
- Moon City checked on 2026-11-07: the packaged and loose entries render the same title, the load item reports a missing record and returns, a new game takes an empty name into the village with four-direction movement, and the exit item halts the program.
- Unnamed Village checked on 2026-11-07: the map starts directly, Right walks and scrolls, Up and Left turn the character at the fenced spawn, Down is blocked there, and after walking east the character can leave the edge in the other directions.
- Dream Twenty-One checked on 2026-11-07: the splash starts a round, the arrow keys move the pile cursor, Enter places the next card into the selected pile and scores 50 points, two aces total 12, and Escape shows the game-over screen with the score and waits for a key before halting.
- Dream Mario checked on 2026-11-07: the intro ignores an early Enter, the title's Start entry shows the map prompt, Enter begins map 1, D and A walk right and left, L jumps and lands, W and S leave the idle frame unchanged, and Down selects the help entry.
- Happy Gomoku checked on 2026-11-07: the title toggles both modes, the cursor wraps around a 13-by-13 board, solo stones and the AI reply are checked, undo removes a round, restart clears the board, and five black stones in two-player mode show the win overlay and keep the score for a white-starting rematch.
- Rubber House checked on 2026-11-07: the title walks all five entries, opens the seven-page rules and the settings, fills and cancels the key binding, shows both locked entries, halts from the exit entry, starts level 1 through the prompt, and covers right walking with bead collection, the left drop, idle and vertical keys, plus both pause answers.
- Wuxia checked on 2026-11-07: the title walks all three entries with bidirectional wrapping, 初涉武林 plays the night dialog into the room scene, 游龙再现 opens the two-slot load menu and reports the empty slot, Escape returns to the selected title entry, and 豹隐山林 halts the program.
- Pipe Works checked on 2026-11-07: the title walks all six entries with clamped rows and reading-order columns, opens the manual, high-score list, difficulty selector and both about pages, starts the logo splash into the animated board with a moving cursor, returns with Escape, and halts from the exit entry. The 文曲星 build runs the same screens with addresses shifted by eight bytes, draws the difficulty marker one pixel lower, and leaves the title on screen when the exit entry halts.
- Rotary Slot Machine checked on 2026-11-07: one dedicated release workflow checks the splash and menu with bidirectional wrapping, both pages of the instructions and version screens, starting a round with 20 credits, betting and withdrawing on two columns, a deterministic spin and result, the quit dialog cancel/confirm paths, and the menu exit item.
- Bottomless Pit checked on 2026-11-07: one dedicated release workflow cycles the three difficulty levels, starts a fall, checks the horizontal-only steering, waits for the floor counter, saves the replay record through the `Yes?` prompt, replays it without input, and aborts with Escape.
- Jianghu: Pirate Legend checked on 2026-11-07: one dedicated release workflow wraps the three-entry title in both directions, advances the typed intro into a walkable field with horizontal and upward movement, reports the missing save from 渔龙再现 and retries it, and fades out from 豹隐山林 with an exit code of 0.
- Jianghu: Chaos checked on 2026-11-07: one dedicated release workflow wraps the three-entry title in both directions, advances the typed intro into a walkable field with horizontal and upward movement, reports the missing save from 再续前缘 over the two-slot load menu and returns to the selected entry, and fades out from 退隐江湖 with an exit code of 0.
- Wangwang Tycoon checked on 2026-11-07: one dedicated release workflow walks the four-entry title, opens and closes the manual, toggles both settings pages and the character modes, lists the four save slots and returns from the empty slot, starts the board through the scrolling transition, takes a turn into the purchase prompt and the next AI event, and exits from both the menu entry and Escape.
- Meteor Butterfly Sword checked on 2026-11-07: one dedicated release workflow leaves the loading splash for the three-entry brush title, walks the entries with clamped horizontal selection, opens the about page and the credits, adjusts and restores the difficulty bar while recording the post-adjustment escape behavior, starts the arena from the control page, moves and attacks with the letter keys, and quits with code 0.
- Haidian Floating Life checked on 2026-11-07: one dedicated release workflow enters a name, starts the life menu with its fixed values and key mapping, buys and sells a cheap good with exact cash changes, donates blood and takes treatment at the hospital, pays the bar bill, is refused an unaffordable computer and a small casino stake, takes and repays a bank loan, sleeps through a night with cash, debt, health and calendar changes, saves a 31-byte record, changes the statistics again, loads the record back, and halts from the system exit entry.
- Bomb Submarine checked on 2026-11-07: one dedicated release workflow walks the four-entry title menu with wrapping in both directions, clamps the level selector between one and nine and restarts it at one after Escape, starts the ninth level, moves the ship to both edges, scores 25 points with twenty z charges and 22 with twenty x charges, pauses and resumes the round with a frozen timer, expires the level timer back into the selector with the score cleared, scrolls the help text to its last page, and reaches both the story and exit credit endings with halt code 0.
- Spirit Sword Shooting checked on 2026-11-07: one dedicated release workflow opens the splash, first-run message and logo into the three-entry title with clamped ends, leaves the F5 help page with Down, opens the empty leaderboard, starts level one with ten arrows and the zeroed HUD, fires a hit for thirteen points, drains the quiver into level two with the arrows restored and the total kept, pauses and resumes the round, enters a leaderboard name, checks the 48-byte record and reads it back in a fresh emulator, and walks the four nested exit questions to a halt with code 0.
- Pirate Ship checked on 2026-11-07: one dedicated release workflow covers the packaged and unpacked entries, finishes the splash and logo, wraps the four-entry menu in both directions, scrolls the help text to its last page, opens all four credit pages, halts from the exit entry, wraps the difficulty selector, compares the easy, normal and hard starting fleets and sea layouts, moves the main ship with the arrows and letter keys, fires and lets the pirates take a turn, cancels and confirms the quit prompt, sinks three ships into the reef with life deductions, reaches game over, and returns to the difficulty selector.
- War Sword checked on 2026-11-07: one dedicated release workflow leaves the splash into the four-entry title, clamps the entry selection in both directions, toggles the random/download map and the three difficulties, types a name, adjusts and clamps the map width and height, starts the generated map with the temple under the cursor, moves and blocks the cursor in all four directions, opens the temple info and the field build menu, advances the turn counter, opens the system and memory-point panels, reports the missing save and returns to the first entry, shows the help page, and walks the editor canvas through its tile keys, start and city placement, the missing-start warning and the return to the menu.
- War Sword A checked on 2026-11-07: one dedicated release workflow leaves the splash into the four-icon title, walks and wraps the entry selection, opens the shared unfinished notice from the three stub entries, toggles the map and difficulty options, types a name, starts the default size screen and clamps the width and height at their minima, generates the scripted 20x15 map with fixed cells, moves and blocks the hero in all four directions, opens the unfinished and resource field messages, and opens the system menu and closes it from all four stub entries without writing files.
- 158 dedicated release workflows passed in this audit.
- 227 of 324 entry points were loaded by those successful workflows.
- 97 entry points still need dedicated workflow verification.
- The historical [startup gallery](Game-Compatibility.md) is a separate check
  and must not be interpreted as this workflow audit.

## Reproduce locally

Set `LAVAX_GAME_ROOT` to the corpus directory and run:

```powershell
cargo test --release -p lavaxemu --test game_compatibility -- --ignored --test-threads=1
```

To collect exact entry paths for one test, set `LAVAX_AUDIT_LOG` to a new log
file and run that test with `--exact`. Only use its logged entries as successful
evidence after the complete test exits successfully. `LAVAX_CAPTURE_DIR` selects
an optional screenshot directory. Keep each test's logs and captures separate.

## Entry matrix

| ID | Entry | Purpose | Workflow evidence |
| ---: | --- | --- | --- |
| 000 | `LAVA1 2 口水王1.2/口水王2 1.2灰度/口水王2.lav` | Game / game package | [Workflow passed: `spitting_king_versions_move_shoot_adjust_power_and_confirm_exit`](../crates/lavaxemu/tests/game_compatibility.rs#L2081) |
| 001 | `LAVA1 2 口水王1.2/口水王2 1.2黑白/口水王2.lav` | Game / game package | [Workflow passed: `spitting_king_versions_move_shoot_adjust_power_and_confirm_exit`](../crates/lavaxemu/tests/game_compatibility.rs#L2081) |
| 002 | `LAVA1 3D迷宫/3D迷宫.lav` | Game / game package | [Workflow passed: `maze_3d_walks_turns_and_switches_the_satellite_map`](../crates/lavaxemu/tests/game_compatibility.rs#L1936) |
| 003 | `LAVA1 Cloud/打包的文件/bobo_cloud.pac` | Game / game package | [Workflow passed: `cloud_versions_configure_keys_jump_and_persist_the_configuration`](../crates/lavaxemu/tests/game_compatibility.rs#L2022) |
| 004 | `LAVA1 Cloud/未打包的文件/cloud_all.lav` | Game / game package | [Workflow passed: `cloud_versions_configure_keys_jump_and_persist_the_configuration`](../crates/lavaxemu/tests/game_compatibility.rs#L2022) |
| 005 | `LAVA1 Crazy_ball/crazyball.lav` | Game / game package | Pending |
| 006 | `LAVA1 Crazy_ball/mapedit.lav` | Map editor | Pending |
| 007 | `LAVA1 CS 1.3/Cs(lav)1_3.pac` | Game / game package | [Workflow passed: `counter_strike_13_loads_resources_and_enters_a_mission`](../crates/lavaxemu/tests/game_compatibility.rs#L418) |
| 008 | `LAVA1 CS 1.3/非pac/Cs(lav)1_3.lav` | Game / game package | [Workflow passed: `counter_strike_13_loads_resources_and_enters_a_mission`](../crates/lavaxemu/tests/game_compatibility.rs#L418) |
| 009 | `LAVA1 CS蓝色行动/CS蓝色行动.lav` | Game / game package | Pending |
| 010 | `LAVA1 Dance_all/打包的文件/BoBo_dance.pac` | Game / game package | [Workflow passed: `bobo_dance_versions_score_the_direction_sequence_and_open_the_help`](../crates/lavaxemu/tests/game_compatibility.rs#L2149) |
| 011 | `LAVA1 Dance_all/未打包的文件/Bobo_Dance.lav` | Game / game package | [Workflow passed: `bobo_dance_versions_score_the_direction_sequence_and_open_the_help`](../crates/lavaxemu/tests/game_compatibility.rs#L2149) |
| 012 | `LAVA1 EM星纪传说—格斗/EM星纪传说—格斗.lav` | Game / game package | [Workflow passed: `em_fighter_moves_attacks_and_finishes_the_bout`](../crates/lavaxemu/tests/game_compatibility.rs#L2298) |
| 013 | `LAVA1 IQ大作战/IQ大作战.lav` | Game / game package | [Workflow passed: `iq_battle_moves_removes_groups_uses_a_bomb_and_returns_to_the_menu`](../crates/lavaxemu/tests/game_compatibility.rs#L2342) |
| 014 | `LAVA1 Just_Fly/Just_Fly.pac` | Game / game package | [Workflow passed: `just_fly_versions_respond_in_both_modes_and_replay_a_saved_record`](../crates/lavaxemu/tests/game_compatibility.rs#L2216) |
| 015 | `LAVA1 Just_Fly/非pac/Just_Fly.lav` | Game / game package | [Workflow passed: `just_fly_versions_respond_in_both_modes_and_replay_a_saved_record`](../crates/lavaxemu/tests/game_compatibility.rs#L2216) |
| 016 | `LAVA1 LAVA2 江湖2_demo/world2.lav` | Game / game package | Pending |
| 017 | `LAVA1 LAVA2 江湖2_demo/world2_16.lav` | Game / game package | Pending |
| 018 | `LAVA1 Lazy YY牛奶大战/Ver1.2/打包的文件/LazyYY牛奶大战.pac` | Game / game package | [Workflow passed: `lazy_yy_versions_feed_pause_and_return_to_the_menu`](../crates/lavaxemu/tests/game_compatibility.rs#L1357) |
| 019 | `LAVA1 Lazy YY牛奶大战/Ver1.2/未打包的文件/LazyYY.lav` | Game / game package | [Workflow passed: `lazy_yy_versions_feed_pause_and_return_to_the_menu`](../crates/lavaxemu/tests/game_compatibility.rs#L1357) |
| 020 | `LAVA1 Lazy YY牛奶大战/Ver1.5 For TC800 Only/Lazy YY牛奶大战.pac` | Game / game package | [Workflow passed: `lazy_yy_versions_feed_pause_and_return_to_the_menu`](../crates/lavaxemu/tests/game_compatibility.rs#L1357) |
| 021 | `LAVA1 lost fantasy/LF_demo(PC).lav` | Game / game package | [Workflow passed: `lost_fantasy_device_and_pc_versions_draw_and_move`](../crates/lavaxemu/tests/game_compatibility.rs#L523) |
| 022 | `LAVA1 lost fantasy/LF_demo.lav` | Game / game package | [Workflow passed: `lost_fantasy_device_and_pc_versions_draw_and_move`](../crates/lavaxemu/tests/game_compatibility.rs#L523) |
| 023 | `LAVA1 PACMAN/PACMAN.pac` | Game / game package | [Workflow passed: `pacman_versions_select_a_stage_move_and_finish_a_round`](../crates/lavaxemu/tests/game_compatibility.rs#L1983) |
| 024 | `LAVA1 PACMAN/非pac/PACMAN.lav` | Game / game package | [Workflow passed: `pacman_versions_select_a_stage_move_and_finish_a_round`](../crates/lavaxemu/tests/game_compatibility.rs#L1983) |
| 025 | `LAVA1 puke21/扑克21.lav` | Game / game package | Pending |
| 026 | `LAVA1 Puzzled Sheep/Psheep.lav` | Game / game package | [Workflow passed: `puzzled_sheep_loads_levels_moves_the_cursor_and_runs_the_simulation`](../crates/lavaxemu/tests/game_compatibility.rs#L348) |
| 027 | `LAVA1 QUEERBOX/QUEERBOX.lav` | Game / game package | [Workflow passed: `queerbox_moves_a_top_block_restarts_and_returns_to_the_title`](../crates/lavaxemu/tests/game_compatibility.rs#L2404) |
| 028 | `LAVA1 Q版Dancer/Q版Dancer.lav` | Game / game package | [Workflow passed: `q_dancer_enters_the_test_answers_a_sequence_and_scores`](../crates/lavaxemu/tests/game_compatibility.rs#L2450) |
| 029 | `LAVA1 RC赛车/RC赛车.lav` | Game / game package | [Workflow passed: `rc_racing_accelerates_steers_pauses_and_returns_to_the_menu`](../crates/lavaxemu/tests/game_compatibility.rs#L2522) |
| 030 | `LAVA1 Rush_Out/Rush_Out_For_Lava(8K).lav` | Game / game package | [Workflow passed: `rush_out_variants_fly_drop_bombs_pause_and_return_to_the_menu`](../crates/lavaxemu/tests/game_compatibility.rs#L2588) |
| 031 | `LAVA1 Rush_Out/Rush_Out_For_Lava1(16k).lav` | Game / game package | [Workflow passed: `rush_out_variants_fly_drop_bombs_pause_and_return_to_the_menu`](../crates/lavaxemu/tests/game_compatibility.rs#L2588) |
| 032 | `LAVA1 Rush_Out/Rush_Out_For_Lava2(16k).lav` | Game / game package | [Workflow passed: `rush_out_variants_fly_drop_bombs_pause_and_return_to_the_menu`](../crates/lavaxemu/tests/game_compatibility.rs#L2588) |
| 033 | `LAVA1 SnowFight(打雪仗)/SnowFight.lav` | Game / game package | [Workflow passed: `snow_fight_opens_help_moves_throws_and_finishes_a_round`](../crates/lavaxemu/tests/game_compatibility.rs#L2728) |
| 034 | `LAVA1 SpaceWars/SWR无延时.lav` | Game / game package | [Workflow passed: `space_wars_variants_move_fire_and_exit_the_mission`](../crates/lavaxemu/tests/game_compatibility.rs#L2811) |
| 035 | `LAVA1 SpaceWars/SWR有延时.lav` | Game / game package | [Workflow passed: `space_wars_variants_move_fire_and_exit_the_mission`](../crates/lavaxemu/tests/game_compatibility.rs#L2811) |
| 036 | `LAVA1 TRAVEL/travel.lav` | Game / game package | [Workflow passed: `travel_variants_explore_fight_collect_the_tools_and_finish_the_demo`](../crates/lavaxemu/tests/game_compatibility.rs#L2889) |
| 037 | `LAVA1 TRAVEL/打包travel.pac` | Game / game package | [Workflow passed: `travel_variants_explore_fight_collect_the_tools_and_finish_the_demo`](../crates/lavaxemu/tests/game_compatibility.rs#L2889) |
| 038 | `LAVA1 WarCraft/WarCraft.pac` | Game / game package | [Workflow passed: `warcraft_variants_produce_move_collect_income_and_exit`](../crates/lavaxemu/tests/game_compatibility.rs#L3022) |
| 039 | `LAVA1 WarCraft/非pac/WarCraft.lav` | Game / game package | [Workflow passed: `warcraft_variants_produce_move_collect_income_and_exit`](../crates/lavaxemu/tests/game_compatibility.rs#L3022) |
| 040 | `LAVA1 七彩泡泡/七彩泡泡.lav` | Game / game package | [Workflow passed: `rainbow_bubbles_moves_fires_pauses_and_replays_a_saved_round`](../crates/lavaxemu/tests/game_compatibility.rs#L3107) |
| 041 | `LAVA1 七彩连珠/七彩连珠.lav` | Game / game package | [Workflow passed: `color_lines_selects_moves_cancels_and_scores_a_line`](../crates/lavaxemu/tests/game_compatibility.rs#L3189) |
| 042 | `LAVA1 七王五二三/七王五二三.lav` | Game / game package | [Workflow passed: `seven_king_selects_plays_passes_finishes_and_restarts_a_round`](../crates/lavaxemu/tests/game_compatibility.rs#L3286) |
| 043 | `LAVA1 三国志/三国志.pac` | Game / game package | [Workflow passed: `three_kingdoms_develops_recruits_and_reloads_a_saved_city`](../crates/lavaxemu/tests/game_compatibility.rs#L3381) |
| 044 | `LAVA1 中国灯谜/中国灯谜.lav` | Game / game package | [Workflow passed: `chinese_riddles_browses_all_categories_reveals_answers_and_exits`](../crates/lavaxemu/tests/game_compatibility.rs#L3524) |
| 045 | `LAVA1 中国烟火 1.1/中国烟火.lav` | Game / game package | [Workflow passed: `chinese_fireworks_rotates_connects_scores_pauses_and_confirms_exit`](../crates/lavaxemu/tests/game_compatibility.rs#L3627) |
| 046 | `LAVA1 中国象棋/象棋90.lav` | Game / game package | [Workflow passed: `xiangqi_moves_captures_undoes_and_reloads_a_saved_game`](../crates/lavaxemu/tests/game_compatibility.rs#L3775) |
| 047 | `LAVA1 中国麻将/中国麻将.lav` | Game / game package | [Workflow passed: `chinese_mahjong_draws_selects_discards_and_finishes_a_hand`](../crates/lavaxemu/tests/game_compatibility.rs#L3918) |
| 048 | `LAVA1 中学传奇/打包的/SCH_RPGPACK.pac` | Game / game package | [Workflow passed: `school_legend_versions_move_use_medicine_and_reload_progress`](../crates/lavaxemu/tests/game_compatibility.rs#L3986) |
| 049 | `LAVA1 中学传奇/未打包的/中学传奇.lav` | Game / game package | [Workflow passed: `school_legend_versions_move_use_medicine_and_reload_progress`](../crates/lavaxemu/tests/game_compatibility.rs#L3986) |
| 050 | `LAVA1 五子连珠/WZLZ_16灰度.lav` | Game / game package | [Workflow passed: `five_in_a_row_versions_win_undo_save_reload_and_restart`](../crates/lavaxemu/tests/game_compatibility.rs#L4195) |
| 051 | `LAVA1 五子连珠/WZLZ_黑白.lav` | Game / game package | [Workflow passed: `five_in_a_row_versions_win_undo_save_reload_and_restart`](../crates/lavaxemu/tests/game_compatibility.rs#L4195) |
| 052 | `LAVA1 五虎/Fivet.lav` | Game / game package | [Workflow passed: `five_tigers_scores_complete_boards_and_restarts`](../crates/lavaxemu/tests/game_compatibility.rs#L4340) |
| 053 | `LAVA1 五骰Yahtzee/Yahtzee.lav` | Game / game package | [Workflow passed: `yahtzee_holds_rerolls_scores_every_category_and_finishes`](../crates/lavaxemu/tests/game_compatibility.rs#L4440) |
| 054 | `LAVA1 仙剑后传/Pala.lav` | Game / game package | Pending |
| 055 | `LAVA1 仙剑后传/Pala_.lav` | Game / game package | Pending |
| 056 | `LAVA1 仙剑后传/Pala_TC800.lav` | Game / game package | Pending |
| 057 | `LAVA1 仙境奇缘/仙境奇缘/打包后的文件/仙境奇缘.pac` | Game / game package | [Workflow passed: `fairy_fate_moves_saves_restores_and_resolves_a_battle`](../crates/lavaxemu/tests/game_compatibility.rs#L4617) |
| 058 | `LAVA1 仙境奇缘/仙境奇缘/未打包文件/仙境奇缘.lav` | Game / game package | [Workflow passed: `fairy_fate_moves_saves_restores_and_resolves_a_battle`](../crates/lavaxemu/tests/game_compatibility.rs#L4617) |
| 059 | `LAVA1 任意拼图/任意拼图.lav` | Game / game package | [Workflow passed: `arbitrary_picture_puzzle_swaps_tiles_completes_all_pictures_and_exits`](../crates/lavaxemu/tests/game_compatibility.rs#L4741) |
| 060 | `LAVA1 俄罗斯扑克/俄罗斯扑克.lav` | Game / game package | [Workflow passed: `russian_poker_moves_accelerates_locks_scores_and_finishes`](../crates/lavaxemu/tests/game_compatibility.rs#L4847) |
| 061 | `LAVA1 俄罗斯方块1.3/俄罗斯方块灰.lav` | Game / game package | [Workflow passed: `tetris_13_versions_rotate_clear_lines_and_preserve_ranking_names`](../crates/lavaxemu/tests/game_compatibility.rs#L4917) |
| 062 | `LAVA1 俄罗斯方块1.3/俄罗斯方块黑.lav` | Game / game package | [Workflow passed: `tetris_13_versions_rotate_clear_lines_and_preserve_ranking_names`](../crates/lavaxemu/tests/game_compatibility.rs#L4917) |
| 063 | `LAVA1 假日神秘岛/假日神秘岛 abcdef.lav` | Game / game package | [Workflow passed: `holiday_island_finishes_the_intro_talks_and_moves_on_the_map`](../crates/lavaxemu/tests/game_compatibility.rs#L1064) |
| 064 | `LAVA1 先赌为快/先赌★为快.lav` | Game / game package | [Workflow passed: `gamble_creates_a_local_account_and_accepts_bets`](../crates/lavaxemu/tests/game_compatibility.rs#L897) |
| 065 | `LAVA1 冒险岛IV/黑白版/打包的文件/冒险岛IV.pac` | Game / game package | [Workflow passed: `adventure_island_iv_versions_clear_a_stage_and_restore_custom_controls`](../crates/lavaxemu/tests/game_compatibility.rs#L5108) |
| 066 | `LAVA1 冒险岛IV/黑白版/未打包的文件/冒险岛IV.lav` | Game / game package | [Workflow passed: `adventure_island_iv_versions_clear_a_stage_and_restore_custom_controls`](../crates/lavaxemu/tests/game_compatibility.rs#L5108) |
| 067 | `LAVA1 冠军足球经理05-06赛季0.7/CFM.lav` | Game / game package | [Workflow passed: `champion_football_manager_creates_a_coach_and_finishes_a_match`](../crates/lavaxemu/tests/game_compatibility.rs#L1516) |
| 068 | `LAVA1 决战娜美克/决战娜美克.lav` | Game / game package | [Workflow passed: `namek_retries_after_three_misses_and_completes_five_stages`](../crates/lavaxemu/tests/game_compatibility.rs#L5350) |
| 069 | `LAVA1 凌空抽射1.5/凌空抽射v1.5.lav` | Game / game package | [Workflow passed: `volley_shooting_scores_saves_records_and_drives_goalkeeper_controls`](../crates/lavaxemu/tests/game_compatibility.rs#L5443) |
| 070 | `LAVA1 凤凰战斗机/Fighter.lav` | Game / game package | [Workflow passed: `phoenix_fighter_moves_shoots_bombs_pauses_retries_and_clears_a_stage`](../crates/lavaxemu/tests/game_compatibility.rs#L5609) |
| 071 | `LAVA1 动力滑雪/滑雪.lav` | Game / game package | [Workflow passed: `power_ski_moves_jumps_recovers_and_presents_both_endings`](../crates/lavaxemu/tests/game_compatibility.rs#L5759) |
| 072 | `LAVA1 勇者传说/勇者传说.lav` | Game / game package | [Workflow passed: `brave_legend_restarts_moves_platforms_and_completes_all_forty_stages`](../crates/lavaxemu/tests/game_compatibility.rs#L5926) |
| 073 | `LAVA1 勇者斗恶龙/勇者斗恶龙.lav` | Game / game package | [Workflow passed: `dragon_quest_editions_trade_defend_recruit_and_restore_progress`](../crates/lavaxemu/tests/game_compatibility.rs#L6189) |
| 074 | `LAVA1 勇者斗恶龙特别版/勇者斗恶龙.lav` | Game / game package | [Workflow passed: `dragon_quest_editions_trade_defend_recruit_and_restore_progress`](../crates/lavaxemu/tests/game_compatibility.rs#L6189) |
| 075 | `LAVA1 勇闯地狱/勇闯地狱.lav` | Game / game package | [Workflow passed: `hell_adventure_triggers_switches_restarts_and_clears_the_first_stage`](../crates/lavaxemu/tests/game_compatibility.rs#L6340) |
| 076 | `LAVA1 博士失踪记/博士失踪记.lav` | Game / game package | [Workflow passed: `missing_doctor_talks_collects_an_item_and_solves_the_alarm_puzzle`](../crates/lavaxemu/tests/game_compatibility.rs#L6435) |
| 077 | `LAVA1 口袋妖怪PK场/pmpk.pac` | Game / game package | [Workflow passed: `pokemon_pk_configures_both_parties_replaces_fainted_members_and_finishes_both_results`](../crates/lavaxemu/tests/game_compatibility.rs#L6558) |
| 078 | `LAVA1 口袋妖怪（未完成）/pokemon.lav` | Game / game package | [Workflow passed: `pokemon_prototype_explores_collects_an_item_and_browses_its_existing_menus`](../crates/lavaxemu/tests/game_compatibility.rs#L6781) |
| 079 | `LAVA1 吃月饼/吃月饼.lav` | Game / game package | [Workflow passed: `mooncake_opens_help_sets_speed_moves_and_reacts_to_all_five_items`](../crates/lavaxemu/tests/game_compatibility.rs#L6910) |
| 080 | `LAVA1 吞食天地0.29/tunshi_V0.29.lav` | Game / game package | [Workflow passed: `tunshi_equips_explores_and_finishes_both_battle_results`](../crates/lavaxemu/tests/game_compatibility.rs#L7015) |
| 081 | `LAVA1 听剑/听剑.pac` | Game / game package | [Workflow passed: `sword_versions_equip_follow_the_opening_story_and_finish_a_battle`](../crates/lavaxemu/tests/game_compatibility.rs#L7289), [Workflow passed: `sword_versions_preserve_both_native_save_slots`](../crates/lavaxemu/tests/game_compatibility.rs#L7202) |
| 082 | `LAVA1 听剑/非pac/Sword.lav` | Game / game package | [Workflow passed: `sword_versions_equip_follow_the_opening_story_and_finish_a_battle`](../crates/lavaxemu/tests/game_compatibility.rs#L7289), [Workflow passed: `sword_versions_preserve_both_native_save_slots`](../crates/lavaxemu/tests/game_compatibility.rs#L7202) |
| 083 | `LAVA1 四人麻将/MahJong.lav` | Game / game package | [Workflow passed: `four_player_mahjong_selects_discards_and_restarts_after_a_draw`](../crates/lavaxemu/tests/game_compatibility.rs#L7412) |
| 084 | `LAVA1 困虫斗 1.1/打包/kcd.pac` | Game / game package | [Workflow passed: `trapped_spider_versions_capture_territory_win_and_restore_the_ranking`](../crates/lavaxemu/tests/game_compatibility.rs#L7522) |
| 085 | `LAVA1 困虫斗 1.1/未打包/困虫斗.lav` | Game / game package | [Workflow passed: `trapped_spider_versions_capture_territory_win_and_restore_the_ranking`](../crates/lavaxemu/tests/game_compatibility.rs#L7522) |
| 086 | `LAVA1 圈地运动/圈地运动.lav` | Game / game package | [Workflow passed: `territory_game_finishes_two_player_and_computer_boards`](../crates/lavaxemu/tests/game_compatibility.rs#L7656) |
| 087 | `LAVA1 圣金/Shengjin2.lav` | Game / game package | [Workflow passed: `saints_vs_gold_completes_training_and_an_eight_member_battle`](../crates/lavaxemu/tests/game_compatibility.rs#L7771) |
| 088 | `LAVA1 坦克2004/坦克2004.pac` | Game / game package | [Workflow passed: `tank_2004_versions_move_fire_pause_and_restore_a_native_save`](../crates/lavaxemu/tests/game_compatibility.rs#L7898) |
| 089 | `LAVA1 坦克2004/非PAC/坦克2004.lav` | Game / game package | [Workflow passed: `tank_2004_versions_move_fire_pause_and_restore_a_native_save`](../crates/lavaxemu/tests/game_compatibility.rs#L7898) |
| 090 | `LAVA1 塞尔达小游戏/文曲星版Zelda.lav` | Game / game package | [Workflow passed: `zelda_minigames_hit_lizards_open_a_chest_time_out_and_configure_controls`](../crates/lavaxemu/tests/game_compatibility.rs#L8000) |
| 091 | `LAVA1 大战略/大战略.pac` | Game / game package | [Workflow passed: `strategy_versions_move_attack_change_turns_and_restore_a_campaign`](../crates/lavaxemu/tests/game_compatibility.rs#L8177) |
| 092 | `LAVA1 大战略/非pac/dzl.lav` | Game / game package | [Workflow passed: `strategy_versions_move_attack_change_turns_and_restore_a_campaign`](../crates/lavaxemu/tests/game_compatibility.rs#L8177) |
| 093 | `LAVA1 天空格斗艺术正式版/HFAcs.lav` | Game / game package | [Workflow passed: `sky_fighting_art_configures_moves_fires_pauses_and_finishes_a_battle`](../crates/lavaxemu/tests/game_compatibility.rs#L8251) |
| 094 | `LAVA1 天鹰战机·血战/Eagle.lav` | Game / game package | [Workflow passed: `eagle_versions_aim_fire_pause_finish_and_restore_the_ranking`](../crates/lavaxemu/tests/game_compatibility.rs#L8373) |
| 095 | `LAVA1 天鹰战机·血战/VS版/NC2600.lav` | Game / game package | [Workflow passed: `eagle_versions_aim_fire_pause_finish_and_restore_the_ranking`](../crates/lavaxemu/tests/game_compatibility.rs#L8373) |
| 096 | `LAVA1 天鹰战机·血战/VS版/NC3000.lav` | Game / game package | [Workflow passed: `eagle_versions_aim_fire_pause_finish_and_restore_the_ranking`](../crates/lavaxemu/tests/game_compatibility.rs#L8373) |
| 097 | `LAVA1 天鹰战机·血战/VS版/TC1000.lav` | Game / game package | [Workflow passed: `eagle_versions_aim_fire_pause_finish_and_restore_the_ranking`](../crates/lavaxemu/tests/game_compatibility.rs#L8373) |
| 098 | `LAVA1 天鹰战机·血战/VS版/TC1000S.lav` | Game / game package | [Workflow passed: `eagle_versions_aim_fire_pause_finish_and_restore_the_ranking`](../crates/lavaxemu/tests/game_compatibility.rs#L8373) |
| 099 | `LAVA1 天鹰战机·血战/VS版/TC808.lav` | Game / game package | [Workflow passed: `eagle_versions_aim_fire_pause_finish_and_restore_the_ranking`](../crates/lavaxemu/tests/game_compatibility.rs#L8373) |
| 100 | `LAVA1 天龙八步/Liuye.lav` | Game / game package | [Workflow passed: `liuye_moves_answers_a_riddle_fights_and_restores_native_progress`](../crates/lavaxemu/tests/game_compatibility.rs#L8511) |
| 101 | `LAVA1 太空寻宝/太空寻宝.lav` | Game / game package | [Workflow passed: `space_treasure_places_mirrors_clears_a_stage_retries_and_exits`](../crates/lavaxemu/tests/game_compatibility.rs#L8618) |
| 102 | `LAVA1 太空战舰/太空战舰.lav` | Game / game package | [Workflow passed: `space_battleship_paces_turns_moves_fires_pauses_and_finishes_a_battle`](../crates/lavaxemu/tests/game_compatibility.rs#L8701) |
| 103 | `LAVA1 奇门九星诀房屋风水勘测/9stars.lav` | Utility / education | [Workflow passed: `nine_stars_cycles_all_orientation_results_and_exits`](../crates/lavaxemu/tests/game_compatibility.rs#L8795) |
| 104 | `LAVA1 宇宙大战/宇宙大战.lav` | Game / game package | [Workflow passed: `cosmos_moves_pauses_finishes_and_restores_native_records`](../crates/lavaxemu/tests/game_compatibility.rs#L8867) |
| 105 | `LAVA1 宠物世界/GVmaker/NC2600/宠世L_2K6.lav` | Game / game package | [Workflow passed: `pet_world_legacy_versions_reopen_the_main_game_save`](../crates/lavaxemu/tests/game_compatibility.rs#L684) |
| 106 | `LAVA1 宠物世界/GVmaker/NC2600/宠物世界26.lav` | Game / game package | [Workflow passed: `pet_world_legacy_versions_reopen_the_main_game_save`](../crates/lavaxemu/tests/game_compatibility.rs#L684) |
| 107 | `LAVA1 宠物世界/GVmaker/NC3000/宠世L_3K.lav` | Game / game package | [Workflow passed: `pet_world_legacy_versions_reopen_the_main_game_save`](../crates/lavaxemu/tests/game_compatibility.rs#L684) |
| 108 | `LAVA1 宠物世界/GVmaker/NC3000/宠物世界3H.lav` | Game / game package | [Workflow passed: `pet_world_3h_loads_nested_assets_and_opens_pet_management`](../crates/lavaxemu/tests/game_compatibility.rs#L290) |
| 109 | `LAVA1 宠物世界/GVmaker/NC3000/宠物世界3K.lav` | Game / game package | [Workflow passed: `pet_world_legacy_versions_reopen_the_main_game_save`](../crates/lavaxemu/tests/game_compatibility.rs#L684) |
| 110 | `LAVA1 宠物世界/GVmaker/TC1000/宠世L_1K.lav` | Game / game package | [Workflow passed: `pet_world_legacy_versions_reopen_the_main_game_save`](../crates/lavaxemu/tests/game_compatibility.rs#L684) |
| 111 | `LAVA1 宠物世界/GVmaker/TC1000/宠物世界1K.lav` | Game / game package | [Workflow passed: `pet_world_legacy_versions_reopen_the_main_game_save`](../crates/lavaxemu/tests/game_compatibility.rs#L684) |
| 112 | `LAVA1 宠物世界/GVmaker/TC1000S/宠世L_1KS.lav` | Game / game package | [Workflow passed: `pet_world_legacy_versions_reopen_the_main_game_save`](../crates/lavaxemu/tests/game_compatibility.rs#L684) |
| 113 | `LAVA1 宠物世界/GVmaker/TC1000S/宠物世界1S.lav` | Game / game package | [Workflow passed: `pet_world_legacy_versions_reopen_the_main_game_save`](../crates/lavaxemu/tests/game_compatibility.rs#L684) |
| 114 | `LAVA1 宠物世界/GVmaker/TC800/宠物世界80.lav` | Game / game package | [Workflow passed: `pet_world_80_upgrades_a_pet_saves_and_restores_progress`](../crates/lavaxemu/tests/game_compatibility.rs#L8971) |
| 115 | `LAVA1 宠物世界/GVmaker/TC808/宠世L_808.lav` | Game / game package | [Workflow passed: `pet_world_legacy_versions_reopen_the_main_game_save`](../crates/lavaxemu/tests/game_compatibility.rs#L684) |
| 116 | `LAVA1 宠物世界/GVmaker/TC808/宠物世界88.lav` | Game / game package | [Workflow passed: `pet_world_legacy_versions_reopen_the_main_game_save`](../crates/lavaxemu/tests/game_compatibility.rs#L684) |
| 117 | `LAVA1 宠物世界/自己绘制宠物(文曲星端)/宠世导出入.lav` | Pet import/export | [Workflow passed: `pet_picture_converter_restores_and_round_trips_native_images_without_growth`](../crates/lavaxemu/tests/game_compatibility.rs#L9057), [Workflow passed: `pixel_album_edits_both_sizes_switches_frames_and_reopens_native_saves`](../crates/lavaxemu/tests/game_compatibility.rs#L9118) |
| 118 | `LAVA1 宠物世界/自己绘制宠物(文曲星端)/象素画册.lav` | Pixel editor | [Workflow passed: `pixel_album_edits_both_sizes_switches_frames_and_reopens_native_saves`](../crates/lavaxemu/tests/game_compatibility.rs#L9118) |
| 119 | `LAVA1 密室杀机/密室杀机.lav` | Game / game package | [Workflow passed: `locked_room_creates_an_account_moves_triggers_a_scene_and_restores_progress`](../crates/lavaxemu/tests/game_compatibility.rs#L9259) |
| 120 | `LAVA1 富甲天下/富甲天下.pac` | Game / game package | [Workflow passed: `rich_world_versions_move_take_turns_and_restore_native_saves`](../crates/lavaxemu/tests/game_compatibility.rs#L9395) |
| 121 | `LAVA1 富甲天下/非pac/富甲天下.lav` | Game / game package | [Workflow passed: `rich_world_versions_move_take_turns_and_restore_native_saves`](../crates/lavaxemu/tests/game_compatibility.rs#L9395) |
| 122 | `LAVA1 对战方块/For Computer（模拟器用）.lav` | Game / game package | [Workflow passed: `battle_blocks_versions_control_both_boards_clear_a_line_pause_and_finish`](../crates/lavaxemu/tests/game_compatibility.rs#L9498) |
| 123 | `LAVA1 对战方块/对战方块.lav` | Game / game package | [Workflow passed: `battle_blocks_versions_control_both_boards_clear_a_line_pause_and_finish`](../crates/lavaxemu/tests/game_compatibility.rs#L9498) |
| 124 | `LAVA1 小猴过河/小猴过河.lav` | Game / game package | [Workflow passed: `monkey_river_places_roads_retries_and_completes_a_crossing`](../crates/lavaxemu/tests/game_compatibility.rs#L9687) |
| 125 | `LAVA1 山尘围棋2.0/存档转化器_山尘围棋.lav` | Save converter | [Workflow passed: `mountain_go_converter_preserves_a_legacy_record_and_reloads_its_output`](../crates/lavaxemu/tests/game_compatibility.rs#L10037) |
| 126 | `LAVA1 山尘围棋2.0/山尘围棋v2.lav` | Game / game package | [Workflow passed: `mountain_go_captures_undoes_redoes_and_restores_a_native_record`](../crates/lavaxemu/tests/game_compatibility.rs#L9837), [Workflow passed: `mountain_go_converter_preserves_a_legacy_record_and_reloads_its_output`](../crates/lavaxemu/tests/game_compatibility.rs#L10037) |
| 127 | `LAVA1 山尘围棋v2.19/SCGO_219.lav` | Game / game package | [Workflow passed: `mountain_go_219_versions_capture_and_restore_through_the_file_list`](../crates/lavaxemu/tests/game_compatibility.rs#L9921) |
| 128 | `LAVA1 山尘围棋v2.19/SCGO_219_16gray.lav` | Game / game package | [Workflow passed: `mountain_go_219_versions_capture_and_restore_through_the_file_list`](../crates/lavaxemu/tests/game_compatibility.rs#L9921) |
| 129 | `LAVA1 山村淘金记/gold.pac` | Game / game package | [Workflow passed: `village_gold_versions_explore_and_restore_a_native_diary`](../crates/lavaxemu/tests/game_compatibility.rs#L10130) |
| 130 | `LAVA1 山村淘金记/非pac/山村淘金记.lav` | Game / game package | [Workflow passed: `village_gold_versions_explore_and_restore_a_native_diary`](../crates/lavaxemu/tests/game_compatibility.rs#L10130) |
| 131 | `LAVA1 帝国传4.1/帝国传灰/帝国传.lav` | Game / game package | [Workflow passed: `empire_versions_preserve_timed_construction_costs_and_pending_saves`](../crates/lavaxemu/tests/game_compatibility.rs#L10228) |
| 132 | `LAVA1 帝国传4.1/帝国传黑白/帝国传.lav` | Game / game package | [Workflow passed: `empire_versions_preserve_timed_construction_costs_and_pending_saves`](../crates/lavaxemu/tests/game_compatibility.rs#L10228) |
| 133 | `LAVA1 幕天席地1.1/幕天助手.lav` | Game helper | [Workflow passed: `sky_and_land_helper_reads_a_native_account_and_browses_its_catalogs`](../crates/lavaxemu/tests/game_compatibility.rs#L10344) |
| 134 | `LAVA1 幕天席地1.1/幕天席地.lav` | Game / game package | [Workflow passed: `sky_and_land_creates_a_local_save_and_moves_on_the_map`](../crates/lavaxemu/tests/game_compatibility.rs#L1146), [Workflow passed: `sky_and_land_helper_reads_a_native_account_and_browses_its_catalogs`](../crates/lavaxemu/tests/game_compatibility.rs#L10344) |
| 135 | `LAVA1 幕天席地2封印之石1.3/幕天席地2.lav` | Game / game package | [Workflow passed: `sky_and_land_two_explores_defeats_an_enemy_heals_and_restores_progress`](../crates/lavaxemu/tests/game_compatibility.rs#L10459) |
| 136 | `LAVA1 幕天席地3v0.7DEMO/幕天席地3_plus.lav` | Game / game package | [Workflow passed: `sky_and_land_three_demo_travels_fights_heals_and_restores_its_account`](../crates/lavaxemu/tests/game_compatibility.rs#L10575) |
| 137 | `LAVA1 平面魔方/平面魔方.pac` | Game / game package | [Workflow passed: `flat_cube_selects_shifts_swaps_and_returns_to_the_menu`](../crates/lavaxemu/tests/game_compatibility.rs#L1270) |
| 138 | `LAVA1 平面魔方/非pac/平面魔方.lav` | Game / game package | [Workflow passed: `flat_cube_selects_shifts_swaps_and_returns_to_the_menu`](../crates/lavaxemu/tests/game_compatibility.rs#L1270) |
| 139 | `LAVA1 幻影战机/幻影战机.lav` | Game / game package | [Workflow passed: `phantom_fighter_moves_pauses_destroys_enemies_and_reaches_stage_two`](../crates/lavaxemu/tests/game_compatibility.rs#L10712) |
| 140 | `LAVA1 幽灵屋/105(打包后的).pac` | Game / game package | [Workflow passed: `ghost_house_versions_collect_charms_clear_stage_pause_and_restore_both_save_slots`](../crates/lavaxemu/tests/game_compatibility.rs#L10827) |
| 141 | `LAVA1 幽灵屋/105.lav` | Game / game package | [Workflow passed: `ghost_house_versions_collect_charms_clear_stage_pause_and_restore_both_save_slots`](../crates/lavaxemu/tests/game_compatibility.rs#L10827) |
| 142 | `LAVA1 异域传说序章/Land.pac` | Game / game package | [Workflow passed: `foreign_land_versions_fight_follow_the_school_story_and_preserve_both_save_slots`](../crates/lavaxemu/tests/game_compatibility.rs#L10987) |
| 143 | `LAVA1 异域传说序章/非pac/Land.lav` | Game / game package | [Workflow passed: `foreign_land_versions_fight_follow_the_school_story_and_preserve_both_save_slots`](../crates/lavaxemu/tests/game_compatibility.rs#L10987) |
| 144 | `LAVA1 强力钻土机/强力钻土机(Tc800).lav` | Game / game package | [Workflow passed: `power_driller_versions_clear_training_and_stage_one_and_restore_unlocks`](../crates/lavaxemu/tests/game_compatibility.rs#L11156) |
| 145 | `LAVA1 强力钻土机/强力钻土机(其它机型).lav` | Game / game package | [Workflow passed: `power_driller_versions_clear_training_and_stage_one_and_restore_unlocks`](../crates/lavaxemu/tests/game_compatibility.rs#L11156) |
| 146 | `LAVA1 强力钻土机/强力钻土机(模拟器).lav` | Game / game package | [Workflow passed: `power_driller_versions_clear_training_and_stage_one_and_restore_unlocks`](../crates/lavaxemu/tests/game_compatibility.rs#L11156) |
| 147 | `LAVA1 忍う者/打包的/忍う者.pac` | Game / game package | [Workflow passed: `ninja_versions_keep_the_default_name_win_a_battle_and_restore_both_slots`](../crates/lavaxemu/tests/game_compatibility.rs#L11370) |
| 148 | `LAVA1 忍う者/没打包的/忍う者.lav` | Game / game package | [Workflow passed: `ninja_versions_keep_the_default_name_win_a_battle_and_restore_both_slots`](../crates/lavaxemu/tests/game_compatibility.rs#L11370) |
| 149 | `LAVA1 情怨/pac/Love.pac` | Game / game package | [Workflow passed: `love_versions_save_and_restore_both_native_slots_from_an_empty_file`](../crates/lavaxemu/tests/game_compatibility.rs#L11533) |
| 150 | `LAVA1 情怨/非pac/Love_I.lav` | Game / game package | [Workflow passed: `love_versions_save_and_restore_both_native_slots_from_an_empty_file`](../crates/lavaxemu/tests/game_compatibility.rs#L11533) |
| 151 | `LAVA1 我的世界~霸王屎/我的世界.lav` | Game / game package | [Workflow passed: `my_world_creates_a_character_and_moves_on_the_map`](../crates/lavaxemu/tests/game_compatibility.rs#L1037) |
| 152 | `LAVA1 我的世界异界版/我的世界.lav` | Game / game package | [Workflow passed: `my_world_other_realm_reaches_the_map_and_moves`](../crates/lavaxemu/tests/game_compatibility.rs#L11629) |
| 153 | `LAVA1 战略1号1.0/战略1号.pac` | Game / game package | [Workflow passed: `strategy_one_versions_show_the_battlefield_and_accept_orders`](../crates/lavaxemu/tests/game_compatibility.rs#L11666) |
| 154 | `LAVA1 战略1号1.0/非pac/战略1号.lav` | Game / game package | [Workflow passed: `strategy_one_versions_show_the_battlefield_and_accept_orders`](../crates/lavaxemu/tests/game_compatibility.rs#L11666) |
| 155 | `LAVA1 扑克大战/扑克大战.lav` | Game / game package | [Workflow passed: `poker_battle_bets_replaces_a_card_and_settles_a_hand`](../crates/lavaxemu/tests/game_compatibility.rs#L11737) |
| 156 | `LAVA1 打飞机/打飞机.lav` | Game / game package | [Workflow passed: `plane_shooter_enters_practice_and_accepts_board_controls`](../crates/lavaxemu/tests/game_compatibility.rs#L11826) |
| 157 | `LAVA1 扫雷/扫雷.lav` | Game / game package | [Workflow passed: `minesweeper_opens_cells_marks_and_moves_on_the_board`](../crates/lavaxemu/tests/game_compatibility.rs#L11908) |
| 158 | `LAVA1 扳手腕/扳手腕.lav` | Game / game package | [Workflow passed: `wrist_wrestling_selects_a_fighter_and_responds_to_gameplay_input`](../crates/lavaxemu/tests/game_compatibility.rs#L12020) |
| 159 | `LAVA1 找茬/大家来找茬.lav` | Game / game package | [Workflow passed: `spot_the_difference_finds_all_three_differences_and_advances`](../crates/lavaxemu/tests/game_compatibility.rs#L12140) |
| 160 | `LAVA1 找茬/找茬.pac` | Game / game package | [Workflow passed: `spot_the_difference_finds_all_three_differences_and_advances`](../crates/lavaxemu/tests/game_compatibility.rs#L12140) |
| 161 | `LAVA1 护卫战舰/护卫战舰.pac` | Game / game package | [Workflow passed: `battleship_advances_menus_and_battle_and_returns_after_game_over`](../crates/lavaxemu/tests/game_compatibility.rs#L12321) |
| 162 | `LAVA1 护卫战舰/非PAC/护卫战舰.lav` | Game / game package | [Workflow passed: `battleship_advances_menus_and_battle_and_returns_after_game_over`](../crates/lavaxemu/tests/game_compatibility.rs#L12321) |
| 163 | `LAVA1 拳击/boxing.lav` | Game / game package | [Workflow passed: `boxing_resolves_punches_and_finishes_a_match`](../crates/lavaxemu/tests/game_compatibility.rs#L12468) |
| 164 | `LAVA1 拿火柴/拿火柴.lav` | Game / game package | [Workflow passed: `matchstick_game_takes_turns_and_restarts_or_quits`](../crates/lavaxemu/tests/game_compatibility.rs#L12568) |
| 165 | `LAVA1 接水管/接水管.lav` | Game / game package | [Workflow passed: `pipe_game_places_pipes_counts_down_pauses_and_quits`](../crates/lavaxemu/tests/game_compatibility.rs#L12701) |
| 166 | `LAVA1 推箱达人/推箱达人.pac` | Game / game package | [Workflow passed: `sokoban_slides_boxes_pairs_glyphs_and_advances_levels`](../crates/lavaxemu/tests/game_compatibility.rs#L12847) |
| 167 | `LAVA1 推箱达人/未打包的/推箱达人.lav` | Game / game package | [Workflow passed: `sokoban_slides_boxes_pairs_glyphs_and_advances_levels`](../crates/lavaxemu/tests/game_compatibility.rs#L12847) |
| 168 | `LAVA1 搬运工/搬运工.lav` | Game / game package | [Workflow passed: `porter_pushes_crates_uses_a_bomb_and_restores_board_state`](../crates/lavaxemu/tests/game_compatibility.rs#L12994) |
| 169 | `LAVA1 撞球高手/撞球高手.lav` | Game / game package | [Workflow passed: `billiards_racks_aims_fires_and_quits_from_the_title`](../crates/lavaxemu/tests/game_compatibility.rs#L13154) |
| 170 | `LAVA1 数独1.3/sudoku.lav` | Game / game package | [Workflow passed: `sudoku_plays_a_level_fills_digits_and_switches_levels`](../crates/lavaxemu/tests/game_compatibility.rs#L13281) |
| 171 | `LAVA1 数电模拟器/已打包的文件/数电模拟器.pac` | Circuit simulator | [Workflow passed: `circuit_simulator_browses_chips_edits_waveforms_and_exits`](../crates/lavaxemu/tests/game_compatibility.rs#L13440) |
| 172 | `LAVA1 数电模拟器/未打包的文件/数电模拟器.lav` | Circuit simulator | [Workflow passed: `circuit_simulator_browses_chips_edits_waveforms_and_exits`](../crates/lavaxemu/tests/game_compatibility.rs#L13440) |
| 173 | `LAVA1 新英雄坛说/新英雄传说.pac` | Game / game package | [Workflow passed: `new_hero_reaches_the_village_and_scrolls_the_map`](../crates/lavaxemu/tests/game_compatibility.rs#L832) |
| 174 | `LAVA1 新英雄坛说/非pac/Hero.lav` | Game / game package | [Workflow passed: `new_hero_reaches_the_village_and_scrolls_the_map`](../crates/lavaxemu/tests/game_compatibility.rs#L832) |
| 175 | `LAVA1 旋转老虎机/旋转老虎机.lav` | Game / game package | [Workflow passed: `slot_machine_places_bets_spins_and_exits`](../crates/lavaxemu/tests/game_compatibility.rs#L13635) |
| 176 | `LAVA1 无底洞/无底洞.lav` | Game / game package | [Workflow passed: `bottomless_pit_selects_difficulty_descends_saves_and_replays`](../crates/lavaxemu/tests/game_compatibility.rs#L13840) |
| 177 | `LAVA1 星剑传奇/打包的[星剑传奇]/星剑传奇.pac` | Game / game package | [Workflow passed: `star_legend_package_registration_writes_a_usable_record`](../crates/lavaxemu/tests/game_compatibility.rs#L13933) |
| 178 | `LAVA1 星剑传奇/未打包的[星剑传奇]/星剑传奇.lav` | Game / game package | [Workflow passed: `star_legend_registers_a_character_and_loads_the_map`](../crates/lavaxemu/tests/game_compatibility.rs#L1214) |
| 179 | `LAVA1 星剑传奇/未打包的[星剑传奇]/星剑注册.lav` | Account registration | [Workflow passed: `star_legend_registers_a_character_and_loads_the_map`](../crates/lavaxemu/tests/game_compatibility.rs#L1214) |
| 180 | `LAVA1 是男人就下100层（模拟器可能不能运行）/是男人就下100层.lav` | Game / game package | [Workflow passed: `down_hundred_floors_steers_and_reaches_game_over`](../crates/lavaxemu/tests/game_compatibility.rs#L13985) |
| 181 | `LAVA1 是男人就撑20秒正式版/是男人就撑20秒正式版.lav` | Game / game package | [Workflow passed: `hold_twenty_seconds_moves_the_player_and_shows_the_result`](../crates/lavaxemu/tests/game_compatibility.rs#L14040) |
| 182 | `LAVA1 是男人就飞10000米/是男人就飞10000米.lav` | Game / game package | [Workflow passed: `fly_ten_thousand_meters_adjusts_speed_and_climbs`](../crates/lavaxemu/tests/game_compatibility.rs#L14109) |
| 183 | `LAVA1 暗子象棋/暗子象棋.pac` | Game / game package | [Workflow passed: `dark_chess_loads_the_board_and_responds_to_moves`](../crates/lavaxemu/tests/game_compatibility.rs#L130) |
| 184 | `LAVA1 暗子象棋/未打包/暗子象棋.lav` | Game / game package | [Workflow passed: `dark_chess_loads_the_board_and_responds_to_moves`](../crates/lavaxemu/tests/game_compatibility.rs#L130) |
| 185 | `LAVA1 月光城/Moon.pac` | Game / game package | [Workflow passed: `moon_city_reaches_the_village_and_handles_menu_items`](../crates/lavaxemu/tests/game_compatibility.rs#L14191) |
| 186 | `LAVA1 月光城/非pac/Moon.lav` | Game / game package | [Workflow passed: `moon_city_reaches_the_village_and_handles_menu_items`](../crates/lavaxemu/tests/game_compatibility.rs#L14191) |
| 187 | `LAVA1 未知/game.lav` | Game / game package | [Workflow passed: `unnamed_village_map_walks_and_scrolls`](../crates/lavaxemu/tests/game_compatibility.rs#L14277) |
| 188 | `LAVA1 极速狂飙/极速狂飙.lav` | Game / game package | [Workflow passed: `speed_racing_loads_the_track_and_responds_to_steering`](../crates/lavaxemu/tests/game_compatibility.rs#L553) |
| 189 | `LAVA1 极限斗地主/极限斗地主.lav` | Game / game package | [Workflow passed: `extreme_landlord_versions_deal_select_play_and_save`](../crates/lavaxemu/tests/game_compatibility.rs#L1424) |
| 190 | `LAVA1 极限斗地主3.2/极限斗地主.lav` | Game / game package | [Workflow passed: `extreme_landlord_versions_deal_select_play_and_save`](../crates/lavaxemu/tests/game_compatibility.rs#L1424) |
| 191 | `LAVA1 极限斗地主3.2/极限斗地主NC1020版.lav` | Game / game package | [Workflow passed: `extreme_landlord_versions_deal_select_play_and_save`](../crates/lavaxemu/tests/game_compatibility.rs#L1424) |
| 192 | `LAVA1 极限斗地主3.2/极限斗地主tc800版.lav` | Game / game package | [Workflow passed: `extreme_landlord_versions_deal_select_play_and_save`](../crates/lavaxemu/tests/game_compatibility.rs#L1424) |
| 193 | `LAVA1 梦中世界（更新6）/梦中世界.lav` | Game / game package | [Workflow passed: `dream_world_creates_an_account_loads_the_map_and_moves`](../crates/lavaxemu/tests/game_compatibility.rs#L1676) |
| 194 | `LAVA1 梦幻21点/梦幻21点.lav` | Game / game package | [Workflow passed: `dream_twenty_one_places_cards_into_the_selected_pile`](../crates/lavaxemu/tests/game_compatibility.rs#L14326) |
| 195 | `LAVA1 梦幻mario/梦幻mario.lav` | Game / game package | [Workflow passed: `dream_mario_walks_jumps_and_ignores_unbound_keys`](../crates/lavaxemu/tests/game_compatibility.rs#L14478) |
| 196 | `LAVA1 橡皮屋/橡皮屋.lav` | Game / game package | [Workflow passed: `rubber_house_menu_settings_and_level_one`](../crates/lavaxemu/tests/game_compatibility.rs#L14613) |
| 197 | `LAVA1 欢乐五子棋/欢乐五子棋.lav` | Game / game package | [Workflow passed: `happy_gomoku_menu_places_stones_undoes_restarts_and_detects_a_win`](../crates/lavaxemu/tests/game_compatibility.rs#L14799) |
| 198 | `LAVA1 武侠/武侠.lav` | Game / game package | [Workflow passed: `wuxia_title_menu_save_slots_and_story_intro`](../crates/lavaxemu/tests/game_compatibility.rs#L14973) |
| 199 | `LAVA1 水管工/PC模拟器端/水管工.lav` | Game / game package | [Workflow passed: `pipe_works_menu_screens_gameplay_and_exit`](../crates/lavaxemu/tests/game_compatibility.rs#L15082) |
| 200 | `LAVA1 水管工/文曲星端/水管工.lav` | Game / game package | [Workflow passed: `pipe_works_wqx_menu_screens_gameplay_and_exit`](../crates/lavaxemu/tests/game_compatibility.rs#L15361) |
| 201 | `LAVA1 江湖--海贼王传说/world.lav` | Game / game package | [Workflow passed: `jianghu_pirate_title_intro_field_and_episode_branches`](../crates/lavaxemu/tests/game_compatibility.rs#L15575) |
| 202 | `LAVA1 江湖-混沌初开/world.lav` | Game / game package | [Workflow passed: `jianghu_chaos_title_intro_field_and_save_branches`](../crates/lavaxemu/tests/game_compatibility.rs#L15691) |
| 203 | `LAVA1 汪汪大富翁/wangwang.lav` | Game / game package | [Workflow passed: `wangwang_tycoon_title_settings_slots_and_board_turns`](../crates/lavaxemu/tests/game_compatibility.rs#L15798) |
| 204 | `LAVA1 流星蝴蝶剑/流星蝴蝶剑.lav` | Game / game package | [Workflow passed: `meteor_butterfly_sword_title_pages_difficulty_and_arena`](../crates/lavaxemu/tests/game_compatibility.rs#L15957) |
| 205 | `LAVA1 海淀浮生记/海淀浮生记.lav` | Game / game package | [Workflow passed: `haidian_floating_life_name_menu_actions_and_save_round_trip`](../crates/lavaxemu/tests/game_compatibility.rs#L16109) |
| 206 | `LAVA1 海盗船/打包/海盗船.pac` | Game / game package | [Workflow passed: `pirate_ship_menus_help_credits_difficulty_and_sailing`](../crates/lavaxemu/tests/game_compatibility.rs#L16440) |
| 207 | `LAVA1 海盗船/未打包/海盗船.lav` | Game / game package | [Workflow passed: `pirate_ship_menus_help_credits_difficulty_and_sailing`](../crates/lavaxemu/tests/game_compatibility.rs#L16440) |
| 208 | `LAVA1 火箭车/火箭车.lav` | Game / game package | [Workflow passed: `rocket_car_keeps_the_track_visible_and_responds_to_controls`](../crates/lavaxemu/tests/game_compatibility.rs#L1316) |
| 209 | `LAVA1 灵剑射击/灵剑射击.lav` | Game / game package | [Workflow passed: `spirit_sword_menu_help_arrows_scoring_level_and_rank`](../crates/lavaxemu/tests/game_compatibility.rs#L16751) |
| 210 | `LAVA1 炸潜艇/炸潜艇v1.1 2007.7.6.lav` | Game / game package | [Workflow passed: `bomb_submarine_menu_help_level_select_scoring_pause_and_exit`](../crates/lavaxemu/tests/game_compatibility.rs#L17019) |
| 211 | `LAVA1 煞魔剑/mywar1.lav` | Game / game package | [Workflow passed: `war_sword_menu_options_random_map_turn_editor_help_and_load_errors`](../crates/lavaxemu/tests/game_compatibility.rs#L17272) |
| 212 | `LAVA1 煞魔剑A/煞魔剑A.lav` | Game / game package | [Workflow passed: `war_sword_a_splash_stub_menus_options_map_and_field_messages`](../crates/lavaxemu/tests/game_compatibility.rs#L17634) |
| 213 | `LAVA1 煞魔剑正式版/【煞魔剑】 正式版.lav` | Game / game package | Pending |
| 214 | `LAVA1 爬虫大陆编年史/爬虫大陆.lav` | Game / game package | Pending |
| 215 | `LAVA1 版炸弹人/BomB.lav` | Game / game package | Pending |
| 216 | `LAVA1 猫狗大战2/猫狗大战Ⅱ.lav` | Game / game package | Pending |
| 217 | `LAVA1 生死时速II/生死时速Ⅱ（修改版）.lav` | Game / game package | [Workflow passed: `modified_speed_racing_steers_and_returns_from_the_score_screen`](../crates/lavaxemu/tests/game_compatibility.rs#L601) |
| 218 | `LAVA1 生物钟/生物钟.lav` | Utility / education | Pending |
| 219 | `LAVA1 疯狂F2/F2.lav` | Game / game package | [Workflow passed: `crazy_f2_counts_presses_and_finishes_the_timed_round`](../crates/lavaxemu/tests/game_compatibility.rs#L499) |
| 220 | `LAVA1 疯狂弹球/疯狂弹球.pac` | Game / game package | [Workflow passed: `crazy_ball_packaged_and_loose_versions_load_the_stage_and_move_the_paddle`](../crates/lavaxemu/tests/game_compatibility.rs#L1096) |
| 221 | `LAVA1 疯狂弹球/非pac/crazyball.lav` | Game / game package | [Workflow passed: `crazy_ball_packaged_and_loose_versions_load_the_stage_and_move_the_paddle`](../crates/lavaxemu/tests/game_compatibility.rs#L1096) |
| 222 | `LAVA1 疯狂弹球/非pac/mapedit.lav` | Map editor | Pending |
| 223 | `LAVA1 疯狂跳棋/打包的文件/fktq.pac` | Game / game package | Pending |
| 224 | `LAVA1 疯狂跳棋/疯狂跳棋.lav` | Game / game package | Pending |
| 225 | `LAVA1 疾龙云飞/打包的文件/bobo_cloud.pac` | Game / game package | [Workflow passed: `cloud_versions_configure_keys_jump_and_persist_the_configuration`](../crates/lavaxemu/tests/game_compatibility.rs#L2022) |
| 226 | `LAVA1 疾龙云飞/未打包的文件/cloud_all.lav` | Game / game package | [Workflow passed: `cloud_versions_configure_keys_jump_and_persist_the_configuration`](../crates/lavaxemu/tests/game_compatibility.rs#L2022) |
| 227 | `LAVA1 白白快跑/已打包的/白白快跑.pac` | Game / game package | Pending |
| 228 | `LAVA1 白白快跑/未打包的/LAVA/白白快跑.lav` | Game / game package | Pending |
| 229 | `LAVA1 白金贪吃蛇/白金贪吃蛇.lav` | Game / game package | Pending |
| 230 | `LAVA1 百战天虫即时版/VS版/GVmaker/天虫即时vs.lav` | Game / game package | Pending |
| 231 | `LAVA1 百战天虫即时版/天虫即时.lav` | Game / game package | Pending |
| 232 | `LAVA1 百战天虫即时版/资源管理器.lav` | Utility / education | [Workflow passed: `resource_manager_selects_files_and_cancels_the_dialog`](../crates/lavaxemu/tests/game_compatibility.rs#L652) |
| 233 | `LAVA1 礼物大作战/YoGamesPC.lav` | Game / game package | Pending |
| 234 | `LAVA1 礼物大作战/YoGamesWQX.lav` | Game / game package | Pending |
| 235 | `LAVA1 神州/神州.pac` | Game / game package | [Workflow passed: `magic_packaged_and_loose_versions_load_resources_and_move_on_the_map`](../crates/lavaxemu/tests/game_compatibility.rs#L720) |
| 236 | `LAVA1 神州/非pac/Magic.lav` | Game / game package | [Workflow passed: `magic_packaged_and_loose_versions_load_resources_and_move_on_the_map`](../crates/lavaxemu/tests/game_compatibility.rs#L720) |
| 237 | `LAVA1 种金币/种金币.lav` | Game / game package | [Workflow passed: `planting_gold_moves_a_coin_and_restarts_the_level`](../crates/lavaxemu/tests/game_compatibility.rs#L1121) |
| 238 | `LAVA1 秘境还生/秘境还生.lav` | Game / game package | Pending |
| 239 | `LAVA1 空当接龙/空当接龙.lav` | Game / game package | Pending |
| 240 | `LAVA1 空间战争/SpaceWar.lav` | Game / game package | Pending |
| 241 | `LAVA1 类似十全十美/allout.lav` | Game / game package | Pending |
| 242 | `LAVA1 精灵岛5-勇者之旅/精灵岛5.lav` | Game / game package | Pending |
| 243 | `LAVA1 精灵岛5-勇者之旅/精灵岛5.pac` | Game / game package | Pending |
| 244 | `LAVA1 绝对下落/分步安装/绝对下落.lav` | Game / game package | Pending |
| 245 | `LAVA1 美少女梦工厂2黑白版/PM2.lav` | Game / game package | [Workflow passed: `princess_maker_2_shows_stats_executes_a_plan_and_reloads_its_save`](../crates/lavaxemu/tests/game_compatibility.rs#L1616) |
| 246 | `LAVA1 翼剑天殇/翼剑天殇.pac` | Game / game package | Pending |
| 247 | `LAVA1 翼剑天殇/非pac/Wing.lav` | Game / game package | Pending |
| 248 | `LAVA1 花之精灵2.0/flower2.0.pac` | Game / game package | Pending |
| 249 | `LAVA1 花之精灵2.0/非pac/花之精灵.lav` | Game / game package | Pending |
| 250 | `LAVA1 英雄无敌2完美版/《英雄无敌2》完美版.lav` | Game / game package | Pending |
| 251 | `LAVA1 英雄无敌对战版/英雄无敌v1.0.lav` | Game / game package | Pending |
| 252 | `LAVA1 萝卜大作战修改版/萝卜大作战.pac` | Game / game package | Pending |
| 253 | `LAVA1 萝卜大作战修改版/非pac/MapEditor.lav` | Map editor | Pending |
| 254 | `LAVA1 萝卜大作战修改版/非pac/萝卜大作战.lav` | Game / game package | Pending |
| 255 | `LAVA1 蛙蛙大富翁/蛙蛙大富翁打包.pac` | Game / game package | Pending |
| 256 | `LAVA1 蛙蛙大富翁/非pac/蛙蛙大富翁.lav` | Game / game package | Pending |
| 257 | `LAVA1 蛙蛙马戏团 1.2/蛙蛙马戏团.lav` | Game / game package | Pending |
| 258 | `LAVA1 蜀山群侠传/蜀山注册.lav` | Account registration | [Workflow passed: `shushan_registers_a_character_and_enters_the_game`](../crates/lavaxemu/tests/game_compatibility.rs#L976) |
| 259 | `LAVA1 蜀山群侠传/蜀山群侠传.lav` | Game / game package | [Workflow passed: `shushan_registers_a_character_and_enters_the_game`](../crates/lavaxemu/tests/game_compatibility.rs#L976) |
| 260 | `LAVA1 蟑螂历险记/测试版/蟑螂历险记 V1.0.lav` | Game / game package | Pending |
| 261 | `LAVA1 蟑螂历险记/白金版/蟑螂历险记V3.0.lav` | Game / game package | Pending |
| 262 | `LAVA1 蟑螂历险记/精简版/蟑螂历险记V2.0.lav` | Game / game package | Pending |
| 263 | `LAVA1 蟑螂历险记/终结完美版/蟑螂历险记V3.2.lav` | Game / game package | Pending |
| 264 | `LAVA1 蟑螂历险记3.2/蟑螂历险记V3.2.lav` | Game / game package | Pending |
| 265 | `LAVA1 衰人大战（很小的游戏）/衰.lav` | Game / game package | Pending |
| 266 | `LAVA1 谷虫迷宫大冒险/打包/谷虫大冒险.pac` | Game / game package | Pending |
| 267 | `LAVA1 谷虫迷宫大冒险/未打包/gcdmx.lav` | Game / game package | Pending |
| 268 | `LAVA1 贪吃小精灵/Clear.lav` | Game / game package | Pending |
| 269 | `LAVA1 贪吃蛇鲍勃/贪吃蛇鲍勃.lav` | Game / game package | Pending |
| 270 | `LAVA1 赤色要塞/非pac/Jackal.lav` | Game / game package | Pending |
| 271 | `LAVA1 超级机器人大战/Download/SRW.pac` | Game / game package | Pending |
| 272 | `LAVA1 超级机器人大战/file/SRW.lav` | Game / game package | Pending |
| 273 | `LAVA1 超级机器人大战/Setup/SRW.lav` | Installer | [Workflow passed: `super_robot_wars_installs_resources_and_accepts_map_input`](../crates/lavaxemu/tests/game_compatibility.rs#L751) |
| 274 | `LAVA1 超级机器人大战/Setup/SRWDate.lav` | Installer | [Workflow passed: `super_robot_wars_installs_resources_and_accepts_map_input`](../crates/lavaxemu/tests/game_compatibility.rs#L751) |
| 275 | `LAVA1 超级舞者/超级舞者.lav` | Game / game package | Pending |
| 276 | `LAVA1 连连看/专供LAVA2机/连连看.lav` | Game / game package | Pending |
| 277 | `LAVA1 连连看/打包的文件/连连看.pac` | Game / game package | Pending |
| 278 | `LAVA1 连连看/未打包的文件/连连看.lav` | Game / game package | Pending |
| 279 | `LAVA1 连连看/过关动画/过关画面.lav` | Ending animation | Pending |
| 280 | `LAVA1 连连看/过关动画/通关动画.lav` | Ending animation | Pending |
| 281 | `LAVA1 迷宫之旅/maze2.lav` | Game / game package | Pending |
| 282 | `LAVA1 逆转裁判/逆转裁判.lav` | Game / game package | Pending |
| 283 | `LAVA1 钻石方块1.1/钻石方块灰.lav` | Game / game package | Pending |
| 284 | `LAVA1 钻石方块1.1/钻石方块黑.lav` | Game / game package | Pending |
| 285 | `LAVA1 钻石棋/钻石棋.lav` | Game / game package | Pending |
| 286 | `LAVA1 雪人传奇/雪人传奇.pac` | Game / game package | Pending |
| 287 | `LAVA1 雪人传奇/非pac/雪人传奇.lav` | Game / game package | Pending |
| 288 | `LAVA1 零度恐慌/零度恐惧.lav` | Game / game package | Pending |
| 289 | `LAVA1 青蛙冒险/青蛙冒险.lav` | Game / game package | Pending |
| 290 | `LAVA1 风斗/wind.lav` | Game / game package | Pending |
| 291 | `LAVA1 飞翔理科/元素周期.lav` | Utility / education | Pending |
| 292 | `LAVA1 飞翔理科/飞翔理科1020.lav` | Game / game package | Pending |
| 293 | `LAVA1 飞行棋/非pac/飞行棋.lav` | Game / game package | Pending |
| 294 | `LAVA1 飞行棋/飞行棋 v1.1 For WQX Lava All Fixed.pac` | Game / game package | Pending |
| 295 | `LAVA1 魔塔/魔塔.lav` | Game / game package | Pending |
| 296 | `LAVA1 魔塔整合版/魔塔整合版.lav` | Game / game package | Pending |
| 297 | `LAVA1 魔法拼图/LAVA/PTV1.0.lav` | Game / game package | Pending |
| 298 | `LAVA1 魔法拼图/Package/PTV1.0.pac` | Game / game package | Pending |
| 299 | `LAVA1 魔法方块/魔法方块.lav` | Game / game package | Pending |
| 300 | `LAVA1 魔界传说/魔界传说程序/魔界传说.lav` | Game / game package | Pending |
| 301 | `LAVA1 魔界传说/魔界传说程序/魔界传说安装包.pac` | Game / game package | Pending |
| 302 | `LAVA1 黑白五子连珠/已打包文件/五子联珠黑.pac` | Game / game package | Pending |
| 303 | `LAVA1 黑白五子连珠/未打包文件/五子连珠.lav` | Game / game package | Pending |
| 304 | `LAVA1 黑白牛牛/黑白牛牛.lav` | Game / game package | Pending |
| 305 | `LAVA1 龙之传说/非pac/longX.lav` | Game / game package | [Workflow passed: `dragon_legend_selects_classes_moves_and_reloads_a_local_save`](../crates/lavaxemu/tests/game_compatibility.rs#L1761) |
| 306 | `LAVA1 龙之吻体验版/l2体验版.lav` | Game / game package | [Workflow passed: `dragon_kiss_creates_a_chinese_name_moves_and_loads_party_progress`](../crates/lavaxemu/tests/game_compatibility.rs#L1866) |
| 307 | `LAVA1 龙之崛起[十一特别版]/LAVA/G.龙之崛起.lav` | Game / game package | Pending |
| 308 | `LAVA1 龙之崛起[十一特别版]/LAVA/L.龙之崛起.lav` | Game / game package | Pending |
| 309 | `LAVA1 龙之崛起[十一特别版]/Package/龙之崛起.pac` | Game / game package | Pending |
| 310 | `LAVA2 仙境传说/仙境.pac` | Game / game package | Pending |
| 311 | `LAVA2 仙境传说/非pac/仙境.lav` | Game / game package | Pending |
| 312 | `LAVA2 冒险岛IV/灰度版/打包的文件/冒险岛IV.pac` | Game / game package | [Workflow passed: `adventure_island_iv_versions_clear_a_stage_and_restore_custom_controls`](../crates/lavaxemu/tests/game_compatibility.rs#L5108) |
| 313 | `LAVA2 冒险岛IV/灰度版/未打包的文件/冒险岛IV.lav` | Game / game package | [Workflow passed: `adventure_island_iv_versions_clear_a_stage_and_restore_custom_controls`](../crates/lavaxemu/tests/game_compatibility.rs#L5108) |
| 314 | `LAVA2 心跳回忆/心跳回忆.lav` | Game / game package | [Workflow passed: `tokimeki_reaches_the_room_and_selects_an_activity`](../crates/lavaxemu/tests/game_compatibility.rs#L854) |
| 315 | `LAVA2 打地鼠/LAVA2/Hit.lav` | Game / game package | [Workflow passed: `hit_loads_the_logo_runs_and_pauses_with_the_keyboard`](../crates/lavaxemu/tests/game_compatibility.rs#L378) |
| 316 | `LAVA2 打地鼠/Package/HitV1.0.pac` | Game / game package | Pending |
| 317 | `LAVA2 挑战推箱子1.31/挑战推箱子.lav` | Game / game package | Pending |
| 318 | `LAVA2 搬运工/搬运工.lav` | Game / game package | Pending |
| 319 | `LAVA2 易码事件簿-寻找CEO/非pac/EMStory.lav` | Game / game package | Pending |
| 320 | `LAVA2 玩转变色龙/玩转变色龙.lav` | Game / game package | Pending |
| 321 | `LAVA2 美少女梦工场/非pac/PM4.lav` | Game / game package | [Workflow passed: `princess_maker_4_reaches_gameplay_and_opens_the_status_menu`](../crates/lavaxemu/tests/game_compatibility.rs#L317) |
| 322 | `LAVA2 魔法纪元/魔法纪元.lav` | Game / game package | [Workflow passed: `magic_era_accepts_the_device_identifier_and_enters_gameplay`](../crates/lavaxemu/tests/game_compatibility.rs#L798) |
| 323 | `LAVA2 麻将智多星/BoBo_MJ.lav` | Game / game package | Pending |
