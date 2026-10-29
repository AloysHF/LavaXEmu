use std::path::PathBuf;

use lavaxemu_core::{Emulator, FrameStatus};

#[path = "../src/standalone/content.rs"]
#[allow(dead_code)]
mod content;

#[path = "../src/standalone/input.rs"]
#[allow(dead_code)]
mod host_input;

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

fn replay_start(emulator: &mut Emulator, frames: usize) {
    let keys = [13, 13, b'b', 13, 23, 23, 22, 20, 21, b'b', b'n', b'm'];
    for frame in 0..frames {
        if frame >= 300 {
            let input = frame - 300;
            emulator
                .input_mut()
                .set_keys((input % 60 < 3).then_some(keys[(input / 60) % keys.len()]));
        }
        advance(emulator, 1);
    }
    emulator.input_mut().set_keys([]);
}

fn input_branch(original: &Emulator, key: u8) -> Emulator {
    let mut emulator = original.clone();
    for frame in 0..60 {
        emulator.input_mut().set_keys((frame < 18).then_some(key));
        advance(&mut emulator, 1);
    }
    emulator
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
fn loose_program_mounts_matching_sibling_package_without_replacing_its_save() {
    fn package(lav: &[u8], resource: &str) -> Vec<u8> {
        let mut data = vec![0; 18];
        data[..4].copy_from_slice(b"PAC ");
        data[16..18].copy_from_slice(&3u16.to_le_bytes());
        for (name, bytes) in [
            ("LAVA/test.lav", lav),
            (resource, &[1, 2][..]),
            ("LavaData/slot.sav", &[0][..]),
        ] {
            let mut header = [0; 64];
            header[..4].copy_from_slice(&(bytes.len() as u32).to_le_bytes());
            header[4..4 + name.len()].copy_from_slice(name.as_bytes());
            data.extend_from_slice(&header);
            data.extend_from_slice(bytes);
        }
        data
    }
    let root = tempfile::tempdir().unwrap();
    let loose = root.path().join("unpacked");
    std::fs::create_dir(&loose).unwrap();
    let program = loose.join("test.lav");
    let lav = b"LAV\x12\0\0\0\0\0\0\0\0\0\0\0\0\x40";
    std::fs::write(&program, lav).unwrap();
    std::fs::write(loose.join("Slot.sav"), [9]).unwrap();
    let wrong = [lav.as_slice(), &[0x40]].concat();
    std::fs::write(
        root.path().join("00-unrelated.pac"),
        package(&wrong, "unrelated.dat"),
    )
    .unwrap();
    std::fs::write(root.path().join("01-broken.pac"), b"PAC ").unwrap();
    std::fs::write(
        root.path().join("02-matched.pac"),
        package(lav, "CBMap/stagepack.dat"),
    )
    .unwrap();
    let mut emulator = content::load_emulator(&program, &[]).unwrap();
    assert_eq!(
        emulator.files().file("/CBMap/stagepack.dat"),
        Some(&[1, 2][..])
    );
    assert_eq!(emulator.files().file("/LavaData/slot.sav"), Some(&[9][..]));
    assert!(emulator.files().file("unrelated.dat").is_none());
    let handle = emulator
        .files_mut()
        .open("/LavaData/slot.sav", "w")
        .unwrap();
    emulator.files_mut().write(handle, &[7]);
    emulator.files_mut().close(handle);
    content::flush_dirty_files(&program, emulator.files_mut()).unwrap();
    assert_eq!(std::fs::read(loose.join("Slot.sav")).unwrap(), [7]);
    assert!(!loose.join("lavadata").exists());
    assert!(!loose.join("CBMap").exists());
}

#[test]
fn standalone_loads_a_filename_in_its_working_directory() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(
        root.path().join("test.lav"),
        b"LAV\x12\0\0\0\0\0\0\0\0\0\0\0\0\x40",
    )
    .unwrap();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_lavaxemu"))
        .current_dir(root.path())
        .args(["--info", "test.lav"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "filename-only launch failed: {}",
        String::from_utf8_lossy(&output.stderr)
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
fn magic_packaged_and_loose_versions_load_resources_and_move_on_the_map() {
    for (path, name) in [
        ("LAVA1 神州/神州.pac", "magic"),
        ("LAVA1 神州/非pac/Magic.lav", "magic-loose"),
    ] {
        let mut emulator = load(path);
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
        capture(&emulator, &format!("{name}-map"));
        let mut idle = emulator.clone();
        hold(&mut emulator, 22, 6);
        advance(&mut idle, 36);
        assert!(
            emulator.display().indexed_frame() != idle.display().indexed_frame(),
            "right input must visibly move the player in {path} relative to an idle replay"
        );
        capture(&emulator, &format!("{name}-movement"));
    }
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

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn new_hero_reaches_the_village_and_scrolls_the_map() {
    let mut emulator = load("LAVA1 新英雄坛说/非pac/Hero.lav");
    replay_start(&mut emulator, 7200);
    let mut idle = emulator.clone();
    advance(&mut idle, 60);
    let moved = input_branch(&emulator, 20);
    assert!(
        moved.display().indexed_frame() != idle.display().indexed_frame(),
        "up must move the player in the village"
    );
    let scrolled = input_branch(&emulator, 21);
    assert!(
        scrolled.display().indexed_frame() != idle.display().indexed_frame(),
        "down must move the player and scroll the village"
    );
    capture(&emulator, "hero-village");
    capture(&moved, "hero-movement");
    capture(&scrolled, "hero-scroll");
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn tokimeki_reaches_the_room_and_selects_an_activity() {
    let mut emulator = load("LAVA2 心跳回忆/心跳回忆.lav");
    replay_start(&mut emulator, 7200);
    let mut idle = emulator.clone();
    advance(&mut idle, 60);
    let selected = input_branch(&emulator, 22);
    assert!(
        selected.display().indexed_frame() != idle.display().indexed_frame(),
        "right must select another room activity"
    );
    capture(&emulator, "tokimeki-room");
    capture(&selected, "tokimeki-selection");
    let mut menu = emulator.clone();
    for _ in 0..10 {
        press(&mut menu, 23);
    }
    for _ in 0..3 {
        press(&mut menu, 22);
    }
    let room = menu.display().indexed_frame().to_vec();
    press(&mut menu, 13);
    assert_ne!(
        room,
        menu.display().indexed_frame(),
        "the club/work activity menu must open"
    );
    capture(&menu, "tokimeki-activity");
    press(&mut menu, 13);
    capture(&menu, "tokimeki-return");
    assert!(
        room == menu.display().indexed_frame(),
        "selecting the ESC menu item must return to the room"
    );
    let help = input_branch(&emulator, 25);
    assert!(
        help.display().indexed_frame() != idle.display().indexed_frame(),
        "Help must explain the activity icons"
    );
    capture(&help, "tokimeki-help");
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn gamble_creates_a_local_account_and_accepts_bets() {
    let path = "LAVA1 先赌为快/先赌★为快.lav";
    let mut emulator = load(path);
    advance(&mut emulator, 300);
    for key in [13, 21, 13]
        .into_iter()
        .chain(*b"lava")
        .chain([13])
        .chain(*b"test")
        .chain([13])
        .chain(*b"test")
        .chain([13])
    {
        press(&mut emulator, key);
    }
    capture(&emulator, "gamble-registration");
    emulator.input_mut().set_keys([13]);
    let mut halted = false;
    for _ in 0..120 {
        if matches!(emulator.run_frame().unwrap().status, FrameStatus::Halted(0)) {
            halted = true;
            break;
        }
        emulator.input_mut().set_keys([]);
    }
    assert!(
        halted,
        "registration must finish and request restarting the game"
    );
    let files = emulator.files_mut().take_dirty_files();
    assert!(
        files
            .iter()
            .any(|(path, bytes)| path.ends_with("lava.dat") && !bytes.is_empty())
    );
    let mut emulator = load(path);
    for (path, bytes) in files {
        emulator.files_mut().import_file(&path, bytes);
    }
    advance(&mut emulator, 300);
    for key in [13, 13]
        .into_iter()
        .chain(*b"lava")
        .chain([13])
        .chain(*b"test")
        .chain([13])
    {
        press(&mut emulator, key);
    }
    capture(&emulator, "gamble-board");
    let board = emulator.display().indexed_frame().to_vec();
    press(&mut emulator, b'q');
    assert!(
        board != emulator.display().indexed_frame(),
        "Q must add a bet"
    );
    capture(&emulator, "gamble-bet");
    press(&mut emulator, 13);
    advance(&mut emulator, 1200);
    capture(&emulator, "gamble-round");
    press(&mut emulator, 13);
    advance(&mut emulator, 1800);
    assert_eq!(
        emulator.vm().pc(),
        0x6124,
        "the round must reach its result prompt"
    );
    capture(&emulator, "gamble-settled");
    press(&mut emulator, 13);
    let next_round = emulator.display().indexed_frame().to_vec();
    press(&mut emulator, b'w');
    assert!(
        next_round != emulator.display().indexed_frame(),
        "another bet must be possible after the round"
    );
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn shushan_registers_a_character_and_enters_the_game() {
    let mut registration = load("LAVA1 蜀山群侠传/蜀山注册.lav");
    advance(&mut registration, 300);
    for key in [13]
        .into_iter()
        .chain(*b"lava")
        .chain([13])
        .chain(*b"test")
        .chain([13, b'y'])
    {
        press(&mut registration, key);
    }
    capture(&registration, "shushan-registration");
    let files = registration.files_mut().take_dirty_files();
    assert!(
        files
            .iter()
            .any(|(path, bytes)| path.ends_with("m-lava.dat") && !bytes.is_empty())
    );
    let mut emulator = load("LAVA1 蜀山群侠传/蜀山群侠传.lav");
    for (path, bytes) in files {
        emulator.files_mut().import_file(&path, bytes);
    }
    advance(&mut emulator, 1200);
    for key in [13, 13]
        .into_iter()
        .chain(*b"lava")
        .chain([13])
        .chain(*b"test")
        .chain([13, 13])
    {
        press(&mut emulator, key);
    }
    for _ in 0..15 {
        press(&mut emulator, 13);
    }
    capture(&emulator, "shushan-map");
    let mut idle = emulator.clone();
    advance(&mut idle, 60);
    let moved = input_branch(&emulator, 20);
    assert!(
        moved.display().indexed_frame() != idle.display().indexed_frame(),
        "up must move the registered character"
    );
    capture(&moved, "shushan-movement");
    let map = emulator.display().indexed_frame().to_vec();
    press(&mut emulator, 30);
    assert!(
        map != emulator.display().indexed_frame(),
        "F3 must open character status"
    );
    capture(&emulator, "shushan-status");
    press(&mut emulator, 27);
    assert!(
        map == emulator.display().indexed_frame(),
        "closing status must restore the map"
    );
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn my_world_creates_a_character_and_moves_on_the_map() {
    let mut emulator = load("LAVA1 我的世界~霸王屎/我的世界.lav");
    let keys = [13, b'l', b'a', b'v', b'a', 13, 13, 13, 13, 13];
    for frame in 0..3600 {
        if frame >= 900 {
            let input = frame - 900;
            emulator.input_mut().set_keys(
                (input % 60 < 3 && input / 60 < keys.len())
                    .then_some(keys[(input / 60) % keys.len()]),
            );
        }
        advance(&mut emulator, 1);
    }
    emulator.input_mut().set_keys([]);
    let mut idle = emulator.clone();
    advance(&mut idle, 60);
    let moved = input_branch(&emulator, 20);
    assert!(
        moved.display().indexed_frame() != idle.display().indexed_frame(),
        "up must move the created character"
    );
    capture(&emulator, "my-world-map");
    capture(&moved, "my-world-movement");
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn holiday_island_finishes_the_intro_talks_and_moves_on_the_map() {
    let mut emulator = load("LAVA1 假日神秘岛/假日神秘岛 abcdef.lav");
    replay_start(&mut emulator, 18_000);
    capture(&emulator, "holiday-island-map");
    let dialogue = input_branch(&emulator, 22);
    let mut idle = emulator.clone();
    advance(&mut idle, 60);
    assert!(
        dialogue.display().indexed_frame() != idle.display().indexed_frame(),
        "right must trigger the nearby conversation"
    );
    capture(&dialogue, "holiday-island-dialogue");
    for _ in 0..12 {
        emulator.input_mut().set_keys([22]);
        advance(&mut emulator, 3);
        emulator.input_mut().set_keys([]);
        advance(&mut emulator, 57);
    }
    advance(&mut emulator, 2880);
    let mut idle = emulator.clone();
    advance(&mut idle, 60);
    let moved = input_branch(&emulator, 23);
    assert!(
        moved.display().indexed_frame() != idle.display().indexed_frame(),
        "left must move the player after the conversation"
    );
    capture(&emulator, "holiday-island-path");
    capture(&moved, "holiday-island-movement");
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn crazy_ball_packaged_and_loose_versions_load_the_stage_and_move_the_paddle() {
    for (path, name) in [
        ("LAVA1 疯狂弹球/疯狂弹球.pac", "crazy-ball-pac"),
        ("LAVA1 疯狂弹球/非pac/crazyball.lav", "crazy-ball-loose"),
    ] {
        let mut emulator = load(path);
        assert!(emulator.files().file("/CBMap/stagepack.dat").is_some());
        replay_start(&mut emulator, 1200);
        capture(&emulator, &format!("{name}-ready"));
        press(&mut emulator, 13);
        capture(&emulator, &format!("{name}-launch"));
        let mut idle = emulator.clone();
        advance(&mut idle, 36);
        hold(&mut emulator, 23, 6);
        capture(&emulator, &format!("{name}-left"));
        capture(&idle, &format!("{name}-idle"));
        assert!(
            emulator.display().indexed_frame() != idle.display().indexed_frame(),
            "left must move the paddle in {path}"
        );
    }
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn planting_gold_moves_a_coin_and_restarts_the_level() {
    let mut emulator = load("LAVA1 种金币/种金币.lav");
    advance(&mut emulator, 300);
    press(&mut emulator, 13);
    press(&mut emulator, 13);
    advance(&mut emulator, 300);
    let board = emulator.display().indexed_frame().to_vec();
    capture(&emulator, "planting-gold-board");
    press(&mut emulator, b'd');
    assert!(
        board != emulator.display().indexed_frame(),
        "D must move a coin and update the step count"
    );
    capture(&emulator, "planting-gold-movement");
    press(&mut emulator, 28);
    advance(&mut emulator, 120);
    capture(&emulator, "planting-gold-reset");
    assert!(
        board == emulator.display().indexed_frame(),
        "F1 must restore the initial level layout and step count"
    );
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn sky_and_land_creates_a_local_save_and_moves_on_the_map() {
    let mut emulator = load("LAVA1 幕天席地1.1/幕天席地.lav");
    advance(&mut emulator, 300);
    for key in [13, 13, 20, 13] {
        hold(&mut emulator, key, 18);
        advance(&mut emulator, 12);
    }
    for key in b"ylava"
        .iter()
        .copied()
        .chain([13])
        .chain(*b"test")
        .chain([13])
    {
        press(&mut emulator, key);
    }
    advance(&mut emulator, 600);
    capture(&emulator, "sky-land-map");
    // The footer marquee advances independently of the saved map.
    let viewport = usize::from(emulator.display().width()) * 64;
    let initial_map = emulator.display().indexed_frame()[..viewport].to_vec();
    let mut idle = emulator.clone();
    advance(&mut idle, 60);
    let moved = input_branch(&emulator, 20);
    capture(&moved, "sky-land-movement");
    assert!(
        moved.display().indexed_frame() != idle.display().indexed_frame(),
        "up must move the new character"
    );
    hold(&mut emulator, 25, 18);
    capture(&emulator, "sky-land-system");
    press(&mut emulator, 13);
    capture(&emulator, "sky-land-save");
    let files = emulator.files_mut().take_dirty_files();
    assert!(
        files
            .iter()
            .any(|(path, bytes)| path.ends_with("lava.sls") && bytes.len() == 88),
        "save progress must write the local account file"
    );
    let mut reloaded = load("LAVA1 幕天席地1.1/幕天席地.lav");
    for (path, bytes) in files {
        reloaded.files_mut().import_file(&path, bytes);
    }
    advance(&mut reloaded, 300);
    for key in [13, 13, 21, 13] {
        hold(&mut reloaded, key, 18);
        advance(&mut reloaded, 12);
    }
    for key in b"lava"
        .iter()
        .copied()
        .chain([13])
        .chain(*b"test")
        .chain([13])
    {
        press(&mut reloaded, key);
    }
    advance(&mut reloaded, 600);
    capture(&reloaded, "sky-land-reloaded");
    assert!(
        initial_map == reloaded.display().indexed_frame()[..viewport],
        "logging into the written account must restore the saved map"
    );
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn star_legend_registers_a_character_and_loads_the_map() {
    let directory = "LAVA1 星剑传奇/未打包的[星剑传奇]";
    let mut registration = load(&format!("{directory}/星剑注册.lav"));
    advance(&mut registration, 300);
    for key in [13]
        .into_iter()
        .chain(*b"lavaxx")
        .chain([13])
        .chain(*b"testxx")
        .chain([13, b'y', 13])
        .chain(*b"lava")
        .chain([13])
    {
        press(&mut registration, key);
    }
    capture(&registration, "star-legend-registration");
    let files = registration.files_mut().take_dirty_files();
    assert!(
        files
            .iter()
            .any(|(path, bytes)| path.ends_with("s-lavaxx.dat") && bytes.len() == 194)
    );
    let mut emulator = load(&format!("{directory}/星剑传奇.lav"));
    for (path, bytes) in files {
        emulator.files_mut().import_file(&path, bytes);
    }
    advance(&mut emulator, 600);
    capture(&emulator, "star-legend-start");
    press(&mut emulator, 13);
    capture(&emulator, "star-legend-login");
    for key in b"lavaxx"
        .iter()
        .copied()
        .chain([13])
        .chain(*b"testxx")
        .chain([13])
    {
        press(&mut emulator, key);
    }
    advance(&mut emulator, 1200);
    capture(&emulator, "star-legend-character");
    press(&mut emulator, 13);
    advance(&mut emulator, 600);
    capture(&emulator, "star-legend-map");
    let mut idle = emulator.clone();
    advance(&mut idle, 60);
    let moved = input_branch(&emulator, 20);
    capture(&moved, "star-legend-movement");
    assert!(
        moved.display().indexed_frame() != idle.display().indexed_frame(),
        "up must move the registered character on the map"
    );
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn flat_cube_selects_shifts_swaps_and_returns_to_the_menu() {
    for (path, name) in [
        ("LAVA1 平面魔方/平面魔方.pac", "flat-cube-pac"),
        ("LAVA1 平面魔方/非pac/平面魔方.lav", "flat-cube-loose"),
    ] {
        let mut emulator = load(path);
        advance(&mut emulator, 300);
        press(&mut emulator, 22);
        press(&mut emulator, 13);
        capture(&emulator, &format!("{name}-board"));
        let board = emulator.clone();
        press(&mut emulator, 22);
        assert!(
            board.display().indexed_frame() != emulator.display().indexed_frame(),
            "right must move the selection in {path}"
        );
        let mut shifted = board.clone();
        press(&mut shifted, b'm');
        press(&mut shifted, 22);
        capture(&shifted, &format!("{name}-shift"));
        assert!(
            board.display().indexed_frame() != shifted.display().indexed_frame(),
            "M then right must shift a row and increment the move count in {path}"
        );
        let mut swapped = board.clone();
        press(&mut swapped, b'c');
        press(&mut swapped, 22);
        press(&mut swapped, 13);
        capture(&swapped, &format!("{name}-swap"));
        assert!(
            board.display().indexed_frame() != swapped.display().indexed_frame(),
            "C then right and Enter must swap tiles and consume an exchange in {path}"
        );
        press(&mut swapped, 27);
        capture(&swapped, &format!("{name}-exit-confirmation"));
        press(&mut swapped, b'y');
        capture(&swapped, &format!("{name}-menu"));
        assert!(
            board.display().indexed_frame() != swapped.display().indexed_frame(),
            "confirming exit must return to the menu in {path}"
        );
    }
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn rocket_car_keeps_the_track_visible_and_responds_to_controls() {
    let mut emulator = load("LAVA1 火箭车/火箭车.lav");
    advance(&mut emulator, 300);
    let title = emulator.display().indexed_frame().to_vec();
    press(&mut emulator, 13);
    advance(&mut emulator, 600);
    let frame = emulator.display().indexed_frame();
    let track_pixels = (0..80)
        .filter(|&y| !(16..64).contains(&y))
        .flat_map(|y| &frame[y * 160..y * 160 + 120])
        .filter(|&&pixel| pixel != 0)
        .count();
    assert!(
        track_pixels > 200,
        "direct graphics access must preserve the moving track traffic"
    );
    capture(&emulator, "rocket-car-track");
    let mut idle = emulator.clone();
    advance(&mut idle, 60);
    let steered = input_branch(&emulator, 20);
    capture(&steered, "rocket-car-steering");
    assert!(
        steered.display().indexed_frame() != idle.display().indexed_frame(),
        "up must move the player's car relative to idle traffic"
    );
    press(&mut emulator, 27);
    capture(&emulator, "rocket-car-score");
    for key in b"lava" {
        press(&mut emulator, *key);
    }
    press(&mut emulator, 13);
    press(&mut emulator, 13);
    capture(&emulator, "rocket-car-menu");
    assert!(
        title == emulator.display().indexed_frame(),
        "entering the driver name must return to the title menu"
    );
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn lazy_yy_versions_feed_pause_and_return_to_the_menu() {
    for (path, name, feed, resume) in [
        (
            "LAVA1 Lazy YY牛奶大战/Ver1.2/未打包的文件/LazyYY.lav",
            "lazy-yy-loose",
            b'g',
            32,
        ),
        (
            "LAVA1 Lazy YY牛奶大战/Ver1.2/打包的文件/LazyYY牛奶大战.pac",
            "lazy-yy-pac",
            b'g',
            32,
        ),
        (
            "LAVA1 Lazy YY牛奶大战/Ver1.5 For TC800 Only/Lazy YY牛奶大战.pac",
            "lazy-yy-grey",
            23,
            13,
        ),
    ] {
        let mut emulator = load(path);
        for frame in 0usize..1200 {
            let input = frame.saturating_sub(300);
            emulator
                .input_mut()
                .set_keys((frame >= 300 && input / 60 < 5 && input % 60 < 3).then_some(13));
            advance(&mut emulator, 1);
        }
        emulator.input_mut().set_keys([]);
        capture(&emulator, &format!("{name}-play"));
        let mut idle = emulator.clone();
        advance(&mut idle, 60);
        let fed = input_branch(&emulator, feed);
        capture(&fed, &format!("{name}-feed"));
        assert!(
            fed.display().indexed_frame() != idle.display().indexed_frame(),
            "feeding must affect the milk/targets in {path}"
        );
        let mut paused = emulator.clone();
        for frame in 0..60 {
            paused
                .input_mut()
                .set_keys(host_input::keyboard_keys(|key| {
                    frame < 18 && key == minifb::Key::Space
                }));
            advance(&mut paused, 1);
        }
        capture(&paused, &format!("{name}-pause"));
        let pause_frame = paused.display().indexed_frame().to_vec();
        advance(&mut paused, 60);
        assert!(
            pause_frame == paused.display().indexed_frame(),
            "pause must freeze the scene in {path}"
        );
        press(&mut paused, resume);
        assert!(
            pause_frame != paused.display().indexed_frame(),
            "resume must restore play in {path}"
        );
        press(&mut emulator, 27);
        capture(&emulator, &format!("{name}-menu"));
    }
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn extreme_landlord_versions_deal_select_play_and_save() {
    fn step_key(emulator: &mut Emulator, key: u8) {
        press(emulator, key);
        advance(emulator, 27);
    }
    for (path, name, first_run, pinyin) in [
        (
            "LAVA1 极限斗地主/极限斗地主.lav",
            "landlord-existing",
            false,
            false,
        ),
        (
            "LAVA1 极限斗地主3.2/极限斗地主.lav",
            "landlord-standard",
            true,
            false,
        ),
        (
            "LAVA1 极限斗地主3.2/极限斗地主NC1020版.lav",
            "landlord-nc1020",
            true,
            false,
        ),
        (
            "LAVA1 极限斗地主3.2/极限斗地主tc800版.lav",
            "landlord-tc800",
            true,
            true,
        ),
    ] {
        let mut emulator = load(path);
        advance(&mut emulator, 300);
        if first_run {
            for key in [21, 21, 21, b'y'] {
                step_key(&mut emulator, key);
            }
        }
        for _ in 0..8 {
            step_key(&mut emulator, 13);
        }
        if first_run {
            if pinyin {
                step_key(&mut emulator, 26);
            }
            for key in b"lava".iter().copied().chain([13, 13, 13, 13]) {
                step_key(&mut emulator, key);
            }
        }
        advance(&mut emulator, 600);
        capture(&emulator, &format!("{name}-dealt"));
        let mut selection = emulator.clone();
        press(&mut selection, 22);
        let cursor = selection.display().indexed_frame().to_vec();
        press(&mut selection, 20);
        capture(&selection, &format!("{name}-selected"));
        assert!(
            cursor != selection.display().indexed_frame(),
            "up must raise a selected card in {path}"
        );
        press(&mut selection, 21);
        assert!(
            cursor == selection.display().indexed_frame(),
            "down must cancel that selection in {path}"
        );
        let hand = emulator.display().indexed_frame()[60 * 160..].to_vec();
        press(&mut emulator, b'p');
        press(&mut emulator, 13);
        advance(&mut emulator, 180);
        capture(&emulator, &format!("{name}-played"));
        assert!(
            hand != emulator.display().indexed_frame()[60 * 160..],
            "confirming the suggested play must change the hand in {path}"
        );
        emulator.files_mut().take_dirty_files();
        for key in [31, b'y', 13] {
            step_key(&mut emulator, key);
        }
        capture(&emulator, &format!("{name}-scores"));
        assert!(
            emulator
                .files_mut()
                .take_dirty_files()
                .iter()
                .any(|(path, bytes)| path.contains("jxddzsave") && !bytes.is_empty()),
            "ending the round must write the updated score file in {path}"
        );
    }
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn champion_football_manager_creates_a_coach_and_finishes_a_match() {
    fn step_key(emulator: &mut Emulator, key: u8) {
        press(emulator, key);
        advance(emulator, 27);
    }
    let mut emulator = load("LAVA1 冠军足球经理05-06赛季0.7/CFM.lav");
    advance(&mut emulator, 300);
    for key in [13]
        .into_iter()
        .chain(*b"lava")
        .chain([13])
        .chain(*b"test")
        .chain([13])
        .chain(*b"test")
        .chain([13, 13, 13])
    {
        step_key(&mut emulator, key);
    }
    advance(&mut emulator, 420);
    for key in [21, 13]
        .into_iter()
        .chain([21; 19])
        .chain([13, 13, 13, 13, 27, 13, 13, 13, 13, 13])
        .chain([27; 6])
    {
        step_key(&mut emulator, key);
    }
    capture(&emulator, "cfm-manager");
    let manager = emulator.display().indexed_frame().to_vec();
    let mut training = emulator.clone();
    press(&mut training, 21);
    press(&mut training, 13);
    capture(&training, "cfm-training");
    assert!(
        manager != training.display().indexed_frame(),
        "training arrangements must open from the manager menu"
    );
    for _ in 0..5 {
        step_key(&mut emulator, 13);
    }
    advance(&mut emulator, 2880);
    capture(&emulator, "cfm-results");
    for _ in 0..5 {
        step_key(&mut emulator, 27);
    }
    capture(&emulator, "cfm-next-fixture");
    let saved_manager = emulator.display().indexed_frame().to_vec();
    assert!(
        manager != emulator.display().indexed_frame(),
        "a completed match must advance the manager's fixture and league record"
    );
    let mut statistics = emulator.clone();
    for key in [21, 21, 13, 13] {
        step_key(&mut statistics, key);
    }
    capture(&statistics, "cfm-standings");
    for _ in 0..4 {
        step_key(&mut emulator, 21);
    }
    emulator.files_mut().take_dirty_files();
    emulator.input_mut().set_keys([13]);
    for _ in 0..60 {
        let frame = emulator
            .run_frame()
            .expect("saving the career must execute");
        if let FrameStatus::Halted(code) = frame.status {
            assert_eq!(code, 0);
            break;
        }
    }
    assert!(
        !emulator.vm().is_running(),
        "save and leave must exit normally"
    );
    capture(&emulator, "cfm-save-return");
    let files = emulator.files_mut().take_dirty_files();
    assert!(
        files
            .iter()
            .any(|(path, bytes)| path.ends_with("mdata0.sav") && bytes.len() == 4014),
        "leaving must save the played career"
    );
    let mut reloaded = load("LAVA1 冠军足球经理05-06赛季0.7/CFM.lav");
    for (path, bytes) in files {
        reloaded.files_mut().import_file(&path, bytes);
    }
    advance(&mut reloaded, 300);
    for key in [21, 13, 13].into_iter().chain(*b"test").chain([13]) {
        step_key(&mut reloaded, key);
    }
    advance(&mut reloaded, 300);
    capture(&reloaded, "cfm-reloaded");
    assert!(
        saved_manager == reloaded.display().indexed_frame(),
        "loading the career must restore its next fixture and league record"
    );
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn princess_maker_2_shows_stats_executes_a_plan_and_reloads_its_save() {
    fn step_key(emulator: &mut Emulator, key: u8) {
        press(emulator, key);
        advance(emulator, 27);
    }
    let path = "LAVA1 美少女梦工厂2黑白版/PM2.lav";
    let mut emulator = load(path);
    advance(&mut emulator, 300);
    for key in [13, 13, 13, b'a', 13, b'y', 13] {
        step_key(&mut emulator, key);
    }
    advance(&mut emulator, 480);
    for _ in 0..100 {
        step_key(&mut emulator, 13);
    }
    capture(&emulator, "pm2-main");
    let status = input_branch(&emulator, b'z');
    capture(&status, "pm2-status");
    assert!(
        status.display().indexed_frame() != emulator.display().indexed_frame(),
        "Z must open the character's attributes"
    );
    let mut working = emulator.clone();
    for key in [b'c', 13, 13, 13, 13] {
        step_key(&mut working, key);
    }
    advance(&mut working, 2100);
    capture(&working, "pm2-housework");
    assert!(
        working.display().indexed_frame() != emulator.display().indexed_frame(),
        "executing housework must advance the date and character state"
    );
    step_key(&mut emulator, b'x');
    step_key(&mut emulator, 13);
    capture(&emulator, "pm2-saved");
    let saved = emulator.display().indexed_frame().to_vec();
    let files = emulator.files_mut().take_dirty_files();
    assert!(
        files
            .iter()
            .any(|(path, bytes)| path.ends_with("save.dat") && !bytes.is_empty()),
        "time stop must write the character save"
    );
    let mut reloaded = load(path);
    for (path, bytes) in files {
        reloaded.files_mut().import_file(&path, bytes);
    }
    advance(&mut reloaded, 300);
    step_key(&mut reloaded, 21);
    step_key(&mut reloaded, 13);
    advance(&mut reloaded, 60);
    capture(&reloaded, "pm2-reloaded");
    assert!(
        saved == reloaded.display().indexed_frame(),
        "loading progress must restore the saved character screen"
    );
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn dream_world_creates_an_account_loads_the_map_and_moves() {
    fn step_key(emulator: &mut Emulator, key: u8) {
        press(emulator, key);
        advance(emulator, 27);
    }
    let path = "LAVA1 梦中世界（更新6）/梦中世界.lav";
    let mut registration = load(path);
    advance(&mut registration, 300);
    for key in [21, 21, 13]
        .into_iter()
        .chain(*b"lava")
        .chain([21])
        .chain(*b"test")
        .chain([21])
        .chain(*b"test")
        .chain([13])
        .chain(*b"hero")
        .chain([13, b'a'])
    {
        step_key(&mut registration, key);
    }
    capture(&registration, "dream-world-registration");
    let files = registration.files_mut().take_dirty_files();
    assert!(
        files
            .iter()
            .any(|(path, bytes)| path.ends_with("lava.dmn") && !bytes.is_empty()),
        "account creation must write the character file"
    );
    let mut emulator = load(path);
    for (path, bytes) in files {
        emulator.files_mut().import_file(&path, bytes);
    }
    advance(&mut emulator, 300);
    for key in [13]
        .into_iter()
        .chain(*b"lava")
        .chain([21])
        .chain(*b"test")
        .chain([13])
    {
        step_key(&mut emulator, key);
    }
    advance(&mut emulator, 600);
    capture(&emulator, "dream-world-map");
    let mut idle = emulator.clone();
    advance(&mut idle, 60);
    let mut moved = input_branch(&emulator, 21);
    capture(&moved, "dream-world-movement");
    assert!(
        moved.display().indexed_frame() != idle.display().indexed_frame(),
        "down must move the player and scroll the map"
    );
    let role = input_branch(&emulator, 28);
    capture(&role, "dream-world-character");
    assert!(
        role.display().indexed_frame() != idle.display().indexed_frame(),
        "F1 must display the character record"
    );
    press(&mut moved, 32);
    capture(&moved, "dream-world-system");
    moved.input_mut().set_keys(*b"c");
    for _ in 0..60 {
        if let FrameStatus::Halted(code) = moved.run_frame().unwrap().status {
            assert_eq!(code, 0);
            break;
        }
    }
    capture(&moved, "dream-world-exit");
    assert!(
        !moved.vm().is_running(),
        "the system menu must exit normally"
    );
    assert!(
        moved
            .files_mut()
            .take_dirty_files()
            .iter()
            .any(|(path, bytes)| path.ends_with("lava.dmn") && bytes.len() == 11357),
        "leaving must save the played character"
    );
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn dragon_legend_selects_classes_moves_and_reloads_a_local_save() {
    fn step_key(emulator: &mut Emulator, key: u8) {
        press(emulator, key);
        advance(emulator, 27);
    }
    let path = "LAVA1 龙之传说/非pac/longX.lav";
    let mut emulator = load(path);
    advance(&mut emulator, 300);
    for key in [13, 13, 13, b'q', b'y', b'w', b'y', 13, 13, 13, 13] {
        step_key(&mut emulator, key);
    }
    advance(&mut emulator, 240);
    for _ in 0..80 {
        step_key(&mut emulator, 13);
    }
    capture(&emulator, "dragon-legend-map");
    let mut idle = emulator.clone();
    advance(&mut idle, 60);
    let moved = input_branch(&emulator, 21);
    capture(&moved, "dragon-legend-movement");
    assert!(
        moved.display().indexed_frame() != idle.display().indexed_frame(),
        "down must move the party through the town"
    );
    let mut character = input_branch(&emulator, b'c');
    press(&mut character, 13);
    capture(&character, "dragon-legend-character");
    assert!(
        character.display().indexed_frame() != idle.display().indexed_frame(),
        "C must open the party's character record"
    );
    let mut battle = emulator.clone();
    advance(&mut battle, 1200);
    for _ in 0..6 {
        battle.input_mut().set_keys([23]);
        advance(&mut battle, 1);
        battle.input_mut().set_keys([]);
        advance(&mut battle, 11);
    }
    for key in [13]
        .into_iter()
        .chain([23; 30])
        .chain([22; 30])
        .chain([23; 30])
        .chain([22; 30])
    {
        step_key(&mut battle, key);
    }
    advance(&mut battle, 2340);
    capture(&battle, "dragon-legend-battle");
    let ready = battle.display().indexed_frame().to_vec();
    let mut attacking = battle.clone();
    step_key(&mut attacking, b'b');
    capture(&attacking, "dragon-legend-target");
    step_key(&mut attacking, 13);
    capture(&attacking, "dragon-legend-attack");
    assert!(
        ready != attacking.display().indexed_frame(),
        "B and Enter must resolve an ordinary attack"
    );
    for _ in 0..10 {
        step_key(&mut battle, b'h');
        step_key(&mut battle, 13);
    }
    advance(&mut battle, 1200);
    capture(&battle, "dragon-legend-escaped");
    // Coordinate bytes distinguish world movement from battle menu input.
    let position = battle.vm().memory()[0x65d1..0x65d3].to_vec();
    let walking = input_branch(&battle, 21);
    assert!(
        position != walking.vm().memory()[0x65d1..0x65d3],
        "successful escape must restore world-map movement"
    );
    for key in [b'b', 21, 21, 21, 21, 21, 13, 13] {
        step_key(&mut emulator, key);
    }
    capture(&emulator, "dragon-legend-system");
    let files = emulator.files_mut().take_dirty_files();
    assert!(
        files
            .iter()
            .any(|(path, bytes)| path.ends_with("lsav.dat") && !bytes.is_empty()),
        "the system save action must write party progress"
    );
    step_key(&mut emulator, 27);
    step_key(&mut emulator, 27);
    capture(&emulator, "dragon-legend-saved-map");
    let saved = emulator.display().indexed_frame()[..64 * 160].to_vec();
    let mut reloaded = load(path);
    for (path, bytes) in files {
        reloaded.files_mut().import_file(&path, bytes);
    }
    advance(&mut reloaded, 300);
    step_key(&mut reloaded, b'c');
    step_key(&mut reloaded, 13);
    advance(&mut reloaded, 600);
    capture(&reloaded, "dragon-legend-reloaded");
    assert!(
        saved == reloaded.display().indexed_frame()[..64 * 160],
        "continue must restore the saved town viewport"
    );
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn dragon_kiss_creates_a_chinese_name_moves_and_loads_party_progress() {
    fn step_key(emulator: &mut Emulator, key: u8) {
        press(emulator, key);
        advance(emulator, 27);
    }
    let path = "LAVA1 龙之吻体验版/l2体验版.lav";
    let mut emulator = load(path);
    advance(&mut emulator, 300);
    for key in [b'n', 23, b'w', b'e', b'n', 32] {
        step_key(&mut emulator, key);
    }
    capture(&emulator, "dragon-kiss-pinyin");
    for key in [13, 13, b'q', b'y', 13, b'w', b'y', 13, 13, 13, 13] {
        step_key(&mut emulator, key);
    }
    advance(&mut emulator, 180);
    for _ in 0..80 {
        step_key(&mut emulator, 13);
    }
    advance(&mut emulator, 5760);
    capture(&emulator, "dragon-kiss-town");
    let mut idle = emulator.clone();
    advance(&mut idle, 60);
    let moved = input_branch(&emulator, 22);
    capture(&moved, "dragon-kiss-movement");
    assert!(
        (64..80).any(|row| {
            let coordinates = row * 160..row * 160 + 32;
            moved.display().indexed_frame()[coordinates.clone()]
                != idle.display().indexed_frame()[coordinates]
        }),
        "right must move the named character through town"
    );
    let mut character = input_branch(&emulator, b'c');
    press(&mut character, 13);
    capture(&character, "dragon-kiss-character");
    let attributes = character.display().indexed_frame().to_vec();
    for key in [b'b', 21, 21, 21, 21, 21, 13, 13] {
        step_key(&mut emulator, key);
    }
    capture(&emulator, "dragon-kiss-save-menu");
    let files = emulator.files_mut().take_dirty_files();
    assert!(
        files
            .iter()
            .any(|(path, bytes)| path.ends_with("l2sav.dat") && !bytes.is_empty()),
        "save must write the demo's party file"
    );
    let mut reloaded = load(path);
    for (path, bytes) in files {
        reloaded.files_mut().import_file(&path, bytes);
    }
    advance(&mut reloaded, 300);
    for key in [b'c', 13] {
        step_key(&mut reloaded, key);
    }
    advance(&mut reloaded, 600);
    capture(&reloaded, "dragon-kiss-reloaded-town");
    for key in [b'c', 13] {
        step_key(&mut reloaded, key);
    }
    capture(&reloaded, "dragon-kiss-reloaded-character");
    assert!(
        attributes == reloaded.display().indexed_frame(),
        "loading must preserve the Chinese name, class and party attributes"
    );
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn maze_3d_walks_turns_and_switches_the_satellite_map() {
    fn viewport(emulator: &Emulator) -> &[u8] {
        &emulator.display().indexed_frame()[14 * 160..]
    }
    let mut emulator = load("LAVA1 3D迷宫/3D迷宫.lav");
    advance(&mut emulator, 300);
    for _ in 0..20 {
        press(&mut emulator, 13);
        advance(&mut emulator, 27);
    }
    press(&mut emulator, 27);
    advance(&mut emulator, 5700);
    capture(&emulator, "maze-3d-corridor");
    let mut idle = emulator.clone();
    advance(&mut idle, 60);
    let moved = input_branch(&emulator, 20);
    capture(&moved, "maze-3d-movement");
    assert!(
        viewport(&moved) != viewport(&idle),
        "Up must change the corridor geometry"
    );
    let turned = input_branch(&emulator, 19);
    capture(&turned, "maze-3d-turn");
    assert!(
        (0..14).any(|row| {
            let direction = row * 160..row * 160 + 48;
            turned.display().indexed_frame()[direction.clone()]
                != idle.display().indexed_frame()[direction]
        }),
        "Page Up must turn the direction indicator"
    );
    let mut map = input_branch(&emulator, 13);
    capture(&map, "maze-3d-satellite");
    assert!(
        viewport(&map) != viewport(&idle),
        "Enter must open the satellite maze map"
    );
    press(&mut map, 27);
    capture(&map, "maze-3d-returned");
    assert!(
        viewport(&map) == viewport(&idle),
        "Escape must return to the same corridor"
    );
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn pacman_versions_select_a_stage_move_and_finish_a_round() {
    for path in ["LAVA1 PACMAN/非pac/PACMAN.lav", "LAVA1 PACMAN/PACMAN.pac"] {
        let mut emulator = load(path);
        advance(&mut emulator, 300);
        press(&mut emulator, 13);
        capture(&emulator, "pacman-stage-selector");
        // The supplied progress file unlocks only stage one.
        press(&mut emulator, 13);
        capture(&emulator, "pacman-maze");
        let mut idle = emulator.clone();
        advance(&mut idle, 18);
        for key in [23, 22, 21] {
            let mut moving = emulator.clone();
            moving.input_mut().set_keys([key]);
            advance(&mut moving, 18);
            capture(&moving, &format!("pacman-move-{key}"));
            assert!(
                moving.display().indexed_frame() != idle.display().indexed_frame(),
                "directional movement must differ from the same ghost simulation without input"
            );
        }
        let mut ended = false;
        for _ in 0..600 {
            let result = emulator
                .run_frame()
                .expect("the round must execute without errors");
            if let FrameStatus::Halted(code) = result.status {
                assert_eq!(code, 0, "losing must finish normally");
                ended = true;
                break;
            }
        }
        capture(&emulator, "pacman-round-ended");
        assert!(ended, "the idle player must reach the You Lost result");
    }
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn cloud_versions_configure_keys_jump_and_persist_the_configuration() {
    for path in [
        "LAVA1 Cloud/未打包的文件/cloud_all.lav",
        "LAVA1 Cloud/打包的文件/bobo_cloud.pac",
        "LAVA1 疾龙云飞/未打包的文件/cloud_all.lav",
        "LAVA1 疾龙云飞/打包的文件/bobo_cloud.pac",
    ] {
        let mut emulator = load(path);
        advance(&mut emulator, 300);
        // Configure Enter for jumping and B for acceleration through the UI.
        for key in [21, 13, 13, b'b', 27, 20, 13, 13, 13] {
            press(&mut emulator, key);
            advance(&mut emulator, 27);
        }
        advance(&mut emulator, 660);
        capture(&emulator, "cloud-ready");
        let files = emulator.files_mut().take_dirty_files();
        assert!(
            files.iter().any(|(path, bytes)| {
                path.ends_with("cloud_cfg.dat") && bytes.starts_with(&[13, b'b'])
            }),
            "keyboard settings must be saved"
        );
        let mut idle = emulator.clone();
        press(&mut idle, 13);
        advance(&mut idle, 327);
        capture(&idle, "cloud-missed-jump");
        press(&mut emulator, 13);
        advance(&mut emulator, 37);
        press(&mut emulator, 13);
        capture(&emulator, "cloud-jump");
        advance(&mut emulator, 257);
        capture(&emulator, "cloud-scored-jump");
        let scored = emulator.display().indexed_frame().to_vec();
        assert!(
            scored != idle.display().indexed_frame(),
            "a timed jump must earn a different result from falling"
        );
        let mut reloaded = load(path);
        for (path, bytes) in files {
            reloaded.files_mut().import_file(&path, bytes);
        }
        advance(&mut reloaded, 300);
        press(&mut reloaded, 13);
        advance(&mut reloaded, 27);
        press(&mut reloaded, 13);
        advance(&mut reloaded, 37);
        press(&mut reloaded, 13);
        advance(&mut reloaded, 257);
        capture(&reloaded, "cloud-reloaded-result");
        assert!(
            scored == reloaded.display().indexed_frame(),
            "reloading the actual configuration must preserve the timed jump result"
        );
    }
}
