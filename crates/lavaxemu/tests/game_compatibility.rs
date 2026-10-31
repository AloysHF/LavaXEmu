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
    let emulator =
        content::load_emulator(&root.join(relative), &[]).expect("load game and companions");
    if let Some(path) = std::env::var_os("LAVAX_AUDIT_LOG") {
        use std::io::Write;
        let mut log = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .unwrap();
        writeln!(log, "{}", relative.replace('\\', "/")).unwrap();
    }
    emulator
}

fn fixture_path(filename: &str) -> String {
    let root = PathBuf::from(std::env::var_os("LAVAX_GAME_ROOT").expect("set LAVAX_GAME_ROOT"));
    let mut directories = vec![root.clone()];
    let mut matches = Vec::new();
    while let Some(directory) = directories.pop() {
        for entry in std::fs::read_dir(directory).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                directories.push(path);
            } else if path.file_name().is_some_and(|name| name == filename) {
                matches.push(path);
            }
        }
    }
    assert_eq!(matches.len(), 1, "the requested fixture must be unique");
    matches[0]
        .strip_prefix(root)
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned()
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
    for path in [
        "LAVA1 暗子象棋/未打包/暗子象棋.lav",
        "LAVA1 暗子象棋/暗子象棋.pac",
    ] {
        let mut emulator = load(path);
        advance(&mut emulator, 360);
        for _ in 0..3 {
            press(&mut emulator, 13);
        }
        capture(&emulator, "dark-chess-board");
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
        capture(&emulator, "dark-chess-flipped");
        advance(&mut emulator, 1200);
    }
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
    for path in [
        "LAVA1 CS 1.3/非pac/Cs(lav)1_3.lav",
        "LAVA1 CS 1.3/Cs(lav)1_3.pac",
    ] {
        let mut emulator = load(path);
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
        advance(&mut emulator, 60);
        let mission = emulator.display().indexed_frame().to_vec();
        capture(&emulator, "cs13-mission");
        let mut buying = input_branch(&emulator, b'b');
        advance(&mut buying, 300);
        capture(&buying, "cs13-buy-menu");
        press(&mut buying, b'b');
        advance(&mut buying, 300);
        capture(&buying, "cs13-pistol-list");
        press(&mut buying, 13);
        advance(&mut buying, 300);
        capture(&buying, "cs13-purchased");
        let clip_before =
            u16::from_le_bytes(buying.vm().memory()[0x201c..0x201e].try_into().unwrap());
        let mut firing = buying.clone();
        hold(&mut firing, b'z', 120);
        capture(&firing, "cs13-fire");
        let clip_after =
            u16::from_le_bytes(firing.vm().memory()[0x201c..0x201e].try_into().unwrap());
        assert!(
            clip_before > 0 && clip_after < clip_before,
            "Z must fire the purchased pistol and consume ammunition"
        );
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
    for _ in 0..400 {
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

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn spitting_king_versions_move_shoot_adjust_power_and_confirm_exit() {
    for variant in ["口水王2 1.2黑白", "口水王2 1.2灰度"] {
        let mut emulator = load(&format!("LAVA1 2 口水王1.2/{variant}/口水王2.lav"));
        advance(&mut emulator, 300);
        for key in [20, 13, 20, 20, 13, 13, 13, 13] {
            press(&mut emulator, key);
            advance(&mut emulator, 27);
        }
        advance(&mut emulator, 1020);
        capture(&emulator, &format!("spitting-king-{variant}-field"));
        let mut idle = emulator.clone();
        advance(&mut idle, 60);
        let moved = input_branch(&emulator, 20);
        capture(&moved, &format!("spitting-king-{variant}-movement"));
        assert!(
            moved.display().indexed_frame() != idle.display().indexed_frame(),
            "Up must move the player independently of traffic"
        );
        let shot = input_branch(&emulator, b's');
        capture(&shot, &format!("spitting-king-{variant}-shot"));
        assert!(
            shot.display().indexed_frame() != idle.display().indexed_frame(),
            "S must fire a shot"
        );
        let powered = input_branch(&emulator, 19);
        capture(&powered, &format!("spitting-king-{variant}-power"));
        assert!(
            powered.display().indexed_frame() != idle.display().indexed_frame(),
            "Page Up must increase shot power"
        );
        emulator = input_branch(&emulator, 27);
        capture(&emulator, &format!("spitting-king-{variant}-exit-question"));
        press(&mut emulator, b'n');
        let mut idle = emulator.clone();
        advance(&mut idle, 60);
        let moved = input_branch(&emulator, 20);
        assert!(
            moved.display().indexed_frame() != idle.display().indexed_frame(),
            "N must cancel exit and restore player control"
        );
        hold(&mut emulator, 27, 18);
        press(&mut emulator, b'y');
        capture(&emulator, &format!("spitting-king-{variant}-exited"));
        let mut exited = false;
        for frame in 0..600 {
            emulator.input_mut().set_keys(if frame < 3 {
                Some(b'y')
            } else if (300..303).contains(&frame) {
                Some(13)
            } else {
                None
            });
            if let FrameStatus::Halted(code) = emulator.run_frame().unwrap().status {
                assert_eq!(code, 0);
                exited = true;
                break;
            }
        }
        capture(&emulator, "spitting-king-final-confirmation");
        assert!(
            exited,
            "the final Y confirmation must leave the program normally"
        );
    }
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn bobo_dance_versions_score_the_direction_sequence_and_open_the_help() {
    fn fast_key(emulator: &mut Emulator, key: u8) {
        for frame in 0..4 {
            emulator.input_mut().set_keys((frame < 3).then_some(key));
            advance(emulator, 1);
        }
    }
    fn score(emulator: &Emulator) -> Vec<u8> {
        (65..75)
            .flat_map(|row| {
                emulator.display().indexed_frame()[row * 160 + 120..row * 160 + 151]
                    .iter()
                    .copied()
            })
            .collect()
    }
    for path in [
        "LAVA1 Dance_all/未打包的文件/Bobo_Dance.lav",
        "LAVA1 Dance_all/打包的文件/BoBo_dance.pac",
    ] {
        let mut emulator = load(path);
        advance(&mut emulator, 300);
        let mut help = emulator.clone();
        press(&mut emulator, 13);
        advance(&mut emulator, 867);
        capture(&emulator, "bobo-dance-sequence");
        let mut idle = emulator.clone();
        advance(&mut idle, 600);
        let mut wrong = emulator.clone();
        for key in [20, 20, 13] {
            fast_key(&mut wrong, key);
        }
        advance(&mut wrong, 588);
        assert!(
            score(&wrong) == score(&idle),
            "the wrong sequence must not score"
        );
        for key in [20, 21, 13] {
            fast_key(&mut emulator, key);
        }
        advance(&mut emulator, 588);
        capture(&emulator, "bobo-dance-scored");
        assert!(
            score(&emulator) != score(&idle),
            "the correct sequence and timed confirmation must score"
        );
        let title = help.display().indexed_frame().to_vec();
        press(&mut help, 22);
        press(&mut help, 13);
        capture(&help, "bobo-dance-help");
        assert!(
            title != help.display().indexed_frame(),
            "Help must show the instructions"
        );
        for _ in 0..5 {
            press(&mut help, 13);
        }
        capture(&help, "bobo-dance-returned-title");
        assert!(
            title == help.display().indexed_frame(),
            "the last help page must return to the title"
        );
    }
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn just_fly_versions_respond_in_both_modes_and_replay_a_saved_record() {
    fn start_key(emulator: &mut Emulator, key: u8, frames: usize) {
        emulator.input_mut().set_keys([key]);
        advance(emulator, frames);
        emulator.input_mut().set_keys([]);
    }
    for path in [
        "LAVA1 Just_Fly/非pac/Just_Fly.lav",
        "LAVA1 Just_Fly/Just_Fly.pac",
    ] {
        let mut emulator = load(path);
        let initial_record = u16::from_le_bytes(
            emulator.files_mut().file("/LavaData/justfly.sav").unwrap()[10..12]
                .try_into()
                .unwrap(),
        );
        advance(&mut emulator, 300);
        press(&mut emulator, 13);
        let mut held = emulator.clone();
        start_key(&mut held, 20, 18);
        capture(&held, "just-fly-climbing");
        let mut released = emulator.clone();
        start_key(&mut released, 20, 1);
        advance(&mut released, 17);
        capture(&released, "just-fly-descending");
        assert!(
            held.display().indexed_frame() != released.display().indexed_frame(),
            "holding must climb while releasing descends"
        );
        let mut mode_zero = emulator.clone();
        press(&mut mode_zero, 31);
        capture(&mode_zero, "just-fly-settings");
        press(&mut mode_zero, 19);
        press(&mut mode_zero, 13);
        start_key(&mut mode_zero, 20, 3);
        let mut up = mode_zero.clone();
        advance(&mut up, 18);
        start_key(&mut mode_zero, 21, 3);
        advance(&mut mode_zero, 15);
        capture(&mode_zero, "just-fly-mode-zero");
        assert!(
            up.display().indexed_frame() != mode_zero.display().indexed_frame(),
            "mode zero must respond to a single Down press after Up"
        );
        let mut recorded_scene = Vec::new();
        for frame in 0..300 {
            emulator.input_mut().set_keys((frame % 7 < 3).then_some(20));
            advance(&mut emulator, 1);
            if frame == 59 {
                recorded_scene = emulator.display().indexed_frame()[16 * 160..64 * 160].to_vec();
            }
        }
        emulator.input_mut().set_keys([]);
        advance(&mut emulator, 300);
        capture(&emulator, "just-fly-new-record");
        let files = emulator.files_mut().take_dirty_files();
        assert!(
            files.iter().any(|(path, bytes)| {
                path.ends_with("justfly.sav")
                    && bytes.len() == 16220
                    && u16::from_le_bytes(bytes[10..12].try_into().unwrap()) > initial_record
            }),
            "beating the supplied record must write the replay file"
        );
        let mut replay = load(path);
        for (path, bytes) in files {
            replay.files_mut().import_file(&path, bytes);
        }
        advance(&mut replay, 300);
        press(&mut replay, 13);
        start_key(&mut replay, b'r', 1);
        advance(&mut replay, 59);
        capture(&replay, "just-fly-replay");
        assert!(
            recorded_scene == replay.display().indexed_frame()[16 * 160..64 * 160],
            "the saved replay must restore the recorded flight trajectory"
        );
    }
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn em_fighter_moves_attacks_and_finishes_the_bout() {
    let mut emulator = load("LAVA1 EM星纪传说—格斗/EM星纪传说—格斗.lav");
    advance(&mut emulator, 300);
    emulator.input_mut().set_keys([13]);
    advance(&mut emulator, 3);
    emulator.input_mut().set_keys([]);
    advance(&mut emulator, 7);
    capture(&emulator, "em-fighter-arena");
    let mut left = emulator.clone();
    hold(&mut left, b'a', 180);
    let mut right = emulator.clone();
    hold(&mut right, b'd', 180);
    capture(&left, "em-fighter-left");
    capture(&right, "em-fighter-right");
    assert!(
        left.display().indexed_frame() != right.display().indexed_frame(),
        "A and D must move the fighter in different directions"
    );
    let mut attacking = emulator.clone();
    hold(&mut attacking, b'l', 180);
    let mut idle = emulator.clone();
    advance(&mut idle, 210);
    capture(&attacking, "em-fighter-attack");
    assert!(
        attacking.display().indexed_frame() != idle.display().indexed_frame(),
        "L must animate an attack independently of the opponent"
    );
    let mut ended = false;
    for _ in 0..12000 {
        if let FrameStatus::Halted(code) = emulator.run_frame().unwrap().status {
            assert_eq!(code, 0);
            ended = true;
            break;
        }
    }
    capture(&emulator, "em-fighter-result");
    assert!(
        ended,
        "the unattended bout must reach its result and finish normally"
    );
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn iq_battle_moves_removes_groups_uses_a_bomb_and_returns_to_the_menu() {
    fn board(emulator: &Emulator) -> Vec<u8> {
        (0..80)
            .flat_map(|row| {
                emulator.display().indexed_frame()[row * 160..row * 160 + 82]
                    .iter()
                    .copied()
            })
            .collect()
    }
    let mut emulator = load("LAVA1 IQ大作战/IQ大作战.lav");
    advance(&mut emulator, 300);
    for _ in 0..2 {
        press(&mut emulator, 13);
        advance(&mut emulator, 27);
    }
    let menu = emulator.display().indexed_frame().to_vec();
    press(&mut emulator, 13);
    advance(&mut emulator, 747);
    capture(&emulator, "iq-battle-board");
    let mut idle = emulator.clone();
    advance(&mut idle, 60);
    let moved = input_branch(&emulator, 22);
    capture(&moved, "iq-battle-cursor");
    assert!(
        board(&moved) != board(&idle),
        "Right must move the selection"
    );
    let removed = input_branch(&emulator, 13);
    capture(&removed, "iq-battle-removed");
    assert!(
        board(&removed) != board(&idle),
        "Enter must remove the selected matching group"
    );
    let mut bomb = input_branch(&emulator, b'b');
    capture(&bomb, "iq-battle-bomb");
    assert!(board(&bomb) != board(&idle), "B must remove a single tile");
    assert!(
        (40..60).any(|row| {
            let counter = row * 160 + 90..row * 160 + 160;
            bomb.display().indexed_frame()[counter.clone()]
                != idle.display().indexed_frame()[counter]
        }),
        "using a bomb must update its counter"
    );
    hold(&mut bomb, 27, 18);
    capture(&bomb, "iq-battle-menu");
    assert!(
        menu == bomb.display().indexed_frame(),
        "Escape must return to the original menu"
    );
    press(&mut bomb, 13);
    advance(&mut bomb, 300);
    capture(&bomb, "iq-battle-new-round");
    assert!(
        menu != bomb.display().indexed_frame(),
        "Start must open another round"
    );
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn queerbox_moves_a_top_block_restarts_and_returns_to_the_title() {
    fn steps(emulator: &Emulator) -> Vec<u8> {
        (40..60)
            .flat_map(|row| {
                emulator.display().indexed_frame()[row * 160..row * 160 + 32]
                    .iter()
                    .copied()
            })
            .collect()
    }
    let mut emulator = load("LAVA1 QUEERBOX/QUEERBOX.lav");
    advance(&mut emulator, 300);
    let title = emulator.display().indexed_frame().to_vec();
    for _ in 0..2 {
        press(&mut emulator, 13);
        advance(&mut emulator, 27);
    }
    capture(&emulator, "queerbox-board");
    let zero_steps = steps(&emulator);
    for key in [20, 20, 13, 22, 13] {
        press(&mut emulator, key);
        advance(&mut emulator, 27);
    }
    capture(&emulator, "queerbox-transferred");
    assert!(
        zero_steps != steps(&emulator),
        "moving the top tile to the next column must count a step"
    );
    press(&mut emulator, 28);
    capture(&emulator, "queerbox-restarted");
    assert!(
        zero_steps == steps(&emulator),
        "F1 must restart with zero steps"
    );
    press(&mut emulator, 27);
    press(&mut emulator, b'y');
    advance(&mut emulator, 120);
    capture(&emulator, "queerbox-title");
    assert!(
        title == emulator.display().indexed_frame(),
        "Escape and Y must return to the title"
    );
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn q_dancer_enters_the_test_answers_a_sequence_and_scores() {
    let word = |emulator: &Emulator, address: usize| {
        u16::from_le_bytes(
            emulator.vm().memory()[address..address + 2]
                .try_into()
                .unwrap(),
        )
    };
    let mut emulator = load("LAVA1 Q版Dancer/Q版Dancer.lav");
    advance(&mut emulator, 300);
    // The supplied source identifies the pixel cursor and contest hit box.
    for _ in 0..3000 {
        let x = word(&emulator, 0x2034);
        let y = word(&emulator, 0x2036);
        if x == 60 && y == 58 {
            break;
        }
        let key = if x > 60 {
            23
        } else if x < 60 {
            22
        } else if y > 58 {
            20
        } else {
            21
        };
        emulator.input_mut().set_keys([key]);
        advance(&mut emulator, 1);
    }
    emulator.input_mut().set_keys([]);
    assert_eq!(word(&emulator, 0x2034), 60);
    assert_eq!(word(&emulator, 0x2036), 58);
    advance(&mut emulator, 20);
    capture(&emulator, "q-dancer-menu");
    emulator.input_mut().set_keys([13]);
    advance(&mut emulator, 1);
    emulator.input_mut().set_keys([]);
    advance(&mut emulator, 90);
    emulator.input_mut().set_keys([13]);
    advance(&mut emulator, 1);
    emulator.input_mut().set_keys([]);
    advance(&mut emulator, 90);
    emulator.input_mut().set_keys([13]);
    advance(&mut emulator, 1);
    emulator.input_mut().set_keys([]);
    advance(&mut emulator, 90);
    advance(&mut emulator, 2400);
    let sequence = emulator.vm().memory()[0x2002..0x2006].to_vec();
    assert!(
        sequence
            .iter()
            .all(|key| b"qwerasdfiopkl".contains(key) || *key == 13)
    );
    assert_eq!(word(&emulator, 0x203a), 0);
    for key in sequence {
        emulator.input_mut().set_keys([key]);
        advance(&mut emulator, 1);
        emulator.input_mut().set_keys([]);
        advance(&mut emulator, 60);
    }
    assert_eq!(word(&emulator, 0x203a), 0, "correct sequence must not miss");
    assert!(word(&emulator, 0x203c) > 0, "correct sequence must score");
    let saved = emulator.save_state().unwrap();
    let mut restored = load("LAVA1 Q版Dancer/Q版Dancer.lav");
    restored.load_state(&saved).unwrap();
    assert_eq!(word(&restored, 0x203a), 0);
    assert_eq!(word(&restored, 0x203c), word(&emulator, 0x203c));
    capture(&emulator, "q-dancer-fixed-answer");
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn rc_racing_accelerates_steers_pauses_and_returns_to_the_menu() {
    let mut emulator = load("LAVA1 RC赛车/RC赛车.lav");
    advance(&mut emulator, 300);
    for _ in 0..2 {
        press(&mut emulator, 13);
        advance(&mut emulator, 27);
    }
    advance(&mut emulator, 780);
    let menu = emulator.display().indexed_frame().to_vec();
    press(&mut emulator, 13);
    advance(&mut emulator, 27);
    capture(&emulator, "rc-racing-track");
    let mut idle = emulator.clone();
    advance(&mut idle, 120);
    for _ in 0..120 {
        emulator.input_mut().set_keys(*b"a");
        advance(&mut emulator, 1);
    }
    emulator.input_mut().set_keys([]);
    capture(&emulator, "rc-racing-speed");
    assert!(
        (65..80).any(|row| {
            let speed = row * 160 + 88..row * 160 + 150;
            emulator.display().indexed_frame()[speed.clone()]
                != idle.display().indexed_frame()[speed]
        }),
        "A must increase the speed indicator"
    );
    let mut left = emulator.clone();
    let mut right = emulator.clone();
    for _ in 0..45 {
        left.input_mut().set_keys([b'a', 23]);
        advance(&mut left, 1);
        right.input_mut().set_keys([b'a', 22]);
        advance(&mut right, 1);
    }
    capture(&right, "rc-racing-steering");
    assert!(
        left.display().indexed_frame() != right.display().indexed_frame(),
        "opposite steering inputs must change the race view"
    );
    press(&mut emulator, b'p');
    capture(&emulator, "rc-racing-paused");
    let paused = emulator.display().indexed_frame().to_vec();
    advance(&mut emulator, 120);
    assert!(
        paused == emulator.display().indexed_frame(),
        "pause must freeze the race view"
    );
    press(&mut emulator, 13);
    hold(&mut emulator, b'a', 30);
    assert!(
        paused != emulator.display().indexed_frame(),
        "confirmation must resume the race"
    );
    press(&mut emulator, 27);
    advance(&mut emulator, 300);
    capture(&emulator, "rc-racing-menu");
    assert!(
        menu == emulator.display().indexed_frame(),
        "Escape must return to the race menu"
    );
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn rush_out_variants_fly_drop_bombs_pause_and_return_to_the_menu() {
    for (variant, path) in [
        ("8k", "LAVA1 Rush_Out/Rush_Out_For_Lava(8K).lav"),
        ("lava1", "LAVA1 Rush_Out/Rush_Out_For_Lava1(16k).lav"),
        ("lava2", "LAVA1 Rush_Out/Rush_Out_For_Lava2(16k).lav"),
    ] {
        let mut emulator = load(path);
        advance(&mut emulator, 300);
        press(&mut emulator, 13);
        advance(&mut emulator, 567);
        let menu = emulator.display().indexed_frame().to_vec();
        press(&mut emulator, 13);
        advance(&mut emulator, 237);
        capture(&emulator, &format!("rush-out-{variant}-tunnel"));
        let ready = emulator.clone();
        let mut idle = emulator.clone();
        let mut bomb = emulator.clone();
        for _ in 0..60 {
            emulator.input_mut().set_keys([20]);
            advance(&mut emulator, 1);
            bomb.input_mut().set_keys(*b"x");
            advance(&mut bomb, 1);
            advance(&mut idle, 1);
        }
        assert!(
            (10..16).any(|row| {
                let hud = row * 160 + 5..row * 160 + 33;
                bomb.display().indexed_frame()[hud.clone()] != idle.display().indexed_frame()[hud]
            }),
            "X must reduce the available bomb indicator: {variant}"
        );
        capture(&bomb, &format!("rush-out-{variant}-bomb"));
        let avatar_y = |emulator: &Emulator| {
            let pixels = emulator.display().indexed_frame();
            let water = pixels[30 * 160 + 30];
            (18..79).find(|row| (16..24).any(|x| pixels[row * 160 + x] != water))
        };
        let mut positions = None;
        for _ in 0..8 {
            emulator.input_mut().set_keys([20]);
            advance(&mut emulator, 1);
            advance(&mut idle, 1);
            if let (Some(up), Some(down)) = (avatar_y(&emulator), avatar_y(&idle)) {
                positions = Some((up, down));
                break;
            }
        }
        let (up, down) = positions.expect("both submarines must be visible between redraws");
        assert!(
            up < down,
            "Up must lift the submarine above the released-key branch: {variant}"
        );
        capture(&emulator, &format!("rush-out-{variant}-flying"));
        emulator = ready.clone();
        hold(&mut emulator, b'z', 12);
        let paused = emulator.display().indexed_frame().to_vec();
        capture(&emulator, &format!("rush-out-{variant}-paused"));
        advance(&mut emulator, 120);
        assert!(
            paused == emulator.display().indexed_frame(),
            "pause must freeze {variant}"
        );
        hold(&mut emulator, 20, 12);
        assert!(
            paused != emulator.display().indexed_frame(),
            "Up must resume {variant}"
        );
        hold(&mut emulator, 27, 12);
        press(&mut emulator, b'y');
        advance(&mut emulator, 300);
        capture(&emulator, &format!("rush-out-{variant}-menu"));
        assert!(
            menu == emulator.display().indexed_frame(),
            "Escape must return to the menu: {variant}"
        );
        if variant != "8k" {
            idle.input_mut().set_keys([]);
            advance(&mut idle, 600);
            let game_over = idle.display().indexed_frame().to_vec();
            for _ in 0..2 {
                press(&mut idle, 13);
                advance(&mut idle, 87);
            }
            capture(&idle, &format!("rush-out-{variant}-name"));
            press(&mut idle, 13);
            advance(&mut idle, 27);
            press(&mut idle, b' ');
            advance(&mut idle, 87);
            capture(&idle, &format!("rush-out-{variant}-top-five"));
            press(&mut idle, 13);
            advance(&mut idle, 87);
            press(&mut idle, b'y');
            advance(&mut idle, 87);
            press(&mut idle, 13);
            advance(&mut idle, 207);
            let files = idle.files_mut().take_dirty_files();
            let top = files
                .iter()
                .find(|(name, _)| name.ends_with("rush_top.dat"))
                .unwrap();
            assert_eq!(top.1.len(), 55);
            assert_eq!(top.1[0], b'A', "the entered name must be saved");
            assert!(u16::from_le_bytes(top.1[45..47].try_into().unwrap()) > 0);
            let recording = files
                .iter()
                .find(|(name, _)| name.ends_with("rush_rpl.dat"))
                .unwrap();
            assert_eq!(recording.1.len(), 12012);
            assert_eq!(recording.1[0], 1, "the first replay slot must be populated");
            let mut replay = load(path);
            for (name, bytes) in files {
                replay.files_mut().import_file(&name, bytes);
            }
            advance(&mut replay, 300);
            press(&mut replay, 13);
            advance(&mut replay, 567);
            press(&mut replay, 21);
            advance(&mut replay, 27);
            press(&mut replay, 13);
            advance(&mut replay, 27);
            capture(&replay, &format!("rush-out-{variant}-saved-slot"));
            press(&mut replay, 13);
            advance(&mut replay, 237);
            capture(&replay, &format!("rush-out-{variant}-replaying"));
            assert!(
                replay.display().indexed_frame()[18 * 160..]
                    == ready.display().indexed_frame()[18 * 160..],
                "a fresh session must replay the initial tunnel: {variant}"
            );
            advance(&mut replay, 600);
            assert!(
                game_over == replay.display().indexed_frame(),
                "the recorded collision must end at the same game-over screen: {variant}"
            );
        }
    }
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn snow_fight_opens_help_moves_throws_and_finishes_a_round() {
    let mut emulator = load("LAVA1 SnowFight(打雪仗)/SnowFight.lav");
    advance(&mut emulator, 300);
    let menu = emulator.display().indexed_frame().to_vec();
    let menu_matches = |emulator: &Emulator| {
        (30..74).all(|row| {
            let choices = row * 160 + 14..row * 160 + 73;
            menu[choices.clone()] == emulator.display().indexed_frame()[choices]
        })
    };
    press(&mut emulator, 21);
    advance(&mut emulator, 27);
    press(&mut emulator, 13);
    advance(&mut emulator, 27);
    capture(&emulator, "snow-fight-help");
    assert!(menu != emulator.display().indexed_frame());
    press(&mut emulator, 13);
    advance(&mut emulator, 27);
    capture(&emulator, "snow-fight-help-return");
    assert!(menu_matches(&emulator));
    press(&mut emulator, 13);
    advance(&mut emulator, 900);
    capture(&emulator, "snow-fight-field");
    let left = input_branch(&emulator, b'a');
    let right = input_branch(&emulator, b'd');
    let player_x = |emulator: &Emulator| {
        (15..140)
            .find(|&x| {
                (x..x + 4).all(|x| {
                    let display = emulator.display();
                    display.palette()[usize::from(display.indexed_frame()[72 * 160 + x])]
                        == [0, 0, 0]
                })
            })
            .expect("the player's body must be visible")
    };
    assert!(
        player_x(&left) < player_x(&right),
        "A and D must move in opposite directions"
    );
    capture(&right, "snow-fight-moved");
    let attack = input_branch(&emulator, b'l');
    let mut idle = emulator.clone();
    advance(&mut idle, 60);
    assert!(
        (60..79).any(|row| {
            let player = row * 160 + 60..row * 160 + 100;
            attack.display().indexed_frame()[player.clone()]
                != idle.display().indexed_frame()[player]
        }),
        "L must animate the player's throwing pose"
    );
    capture(&attack, "snow-fight-throw");
    for _ in 0..12000 {
        emulator.input_mut().set_keys(*b"l");
        advance(&mut emulator, 1);
    }
    emulator.input_mut().set_keys([]);
    assert_eq!(
        emulator.run_frame().unwrap().status,
        FrameStatus::WaitingForInput
    );
    capture(&emulator, "snow-fight-result");
    press(&mut emulator, 13);
    advance(&mut emulator, 120);
    assert!(
        menu_matches(&emulator),
        "confirming the result must reopen the menu"
    );
    press(&mut emulator, 21);
    press(&mut emulator, 21);
    for frame in 0..120 {
        emulator.input_mut().set_keys((frame < 3).then_some(13));
        if let FrameStatus::Halted(code) = emulator.run_frame().unwrap().status {
            assert_eq!(code, 0);
            return;
        }
    }
    panic!("the Exit menu item must finish the game");
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn space_wars_variants_move_fire_and_exit_the_mission() {
    let word = |emulator: &Emulator, address| {
        u16::from_le_bytes(
            emulator.vm().memory()[address..address + 2]
                .try_into()
                .unwrap(),
        )
    };
    let branch = |original: &Emulator, key| {
        let mut emulator = original.clone();
        for frame in 0..30 {
            emulator.input_mut().set_keys((frame < 18).then_some(key));
            advance(&mut emulator, 1);
        }
        emulator
    };
    for (variant, path) in [
        ("fast", "LAVA1 SpaceWars/SWR无延时.lav"),
        ("delayed", "LAVA1 SpaceWars/SWR有延时.lav"),
    ] {
        let mut emulator = load(path);
        advance(&mut emulator, 300);
        let title = emulator.display().indexed_frame().to_vec();
        press(&mut emulator, b's');
        advance(&mut emulator, 57);
        capture(&emulator, &format!("space-wars-{variant}-mission"));
        assert_eq!(word(&emulator, 0x3187), 70);
        assert_eq!(word(&emulator, 0x3189), 60);
        let idle = branch(&emulator, 0);
        let left = branch(&emulator, b'a');
        let right = branch(&emulator, b'd');
        let up = branch(&emulator, b'w');
        let down = branch(&emulator, b's');
        assert!(word(&left, 0x3187) < word(&idle, 0x3187));
        assert!(word(&right, 0x3187) > word(&idle, 0x3187));
        assert!(word(&up, 0x3189) < word(&idle, 0x3189));
        assert!(word(&down, 0x3189) > word(&idle, 0x3189));
        capture(&right, &format!("space-wars-{variant}-moved"));
        let mut shot = emulator.clone();
        for _ in 0..6 {
            shot.input_mut().set_keys(*b"l");
            advance(&mut shot, 1);
        }
        assert_eq!(word(&emulator, 0x3209), 0);
        assert_eq!(
            word(&shot, 0x3209),
            4,
            "L must create a four-pixel projectile"
        );
        assert_eq!(word(&shot, 0x31c1), word(&shot, 0x3187) + 4);
        assert!(
            word(&shot, 0x31cd) < word(&shot, 0x3189),
            "the projectile must travel above the ship"
        );
        assert!(shot.display().indexed_frame() != emulator.display().indexed_frame());
        capture(&shot, &format!("space-wars-{variant}-shot"));
        let mut defeated = emulator.clone();
        advance(&mut defeated, 600);
        capture(&defeated, &format!("space-wars-{variant}-title"));
        assert!(
            title[..55 * 160] == defeated.display().indexed_frame()[..55 * 160],
            "a lost mission must return to the title"
        );
        let mut exited = false;
        for _ in 0..30 {
            emulator.input_mut().set_keys([27]);
            if let FrameStatus::Halted(code) = emulator.run_frame().unwrap().status {
                assert_eq!(code, 0);
                exited = true;
                break;
            }
        }
        assert!(exited, "Escape must exit the active mission: {variant}");
    }
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn travel_variants_explore_fight_collect_the_tools_and_finish_the_demo() {
    let word = |e: &Emulator, p| u16::from_le_bytes(e.vm().memory()[p..p + 2].try_into().unwrap());
    let tap = |e: &mut Emulator, key| {
        press(e, key);
        advance(e, 27);
    };
    let walk = |e: &mut Emulator, x: u16, y: u16| {
        for (address, target, lower, higher) in [(0x48bf, x, 23, 22), (0x48c1, y, 20, 21)] {
            for _ in 0..300 {
                let current = word(e, address);
                if current.abs_diff(target) <= 1 {
                    break;
                }
                e.input_mut()
                    .set_keys([if current < target { higher } else { lower }]);
                advance(e, 1);
            }
            assert!(
                word(e, address).abs_diff(target) <= 1,
                "cursor {address:x} must reach {target}, got {} pc {:x}",
                word(e, address),
                e.vm().pc()
            );
        }
        e.input_mut().set_keys([]);
    };
    for (variant, path) in [
        ("loose", "LAVA1 TRAVEL/travel.lav"),
        ("pac", "LAVA1 TRAVEL/打包travel.pac"),
    ] {
        let mut e = load(path);
        advance(&mut e, 13200);
        for _ in 0..5 {
            tap(&mut e, 13);
        }
        assert_eq!(word(&e, 0x48c3), 0);
        capture(&e, &format!("travel-{variant}-forest"));
        walk(&mut e, 78, 44);
        capture(&e, &format!("travel-{variant}-path-prompt"));
        tap(&mut e, 13);
        assert_eq!(word(&e, 0x48c3), 1);
        walk(&mut e, 24, 44);
        for _ in 0..5 {
            tap(&mut e, 13);
        }
        capture(&e, &format!("travel-{variant}-battle"));
        for frame in 0..8000 {
            e.input_mut().set_keys((frame % 60 < 3).then_some(13));
            advance(&mut e, 1);
            if frame == 600 {
                capture(&e, &format!("travel-{variant}-battle"));
            }
            if word(&e, 0x4803) == 1 {
                break;
            }
        }
        assert_eq!(word(&e, 0x4803), 1, "the trial battle must finish");
        e.input_mut().set_keys([]);
        capture(&e, &format!("travel-{variant}-battle-result"));
        for _ in 0..2 {
            tap(&mut e, 13);
        }
        walk(&mut e, 6, 18);
        tap(&mut e, 13);
        assert_eq!(word(&e, 0x48c3), 2);
        for _ in 0..3 {
            tap(&mut e, 13);
        }
        capture(&e, &format!("travel-{variant}-mountain"));
        walk(&mut e, 150, 64);
        tap(&mut e, 13);
        for _ in 0..3 {
            tap(&mut e, 13);
        }
        assert_eq!(word(&e, 0x48c3), 0);
        capture(&e, &format!("travel-{variant}-cabin"));
        walk(&mut e, 56, 56);
        tap(&mut e, 13);
        for _ in 0..4 {
            tap(&mut e, 13);
        }
        assert_eq!(word(&e, 0x48c3), 1);
        capture(&e, &format!("travel-{variant}-room"));
        walk(&mut e, 128, 50);
        tap(&mut e, 13);
        for _ in 0..8 {
            tap(&mut e, 13);
        }
        assert_eq!(
            word(&e, 0x47f5),
            1,
            "the cabinet must supply climbing tools"
        );
        capture(&e, &format!("travel-{variant}-tools"));
        let mut quit = e.clone();
        for _ in 0..10 {
            quit.input_mut().set_keys([27]);
            if matches!(quit.run_frame().unwrap().status, FrameStatus::Halted(0)) {
                break;
            }
        }
        assert!(!quit.vm().is_running(), "Escape must exit exploration");
        walk(&mut e, 80, 74);
        tap(&mut e, 13);
        assert_eq!(word(&e, 0x48c3), 0);
        walk(&mut e, 16, 64);
        tap(&mut e, 13);
        assert_eq!(word(&e, 0x48c3), 2);
        walk(&mut e, 78, 10);
        tap(&mut e, 13);
        for _ in 0..1200 {
            e.input_mut().set_keys([]);
            let status = e.run_frame().unwrap().status;
            if matches!(
                status,
                FrameStatus::WaitingForInput | FrameStatus::Halted(_)
            ) {
                break;
            }
        }
        tap(&mut e, 13);
        capture(&e, &format!("travel-{variant}-ending"));
        assert_eq!(e.run_frame().unwrap().status, FrameStatus::WaitingForInput);
        assert_eq!(
            word(&e, 0x48bd),
            0,
            "the climbing tools must unlock the demo ending"
        );
    }
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn warcraft_variants_produce_move_collect_income_and_exit() {
    let gold =
        |e: &Emulator| u16::from_le_bytes(e.vm().memory()[0x42bc..0x42be].try_into().unwrap());
    let tap = |e: &mut Emulator, key| {
        e.input_mut().set_keys([key]);
        advance(e, 1);
        e.input_mut().set_keys([]);
        advance(e, 59);
    };
    for (variant, path) in [
        ("loose", "LAVA1 WarCraft/非pac/WarCraft.lav"),
        ("pac", "LAVA1 WarCraft/WarCraft.pac"),
    ] {
        let mut e = load(path);
        for frame in 0..1200 {
            e.input_mut()
                .set_keys(((300..660).contains(&frame) && (frame - 300) % 60 < 3).then_some(13));
            advance(&mut e, 1);
        }
        assert_eq!(gold(&e), 800);
        capture(&e, &format!("warcraft-{variant}-board"));
        let mut menu = e.clone();
        tap(&mut menu, 25);
        assert_eq!(
            menu.run_frame().unwrap().status,
            FrameStatus::WaitingForInput
        );
        let pixels = menu.display().indexed_frame();
        let background = pixels[60 * 160 + 50];
        assert!(
            (16..65).all(|y| (16..74).all(|x| pixels[y * 160 + x] == background)),
            "opening the system menu must not consume an old Enter and open speed settings"
        );
        capture(&menu, &format!("warcraft-{variant}-system-menu"));
        tap(&mut e, 19);
        tap(&mut e, 13);
        tap(&mut e, 13);
        assert_eq!(gold(&e), 725, "training a worker must cost 75 gold");
        capture(&e, &format!("warcraft-{variant}-trained"));
        let before_move = e.display().indexed_frame().to_vec();
        tap(&mut e, 23);
        tap(&mut e, 13);
        capture(&e, &format!("warcraft-{variant}-unit-menu"));
        for key in [13, 21, 13, 19] {
            tap(&mut e, key);
        }
        assert!(
            (32..64).any(|y| {
                let tile = y * 160 + 64..y * 160 + 80;
                before_move[tile.clone()] != e.display().indexed_frame()[tile]
            }),
            "moving the worker must change its map tiles after recentering the cursor"
        );
        capture(&e, &format!("warcraft-{variant}-moved"));
        tap(&mut e, b'j');
        advance(&mut e, 540);
        assert_eq!(gold(&e), 755, "ending a turn must collect worker income");
        for _ in 0..30 {
            e.input_mut().set_keys([26]);
            advance(&mut e, 1);
        }
        capture(&e, &format!("warcraft-{variant}-income"));
        e.input_mut().set_keys([]);
        tap(&mut e, 13);
        tap(&mut e, 25);
        for _ in 0..3 {
            tap(&mut e, 21);
        }
        let mut exited = false;
        for frame in 0..1200 {
            e.input_mut()
                .set_keys((frame == 0 || frame == 180).then_some(13));
            if let FrameStatus::Halted(code) = e.run_frame().unwrap().status {
                assert_eq!(code, 1);
                exited = true;
                break;
            }
        }
        capture(&e, &format!("warcraft-{variant}-exit"));
        assert!(exited, "the system Exit item must exit normally");
    }
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn rainbow_bubbles_moves_fires_pauses_and_replays_a_saved_round() {
    let path = "LAVA1 七彩泡泡/七彩泡泡.lav";
    let mut e = load(path);
    for frame in 0..1200 {
        e.input_mut()
            .set_keys(((300..480).contains(&frame) && (frame - 300) % 60 < 3).then_some(13));
        advance(&mut e, 1);
    }
    press(&mut e, 13);
    advance(&mut e, 267);
    capture(&e, "rainbow-bubbles-field");
    let left = input_branch(&e, 23);
    let right = input_branch(&e, 22);
    assert!(
        left.display().indexed_frame()[64 * 160..] != right.display().indexed_frame()[64 * 160..],
        "opposite directions must move the hero"
    );
    capture(&right, "rainbow-bubbles-moving");
    let mut paused = e.clone();
    press(&mut paused, b'p');
    let frozen = paused.display().indexed_frame().to_vec();
    capture(&paused, "rainbow-bubbles-paused");
    advance(&mut paused, 120);
    assert!(frozen == paused.display().indexed_frame());
    press(&mut paused, 13);
    assert!(
        frozen != paused.display().indexed_frame(),
        "confirmation must resume the round"
    );
    for _ in 0..2400 {
        e.input_mut().set_keys(*b"q");
        advance(&mut e, 1);
    }
    e.input_mut().set_keys([]);
    let game_over = e.display().indexed_frame().to_vec();
    capture(&e, "rainbow-bubbles-game-over");
    assert!(
        e.vm().memory()[0x565c] > 0,
        "shots must hit bubbles during the round"
    );
    press(&mut e, 13);
    capture(&e, "rainbow-bubbles-statistics");
    press(&mut e, 13);
    press(&mut e, b's');
    advance(&mut e, 267);
    capture(&e, "rainbow-bubbles-menu");
    let files = e.files_mut().take_dirty_files();
    let recording = files
        .iter()
        .find(|(name, _)| name.ends_with("bu_replay.dat"))
        .expect("S must write the recording");
    assert!(recording.1.len() > 16);
    let mut replay = load(path);
    for (name, bytes) in files {
        replay.files_mut().import_file(&name, bytes);
    }
    advance(&mut replay, 300);
    press(&mut replay, 13);
    advance(&mut replay, 267);
    press(&mut replay, 22);
    press(&mut replay, 13);
    advance(&mut replay, 3000);
    capture(&replay, "rainbow-bubbles-replayed");
    assert!(
        game_over == replay.display().indexed_frame(),
        "a fresh session must replay the same round ending"
    );
    for _ in 0..3 {
        press(&mut e, 22);
    }
    for frame in 0..1200 {
        e.input_mut().set_keys((frame % 60 < 3).then_some(13));
        if let FrameStatus::Halted(code) = e.run_frame().unwrap().status {
            assert_eq!(code, 0);
            return;
        }
    }
    panic!("Exit must finish the game");
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn color_lines_selects_moves_cancels_and_scores_a_line() {
    let tap = |e: &mut Emulator, key| {
        e.input_mut().set_keys([key]);
        advance(e, 1);
        e.input_mut().set_keys([]);
        advance(e, 5);
    };
    let point = |e: &mut Emulator, cell: usize| {
        for _ in 0..8 {
            tap(e, 23);
            tap(e, 20);
        }
        for _ in 0..cell / 9 {
            tap(e, 22);
        }
        for _ in 0..cell % 9 {
            tap(e, 21);
        }
    };
    let balls = |e: &Emulator| {
        e.vm().memory()[0x2358..0x23a9]
            .iter()
            .filter(|&&v| v != 0)
            .count()
    };
    let score =
        |e: &Emulator| u16::from_le_bytes(e.vm().memory()[0x23ad..0x23af].try_into().unwrap());
    let path = "LAVA1 七彩连珠/七彩连珠.lav";
    let mut e = load(path);
    for frame in 0..1200 {
        e.input_mut()
            .set_keys(((300..480).contains(&frame) && (frame - 300) % 60 < 3).then_some(13));
        advance(&mut e, 1);
    }
    capture(&e, "color-lines-selected");
    let initial_balls = balls(&e);
    tap(&mut e, 27);
    assert_eq!(balls(&e), initial_balls, "cancel must preserve all balls");
    assert_eq!(score(&e), 0);
    let route = [
        (7, 6),
        (30, 31),
        (31, 39),
        (29, 30),
        (52, 61),
        (24, 32),
        (61, 62),
        (68, 59),
        (80, 71),
        (31, 59),
        (34, 59),
        (3, 2),
        (35, 43),
        (21, 12),
        (24, 42),
    ];
    for (turn, (source, destination)) in route.into_iter().enumerate() {
        let before = balls(&e);
        point(&mut e, source);
        tap(&mut e, 13);
        point(&mut e, destination);
        if turn == 14 {
            capture(&e, "color-lines-before-clear");
        }
        tap(&mut e, 13);
        advance(&mut e, 60);
        if turn == 0 {
            assert_eq!(
                balls(&e),
                before + 3,
                "a non-scoring move must add three balls"
            );
            capture(&e, "color-lines-moved");
        }
        if turn == 14 {
            assert!(balls(&e) < before, "the completed line must remove balls");
        }
    }
    assert_eq!(score(&e), 10, "the completed line must award points");
    capture(&e, "color-lines-scored");
    let mut menu = load(path);
    advance(&mut menu, 300);
    for _ in 0..3 {
        tap(&mut menu, 21);
    }
    for frame in 0..1200 {
        menu.input_mut().set_keys((frame % 60 < 3).then_some(13));
        if let FrameStatus::Halted(code) = menu.run_frame().unwrap().status {
            assert_eq!(code, 0);
            return;
        }
    }
    panic!("the title Exit choice must terminate the game");
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn seven_king_selects_plays_passes_finishes_and_restarts_a_round() {
    fn wait(e: &mut Emulator) -> usize {
        e.input_mut().set_keys([]);
        for _ in 0..12000 {
            let result = e.run_frame().unwrap();
            assert!(!matches!(result.status, FrameStatus::Halted(_)));
            if result.status == FrameStatus::WaitingForInput {
                return e.vm().pc();
            }
        }
        panic!("the card game must reach its next input prompt");
    }
    fn key(e: &mut Emulator, key: u8) -> usize {
        e.input_mut().set_keys([key]);
        e.run_frame().unwrap();
        wait(e)
    }
    let mut e = load("LAVA1 七王五二三/七王五二三.lav");
    for frame in 0..1200 {
        e.input_mut()
            .set_keys(((300..480).contains(&frame) && (frame - 300) % 60 < 3).then_some(13));
        advance(&mut e, 1);
    }
    assert_eq!(wait(&mut e), 0x257c);
    assert_eq!(&e.vm().memory()[0x3d3e..0x3d40], &[5, 5]);
    assert_eq!(e.vm().memory()[0x3d39], 10);
    capture(&e, "seven-king-dealt");
    let hand = e.display().indexed_frame().to_vec();
    key(&mut e, 22);
    assert!(
        hand != e.display().indexed_frame(),
        "Right must move the card cursor"
    );
    key(&mut e, 23);
    key(&mut e, 20);
    capture(&e, "seven-king-selected");
    assert!(hand != e.display().indexed_frame(), "Up must select a card");
    key(&mut e, 21);
    assert!(
        hand == e.display().indexed_frame(),
        "Down must cancel the selection"
    );
    key(&mut e, 20);
    key(&mut e, 13);
    assert!(
        e.vm().memory()[0x3d39] > 10,
        "a legal play must progress the deck and refill the hand"
    );
    capture(&e, "seven-king-played");
    let mut finished = false;
    for _ in 0..100 {
        for command in [20, 13, 21, b'p'] {
            let pc = key(&mut e, command);
            if pc != 0x257c {
                assert_eq!(pc, 0x1997, "the round must end at its result menu");
                finished = true;
                break;
            }
        }
        if finished {
            break;
        }
    }
    assert!(
        finished,
        "legal single-card play and passing must complete the round"
    );
    assert_eq!(e.vm().memory()[0x3d39], 54);
    assert!(e.vm().memory()[0x3d3e..0x3d40].contains(&0));
    capture(&e, "seven-king-result");
    assert_eq!(key(&mut e, 13), 0x257c, "Play Again must start a new deal");
    assert_eq!(&e.vm().memory()[0x3d3e..0x3d40], &[5, 5]);
    capture(&e, "seven-king-restarted");
    let restarted = e.display().indexed_frame().to_vec();
    key(&mut e, 27);
    capture(&e, "seven-king-confirm-exit");
    key(&mut e, b'n');
    assert!(
        restarted == e.display().indexed_frame(),
        "N must cancel quitting without changing the deal"
    );
    key(&mut e, 27);
    e.input_mut().set_keys(*b"y");
    for _ in 0..120 {
        if let FrameStatus::Halted(code) = e.run_frame().unwrap().status {
            assert_eq!(code, 0);
            return;
        }
        e.input_mut().set_keys([]);
    }
    panic!("Y must confirm normal exit");
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn three_kingdoms_develops_recruits_and_reloads_a_saved_city() {
    let tap = |e: &mut Emulator, key| {
        e.input_mut().set_keys([key]);
        advance(e, 1);
        e.input_mut().set_keys([]);
        advance(e, 59);
    };
    let word = |e: &Emulator, p| u16::from_le_bytes(e.vm().memory()[p..p + 2].try_into().unwrap());
    let path = "LAVA1 三国志/三国志.pac";
    let mut e = load(path);
    for frame in 0..1200 {
        e.input_mut()
            .set_keys(((300..480).contains(&frame) && (frame - 300) % 60 < 3).then_some(13));
        advance(&mut e, 1);
    }
    assert_eq!(e.vm().memory()[0x2008], 5);
    assert_eq!(word(&e, 0x2086), 800);
    capture(&e, "three-kingdoms-city");
    let mut campaign = e.clone();
    for _ in 0..3 {
        tap(&mut e, 13);
    }
    capture(&e, "three-kingdoms-officer");
    tap(&mut e, 13);
    assert_eq!(
        word(&e, 0x2086),
        820,
        "population development must increase the city's population"
    );
    assert_eq!(e.vm().memory()[0x2008], 4);
    for key in [21, 13, 21, 13, 22, 20] {
        tap(&mut e, key);
    }
    capture(&e, "three-kingdoms-recruit-number");
    tap(&mut e, 13);
    assert_eq!(
        word(&e, 0x208c),
        190,
        "recruiting 100 troops must cost 20 gold"
    );
    assert_eq!(word(&e, 0x2094), 100);
    assert_eq!(e.vm().memory()[0x2008], 3);
    capture(&e, "three-kingdoms-recruited");
    let original_save = e.files().file("/LavaData/sgz.sav").unwrap().to_vec();
    for key in [20, 13, 21, 21, 21, 21, 21, 13, 13] {
        tap(&mut e, key);
    }
    capture(&e, "three-kingdoms-save-slots");
    tap(&mut e, 13);
    let files = e.files_mut().take_dirty_files();
    let saved = files
        .iter()
        .find(|(name, _)| name.ends_with("sgz.sav"))
        .expect("saving a slot must update sgz.sav");
    assert_eq!(saved.1.len(), 16384);
    assert_ne!(saved.1, original_save);
    let mut restored = load(path);
    for (name, bytes) in files {
        restored.files_mut().import_file(&name, bytes);
    }
    advance(&mut restored, 300);
    for key in [13, 21, 13, 13] {
        tap(&mut restored, key);
    }
    advance(&mut restored, 600);
    assert_eq!(word(&restored, 0x2086), 820);
    assert_eq!(word(&restored, 0x208c), 190);
    assert_eq!(word(&restored, 0x2094), 100);
    capture(&restored, "three-kingdoms-restored");
    let map_point = |e: &mut Emulator, x: u16, y: u16| {
        assert_eq!(e.vm().pc(), 0x2ff7);
        let bp = e.vm().local_bp() as usize;
        for (address, target, lower, higher) in [(bp + 9, x, 23, 22), (bp + 11, y, 20, 21)] {
            for _ in 0..20 {
                let current = word(e, address);
                if current == target {
                    break;
                }
                tap(e, if current < target { higher } else { lower });
            }
            assert_eq!(word(e, address), target);
        }
        tap(e, 13);
    };
    tap(&mut campaign, 27);
    map_point(&mut campaign, 3, 2);
    assert_eq!(
        campaign.vm().memory()[0x2e32],
        4,
        "switch to the frontier city of Tianshui"
    );
    for key in [21, 13, 13] {
        tap(&mut campaign, key);
    }
    map_point(&mut campaign, 6, 2);
    for key in [13, 21, 21, 13, 13, 22, 20, 13, 22, 20, 13] {
        tap(&mut campaign, key);
    }
    assert_eq!(
        campaign.vm().pc(),
        0xa4e9,
        "the expedition must enter tactical unit selection"
    );
    assert_eq!(campaign.vm().memory()[0x3c1f], 0);
    capture(&campaign, "three-kingdoms-battle");
    for key in [13, 13, 22, 13] {
        tap(&mut campaign, key);
    }
    assert_ne!(campaign.vm().memory()[0x3c1f], 0);
    assert_eq!(
        campaign.vm().memory()[0x3c20],
        0,
        "Move must move the general from camp to its neighboring tile"
    );
    capture(&campaign, "three-kingdoms-battle-moved");
    for key in [13, 22, 22, 22, 22, 13] {
        tap(&mut campaign, key);
    }
    tap(&mut campaign, 13);
    capture(&campaign, "three-kingdoms-retreat");
    tap(&mut campaign, 13);
    assert_eq!(
        campaign.vm().pc(),
        0xda1,
        "retreat must return to strategic commands"
    );
    assert_eq!(campaign.vm().memory()[0x2e32], 4);
    capture(&campaign, "three-kingdoms-after-battle");
    for key in [13, 21, 21, 21, 21, 21, 13, 21] {
        tap(&mut restored, key);
    }
    for frame in 0..120 {
        restored.input_mut().set_keys((frame == 0).then_some(13));
        if let FrameStatus::Halted(code) = restored.run_frame().unwrap().status {
            assert_eq!(code, 0);
            return;
        }
    }
    panic!("the system Exit choice must terminate the game");
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn chinese_riddles_browses_all_categories_reveals_answers_and_exits() {
    fn tap(e: &mut Emulator, key: u8) {
        e.input_mut().set_keys([key]);
        advance(e, 1);
        e.input_mut().set_keys([]);
        advance(e, 300);
    }
    fn record(e: &Emulator) -> (usize, usize) {
        let memory = e.vm().memory();
        let key_bp = e.vm().local_bp() as usize;
        let viewer = usize::from(u16::from_le_bytes(
            memory[key_bp + 3..key_bp + 5].try_into().unwrap(),
        ));
        let int = |p| i32::from_le_bytes(memory[p..p + 4].try_into().unwrap());
        let index = usize::try_from(int(viewer + 5) + int(viewer + 17)).unwrap();
        let data = e.files().file("/LavaData/ZGDM.dat").unwrap();
        let base = 0x660 + 796 * 16;
        assert!(index < 7807);
        let expected = &data[base + index * 34..base + (index + 1) * 34];
        assert_eq!(
            &memory[viewer + 27..viewer + 43],
            &expected[2..18],
            "the viewer must read the selected question"
        );
        assert_eq!(
            &memory[viewer + 61..viewer + 77],
            &expected[18..34],
            "the viewer must read its matching answer"
        );
        (index, viewer)
    }
    let path = "LAVA1 中国灯谜/中国灯谜.lav";
    let boundaries = [0, 2972, 4620, 5389, 6106, 6764, 7807];
    for category in 0..6 {
        let mut e = load(path);
        advance(&mut e, 300);
        tap(&mut e, 13);
        for _ in 0..category {
            tap(&mut e, 21);
        }
        tap(&mut e, 13);
        let (first, viewer) = record(&e);
        assert!((boundaries[category]..boundaries[category + 1]).contains(&first));
        let question = e.display().indexed_frame().to_vec();
        capture(&e, &format!("chinese-riddles-{category}-question"));
        tap(&mut e, 13);
        assert_eq!(record(&e).0, first);
        assert!(
            question != e.display().indexed_frame(),
            "Enter must reveal the answer"
        );
        capture(&e, &format!("chinese-riddles-{category}-answer"));
        tap(&mut e, 21);
        let next = record(&e).0;
        assert_eq!(next, first + 1, "Down must advance one riddle");
        assert_eq!(
            &e.vm().memory()[viewer + 78..viewer + 82],
            &[0xb4, 0xf0, 0xb0, 0xb8],
            "changing riddles must restore the hidden-answer label"
        );
        tap(&mut e, 20);
        assert_eq!(record(&e).0, first, "Up must restore the previous riddle");
        tap(&mut e, b'a');
        assert_eq!(record(&e).0, first);
        assert!(
            question != e.display().indexed_frame(),
            "an ordinary letter must also reveal the answer"
        );
        tap(&mut e, 27);
        capture(&e, &format!("chinese-riddles-{category}-returned"));
    }
    let mut random = load(path);
    advance(&mut random, 300);
    tap(&mut random, 21);
    tap(&mut random, 13);
    let first = record(&random).0;
    tap(&mut random, 22);
    assert_ne!(
        record(&random).0,
        first,
        "random browsing must select another record"
    );
    capture(&random, "chinese-riddles-random");
    tap(&mut random, 27);
    for frame in 0..1200 {
        let key = if frame == 0 {
            Some(27)
        } else if frame % 60 < 3 {
            Some(13)
        } else {
            None
        };
        random.input_mut().set_keys(key);
        if let FrameStatus::Halted(code) = random.run_frame().unwrap().status {
            assert_eq!(code, 0);
            return;
        }
    }
    panic!("Escape from the title must exit the browser");
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn chinese_fireworks_rotates_connects_scores_pauses_and_confirms_exit() {
    let word = |e: &Emulator, p| u16::from_le_bytes(e.vm().memory()[p..p + 2].try_into().unwrap());
    let score =
        |e: &Emulator| i32::from_le_bytes(e.vm().memory()[0x20e5..0x20e9].try_into().unwrap());
    let mut e = load("LAVA1 中国烟火 1.1/中国烟火.lav");
    for frame in 0..1200 {
        e.input_mut()
            .set_keys(((300..480).contains(&frame) && (frame - 300) % 60 < 3).then_some(13));
        advance(&mut e, 1);
    }
    e.input_mut().set_keys([13]);
    advance(&mut e, 1);
    e.input_mut().set_keys([]);
    advance(&mut e, 299);
    capture(&e, "chinese-fireworks-board");
    let mut paused = e.clone();
    press(&mut paused, b'p');
    let frozen = paused.display().indexed_frame().to_vec();
    capture(&paused, "chinese-fireworks-paused");
    advance(&mut paused, 120);
    assert!(
        frozen == paused.display().indexed_frame(),
        "pause must freeze the game view"
    );
    press(&mut paused, 13);
    assert!(
        frozen != paused.display().indexed_frame(),
        "Enter must resume the fireworks"
    );
    let mut idle = e.clone();
    let mut elapsed = 0;
    for (x, y, rotation) in [
        (0, 5, 3),
        (1, 5, 1),
        (2, 5, 1),
        (3, 5, 3),
        (4, 5, 1),
        (5, 5, 0),
    ] {
        for (address, target, negative, positive) in [(0x2027, x, 23, 22), (0x2029, y, 20, 21)] {
            for _ in 0..300 {
                let current = word(&e, address);
                if current == target {
                    break;
                }
                e.input_mut()
                    .set_keys([if current < target { positive } else { negative }]);
                advance(&mut e, 1);
                elapsed += 1;
            }
            e.input_mut().set_keys([]);
            advance(&mut e, 6);
            elapsed += 6;
            assert_eq!(
                word(&e, address),
                target,
                "arrows must reach the selected fuse"
            );
        }
        let cell = 0x35ab + (usize::from(x) * 20 + usize::from(y)) * 12;
        for _ in 0..10 {
            if word(&e, cell + 8) == rotation {
                break;
            }
            e.input_mut().set_keys(*b"z");
            advance(&mut e, 1);
            e.input_mut().set_keys([]);
            advance(&mut e, 6);
            elapsed += 7;
        }
        assert_eq!(
            word(&e, cell + 8),
            rotation,
            "Z must rotate the fuse to its connected orientation"
        );
    }
    capture(&e, "chinese-fireworks-connected");
    for _ in 0..12000 {
        e.input_mut().set_keys([]);
        advance(&mut e, 1);
        elapsed += 1;
        if score(&e) > 0 {
            break;
        }
    }
    advance(&mut idle, elapsed);
    assert!(
        score(&e) > score(&idle),
        "connecting the fuses must light fireworks and score above the idle branch"
    );
    capture(&e, "chinese-fireworks-scored");

    let confirm = |e: &mut Emulator| {
        let mut waiting = false;
        for _ in 0..120 {
            e.input_mut().set_keys([27]);
            if e.run_frame().unwrap().status == FrameStatus::WaitingForInput {
                waiting = true;
                break;
            }
        }
        e.input_mut().set_keys([]);
        assert!(waiting, "Escape must open a confirmation prompt");
    };
    confirm(&mut e);
    capture(&e, "chinese-fireworks-confirm-exit");
    let confirmation = e.display().indexed_frame().to_vec();
    press(&mut e, b'n');
    assert!(e.vm().is_running());
    assert!(
        confirmation != e.display().indexed_frame(),
        "N must close the confirmation and resume play"
    );
    confirm(&mut e);
    press(&mut e, b'y');
    capture(&e, "chinese-fireworks-name");
    for key in *b"emu" {
        press(&mut e, key);
    }
    press(&mut e, 13);
    capture(&e, "chinese-fireworks-high-score");
    let files = e.files_mut().take_dirty_files();
    let saved = files
        .iter()
        .find(|(name, _)| name.ends_with("firefig.dat"))
        .expect("the leaderboard must be saved");
    assert_eq!(
        i32::from_le_bytes(saved.1[1..5].try_into().unwrap()),
        score(&e)
    );
    assert_eq!(&saved.1[21..24], b"emu");
    press(&mut e, 13);
    capture(&e, "chinese-fireworks-menu");
    press(&mut e, 21);
    press(&mut e, 21);
    press(&mut e, 22);
    for frame in 0..1200 {
        e.input_mut().set_keys((frame % 60 < 3).then_some(13));
        if let FrameStatus::Halted(code) = e.run_frame().unwrap().status {
            assert_eq!(code, 0);
            return;
        }
    }
    panic!("Stop Fireworks must exit after the closing pages");
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn xiangqi_moves_captures_undoes_and_reloads_a_saved_game() {
    fn start(e: &mut Emulator) {
        for frame in 0..1200 {
            e.input_mut()
                .set_keys(((300..480).contains(&frame) && (frame - 300) % 60 < 3).then_some(13));
            advance(e, 1);
        }
    }
    fn point(e: &mut Emulator, row: u16, col: u16) {
        let bp = e.vm().local_bp() as usize;
        for (address, target, negative, positive) in
            [(bp + 12, row, 20, 21), (bp + 14, col, 23, 22)]
        {
            for _ in 0..16 {
                let current =
                    u16::from_le_bytes(e.vm().memory()[address..address + 2].try_into().unwrap());
                if current == target {
                    break;
                }
                press(e, if current < target { positive } else { negative });
            }
            assert_eq!(
                u16::from_le_bytes(e.vm().memory()[address..address + 2].try_into().unwrap()),
                target
            );
        }
    }
    fn board(e: &Emulator) -> &[u8] {
        &e.vm().memory()[0x274a..0x27b8]
    }
    fn cell(e: &Emulator, row: usize, col: usize) -> u8 {
        board(e)[row * 11 + col]
    }
    let path = "LAVA1 中国象棋/象棋90.lav";
    let mut e = load(path);
    start(&mut e);
    capture(&e, "xiangqi-board");
    assert_eq!(cell(&e, 5, 4), 1);
    assert_eq!(cell(&e, 5, 7), 11);
    point(&mut e, 5, 4);
    press(&mut e, 13);
    point(&mut e, 5, 5);
    press(&mut e, 13);
    assert_eq!(cell(&e, 5, 4), 0);
    assert_eq!(cell(&e, 5, 5), 1);
    point(&mut e, 5, 7);
    press(&mut e, 13);
    point(&mut e, 5, 6);
    press(&mut e, 13);
    assert_eq!(cell(&e, 5, 7), 0);
    assert_eq!(cell(&e, 5, 6), 11);
    let before_capture = board(&e).to_vec();
    point(&mut e, 5, 5);
    press(&mut e, 13);
    point(&mut e, 5, 6);
    press(&mut e, 13);
    assert_eq!(cell(&e, 5, 5), 0);
    assert_eq!(
        cell(&e, 5, 6),
        1,
        "the black pawn must capture the red pawn"
    );
    let captured = board(&e).to_vec();
    capture(&e, "xiangqi-captured");
    press(&mut e, b'i');
    assert_eq!(board(&e), before_capture);
    capture(&e, "xiangqi-undone");
    press(&mut e, b'o');
    assert_eq!(board(&e), captured);
    for key in [29, b'y', b'e', b'm', b'u', 13, 13] {
        press(&mut e, key);
    }
    let files = e.files_mut().take_dirty_files();
    let saved = files
        .iter()
        .find(|(name, _)| name.ends_with("emu.cxq"))
        .expect("F2 must write the named game");
    assert!(saved.1.starts_with(b"[SCXQ_CXQ]"));
    capture(&e, "xiangqi-saved");
    let mut restored = load(path);
    for (name, bytes) in files {
        restored.files_mut().import_file(&name, bytes);
    }
    start(&mut restored);
    for key in [30, b'y'] {
        press(&mut restored, key);
    }
    capture(&restored, "xiangqi-file-picker");
    for key in [13, 13] {
        press(&mut restored, key);
    }
    capture(&restored, "xiangqi-restored");
    assert_eq!(
        board(&restored),
        captured,
        "loading CXQ must restore the moved and captured pieces"
    );
    let mut text_record = load(path);
    start(&mut text_record);
    let initial = board(&text_record).to_vec();
    let text_keys = [30, b'y', 13, b'n', 13, 13];
    for frame in 0..1200 {
        text_record.input_mut().set_keys(
            (frame / 60 < text_keys.len() && frame % 60 < 3)
                .then_some(text_keys[(frame / 60) % text_keys.len()]),
        );
        advance(&mut text_record, 1);
    }
    capture(&text_record, "xiangqi-text-record-start");
    assert_eq!(board(&text_record), initial);
    press(&mut text_record, b'o');
    advance(&mut text_record, 120);
    assert_eq!(cell(&text_record, 3, 4), 0);
    assert_eq!(cell(&text_record, 3, 5), 1);
    assert_ne!(board(&text_record), initial);
    capture(&text_record, "xiangqi-text-record");
    press(&mut text_record, b'i');
    advance(&mut text_record, 120);
    assert_eq!(board(&text_record), initial);
    press(&mut restored, 27);
    capture(&restored, "xiangqi-confirm-exit");
    press(&mut restored, b'n');
    assert_eq!(board(&restored), captured);
    press(&mut restored, 27);
    for frame in 0..1200 {
        let key = if frame < 3 {
            Some(b'y')
        } else if frame % 60 < 3 {
            Some(13)
        } else {
            None
        };
        restored.input_mut().set_keys(key);
        if let FrameStatus::Halted(code) = restored.run_frame().unwrap().status {
            assert_eq!(code, 255);
            return;
        }
    }
    panic!("confirmed exit must terminate normally");
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn chinese_mahjong_draws_selects_discards_and_finishes_a_hand() {
    fn word(e: &Emulator, address: usize) -> u16 {
        u16::from_le_bytes(e.vm().memory()[address..address + 2].try_into().unwrap())
    }
    let mut e = load("LAVA1 中国麻将/中国麻将.lav");
    for frame in 0..1200 {
        e.input_mut()
            .set_keys(((300..480).contains(&frame) && (frame - 300) % 60 < 3).then_some(13));
        advance(&mut e, 1);
    }
    capture(&e, "chinese-mahjong-dealt");
    let wall = word(&e, 0x2186);
    assert_eq!(word(&e, 0x2210), 0);
    assert_eq!(e.vm().memory()[0x21a9], 254);
    press(&mut e, 13);
    assert_eq!(word(&e, 0x2186), wall - 1);
    assert!(e.vm().memory()[0x21a9] < 34);
    assert_eq!(word(&e, 0x221c), 13);
    capture(&e, "chinese-mahjong-drawn");
    press(&mut e, 23);
    assert_eq!(word(&e, 0x221c), 12);
    press(&mut e, 22);
    assert_eq!(word(&e, 0x221c), 13);
    press(&mut e, 23);
    let mut remaining = e.vm().memory()[0x219c..0x21aa].to_vec();
    remaining.remove(12);
    remaining.sort_unstable();
    remaining.push(254);
    capture(&e, "chinese-mahjong-selected");
    press(&mut e, 13);
    advance(&mut e, 120);
    assert_eq!(&e.vm().memory()[0x219c..0x21aa], remaining);
    assert!(
        word(&e, 0x2186) < wall - 1,
        "the computer must take its turn"
    );
    capture(&e, "chinese-mahjong-discarded");
    let mut finished = false;
    for frame in 0..18000 {
        let key = if word(&e, 0x2186) == 0 { 27 } else { 13 };
        e.input_mut().set_keys((frame % 60 < 3).then_some(key));
        advance(&mut e, 1);
        if word(&e, 0x2210) != 0 {
            finished = true;
            break;
        }
    }
    assert!(finished, "draw/discard input must reach a hand result");
    let result = word(&e, 0x2210);
    assert!((1..=3).contains(&result));
    e.input_mut().set_keys([]);
    advance(&mut e, 120);
    capture(&e, "chinese-mahjong-hand-result");
    assert_eq!(word(&e, 0x2210), result);
    press(&mut e, 13);
    advance(&mut e, 600);
    assert_eq!(word(&e, 0x2210), 0);
    assert!(word(&e, 0x2186) > 50);
    assert!(
        e.vm().memory()[0x219c..0x21a9]
            .iter()
            .all(|&tile| tile < 34)
    );
    capture(&e, "chinese-mahjong-next-hand");
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn school_legend_versions_move_use_medicine_and_reload_progress() {
    fn tap(e: &mut Emulator, key: u8) {
        hold(e, key, 1);
        advance(e, 29);
    }
    fn menu(e: &mut Emulator) {
        hold(e, 25, 18);
        advance(e, 12);
    }
    fn replay(e: &mut Emulator, keys: &[u8], held: usize, frames: usize) {
        for frame in 0..frames {
            e.input_mut().set_keys(
                (frame / 60 < keys.len() && frame % 60 < held)
                    .then_some(keys[(frame / 60) % keys.len()]),
            );
            advance(e, 1);
        }
    }
    fn word(e: &Emulator, address: usize) -> u16 {
        u16::from_le_bytes(e.vm().memory()[address..address + 2].try_into().unwrap())
    }
    fn on_map(e: &Emulator) -> bool {
        let bp = e.vm().local_bp() as usize;
        if bp == 0x614f {
            return true;
        }
        let m = e.vm().memory();
        let parent = u16::from_le_bytes(m[bp + 3..bp + 5].try_into().unwrap());
        let return_pc =
            u32::from(m[bp]) | (u32::from(m[bp + 1]) << 8) | (u32::from(m[bp + 2]) << 16);
        parent == 0x614f && return_pc == 0x8798
    }
    for (version, path) in [
        ("loose", "LAVA1 中学传奇/未打包的/中学传奇.lav"),
        ("pac", "LAVA1 中学传奇/打包的/SCH_RPGPACK.pac"),
    ] {
        let mut e = load(path);
        for frame in 0..2400 {
            e.input_mut()
                .set_keys(((300..1020).contains(&frame) && (frame - 300) % 60 < 3).then_some(13));
            advance(&mut e, 1);
        }
        capture(&e, &format!("school-{version}-town"));
        let mut battle = e.clone();
        let mut idle = e.clone();
        advance(&mut idle, 60);
        hold(&mut e, 21, 18);
        advance(&mut e, 12);
        assert_ne!(e.display().indexed_frame(), idle.display().indexed_frame());
        capture(&e, &format!("school-{version}-moved"));
        advance(&mut e, 600);
        menu(&mut e);
        capture(&e, &format!("school-{version}-original-stats"));
        let original_stats = e.display().indexed_frame().to_vec();
        hold(&mut e, 22, 18);
        advance(&mut e, 12);
        hold(&mut e, 13, 18);
        advance(&mut e, 12);
        for key in [22, 22, 22, 22, 13] {
            tap(&mut e, key);
        }
        capture(&e, &format!("school-{version}-medicine"));
        tap(&mut e, b'y');
        capture(&e, &format!("school-{version}-medicine-used"));
        assert_eq!(
            u16::from_le_bytes(e.vm().memory()[0x4322..0x4324].try_into().unwrap()),
            300
        );
        for _ in 0..3 {
            tap(&mut e, 27);
        }
        menu(&mut e);
        let upgraded_stats = e.display().indexed_frame().to_vec();
        assert!(
            upgraded_stats != original_stats,
            "medicine must change the displayed character statistics"
        );
        capture(&e, &format!("school-{version}-upgraded"));
        for key in [22, 22, 22, 22, 13] {
            hold(&mut e, key, 18);
            advance(&mut e, 12);
        }
        tap(&mut e, 13);
        tap(&mut e, 13);
        let files = e.files_mut().take_dirty_files();
        assert!(
            files
                .iter()
                .any(|(name, bytes)| name.ends_with("sch_save.dat") && !bytes.is_empty())
        );
        capture(&e, &format!("school-{version}-saved"));
        let mut restored = load(path);
        for (name, bytes) in files {
            restored.files_mut().import_file(&name, bytes);
        }
        advance(&mut restored, 300);
        tap(&mut restored, 21);
        tap(&mut restored, 13);
        advance(&mut restored, 1800);
        tap(&mut restored, 13);
        advance(&mut restored, 600);
        tap(&mut restored, 13);
        for _ in 0..3 {
            tap(&mut restored, 27);
        }
        advance(&mut restored, 120);
        capture(&restored, &format!("school-{version}-restored-map"));
        menu(&mut restored);
        capture(&restored, &format!("school-{version}-restored-stats"));
        assert_eq!(
            u16::from_le_bytes(restored.vm().memory()[0x4322..0x4324].try_into().unwrap()),
            300
        );
        assert!(
            restored.display().indexed_frame()[160 * 13..] == upgraded_stats[160 * 13..],
            "loading must preserve the upgraded statistics"
        );

        battle.input_mut().set_keys([]);
        advance(&mut battle, 1);
        let mut route = vec![21; 2];
        route.extend([23; 4]);
        route.extend([21; 4]);
        route.extend([22; 14]);
        route.push(20);
        replay(&mut battle, &route, 8, route.len() * 60);
        replay(&mut battle, &[20], 8, 1800);
        replay(&mut battle, &[20, 20, 20, 13, 13, 13, 13, 13], 8, 1800);
        assert_eq!(battle.vm().memory()[0x43bc], 2);
        capture(&battle, &format!("school-{version}-training-area"));
        let mut walk = vec![21; 8];
        walk.extend([23; 6]);
        walk.extend([13; 4]);
        replay(&mut battle, &walk, 8, 3000);
        let search = [22, 22, 23, 23, 20, 20, 21, 21].repeat(10);
        replay(&mut battle, &search, 8, 5400);
        let enemy_hp = word(&battle, 0x4398);
        assert!(enemy_hp > 0);
        assert_eq!(word(&battle, 0x4312), 200);
        capture(&battle, &format!("school-{version}-battle"));
        replay(&mut battle, &[23; 5], 18, 300);
        for _ in 0..12 {
            replay(&mut battle, &[13], 18, 60);
            if word(&battle, 0x4398) < enemy_hp {
                break;
            }
        }
        assert!(
            word(&battle, 0x4398) < enemy_hp,
            "normal attack must damage the enemy"
        );
        advance(&mut battle, 60);
        capture(&battle, &format!("school-{version}-attack"));
        for _ in 0..12 {
            replay(&mut battle, &[13], 18, 60);
            if word(&battle, 0x4312) < 200 {
                break;
            }
        }
        assert!(word(&battle, 0x4312) < 200, "the enemy must counterattack");
        advance(&mut battle, 60);
        capture(&battle, &format!("school-{version}-counterattack"));
        for _ in 0..60 {
            replay(&mut battle, &[13], 18, 60);
            if word(&battle, 0x4328) > 0 {
                break;
            }
        }
        assert!(word(&battle, 0x4312) > 0);
        assert!(word(&battle, 0x4316) > 0, "victory must award experience");
        assert!(word(&battle, 0x4328) > 0, "victory must award money");
        for _ in 0..12 {
            if on_map(&battle) {
                break;
            }
            tap(&mut battle, 13);
        }
        assert!(
            on_map(&battle),
            "acknowledging victory must return to the map"
        );
        capture(&battle, &format!("school-{version}-victory-map"));
        let position = |e: &Emulator| {
            (
                u16::from(e.vm().memory()[0x43b8]) + u16::from(e.vm().memory()[0x43b6]),
                u16::from(e.vm().memory()[0x43b9]) + u16::from(e.vm().memory()[0x43b7]),
            )
        };
        let before_move = position(&battle);
        let mut moved = false;
        for key in [22, 23, 20, 21] {
            let mut candidate = battle.clone();
            replay(&mut candidate, &[key, key], 8, 120);
            if position(&candidate) != before_move {
                battle = candidate;
                moved = true;
                break;
            }
        }
        assert!(
            moved,
            "the victorious character must resume walking on the map"
        );
        capture(&battle, &format!("school-{version}-after-victory"));
    }
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn five_in_a_row_versions_win_undo_save_reload_and_restart() {
    fn tap(e: &mut Emulator, key: u8) {
        press(e, key);
        advance(e, 27);
    }
    fn start(e: &mut Emulator) {
        for frame in 0..1800 {
            e.input_mut()
                .set_keys(((300..480).contains(&frame) && (frame - 300) % 60 < 3).then_some(13));
            advance(e, 1);
        }
    }
    fn point(e: &mut Emulator, cursor: usize, x: u8, y: u8) {
        for (axis, target, negative, positive) in [(0, x, 23, 22), (1, y, 20, 21)] {
            for _ in 0..40 {
                let m = e.vm().memory();
                let current = if axis == 0 {
                    m[cursor]
                } else {
                    m[cursor + 1] + m[cursor - 2]
                };
                if current == target {
                    break;
                }
                tap(e, if current < target { positive } else { negative });
            }
            let m = e.vm().memory();
            assert_eq!(
                if axis == 0 {
                    m[cursor]
                } else {
                    m[cursor + 1] + m[cursor - 2]
                },
                target
            );
        }
    }
    fn board(e: &Emulator, cursor: usize) -> &[u8] {
        &e.vm().memory()[cursor + 10..cursor + 235]
    }
    for (version, path, cursor) in [
        ("mono", "WZLZ_黑白.lav", 0x209a),
        ("gray", "WZLZ_16灰度.lav", 0x21ca),
    ] {
        let path = fixture_path(path);
        let mut e = load(&path);
        start(&mut e);
        assert_eq!(board(&e, cursor)[7 * 15 + 7], 2);
        capture(&e, &format!("five-in-a-row-{version}-board"));
        tap(&mut e, 25);
        capture(&e, &format!("five-in-a-row-{version}-help"));
        tap(&mut e, 13);
        for (turn, (x, y)) in [
            (0, 0),
            (8, 7),
            (0, 1),
            (9, 7),
            (0, 2),
            (10, 7),
            (0, 3),
            (11, 7),
        ]
        .into_iter()
        .enumerate()
        {
            point(&mut e, cursor, x, y);
            tap(&mut e, 13);
            assert_eq!(
                board(&e, cursor)[usize::from(x) * 15 + usize::from(y)],
                if turn % 2 == 0 { 1 } else { 2 }
            );
        }
        assert!((7..=11).all(|x| board(&e, cursor)[x * 15 + 7] == 2));
        assert_eq!(board(&e, cursor).iter().filter(|&&v| v != 0).count(), 9);
        let won = board(&e, cursor).to_vec();
        capture(&e, &format!("five-in-a-row-{version}-won"));
        tap(&mut e, 13);
        tap(&mut e, b'i');
        assert_eq!(board(&e, cursor)[11 * 15 + 7], 0);
        capture(&e, &format!("five-in-a-row-{version}-undone"));
        tap(&mut e, b'o');
        assert_eq!(board(&e, cursor), won);
        tap(&mut e, 13);
        for key in [29, b'y', b'e', b'm', b'u', 13, 13] {
            tap(&mut e, key);
        }
        capture(&e, &format!("five-in-a-row-{version}-save-result"));
        let files = e.files_mut().take_dirty_files();
        assert!(
            files
                .iter()
                .any(|(name, bytes)| name.ends_with("emu.5lz") && bytes.starts_with(b"[WZLZ_5LZ]"))
        );
        capture(&e, &format!("five-in-a-row-{version}-saved"));
        let mut restored = load(&path);
        for (name, bytes) in files {
            restored.files_mut().import_file(&name, bytes);
        }
        start(&mut restored);
        for key in [30, b'y'] {
            tap(&mut restored, key);
        }
        capture(&restored, &format!("five-in-a-row-{version}-file-picker"));
        for key in [13, 13] {
            tap(&mut restored, key);
        }
        advance(&mut restored, 120);
        assert_eq!(board(&restored, cursor), won);
        capture(&restored, &format!("five-in-a-row-{version}-restored"));
        for key in [28, b'n'] {
            tap(&mut restored, key);
        }
        assert_eq!(board(&restored, cursor), won);
        for key in [28, b'y'] {
            tap(&mut restored, key);
        }
        assert!(board(&restored, cursor).iter().all(|&v| v == 0));
        capture(&restored, &format!("five-in-a-row-{version}-new-game"));
        for key in [27, b'n'] {
            tap(&mut restored, key);
        }
        assert!(board(&restored, cursor).iter().all(|&v| v == 0));
        tap(&mut restored, 27);
        let mut exited = false;
        for frame in 0..1200 {
            restored.input_mut().set_keys(if frame < 3 {
                Some(b'y')
            } else {
                (frame % 60 < 3).then_some(13)
            });
            if let FrameStatus::Halted(code) = restored.run_frame().unwrap().status {
                assert_eq!(code, 255);
                exited = true;
                break;
            }
        }
        assert!(
            exited,
            "confirmed exit must reach the closing page and terminate"
        );
    }
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn five_tigers_scores_complete_boards_and_restarts() {
    fn word(e: &Emulator, at: usize) -> u16 {
        u16::from_le_bytes(e.vm().memory()[at..at + 2].try_into().unwrap())
    }
    fn point(e: &mut Emulator, x: u16, y: u16, letters: bool) {
        for (at, target, neg, pos) in [
            (
                0x2004,
                x,
                if letters { b'a' } else { 23 },
                if letters { b'd' } else { 22 },
            ),
            (
                0x2006,
                y,
                if letters { b'w' } else { 20 },
                if letters { b's' } else { 21 },
            ),
        ] {
            for _ in 0..12 {
                let value = word(e, at);
                if value == target {
                    break;
                }
                press(e, if value < target { pos } else { neg });
            }
            assert_eq!(word(e, at), target);
        }
    }
    let mut e = load(&fixture_path("Fivet.lav"));
    advance(&mut e, 300);
    press(&mut e, 13);
    capture(&e, "five-tigers-empty");
    press(&mut e, b'q');
    let first = e.vm().memory()[0x2008..0x2021].to_vec();
    assert_eq!(e.vm().memory()[0x2025], 1);
    for key in [13, b'p', b'z'] {
        press(&mut e, key);
    }
    assert_eq!(&e.vm().memory()[0x2008..0x2021], first);
    assert_eq!(e.vm().memory()[0x2025], 1);
    press(&mut e, 28);
    assert!(e.vm().memory()[0x2008..0x2021].iter().all(|&v| v == 0));
    for scenario in 0..2 {
        let mut black = Vec::new();
        let mut white = Vec::new();
        let mut expected = [0u8; 25];
        for x in 0..5u16 {
            for y in 0..5u16 {
                let is_black = if scenario == 0 {
                    (x + y) % 2 == 0
                } else {
                    y == 0 || x < 2
                };
                expected[usize::from(x * 5 + y)] = if is_black { 1 } else { 2 };
                if is_black {
                    black.push((x, y));
                } else {
                    white.push((x, y));
                }
            }
        }
        assert_eq!((black.len(), white.len()), (13, 12));
        for turn in 0..25 {
            let (x, y) = if turn % 2 == 0 {
                black[turn / 2]
            } else {
                white[turn / 2]
            };
            point(&mut e, x, y, turn % 2 == 0);
            press(&mut e, [13, b'q', b'z', b'p'][turn % 4]);
            assert_eq!(e.vm().memory()[0x2025], (turn + 1) as u8);
            assert_eq!(
                e.vm().memory()[0x2008 + usize::from(x * 5 + y)],
                expected[usize::from(x * 5 + y)]
            );
        }
        assert_eq!(&e.vm().memory()[0x2008..0x2021], expected);
        assert_eq!(e.vm().memory()[0x2022], 18);
        assert_eq!(e.vm().memory()[0x2023], if scenario == 0 { 12 } else { 8 });
        capture(&e, &format!("five-tigers-result-{scenario}"));
        press(&mut e, 13);
        assert!(e.vm().memory()[0x2008..0x2021].iter().all(|&v| v == 0));
        assert_eq!(&e.vm().memory()[0x2022..0x2026], &[0, 0, 1, 0]);
    }
    capture(&e, "five-tigers-restarted");
    e.input_mut().set_keys([27]);
    let mut exited = false;
    for _ in 0..120 {
        if let FrameStatus::Halted(code) = e.run_frame().unwrap().status {
            assert_eq!(code, 0);
            exited = true;
            break;
        }
    }
    assert!(exited, "Escape must exit the game");
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn yahtzee_holds_rerolls_scores_every_category_and_finishes() {
    fn tap(e: &mut Emulator, key: u8) {
        press(e, key);
        for _ in 0..3000 {
            let result = e.run_frame().unwrap();
            assert!(!matches!(result.status, FrameStatus::Halted(_)));
            if result.status == FrameStatus::WaitingForInput {
                return;
            }
        }
        panic!("the animation must return to input waiting");
    }
    fn word(e: &Emulator, at: usize) -> u16 {
        u16::from_le_bytes(e.vm().memory()[at..at + 2].try_into().unwrap())
    }
    fn expected(dice: &[u8], category: usize) -> u16 {
        let mut counts = [0u16; 7];
        for &die in dice {
            assert!((1..=6).contains(&die));
            counts[usize::from(die)] += 1;
        }
        let sum = dice.iter().map(|&d| u16::from(d)).sum();
        match category {
            1..=6 => counts[category] * category as u16,
            7 => {
                if counts.iter().filter(|&&n| n >= 2).count() >= 2 {
                    sum
                } else {
                    0
                }
            }
            8 => {
                if counts.iter().any(|&n| n >= 3) {
                    sum
                } else {
                    0
                }
            }
            9 => {
                if counts.iter().any(|&n| n >= 4) {
                    sum
                } else {
                    0
                }
            }
            10 => {
                if counts.contains(&2) && counts.contains(&3) {
                    25
                } else {
                    0
                }
            }
            11 => {
                if (1..=3).any(|start| (start..start + 4).all(|v| counts[v] > 0)) {
                    30
                } else {
                    0
                }
            }
            12 => {
                if (1..=2).any(|start| (start..start + 5).all(|v| counts[v] > 0)) {
                    40
                } else {
                    0
                }
            }
            13 => sum,
            14 => {
                if counts.contains(&5) {
                    50
                } else {
                    0
                }
            }
            _ => unreachable!(),
        }
    }
    let mut e = load(&fixture_path("Yahtzee.lav"));
    for frame in 0..1800 {
        e.input_mut()
            .set_keys(((300..480).contains(&frame) && (frame - 300) % 60 < 3).then_some(13));
        advance(&mut e, 1);
    }
    assert_eq!(e.vm().memory()[0x317b], 1);
    tap(&mut e, 32);
    assert_eq!(e.vm().memory()[0x3184], 1);
    tap(&mut e, 22);
    assert_eq!(e.vm().memory()[0x317c], 2);
    tap(&mut e, 32);
    assert_eq!(e.vm().memory()[0x3185], 1);
    tap(&mut e, 23);
    let held = e.vm().memory()[0x317f];
    let held_second = e.vm().memory()[0x3180];
    while e.vm().memory()[0x317e] < 2 {
        tap(&mut e, 13);
        assert_eq!(e.vm().memory()[0x317f], held);
        assert_eq!(e.vm().memory()[0x3180], held_second);
    }
    capture(&e, "yahtzee-held-die");
    let exhausted = e.vm().memory()[0x317f..0x3184].to_vec();
    tap(&mut e, 13);
    assert_eq!(&e.vm().memory()[0x317f..0x3184], exhausted);
    tap(&mut e, 32);
    assert_eq!(e.vm().memory()[0x3184], 0);
    tap(&mut e, 22);
    tap(&mut e, 32);
    assert_eq!(e.vm().memory()[0x3185], 0);
    tap(&mut e, 23);
    tap(&mut e, 27);
    tap(&mut e, b'n');
    let mut upper = 0u16;
    let mut lower = 0u16;
    for category in 1..=14 {
        if e.vm().memory()[0x317b] == 1 {
            tap(&mut e, 20);
        }
        for _ in 0..14 {
            if usize::from(e.vm().memory()[0x317d]) == category {
                break;
            }
            tap(&mut e, 22);
        }
        assert_eq!(usize::from(e.vm().memory()[0x317d]), category);
        let score = expected(&e.vm().memory()[0x317f..0x3184], category);
        assert_eq!(
            u16::from(e.vm().memory()[0x3189 + category - 1]),
            score,
            "category {category} preview"
        );
        if category <= 6 {
            upper += score;
        } else {
            lower += score;
        }
        tap(&mut e, 13);
        assert_eq!(e.vm().memory()[0x3197 + category - 1], 1);
        assert_eq!(
            word(&e, 0x3264),
            upper + lower + if upper >= 63 { 50 } else { 0 }
        );
        if category == 1 {
            if e.vm().memory()[0x317b] == 1 {
                tap(&mut e, 20);
            }
            let before = e.vm().memory()[0x317e];
            tap(&mut e, 13);
            assert_eq!(e.vm().memory()[0x317e], before);
            assert_eq!(
                e.vm().memory()[0x3197..0x31a5]
                    .iter()
                    .filter(|&&v| v == 1)
                    .count(),
                1
            );
        }
    }
    assert!(e.vm().memory()[0x3197..0x31a5].iter().all(|&v| v == 1));
    capture(&e, "yahtzee-final-score");
    tap(&mut e, 13);
    capture(&e, "yahtzee-menu-after-game");
    for key in [21, 21] {
        tap(&mut e, key);
    }
    e.input_mut().set_keys([13]);
    let mut exited = false;
    for _ in 0..120 {
        if let FrameStatus::Halted(code) = e.run_frame().unwrap().status {
            assert_eq!(code, 0);
            exited = true;
            break;
        }
    }
    assert!(exited, "the menu exit must terminate normally");
}
