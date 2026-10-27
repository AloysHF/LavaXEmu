use std::path::PathBuf;

use lavaxemu_core::{Emulator, FrameStatus};

#[path = "../src/standalone/content.rs"]
#[allow(dead_code)]
mod content;

fn load(relative: &str) -> Emulator {
    let root = PathBuf::from(std::env::var_os("LAVAX_GAME_ROOT").expect("set LAVAX_GAME_ROOT"));
    content::load_emulator(&root.join(relative), &[]).expect("load game and companions")
}

fn advance(emulator: &mut Emulator, frames: usize) {
    for _ in 0..frames {
        let result = emulator
            .run_frame()
            .expect("game must execute without errors");
        assert!(
            !matches!(result.status, FrameStatus::Halted(_)),
            "game exited"
        );
    }
}

fn press(emulator: &mut Emulator, key: u8) {
    hold(emulator, key, 3);
}

fn hold(emulator: &mut Emulator, key: u8, frames: usize) {
    for _ in 0..frames {
        emulator.input_mut().set_keys([key]);
        advance(emulator, 1);
    }
    emulator.input_mut().set_keys([]);
    advance(emulator, 30);
}

fn capture(emulator: &Emulator, name: &str) {
    let Some(directory) = std::env::var_os("LAVAX_CAPTURE_DIR") else {
        return;
    };
    let directory = PathBuf::from(directory);
    std::fs::create_dir_all(&directory).unwrap();
    let display = emulator.display();
    let rgb: Vec<u8> = display
        .indexed_frame()
        .iter()
        .flat_map(|&pixel| display.palette()[usize::from(pixel)])
        .collect();
    image::save_buffer(
        directory.join(format!("{name}.png")),
        &rgb,
        u32::from(display.width()),
        u32::from(display.height()),
        image::ColorType::Rgb8,
    )
    .unwrap();
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn dark_chess_loads_the_board_and_responds_to_moves() {
    let mut emulator = load("LAVA1 暗子象棋/未打包/暗子象棋.lav");
    advance(&mut emulator, 360);
    for _ in 0..3 {
        press(&mut emulator, 13);
    }
    let board = emulator.display().indexed_frame().to_vec();
    assert!(board.iter().any(|&pixel| pixel != board[0]));
    press(&mut emulator, 22);
    assert_ne!(
        board,
        emulator.display().indexed_frame(),
        "cursor must move"
    );
    let cursor = emulator.display().indexed_frame().to_vec();
    press(&mut emulator, 13);
    assert_ne!(
        cursor,
        emulator.display().indexed_frame(),
        "piece must flip"
    );
    advance(&mut emulator, 1200);
}

#[test]
fn nested_device_content_loads_and_writes_to_the_same_root() {
    let root = tempfile::tempdir().unwrap();
    let device = root.path().join("device");
    let programs = device.join("GVmaker/NC3000");
    let data = device.join("LavaData");
    std::fs::create_dir_all(&programs).unwrap();
    std::fs::create_dir_all(&data).unwrap();
    let program_path = programs.join("test.lav");
    std::fs::write(&program_path, b"LAV\x12\0\0\0\0\0\0\0\0\0\0\0\0\x40").unwrap();
    std::fs::write(data.join("sprite.dat"), [1, 2]).unwrap();
    let mut emulator = content::load_emulator(&program_path, &[]).unwrap();
    assert_eq!(
        emulator.files().file("/LavaData/sprite.dat"),
        Some([1, 2].as_slice())
    );
    let handle = emulator
        .files_mut()
        .open("/LavaData/slot.sav", "w")
        .unwrap();
    emulator.files_mut().write(handle, &[9, 8]).unwrap();
    emulator.files_mut().close(handle);
    content::flush_dirty_files(&program_path, emulator.files_mut()).unwrap();
    assert_eq!(std::fs::read(data.join("slot.sav")).unwrap(), [9, 8]);
    assert!(!programs.join("lavadata").exists());
    let lava2 = device.join("LAVA2");
    std::fs::create_dir(&lava2).unwrap();
    std::fs::copy(&program_path, lava2.join("test.lav")).unwrap();
    let emulator = content::load_emulator(&lava2.join("test.lav"), &[]).unwrap();
    assert_eq!(
        emulator.files().file("/LavaData/sprite.dat"),
        Some([1, 2].as_slice())
    );

    let flat = root.path().join("flat");
    std::fs::create_dir_all(root.path().join("LavaData")).unwrap();
    std::fs::create_dir_all(&flat).unwrap();
    let flat_program = flat.join("test.lav");
    std::fs::copy(&program_path, &flat_program).unwrap();
    std::fs::write(flat.join("local.dat"), [3]).unwrap();
    let emulator = content::load_emulator(&flat_program, &[]).unwrap();
    assert_eq!(emulator.files().file("local.dat"), Some([3].as_slice()));
    assert!(
        emulator
            .files()
            .file("/device/LavaData/sprite.dat")
            .is_none()
    );
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn pet_world_3h_loads_nested_assets_and_opens_pet_management() {
    let mut emulator = load("LAVA1 宠物世界/GVmaker/NC3000/宠物世界3H.lav");
    advance(&mut emulator, 300);
    let main = emulator.display().indexed_frame().to_vec();
    assert!(main.iter().any(|&pixel| pixel != main[0]));
    press(&mut emulator, 21);
    assert_ne!(
        main,
        emulator.display().indexed_frame(),
        "information must open"
    );
    press(&mut emulator, 27);
    press(&mut emulator, 20);
    let management = emulator.display().indexed_frame().to_vec();
    assert_ne!(main, management, "pet management must open");
    press(&mut emulator, 19);
    assert_ne!(
        management,
        emulator.display().indexed_frame(),
        "pet details must open"
    );
    press(&mut emulator, 27);
    advance(&mut emulator, 300);
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn princess_maker_4_reaches_gameplay_and_opens_the_status_menu() {
    let mut emulator = load("LAVA2 美少女梦工场/非pac/PM4.lav");
    for frame in 0..6000 {
        if frame >= 300 {
            let input_frame = frame - 300;
            let key = [13, b'y', 13][(input_frame / 60) % 3];
            emulator
                .input_mut()
                .set_keys((input_frame % 60 < 3).then_some(key));
        }
        advance(&mut emulator, 1);
        if frame == 2999 {
            capture(&emulator, "pm4-confirmation");
        }
    }
    emulator.input_mut().set_keys([]);
    press(&mut emulator, 13);
    let game = emulator.display().indexed_frame().to_vec();
    capture(&emulator, "pm4-game");
    assert!(game.iter().any(|&pixel| pixel != game[0]));
    press(&mut emulator, 13);
    capture(&emulator, "pm4-status");
    assert!(
        game != emulator.display().indexed_frame(),
        "status menu must open"
    );
    advance(&mut emulator, 600);
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn puzzled_sheep_loads_levels_moves_the_cursor_and_runs_the_simulation() {
    let mut emulator = load("LAVA1 Puzzled Sheep/Psheep.lav");
    advance(&mut emulator, 60);
    let title = emulator.display().indexed_frame().to_vec();
    press(&mut emulator, 13);
    let level = emulator.display().indexed_frame().to_vec();
    capture(&emulator, "sheep-level");
    assert!(title != level, "starting must load the level");
    press(&mut emulator, 22);
    assert!(
        level != emulator.display().indexed_frame(),
        "cursor must move"
    );
    let selection = emulator.display().indexed_frame().to_vec();
    press(&mut emulator, 13);
    capture(&emulator, "sheep-running");
    assert!(
        selection != emulator.display().indexed_frame(),
        "simulation must animate"
    );
    press(&mut emulator, 27);
    advance(&mut emulator, 60);
    assert!(matches!(
        emulator.run_frame().unwrap().status,
        FrameStatus::WaitingForInput
    ));
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn hit_loads_the_logo_runs_and_pauses_with_the_keyboard() {
    let mut emulator = load("LAVA2 打地鼠/LAVA2/Hit.lav");
    advance(&mut emulator, 300);
    press(&mut emulator, 13);
    press(&mut emulator, 13);
    advance(&mut emulator, 120);
    capture(&emulator, "hit-game");
    let seconds = emulator.vm().memory()[0x22df];
    advance(&mut emulator, 120);
    assert_ne!(
        seconds,
        emulator.vm().memory()[0x22df],
        "GetTime must advance the game timer"
    );
    let game = emulator.display().indexed_frame().to_vec();
    for key in *b"tyughjbnm" {
        press(&mut emulator, key);
    }
    assert!(
        game != emulator.display().indexed_frame(),
        "hitting must update the game"
    );
    hold(&mut emulator, b'p', 45);
    let paused = emulator.display().indexed_frame().to_vec();
    advance(&mut emulator, 120);
    assert!(
        paused == emulator.display().indexed_frame(),
        "pause must freeze gameplay"
    );
    press(&mut emulator, 13);
    advance(&mut emulator, 120);
    capture(&emulator, "hit-resumed");
    assert!(
        paused != emulator.display().indexed_frame(),
        "resume must restart gameplay"
    );
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn counter_strike_13_loads_resources_and_enters_a_mission() {
    let mut emulator = load("LAVA1 CS 1.3/非pac/Cs(lav)1_3.lav");
    advance(&mut emulator, 3600);
    let title = emulator.display().indexed_frame().to_vec();
    capture(&emulator, "cs13-title");
    hold(&mut emulator, 13, 18);
    capture(&emulator, "cs13-after-enter");
    advance(&mut emulator, 1200);
    capture(&emulator, "cs13-setup");
    for key in [21, 21, 21, 13] {
        press(&mut emulator, key);
    }
    for _ in 0..3600 {
        if emulator.vm().pc() == 0x4ca4 {
            break;
        }
        advance(&mut emulator, 1);
    }
    assert_eq!(
        emulator.vm().pc(),
        0x4ca4,
        "mission must reach Ready confirmation"
    );
    press(&mut emulator, 13);
    for _ in 0..3600 {
        if (0x5205..0x5f1c).contains(&emulator.vm().pc()) {
            break;
        }
        advance(&mut emulator, 1);
    }
    assert!(
        (0x5205..0x5f1c).contains(&emulator.vm().pc()),
        "mission loading must finish, pc={:x}",
        emulator.vm().pc()
    );
    let mission = emulator.display().indexed_frame().to_vec();
    capture(&emulator, "cs13-mission");
    assert!(
        title != mission,
        "start must leave the title and load a mission"
    );
    let camera = u16::from_le_bytes(emulator.vm().memory()[0x2062..0x2064].try_into().unwrap());
    assert!(camera <= 140);
    hold(&mut emulator, if camera == 140 { 23 } else { 22 }, 45);
    let moved = u16::from_le_bytes(emulator.vm().memory()[0x2062..0x2064].try_into().unwrap());
    assert_ne!(camera, moved, "direction input must move the camera");
    capture(&emulator, "cs13-look");
    assert!(
        mission != emulator.display().indexed_frame(),
        "mission controls must update the view"
    );
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn crazy_f2_counts_presses_and_finishes_the_timed_round() {
    let mut emulator = load("LAVA1 疯狂F2/F2.lav");
    press(&mut emulator, b'b');
    press(&mut emulator, 0x1d);
    advance(&mut emulator, 480);
    capture(&emulator, "f2-playing");
    for _ in 0..5 {
        press(&mut emulator, 0x1d);
    }
    advance(&mut emulator, 360);
    capture(&emulator, "f2-score");
    assert!(
        matches!(
            emulator.run_frame().unwrap().status,
            FrameStatus::WaitingForInput
        ),
        "timed round must reach its score screen"
    );
    let score = emulator.vm().memory()[emulator.vm().local_bp() as usize + 5];
    assert!(score > 0, "F2 presses must be counted");
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn lost_fantasy_device_and_pc_versions_draw_and_move() {
    for (name, image) in [("LF_demo.lav", "lf-device"), ("LF_demo(PC).lav", "lf-pc")] {
        let mut emulator = load(&format!("LAVA1 lost fantasy/{name}"));
        advance(&mut emulator, 1);
        let mut idle = emulator.clone();
        hold(&mut emulator, b'd', 6);
        advance(&mut idle, 36);
        let position = |emulator: &Emulator| {
            [
                emulator.vm().memory()[0x2008],
                emulator.vm().memory()[0x2009],
                emulator.vm().memory()[0x2010],
                emulator.vm().memory()[0x2011],
            ]
        };
        assert_ne!(
            position(&emulator),
            position(&idle),
            "D must change horizontal position in {name}"
        );
        assert!(
            emulator.display().indexed_frame() != idle.display().indexed_frame(),
            "movement must appear in {name}"
        );
        capture(&emulator, image);
    }
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn speed_racing_loads_the_track_and_responds_to_steering() {
    let mut emulator = load("LAVA1 极速狂飙/极速狂飙.lav");
    advance(&mut emulator, 1);
    press(&mut emulator, 13);
    for _ in 0..300 {
        if (1..=4).contains(&u16::from_le_bytes(
            emulator.vm().memory()[0x6515..0x6517].try_into().unwrap(),
        )) {
            break;
        }
        advance(&mut emulator, 1);
    }
    let choice = u16::from_le_bytes(emulator.vm().memory()[0x6515..0x6517].try_into().unwrap());
    assert!((1..=4).contains(&choice));
    for key in std::iter::repeat_n(20, (choice - 1) as usize).chain([13]) {
        emulator.input_mut().set_keys([key]);
        advance(&mut emulator, 1);
        emulator.input_mut().set_keys([]);
        advance(&mut emulator, 1);
    }
    advance(&mut emulator, 120);
    emulator.input_mut().set_keys([13]);
    advance(&mut emulator, 1);
    emulator.input_mut().set_keys([]);
    advance(&mut emulator, 1);
    for _ in 0..300 {
        if (0x155d..0x1f68).contains(&emulator.vm().pc()) {
            break;
        }
        advance(&mut emulator, 1);
    }
    capture(&emulator, "speed-track");
    let mut idle = emulator.clone();
    hold(&mut emulator, 22, 6);
    advance(&mut idle, 36);
    capture(&emulator, "speed-steering");
    assert!(
        emulator.vm().memory()[0x200f..0x2015] != idle.vm().memory()[0x200f..0x2015],
        "steering must change the horizontal position"
    );
    assert!(
        emulator.display().indexed_frame() != idle.display().indexed_frame(),
        "steering must appear on screen"
    );
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn modified_speed_racing_steers_and_returns_from_the_score_screen() {
    let mut emulator = load("LAVA1 生死时速II/生死时速Ⅱ（修改版）.lav");
    advance(&mut emulator, 300);
    press(&mut emulator, 13);
    press(&mut emulator, b'b');
    for _ in 0..300 {
        if (0x156d..0x1936).contains(&emulator.vm().pc()) {
            break;
        }
        advance(&mut emulator, 1);
    }
    assert!((0x156d..0x1936).contains(&emulator.vm().pc()));
    capture(&emulator, "swr-track");
    let mut idle = emulator.clone();
    let base = emulator.vm().local_bp() as usize;
    hold(&mut emulator, 22, 6);
    advance(&mut idle, 36);
    assert!(
        emulator.vm().memory()[base + 7..base + 9] != idle.vm().memory()[base + 7..base + 9],
        "right input must steer the car"
    );
    for _ in 0..1800 {
        if [0x1715, 0x1774].contains(&emulator.vm().pc()) {
            break;
        }
        advance(&mut emulator, 1);
    }
    assert!(
        [0x1715, 0x1774].contains(&emulator.vm().pc()),
        "round must reach its score screen"
    );
    let score = emulator.display().indexed_frame().to_vec();
    capture(&emulator, "swr-score");
    advance(&mut emulator, 120);
    assert!(
        score == emulator.display().indexed_frame(),
        "score screen must stay stable"
    );
    emulator.input_mut().set_keys([27]);
    advance(&mut emulator, 1);
    emulator.input_mut().set_keys([]);
    advance(&mut emulator, 120);
    capture(&emulator, "swr-menu");
    assert!(
        score != emulator.display().indexed_frame(),
        "Escape must return to the menu"
    );
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn resource_manager_selects_files_and_cancels_the_dialog() {
    let mut emulator = load("LAVA1 百战天虫即时版/资源管理器.lav");
    advance(&mut emulator, 300);
    press(&mut emulator, 13);
    advance(&mut emulator, 300);
    assert_eq!(emulator.vm().pc(), 0x5ad1);
    let list = emulator.display().indexed_frame().to_vec();
    capture(&emulator, "filelist-directory");
    let saved = emulator.save_state().unwrap();
    press(&mut emulator, 21);
    assert_ne!(list, emulator.display().indexed_frame());
    press(&mut emulator, 13);
    assert_eq!(emulator.vm().pc(), 0x2772);
    capture(&emulator, "filelist-actions");
    emulator.load_state(&saved).unwrap();
    assert_eq!(list, emulator.display().indexed_frame());
    press(&mut emulator, 27);
    capture(&emulator, "filelist-cancel");
    emulator.input_mut().set_keys([13]);
    let mut halted = false;
    for _ in 0..120 {
        if matches!(emulator.run_frame().unwrap().status, FrameStatus::Halted(0)) {
            halted = true;
            break;
        }
        emulator.input_mut().set_keys([]);
    }
    assert!(halted, "confirming exit must terminate normally");
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn pet_world_legacy_versions_reopen_the_main_game_save() {
    for (folder, main, linked) in [
        ("NC2600", "宠物世界26.lav", "宠世L_2K6.lav"),
        ("NC3000", "宠物世界3K.lav", "宠世L_3K.lav"),
        ("TC1000", "宠物世界1K.lav", "宠世L_1K.lav"),
        ("TC1000S", "宠物世界1S.lav", "宠世L_1KS.lav"),
        ("TC808", "宠物世界88.lav", "宠世L_808.lav"),
    ] {
        let prefix = format!("LAVA1 宠物世界/GVmaker/{folder}");
        let mut main = load(&format!("{prefix}/{main}"));
        advance(&mut main, 300);
        press(&mut main, 23);
        for _ in 0..4 {
            press(&mut main, 22);
        }
        press(&mut main, 13);
        let saved = main.files_mut().take_dirty_files();
        assert!(saved.iter().any(|(path, _)| path.ends_with(".sav")));
        let mut linked = load(&format!("{prefix}/{linked}"));
        for (path, bytes) in saved {
            linked.files_mut().import_file(&path, bytes);
        }
        advance(&mut linked, 300);
        capture(&linked, &format!("pet-linked-{folder}"));
        let before = linked.display().indexed_frame().to_vec();
        press(&mut linked, 20);
        assert_ne!(before, linked.display().indexed_frame(), "{folder}");
        capture(&linked, &format!("pet-linked-{folder}-management"));
        press(&mut linked, 19);
        press(&mut linked, 27);
        advance(&mut linked, 300);
    }
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn magic_pac_loads_embedded_resources_and_moves_on_the_map() {
    let mut emulator = load("LAVA1 神州/神州.pac");
    assert!(emulator.files().file("/LavaData/GameSource.dat").is_some());
    for frame in 0..1800 {
        if frame >= 300 {
            let input = frame - 300;
            emulator
                .input_mut()
                .set_keys((input % 60 < 3).then_some([13, b'b'][(input / 60) % 2]));
        }
        advance(&mut emulator, 1);
    }
    emulator.input_mut().set_keys([]);
    capture(&emulator, "magic-map");
    let mut idle = emulator.clone();
    hold(&mut emulator, 22, 6);
    advance(&mut idle, 36);
    assert!(
        emulator.display().indexed_frame() != idle.display().indexed_frame(),
        "right input must visibly move the player relative to an idle replay"
    );
    capture(&emulator, "magic-movement");
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn super_robot_wars_installs_resources_and_accepts_map_input() {
    let mut installer = load("LAVA1 超级机器人大战/Setup/SRWDate.lav");
    let mut installed = false;
    for _ in 0..300 {
        if matches!(
            installer.run_frame().unwrap().status,
            FrameStatus::Halted(0)
        ) {
            installed = true;
            break;
        }
    }
    assert!(installed, "data installer must finish successfully");
    let files = installer.files_mut().take_dirty_files();
    assert!(
        files
            .iter()
            .any(|(path, bytes)| path.ends_with("srw.dat") && bytes.len() > 100_000)
    );
    let mut emulator = load("LAVA1 超级机器人大战/Setup/SRW.lav");
    for (path, bytes) in files {
        emulator.files_mut().import_file(&path, bytes);
    }
    for frame in 0..1800 {
        if frame >= 300 {
            let input = frame - 300;
            let keys = [13, 13, b'b', 13, 23, 23, 22, 20, 21, b'b', b'n', b'm'];
            emulator
                .input_mut()
                .set_keys((input % 60 < 3).then_some(keys[(input / 60) % keys.len()]));
        }
        advance(&mut emulator, 1);
    }
    emulator.input_mut().set_keys([]);
    capture(&emulator, "srw-map");
    let mut idle = emulator.clone();
    hold(&mut emulator, 22, 6);
    advance(&mut idle, 36);
    assert!(
        emulator.display().indexed_frame() != idle.display().indexed_frame(),
        "right must move the map cursor"
    );
    capture(&emulator, "srw-cursor");
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn magic_era_accepts_the_device_identifier_and_enters_gameplay() {
    let mut emulator = load("LAVA2 魔法纪元/魔法纪元.lav");
    for frame in 0..18_000 {
        if frame >= 300 {
            let input = frame - 300;
            let keys = [13, 13, b'b', 13, 13, 13, 13, 13];
            emulator
                .input_mut()
                .set_keys((input % 60 < 3).then_some(keys[(input / 60) % keys.len()]));
        }
        advance(&mut emulator, 1);
    }
    emulator.input_mut().set_keys([]);
    for frame in 0..60 {
        emulator.input_mut().set_keys((frame < 18).then_some(20));
        advance(&mut emulator, 1);
    }
    capture(&emulator, "magic-era-map");
    let mut idle = emulator.clone();
    for frame in 0..60 {
        emulator.input_mut().set_keys((frame < 18).then_some(22));
        advance(&mut emulator, 1);
        idle.input_mut().set_keys([]);
        advance(&mut idle, 1);
    }
    assert!(
        emulator.display().indexed_frame() != idle.display().indexed_frame(),
        "right must visibly move the player after the introduction"
    );
    capture(&emulator, "magic-era-movement");
}
