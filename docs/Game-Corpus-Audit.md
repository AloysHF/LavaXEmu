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

- Base audit: 2026-10-29; incremental entry checks: 2026-10-30. Full rerun of 44 workflows at `641903a` on 2026-10-30; subsequent entry checks use the same emulator behavior.
- 49 dedicated release workflows passed in this audit.
- 83 of 324 entry points were loaded by those successful workflows.
- 241 entry points still need dedicated workflow verification.
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
| 000 | `LAVA1 2 口水王1.2/口水王2 1.2灰度/口水王2.lav` | Game / game package | [Workflow passed: `spitting_king_versions_move_shoot_adjust_power_and_confirm_exit`](../crates/lavaxemu/tests/game_compatibility.rs#L2052) |
| 001 | `LAVA1 2 口水王1.2/口水王2 1.2黑白/口水王2.lav` | Game / game package | [Workflow passed: `spitting_king_versions_move_shoot_adjust_power_and_confirm_exit`](../crates/lavaxemu/tests/game_compatibility.rs#L2052) |
| 002 | `LAVA1 3D迷宫/3D迷宫.lav` | Game / game package | [Workflow passed: `maze_3d_walks_turns_and_switches_the_satellite_map`](../crates/lavaxemu/tests/game_compatibility.rs#L1907) |
| 003 | `LAVA1 Cloud/打包的文件/bobo_cloud.pac` | Game / game package | [Workflow passed: `cloud_versions_configure_keys_jump_and_persist_the_configuration`](../crates/lavaxemu/tests/game_compatibility.rs#L1993) |
| 004 | `LAVA1 Cloud/未打包的文件/cloud_all.lav` | Game / game package | [Workflow passed: `cloud_versions_configure_keys_jump_and_persist_the_configuration`](../crates/lavaxemu/tests/game_compatibility.rs#L1993) |
| 005 | `LAVA1 Crazy_ball/crazyball.lav` | Game / game package | Pending |
| 006 | `LAVA1 Crazy_ball/mapedit.lav` | Map editor | Pending |
| 007 | `LAVA1 CS 1.3/Cs(lav)1_3.pac` | Game / game package | [Workflow passed: `counter_strike_13_loads_resources_and_enters_a_mission`](../crates/lavaxemu/tests/game_compatibility.rs#L389) |
| 008 | `LAVA1 CS 1.3/非pac/Cs(lav)1_3.lav` | Game / game package | [Workflow passed: `counter_strike_13_loads_resources_and_enters_a_mission`](../crates/lavaxemu/tests/game_compatibility.rs#L389) |
| 009 | `LAVA1 CS蓝色行动/CS蓝色行动.lav` | Game / game package | Pending |
| 010 | `LAVA1 Dance_all/打包的文件/BoBo_dance.pac` | Game / game package | [Workflow passed: `bobo_dance_versions_score_the_direction_sequence_and_open_the_help`](../crates/lavaxemu/tests/game_compatibility.rs#L2120) |
| 011 | `LAVA1 Dance_all/未打包的文件/Bobo_Dance.lav` | Game / game package | [Workflow passed: `bobo_dance_versions_score_the_direction_sequence_and_open_the_help`](../crates/lavaxemu/tests/game_compatibility.rs#L2120) |
| 012 | `LAVA1 EM星纪传说—格斗/EM星纪传说—格斗.lav` | Game / game package | [Workflow passed: `em_fighter_moves_attacks_and_finishes_the_bout`](../crates/lavaxemu/tests/game_compatibility.rs#L2269) |
| 013 | `LAVA1 IQ大作战/IQ大作战.lav` | Game / game package | [Workflow passed: `iq_battle_moves_removes_groups_uses_a_bomb_and_returns_to_the_menu`](../crates/lavaxemu/tests/game_compatibility.rs#L2313) |
| 014 | `LAVA1 Just_Fly/Just_Fly.pac` | Game / game package | [Workflow passed: `just_fly_versions_respond_in_both_modes_and_replay_a_saved_record`](../crates/lavaxemu/tests/game_compatibility.rs#L2187) |
| 015 | `LAVA1 Just_Fly/非pac/Just_Fly.lav` | Game / game package | [Workflow passed: `just_fly_versions_respond_in_both_modes_and_replay_a_saved_record`](../crates/lavaxemu/tests/game_compatibility.rs#L2187) |
| 016 | `LAVA1 LAVA2 江湖2_demo/world2.lav` | Game / game package | Pending |
| 017 | `LAVA1 LAVA2 江湖2_demo/world2_16.lav` | Game / game package | Pending |
| 018 | `LAVA1 Lazy YY牛奶大战/Ver1.2/打包的文件/LazyYY牛奶大战.pac` | Game / game package | [Workflow passed: `lazy_yy_versions_feed_pause_and_return_to_the_menu`](../crates/lavaxemu/tests/game_compatibility.rs#L1328) |
| 019 | `LAVA1 Lazy YY牛奶大战/Ver1.2/未打包的文件/LazyYY.lav` | Game / game package | [Workflow passed: `lazy_yy_versions_feed_pause_and_return_to_the_menu`](../crates/lavaxemu/tests/game_compatibility.rs#L1328) |
| 020 | `LAVA1 Lazy YY牛奶大战/Ver1.5 For TC800 Only/Lazy YY牛奶大战.pac` | Game / game package | [Workflow passed: `lazy_yy_versions_feed_pause_and_return_to_the_menu`](../crates/lavaxemu/tests/game_compatibility.rs#L1328) |
| 021 | `LAVA1 lost fantasy/LF_demo(PC).lav` | Game / game package | [Workflow passed: `lost_fantasy_device_and_pc_versions_draw_and_move`](../crates/lavaxemu/tests/game_compatibility.rs#L494) |
| 022 | `LAVA1 lost fantasy/LF_demo.lav` | Game / game package | [Workflow passed: `lost_fantasy_device_and_pc_versions_draw_and_move`](../crates/lavaxemu/tests/game_compatibility.rs#L494) |
| 023 | `LAVA1 PACMAN/PACMAN.pac` | Game / game package | [Workflow passed: `pacman_versions_select_a_stage_move_and_finish_a_round`](../crates/lavaxemu/tests/game_compatibility.rs#L1954) |
| 024 | `LAVA1 PACMAN/非pac/PACMAN.lav` | Game / game package | [Workflow passed: `pacman_versions_select_a_stage_move_and_finish_a_round`](../crates/lavaxemu/tests/game_compatibility.rs#L1954) |
| 025 | `LAVA1 puke21/扑克21.lav` | Game / game package | Pending |
| 026 | `LAVA1 Puzzled Sheep/Psheep.lav` | Game / game package | [Workflow passed: `puzzled_sheep_loads_levels_moves_the_cursor_and_runs_the_simulation`](../crates/lavaxemu/tests/game_compatibility.rs#L319) |
| 027 | `LAVA1 QUEERBOX/QUEERBOX.lav` | Game / game package | [Workflow passed: `queerbox_moves_a_top_block_restarts_and_returns_to_the_title`](../crates/lavaxemu/tests/game_compatibility.rs#L2375) |
| 028 | `LAVA1 Q版Dancer/Q版Dancer.lav` | Game / game package | [Workflow passed: `q_dancer_enters_the_test_answers_a_sequence_and_scores`](../crates/lavaxemu/tests/game_compatibility.rs#L2421) |
| 029 | `LAVA1 RC赛车/RC赛车.lav` | Game / game package | [Workflow passed: `rc_racing_accelerates_steers_pauses_and_returns_to_the_menu`](../crates/lavaxemu/tests/game_compatibility.rs#L2493) |
| 030 | `LAVA1 Rush_Out/Rush_Out_For_Lava(8K).lav` | Game / game package | [Workflow passed: `rush_out_variants_fly_drop_bombs_pause_and_return_to_the_menu`](../crates/lavaxemu/tests/game_compatibility.rs#L2559) |
| 031 | `LAVA1 Rush_Out/Rush_Out_For_Lava1(16k).lav` | Game / game package | [Workflow passed: `rush_out_variants_fly_drop_bombs_pause_and_return_to_the_menu`](../crates/lavaxemu/tests/game_compatibility.rs#L2559) |
| 032 | `LAVA1 Rush_Out/Rush_Out_For_Lava2(16k).lav` | Game / game package | [Workflow passed: `rush_out_variants_fly_drop_bombs_pause_and_return_to_the_menu`](../crates/lavaxemu/tests/game_compatibility.rs#L2559) |
| 033 | `LAVA1 SnowFight(打雪仗)/SnowFight.lav` | Game / game package | [Workflow passed: `snow_fight_opens_help_moves_throws_and_finishes_a_round`](../crates/lavaxemu/tests/game_compatibility.rs#L2699) |
| 034 | `LAVA1 SpaceWars/SWR无延时.lav` | Game / game package | [Workflow passed: `space_wars_variants_move_fire_and_exit_the_mission`](../crates/lavaxemu/tests/game_compatibility.rs#L2782) |
| 035 | `LAVA1 SpaceWars/SWR有延时.lav` | Game / game package | [Workflow passed: `space_wars_variants_move_fire_and_exit_the_mission`](../crates/lavaxemu/tests/game_compatibility.rs#L2782) |
| 036 | `LAVA1 TRAVEL/travel.lav` | Game / game package | [Workflow passed: `travel_variants_explore_fight_collect_the_tools_and_finish_the_demo`](../crates/lavaxemu/tests/game_compatibility.rs#L2860) |
| 037 | `LAVA1 TRAVEL/打包travel.pac` | Game / game package | [Workflow passed: `travel_variants_explore_fight_collect_the_tools_and_finish_the_demo`](../crates/lavaxemu/tests/game_compatibility.rs#L2860) |
| 038 | `LAVA1 WarCraft/WarCraft.pac` | Game / game package | Pending |
| 039 | `LAVA1 WarCraft/非pac/WarCraft.lav` | Game / game package | Pending |
| 040 | `LAVA1 七彩泡泡/七彩泡泡.lav` | Game / game package | Pending |
| 041 | `LAVA1 七彩连珠/七彩连珠.lav` | Game / game package | Pending |
| 042 | `LAVA1 七王五二三/七王五二三.lav` | Game / game package | Pending |
| 043 | `LAVA1 三国志/三国志.pac` | Game / game package | Pending |
| 044 | `LAVA1 中国灯谜/中国灯谜.lav` | Game / game package | Pending |
| 045 | `LAVA1 中国烟火 1.1/中国烟火.lav` | Game / game package | Pending |
| 046 | `LAVA1 中国象棋/象棋90.lav` | Game / game package | Pending |
| 047 | `LAVA1 中国麻将/中国麻将.lav` | Game / game package | Pending |
| 048 | `LAVA1 中学传奇/打包的/SCH_RPGPACK.pac` | Game / game package | Pending |
| 049 | `LAVA1 中学传奇/未打包的/中学传奇.lav` | Game / game package | Pending |
| 050 | `LAVA1 五子连珠/WZLZ_16灰度.lav` | Game / game package | Pending |
| 051 | `LAVA1 五子连珠/WZLZ_黑白.lav` | Game / game package | Pending |
| 052 | `LAVA1 五虎/Fivet.lav` | Game / game package | Pending |
| 053 | `LAVA1 五骰Yahtzee/Yahtzee.lav` | Game / game package | Pending |
| 054 | `LAVA1 仙剑后传/Pala.lav` | Game / game package | Pending |
| 055 | `LAVA1 仙剑后传/Pala_.lav` | Game / game package | Pending |
| 056 | `LAVA1 仙剑后传/Pala_TC800.lav` | Game / game package | Pending |
| 057 | `LAVA1 仙境奇缘/仙境奇缘/打包后的文件/仙境奇缘.pac` | Game / game package | Pending |
| 058 | `LAVA1 仙境奇缘/仙境奇缘/未打包文件/仙境奇缘.lav` | Game / game package | Pending |
| 059 | `LAVA1 任意拼图/任意拼图.lav` | Game / game package | Pending |
| 060 | `LAVA1 俄罗斯扑克/俄罗斯扑克.lav` | Game / game package | Pending |
| 061 | `LAVA1 俄罗斯方块1.3/俄罗斯方块灰.lav` | Game / game package | Pending |
| 062 | `LAVA1 俄罗斯方块1.3/俄罗斯方块黑.lav` | Game / game package | Pending |
| 063 | `LAVA1 假日神秘岛/假日神秘岛 abcdef.lav` | Game / game package | [Workflow passed: `holiday_island_finishes_the_intro_talks_and_moves_on_the_map`](../crates/lavaxemu/tests/game_compatibility.rs#L1035) |
| 064 | `LAVA1 先赌为快/先赌★为快.lav` | Game / game package | [Workflow passed: `gamble_creates_a_local_account_and_accepts_bets`](../crates/lavaxemu/tests/game_compatibility.rs#L868) |
| 065 | `LAVA1 冒险岛IV/黑白版/打包的文件/冒险岛IV.pac` | Game / game package | Pending |
| 066 | `LAVA1 冒险岛IV/黑白版/未打包的文件/冒险岛IV.lav` | Game / game package | Pending |
| 067 | `LAVA1 冠军足球经理05-06赛季0.7/CFM.lav` | Game / game package | [Workflow passed: `champion_football_manager_creates_a_coach_and_finishes_a_match`](../crates/lavaxemu/tests/game_compatibility.rs#L1487) |
| 068 | `LAVA1 决战娜美克/决战娜美克.lav` | Game / game package | Pending |
| 069 | `LAVA1 凌空抽射1.5/凌空抽射v1.5.lav` | Game / game package | Pending |
| 070 | `LAVA1 凤凰战斗机/Fighter.lav` | Game / game package | Pending |
| 071 | `LAVA1 动力滑雪/滑雪.lav` | Game / game package | Pending |
| 072 | `LAVA1 勇者传说/勇者传说.lav` | Game / game package | Pending |
| 073 | `LAVA1 勇者斗恶龙/勇者斗恶龙.lav` | Game / game package | Pending |
| 074 | `LAVA1 勇者斗恶龙特别版/勇者斗恶龙.lav` | Game / game package | Pending |
| 075 | `LAVA1 勇闯地狱/勇闯地狱.lav` | Game / game package | Pending |
| 076 | `LAVA1 博士失踪记/博士失踪记.lav` | Game / game package | Pending |
| 077 | `LAVA1 口袋妖怪PK场/pmpk.pac` | Game / game package | Pending |
| 078 | `LAVA1 口袋妖怪（未完成）/pokemon.lav` | Game / game package | Pending |
| 079 | `LAVA1 吃月饼/吃月饼.lav` | Game / game package | Pending |
| 080 | `LAVA1 吞食天地0.29/tunshi_V0.29.lav` | Game / game package | Pending |
| 081 | `LAVA1 听剑/听剑.pac` | Game / game package | Pending |
| 082 | `LAVA1 听剑/非pac/Sword.lav` | Game / game package | Pending |
| 083 | `LAVA1 四人麻将/MahJong.lav` | Game / game package | Pending |
| 084 | `LAVA1 困虫斗 1.1/打包/kcd.pac` | Game / game package | Pending |
| 085 | `LAVA1 困虫斗 1.1/未打包/困虫斗.lav` | Game / game package | Pending |
| 086 | `LAVA1 圈地运动/圈地运动.lav` | Game / game package | Pending |
| 087 | `LAVA1 圣金/Shengjin2.lav` | Game / game package | Pending |
| 088 | `LAVA1 坦克2004/坦克2004.pac` | Game / game package | Pending |
| 089 | `LAVA1 坦克2004/非PAC/坦克2004.lav` | Game / game package | Pending |
| 090 | `LAVA1 塞尔达小游戏/文曲星版Zelda.lav` | Game / game package | Pending |
| 091 | `LAVA1 大战略/大战略.pac` | Game / game package | Pending |
| 092 | `LAVA1 大战略/非pac/dzl.lav` | Game / game package | Pending |
| 093 | `LAVA1 天空格斗艺术正式版/HFAcs.lav` | Game / game package | Pending |
| 094 | `LAVA1 天鹰战机·血战/Eagle.lav` | Game / game package | Pending |
| 095 | `LAVA1 天鹰战机·血战/VS版/NC2600.lav` | Game / game package | Pending |
| 096 | `LAVA1 天鹰战机·血战/VS版/NC3000.lav` | Game / game package | Pending |
| 097 | `LAVA1 天鹰战机·血战/VS版/TC1000.lav` | Game / game package | Pending |
| 098 | `LAVA1 天鹰战机·血战/VS版/TC1000S.lav` | Game / game package | Pending |
| 099 | `LAVA1 天鹰战机·血战/VS版/TC808.lav` | Game / game package | Pending |
| 100 | `LAVA1 天龙八步/Liuye.lav` | Game / game package | Pending |
| 101 | `LAVA1 太空寻宝/太空寻宝.lav` | Game / game package | Pending |
| 102 | `LAVA1 太空战舰/太空战舰.lav` | Game / game package | Pending |
| 103 | `LAVA1 奇门九星诀房屋风水勘测/9stars.lav` | Utility / education | Pending |
| 104 | `LAVA1 宇宙大战/宇宙大战.lav` | Game / game package | Pending |
| 105 | `LAVA1 宠物世界/GVmaker/NC2600/宠世L_2K6.lav` | Game / game package | [Workflow passed: `pet_world_legacy_versions_reopen_the_main_game_save`](../crates/lavaxemu/tests/game_compatibility.rs#L655) |
| 106 | `LAVA1 宠物世界/GVmaker/NC2600/宠物世界26.lav` | Game / game package | [Workflow passed: `pet_world_legacy_versions_reopen_the_main_game_save`](../crates/lavaxemu/tests/game_compatibility.rs#L655) |
| 107 | `LAVA1 宠物世界/GVmaker/NC3000/宠世L_3K.lav` | Game / game package | [Workflow passed: `pet_world_legacy_versions_reopen_the_main_game_save`](../crates/lavaxemu/tests/game_compatibility.rs#L655) |
| 108 | `LAVA1 宠物世界/GVmaker/NC3000/宠物世界3H.lav` | Game / game package | [Workflow passed: `pet_world_3h_loads_nested_assets_and_opens_pet_management`](../crates/lavaxemu/tests/game_compatibility.rs#L261) |
| 109 | `LAVA1 宠物世界/GVmaker/NC3000/宠物世界3K.lav` | Game / game package | [Workflow passed: `pet_world_legacy_versions_reopen_the_main_game_save`](../crates/lavaxemu/tests/game_compatibility.rs#L655) |
| 110 | `LAVA1 宠物世界/GVmaker/TC1000/宠世L_1K.lav` | Game / game package | [Workflow passed: `pet_world_legacy_versions_reopen_the_main_game_save`](../crates/lavaxemu/tests/game_compatibility.rs#L655) |
| 111 | `LAVA1 宠物世界/GVmaker/TC1000/宠物世界1K.lav` | Game / game package | [Workflow passed: `pet_world_legacy_versions_reopen_the_main_game_save`](../crates/lavaxemu/tests/game_compatibility.rs#L655) |
| 112 | `LAVA1 宠物世界/GVmaker/TC1000S/宠世L_1KS.lav` | Game / game package | [Workflow passed: `pet_world_legacy_versions_reopen_the_main_game_save`](../crates/lavaxemu/tests/game_compatibility.rs#L655) |
| 113 | `LAVA1 宠物世界/GVmaker/TC1000S/宠物世界1S.lav` | Game / game package | [Workflow passed: `pet_world_legacy_versions_reopen_the_main_game_save`](../crates/lavaxemu/tests/game_compatibility.rs#L655) |
| 114 | `LAVA1 宠物世界/GVmaker/TC800/宠物世界80.lav` | Game / game package | Pending |
| 115 | `LAVA1 宠物世界/GVmaker/TC808/宠世L_808.lav` | Game / game package | [Workflow passed: `pet_world_legacy_versions_reopen_the_main_game_save`](../crates/lavaxemu/tests/game_compatibility.rs#L655) |
| 116 | `LAVA1 宠物世界/GVmaker/TC808/宠物世界88.lav` | Game / game package | [Workflow passed: `pet_world_legacy_versions_reopen_the_main_game_save`](../crates/lavaxemu/tests/game_compatibility.rs#L655) |
| 117 | `LAVA1 宠物世界/自己绘制宠物(文曲星端)/宠世导出入.lav` | Pet import/export | Pending |
| 118 | `LAVA1 宠物世界/自己绘制宠物(文曲星端)/象素画册.lav` | Pixel editor | Pending |
| 119 | `LAVA1 密室杀机/密室杀机.lav` | Game / game package | Pending |
| 120 | `LAVA1 富甲天下/富甲天下.pac` | Game / game package | Pending |
| 121 | `LAVA1 富甲天下/非pac/富甲天下.lav` | Game / game package | Pending |
| 122 | `LAVA1 对战方块/For Computer（模拟器用）.lav` | Game / game package | Pending |
| 123 | `LAVA1 对战方块/对战方块.lav` | Game / game package | Pending |
| 124 | `LAVA1 小猴过河/小猴过河.lav` | Game / game package | Pending |
| 125 | `LAVA1 山尘围棋2.0/存档转化器_山尘围棋.lav` | Save converter | Pending |
| 126 | `LAVA1 山尘围棋2.0/山尘围棋v2.lav` | Game / game package | Pending |
| 127 | `LAVA1 山尘围棋v2.19/SCGO_219.lav` | Game / game package | Pending |
| 128 | `LAVA1 山尘围棋v2.19/SCGO_219_16gray.lav` | Game / game package | Pending |
| 129 | `LAVA1 山村淘金记/gold.pac` | Game / game package | Pending |
| 130 | `LAVA1 山村淘金记/非pac/山村淘金记.lav` | Game / game package | Pending |
| 131 | `LAVA1 帝国传4.1/帝国传灰/帝国传.lav` | Game / game package | Pending |
| 132 | `LAVA1 帝国传4.1/帝国传黑白/帝国传.lav` | Game / game package | Pending |
| 133 | `LAVA1 幕天席地1.1/幕天助手.lav` | Game helper | Pending |
| 134 | `LAVA1 幕天席地1.1/幕天席地.lav` | Game / game package | [Workflow passed: `sky_and_land_creates_a_local_save_and_moves_on_the_map`](../crates/lavaxemu/tests/game_compatibility.rs#L1117) |
| 135 | `LAVA1 幕天席地2封印之石1.3/幕天席地2.lav` | Game / game package | Pending |
| 136 | `LAVA1 幕天席地3v0.7DEMO/幕天席地3_plus.lav` | Game / game package | Pending |
| 137 | `LAVA1 平面魔方/平面魔方.pac` | Game / game package | [Workflow passed: `flat_cube_selects_shifts_swaps_and_returns_to_the_menu`](../crates/lavaxemu/tests/game_compatibility.rs#L1241) |
| 138 | `LAVA1 平面魔方/非pac/平面魔方.lav` | Game / game package | [Workflow passed: `flat_cube_selects_shifts_swaps_and_returns_to_the_menu`](../crates/lavaxemu/tests/game_compatibility.rs#L1241) |
| 139 | `LAVA1 幻影战机/幻影战机.lav` | Game / game package | Pending |
| 140 | `LAVA1 幽灵屋/105(打包后的).pac` | Game / game package | Pending |
| 141 | `LAVA1 幽灵屋/105.lav` | Game / game package | Pending |
| 142 | `LAVA1 异域传说序章/Land.pac` | Game / game package | Pending |
| 143 | `LAVA1 异域传说序章/非pac/Land.lav` | Game / game package | Pending |
| 144 | `LAVA1 强力钻土机/强力钻土机(Tc800).lav` | Game / game package | Pending |
| 145 | `LAVA1 强力钻土机/强力钻土机(其它机型).lav` | Game / game package | Pending |
| 146 | `LAVA1 强力钻土机/强力钻土机(模拟器).lav` | Game / game package | Pending |
| 147 | `LAVA1 忍う者/打包的/忍う者.pac` | Game / game package | Pending |
| 148 | `LAVA1 忍う者/没打包的/忍う者.lav` | Game / game package | Pending |
| 149 | `LAVA1 情怨/pac/Love.pac` | Game / game package | Pending |
| 150 | `LAVA1 情怨/非pac/Love_I.lav` | Game / game package | Pending |
| 151 | `LAVA1 我的世界~霸王屎/我的世界.lav` | Game / game package | [Workflow passed: `my_world_creates_a_character_and_moves_on_the_map`](../crates/lavaxemu/tests/game_compatibility.rs#L1008) |
| 152 | `LAVA1 我的世界异界版/我的世界.lav` | Game / game package | Pending |
| 153 | `LAVA1 战略1号1.0/战略1号.pac` | Game / game package | Pending |
| 154 | `LAVA1 战略1号1.0/非pac/战略1号.lav` | Game / game package | Pending |
| 155 | `LAVA1 扑克大战/扑克大战.lav` | Game / game package | Pending |
| 156 | `LAVA1 打飞机/打飞机.lav` | Game / game package | Pending |
| 157 | `LAVA1 扫雷/扫雷.lav` | Game / game package | Pending |
| 158 | `LAVA1 扳手腕/扳手腕.lav` | Game / game package | Pending |
| 159 | `LAVA1 找茬/大家来找茬.lav` | Game / game package | Pending |
| 160 | `LAVA1 找茬/找茬.pac` | Game / game package | Pending |
| 161 | `LAVA1 护卫战舰/护卫战舰.pac` | Game / game package | Pending |
| 162 | `LAVA1 护卫战舰/非PAC/护卫战舰.lav` | Game / game package | Pending |
| 163 | `LAVA1 拳击/boxing.lav` | Game / game package | Pending |
| 164 | `LAVA1 拿火柴/拿火柴.lav` | Game / game package | Pending |
| 165 | `LAVA1 接水管/接水管.lav` | Game / game package | Pending |
| 166 | `LAVA1 推箱达人/推箱达人.pac` | Game / game package | Pending |
| 167 | `LAVA1 推箱达人/未打包的/推箱达人.lav` | Game / game package | Pending |
| 168 | `LAVA1 搬运工/搬运工.lav` | Game / game package | Pending |
| 169 | `LAVA1 撞球高手/撞球高手.lav` | Game / game package | Pending |
| 170 | `LAVA1 数独1.3/sudoku.lav` | Game / game package | Pending |
| 171 | `LAVA1 数电模拟器/已打包的文件/数电模拟器.pac` | Circuit simulator | Pending |
| 172 | `LAVA1 数电模拟器/未打包的文件/数电模拟器.lav` | Circuit simulator | Pending |
| 173 | `LAVA1 新英雄坛说/新英雄传说.pac` | Game / game package | Pending |
| 174 | `LAVA1 新英雄坛说/非pac/Hero.lav` | Game / game package | [Workflow passed: `new_hero_reaches_the_village_and_scrolls_the_map`](../crates/lavaxemu/tests/game_compatibility.rs#L803) |
| 175 | `LAVA1 旋转老虎机/旋转老虎机.lav` | Game / game package | Pending |
| 176 | `LAVA1 无底洞/无底洞.lav` | Game / game package | Pending |
| 177 | `LAVA1 星剑传奇/打包的[星剑传奇]/星剑传奇.pac` | Game / game package | Pending |
| 178 | `LAVA1 星剑传奇/未打包的[星剑传奇]/星剑传奇.lav` | Game / game package | [Workflow passed: `star_legend_registers_a_character_and_loads_the_map`](../crates/lavaxemu/tests/game_compatibility.rs#L1185) |
| 179 | `LAVA1 星剑传奇/未打包的[星剑传奇]/星剑注册.lav` | Account registration | [Workflow passed: `star_legend_registers_a_character_and_loads_the_map`](../crates/lavaxemu/tests/game_compatibility.rs#L1185) |
| 180 | `LAVA1 是男人就下100层（模拟器可能不能运行）/是男人就下100层.lav` | Game / game package | Pending |
| 181 | `LAVA1 是男人就撑20秒正式版/是男人就撑20秒正式版.lav` | Game / game package | Pending |
| 182 | `LAVA1 是男人就飞10000米/是男人就飞10000米.lav` | Game / game package | Pending |
| 183 | `LAVA1 暗子象棋/暗子象棋.pac` | Game / game package | [Workflow passed: `dark_chess_loads_the_board_and_responds_to_moves`](../crates/lavaxemu/tests/game_compatibility.rs#L101) |
| 184 | `LAVA1 暗子象棋/未打包/暗子象棋.lav` | Game / game package | [Workflow passed: `dark_chess_loads_the_board_and_responds_to_moves`](../crates/lavaxemu/tests/game_compatibility.rs#L101) |
| 185 | `LAVA1 月光城/Moon.pac` | Game / game package | Pending |
| 186 | `LAVA1 月光城/非pac/Moon.lav` | Game / game package | Pending |
| 187 | `LAVA1 未知/game.lav` | Game / game package | Pending |
| 188 | `LAVA1 极速狂飙/极速狂飙.lav` | Game / game package | [Workflow passed: `speed_racing_loads_the_track_and_responds_to_steering`](../crates/lavaxemu/tests/game_compatibility.rs#L524) |
| 189 | `LAVA1 极限斗地主/极限斗地主.lav` | Game / game package | [Workflow passed: `extreme_landlord_versions_deal_select_play_and_save`](../crates/lavaxemu/tests/game_compatibility.rs#L1395) |
| 190 | `LAVA1 极限斗地主3.2/极限斗地主.lav` | Game / game package | [Workflow passed: `extreme_landlord_versions_deal_select_play_and_save`](../crates/lavaxemu/tests/game_compatibility.rs#L1395) |
| 191 | `LAVA1 极限斗地主3.2/极限斗地主NC1020版.lav` | Game / game package | [Workflow passed: `extreme_landlord_versions_deal_select_play_and_save`](../crates/lavaxemu/tests/game_compatibility.rs#L1395) |
| 192 | `LAVA1 极限斗地主3.2/极限斗地主tc800版.lav` | Game / game package | [Workflow passed: `extreme_landlord_versions_deal_select_play_and_save`](../crates/lavaxemu/tests/game_compatibility.rs#L1395) |
| 193 | `LAVA1 梦中世界（更新6）/梦中世界.lav` | Game / game package | [Workflow passed: `dream_world_creates_an_account_loads_the_map_and_moves`](../crates/lavaxemu/tests/game_compatibility.rs#L1647) |
| 194 | `LAVA1 梦幻21点/梦幻21点.lav` | Game / game package | Pending |
| 195 | `LAVA1 梦幻mario/梦幻mario.lav` | Game / game package | Pending |
| 196 | `LAVA1 橡皮屋/橡皮屋.lav` | Game / game package | Pending |
| 197 | `LAVA1 欢乐五子棋/欢乐五子棋.lav` | Game / game package | Pending |
| 198 | `LAVA1 武侠/武侠.lav` | Game / game package | Pending |
| 199 | `LAVA1 水管工/PC模拟器端/水管工.lav` | Game / game package | Pending |
| 200 | `LAVA1 水管工/文曲星端/水管工.lav` | Game / game package | Pending |
| 201 | `LAVA1 江湖--海贼王传说/world.lav` | Game / game package | Pending |
| 202 | `LAVA1 江湖-混沌初开/world.lav` | Game / game package | Pending |
| 203 | `LAVA1 汪汪大富翁/wangwang.lav` | Game / game package | Pending |
| 204 | `LAVA1 流星蝴蝶剑/流星蝴蝶剑.lav` | Game / game package | Pending |
| 205 | `LAVA1 海淀浮生记/海淀浮生记.lav` | Game / game package | Pending |
| 206 | `LAVA1 海盗船/打包/海盗船.pac` | Game / game package | Pending |
| 207 | `LAVA1 海盗船/未打包/海盗船.lav` | Game / game package | Pending |
| 208 | `LAVA1 火箭车/火箭车.lav` | Game / game package | [Workflow passed: `rocket_car_keeps_the_track_visible_and_responds_to_controls`](../crates/lavaxemu/tests/game_compatibility.rs#L1287) |
| 209 | `LAVA1 灵剑射击/灵剑射击.lav` | Game / game package | Pending |
| 210 | `LAVA1 炸潜艇/炸潜艇v1.1 2007.7.6.lav` | Game / game package | Pending |
| 211 | `LAVA1 煞魔剑/mywar1.lav` | Game / game package | Pending |
| 212 | `LAVA1 煞魔剑A/煞魔剑A.lav` | Game / game package | Pending |
| 213 | `LAVA1 煞魔剑正式版/【煞魔剑】 正式版.lav` | Game / game package | Pending |
| 214 | `LAVA1 爬虫大陆编年史/爬虫大陆.lav` | Game / game package | Pending |
| 215 | `LAVA1 版炸弹人/BomB.lav` | Game / game package | Pending |
| 216 | `LAVA1 猫狗大战2/猫狗大战Ⅱ.lav` | Game / game package | Pending |
| 217 | `LAVA1 生死时速II/生死时速Ⅱ（修改版）.lav` | Game / game package | [Workflow passed: `modified_speed_racing_steers_and_returns_from_the_score_screen`](../crates/lavaxemu/tests/game_compatibility.rs#L572) |
| 218 | `LAVA1 生物钟/生物钟.lav` | Utility / education | Pending |
| 219 | `LAVA1 疯狂F2/F2.lav` | Game / game package | [Workflow passed: `crazy_f2_counts_presses_and_finishes_the_timed_round`](../crates/lavaxemu/tests/game_compatibility.rs#L470) |
| 220 | `LAVA1 疯狂弹球/疯狂弹球.pac` | Game / game package | [Workflow passed: `crazy_ball_packaged_and_loose_versions_load_the_stage_and_move_the_paddle`](../crates/lavaxemu/tests/game_compatibility.rs#L1067) |
| 221 | `LAVA1 疯狂弹球/非pac/crazyball.lav` | Game / game package | [Workflow passed: `crazy_ball_packaged_and_loose_versions_load_the_stage_and_move_the_paddle`](../crates/lavaxemu/tests/game_compatibility.rs#L1067) |
| 222 | `LAVA1 疯狂弹球/非pac/mapedit.lav` | Map editor | Pending |
| 223 | `LAVA1 疯狂跳棋/打包的文件/fktq.pac` | Game / game package | Pending |
| 224 | `LAVA1 疯狂跳棋/疯狂跳棋.lav` | Game / game package | Pending |
| 225 | `LAVA1 疾龙云飞/打包的文件/bobo_cloud.pac` | Game / game package | [Workflow passed: `cloud_versions_configure_keys_jump_and_persist_the_configuration`](../crates/lavaxemu/tests/game_compatibility.rs#L1993) |
| 226 | `LAVA1 疾龙云飞/未打包的文件/cloud_all.lav` | Game / game package | [Workflow passed: `cloud_versions_configure_keys_jump_and_persist_the_configuration`](../crates/lavaxemu/tests/game_compatibility.rs#L1993) |
| 227 | `LAVA1 白白快跑/已打包的/白白快跑.pac` | Game / game package | Pending |
| 228 | `LAVA1 白白快跑/未打包的/LAVA/白白快跑.lav` | Game / game package | Pending |
| 229 | `LAVA1 白金贪吃蛇/白金贪吃蛇.lav` | Game / game package | Pending |
| 230 | `LAVA1 百战天虫即时版/VS版/GVmaker/天虫即时vs.lav` | Game / game package | Pending |
| 231 | `LAVA1 百战天虫即时版/天虫即时.lav` | Game / game package | Pending |
| 232 | `LAVA1 百战天虫即时版/资源管理器.lav` | Utility / education | [Workflow passed: `resource_manager_selects_files_and_cancels_the_dialog`](../crates/lavaxemu/tests/game_compatibility.rs#L623) |
| 233 | `LAVA1 礼物大作战/YoGamesPC.lav` | Game / game package | Pending |
| 234 | `LAVA1 礼物大作战/YoGamesWQX.lav` | Game / game package | Pending |
| 235 | `LAVA1 神州/神州.pac` | Game / game package | [Workflow passed: `magic_packaged_and_loose_versions_load_resources_and_move_on_the_map`](../crates/lavaxemu/tests/game_compatibility.rs#L691) |
| 236 | `LAVA1 神州/非pac/Magic.lav` | Game / game package | [Workflow passed: `magic_packaged_and_loose_versions_load_resources_and_move_on_the_map`](../crates/lavaxemu/tests/game_compatibility.rs#L691) |
| 237 | `LAVA1 种金币/种金币.lav` | Game / game package | [Workflow passed: `planting_gold_moves_a_coin_and_restarts_the_level`](../crates/lavaxemu/tests/game_compatibility.rs#L1092) |
| 238 | `LAVA1 秘境还生/秘境还生.lav` | Game / game package | Pending |
| 239 | `LAVA1 空当接龙/空当接龙.lav` | Game / game package | Pending |
| 240 | `LAVA1 空间战争/SpaceWar.lav` | Game / game package | Pending |
| 241 | `LAVA1 类似十全十美/allout.lav` | Game / game package | Pending |
| 242 | `LAVA1 精灵岛5-勇者之旅/精灵岛5.lav` | Game / game package | Pending |
| 243 | `LAVA1 精灵岛5-勇者之旅/精灵岛5.pac` | Game / game package | Pending |
| 244 | `LAVA1 绝对下落/分步安装/绝对下落.lav` | Game / game package | Pending |
| 245 | `LAVA1 美少女梦工厂2黑白版/PM2.lav` | Game / game package | [Workflow passed: `princess_maker_2_shows_stats_executes_a_plan_and_reloads_its_save`](../crates/lavaxemu/tests/game_compatibility.rs#L1587) |
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
| 258 | `LAVA1 蜀山群侠传/蜀山注册.lav` | Account registration | [Workflow passed: `shushan_registers_a_character_and_enters_the_game`](../crates/lavaxemu/tests/game_compatibility.rs#L947) |
| 259 | `LAVA1 蜀山群侠传/蜀山群侠传.lav` | Game / game package | [Workflow passed: `shushan_registers_a_character_and_enters_the_game`](../crates/lavaxemu/tests/game_compatibility.rs#L947) |
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
| 273 | `LAVA1 超级机器人大战/Setup/SRW.lav` | Installer | [Workflow passed: `super_robot_wars_installs_resources_and_accepts_map_input`](../crates/lavaxemu/tests/game_compatibility.rs#L722) |
| 274 | `LAVA1 超级机器人大战/Setup/SRWDate.lav` | Installer | [Workflow passed: `super_robot_wars_installs_resources_and_accepts_map_input`](../crates/lavaxemu/tests/game_compatibility.rs#L722) |
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
| 305 | `LAVA1 龙之传说/非pac/longX.lav` | Game / game package | [Workflow passed: `dragon_legend_selects_classes_moves_and_reloads_a_local_save`](../crates/lavaxemu/tests/game_compatibility.rs#L1732) |
| 306 | `LAVA1 龙之吻体验版/l2体验版.lav` | Game / game package | [Workflow passed: `dragon_kiss_creates_a_chinese_name_moves_and_loads_party_progress`](../crates/lavaxemu/tests/game_compatibility.rs#L1837) |
| 307 | `LAVA1 龙之崛起[十一特别版]/LAVA/G.龙之崛起.lav` | Game / game package | Pending |
| 308 | `LAVA1 龙之崛起[十一特别版]/LAVA/L.龙之崛起.lav` | Game / game package | Pending |
| 309 | `LAVA1 龙之崛起[十一特别版]/Package/龙之崛起.pac` | Game / game package | Pending |
| 310 | `LAVA2 仙境传说/仙境.pac` | Game / game package | Pending |
| 311 | `LAVA2 仙境传说/非pac/仙境.lav` | Game / game package | Pending |
| 312 | `LAVA2 冒险岛IV/灰度版/打包的文件/冒险岛IV.pac` | Game / game package | Pending |
| 313 | `LAVA2 冒险岛IV/灰度版/未打包的文件/冒险岛IV.lav` | Game / game package | Pending |
| 314 | `LAVA2 心跳回忆/心跳回忆.lav` | Game / game package | [Workflow passed: `tokimeki_reaches_the_room_and_selects_an_activity`](../crates/lavaxemu/tests/game_compatibility.rs#L825) |
| 315 | `LAVA2 打地鼠/LAVA2/Hit.lav` | Game / game package | [Workflow passed: `hit_loads_the_logo_runs_and_pauses_with_the_keyboard`](../crates/lavaxemu/tests/game_compatibility.rs#L349) |
| 316 | `LAVA2 打地鼠/Package/HitV1.0.pac` | Game / game package | Pending |
| 317 | `LAVA2 挑战推箱子1.31/挑战推箱子.lav` | Game / game package | Pending |
| 318 | `LAVA2 搬运工/搬运工.lav` | Game / game package | Pending |
| 319 | `LAVA2 易码事件簿-寻找CEO/非pac/EMStory.lav` | Game / game package | Pending |
| 320 | `LAVA2 玩转变色龙/玩转变色龙.lav` | Game / game package | Pending |
| 321 | `LAVA2 美少女梦工场/非pac/PM4.lav` | Game / game package | [Workflow passed: `princess_maker_4_reaches_gameplay_and_opens_the_status_menu`](../crates/lavaxemu/tests/game_compatibility.rs#L288) |
| 322 | `LAVA2 魔法纪元/魔法纪元.lav` | Game / game package | [Workflow passed: `magic_era_accepts_the_device_identifier_and_enters_gameplay`](../crates/lavaxemu/tests/game_compatibility.rs#L769) |
| 323 | `LAVA2 麻将智多星/BoBo_MJ.lav` | Game / game package | Pending |
