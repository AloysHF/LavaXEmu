# Game Compatibility

LAV programs in the local validation corpus were run by the standalone
emulator with default capture timing (300 frames). Every screenshot below
is the indexed-color framebuffer produced by guest execution. If a program
stops, times out, or leaves a single-color framebuffer, the batch does not
create a screenshot. A successful startup capture does not guarantee that
every screen or gameplay path works correctly.

## Supported Program Profile

The current core recognizes LavaX bytecode programs with the standard LAV
file header. It implements the VM subsets needed for stack-based execution,
indexed-color display, keyboard/pointer input, virtual file system access,
and basic system API services.

Validated behavior includes program initialization, startup screens, display
rendering, keyboard input, and continued frame execution.

## Summary

| Status | Count |
| --- | ---: |
| ✅ Pass | 266 |
| ❌ Fail | 1 |
| **Total** | **267** |

## Application List

| # | Application | Screenshot | Status |
| ---: | --- | --- | --- |
| 1 | 105 | <img src="images/105.png" width="120"> | ✅ Pass |
| 2 | 3D迷宫 | <img src="images/3D迷宫.png" width="120"> | ✅ Pass |
| 3 | 9stars | <img src="images/9stars.png" width="120"> | ✅ Pass |
| 4 | BoBo_MJ | <img src="images/BoBo_MJ.png" width="120"> | ✅ Pass |
| 5 | Bobo_Dance | <img src="images/Bobo_Dance.png" width="120"> | ✅ Pass |
| 6 | BomB | <img src="images/BomB.png" width="120"> | ✅ Pass |
| 7 | CFM | <img src="images/CFM.png" width="120"> | ✅ Pass |
| 8 | CS蓝色行动 | <img src="images/CS蓝色行动.png" width="120"> | ✅ Pass |
| 9 | Clear | <img src="images/Clear.png" width="120"> | ✅ Pass |
| 10 | Cs(lav)1_3 | <img src="images/Cs(lav)1_3.png" width="120"> | ✅ Pass |
| 11 | EMStory | <img src="images/EMStory.png" width="120"> | ✅ Pass |
| 12 | EM星纪传说—格斗 | <img src="images/EM星纪传说—格斗.png" width="120"> | ✅ Pass |
| 13 | Eagle | <img src="images/Eagle.png" width="120"> | ✅ Pass |
| 14 | F2 | <img src="images/F2.png" width="120"> | ✅ Pass |
| 15 | Fighter | <img src="images/Fighter.png" width="120"> | ✅ Pass |
| 16 | Fivet | <img src="images/Fivet.png" width="120"> | ✅ Pass |
| 17 | For Computer（模拟器用） | <img src="images/For Computer（模拟器用）.png" width="120"> | ✅ Pass |
| 18 | G.龙之崛起 | <img src="images/G.龙之崛起.png" width="120"> | ✅ Pass |
| 19 | HFAcs | <img src="images/HFAcs.png" width="120"> | ✅ Pass |
| 20 | Hero | <img src="images/Hero.png" width="120"> | ✅ Pass |
| 21 | Hit | <img src="images/Hit.png" width="120"> | ✅ Pass |
| 22 | IQ大作战 | <img src="images/IQ大作战.png" width="120"> | ✅ Pass |
| 23 | Jackal | <img src="images/Jackal.png" width="120"> | ✅ Pass |
| 24 | Just_Fly | <img src="images/Just_Fly.png" width="120"> | ✅ Pass |
| 25 | L.龙之崛起 | <img src="images/L.龙之崛起.png" width="120"> | ✅ Pass |
| 26 | LF_demo(PC) | <img src="images/LF_demo(PC).png" width="120"> | ✅ Pass |
| 27 | LF_demo | <img src="images/LF_demo.png" width="120"> | ✅ Pass |
| 28 | Land | <img src="images/Land.png" width="120"> | ✅ Pass |
| 29 | LazyYY | <img src="images/LazyYY.png" width="120"> | ✅ Pass |
| 30 | Liuye | <img src="images/Liuye.png" width="120"> | ✅ Pass |
| 31 | Love_I | <img src="images/Love_I.png" width="120"> | ✅ Pass |
| 32 | Magic | <img src="images/Magic.png" width="120"> | ✅ Pass |
| 33 | MahJong | <img src="images/MahJong.png" width="120"> | ✅ Pass |
| 34 | MapEditor | <img src="images/MapEditor.png" width="120"> | ✅ Pass |
| 35 | Moon | <img src="images/Moon.png" width="120"> | ✅ Pass |
| 36 | NC2600 | <img src="images/NC2600.png" width="120"> | ✅ Pass |
| 37 | NC3000 | <img src="images/NC3000.png" width="120"> | ✅ Pass |
| 38 | PACMAN | <img src="images/PACMAN.png" width="120"> | ✅ Pass |
| 39 | PM2 | <img src="images/PM2.png" width="120"> | ✅ Pass |
| 40 | PM4 | <img src="images/PM4.png" width="120"> | ✅ Pass |
| 41 | PTV1.0 | <img src="images/PTV1.0.png" width="120"> | ✅ Pass |
| 42 | Pala | <img src="images/Pala.png" width="120"> | ✅ Pass |
| 43 | Pala_ | <img src="images/Pala_.png" width="120"> | ✅ Pass |
| 44 | Pala_TC800 | <img src="images/Pala_TC800.png" width="120"> | ✅ Pass |
| 45 | Psheep | <img src="images/Psheep.png" width="120"> | ✅ Pass |
| 46 | QUEERBOX | <img src="images/QUEERBOX.png" width="120"> | ✅ Pass |
| 47 | Q版Dancer | <img src="images/Q版Dancer.png" width="120"> | ✅ Pass |
| 48 | RC赛车 | <img src="images/RC赛车.png" width="120"> | ✅ Pass |
| 49 | Rush_Out_For_Lava(8K) | <img src="images/Rush_Out_For_Lava(8K).png" width="120"> | ✅ Pass |
| 50 | Rush_Out_For_Lava1(16k) | <img src="images/Rush_Out_For_Lava1(16k).png" width="120"> | ✅ Pass |
| 51 | Rush_Out_For_Lava2(16k) | <img src="images/Rush_Out_For_Lava2(16k).png" width="120"> | ✅ Pass |
| 52 | SCGO_219 | <img src="images/SCGO_219.png" width="120"> | ✅ Pass |
| 53 | SCGO_219_16gray | <img src="images/SCGO_219_16gray.png" width="120"> | ✅ Pass |
| 54 | SRW-2 | <img src="images/SRW-2.png" width="120"> | ✅ Pass |
| 55 | SRW | <img src="images/SRW.png" width="120"> | ✅ Pass |
| 56 | SRWDate | <img src="images/SRWDate.png" width="120"> | ✅ Pass |
| 57 | SWR无延时 | <img src="images/SWR无延时.png" width="120"> | ✅ Pass |
| 58 | SWR有延时 | <img src="images/SWR有延时.png" width="120"> | ✅ Pass |
| 59 | Shengjin2 | <img src="images/Shengjin2.png" width="120"> | ✅ Pass |
| 60 | SnowFight | <img src="images/SnowFight.png" width="120"> | ✅ Pass |
| 61 | SpaceWar | <img src="images/SpaceWar.png" width="120"> | ✅ Pass |
| 62 | Sword | <img src="images/Sword.png" width="120"> | ✅ Pass |
| 63 | TC1000 | <img src="images/TC1000.png" width="120"> | ✅ Pass |
| 64 | TC1000S | <img src="images/TC1000S.png" width="120"> | ✅ Pass |
| 65 | TC808 | <img src="images/TC808.png" width="120"> | ✅ Pass |
| 66 | WZLZ_16灰度 | <img src="images/WZLZ_16灰度.png" width="120"> | ✅ Pass |
| 67 | WZLZ_黑白 | <img src="images/WZLZ_黑白.png" width="120"> | ✅ Pass |
| 68 | WarCraft | <img src="images/WarCraft.png" width="120"> | ✅ Pass |
| 69 | Wing | <img src="images/Wing.png" width="120"> | ✅ Pass |
| 70 | Yahtzee | <img src="images/Yahtzee.png" width="120"> | ✅ Pass |
| 71 | YoGamesPC | <img src="images/YoGamesPC.png" width="120"> | ✅ Pass |
| 72 | YoGamesWQX | <img src="images/YoGamesWQX.png" width="120"> | ✅ Pass |
| 73 | allout | <img src="images/allout.png" width="120"> | ✅ Pass |
| 74 | boxing | <img src="images/boxing.png" width="120"> | ✅ Pass |
| 75 | cloud_all-2 | <img src="images/cloud_all-2.png" width="120"> | ✅ Pass |
| 76 | cloud_all | <img src="images/cloud_all.png" width="120"> | ✅ Pass |
| 77 | crazyball-2 | <img src="images/crazyball-2.png" width="120"> | ✅ Pass |
| 78 | crazyball | <img src="images/crazyball.png" width="120"> | ✅ Pass |
| 79 | dzl | <img src="images/dzl.png" width="120"> | ✅ Pass |
| 80 | game | <img src="images/game.png" width="120"> | ✅ Pass |
| 81 | gcdmx | <img src="images/gcdmx.png" width="120"> | ✅ Pass |
| 82 | l2体验版 | <img src="images/l2体验版.png" width="120"> | ✅ Pass |
| 83 | longX | <img src="images/longX.png" width="120"> | ✅ Pass |
| 84 | mapedit-2 | <img src="images/mapedit-2.png" width="120"> | ✅ Pass |
| 85 | mapedit | <img src="images/mapedit.png" width="120"> | ✅ Pass |
| 86 | maze2 | <img src="images/maze2.png" width="120"> | ✅ Pass |
| 87 | mywar1 | <img src="images/mywar1.png" width="120"> | ✅ Pass |
| 88 | pokemon | <img src="images/pokemon.png" width="120"> | ✅ Pass |
| 89 | sudoku | <img src="images/sudoku.png" width="120"> | ✅ Pass |
| 90 | travel | <img src="images/travel.png" width="120"> | ✅ Pass |
| 91 | tunshi_V0.29 | <img src="images/tunshi_V0.29.png" width="120"> | ✅ Pass |
| 92 | wangwang | <img src="images/wangwang.png" width="120"> | ✅ Pass |
| 93 | wind | <img src="images/wind.png" width="120"> | ✅ Pass |
| 94 | world-2 | <img src="images/world-2.png" width="120"> | ✅ Pass |
| 95 | world | <img src="images/world.png" width="120"> | ✅ Pass |
| 96 | world2 | <img src="images/world2.png" width="120"> | ✅ Pass |
| 97 | world2_16 | <img src="images/world2_16.png" width="120"> | ✅ Pass |
| 98 | 《英雄无敌2》完美版 | <img src="images/《英雄无敌2》完美版.png" width="120"> | ✅ Pass |
| 99 | 【煞魔剑】 正式版 | <img src="images/【煞魔剑】 正式版.png" width="120"> | ✅ Pass |
| 100 | 七彩泡泡 | <img src="images/七彩泡泡.png" width="120"> | ✅ Pass |
| 101 | 七彩连珠 | <img src="images/七彩连珠.png" width="120"> | ✅ Pass |
| 102 | 七王五二三 | <img src="images/七王五二三.png" width="120"> | ✅ Pass |
| 103 | 中国灯谜 | <img src="images/中国灯谜.png" width="120"> | ✅ Pass |
| 104 | 中国烟火 | <img src="images/中国烟火.png" width="120"> | ✅ Pass |
| 105 | 中国麻将 | <img src="images/中国麻将.png" width="120"> | ✅ Pass |
| 106 | 中学传奇 | <img src="images/中学传奇.png" width="120"> | ✅ Pass |
| 107 | 五子连珠 | <img src="images/五子连珠.png" width="120"> | ✅ Pass |
| 108 | 仙境 | <img src="images/仙境.png" width="120"> | ✅ Pass |
| 109 | 仙境奇缘 | <img src="images/仙境奇缘.png" width="120"> | ✅ Pass |
| 110 | 任意拼图 | <img src="images/任意拼图.png" width="120"> | ✅ Pass |
| 111 | 俄罗斯扑克 | <img src="images/俄罗斯扑克.png" width="120"> | ✅ Pass |
| 112 | 俄罗斯方块灰 | <img src="images/俄罗斯方块灰.png" width="120"> | ✅ Pass |
| 113 | 俄罗斯方块黑 | <img src="images/俄罗斯方块黑.png" width="120"> | ✅ Pass |
| 114 | 假日神秘岛 abcdef | <img src="images/假日神秘岛 abcdef.png" width="120"> | ✅ Pass |
| 115 | 元素周期 | <img src="images/元素周期.png" width="120"> | ✅ Pass |
| 116 | 先赌★为快 | <img src="images/先赌★为快.png" width="120"> | ✅ Pass |
| 117 | 冒险岛IV-2 | <img src="images/冒险岛IV-2.png" width="120"> | ✅ Pass |
| 118 | 冒险岛IV | <img src="images/冒险岛IV.png" width="120"> | ✅ Pass |
| 119 | 决战娜美克 | <img src="images/决战娜美克.png" width="120"> | ✅ Pass |
| 120 | 凌空抽射v1.5 | <img src="images/凌空抽射v1.5.png" width="120"> | ✅ Pass |
| 121 | 勇者传说 | <img src="images/勇者传说.png" width="120"> | ✅ Pass |
| 122 | 勇者斗恶龙-2 | <img src="images/勇者斗恶龙-2.png" width="120"> | ✅ Pass |
| 123 | 勇者斗恶龙 | <img src="images/勇者斗恶龙.png" width="120"> | ✅ Pass |
| 124 | 勇闯地狱 | <img src="images/勇闯地狱.png" width="120"> | ✅ Pass |
| 125 | 博士失踪记 | <img src="images/博士失踪记.png" width="120"> | ✅ Pass |
| 126 | 口水王2-2 | <img src="images/口水王2-2.png" width="120"> | ✅ Pass |
| 127 | 口水王2 | <img src="images/口水王2.png" width="120"> | ✅ Pass |
| 128 | 吃月饼 | <img src="images/吃月饼.png" width="120"> | ✅ Pass |
| 129 | 困虫斗 | <img src="images/困虫斗.png" width="120"> | ✅ Pass |
| 130 | 圈地运动 | <img src="images/圈地运动.png" width="120"> | ✅ Pass |
| 131 | 坦克2004 | <img src="images/坦克2004.png" width="120"> | ✅ Pass |
| 132 | 大家来找茬 | <img src="images/大家来找茬.png" width="120"> | ✅ Pass |
| 133 | 天虫即时 | <img src="images/天虫即时.png" width="120"> | ✅ Pass |
| 134 | 天虫即时vs | <img src="images/天虫即时vs.png" width="120"> | ✅ Pass |
| 135 | 太空寻宝 | <img src="images/太空寻宝.png" width="120"> | ✅ Pass |
| 136 | 太空战舰 | <img src="images/太空战舰.png" width="120"> | ✅ Pass |
| 137 | 存档转化器_山尘围棋 | <img src="images/存档转化器_山尘围棋.png" width="120"> | ✅ Pass |
| 138 | 宇宙大战 | <img src="images/宇宙大战.png" width="120"> | ✅ Pass |
| 139 | 宠世L_1K | <img src="images/宠世L_1K.png" width="120"> | ✅ Pass |
| 140 | 宠世L_1KS | <img src="images/宠世L_1KS.png" width="120"> | ✅ Pass |
| 141 | 宠世L_2K6 | <img src="images/宠世L_2K6.png" width="120"> | ✅ Pass |
| 142 | 宠世L_3K | <img src="images/宠世L_3K.png" width="120"> | ✅ Pass |
| 143 | 宠世L_808 | <img src="images/宠世L_808.png" width="120"> | ✅ Pass |
| 144 | 宠世导出入 | <img src="images/宠世导出入.png" width="120"> | ✅ Pass |
| 145 | 宠物世界1K | <img src="images/宠物世界1K.png" width="120"> | ✅ Pass |
| 146 | 宠物世界1S | <img src="images/宠物世界1S.png" width="120"> | ✅ Pass |
| 147 | 宠物世界26 | <img src="images/宠物世界26.png" width="120"> | ✅ Pass |
| 148 | 宠物世界3K | <img src="images/宠物世界3K.png" width="120"> | ✅ Pass |
| 149 | 宠物世界80 | <img src="images/宠物世界80.png" width="120"> | ✅ Pass |
| 150 | 宠物世界88 | <img src="images/宠物世界88.png" width="120"> | ✅ Pass |
| 151 | 密室杀机 | <img src="images/密室杀机.png" width="120"> | ✅ Pass |
| 152 | 富甲天下 | <img src="images/富甲天下.png" width="120"> | ✅ Pass |
| 153 | 对战方块 | <img src="images/对战方块.png" width="120"> | ✅ Pass |
| 154 | 小猴过河 | <img src="images/小猴过河.png" width="120"> | ✅ Pass |
| 155 | 山尘围棋v2 | <img src="images/山尘围棋v2.png" width="120"> | ✅ Pass |
| 156 | 山村淘金记 | <img src="images/山村淘金记.png" width="120"> | ✅ Pass |
| 157 | 帝国传-2 | <img src="images/帝国传-2.png" width="120"> | ✅ Pass |
| 158 | 帝国传 | <img src="images/帝国传.png" width="120"> | ✅ Pass |
| 159 | 幕天助手 | <img src="images/幕天助手.png" width="120"> | ✅ Pass |
| 160 | 幕天席地 | <img src="images/幕天席地.png" width="120"> | ✅ Pass |
| 161 | 幕天席地2 | <img src="images/幕天席地2.png" width="120"> | ✅ Pass |
| 162 | 幕天席地3_plus | <img src="images/幕天席地3_plus.png" width="120"> | ✅ Pass |
| 163 | 平面魔方 | <img src="images/平面魔方.png" width="120"> | ✅ Pass |
| 164 | 幻影战机 | <img src="images/幻影战机.png" width="120"> | ✅ Pass |
| 165 | 强力钻土机(Tc800) | <img src="images/强力钻土机(Tc800).png" width="120"> | ✅ Pass |
| 166 | 强力钻土机(其它机型) | <img src="images/强力钻土机(其它机型).png" width="120"> | ✅ Pass |
| 167 | 强力钻土机(模拟器) | <img src="images/强力钻土机(模拟器).png" width="120"> | ✅ Pass |
| 168 | 心跳回忆 | <img src="images/心跳回忆.png" width="120"> | ✅ Pass |
| 169 | 忍う者 | <img src="images/忍う者.png" width="120"> | ✅ Pass |
| 170 | 我的世界-2 | <img src="images/我的世界-2.png" width="120"> | ✅ Pass |
| 171 | 我的世界 | <img src="images/我的世界.png" width="120"> | ✅ Pass |
| 172 | 战略1号 | <img src="images/战略1号.png" width="120"> | ✅ Pass |
| 173 | 扑克21 | <img src="images/扑克21.png" width="120"> | ✅ Pass |
| 174 | 扑克大战 | <img src="images/扑克大战.png" width="120"> | ✅ Pass |
| 175 | 打飞机 | <img src="images/打飞机.png" width="120"> | ✅ Pass |
| 176 | 扫雷 | <img src="images/扫雷.png" width="120"> | ✅ Pass |
| 177 | 扳手腕 | <img src="images/扳手腕.png" width="120"> | ✅ Pass |
| 178 | 护卫战舰 | <img src="images/护卫战舰.png" width="120"> | ✅ Pass |
| 179 | 拿火柴 | <img src="images/拿火柴.png" width="120"> | ✅ Pass |
| 180 | 挑战推箱子 | <img src="images/挑战推箱子.png" width="120"> | ✅ Pass |
| 181 | 接水管 | <img src="images/接水管.png" width="120"> | ✅ Pass |
| 182 | 推箱达人 | <img src="images/推箱达人.png" width="120"> | ✅ Pass |
| 183 | 搬运工-2 | <img src="images/搬运工-2.png" width="120"> | ✅ Pass |
| 184 | 搬运工 | <img src="images/搬运工.png" width="120"> | ✅ Pass |
| 185 | 撞球高手 | <img src="images/撞球高手.png" width="120"> | ✅ Pass |
| 186 | 数电模拟器 | <img src="images/数电模拟器.png" width="120"> | ✅ Pass |
| 187 | 文曲星版Zelda | <img src="images/文曲星版Zelda.png" width="120"> | ✅ Pass |
| 188 | 旋转老虎机 | <img src="images/旋转老虎机.png" width="120"> | ✅ Pass |
| 189 | 无底洞 | <img src="images/无底洞.png" width="120"> | ✅ Pass |
| 190 | 星剑传奇 | <img src="images/星剑传奇.png" width="120"> | ✅ Pass |
| 191 | 星剑注册 | <img src="images/星剑注册.png" width="120"> | ✅ Pass |
| 192 | 是男人就下100层 | <img src="images/是男人就下100层.png" width="120"> | ✅ Pass |
| 193 | 是男人就撑20秒正式版 | <img src="images/是男人就撑20秒正式版.png" width="120"> | ✅ Pass |
| 194 | 是男人就飞10000米 | <img src="images/是男人就飞10000米.png" width="120"> | ✅ Pass |
| 195 | 暗子象棋 | <img src="images/暗子象棋.png" width="120"> | ✅ Pass |
| 196 | 极速狂飙 | <img src="images/极速狂飙.png" width="120"> | ✅ Pass |
| 197 | 极限斗地主-2 | <img src="images/极限斗地主-2.png" width="120"> | ✅ Pass |
| 198 | 极限斗地主 | <img src="images/极限斗地主.png" width="120"> | ✅ Pass |
| 199 | 极限斗地主NC1020版 | <img src="images/极限斗地主NC1020版.png" width="120"> | ✅ Pass |
| 200 | 极限斗地主tc800版 | <img src="images/极限斗地主tc800版.png" width="120"> | ✅ Pass |
| 201 | 梦中世界 | <img src="images/梦中世界.png" width="120"> | ✅ Pass |
| 202 | 梦幻21点 | <img src="images/梦幻21点.png" width="120"> | ✅ Pass |
| 203 | 梦幻mario | <img src="images/梦幻mario.png" width="120"> | ✅ Pass |
| 204 | 橡皮屋 | <img src="images/橡皮屋.png" width="120"> | ✅ Pass |
| 205 | 欢乐五子棋 | <img src="images/欢乐五子棋.png" width="120"> | ✅ Pass |
| 206 | 武侠 | <img src="images/武侠.png" width="120"> | ✅ Pass |
| 207 | 水管工-2 | <img src="images/水管工-2.png" width="120"> | ✅ Pass |
| 208 | 水管工 | <img src="images/水管工.png" width="120"> | ✅ Pass |
| 209 | 流星蝴蝶剑 | <img src="images/流星蝴蝶剑.png" width="120"> | ✅ Pass |
| 210 | 海淀浮生记 | <img src="images/海淀浮生记.png" width="120"> | ✅ Pass |
| 211 | 海盗船 | <img src="images/海盗船.png" width="120"> | ✅ Pass |
| 212 | 滑雪 | <img src="images/滑雪.png" width="120"> | ✅ Pass |
| 213 | 火箭车 | <img src="images/火箭车.png" width="120"> | ✅ Pass |
| 214 | 灵剑射击 | <img src="images/灵剑射击.png" width="120"> | ✅ Pass |
| 215 | 炸潜艇v1.1 2007.7.6 | <img src="images/炸潜艇v1.1 2007.7.6.png" width="120"> | ✅ Pass |
| 216 | 煞魔剑A | <img src="images/煞魔剑A.png" width="120"> | ✅ Pass |
| 217 | 爬虫大陆 | <img src="images/爬虫大陆.png" width="120"> | ✅ Pass |
| 218 | 猫狗大战Ⅱ | <img src="images/猫狗大战Ⅱ.png" width="120"> | ✅ Pass |
| 219 | 玩转变色龙 | <img src="images/玩转变色龙.png" width="120"> | ✅ Pass |
| 220 | 生死时速Ⅱ（修改版） | <img src="images/生死时速Ⅱ（修改版）.png" width="120"> | ✅ Pass |
| 221 | 生物钟 | <img src="images/生物钟.png" width="120"> | ✅ Pass |
| 222 | 疯狂跳棋 | <img src="images/疯狂跳棋.png" width="120"> | ✅ Pass |
| 223 | 白白快跑 | <img src="images/白白快跑.png" width="120"> | ✅ Pass |
| 224 | 白金贪吃蛇 | <img src="images/白金贪吃蛇.png" width="120"> | ✅ Pass |
| 225 | 种金币 | <img src="images/种金币.png" width="120"> | ✅ Pass |
| 226 | 秘境还生 | <img src="images/秘境还生.png" width="120"> | ✅ Pass |
| 227 | 空当接龙 | <img src="images/空当接龙.png" width="120"> | ✅ Pass |
| 228 | 精灵岛5 | <img src="images/精灵岛5.png" width="120"> | ✅ Pass |
| 229 | 绝对下落 | <img src="images/绝对下落.png" width="120"> | ✅ Pass |
| 230 | 花之精灵 | <img src="images/花之精灵.png" width="120"> | ✅ Pass |
| 231 | 英雄无敌v1.0 | <img src="images/英雄无敌v1.0.png" width="120"> | ✅ Pass |
| 232 | 萝卜大作战 | <img src="images/萝卜大作战.png" width="120"> | ✅ Pass |
| 233 | 蛙蛙大富翁 | <img src="images/蛙蛙大富翁.png" width="120"> | ✅ Pass |
| 234 | 蛙蛙马戏团 | <img src="images/蛙蛙马戏团.png" width="120"> | ✅ Pass |
| 235 | 蜀山注册 | <img src="images/蜀山注册.png" width="120"> | ✅ Pass |
| 236 | 蜀山群侠传 | <img src="images/蜀山群侠传.png" width="120"> | ✅ Pass |
| 237 | 蟑螂历险记 V1.0 | <img src="images/蟑螂历险记 V1.0.png" width="120"> | ✅ Pass |
| 238 | 蟑螂历险记V2.0 | <img src="images/蟑螂历险记V2.0.png" width="120"> | ✅ Pass |
| 239 | 蟑螂历险记V3.0 | <img src="images/蟑螂历险记V3.0.png" width="120"> | ✅ Pass |
| 240 | 蟑螂历险记V3.2-2 | <img src="images/蟑螂历险记V3.2-2.png" width="120"> | ✅ Pass |
| 241 | 蟑螂历险记V3.2 | <img src="images/蟑螂历险记V3.2.png" width="120"> | ✅ Pass |
| 242 | 衰 | <img src="images/衰.png" width="120"> | ✅ Pass |
| 243 | 象棋90 | <img src="images/象棋90.png" width="120"> | ✅ Pass |
| 244 | 象素画册 | <img src="images/象素画册.png" width="120"> | ✅ Pass |
| 245 | 贪吃蛇鲍勃 | <img src="images/贪吃蛇鲍勃.png" width="120"> | ✅ Pass |
| 246 | 资源管理器 | <img src="images/资源管理器.png" width="120"> | ✅ Pass |
| 247 | 超级舞者 | <img src="images/超级舞者.png" width="120"> | ✅ Pass |
| 248 | 过关画面 | <img src="images/过关画面.png" width="120"> | ✅ Pass |
| 249 | 连连看-2 | <img src="images/连连看-2.png" width="120"> | ✅ Pass |
| 250 | 连连看 | <img src="images/连连看.png" width="120"> | ✅ Pass |
| 251 | 逆转裁判 | <img src="images/逆转裁判.png" width="120"> | ✅ Pass |
| 252 | 通关动画 | <img src="images/通关动画.png" width="120"> | ✅ Pass |
| 253 | 钻石方块灰 | <img src="images/钻石方块灰.png" width="120"> | ✅ Pass |
| 254 | 钻石方块黑 | <img src="images/钻石方块黑.png" width="120"> | ✅ Pass |
| 255 | 钻石棋 | <img src="images/钻石棋.png" width="120"> | ✅ Pass |
| 256 | 雪人传奇 | <img src="images/雪人传奇.png" width="120"> | ✅ Pass |
| 257 | 零度恐惧 | <img src="images/零度恐惧.png" width="120"> | ✅ Pass |
| 258 | 青蛙冒险 | <img src="images/青蛙冒险.png" width="120"> | ✅ Pass |
| 259 | 飞翔理科1020 | <img src="images/飞翔理科1020.png" width="120"> | ✅ Pass |
| 260 | 飞行棋 | <img src="images/飞行棋.png" width="120"> | ✅ Pass |
| 261 | 魔塔 | <img src="images/魔塔.png" width="120"> | ✅ Pass |
| 262 | 魔塔整合版 | <img src="images/魔塔整合版.png" width="120"> | ✅ Pass |
| 263 | 魔法方块 | <img src="images/魔法方块.png" width="120"> | ✅ Pass |
| 264 | 魔法纪元 | <img src="images/魔法纪元.png" width="120"> | ✅ Pass |
| 265 | 魔界传说 | <img src="images/魔界传说.png" width="120"> | ✅ Pass |
| 266 | 黑白牛牛 | <img src="images/黑白牛牛.png" width="120"> | ✅ Pass |

## Known Limitations

- The emulator is in early development; some system APIs are not yet
  implemented.
- Audio playback is not yet implemented.
- Not all 87 system API services are fully implemented.
- Some programs may require specific system API behaviors that are not
  yet supported.
- Compatibility with all LavaX bytecode variants is not guaranteed.
- One program failed with "invalid LavaX opcode 0x00 at program offset 0x72".

## Reporting a Compatibility Issue

Include the program name, the last visible screen, the input that triggers
the problem, and any error text. When possible, reproduce it with headless
mode:

```bash
lavaxemu path/to/game.lav --headless --frames 600 --screenshot frame.png
```

Set `RUST_LOG=debug` to enable verbose logging. Do not attach copyrighted
game files to public issue reports.

## Adding New Programs

To add a new program to the compatibility list:

1. Place the `.lav` file in the validation directory
2. Run the batch screenshot script:
   ```powershell
   pwsh scripts/batch-screenshots.ps1
   ```
3. Verify the screenshot shows a valid game screen
4. Add an entry to the Application List table above
5. Update the Summary counts
