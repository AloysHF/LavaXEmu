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
    fixture_path_with_size(filename, None)
}

fn fixture_path_with_size(filename: &str, size: Option<u64>) -> String {
    let root = PathBuf::from(std::env::var_os("LAVAX_GAME_ROOT").expect("set LAVAX_GAME_ROOT"));
    let mut directories = vec![root.clone()];
    let mut matches = Vec::new();
    while let Some(directory) = directories.pop() {
        for entry in std::fs::read_dir(directory).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                directories.push(path);
            } else if path.file_name().is_some_and(|name| name == filename)
                && size.is_none_or(|size| path.metadata().unwrap().len() == size)
            {
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
    for (name, path) in [
        ("packed", "LAVA1 新英雄坛说/新英雄传说.pac"),
        ("unpacked", "LAVA1 新英雄坛说/非pac/Hero.lav"),
    ] {
        let mut emulator = load(path);
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
        capture(&emulator, &format!("hero-{name}-village"));
        capture(&moved, &format!("hero-{name}-movement"));
        capture(&scrolled, &format!("hero-{name}-scroll"));
    }
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

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn fairy_fate_moves_saves_restores_and_resolves_a_battle() {
    fn replay(e: &mut Emulator, keys: &[u8], held: usize, start: usize, frames: usize) {
        for frame in 0..frames {
            let input = frame.saturating_sub(start);
            e.input_mut().set_keys(
                (frame >= start && input / 60 < keys.len() && input % 60 < held)
                    .then_some(keys[(input / 60) % keys.len()]),
            );
            advance(e, 1);
        }
    }
    fn word(e: &Emulator, address: usize) -> u16 {
        u16::from_le_bytes(e.vm().memory()[address..address + 2].try_into().unwrap())
    }
    fn value(e: &Emulator, address: usize) -> i32 {
        i32::from_le_bytes(e.vm().memory()[address..address + 4].try_into().unwrap())
    }
    fn location(e: &Emulator) -> (u16, u16, u16) {
        (word(e, 0x69a8), word(e, 0x69aa), word(e, 0x69b2))
    }
    for (version, filename) in [("loose", "仙境奇缘.lav"), ("package", "仙境奇缘.pac")] {
        let path = fixture_path(filename);
        let mut e = load(&path);
        replay(&mut e, &[13; 12], 18, 300, 3600);
        replay(&mut e, &[13; 60], 18, 0, 6000);
        assert_eq!(location(&e), (8, 10, 1));
        assert_eq!(value(&e, 0x69d2), 200);
        let mut battle = e.clone();
        replay(&mut e, b"dd", 3, 0, 600);
        assert_eq!(location(&e), (10, 10, 1));
        capture(&e, &format!("fairy-{version}-moved"));
        let field = e.display().indexed_frame().to_vec();
        replay(&mut e, &[31, 22, 22, 22, 13], 3, 0, 600);
        replay(&mut e, &[13; 3], 3, 0, 1200);
        capture(&e, &format!("fairy-{version}-saved"));
        let files = e.files_mut().take_dirty_files();
        assert!(
            files
                .iter()
                .any(|(name, bytes)| name.ends_with("xjsava1.dat") && bytes.len() == 952)
        );
        let mut restored = load(&path);
        for (name, bytes) in files {
            restored.files_mut().import_file(&name, bytes);
        }
        replay(&mut restored, &[13], 3, 300, 1800);
        replay(&mut restored, &[21, 13, 13], 3, 0, 1200);
        replay(&mut restored, &[13, 27, 27], 3, 0, 600);
        assert_eq!(location(&restored), (10, 10, 1));
        assert_eq!(value(&restored, 0x69d2), 200);
        // Loading resets the actor's facing; the surrounding map must match.
        assert!(
            restored
                .display()
                .indexed_frame()
                .iter()
                .zip(&field)
                .enumerate()
                .all(|(i, (a, b))| {
                    ((64..80).contains(&(i % 160)) && (32..64).contains(&(i / 160))) || a == b
                }),
            "loading must redraw the saved map"
        );
        capture(&restored, &format!("fairy-{version}-restored"));

        let mut route = vec![b's'; 30];
        for (key, count) in [
            (b'a', 15),
            (b's', 20),
            (b'd', 4),
            (b's', 50),
            (b'd', 12),
            (b's', 20),
            (b'd', 2),
            (b's', 18),
            (b'd', 5),
            (b's', 30),
        ] {
            route.extend(std::iter::repeat_n(key, count));
        }
        let frames = route.len() * 60 + 600;
        replay(&mut battle, &route, 3, 0, frames);
        assert_eq!(location(&battle), (25, 15, 3));
        let health = value(&battle, 0x69de);
        let enemy_health = value(&battle, 0x69fe);
        assert!((1..200).contains(&health), "the enemy must inflict damage");
        assert!(enemy_health > 0);
        capture(&battle, &format!("fairy-{version}-battle"));
        let mut attacks = vec![b'a'; 4];
        attacks.extend([13; 20]);
        replay(&mut battle, &attacks, 18, 0, 4800);
        assert!(
            value(&battle, 0x69fe) < enemy_health,
            "attacking must damage the enemy"
        );
        assert!(value(&battle, 0x69de) < health, "enemy turns must continue");
        capture(&battle, &format!("fairy-{version}-battle-damage"));
        let mut defeated = false;
        let mut exited = false;
        for frame in 0..12000 {
            battle
                .input_mut()
                .set_keys((frame / 60 < 100 && frame % 60 < 3).then_some(13));
            let status = battle.run_frame().unwrap().status;
            if value(&battle, 0x69de) <= 0 && !defeated {
                capture(&battle, &format!("fairy-{version}-defeated"));
                defeated = true;
            }
            if let FrameStatus::Halted(code) = status {
                assert_eq!(code, 0);
                exited = true;
                break;
            }
        }
        assert!(
            defeated && exited,
            "defeat must terminate normally after confirmation"
        );
        capture(&battle, &format!("fairy-{version}-game-over"));
    }
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn arbitrary_picture_puzzle_swaps_tiles_completes_all_pictures_and_exits() {
    fn tap(e: &mut Emulator, key: u8) {
        for frame in 0..60 {
            e.input_mut().set_keys((frame < 3).then_some(key));
            advance(e, 1);
        }
    }
    fn word(e: &Emulator, address: usize) -> u16 {
        u16::from_le_bytes(e.vm().memory()[address..address + 2].try_into().unwrap())
    }
    fn board(e: &Emulator) -> Vec<u16> {
        (0..24).map(|i| word(e, 0x3540 + i * 2)).collect()
    }
    fn select(e: &mut Emulator, target: usize) {
        let (x, y) = (word(e, 0x357f) as usize, word(e, 0x3581) as usize);
        let (tx, ty) = (target % 6, target / 6);
        for _ in 0..x.abs_diff(tx) {
            tap(e, if tx > x { 22 } else { 23 });
        }
        for _ in 0..y.abs_diff(ty) {
            tap(e, if ty > y { 21 } else { 20 });
        }
        assert_eq!((word(e, 0x357f), word(e, 0x3581)), (tx as u16, ty as u16));
        tap(e, 13);
    }
    let path = fixture_path("任意拼图.lav");
    let mut pictures = Vec::new();
    for (number, key) in (*b"bnmg").into_iter().enumerate() {
        let mut e = load(&path);
        advance(&mut e, 300);
        tap(&mut e, 13);
        tap(&mut e, key);
        let preview = e.display().indexed_frame().to_vec();
        assert!(
            !pictures.contains(&preview),
            "each choice must load a different picture"
        );
        pictures.push(preview.clone());
        capture(&e, &format!("picture-puzzle-{number}-preview"));
        tap(&mut e, 13);
        let mut tiles = board(&e);
        let goal: Vec<u16> = (0..24).collect();
        let mut sorted = tiles.clone();
        sorted.sort_unstable();
        assert_eq!(sorted, goal);
        assert_ne!(tiles, goal);
        tap(&mut e, 23);
        tap(&mut e, 20);
        assert_eq!((word(&e, 0x357f), word(&e, 0x3581)), (0, 0));
        assert_eq!(
            board(&e),
            tiles,
            "boundary presses must preserve every tile"
        );
        assert_eq!(word(&e, 0x358b), 0);
        capture(&e, &format!("picture-puzzle-{number}-shuffled"));
        let mut swaps = 0;
        for i in 0..24 {
            if tiles[i] == i as u16 {
                continue;
            }
            let j = tiles.iter().position(|&tile| tile == i as u16).unwrap();
            select(&mut e, i);
            assert_eq!(board(&e), tiles, "selection alone must not swap tiles");
            select(&mut e, j);
            tiles.swap(i, j);
            swaps += 1;
            assert_eq!(
                board(&e),
                tiles,
                "Enter must exchange exactly the selected pair"
            );
            assert_eq!(word(&e, 0x358b), swaps);
        }
        assert_eq!(board(&e), goal);
        assert_eq!(word(&e, 0x357b), 24);
        let (cx, cy) = (word(&e, 0x357f) as usize, word(&e, 0x3581) as usize);
        capture(&e, &format!("picture-puzzle-{number}-completed"));
        for y in 8..72 {
            for x in 8..104 {
                let cursor = (x - 8) / 16 == cx && (y - 8) / 16 == cy;
                let message = (32..92).contains(&x) && (32..44).contains(&y);
                if !cursor && !message {
                    assert_eq!(
                        e.display().indexed_frame()[y * 160 + x],
                        preview[y * 160 + x],
                        "restored picture {number} at {x},{y}"
                    );
                }
            }
        }
        let mut exited = false;
        for frame in 0..120 {
            e.input_mut().set_keys((frame < 3).then_some(13));
            if let FrameStatus::Halted(code) = e.run_frame().unwrap().status {
                assert_eq!(code, 0);
                exited = true;
                break;
            }
        }
        assert!(exited, "acknowledging completion must exit normally");
    }
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn russian_poker_moves_accelerates_locks_scores_and_finishes() {
    fn word(e: &Emulator, address: usize) -> u16 {
        u16::from_le_bytes(e.vm().memory()[address..address + 2].try_into().unwrap())
    }
    fn replay(e: &mut Emulator, key: u8, held: usize, frames: usize) {
        for frame in 0..frames {
            e.input_mut().set_keys((frame % 60 < held).then_some(key));
            advance(e, 1);
        }
    }
    let mut e = load(&fixture_path("俄罗斯扑克.lav"));
    advance(&mut e, 300);
    capture(&e, "russian-poker-title");
    replay(&mut e, 13, 3, 60);
    assert_eq!(word(&e, 0x2006), 0);
    assert_eq!(word(&e, 0x2008), 3);
    assert_eq!(word(&e, 0x200a), 11);
    assert_eq!(word(&e, 0x200c), 11);
    capture(&e, "russian-poker-first-card");
    for (key, column) in [(23, 0), (22, 6)] {
        let mut moved = e.clone();
        replay(&mut moved, key, 3, 720);
        assert_eq!(word(&moved, 0x2008), column);
        assert_eq!(word(&moved, 0x2006), 0);
        capture(&moved, &format!("russian-poker-boundary-{column}"));
    }
    let mut idle = e.clone();
    advance(&mut idle, 240);
    replay(&mut e, 21, 59, 240);
    assert_eq!(word(&idle, 0x2063), 0);
    assert_eq!(word(&idle, 0x2065), 0);
    assert_eq!(word(&e, 0x2063), 11);
    assert_eq!(word(&e, 0x2065), 11);
    assert_eq!(word(&e, 0x2006), 0);
    capture(&e, "russian-poker-two-locked-cards");
    replay(&mut e, 21, 59, 240);
    assert_eq!(word(&e, 0x2006), 20);
    capture(&e, "russian-poker-scored");
    let mut overflowed = false;
    for frame in 0..12000 {
        e.input_mut().set_keys((frame % 60 < 59).then_some(21));
        if let FrameStatus::Halted(code) = e.run_frame().unwrap().status {
            assert_eq!(code, 0);
            overflowed = true;
            break;
        }
    }
    assert!(overflowed, "stacking cards must reach a normal game end");
    assert_ne!(
        word(&e, 0x2055),
        0,
        "the stack must reach the top playable row"
    );
    assert!(word(&e, 0x2006) >= 20);
    capture(&e, "russian-poker-finished");
    let mut exited = false;
    for frame in 0..120 {
        idle.input_mut().set_keys((frame < 3).then_some(27));
        if let FrameStatus::Halted(code) = idle.run_frame().unwrap().status {
            assert_eq!(code, 1, "Escape explicitly requests exit(1)");
            exited = true;
            break;
        }
    }
    assert!(exited, "Escape must terminate play");
    capture(&idle, "russian-poker-escape-exit");
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn tetris_13_versions_rotate_clear_lines_and_preserve_ranking_names() {
    fn word(e: &Emulator, address: usize) -> u16 {
        u16::from_le_bytes(e.vm().memory()[address..address + 2].try_into().unwrap())
    }
    fn tap(e: &mut Emulator, key: u8, frames: usize) {
        for frame in 0..frames {
            e.input_mut().set_keys((frame < 3).then_some(key));
            advance(e, 1);
        }
    }
    fn grid(e: &Emulator) -> Vec<u8> {
        (1..11)
            .flat_map(|x| (0..24).map(move |y| e.vm().memory()[0x205c + x * 26 + y]))
            .collect()
    }
    let placements = [
        (0, 1),
        (0, 4),
        (0, 8),
        (1, 8),
        (3, 1),
        (1, 4),
        (1, 5),
        (0, 0),
        (2, 8),
        (1, 3),
        (1, 4),
        (0, 6),
        (1, 8),
        (0, 5),
        (1, 1),
        (2, 2),
        (0, 1),
        (1, 4),
        (0, 3),
        (2, 7),
        (3, 7),
        (0, 3),
        (0, 7),
        (0, 0),
        (0, 9),
        (0, 8),
        (1, 5),
        (2, 0),
        (1, 2),
        (0, 1),
        (1, 6),
        (0, 0),
        (1, 3),
        (0, 4),
        (0, 9),
        (0, 1),
        (0, 7),
        (0, 5),
        (2, 7),
        (1, 0),
    ];
    for (version, filename, extra_prompt) in [
        ("mono", "俄罗斯方块黑.lav", false),
        ("gray", "俄罗斯方块灰.lav", true),
    ] {
        let path = fixture_path(filename);
        let mut e = load(&path);
        advance(&mut e, 300);
        if extra_prompt {
            tap(&mut e, 13, 60);
        }
        tap(&mut e, 13, 60);
        tap(&mut e, 13, 2);
        assert_eq!(word(&e, 0x2042), 1);
        let mut paused = e.clone();
        paused.input_mut().release_all();
        tap(&mut paused, 13, 6);
        let picture = paused.display().indexed_frame().to_vec();
        let paused_grid = grid(&paused);
        let y = word(&paused, 0x2042);
        advance(&mut paused, 180);
        assert_eq!(word(&paused, 0x2042), y);
        assert_eq!(grid(&paused), paused_grid);
        assert_eq!(paused.display().indexed_frame(), picture);
        capture(&paused, &format!("tetris13-{version}-paused"));
        tap(&mut paused, 13, 6);
        advance(&mut paused, 30);
        assert_ne!(word(&paused, 0x2042), y, "resuming must restart gravity");

        let mut previous_lines = 0;
        let mut expected_score = 0;
        let mut cleared_sizes = std::collections::BTreeSet::new();
        for (n, (rotations, target)) in placements.into_iter().enumerate() {
            for _ in 0..rotations {
                let rotation = word(&e, 0x2058);
                tap(&mut e, 20, 6);
                assert_ne!(word(&e, 0x2058), rotation);
            }
            for _ in 0..12 {
                let x = word(&e, 0x2040);
                if x == target {
                    break;
                }
                tap(&mut e, if x < target { 22 } else { 23 }, 6);
            }
            assert_eq!(word(&e, 0x2040), target);
            let mut last_y = word(&e, 0x2042);
            let mut landed = false;
            for _ in 0..150 {
                tap(&mut e, 21, 6);
                let y = word(&e, 0x2042);
                if last_y > 3 && y <= 3 {
                    landed = true;
                    break;
                }
                last_y = y;
            }
            assert!(landed, "piece {n} must lock and spawn its successor");
            let lines = word(&e, 0x225f);
            let cleared = lines - previous_lines;
            if cleared > 0 {
                cleared_sizes.insert(cleared);
            }
            expected_score += 10 * cleared * cleared;
            assert_eq!(word(&e, 0x2241), expected_score);
            assert_eq!(
                grid(&e).iter().filter(|&&c| c != 0).count(),
                4 * (n + 1) - 10 * usize::from(lines)
            );
            previous_lines = lines;
        }
        assert_eq!(previous_lines, 14);
        assert_eq!(expected_score, 220);
        assert_eq!(cleared_sizes, [1, 2, 3].into_iter().collect());
        capture(&e, &format!("tetris13-{version}-cleared"));
        let mut ended = false;
        for frame in 0..6000 {
            e.input_mut().set_keys((frame % 60 < 59).then_some(21));
            if e.run_frame().unwrap().status == FrameStatus::WaitingForInput {
                ended = true;
                break;
            }
        }
        assert!(ended, "stacking must reach the ranking prompt");
        capture(&e, &format!("tetris13-{version}-game-over"));
        tap(&mut e, 13, 60);
        capture(&e, &format!("tetris13-{version}-name"));
        tap(&mut e, 13, 60);
        capture(&e, &format!("tetris13-{version}-ranking"));
        let ranking = e.display().indexed_frame()[..160 * 70].to_vec();
        let files = e.files_mut().take_dirty_files();
        let (_, saved) = files
            .iter()
            .find(|(name, _)| name.ends_with("elsboxsave.dat"))
            .expect("ranking file must be written");
        assert_eq!(saved.len(), 101);
        assert_eq!(u32::from_le_bytes(saved[..4].try_into().unwrap()), 220);
        assert_eq!(u16::from_le_bytes(saved[20..22].try_into().unwrap()), 14);
        assert_eq!(
            &saved[35..41],
            &[0xce, 0xc4, 0xc7, 0xfa, 0xd0, 0xc7],
            "the confirmed name must survive reused stack storage"
        );
        let mut restored = load(&path);
        for (name, bytes) in files {
            restored.files_mut().import_file(&name, bytes);
        }
        advance(&mut restored, 300);
        if extra_prompt {
            tap(&mut restored, 13, 60);
        }
        for _ in 0..3 {
            tap(&mut restored, 22, 60);
        }
        tap(&mut restored, 13, 60);
        capture(&restored, &format!("tetris13-{version}-ranking-restored"));
        assert_eq!(&restored.display().indexed_frame()[..160 * 70], ranking);
        tap(&mut restored, 27, 60);
        tap(&mut restored, 27, 60);
        capture(&restored, &format!("tetris13-{version}-exit-prompt"));
        let mut exited = false;
        for frame in 0..1200 {
            restored.input_mut().set_keys((frame < 3).then_some(b'y'));
            if let FrameStatus::Halted(code) = restored.run_frame().unwrap().status {
                assert_eq!(code, 0);
                exited = true;
                break;
            }
        }
        assert!(exited, "confirmed exit must terminate normally");
    }
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn adventure_island_iv_versions_clear_a_stage_and_restore_custom_controls() {
    fn word(e: &Emulator, address: usize) -> u16 {
        u16::from_le_bytes(e.vm().memory()[address..address + 2].try_into().unwrap())
    }
    fn tap(e: &mut Emulator, key: u8) {
        for frame in 0..60 {
            e.input_mut().set_keys((frame < 3).then_some(key));
            advance(e, 1);
        }
    }
    fn start(e: &mut Emulator) {
        for frame in 0..1800 {
            e.input_mut()
                .set_keys(((300..900).contains(&frame) && (frame - 300) % 60 < 3).then_some(13));
            advance(e, 1);
        }
        assert_eq!((word(e, 0x24f6), word(e, 0x24f8)), (32, 56));
        assert_eq!(word(e, 0x24dc), 0);
        assert_eq!(word(e, 0x25ea), 0);
    }
    let route = [
        (b'd', 16),
        (b'u', 4),
        (b'd', 48),
        (b'a', 8),
        (b'u', 4),
        (b'a', 16),
        (b'd', 16),
        (b'u', 4),
        (b'd', 40),
        (b'u', 4),
        (b'd', 64),
        (b'u', 4),
        (b'd', 16),
        (b'u', 12),
        (b'd', 8),
        (b'u', 4),
        (b'd', 8),
        (b'u', 4),
        (b'd', 24),
        (b'u', 4),
        (b'd', 40),
        (b'u', 4),
        (b'd', 72),
        (b'u', 8),
        (b'd', 8),
        (b'u', 4),
        (b'd', 8),
        (b'u', 4),
        (b'd', 8),
        (b'u', 8),
        (b'd', 8),
        (b'u', 4),
        (b'd', 8),
        (b'u', 4),
        (b'd', 8),
        (b'u', 8),
        (b'd', 80),
        (b'u', 4),
        (b'd', 24),
        (b'a', 8),
        (b'u', 4),
        (b'a', 16),
        (b'u', 4),
        (b'd', 16),
        (b'u', 12),
        (b'a', 8),
        (b'u', 4),
        (b'd', 16),
        (b'u', 12),
        (b'a', 8),
        (b'u', 4),
        (b'd', 16),
        (b'u', 4),
        (b'd', 8),
        (b'a', 272),
        (b'u', 4),
        (b'a', 32),
        (0, 12),
        (b'u', 4),
        (b'a', 24),
        (0, 12),
        (b'a', 32),
        (0, 12),
        (b'u', 4),
        (b'a', 24),
        (0, 32),
        (b'u', 6),
        (b'd', 59),
        (0, 1),
        (b'd', 59),
        (0, 1),
    ];
    for (version, filename, size) in [
        ("mono-loose", "冒险岛IV.lav", 26_437),
        ("mono-package", "冒险岛IV.pac", 90_121),
        ("gray-loose", "冒险岛IV.lav", 25_852),
        ("gray-package", "冒险岛IV.pac", 203_106),
    ] {
        let path = fixture_path_with_size(filename, Some(size));
        let mut e = load(&path);
        start(&mut e);
        let initial_scene = e.display().indexed_frame().to_vec();
        let mut config = e.clone();
        tap(&mut config, b'p');
        let paused = config.display().indexed_frame().to_vec();
        let position = (word(&config, 0x24f6), word(&config, 0x24f8));
        advance(&mut config, 180);
        assert_eq!((word(&config, 0x24f6), word(&config, 0x24f8)), position);
        assert_eq!(config.display().indexed_frame(), paused);
        capture(&config, &format!("adventure4-{version}-paused"));
        tap(&mut config, 13);
        for _ in 0..12 {
            config.input_mut().set_keys(*b"d");
            advance(&mut config, 1);
        }
        assert!(
            word(&config, 0x24f6) > position.0,
            "resuming must restore movement"
        );
        config.input_mut().release_all();
        for key in [
            b'p', 21, 13, 22, 22, 13, 21, 13, b'h', b'l', b'k', b'q', b'y',
        ] {
            tap(&mut config, key);
        }
        capture(&config, &format!("adventure4-{version}-controls-set"));
        for key in [27, 22, 22] {
            tap(&mut config, key);
        }
        let mut exited = false;
        for frame in 0..180 {
            config.input_mut().set_keys((frame < 3).then_some(13));
            if let FrameStatus::Halted(code) = config.run_frame().unwrap().status {
                assert_eq!(code, 0);
                exited = true;
                break;
            }
        }
        assert!(exited, "exiting must save the selected controls");
        let files = config.files_mut().take_dirty_files();
        let (_, saved) = files
            .iter()
            .find(|(name, _)| name.ends_with("mxdsav.dat"))
            .expect("configuration must be written");
        assert_eq!(saved.len(), 36);
        assert_eq!(&saved[28..], &[104, 0, 108, 0, 107, 0, 113, 0]);
        let mut restored = load(&path);
        for (name, bytes) in files {
            restored.files_mut().import_file(&name, bytes);
        }
        start(&mut restored);
        for old in *b"adup" {
            let mut branch = restored.clone();
            for _ in 0..12 {
                branch.input_mut().set_keys([old]);
                advance(&mut branch, 1);
            }
            assert_eq!((word(&branch, 0x24f6), word(&branch, 0x24f8)), (32, 56));
            assert_ne!(
                branch.run_frame().unwrap().status,
                FrameStatus::WaitingForInput
            );
        }
        for key in *b"hlkq" {
            let mut branch = restored.clone();
            for _ in 0..12 {
                branch.input_mut().set_keys([key]);
                advance(&mut branch, 1);
            }
            match key {
                b'h' => assert!(word(&branch, 0x24f6) < 32),
                b'l' => assert!(word(&branch, 0x24f6) > 32),
                b'k' => assert!(word(&branch, 0x24f8) < 56),
                b'q' => assert_eq!(
                    branch.run_frame().unwrap().status,
                    FrameStatus::WaitingForInput
                ),
                _ => unreachable!(),
            }
            capture(&branch, &format!("adventure4-{version}-restored-key-{key}"));
        }

        let mut frame = 0;
        let mut previous_life = word(&e, 0x25ec);
        let mut previous_keys = 0;
        let mut damaged = false;
        let mut respawned = false;
        let mut horizontal_scroll = false;
        let mut vertical_scroll = false;
        for (key, count) in route {
            for _ in 0..count {
                e.input_mut().set_keys((key != 0).then_some(key));
                advance(&mut e, 1);
                frame += 1;
                let life = word(&e, 0x25ec);
                let keys = word(&e, 0x25ea);
                assert!(life > 0);
                if life < previous_life {
                    assert!(keys >= previous_keys, "damage must preserve collected keys");
                    if !damaged {
                        capture(&e, &format!("adventure4-{version}-hurt"));
                    }
                    damaged = true;
                }
                if damaged
                    && keys > 0
                    && word(&e, 0x25c2) == 0
                    && (30..49).contains(&word(&e, 0x24f6))
                    && word(&e, 0x24f8) == 56
                {
                    respawned = true;
                }
                horizontal_scroll |= word(&e, 0x25c2) > 0;
                vertical_scroll |= word(&e, 0x25c4) == 1;
                previous_life = life;
                previous_keys = keys;
                if let Some(expected) = [(96, 1), (244, 2), (760, 3), (1204, 4)]
                    .into_iter()
                    .find_map(|(f, k)| (f == frame).then_some(k))
                {
                    assert_eq!(keys, expected);
                    capture(&e, &format!("adventure4-{version}-keys-{expected}"));
                }
            }
        }
        assert_eq!(frame, 1350);
        assert!(damaged && respawned && horizontal_scroll && vertical_scroll);
        assert_eq!(word(&e, 0x24dc), 1, "the exit must advance to stage two");
        assert_eq!(
            word(&e, 0x25ea),
            0,
            "the next stage starts with no collected keys"
        );
        assert_eq!(word(&e, 0x25ec), 2);
        assert_ne!(e.display().indexed_frame(), initial_scene);
        capture(&e, &format!("adventure4-{version}-stage-two"));
    }
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn namek_retries_after_three_misses_and_completes_five_stages() {
    fn word(e: &Emulator, address: usize) -> u16 {
        u16::from_le_bytes(e.vm().memory()[address..address + 2].try_into().unwrap())
    }
    fn wait(e: &mut Emulator) -> FrameStatus {
        for _ in 0..6000 {
            let status = e.run_frame().unwrap().status;
            if matches!(
                status,
                FrameStatus::WaitingForInput | FrameStatus::Halted(_)
            ) {
                return status;
            }
        }
        panic!("the next interaction must be reachable");
    }
    fn key(e: &mut Emulator, value: u8) {
        e.input_mut().release_all();
        e.input_mut().set_keys([value]);
        advance(e, 1);
        e.input_mut().release_all();
        assert_eq!(wait(e), FrameStatus::WaitingForInput);
    }
    fn start(e: &mut Emulator) {
        for frame in 0..1800 {
            e.input_mut().set_keys(
                (frame >= 300 && (frame - 300) / 60 < 10 && (frame - 300) % 60 < 3).then_some(13),
            );
            advance(e, 1);
        }
        e.input_mut().release_all();
        assert_eq!(e.vm().pc(), 0x1cdc);
        assert_eq!(word(e, 0x3632), 1);
        assert_eq!(word(e, 0x3634), 0);
        assert_eq!(word(e, 0x3636), 0);
    }
    let path = fixture_path("决战娜美克.lav");
    let mut e = load(&path);
    start(&mut e);
    capture(&e, "namek-hidden-target");
    let first_scene = e.display().indexed_frame().to_vec();
    for misses in 1..=3 {
        let target = word(&e, 0x362a);
        key(&mut e, b"bnmghjtyu"[(target % 9) as usize]);
        assert_eq!(e.vm().pc(), 0x1ef7);
        assert_eq!(word(&e, 0x3636), misses);
        assert_eq!(word(&e, 0x3634), 0);
        capture(&e, &format!("namek-miss-{misses}"));
        key(&mut e, 13);
    }
    assert_eq!(e.vm().pc(), 0x1f62);
    capture(&e, "namek-loss");
    key(&mut e, 13);
    assert_eq!(e.vm().pc(), 0x16a1);
    start(&mut e);
    for stage in 1..=5 {
        assert_eq!(word(&e, 0x3632), stage);
        assert_eq!(word(&e, 0x3638), 200 - 30 * (stage - 1));
        for hit in 1..=3 {
            assert_eq!(e.vm().pc(), 0x1cdc);
            let target = word(&e, 0x362a);
            assert!((1..=9).contains(&target));
            assert_eq!(word(&e, 0x362c), 38 + 36 * ((target - 1) % 3));
            assert_eq!(word(&e, 0x362e), 26 * (2 - (target - 1) / 3));
            key(&mut e, b"bnmghjtyu"[(target - 1) as usize]);
            assert_eq!(e.vm().pc(), 0x1e70);
            assert_eq!(word(&e, 0x3628), target);
            assert_eq!(word(&e, 0x3634), hit);
            assert_eq!(word(&e, 0x3636), 0);
            assert_ne!(e.display().indexed_frame(), first_scene);
            capture(&e, &format!("namek-stage-{stage}-hit-{hit}"));
            key(&mut e, 13);
        }
        assert_eq!(e.vm().pc(), 0x1ad0);
        assert_eq!(word(&e, 0x3632), stage + 1);
        assert_eq!(word(&e, 0x3634), 0);
        capture(&e, &format!("namek-upgrade-{stage}"));
        key(&mut e, 13);
    }
    assert_ne!(
        e.vm().pc(),
        0x1cdc,
        "five stages must reach the ending rather than stage six"
    );
    capture(&e, "namek-victory");
    e.input_mut().set_keys([13]);
    e.run_frame().unwrap();
    e.input_mut().release_all();
    assert_eq!(wait(&mut e), FrameStatus::Halted(0));
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn volley_shooting_scores_saves_records_and_drives_goalkeeper_controls() {
    fn word(e: &Emulator, p: usize) -> u16 {
        u16::from_le_bytes(e.vm().memory()[p..p + 2].try_into().unwrap())
    }
    fn tap(e: &mut Emulator, k: u8) {
        e.input_mut().set_keys([k]);
        advance(e, 3);
        e.input_mut().set_keys([]);
        advance(e, 57);
    }
    fn menu(e: &mut Emulator) {
        tap(e, 13);
        tap(e, 13);
        advance(e, 180);
        assert_eq!(word(e, 0x204a), 10);
    }
    fn quit(e: &mut Emulator, expected: &[u8], pc: usize) {
        e.input_mut().set_keys(*b"q");
        advance(e, 60);
        e.input_mut().set_keys([]);
        advance(e, 240);
        assert_eq!(e.vm().pc(), pc);
        let files = e.files_mut().take_dirty_files();
        assert_eq!(
            files
                .iter()
                .find(|(p, _)| p.ends_with("zqtop.dat"))
                .unwrap()
                .1,
            expected
        );
    }
    let path = fixture_path("凌空抽射v1.5.lav");
    let mut e = load(&path);
    for f in 0..2400 {
        e.input_mut()
            .set_keys((f >= 300 && (f - 300) / 60 < 10 && (f - 300) % 60 < 3).then_some(13));
        advance(&mut e, 1);
    }
    e.input_mut().set_keys([]);
    assert_eq!(word(&e, 0x2050), 0);
    assert_eq!(word(&e, 0x2000), 74);
    capture(&e, "volley-shooting-start");
    for (key, left) in [(b'a', true), (b'd', false)] {
        let mut branch = e.clone();
        branch.input_mut().set_keys([key]);
        advance(&mut branch, 60);
        assert_eq!(word(&branch, 0x2000), if left { 49 } else { 99 });
        capture(&branch, &format!("volley-move-{key}"));
    }
    let mut rng = 1_234_567u32;
    let mut key = 0;
    let mut score = 0;
    let mut kicked = false;
    for f in 0..65_000 {
        if f % 12 == 0 {
            rng = rng.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            key = [0, 13, 13, 0, b'a', b'd', 0, 13][(rng >> 24) as usize % 8];
        }
        e.input_mut().set_keys((key != 0).then_some(key));
        advance(&mut e, 1);
        if word(&e, 0x200e) < 77 && !kicked {
            kicked = true;
            capture(&e, "volley-kicked-ball");
        }
        let next = word(&e, 0x2050);
        if next != score {
            assert_eq!(next, score + 5);
            score = next;
            assert_eq!(word(&e, 0x2052), score / 5);
            capture(&e, &format!("volley-goals-{}", score / 5));
        }
        if score == 15 {
            break;
        }
    }
    assert!(kicked);
    assert_eq!(score, 15);
    quit(&mut e, &[15, 0, 0], 0x3117);
    capture(&e, "volley-shooting-record");
    menu(&mut e);
    for k in [13, 13, 21, 13] {
        tap(&mut e, k);
    }
    for _ in 0..6000 {
        advance(&mut e, 1);
        if word(&e, 0x2050) >= 3 {
            break;
        }
    }
    assert_eq!(word(&e, 0x2050), 3);
    assert_eq!(
        word(&e, 0x2052),
        0,
        "the saved shot must not count as a conceded goal"
    );
    capture(&e, "volley-goalkeeper-save");
    for (key, field) in [(b'j', 0x202c), (b'k', 0x2028), (b'l', 0x202e)] {
        let mut branch = e.clone();
        branch.input_mut().set_keys([key]);
        advance(&mut branch, 30);
        assert_eq!(word(&branch, field), 1);
        if key == b'j' {
            assert!(word(&branch, 0x2004) < word(&e, 0x2004));
        }
        if key == b'l' {
            assert!(word(&branch, 0x2004) > word(&e, 0x2004));
        }
        assert!(word(&branch, 0x2006) < 18);
        capture(&branch, &format!("volley-goalkeeper-{key}"));
    }
    quit(&mut e, &[15, 3, 0], 0x57f9);
    capture(&e, "volley-goalkeeper-record");
    let saved = e.files().file("/lavadata/zqtop.dat").unwrap().to_vec();
    let mut restored = load(&path);
    restored
        .files_mut()
        .import_file("/lavadata/zqtop.dat", saved);
    for f in 0..1200 {
        restored
            .input_mut()
            .set_keys((f >= 300 && (f - 300) / 60 < 2 && (f - 300) % 60 < 3).then_some(13));
        advance(&mut restored, 1);
    }
    tap(&mut restored, 21);
    tap(&mut restored, 13);
    assert_eq!(&restored.vm().memory()[0x4f47..0x4f4a], &[15, 3, 0]);
    capture(&restored, "volley-restored-records");
    for key in [13, 21, 21, 13] {
        tap(&mut restored, key);
    }
    assert_eq!(restored.vm().pc(), 0x3520);
    capture(&restored, "volley-farewell");
    restored.input_mut().set_keys([13]);
    let mut exited = false;
    for frame in 0..180 {
        if frame == 3 {
            restored.input_mut().set_keys([]);
        }
        if let FrameStatus::Halted(code) = restored.run_frame().unwrap().status {
            assert_eq!(code, 0);
            exited = true;
            break;
        }
    }
    assert!(exited);
    for bytes in [vec![], vec![7], vec![7, 2]] {
        let mut recovered = load(&path);
        recovered
            .files_mut()
            .import_file("/lavadata/zqtop.dat", bytes.clone());
        advance(&mut recovered, 300);
        tap(&mut recovered, 13);
        tap(&mut recovered, 13);
        advance(&mut recovered, 900);
        let mut expected = bytes;
        expected.resize(3, 0);
        assert_eq!(
            recovered.files().file("/lavadata/zqtop.dat").unwrap(),
            expected
        );
    }
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn phoenix_fighter_moves_shoots_bombs_pauses_retries_and_clears_a_stage() {
    fn word(e: &Emulator, p: usize) -> u16 {
        u16::from_le_bytes(e.vm().memory()[p..p + 2].try_into().unwrap())
    }
    let path = fixture_path("Fighter.lav");
    let mut e = load(&path);
    for f in 0..421 {
        e.input_mut()
            .set_keys((f >= 300 && (f - 300) / 60 < 3 && (f - 300) % 60 < 3).then_some(13));
        advance(&mut e, 1);
    }
    e.input_mut().release_all();
    for f in 0..4 {
        e.input_mut().set_keys((f < 3).then_some(13));
        advance(&mut e, 1);
    }
    assert_eq!(&e.vm().memory()[0x39b5..0x39b7], &[107, 2]);
    assert_eq!(&e.vm().memory()[0x39b9..0x39bb], &[20, 30]);
    capture(&e, "phoenix-start");
    for (key, position) in [
        (20, (20, 0)),
        (21, (20, 60)),
        (22, (80, 30)),
        (23, (20, 30)),
    ] {
        let mut branch = e.clone();
        branch.input_mut().set_keys([key]);
        advance(&mut branch, 12);
        assert_eq!(
            (branch.vm().memory()[0x39b9], branch.vm().memory()[0x39ba]),
            position
        );
        capture(&branch, &format!("phoenix-move-{key}"));
    }
    let mut shot = e.clone();
    shot.input_mut().set_keys(*b"a");
    advance(&mut shot, 12);
    assert_ne!(shot.vm().memory()[0x3a05], 0);
    capture(&shot, "phoenix-shooting");
    let mut bomb = e.clone();
    bomb.input_mut().set_keys(*b"z");
    advance(&mut bomb, 12);
    assert_eq!(bomb.vm().memory()[0x39be], 1);
    assert_eq!(&bomb.vm().memory()[0x39bc..0x39be], &[28, 38]);
    capture(&bomb, "phoenix-bombing");
    let mut pause = e.clone();
    pause.input_mut().set_keys([32]);
    advance(&mut pause, 3);
    pause.input_mut().release_all();
    advance(&mut pause, 177);
    assert_eq!(pause.vm().pc(), 0x52c0);
    let paused = pause.display().indexed_frame().to_vec();
    let memory = pause.vm().memory().to_vec();
    advance(&mut pause, 180);
    assert_eq!(pause.display().indexed_frame(), paused);
    assert_eq!(pause.vm().memory(), memory);
    capture(&pause, "phoenix-paused");
    pause.input_mut().set_keys([13]);
    advance(&mut pause, 1);
    pause.input_mut().set_keys([22]);
    advance(&mut pause, 12);
    assert!(pause.vm().memory()[0x39b9] > 20);
    assert_ne!(pause.display().indexed_frame(), paused);
    let mut loss = e.clone();
    loss.input_mut().release_all();
    let mut failed = false;
    for _ in 0..600 {
        let r = loss.run_frame().unwrap();
        if r.status == FrameStatus::WaitingForInput {
            assert!(loss.vm().memory()[0x39b5] <= 100);
            assert_eq!(loss.vm().pc(), 0x5309);
            failed = true;
            break;
        }
    }
    assert!(failed);
    capture(&loss, "phoenix-defeat");
    for _ in 0..2 {
        loss.input_mut().release_all();
        loss.input_mut().set_keys([13]);
        advance(&mut loss, 1);
    }
    loss.input_mut().release_all();
    advance(&mut loss, 1);
    assert_eq!(loss.vm().memory()[0x39b5], 107);
    assert_eq!(&loss.vm().memory()[0x39b9..0x39bb], &[20, 30]);
    capture(&loss, "phoenix-retry");
    let route = [
        0, 0, 21, 20, 20, 0, 0, 21, 21, 22, 23, 20, 22, 0, 23, 0, 0, 20, 21, 21, 21, 0, 20, 0, 0,
        22, 22, 22, 22, 20, 0, 23, 21, 23, 21, 23, 22, 20, 20, 23, 23, 0, 21, 21, 20, 20, 0, 22,
        21, 23, 0, 0, 0, 0, 0, 21, 22, 22, 23, 20, 23, 0, 0, 0, 0, 0, 20, 22, 0, 21, 23, 20, 20,
        22, 23, 21, 22, 21, 21, 22, 23, 0, 23, 20, 21, 22, 22, 20, 20, 23, 23, 21, 22, 21, 0, 22,
        20, 0, 22, 20, 23, 23, 0, 23, 21, 21, 0, 0, 0, 0, 0, 0, 20, 0, 20, 22, 23, 21, 22, 22, 22,
        0, 23, 23, 21, 22, 22, 23, 23, 20, 22, 21, 23, 20, 21, 20, 20, 20, 22, 22, 0, 21, 21, 21,
        20, 23, 21, 23, 23, 0, 20, 20, 22, 20, 21, 21, 23, 0, 0, 20, 22, 21, 23, 22, 23, 0, 20, 20,
        0, 22, 22, 21, 21, 23, 23, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 20, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 20, 0, 22, 22, 22, 22, 22, 23, 23, 22, 23, 23, 23, 21, 21, 22, 22, 22, 23, 0,
        23, 20, 23, 20, 21, 0,
    ];
    let mut saw_boss = false;
    let mut damaged_boss = false;
    for (step, key) in route.into_iter().enumerate() {
        for _ in 0..4 {
            e.input_mut()
                .set_keys((*b"az").into_iter().chain((key != 0).then_some(key)));
            advance(&mut e, 1);
            assert!(e.vm().memory()[0x39b5] > 100);
            if word(&e, 0x3d5b) == 83 && e.vm().memory()[0x39d1] == 6 {
                let hp = word(&e, 0x3d4b);
                if !saw_boss {
                    capture(&e, "phoenix-boss");
                }
                saw_boss = true;
                if hp < 45 && !damaged_boss {
                    capture(&e, "phoenix-boss-damaged");
                }
                damaged_boss |= hp < 45;
            }
        }
        if step % 50 == 49 {
            capture(&e, &format!("phoenix-flight-{step}"));
        }
    }
    assert!(saw_boss && damaged_boss);
    assert_eq!(word(&e, 0x3d4b), 0);
    assert_eq!(e.vm().memory()[0x39c0], 8);
    capture(&e, "phoenix-boss-defeated");
    e.input_mut().release_all();
    advance(&mut e, 12);
    assert_eq!(
        e.vm().memory()[0x3d2a],
        1,
        "the script must advance to stage two"
    );
    assert!(e.vm().memory()[0x39b5] > 100);
    capture(&e, "phoenix-stage-two");
    e.input_mut().set_keys([27]);
    let mut exited = false;
    for _ in 0..10 {
        if let FrameStatus::Halted(code) = e.run_frame().unwrap().status {
            assert_eq!(code, 1, "Escape explicitly requests this exit status");
            exited = true;
            break;
        }
    }
    assert!(exited);
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn power_ski_moves_jumps_recovers_and_presents_both_endings() {
    fn word(e: &Emulator, p: usize) -> u16 {
        u16::from_le_bytes(e.vm().memory()[p..p + 2].try_into().unwrap())
    }
    fn input_window(e: &mut Emulator) {
        for _ in 0..1000 {
            if e.vm().pc() == 0x2e9a {
                return;
            }
            advance(e, 1);
        }
        panic!("the next skiing input window must be reachable");
    }
    fn step(e: &mut Emulator, k: u8) {
        input_window(e);
        e.input_mut().release_all();
        e.input_mut().set_keys([k]);
        advance(e, 1);
        e.input_mut().set_keys([]);
        for _ in 0..1000 {
            if e.vm().pc() == 0x2ecd {
                return;
            }
            advance(e, 1);
        }
        panic!("the skier must be drawn after applying an input");
    }
    fn ending(e: &Emulator, address: usize) {
        let expected: Vec<u8> = e.vm().memory()[address..address + 1600]
            .iter()
            .flat_map(|b| (0..8).map(move |bit| u8::from(b & (0x80 >> bit) != 0)))
            .collect();
        assert!(
            e.display().indexed_frame() == expected,
            "the ending picture must be visible while waiting for confirmation"
        );
    }
    let path = fixture_path("滑雪.lav");
    let mut e = load(&path);
    for f in 0..450 {
        e.input_mut()
            .set_keys((f >= 300 && (f - 300) / 60 < 3 && (f - 300) % 60 < 3).then_some(13));
        advance(&mut e, 1);
    }
    e.input_mut().release_all();
    assert_eq!(word(&e, 0x5793), 3);
    assert_eq!(word(&e, 0x5795), 10);
    assert_eq!(e.vm().memory()[0x5799], 80);
    capture(&e, "ski-start");
    for (key, count, x) in [(22, 8, 100), (23, 14, 40)] {
        let mut branch = e.clone();
        for _ in 0..count {
            step(&mut branch, key);
        }
        assert_eq!(branch.vm().memory()[0x5799], x);
        assert_eq!(word(&branch, 0x5795), 10);
        capture(&branch, &format!("ski-boundary-{key}"));
    }
    let mut jump = e.clone();
    step(&mut jump, 20);
    assert_eq!(jump.vm().memory()[0x5798], 1);
    assert!(jump.vm().memory()[0x579a] < 40);
    capture(&jump, "ski-jump");
    for (key, x) in [(b'a', 77), (b'd', 83)] {
        let mut branch = jump.clone();
        step(&mut branch, key);
        assert_eq!(branch.vm().memory()[0x5799], x);
        assert_eq!(branch.vm().memory()[0x5798], 1);
        capture(&branch, &format!("ski-air-control-{key}"));
    }
    let mut status = e.clone();
    input_window(&mut status);
    status.input_mut().set_keys([25]);
    advance(&mut status, 1);
    status.input_mut().set_keys([]);
    for _ in 0..100 {
        if status.run_frame().unwrap().status == FrameStatus::WaitingForInput {
            break;
        }
    }
    assert_eq!(status.vm().pc(), 0x2dd8);
    let paused = status.display().indexed_frame().to_vec();
    let progress = status.vm().memory()[0x578f..0x579b].to_vec();
    advance(&mut status, 180);
    assert_eq!(status.display().indexed_frame(), paused);
    assert_eq!(&status.vm().memory()[0x578f..0x579b], progress);
    capture(&status, "ski-status-paused");
    status.input_mut().set_keys([27]);
    advance(&mut status, 1);
    status.input_mut().set_keys([]);
    input_window(&mut status);
    assert!(word(&status, 0x5793) > word(&e, 0x5793));
    let mut loss = e.clone();
    let mut energy = 10;
    let mut lost = false;
    for _ in 0..5000 {
        let pc = loss.vm().pc();
        let bp = loss.vm().local_bp() as usize;
        let m = loss.vm().memory();
        let ret = u32::from_le_bytes([m[bp], m[bp + 1], m[bp + 2], 0]);
        let recovering = pc == 0x2ed1 && matches!(ret, 0x2fac | 0x2fb2);
        loss.input_mut().set_keys(recovering.then_some(20));
        let r = loss.run_frame().unwrap();
        let next = word(&loss, 0x5795);
        if next != energy {
            assert_eq!(next + 1, energy);
            energy = next;
            capture(&loss, &format!("ski-collision-{}", 10 - energy));
        }
        if r.status == FrameStatus::WaitingForInput {
            assert_eq!(loss.vm().pc(), 0x2e01);
            lost = true;
            break;
        }
    }
    assert!(lost);
    assert_eq!(energy, 0);
    assert_eq!(word(&loss, 0x5793), 61);
    capture(&loss, "ski-defeat");
    ending(&loss, 0x32c0);
    let mut won = false;
    let mut milestone = 0;
    for _ in 0..30_000 {
        let jumping = e.vm().pc() == 0x2e9a && e.vm().memory()[0x5798] == 0;
        e.input_mut().set_keys(jumping.then_some(20));
        let r = e.run_frame().unwrap();
        assert_eq!(word(&e, 0x5795), 10);
        let distance = word(&e, 0x5793);
        if distance / 250 > milestone
            && (e.vm().pc() == 0x2ecd || r.status == FrameStatus::WaitingForInput)
        {
            milestone = distance / 250;
            capture(&e, &format!("ski-distance-{distance}"));
        }
        if r.status == FrameStatus::WaitingForInput {
            assert_eq!(e.vm().pc(), 0x2e17);
            won = true;
            break;
        }
    }
    assert!(won);
    assert_eq!(word(&e, 0x5793), 1000);
    capture(&e, "ski-victory");
    ending(&e, 0x2c80);
    for key in [13, 21, 21, 13] {
        e.input_mut().release_all();
        e.input_mut().set_keys([key]);
        advance(&mut e, 3);
        e.input_mut().set_keys([]);
        advance(&mut e, 57);
    }
    assert_eq!(e.vm().pc(), 0x2d72);
    capture(&e, "ski-credits");
    e.input_mut().set_keys([13]);
    let mut exited = false;
    for _ in 0..60 {
        if let FrameStatus::Halted(code) = e.run_frame().unwrap().status {
            assert_eq!(code, 0);
            exited = true;
            break;
        }
    }
    assert!(exited);
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn brave_legend_restarts_moves_platforms_and_completes_all_forty_stages() {
    fn word(e: &Emulator, p: usize) -> u16 {
        u16::from_le_bytes(e.vm().memory()[p..p + 2].try_into().unwrap())
    }
    fn key(e: &mut Emulator, k: u8) -> FrameStatus {
        e.input_mut().release_all();
        e.input_mut().set_keys([k]);
        for f in 0..600 {
            let status = e.run_frame().unwrap().status;
            if f == 0 {
                e.input_mut().set_keys([]);
            }
            if matches!(
                status,
                FrameStatus::Running | FrameStatus::WaitingForInput | FrameStatus::Halted(_)
            ) {
                return status;
            }
        }
        panic!("the next stable interaction must be reachable");
    }
    let routes: &[&[u8]] = &[
        &[22, 22, 22, 20, 22, 22, 22, 13, 22, 22, 21],
        &[
            13, 13, 22, 13, 13, 22, 13, 13, 13, 13, 13, 23, 13, 13, 13, 13, 22, 13, 13, 13, 13, 13,
        ],
        &[
            23, 23, 23, 23, 23, 23, 23, 23, 13, 13, 22, 23, 21, 22, 22, 22, 22, 22, 22, 22, 22,
        ],
        &[
            13, 13, 13, 13, 13, 13, 13, 23, 13, 13, 13, 22, 13, 13, 13, 13, 13, 13, 22, 13, 13, 13,
            23, 13, 13, 13, 13, 13, 13, 23, 21,
        ],
        &[
            13, 13, 13, 13, 13, 13, 13, 22, 13, 13, 13, 13, 13, 13, 13, 23, 13, 13, 13, 13, 13, 13,
            22, 22, 21,
        ],
        &[22, 22, 13, 13, 13, 13, 13, 23, 22, 22, 13, 13, 22, 22, 22],
        &[
            22, 22, 22, 22, 13, 13, 22, 22, 13, 13, 23, 23, 13, 22, 21, 23, 23, 13, 23, 23, 23, 23,
            20, 20, 20, 20, 20, 22,
        ],
        &[
            13, 13, 13, 13, 23, 23, 23, 23, 23, 22, 22, 22, 22, 13, 13, 13, 13, 13, 22, 13, 22,
        ],
        &[
            22, 13, 23, 13, 23, 13, 13, 13, 22, 13, 13, 13, 13, 13, 22, 13, 22, 13, 22, 13, 22, 13,
            23,
        ],
        &[
            22, 22, 21, 21, 21, 22, 22, 22, 20, 23, 20, 23, 20, 23, 20, 23, 20, 23, 23, 23, 21,
        ],
        &[
            13, 23, 13, 23, 13, 13, 13, 13, 13, 13, 22, 13, 13, 13, 13, 23, 23, 23, 13, 13, 13, 13,
            13,
        ],
        &[
            22, 22, 22, 22, 13, 13, 23, 13, 13, 23, 13, 22, 13, 13, 22, 13, 13, 13, 23, 13, 13, 13,
            13, 13, 13, 23, 21, 22, 22, 22, 22, 22,
        ],
        &[
            22, 22, 13, 13, 22, 22, 22, 22, 22, 22, 13, 13, 13, 22, 21, 13, 13, 23, 23, 13, 13, 13,
            23, 13, 13, 22, 22, 13, 13, 23, 23,
        ],
        &[
            22, 22, 22, 22, 13, 13, 13, 23, 23, 23, 23, 23, 23, 20, 20, 20, 20, 22, 22, 22, 22, 22,
            13, 13, 13, 23, 23, 23, 23, 23, 20, 20, 20, 20, 22, 22, 22, 22, 13, 13, 13, 23, 23, 23,
            23, 20, 20, 20, 20, 22, 22, 22, 22, 22, 22, 22,
        ],
        &[
            13, 13, 13, 13, 13, 13, 13, 23, 13, 13, 13, 13, 13, 13, 13, 22, 13, 13, 13, 13, 13, 22,
            13, 13, 22, 21,
        ],
        &[22, 22, 13, 23, 23, 13, 22, 22, 22, 22, 21],
        &[
            22, 13, 23, 20, 23, 13, 13, 13, 13, 22, 22, 22, 13, 13, 13, 13, 13, 23, 23, 21,
        ],
        &[22, 22, 13, 13, 23, 22, 22, 20, 20, 22, 22, 22, 22, 22, 21],
        &[
            23, 23, 23, 23, 23, 13, 13, 13, 13, 13, 23, 23, 22, 13, 22, 13, 13, 13, 13, 13, 23, 23,
            23, 13, 23, 21, 13, 13, 13, 13, 13, 23,
        ],
        &[
            22, 13, 13, 13, 22, 13, 13, 13, 23, 13, 13, 13, 23, 13, 13, 13, 23, 13, 23, 23, 13, 13,
            13, 13, 13, 13, 13, 22, 13, 13, 13, 13, 13,
        ],
        &[
            13, 13, 23, 23, 23, 20, 20, 20, 20, 22, 22, 22, 22, 13, 13, 13, 22, 21, 23, 22, 13, 13,
            22, 20, 20, 23, 13, 13, 13, 23, 23, 23, 13, 23, 23, 23, 23,
        ],
        &[
            13, 13, 21, 13, 13, 13, 13, 13, 13, 22, 13, 13, 13, 13, 13, 22, 22, 21,
        ],
        &[
            23, 13, 13, 22, 22, 13, 13, 13, 22, 22, 21, 13, 13, 13, 13, 23, 20, 20, 22, 22, 20, 22,
            22, 22,
        ],
        &[
            13, 13, 13, 23, 23, 13, 23, 13, 21, 22, 22, 22, 13, 13, 13, 13, 13, 23, 23,
        ],
        &[
            22, 22, 23, 13, 13, 23, 23, 23, 20, 20, 20, 20, 20, 20, 20, 22, 13, 13, 13, 13, 23, 13,
            13, 22, 13, 13, 13, 23, 13, 13, 13, 22, 13, 13, 22, 13, 13, 13, 13, 13, 23, 23, 23, 23,
            23, 23,
        ],
        &[
            22, 22, 13, 13, 13, 13, 13, 13, 13, 22, 13, 13, 13, 22, 22, 23, 23,
        ],
        &[
            22, 13, 23, 23, 21, 23, 13, 22, 22, 22, 20, 23, 20, 23, 13, 13, 22, 23, 22, 22, 23, 23,
            23, 20, 20, 20, 20, 20, 20, 22, 22, 22, 22, 22, 22, 23,
        ],
        &[
            23, 23, 13, 22, 13, 22, 13, 13, 13, 13, 23, 23, 23, 23, 23, 23, 21, 22, 21, 13, 23, 13,
            13, 13, 13, 13, 13, 13, 13, 23, 23, 21,
        ],
        &[
            22, 22, 20, 20, 20, 20, 20, 20, 20, 23, 13, 13, 13, 13, 13, 23, 23, 13, 13, 22, 21, 23,
            21,
        ],
        &[
            13, 13, 13, 13, 23, 13, 23, 13, 13, 13, 23, 13, 13, 13, 23, 23, 23, 23, 23, 23, 22,
        ],
        &[
            13, 23, 13, 13, 13, 13, 13, 22, 13, 23, 13, 23, 23, 23, 13, 13, 13, 13, 13, 13, 22, 13,
            13, 22, 13, 22, 23, 23, 23, 23, 13, 23,
        ],
        &[
            23, 23, 13, 23, 23, 13, 13, 22, 13, 13, 22, 13, 13, 13, 22, 13, 13, 13, 23, 13, 13, 23,
            13, 13, 23,
        ],
        &[
            13, 13, 13, 23, 13, 13, 22, 22, 13, 23, 23, 23, 13, 13, 13, 13, 13, 13, 22, 22, 22, 13,
            23, 21, 13, 13, 13, 13, 23, 13, 23, 13, 22, 13, 22, 13, 13, 23,
        ],
        &[
            23, 13, 13, 13, 23, 13, 13, 13, 23, 23, 13, 13, 22, 13, 13, 13, 22, 21, 13, 13, 13, 23,
            13, 13, 22,
        ],
        &[
            23, 23, 23, 13, 13, 13, 23, 13, 13, 23, 13, 13, 13, 13, 13, 13, 23, 13, 13, 23, 13, 13,
            13, 13, 13, 13, 23, 13, 13, 23, 21, 13, 13, 13, 13, 13, 13, 23, 13, 13, 13, 23,
        ],
        &[
            22, 13, 13, 13, 22, 22, 13, 13, 23, 13, 23, 13, 13, 13, 23, 23, 13, 13, 13, 13, 22, 22,
            22, 13, 13, 13, 22, 22, 20, 20, 22, 13, 13, 13, 13, 13, 13, 22,
        ],
        &[
            22, 22, 22, 22, 13, 13, 13, 13, 13, 23, 23, 23, 13, 13, 13, 13, 22, 22, 22, 13, 13, 13,
            13, 13, 13, 23, 23, 23, 22, 22, 13, 13, 13, 13, 13, 22, 13, 13, 23, 13, 13, 13, 13, 23,
            23, 23, 21, 22, 21,
        ],
        &[
            23, 23, 13, 22, 22, 13, 22, 13, 22, 22, 13, 22, 13, 13, 13, 13, 13, 13, 13, 23, 23, 23,
            23, 23, 13, 13, 13, 22, 21, 13, 23, 21, 13, 13, 13, 13, 23, 13, 13, 13, 13, 22, 22, 22,
            13, 22, 22, 13, 22, 13, 13,
        ],
        &[13, 13, 20, 20, 22, 22, 21, 23, 23, 23, 13, 23, 23],
        &[
            22, 13, 13, 13, 23, 13, 13, 13, 13, 13, 13, 23, 13, 22, 13, 13, 23, 23, 13, 13, 13, 13,
            22, 13, 13, 13, 13, 13, 22, 20, 20, 22, 13, 22, 23, 23, 23, 23, 13, 13, 13, 22,
        ],
    ];
    assert_eq!(routes.len(), 40);
    let mut e = load(&fixture_path("勇者传说.lav"));
    for f in 0..2400 {
        e.input_mut()
            .set_keys((f >= 300 && (f - 300) / 60 < 10 && (f - 300) % 60 < 3).then_some(13));
        advance(&mut e, 1);
    }
    e.input_mut().release_all();
    assert_eq!(word(&e, 0x2002), 0);
    assert_eq!((word(&e, 0x2006), word(&e, 0x2008)), (1, 8));
    let original_board = e.vm().memory()[0x200e..0x207c].to_vec();
    let mut retry = e.clone();
    for k in [22, 22, 22, 20, 22, 22, 22, 13] {
        key(&mut retry, k);
    }
    assert_eq!(retry.vm().memory()[0x207c], 1);
    assert_ne!(&retry.vm().memory()[0x200e..0x207c], original_board);
    capture(&retry, "brave-before-restart");
    assert_eq!(key(&mut retry, 28), FrameStatus::WaitingForInput);
    assert_eq!(word(&retry, 0x2002), 0);
    assert_eq!(retry.vm().memory()[0x207c], 0);
    assert_eq!(&retry.vm().memory()[0x200e..0x207c], original_board);
    assert_eq!((word(&retry, 0x2006), word(&retry, 0x2008)), (1, 8));
    capture(&retry, "brave-restarted");
    assert_eq!(key(&mut retry, 13), FrameStatus::Running);
    let mut platform_moves = [0; 4];
    let mut pickups = 0;
    let mut inputs = 0;
    for (level, route) in routes.iter().enumerate() {
        assert_eq!(word(&e, 0x2002), level as u16);
        assert_eq!(e.vm().memory()[0x207c], 0);
        capture(&e, &format!("brave-stage-{:02}-start", level + 1));
        if level == 1 {
            let mut closed = e.clone();
            for k in [13, 13, 22, 13, 13, 13, 13, 13, 13, 22, 13, 13, 13, 13, 13] {
                key(&mut closed, k);
            }
            assert_eq!(closed.vm().memory()[0x207c], 0);
            assert_eq!(
                word(&closed, 0x2002),
                1,
                "entering a door without the key must preserve the stage"
            );
            let p = word(&closed, 0x2008) as usize * 11 + word(&closed, 0x2006) as usize;
            assert_eq!(closed.vm().memory()[0x200e + p], 10);
            capture(&closed, "brave-door-without-key");
        }
        for &k in *route {
            let before = e.vm().memory()[0x200e..0x207c].to_vec();
            let old_key = e.vm().memory()[0x207c];
            let below = (word(&e, 0x2008) as usize + 1) * 11 + word(&e, 0x2006) as usize;
            let tile = before[below];
            let status = key(&mut e, k);
            assert!(!matches!(status, FrameStatus::Halted(_)));
            inputs += 1;
            let after = &e.vm().memory()[0x200e..0x207c];
            if k == 13 && (3..=6).contains(&tile) && after != before {
                let delta = [1isize, -1, -11, 11][usize::from(tile - 3)];
                assert_eq!(after[below], 0);
                assert_eq!(after[below.checked_add_signed(delta).unwrap()], tile);
                if platform_moves[usize::from(tile - 3)] == 0 {
                    capture(&e, &format!("brave-platform-{tile}"));
                }
                platform_moves[usize::from(tile - 3)] += 1;
            }
            if old_key == 0 && e.vm().memory()[0x207c] != 0 {
                assert_eq!(
                    before.iter().filter(|&&v| v == 11).count(),
                    after.iter().filter(|&&v| v == 11).count() + 1
                );
                pickups += 1;
                if level == 0 {
                    capture(&e, "brave-key-collected");
                }
            }
        }
        assert_eq!(e.vm().memory()[0x207c], 1);
        assert_eq!(
            word(&e, 0x2002),
            level as u16 + 1,
            "the route must open the door and complete this stage"
        );
        assert_eq!(e.vm().pc(), if level < 39 { 0x34b5 } else { 0x34c9 });
        capture(&e, &format!("brave-stage-{:02}-complete", level + 1));
        if level < 39 {
            assert_eq!(key(&mut e, 13), FrameStatus::WaitingForInput);
            assert_eq!(key(&mut e, 13), FrameStatus::Running);
        }
    }
    assert_eq!(inputs, 1125);
    assert_eq!(pickups, 40);
    assert!(platform_moves.iter().all(|&n| n > 0));
    assert_eq!(key(&mut e, 13), FrameStatus::WaitingForInput);
    assert_eq!(e.vm().pc(), 0x3501);
    capture(&e, "brave-the-end");
    assert_eq!(key(&mut e, 13), FrameStatus::Halted(0));
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn dragon_quest_editions_trade_defend_recruit_and_restore_progress() {
    fn key(e: &mut Emulator, k: u8) -> FrameStatus {
        e.input_mut().release_all();
        e.input_mut().set_keys([k]);
        for frame in 0..1200 {
            let status = e.run_frame().expect("scripted input must execute").status;
            if frame == 0 {
                e.input_mut().set_keys([]);
            }
            if matches!(
                status,
                FrameStatus::Running | FrameStatus::WaitingForInput | FrameStatus::Halted(_)
            ) {
                return status;
            }
        }
        panic!("input must reach an idle state");
    }
    fn keys(e: &mut Emulator, sequence: &[u8]) {
        for &k in sequence {
            assert!(!matches!(key(e, k), FrameStatus::Halted(_)));
        }
    }
    fn start(e: &mut Emulator, sequence: &[u8], frames: usize) {
        for frame in 0..frames {
            let n = frame.saturating_sub(300);
            e.input_mut().set_keys(
                (frame >= 300 && n / 60 < sequence.len() && n % 60 < 3).then(|| sequence[n / 60]),
            );
            advance(e, 1);
        }
        e.input_mut().release_all();
    }
    fn word(e: &Emulator, address: usize) -> u16 {
        u16::from_le_bytes(e.vm().memory()[address..address + 2].try_into().unwrap())
    }
    fn dword(e: &Emulator, address: usize) -> u32 {
        u32::from_le_bytes(e.vm().memory()[address..address + 4].try_into().unwrap())
    }
    fn battle(e: &Emulator) -> bool {
        let m = e.vm().memory();
        let mut bp = e.vm().local_bp() as usize;
        for _ in 0..20 {
            let ret = u32::from_le_bytes([m[bp], m[bp + 1], m[bp + 2], 0]);
            if (0x6500..0x6bf5).contains(&ret) {
                return true;
            }
            let next = u16::from_le_bytes([m[bp + 3], m[bp + 4]]) as usize;
            if next == 0 || next == bp {
                break;
            }
            bp = next;
        }
        false
    }
    for (edition, companion) in [("standard", "DQM2.dat"), ("special", "HJM2.dat")] {
        let mut path = PathBuf::from(fixture_path(companion));
        path.set_file_name("勇者斗恶龙.lav");
        let path = path.to_str().unwrap();
        let mut e = load(path);
        start(&mut e, &[13; 10], 2400);
        assert_eq!(&e.vm().memory()[0x22a1..0x22a6], &[73, 3, 4, 2, 2]);
        assert_eq!(dword(&e, 0x32ff), 1000);
        assert_eq!(e.vm().memory()[0x334a], 0);
        capture(&e, &format!("dqm-{edition}-palace"));

        let mut shop = e.clone();
        keys(
            &mut shop,
            &[
                21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 22, 22, 21, 21, 21, 22, 22, 22, 20, 22,
                22, 22, 22, 22, 22, 22, 22, 22, 22, 22, 22, 22, 22, 22, 22, 20, 13,
            ],
        );
        assert_eq!(shop.vm().memory()[0x22a1], 20);
        assert_eq!(shop.vm().memory()[0x22a3], 7);
        capture(&shop, &format!("dqm-{edition}-merchant"));
        keys(&mut shop, &[13, 13, b'1', 13]);
        assert_eq!(dword(&shop, 0x32ff), 996);
        assert_eq!(shop.vm().memory()[0x334a], 1);
        capture(&shop, &format!("dqm-{edition}-purchased"));

        keys(
            &mut e,
            &[
                21, 21, 21, 21, 21, 21, 21, 21, 23, 23, 20, 20, 22, 22, 20, 20, 20, 23, 23, 23, 23,
                23, 20, 23, 20, 20, 20, 22, 22, 22, 22, 22, 22, 22, 21, 21, 21, 23, 23, 23, 23, 23,
                23, 20, 23, 20, 20, 20, 13,
            ],
        );
        assert_eq!(e.vm().memory()[0x22a1], 12);
        capture(&e, &format!("dqm-{edition}-dungeon"));
        for n in 0..14 {
            assert!(!battle(&e), "the encounter must occur on the recorded step");
            keys(&mut e, &[if n % 6 < 3 { 21 } else { 20 }]);
        }
        assert!(battle(&e));
        assert_eq!(word(&e, 0x3390), 12);
        assert_eq!(word(&e, 0x3462), 41);
        assert_eq!(e.vm().memory()[0x3460], 10);
        assert_eq!(dword(&e, 0x3447), 545);
        assert_eq!(e.vm().memory()[0x334e], 1);
        capture(&e, &format!("dqm-{edition}-encounter"));
        let mut defended = e.clone();
        keys(&mut defended, &[22, 22, 13, 21, 13]);
        assert!(battle(&defended));
        assert_eq!(word(&defended, 0x3390), 12);
        assert_eq!(word(&defended, 0x3462), 41);
        assert_eq!(dword(&defended, 0x3447), 545);
        capture(&defended, &format!("dqm-{edition}-defended"));
        for n in 0..5 {
            assert!(battle(&e));
            keys(&mut e, &[13]);
            if n == 0 {
                assert_eq!(word(&e, 0x3390), 0);
                assert_eq!(e.vm().memory()[0x3460], 11);
                assert_eq!(dword(&e, 0x3447), 6);
                assert_eq!(word(&e, 0x3462), 45);
                capture(&e, &format!("dqm-{edition}-victory-level-up"));
            }
            if n == 1 {
                assert_eq!(e.vm().memory()[0x334e], 2);
                capture(&e, &format!("dqm-{edition}-recruited"));
            }
        }
        assert!(!battle(&e));
        let party = e.vm().memory()[0x3422..0x3468].to_vec();
        capture(&e, &format!("dqm-{edition}-returned-to-dungeon"));
        keys(&mut e, &[31, 22, 22, 22, 13, 21, 21, 13, 13]);
        let files = e.files_mut().take_dirty_files();
        let save = files
            .iter()
            .find(|(name, _)| name.ends_with("/dqm_save.dat"))
            .expect("saving must create the selected native save slot");
        assert_eq!(save.1.len(), 1820);
        capture(&e, &format!("dqm-{edition}-saved"));
        let mut restored = load(path);
        restored.files_mut().import_file(&save.0, save.1.clone());
        start(&mut restored, &[13, 22, 13, 13], 1800);
        assert_eq!(&restored.vm().memory()[0x3422..0x3468], party);
        assert_eq!(restored.vm().memory()[0x334e], 2);
        assert_eq!(&restored.vm().memory()[0x22a1..0x22a6], &[73, 3, 4, 2, 2]);
        capture(&restored, &format!("dqm-{edition}-restored-party"));
        keys(&mut restored, &[31, 22, 22, 22, 13, 21, 21, 21, 13]);
        capture(&restored, &format!("dqm-{edition}-farewell"));
        assert_eq!(key(&mut restored, 13), FrameStatus::Halted(0));
    }
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn hell_adventure_triggers_switches_restarts_and_clears_the_first_stage() {
    fn key(e: &mut Emulator, k: u8) -> FrameStatus {
        e.input_mut().release_all();
        e.input_mut().set_keys([k]);
        let mut status = FrameStatus::Running;
        for frame in 0..1001 {
            status = e.run_frame().expect("scripted input must execute").status;
            if frame == 0 {
                e.input_mut().set_keys([]);
            }
            if matches!(
                status,
                FrameStatus::Running | FrameStatus::WaitingForInput | FrameStatus::Halted(_)
            ) {
                return status;
            }
        }
        status
    }
    fn position(e: &Emulator) -> &[u8] {
        &e.vm().memory()[0x2000..0x2002]
    }
    fn tile(e: &Emulator, x: usize, y: usize) -> u8 {
        e.vm().memory()[0x2002 + y * 20 + x]
    }
    let mut e = load(&fixture_path("勇闯地狱.lav"));
    advance(&mut e, 300);
    for _ in 0..5 {
        key(&mut e, 13);
    }
    assert_eq!(position(&e), &[8, 8]);
    assert_eq!(e.vm().memory()[0x2174], 0);
    let initial = e.vm().memory()[0x2002..0x20ca].to_vec();
    capture(&e, "hell-stage-one");
    for k in [20, 23] {
        key(&mut e, k);
        assert_eq!(position(&e), &[8, 8], "border walls must block movement");
    }
    for _ in 0..9 {
        key(&mut e, 22);
    }
    assert_eq!(position(&e), &[88, 16]);
    assert_eq!(tile(&e, 4, 2), 0);
    assert_eq!(tile(&e, 10, 1), 2);
    assert_eq!(tile(&e, 5, 2), 4);
    assert_eq!(tile(&e, 10, 2), 11);
    capture(&e, "hell-first-switch-forced-turn");
    for k in [20, 22, 21] {
        key(&mut e, k);
        assert_eq!(position(&e), &[88, 16]);
    }
    key(&mut e, 23);
    assert_eq!(position(&e), &[72, 16]);
    assert_eq!(tile(&e, 5, 2), 0);
    assert_eq!(tile(&e, 10, 2), 2);
    capture(&e, "hell-second-switch");
    key(&mut e, 28);
    assert_eq!(position(&e), &[8, 8]);
    assert_eq!(&e.vm().memory()[0x2002..0x20ca], initial);
    assert_eq!(e.vm().memory()[0x2174], 0);
    capture(&e, "hell-restarted-board");
    for k in [
        22, 22, 22, 22, 22, 22, 22, 22, 22, 23, 20, 23, 23, 23, 23, 21, 21, 21, 21, 21, 21, 23, 23,
        23, 20, 20, 20, 20, 23,
    ] {
        assert!(!matches!(key(&mut e, k), FrameStatus::Halted(_)));
    }
    assert_eq!(e.vm().memory()[0x2174], 1);
    assert_eq!(position(&e), &[8, 8]);
    assert_ne!(&e.vm().memory()[0x2002..0x20ca], initial);
    capture(&e, "hell-stage-two");
    key(&mut e, 27);
    assert_eq!(e.vm().pc(), 0x1306);
    capture(&e, "hell-returned-to-menu");
    for k in [21, 21, 21, 13] {
        key(&mut e, k);
    }
    capture(&e, "hell-help");
    for page in 0..8 {
        if e.vm().pc() == 0x1306 {
            break;
        }
        key(&mut e, 13);
        capture(&e, &format!("hell-help-page-{page}"));
    }
    assert_eq!(e.vm().pc(), 0x1306);
    for k in [21, 21, 21, 21, 13] {
        key(&mut e, k);
    }
    capture(&e, "hell-farewell");
    assert_eq!(key(&mut e, 13), FrameStatus::Halted(0));
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn missing_doctor_talks_collects_an_item_and_solves_the_alarm_puzzle() {
    fn key(e: &mut Emulator, k: u8) -> FrameStatus {
        e.input_mut().release_all();
        e.input_mut().set_keys([k]);
        for frame in 0..1200 {
            let status = e.run_frame().expect("scripted input must execute").status;
            if frame == 0 {
                e.input_mut().set_keys([]);
            }
            if matches!(
                status,
                FrameStatus::Running | FrameStatus::WaitingForInput | FrameStatus::Halted(_)
            ) {
                assert!(!matches!(status, FrameStatus::Halted(_)));
                return status;
            }
        }
        panic!("input must reach a dialog or the movement loop");
    }
    fn keys(e: &mut Emulator, sequence: &[u8]) {
        for &k in sequence {
            key(e, k);
        }
    }
    fn word(e: &Emulator, address: usize) -> u16 {
        u16::from_le_bytes(e.vm().memory()[address..address + 2].try_into().unwrap())
    }
    fn position(e: &Emulator) -> (u16, u16) {
        (
            word(e, 0x30e9) + word(e, 0x30ed),
            word(e, 0x30eb) + word(e, 0x30ef),
        )
    }
    fn dialogs(e: &mut Emulator, phase: &str) {
        capture(e, &format!("doctor-{phase}-dialog"));
        for _ in 0..64 {
            if e.vm().pc() == 0x398a {
                return;
            }
            key(e, 13);
        }
        panic!("dialog pages must return to movement");
    }
    let mut e = load(&fixture_path("博士失踪记.lav"));
    advance(&mut e, 300);
    assert_eq!(e.vm().pc(), 0x398a);
    assert_eq!(position(&e), (11, 2));
    assert_eq!(word(&e, 0x30e7), 0);
    assert_eq!(word(&e, 0x304a), 2);
    capture(&e, "doctor-opening-street");
    key(&mut e, 21);
    assert_eq!(position(&e), (11, 3));
    key(&mut e, 20);
    assert_eq!(position(&e), (11, 2));
    key(&mut e, 20);
    assert_eq!(position(&e), (11, 1));
    key(&mut e, 20);
    assert_eq!(position(&e), (11, 1), "the tree must block movement");
    keys(&mut e, &[21, 21]);
    keys(&mut e, &[23; 10]);
    assert_eq!(position(&e), (1, 3));
    keys(&mut e, &[13, 13]);
    dialogs(&mut e, "doctor-conversation");
    assert_eq!(word(&e, 0x30e7), 10);
    capture(&e, "doctor-first-conversation-complete");
    keys(
        &mut e,
        &[
            22, 22, 22, 22, 22, 22, 22, 22, 22, 20, 22, 22, 22, 22, 22, 22, 22, 20, 22,
        ],
    );
    assert_eq!(position(&e), (18, 17));
    capture(&e, "doctor-home-cabinet");
    keys(&mut e, &[13, 21, 13]);
    dialogs(&mut e, "slingshot-found");
    assert_eq!(word(&e, 0x304a), 3);
    assert_eq!(&e.vm().memory()[0x304c..0x304f], &[0x3e, 0x40, 0x3c]);
    assert_eq!(e.vm().memory()[0x2000 + 16 * 31 + 18], 0x46);
    capture(&e, "doctor-slingshot-in-inventory");
    key(&mut e, 22);
    dialogs(&mut e, "next-morning");
    assert_eq!(word(&e, 0x30e7), 15);
    assert_eq!(position(&e), (18, 17));
    assert_eq!(e.vm().memory()[0x2000 + 18 * 31 + 12], 0x3b);
    keys(&mut e, &[21, 23, 23, 23, 23, 23]);
    assert_eq!(position(&e), (13, 18));
    keys(&mut e, &[13, 13]);
    dialogs(&mut e, "police-questioning");
    assert_eq!(word(&e, 0x30e7), 20);
    assert_eq!(e.vm().memory()[0x2000 + 18 * 31 + 12], 0);
    assert_eq!(e.vm().memory()[0x203f], 0x3b);
    capture(&e, "doctor-police-moved-to-door");
    keys(&mut e, &[23, 23, 21, 23, 23, 23, 20]);
    assert_eq!(position(&e), (8, 2));
    keys(&mut e, &[13, 21, 21, 13, 21, 13]);
    dialogs(&mut e, "phone-has-no-signal");
    assert_eq!(word(&e, 0x30e7), 20);
    assert_eq!(e.vm().memory()[0x203f], 0x3b);
    keys(&mut e, &[13, 21, 21, 13, 21, 21, 13]);
    dialogs(&mut e, "alarm-diversion");
    assert_eq!(word(&e, 0x30e7), 30);
    assert_eq!(e.vm().memory()[0x203f], 0x12);
    assert_eq!(position(&e), (8, 2));
    assert_eq!(&e.vm().memory()[0x304c..0x304f], &[0x3e, 0x40, 0x3c]);
    capture(&e, "doctor-alarm-puzzle-solved");
    key(&mut e, 25);
    for page in 0..3 {
        let pixels = e.display().indexed_frame();
        assert!(
            pixels.iter().any(|&p| p != pixels[0]),
            "help pages must show text"
        );
        capture(&e, &format!("doctor-help-page-{page}"));
        key(&mut e, 13);
    }
    assert_eq!(e.vm().pc(), 0x398a);
    assert_eq!(word(&e, 0x30e7), 30);
    assert_eq!(position(&e), (8, 2));
    capture(&e, "doctor-help-returned-to-street");
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn pokemon_pk_configures_both_parties_replaces_fainted_members_and_finishes_both_results() {
    fn key(e: &mut Emulator, k: u8) -> FrameStatus {
        e.input_mut().release_all();
        e.input_mut().set_keys([k]);
        for frame in 0..1200 {
            let status = e.run_frame().expect("scripted input must execute").status;
            if frame == 0 {
                e.input_mut().set_keys([]);
            }
            if matches!(
                status,
                FrameStatus::Running | FrameStatus::WaitingForInput | FrameStatus::Halted(_)
            ) {
                return status;
            }
        }
        panic!("input must reach the next menu or message");
    }
    fn keys(e: &mut Emulator, sequence: &[u8]) {
        for &k in sequence {
            assert!(!matches!(key(e, k), FrameStatus::Halted(_)));
        }
    }
    fn word(e: &Emulator, address: usize) -> i16 {
        i16::from_le_bytes(e.vm().memory()[address..address + 2].try_into().unwrap())
    }
    fn hp(e: &Emulator, player: usize, slot: usize) -> i16 {
        word(e, [0x3297, 0x32d9][player] + slot * 22 + 16)
    }
    fn assert_p2_roster(e: &Emulator) {
        use lavaxemu_core::{BufferTarget, DrawOperation};
        let mut expected = e.display().clone();
        for slot in 1..=3 {
            let y = slot as i32 * 16;
            expected.draw_rectangle(
                BufferTarget::Front,
                105,
                y,
                146,
                y + 11,
                true,
                DrawOperation::Clear,
            );
            for (x, text) in [
                (105, hp(e, 1, slot).to_string()),
                (123, "/".to_owned()),
                (129, word(e, 0x32d9 + slot * 22 + 14).to_string()),
            ] {
                expected.draw_text(
                    BufferTarget::Front,
                    x,
                    y,
                    text.as_bytes(),
                    false,
                    DrawOperation::Set,
                );
            }
            if slot == word(e, 0x37ad) as usize {
                expected.draw_rectangle(
                    BufferTarget::Front,
                    105,
                    y,
                    146,
                    y + 11,
                    true,
                    DrawOperation::Invert,
                );
            }
        }
        assert_eq!(
            e.display().indexed_frame(),
            expected.indexed_frame(),
            "P2's displayed HP must match its own party"
        );
    }
    let path = fixture_path("pmpk.pac");
    for winner in [2, 1] {
        let mut e = load(&path);
        advance(&mut e, 300);
        assert_eq!(e.vm().pc(), 0x13756);
        capture(&e, &format!("pokemon-pk-{winner}-title"));
        for player in 0..2 {
            if player == 0 {
                keys(&mut e, &[13, 13]);
            } else {
                keys(&mut e, &[13]);
            }
            assert_eq!(e.vm().pc(), 0xf9ba);
            let fire = player + 1 == winner;
            if fire {
                keys(&mut e, &[21]);
            }
            keys(&mut e, &[13; 3]);
            assert_eq!(e.vm().pc(), 0xf2c7);
            capture(
                &e,
                &format!("pokemon-pk-{winner}-p{}-move-list", player + 1),
            );
            for slot in 1..=3 {
                let party = [0x3297, 0x32d9][player];
                assert_eq!(word(&e, party + slot * 22 + 18), if fire { 2 } else { 1 });
                keys(&mut e, &[13, 21, 13, 21, 13, 21, 13]);
            }
        }
        keys(&mut e, &[13; 5]);
        assert_eq!(e.vm().pc(), 0x1186b);
        for player in 0..2 {
            let fire = player + 1 == winner;
            let ids = if fire { [1, 2, 3, 4] } else { [19, 20, 15, 16] };
            let moves = [0x35df, 0x366f][player];
            for slot in 0..3 {
                assert_eq!(hp(&e, player, slot + 1), if fire { 360 } else { 364 });
                for (n, id) in ids.iter().enumerate() {
                    let record = moves + (slot * 4 + n + 1) * 12;
                    assert_eq!(word(&e, record + 6), *id);
                    assert_eq!(word(&e, record + 2), word(&e, record));
                    for offset in [0, 4, 6, 8, 10] {
                        assert_eq!(
                            word(&e, record + offset),
                            word(&e, 0x6000 + *id as usize * 12 + offset)
                        );
                    }
                }
            }
            if fire {
                assert_eq!(word(&e, moves + 12), 15);
                assert_eq!(word(&e, moves + 12 + 10), 75);
            }
        }
        capture(&e, &format!("pokemon-pk-{winner}-battle-ready"));
        let mut damaged = [false; 2];
        let mut rejected = [false; 2];
        let mut confirmed_p2 = false;
        let mut fainted = [0; 2];
        let win_pc = if winner == 1 { 0x10b72 } else { 0x10b91 };
        for _ in 0..180 {
            let pc = e.vm().pc();
            if pc == win_pc {
                break;
            }
            let active = [word(&e, 0x37a1) as usize, word(&e, 0x37ad) as usize];
            assert!(active.iter().all(|&slot| (1..=3).contains(&slot)));
            if pc == 0x11050 {
                assert_p2_roster(&e);
            }
            let k = match pc {
                0xff4a | 0x10326 => {
                    let player = usize::from(pc == 0x10326);
                    let fire = player + 1 == winner;
                    let column = if fire { 1 } else { 2 };
                    if word(&e, 0x379d) != column {
                        22
                    } else if word(&e, 0x379f) != 2 {
                        21
                    } else {
                        13
                    }
                }
                0x10d83 | 0x11050 => {
                    let player = usize::from(pc == 0x11050);
                    if hp(&e, player, active[player]) <= 0 {
                        if !rejected[player] {
                            let mut invalid = e.clone();
                            keys(&mut invalid, &[13]);
                            assert_eq!(invalid.vm().pc(), pc, "a defeated member must be rejected");
                            capture(
                                &invalid,
                                &format!("pokemon-pk-{winner}-p{}-fainted-rejected", player + 1),
                            );
                            rejected[player] = true;
                        }
                        21
                    } else {
                        13
                    }
                }
                _ => 13,
            };
            keys(&mut e, &[k]);
            if pc == 0x11050 && k == 13 {
                assert_ne!(
                    e.vm().pc(),
                    pc,
                    "P2 must be able to confirm its own living member"
                );
                confirmed_p2 = true;
                capture(&e, &format!("pokemon-pk-{winner}-p2-confirmed"));
            }
            for player in 0..2 {
                let max_hp = if player + 1 == winner { 360 } else { 364 };
                if !damaged[player] && (1..=3).any(|slot| hp(&e, player, slot) < max_hp) {
                    damaged[player] = true;
                    capture(&e, &format!("pokemon-pk-{winner}-p{}-damaged", player + 1));
                }
                let count = (1..=3).filter(|&slot| hp(&e, player, slot) == 0).count();
                if count > fainted[player] {
                    fainted[player] = count;
                    capture(
                        &e,
                        &format!("pokemon-pk-{winner}-p{}-fainted-{count}", player + 1),
                    );
                }
            }
        }
        assert_eq!(e.vm().pc(), win_pc);
        assert!(damaged.into_iter().all(|value| value));
        assert!(rejected[2 - winner]);
        assert!(confirmed_p2);
        assert_eq!(fainted[2 - winner], 3);
        assert!((1..=3).any(|slot| hp(&e, winner - 1, slot) > 0));
        capture(&e, &format!("pokemon-pk-{winner}-victory"));
        keys(&mut e, &[13, 13]);
        assert_eq!(e.vm().pc(), 0x13756);
        keys(&mut e, &[22, 13]);
        assert_eq!(e.vm().pc(), 0x138b2);
        capture(&e, &format!("pokemon-pk-{winner}-credits"));
        keys(&mut e, &[13, 22, 22]);
        assert_eq!(key(&mut e, 13), FrameStatus::Halted(0));
    }
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn pokemon_prototype_explores_collects_an_item_and_browses_its_existing_menus() {
    fn key(e: &mut Emulator, k: u8) -> FrameStatus {
        e.input_mut().release_all();
        // Wait until the guest's input debounce window accepts a fresh press.
        for _ in 0..120 {
            let tick = ((e.elapsed_ms() % 1000) * 256 / 1000) as u8;
            if (120..=200).contains(&tick.wrapping_sub(e.vm().memory()[0x200c])) {
                break;
            }
            advance(e, 1);
        }
        let mut status = FrameStatus::Running;
        for frame in 0..120 {
            e.input_mut().set_keys((frame < 6).then_some(k));
            status = e.run_frame().expect("scripted input must execute").status;
            if matches!(status, FrameStatus::Halted(_)) {
                return status;
            }
        }
        status
    }
    fn keys(e: &mut Emulator, sequence: &[u8]) {
        for &k in sequence {
            assert!(!matches!(key(e, k), FrameStatus::Halted(_)));
        }
    }
    fn word(e: &Emulator, address: usize) -> u16 {
        u16::from_le_bytes(e.vm().memory()[address..address + 2].try_into().unwrap())
    }
    fn location(e: &Emulator) -> (u8, u16, u16) {
        (e.vm().memory()[0x2900], word(e, 0x2909), word(e, 0x290b))
    }
    let mut e = load(&fixture_path("pokemon.lav"));
    advance(&mut e, 600);
    assert_eq!(location(&e), (0, 22, 3));
    assert_eq!(word(&e, 0x2907), 4);
    let party = e.vm().memory()[0x2911..0x2a4d].to_vec();
    let resources = e.files().file("/pokemon.dat").unwrap().to_vec();
    capture(&e, "pokemon-prototype-start");
    let mut room = e.clone();
    keys(&mut room, &[20]);
    assert_eq!(location(&room), (1, 5, 10));
    capture(&room, "pokemon-prototype-room-entered");
    for (sequence, expected, face) in [
        (&[20, 20][..], (1, 5, 8), 0),
        (&[23, 23][..], (1, 4, 8), 2),
        (&[21, 21][..], (1, 4, 9), 1),
        (&[22, 22][..], (1, 5, 9), 3),
    ] {
        keys(&mut room, sequence);
        assert_eq!(location(&room), expected);
        assert_eq!(word(&room, 0x280d), face);
        capture(&room, &format!("pokemon-prototype-room-direction-{face}"));
    }
    keys(&mut room, &[20, 20, 20, 23, 23, 20, 13]);
    assert_eq!(location(&room), (1, 4, 7));
    capture(&room, "pokemon-prototype-room-dialog");
    keys(&mut room, &[27, 22, 22, 21, 21, 21, 21, 21]);
    assert_eq!(location(&room), (0, 22, 3));
    capture(&room, "pokemon-prototype-room-returned");
    assert_eq!(key(&mut room, b'q'), FrameStatus::Halted(0));

    keys(&mut e, &[21, 21, 23, 23]);
    assert_eq!(location(&e), (0, 21, 4));
    assert_eq!(e.vm().memory()[0x5840 + 130], 1);
    capture(&e, "pokemon-prototype-item-collected");
    keys(&mut e, &[27, 13, 22, 22, 13]);
    capture(&e, "pokemon-prototype-bag-general");
    let general = e.display().indexed_frame().to_vec();
    keys(&mut e, &[22]);
    assert_ne!(e.display().indexed_frame(), general);
    assert_eq!(e.vm().memory()[0x5840 + 130], 1);
    capture(&e, "pokemon-prototype-bag-ball");
    for page in 2..=4 {
        keys(&mut e, &[22]);
        capture(&e, &format!("pokemon-prototype-bag-page-{page}"));
    }
    keys(&mut e, &[27, 23, 23, 13]);
    let mut expected = e.display().clone();
    for index in 0..5 {
        let name = &resources[index * 0x580 + 0x500..index * 0x580 + 0x50b];
        let end = name.iter().position(|&c| c == 0).unwrap();
        expected.draw_text(
            lavaxemu_core::BufferTarget::Front,
            90,
            4 + index as i32 * 15,
            &name[..end],
            false,
            lavaxemu_core::DrawOperation::Invert,
        );
    }
    assert_eq!(
        e.display().indexed_frame(),
        expected.indexed_frame(),
        "catalog names must retain their inverted text styling"
    );
    capture(&e, "pokemon-prototype-catalog-list");
    keys(&mut e, &[21, 13]);
    assert_eq!(
        &e.vm().memory()[0x5724..0x572f],
        &resources[3 * 0x580 + 0x500..3 * 0x580 + 0x50b]
    );
    capture(&e, "pokemon-prototype-catalog-attributes");
    let attributes = e.display().indexed_frame().to_vec();
    keys(&mut e, &[22]);
    assert_ne!(e.display().indexed_frame(), attributes);
    capture(&e, "pokemon-prototype-catalog-description");
    keys(&mut e, &[23, 27, 27, 22, 13, 13]);
    capture(&e, "pokemon-prototype-party-status");
    let status = e.display().indexed_frame().to_vec();
    keys(&mut e, &[22]);
    assert_ne!(e.display().indexed_frame(), status);
    capture(&e, "pokemon-prototype-party-abilities");
    keys(&mut e, &[22]);
    capture(&e, "pokemon-prototype-party-moves");
    let first = e.display().indexed_frame().to_vec();
    keys(&mut e, &[21]);
    assert_ne!(e.display().indexed_frame(), first);
    capture(&e, "pokemon-prototype-next-party-member");
    keys(&mut e, &[20, 27, 27, 27]);
    assert_eq!(location(&e), (0, 21, 4));
    assert_eq!(&e.vm().memory()[0x2911..0x2a4d], party);
    assert_eq!(e.vm().memory()[0x5840 + 130], 1);
    capture(&e, "pokemon-prototype-menus-returned");
    assert_eq!(key(&mut e, b'q'), FrameStatus::Halted(0));
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn mooncake_opens_help_sets_speed_moves_and_reacts_to_all_five_items() {
    const BASE: usize = 0x2bd7;
    fn key(e: &mut Emulator, k: u8) -> FrameStatus {
        e.input_mut().release_all();
        let mut status = FrameStatus::Running;
        for frame in 0..60 {
            e.input_mut().set_keys((frame == 0).then_some(k));
            status = e.run_frame().unwrap().status;
            if matches!(status, FrameStatus::Halted(_)) {
                break;
            }
        }
        status
    }
    let mut e = load(&fixture_path("吃月饼.lav"));
    advance(&mut e, 300);
    assert_eq!(e.vm().pc(), 0xdf5);
    capture(&e, "mooncake-title");
    key(&mut e, b'm');
    assert_eq!(e.vm().pc(), 0xed2);
    capture(&e, "mooncake-help");
    key(&mut e, 13);
    assert_eq!(e.vm().pc(), 0xdf5);
    key(&mut e, b'n');
    assert_eq!(e.vm().memory()[BASE + 9], 10);
    for _ in 0..12 {
        key(&mut e, 20);
    }
    assert_eq!(e.vm().memory()[BASE + 9], 100);
    capture(&e, "mooncake-speed-maximum");
    for _ in 0..12 {
        key(&mut e, 21);
    }
    assert_eq!(e.vm().memory()[BASE + 9], 0);
    capture(&e, "mooncake-speed-minimum");
    key(&mut e, 20);
    key(&mut e, 13);
    assert_eq!(e.vm().memory()[BASE + 9], 10);
    key(&mut e, b'b');
    capture(&e, "mooncake-started");
    let initial = e.clone();
    for (k, expected) in [(b'a', 5), (b'd', 126)] {
        let mut branch = initial.clone();
        branch.input_mut().set_keys([k]);
        advance(&mut branch, 400);
        assert_eq!(branch.vm().memory()[BASE + 6], expected);
        capture(&branch, &format!("mooncake-boundary-{expected}"));
    }
    let mut caught = [false; 5];
    let mut recovered = [false; 5];
    for _ in 0..12000 {
        let m = e.vm().memory();
        let x = i16::from(m[BASE + 6]);
        let object_x = m[BASE + 0xb];
        let object_y = m[BASE + 0x15];
        let kind = usize::from(m[BASE + 0x1f]);
        let stun = m[BASE + 0x2b];
        let face_before = m[BASE + 5];
        let target = i16::from(object_x) - 8;
        let k = if stun > 0 {
            None
        } else if x < target - 3 || (x <= target + 3 && m[BASE + 10] < 11) {
            Some(b'd')
        } else if x > target + 3 || m[BASE + 10] > 11 {
            Some(b'a')
        } else {
            None
        };
        e.input_mut().set_keys(k);
        advance(&mut e, 1);
        let m = e.vm().memory();
        if stun == 0 && (40..=65).contains(&object_y) && m[BASE + 0x15] == 0 {
            let (face, duration) = [(3, 50), (0, 0), (2, 20), (1, 30), (4, 40)][kind];
            assert_eq!(m[BASE + 5], face);
            assert_eq!(m[BASE + 0x2b], duration);
            assert!((i16::from(m[BASE + 6]) - i16::from(object_x) + 8).abs() < 8);
            caught[kind] = true;
            recovered[kind] |= duration == 0;
            capture(&e, &format!("mooncake-item-{kind}"));
        }
        if stun == 1 && m[BASE + 0x2b] == 0 {
            assert_eq!(m[BASE + 5], face_before);
        }
        if stun == 0 && face_before != 0 && m[BASE + 5] == 0 {
            let kind = [1, 3, 2, 0, 4][usize::from(face_before)];
            assert!(caught[kind]);
            recovered[kind] = true;
        }
        if caught.into_iter().all(|v| v) && recovered.into_iter().all(|v| v) {
            break;
        }
    }
    assert!(caught.into_iter().all(|v| v));
    assert!(recovered.into_iter().all(|v| v));
    capture(&e, "mooncake-recovered");
    key(&mut e, 27);
    assert_eq!(e.vm().pc(), 0xdf5);
    capture(&e, "mooncake-returned");
    key(&mut e, 27);
    capture(&e, "mooncake-credits");
    assert_eq!(key(&mut e, 13), FrameStatus::Halted(0));
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn tunshi_equips_explores_and_finishes_both_battle_results() {
    fn key(e: &mut Emulator, k: u8) -> FrameStatus {
        e.input_mut().release_all();
        e.input_mut().set_keys([k]);
        for frame in 0..4000 {
            let status = e.run_frame().unwrap().status;
            if frame == 0 {
                e.input_mut().release_all();
            }
            if matches!(
                status,
                FrameStatus::WaitingForInput | FrameStatus::Halted(_)
            ) {
                return status;
            }
        }
        panic!("scripted input did not reach its next wait");
    }
    fn word(e: &Emulator, address: usize) -> i16 {
        i16::from_le_bytes(e.vm().memory()[address..address + 2].try_into().unwrap())
    }
    fn hp(e: &Emulator, side: usize) -> i32 {
        (side * 5..side * 5 + 5)
            .map(|slot| i32::from(word(e, 0x425f + slot * 48)))
            .sum()
    }
    let mut e = load(&fixture_path("tunshi_V0.29.lav"));
    advance(&mut e, 1200);
    let title_bp = e.vm().local_bp();
    key(&mut e, 13);
    capture(&e, "tunshi-intro");
    for _ in 0..42 {
        key(&mut e, 13);
    }
    assert_eq!(e.vm().pc(), 0x1ed);
    assert_eq!(e.vm().memory()[0x4210], 5);
    assert_eq!((word(&e, 0x2000), word(&e, 0x2002)), (3, 3));
    let party = e.vm().memory()[0x4248..0x4338].to_vec();
    capture(&e, "tunshi-start");
    key(&mut e, 23);
    for _ in 0..16 {
        key(&mut e, 21);
    }
    assert_eq!(e.vm().memory()[0x4210], 1);
    assert_eq!((word(&e, 0x2000), word(&e, 0x2002)), (9, 5));
    capture(&e, "tunshi-city");
    for _ in 0..21 {
        key(&mut e, 21);
    }
    assert_eq!(e.vm().memory()[0x4210], 0);
    assert_eq!((word(&e, 0x2000), word(&e, 0x2002)), (-2, 22));
    capture(&e, "tunshi-world");
    for k in [21, 21, 22, 22, 22, 22, 21, 21, 22, 22] {
        if e.vm().local_bp() != 0x47e5 {
            break;
        }
        key(&mut e, k);
    }
    for turn in 0..200 {
        if e.vm().local_bp() != 0x47e5 {
            break;
        }
        key(&mut e, if turn % 2 == 0 { 22 } else { 23 });
    }
    assert_ne!(e.vm().local_bp(), 0x47e5);
    assert!(hp(&e, 0) > 0 && hp(&e, 1) > 0);
    capture(&e, "tunshi-battle");
    for _ in 0..100 {
        key(&mut e, 13);
        if word(&e, 0x2010) == 5 || e.vm().pc() == 0x1b29 {
            break;
        }
    }
    assert_eq!(hp(&e, 0), 0);
    assert!(
        hp(&e, 1) > 0,
        "defeated enemies must not turn a party wipe into victory"
    );
    assert_eq!(word(&e, 0x2010), 5);
    assert_eq!(
        &e.vm().memory()[0x383f..0x3847],
        &[200, 0, 0, 0, 0, 0, 0, 0]
    );
    capture(&e, "tunshi-defeat");
    assert!(matches!(key(&mut e, 13), FrameStatus::WaitingForInput));
    assert_eq!(e.vm().local_bp(), title_bp);
    capture(&e, "tunshi-defeat-returned");
    key(&mut e, 13);
    for _ in 0..42 {
        key(&mut e, 13);
    }
    assert_eq!(&e.vm().memory()[0x4248..0x4338], party);
    assert_eq!(e.vm().memory()[0x4210], 5);
    capture(&e, "tunshi-restarted");
    let map_frame = e.display().indexed_frame().to_vec();
    key(&mut e, 13);
    key(&mut e, 27);
    assert_eq!(e.vm().local_bp(), 0x47e5);
    capture(&e, "tunshi-menu-canceled");
    let items = e.files().file("/TSTDitem.dat").unwrap().to_vec();
    for member in 0..5 {
        let base = 0x4248 + member * 48;
        let inventory = e.vm().memory()[base + 0x24..base + 0x28].to_vec();
        for _ in 0..4 {
            for k in [13, 21, 21, 21, 21, 13] {
                key(&mut e, k);
            }
            for _ in 0..member {
                key(&mut e, 21);
            }
            for k in [13, 13, 21, 21, 13, 13] {
                key(&mut e, k);
            }
            assert_eq!(e.vm().pc(), 0x2d11);
            key(&mut e, 13);
            key(&mut e, 13);
            assert_eq!(e.vm().local_bp(), 0x47e5);
        }
        assert_eq!(&e.vm().memory()[base + 0x20..base + 0x24], inventory);
        assert_eq!(&e.vm().memory()[base + 0x24..base + 0x2a], &[0; 6]);
        assert_eq!(
            e.vm().memory()[base + 0x13],
            items[usize::from(inventory[0]) * 10 + 8]
        );
        let defense: u8 = inventory[1..]
            .iter()
            .map(|&id| items[usize::from(id) * 10 + 8])
            .sum();
        assert_eq!(e.vm().memory()[base + 0x14], defense);
        capture(&e, &format!("tunshi-equipped-{member}"));
    }
    for y in 0..80 {
        for x in 0..160 {
            if !(64..80).contains(&x) || !(32..48).contains(&y) {
                assert_eq!(
                    e.display().indexed_frame()[y * 160 + x],
                    map_frame[y * 160 + x],
                    "closed equipment menus must reveal the map at {x},{y}"
                );
            }
        }
    }
    for k in std::iter::once(23)
        .chain(std::iter::repeat_n(21, 37))
        .chain([21, 21, 22, 22, 22, 22, 21, 21, 22, 22])
    {
        if e.vm().local_bp() != 0x47e5 {
            break;
        }
        key(&mut e, k);
    }
    for turn in 0..200 {
        if e.vm().local_bp() != 0x47e5 {
            break;
        }
        key(&mut e, if turn % 2 == 0 { 22 } else { 23 });
    }
    assert_eq!(e.vm().memory()[0x4210], 0);
    assert_ne!(e.vm().local_bp(), 0x47e5);
    capture(&e, "tunshi-equipped-battle");
    for _ in 0..200 {
        key(&mut e, 13);
        if word(&e, 0x2010) == 5 || e.vm().pc() == 0x1b29 {
            break;
        }
    }
    assert_eq!(e.vm().pc(), 0x1b29);
    assert!(hp(&e, 0) > 0);
    assert_eq!(hp(&e, 1), 0);
    capture(&e, "tunshi-victory");
    key(&mut e, 13);
    assert_eq!(e.vm().local_bp(), 0x47e5);
    assert!(word(&e, 0x383f) > 200);
    assert!(word(&e, 0x3843) > 0);
    capture(&e, "tunshi-rewarded");
    key(&mut e, 27);
    assert_eq!(e.vm().local_bp(), title_bp);
    capture(&e, "tunshi-escape-returned");
    key(&mut e, 21);
    key(&mut e, 21);
    key(&mut e, 13);
    capture(&e, "tunshi-credits");
    assert_eq!(key(&mut e, 13), FrameStatus::Halted(0));
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn sword_versions_preserve_both_native_save_slots() {
    fn key(e: &mut Emulator, k: u8) {
        e.input_mut().release_all();
        advance(e, 60);
        for frame in 0..120 {
            e.input_mut().set_keys((frame == 0).then_some(k));
            e.run_frame().unwrap();
        }
    }
    fn field(e: &Emulator) -> bool {
        e.vm().local_bp() == 0x3f43 && (0x286..=0x29b).contains(&e.vm().pc())
    }
    for (variant, name) in [("package", "听剑.pac"), ("loose", "Sword.lav")] {
        let mut e = load(&fixture_path(name));
        advance(&mut e, 1200);
        for _ in 0..10 {
            if field(&e) {
                break;
            }
            key(&mut e, 13);
        }
        assert!(field(&e));
        assert_eq!(&e.vm().memory()[0x2a86..0x2a88], &[3, 2]);
        capture(&e, &format!("sword-{variant}-started"));
        for k in [25, 22, 22, 22, 22, 13, 13, 21, 13] {
            key(&mut e, k);
        }
        let saved = e.files().file("/lavadata/sd_save.dat").unwrap();
        assert_eq!(
            saved.len(),
            4000,
            "saving the second slot first must reserve both slots"
        );
        assert!(saved[..2000].iter().all(|&v| v == 0));
        assert!(saved[2000..].iter().any(|&v| v != 0));
        capture(&e, &format!("sword-{variant}-saved-second-slot-first"));
        let second_slot = saved[2000..].to_vec();
        for _ in 0..8 {
            if field(&e) {
                break;
            }
            key(&mut e, 27);
        }
        assert!(field(&e));
        for k in [22, 22, 20, 20, 13, 13] {
            key(&mut e, k);
        }
        assert_eq!(&e.vm().memory()[0x2a86..0x2a88], &[4, 1]);
        assert_eq!(e.vm().memory()[0x2a55], 1);
        capture(&e, &format!("sword-{variant}-coat-collected"));
        for k in [25, 22, 22, 22, 22, 13, 13, 13] {
            key(&mut e, k);
        }
        let saved = e.files().file("/lavadata/sd_save.dat").unwrap().to_vec();
        assert_eq!(saved.len(), 4000);
        assert_eq!(&saved[2000..], second_slot);
        assert!(saved[..2000].iter().any(|&v| v != 0));
        capture(&e, &format!("sword-{variant}-both-slots-saved"));
        for (slot, position, coats) in [(1, [4, 1], 1), (2, [3, 2], 0)] {
            let mut restored = load(&fixture_path(name));
            restored
                .files_mut()
                .import_file("/lavadata/sd_save.dat", saved.clone());
            advance(&mut restored, 1200);
            key(&mut restored, 20);
            key(&mut restored, 13);
            if slot == 2 {
                key(&mut restored, 21);
            }
            key(&mut restored, 13);
            for _ in 0..8 {
                if field(&restored) {
                    break;
                }
                key(&mut restored, 13);
            }
            assert!(field(&restored));
            assert_eq!(&restored.vm().memory()[0x2a86..0x2a88], &position);
            assert_eq!(restored.vm().memory()[0x2a55], coats);
            assert_eq!(&restored.vm().memory()[0x2a8a..0x2a8c], &[32, 1]);
            capture(&restored, &format!("sword-{variant}-slot-{slot}-restored"));
        }
    }
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn sword_versions_equip_follow_the_opening_story_and_finish_a_battle() {
    fn key(e: &mut Emulator, k: u8) {
        e.input_mut().release_all();
        for frame in 0..180 {
            e.input_mut().set_keys((frame == 60).then_some(k));
            if matches!(e.run_frame().unwrap().status, FrameStatus::Halted(_)) {
                return;
            }
        }
    }
    fn field(e: &Emulator) -> bool {
        e.vm().local_bp() == 0x3f43 && (0x286..=0x29b).contains(&e.vm().pc())
    }
    fn word(e: &Emulator, address: usize) -> u16 {
        u16::from_le_bytes(e.vm().memory()[address..address + 2].try_into().unwrap())
    }
    for (variant, name) in [("package", "听剑.pac"), ("loose", "Sword.lav")] {
        let mut e = load(&fixture_path(name));
        advance(&mut e, 1200);
        for _ in 0..10 {
            if field(&e) {
                break;
            }
            key(&mut e, 13);
        }
        assert!(field(&e));
        for k in [22, 22, 20, 20, 13] {
            key(&mut e, k);
        }
        capture(&e, &format!("sword-{variant}-coat-found"));
        key(&mut e, 13);
        assert_eq!(e.vm().memory()[0x2a55], 1);
        let defense = e.vm().memory()[0x29f9];
        for k in [25, 22, 22, 13, 21, 13, 13, 121] {
            key(&mut e, k);
        }
        assert_eq!(e.vm().memory()[0x2a9c], 3);
        assert!(e.vm().memory()[0x29f9] > defense);
        capture(&e, &format!("sword-{variant}-coat-equipped"));
        for _ in 0..8 {
            if field(&e) {
                break;
            }
            key(&mut e, 27);
        }
        assert!(field(&e));
        for k in std::iter::repeat_n(21, 5)
            .chain(std::iter::repeat_n(22, 4))
            .chain([21; 3])
        {
            key(&mut e, k);
        }
        assert_eq!(&e.vm().memory()[0x2a8a..0x2a8c], &[5, 2]);
        capture(&e, &format!("sword-{variant}-opening-conversation"));
        for _ in 0..50 {
            if field(&e) {
                break;
            }
            key(&mut e, 13);
        }
        assert!(field(&e));
        assert_eq!(e.vm().memory()[0x2aab], 1);
        assert_eq!(&e.vm().memory()[0x2a86..0x2a88], &[3, 2]);
        capture(&e, &format!("sword-{variant}-courtyard"));
        for k in [
            21, 23, 23, 23, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 22, 22, 22, 22, 22, 21, 21, 21,
            21, 21, 21, 21, 21, 22, 22, 22, 22, 22, 22, 22, 22, 22, 22, 22, 22,
        ] {
            key(&mut e, k);
        }
        assert_eq!(&e.vm().memory()[0x2a8a..0x2a8c], &[2, 63]);
        assert_eq!(&e.vm().memory()[0x2a86..0x2a88], &[1, 3]);
        capture(&e, &format!("sword-{variant}-forest"));
        let mut direction = 22;
        for _ in 0..100 {
            if !field(&e) {
                break;
            }
            let x = u16::from(e.vm().memory()[0x2a8a]) + u16::from(e.vm().memory()[0x2a86]);
            if x >= 35 {
                direction = 23;
            } else if x <= 8 {
                direction = 22;
            }
            key(&mut e, direction);
        }
        assert!(!field(&e));
        assert_ne!(e.vm().memory()[0x2958], 0);
        let enemy_hp = word(&e, 0x2ce7);
        let player_hp = word(&e, 0x2c83);
        let money = word(&e, 0x2a1f);
        let experience = word(&e, 0x29ea);
        assert!(enemy_hp > 0 && player_hp > 0);
        capture(&e, &format!("sword-{variant}-battle"));
        let mut enemy_damaged = false;
        let mut player_damaged = false;
        let mut victory = false;
        for _ in 0..200 {
            key(&mut e, 13);
            enemy_damaged |= word(&e, 0x2ce7) < enemy_hp;
            player_damaged |= word(&e, 0x2c83) < player_hp;
            if word(&e, 0x2a1f) > money && !victory {
                capture(&e, &format!("sword-{variant}-victory"));
                victory = true;
            }
            if field(&e) {
                break;
            }
        }
        assert!(field(&e));
        assert!(enemy_damaged && player_damaged && victory);
        assert!(word(&e, 0x29e6) > 0);
        assert!(word(&e, 0x29ea) > experience);
        capture(&e, &format!("sword-{variant}-after-battle"));
        for k in [25, 22, 22, 22, 22, 13, 21, 21, 13, 13] {
            key(&mut e, k);
        }
        assert_eq!(e.run_frame().unwrap().status, FrameStatus::Halted(0));
    }
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn four_player_mahjong_selects_discards_and_restarts_after_a_draw() {
    let mut e = load(&fixture_path("MahJong.lav"));
    advance(&mut e, 600);
    assert_eq!(e.vm().pc(), 0x1b79);
    assert_eq!(e.vm().memory()[0x2145], 54);
    assert_eq!(e.vm().memory()[0x20d8], 0);
    let bp = e.vm().local_bp() as usize;
    assert_eq!(e.vm().memory()[bp + 10], 14);
    capture(&e, "four-mahjong-dealt");
    let hand = e.vm().memory()[0x214a..0x2158].to_vec();
    let initial_frame = e.display().indexed_frame().to_vec();
    press(&mut e, 23);
    assert_eq!(e.vm().memory()[bp + 10], 13);
    assert_eq!(&e.vm().memory()[0x214a..0x2158], hand);
    assert_ne!(e.display().indexed_frame(), initial_frame);
    capture(&e, "four-mahjong-select-previous");
    press(&mut e, 22);
    assert_eq!(e.vm().memory()[bp + 10], 14);
    assert_eq!(e.display().indexed_frame(), initial_frame);
    let discarded = hand[13];
    let mut empty_wall = false;
    let mut draw_message = false;
    let mut restarted = false;
    let mut first_discard = false;
    for frame in 0..6000 {
        e.input_mut().set_keys((frame % 60 == 0).then_some(13));
        let result = e.run_frame().unwrap();
        let remaining = e.vm().memory()[0x2145];
        assert!(
            remaining <= 54,
            "drawing from an empty wall must not wrap the remaining count"
        );
        if !first_discard && e.vm().memory()[0x20d8] > 0 {
            assert_eq!(e.vm().memory()[0x206c], discarded);
            capture(&e, "four-mahjong-discarded");
            first_discard = true;
        }
        if remaining == 0 && !empty_wall {
            capture(&e, "four-mahjong-empty-wall");
            empty_wall = true;
        }
        if empty_wall
            && !draw_message
            && e.vm().pc() == 0x1b9
            && matches!(result.status, FrameStatus::WaitingForInput)
        {
            assert_eq!(remaining, 0);
            capture(&e, "four-mahjong-draw-message");
            draw_message = true;
        }
        if draw_message && remaining == 54 && e.vm().pc() == 0x1b79 {
            assert_eq!(e.vm().memory()[0x20d8], 0);
            assert_ne!(&e.vm().memory()[0x214a..0x2158], hand);
            capture(&e, "four-mahjong-redealt");
            restarted = true;
            break;
        }
    }
    assert!(first_discard && empty_wall && draw_message && restarted);
    let mut winning = e.clone();
    let mut winner = None;
    for frame in 0..36000 {
        winning.input_mut().set_keys(if frame % 60 == 0 {
            vec![104, 13]
        } else {
            vec![104]
        });
        let result = winning.run_frame().unwrap();
        assert!(winning.vm().memory()[0x2145] <= 54);
        if winning.vm().pc() == 0x1b9 && matches!(result.status, FrameStatus::WaitingForInput) {
            let modal = winning.vm().local_bp() as usize;
            let parent = u16::from_le_bytes(
                winning.vm().memory()[modal + 3..modal + 5]
                    .try_into()
                    .unwrap(),
            ) as usize;
            let player = winning.vm().memory()[parent + 5];
            if player < 4 {
                winner = Some(player);
                capture(&winning, "four-mahjong-win");
                break;
            }
        }
    }
    assert!(winner.is_some(), "a native winning result must be reached");
    winning.input_mut().release_all();
    advance(&mut winning, 1);
    press(&mut winning, 13);
    assert_eq!(winning.vm().memory()[0x21ae], winner.unwrap());
    assert!((1..=54).contains(&winning.vm().memory()[0x2145]));
    assert_ne!(winning.vm().pc(), 0x1b9);
    capture(&winning, "four-mahjong-after-win");
    let mut exited = false;
    for frame in 0..600 {
        let exit_wait = e.vm().pc() == 0x1aba;
        e.input_mut().set_keys(if exit_wait {
            Some(27)
        } else {
            (frame % 60 == 0).then_some(13)
        });
        if matches!(e.run_frame().unwrap().status, FrameStatus::Halted(0)) {
            exited = true;
            break;
        }
    }
    assert!(exited);
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn trapped_spider_versions_capture_territory_win_and_restore_the_ranking() {
    fn key(e: &mut Emulator, k: u8, frames: usize) {
        e.input_mut().release_all();
        for frame in 0..frames {
            e.input_mut().set_keys((frame == 0 && k != 0).then_some(k));
            if matches!(e.run_frame().unwrap().status, FrameStatus::Halted(_)) {
                return;
            }
        }
    }
    fn route(e: &mut Emulator, keys: impl IntoIterator<Item = u8>) {
        for k in keys {
            key(e, k, 12);
        }
    }
    fn word(e: &Emulator, a: usize) -> u16 {
        u16::from_le_bytes(e.vm().memory()[a..a + 2].try_into().unwrap())
    }
    for (variant, name) in [("package", "kcd.pac"), ("loose", "困虫斗.lav")] {
        let mut e = load(&fixture_path(name));
        advance(&mut e, 1200);
        capture(&e, &format!("spider-{variant}-title"));
        key(&mut e, 13, 1200);
        assert_eq!(e.vm().pc(), 0x2d64);
        assert_eq!(word(&e, 0x28c3), 1);
        capture(&e, &format!("spider-{variant}-difficulty"));
        key(&mut e, 13, 1800);
        assert_eq!(word(&e, 0x28b9), 252);
        assert_eq!(word(&e, 0x28cb), 1);
        let mut collision = e.clone();
        route(
            &mut collision,
            [22; 9]
                .into_iter()
                .chain([97])
                .chain([21; 14])
                .chain([0; 3]),
        );
        assert_eq!(word(&collision, 0x28cb), 0);
        assert_eq!(word(&collision, 0x28b9), 252);
        assert_eq!(word(&collision, 0x289b), 0);
        assert_eq!(
            [word(&collision, 0x2893), word(&collision, 0x2895)],
            [10, 1]
        );
        capture(&collision, &format!("spider-{variant}-collision-retreat"));
        route(
            &mut e,
            [22; 7]
                .into_iter()
                .chain([97])
                .chain([21; 14])
                .chain([0; 3]),
        );
        assert_eq!(word(&e, 0x28b9), 154);
        assert_eq!(word(&e, 0x289b), 0);
        capture(&e, &format!("spider-{variant}-first-capture"));
        route(
            &mut e,
            [20; 7]
                .into_iter()
                .chain([97])
                .chain([22; 11])
                .chain([0; 3]),
        );
        assert_eq!(word(&e, 0x28b9), 77);
        capture(&e, &format!("spider-{variant}-second-capture"));
        route(
            &mut e,
            [23; 6].into_iter().chain([97]).chain([21; 7]).chain([0; 3]),
        );
        assert_eq!(word(&e, 0x28b9), 35);
        capture(&e, &format!("spider-{variant}-third-capture"));
        route(
            &mut e,
            [23; 5]
                .into_iter()
                .chain([20; 4])
                .chain([0; 9])
                .chain([97])
                .chain([22; 5])
                .chain([0; 2]),
        );
        assert_eq!(word(&e, 0x28b9), 20);
        assert_eq!(word(&e, 0x28b7), 4);
        assert_eq!(word(&e, 0x28cb), 1);
        key(&mut e, 0, 72);
        assert_eq!(e.vm().pc(), 0x457);
        assert_eq!(word(&e, 0x28bd), 20);
        capture(&e, &format!("spider-{variant}-victory"));
        for _ in 0..3 {
            key(&mut e, 13, 24);
        }
        assert_eq!(word(&e, 0x28b9), 288);
        assert_eq!(e.vm().pc(), 0x26e2);
        for k in [69, 77, 85, 13] {
            key(&mut e, k, 24);
        }
        assert_eq!(e.vm().pc(), 0x243d);
        let saved = e.files().file("/lavadata/kcd_data.dat").unwrap().to_vec();
        assert_eq!(saved.len(), 193);
        assert!(saved.windows(3).any(|v| v == b"EMU"));
        assert!(saved.windows(3).any(|v| v == b"288"));
        capture(&e, &format!("spider-{variant}-ranking-saved"));
        let mut restored = load(&fixture_path(name));
        restored
            .files_mut()
            .import_file("/lavadata/kcd_data.dat", saved.clone());
        advance(&mut restored, 1200);
        for k in [21, 21, 13] {
            key(&mut restored, k, 24);
        }
        assert_eq!(restored.vm().pc(), 0x243d);
        assert!(restored.vm().memory().windows(3).any(|v| v == b"EMU"));
        assert!(restored.vm().memory().windows(3).any(|v| v == b"288"));
        assert_eq!(
            restored.files().file("/lavadata/kcd_data.dat").unwrap(),
            saved
        );
        capture(&restored, &format!("spider-{variant}-ranking-restored"));
        key(&mut restored, 13, 24);
        key(&mut restored, 13, 1200);
        assert_eq!(restored.vm().pc(), 0x227e);
        key(&mut restored, 20, 24);
        key(&mut restored, 13, 24);
        capture(&restored, &format!("spider-{variant}-farewell"));
        key(&mut restored, 13, 24);
        key(&mut restored, 13, 24);
        assert_eq!(restored.run_frame().unwrap().status, FrameStatus::Halted(0));
    }
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn territory_game_finishes_two_player_and_computer_boards() {
    fn key(e: &mut Emulator, k: u8, frames: usize) {
        e.input_mut().release_all();
        for f in 0..frames {
            e.input_mut().set_keys((f == 0).then_some(k));
            if matches!(e.run_frame().unwrap().status, FrameStatus::Halted(_)) {
                return;
            }
        }
    }
    for (mode, choice) in [("two-player", 110), ("computer", 98)] {
        let mut e = load(&fixture_path("圈地运动.lav"));
        advance(&mut e, 1200);
        key(&mut e, 13, 1200);
        assert_eq!(e.vm().pc(), 0x10b1);
        capture(&e, &format!("territory-{mode}-title"));
        key(&mut e, 13, 120);
        assert_eq!(e.vm().pc(), 0x2286);
        assert_eq!(e.vm().memory()[0x2e68], 1);
        capture(&e, &format!("territory-{mode}-size"));
        key(&mut e, 13, 120);
        assert_eq!(e.vm().pc(), 0x32a4);
        capture(&e, &format!("territory-{mode}-mode"));
        key(&mut e, choice, 180);
        assert_eq!(e.vm().memory()[0x2c4b], 3);
        capture(&e, &format!("territory-{mode}-board"));
        let mut extra_turn = false;
        for _ in 0..24 {
            if e.vm().memory()[0x2e67] == 24 {
                break;
            }
            assert_eq!(e.vm().pc(), 0x374d);
            let mut edge = None;
            for y in 0..=3 {
                for x in 0..3 {
                    if e.vm().memory()[0x2c51 + (y * 2 + 1) * 23 + x * 2 + 2] == 0 {
                        edge = Some((x, y, 22));
                        break;
                    }
                }
                if edge.is_some() {
                    break;
                }
            }
            if edge.is_none() {
                for y in 0..3 {
                    for x in 0..=3 {
                        if e.vm().memory()[0x2c51 + (y * 2 + 2) * 23 + x * 2 + 1] == 0 {
                            edge = Some((x, y, 21));
                            break;
                        }
                    }
                    if edge.is_some() {
                        break;
                    }
                }
            }
            let (x, y, direction) = edge.expect("an unfinished board must have an open edge");
            for _ in 0..4 {
                if e.vm().memory()[0x2c4f] as usize == x {
                    break;
                }
                key(&mut e, 22, 12);
            }
            for _ in 0..4 {
                if e.vm().memory()[0x2c50] as usize == y {
                    break;
                }
                key(&mut e, 21, 12);
            }
            assert_eq!(
                (
                    e.vm().memory()[0x2c4f] as usize,
                    e.vm().memory()[0x2c50] as usize
                ),
                (x, y)
            );
            let lines = e.vm().memory()[0x2e67];
            let turn = e.vm().memory()[0x2e66];
            let score = u16::from(e.vm().memory()[0x2e64]) + u16::from(e.vm().memory()[0x2e65]);
            key(&mut e, 13, 12);
            assert_eq!(e.vm().pc(), 0x3792);
            key(&mut e, direction, 180);
            assert!(e.vm().memory()[0x2e67] > lines);
            let new_score = u16::from(e.vm().memory()[0x2e64]) + u16::from(e.vm().memory()[0x2e65]);
            assert!(new_score >= score && new_score <= 9);
            if mode == "two-player" && e.vm().memory()[0x2e67] < 24 {
                assert_eq!(e.vm().memory()[0x2e67], lines + 1);
                assert_eq!(
                    e.vm().memory()[0x2e66],
                    if new_score > score { turn } else { 1 - turn }
                );
                extra_turn |= new_score > score;
            }
        }
        assert_eq!(e.vm().memory()[0x2e67], 24);
        assert_eq!(
            u16::from(e.vm().memory()[0x2e64]) + u16::from(e.vm().memory()[0x2e65]),
            9
        );
        if mode == "two-player" {
            assert!(extra_turn);
        }
        capture(&e, &format!("territory-{mode}-result"));
        key(&mut e, 13, 1200);
        assert_eq!(e.vm().pc(), 0x10b1);
        key(&mut e, 27, 120);
        capture(&e, &format!("territory-{mode}-credits"));
        key(&mut e, 13, 120);
        assert_eq!(e.run_frame().unwrap().status, FrameStatus::Halted(0));
    }
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn saints_vs_gold_completes_training_and_an_eight_member_battle() {
    fn key(e: &mut Emulator, k: u8) {
        e.input_mut().release_all();
        for f in 0..24 {
            e.input_mut().set_keys((f == 0).then_some(k));
            e.run_frame().unwrap();
        }
    }
    fn reach(e: &mut Emulator, pc: usize) {
        for _ in 0..10000 {
            if e.vm().pc() == pc {
                return;
            }
            e.input_mut().release_all();
            let result = e.run_frame().unwrap();
            if matches!(result.status, FrameStatus::WaitingForInput) && e.vm().pc() != pc {
                key(e, 13);
            }
        }
        assert_eq!(
            e.vm().pc(),
            pc,
            "the original flow must reach its next input prompt"
        );
    }
    fn actor(e: &Emulator, player: usize, field: usize) -> u8 {
        e.vm().memory()[0x2f37 + player * 89 + field]
    }
    let mut e = load(&fixture_path("Shengjin2.lav"));
    advance(&mut e, 1200);
    assert_eq!(e.vm().pc(), 0x5e1c);
    capture(&e, "saints-title");
    for k in [13, 21, 21, 21, 13] {
        key(&mut e, k);
    }
    assert_eq!(e.vm().pc(), 0x5256);
    capture(&e, "saints-training-intro");
    reach(&mut e, 0x198f);
    for player in [0, 1, 4, 5] {
        assert_eq!(actor(&e, player, 0x37), 5);
        assert_eq!(actor(&e, player, 0x38), 1);
    }
    key(&mut e, 98);
    reach(&mut e, 0x198f);
    for player in [0, 1, 4, 5] {
        assert_eq!(actor(&e, player, 0x38), 2);
    }
    capture(&e, "saints-first-charge");
    key(&mut e, 98);
    reach(&mut e, 0x198f);
    assert_eq!(actor(&e, 0, 0x38), 3);
    capture(&e, "saints-second-charge");
    key(&mut e, 104);
    assert_eq!(e.vm().pc(), 0x15d5);
    key(&mut e, 21);
    capture(&e, "saints-move-selection");
    key(&mut e, 13);
    assert_eq!(e.vm().pc(), 0x17b1);
    capture(&e, "saints-target-selection");
    key(&mut e, 13);
    reach(&mut e, 0x198f);
    assert_eq!(actor(&e, 0, 0x39), 6);
    assert_eq!(actor(&e, 0, 0x38), 2);
    assert_eq!(actor(&e, 4, 0x37), 2);
    capture(&e, "saints-attack-landed");
    key(&mut e, 109);
    reach(&mut e, 0x198f);
    assert_eq!(actor(&e, 0, 0x37), 5);
    assert_eq!(actor(&e, 0, 0x39), 3);
    assert_eq!(actor(&e, 0, 0x3d), 8);
    capture(&e, "saints-flight-avoids-damage");
    key(&mut e, 103);
    reach(&mut e, 0x5d48);
    assert_eq!(actor(&e, 0, 0x37), 5);
    assert_eq!(actor(&e, 0, 0x3d), 3);
    capture(&e, "saints-shield-absorbs-damage");
    reach(&mut e, 0x5e04);
    capture(&e, "saints-training-complete");
    key(&mut e, 13);
    assert_eq!(e.vm().pc(), 0x6f2);
    for k in [13, 13, 13, 20, 13] {
        key(&mut e, k);
    }
    assert_eq!(e.vm().pc(), 0xb12);
    for player in 0..8 {
        assert_eq!(actor(&e, player, 0x3c), 0);
    }
    for k in [13, 20, 13] {
        key(&mut e, k);
    }
    let mut damaged = [false; 2];
    let mut defeated = false;
    e.input_mut().release_all();
    for _ in 0..60000 {
        e.run_frame().unwrap();
        for (side, hit) in damaged.iter_mut().enumerate() {
            if !*hit && (side * 4..side * 4 + 4).any(|p| actor(&e, p, 0x37) < 5) {
                *hit = true;
                capture(&e, &format!("saints-battle-side-{side}-damaged"));
            }
        }
        defeated |= (0..8).any(|p| actor(&e, p, 0x3c) == 2 && actor(&e, p, 0x37) == 0);
        if e.vm().pc() == 0x3a9e {
            break;
        }
    }
    assert_eq!(e.vm().pc(), 0x3a9e);
    assert!(damaged.into_iter().all(|hit| hit) && defeated);
    let alive = |side: usize| {
        (side * 4..side * 4 + 4)
            .filter(|&p| actor(&e, p, 0x3c) != 2 && actor(&e, p, 0x37) > 0)
            .count()
    };
    assert!((alive(0) == 0) != (alive(1) == 0));
    capture(&e, "saints-battle-victory");
    key(&mut e, 13);
    assert_eq!(e.vm().pc(), 0x41b5);
    capture(&e, "saints-ratings");
    key(&mut e, 21);
    assert_eq!(e.vm().pc(), 0x41b5);
    capture(&e, "saints-ratings-next");
    key(&mut e, 13);
    assert_eq!(e.vm().pc(), 0x6f2);
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn tank_2004_versions_move_fire_pause_and_restore_a_native_save() {
    fn key(e: &mut Emulator, k: u8, held: usize, frames: usize) {
        e.input_mut().release_all();
        for f in 0..frames {
            e.input_mut().set_keys((f < held).then_some(k));
            e.run_frame().unwrap();
        }
    }
    fn word(e: &Emulator, a: usize) -> u16 {
        u16::from_le_bytes(e.vm().memory()[a..a + 2].try_into().unwrap())
    }
    for (variant, name) in [("package", "坦克2004.pac"), ("loose", "坦克2004.lav")] {
        let mut e = load(&fixture_path(name));
        advance(&mut e, 1200);
        for _ in 0..6 {
            key(&mut e, 13, 1, 24);
        }
        assert_eq!(e.vm().pc(), 0x8c9b);
        assert_eq!((word(&e, 0x6704), word(&e, 0x6706)), (48, 72));
        let mut battle = e.clone();
        for (k, frames) in [
            (20, 8),
            (122, 24),
            (20, 8),
            (122, 24),
            (23, 8),
            (122, 24),
            (20, 8),
            (122, 24),
            (22, 8),
            (122, 24),
            (20, 8),
        ] {
            key(&mut battle, k, frames, frames);
        }
        let score = u32::from_le_bytes(battle.vm().memory()[0x20a0..0x20a4].try_into().unwrap());
        assert_eq!(score, 440);
        capture(&battle, &format!("tank-{variant}-enemy-destroyed"));
        let mut loss = e.clone();
        key(&mut loss, 122, 1800, 1800);
        assert_eq!(loss.vm().pc(), 0x5a01);
        capture(&loss, &format!("tank-{variant}-battle-ended"));
        key(&mut e, 23, 8, 8);
        assert_eq!(word(&e, 0x6708), 2);
        key(&mut e, 20, 8, 8);
        assert_eq!((word(&e, 0x6704), word(&e, 0x6706)), (48, 56));
        capture(&e, &format!("tank-{variant}-moved"));
        key(&mut e, 122, 8, 8);
        assert_eq!(word(&e, 0x6492), 1);
        capture(&e, &format!("tank-{variant}-firing"));
        key(&mut e, 112, 1, 1);
        assert_eq!(e.vm().pc(), 0x64cb);
        let frozen = e.vm().memory()[0x625a..0x6710].to_vec();
        let picture = e.display().indexed_frame().to_vec();
        e.input_mut().release_all();
        advance(&mut e, 120);
        assert_eq!(&e.vm().memory()[0x625a..0x6710], frozen);
        assert_eq!(e.display().indexed_frame(), picture);
        capture(&e, &format!("tank-{variant}-paused"));
        key(&mut e, 13, 1, 1);
        key(&mut e, 115, 1, 1);
        assert_eq!(e.vm().pc(), 0x5121);
        let position = (word(&e, 0x6704), word(&e, 0x6706));
        key(&mut e, 121, 1, 24);
        let saved = e.files().file("/lavadata/tanksave.dat").unwrap().to_vec();
        assert!(!saved.is_empty());
        capture(&e, &format!("tank-{variant}-saved"));
        key(&mut e, 21, 12, 12);
        assert_ne!((word(&e, 0x6704), word(&e, 0x6706)), position);
        key(&mut e, 114, 1, 1);
        assert_eq!(e.vm().pc(), 0x4b12);
        key(&mut e, 121, 1, 24);
        assert_eq!((word(&e, 0x6704), word(&e, 0x6706)), position);
        capture(&e, &format!("tank-{variant}-restored"));
        let mut fresh = load(&fixture_path(name));
        fresh
            .files_mut()
            .import_file("/lavadata/tanksave.dat", saved.clone());
        advance(&mut fresh, 1200);
        key(&mut fresh, 13, 1, 24);
        key(&mut fresh, 13, 1, 240);
        assert_eq!(fresh.vm().pc(), 0x6e29);
        key(&mut fresh, 21, 1, 24);
        key(&mut fresh, 13, 1, 120);
        assert_eq!((word(&fresh, 0x6704), word(&fresh, 0x6706)), position);
        assert_eq!(fresh.files().file("/lavadata/tanksave.dat").unwrap(), saved);
        capture(&fresh, &format!("tank-{variant}-fresh-restored"));
        key(&mut fresh, 27, 1, 120);
        assert_eq!(fresh.vm().pc(), 0x6e29);
        for _ in 0..5 {
            key(&mut fresh, 22, 1, 24);
        }
        key(&mut fresh, 13, 1, 24);
        assert_eq!(fresh.vm().pc(), 0x6fc2);
        capture(&fresh, &format!("tank-{variant}-farewell"));
        key(&mut fresh, 13, 1, 1);
        assert_eq!(fresh.run_frame().unwrap().status, FrameStatus::Halted(0));
    }
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn zelda_minigames_hit_lizards_open_a_chest_time_out_and_configure_controls() {
    fn key(e: &mut Emulator, k: u8) {
        e.input_mut().release_all();
        for f in 0..24 {
            e.input_mut().set_keys((f == 0).then_some(k));
            e.run_frame().unwrap();
        }
    }
    fn w(e: &Emulator, a: usize) -> i32 {
        u16::from_le_bytes(e.vm().memory()[a..a + 2].try_into().unwrap()) as i32
    }
    let mut e = load(&fixture_path("文曲星版Zelda.lav"));
    advance(&mut e, 1200);
    capture(&e, "zelda-title");
    for _ in 0..3 {
        key(&mut e, 13);
    }
    for stage in 0..3 {
        assert_eq!(w(&e, 0x208a), stage);
        for _ in 0..30000 {
            let n = w(&e, 0x20a2);
            let px = w(&e, 0x2078);
            let py = w(&e, 0x207a);
            let bx = w(&e, 0x2094);
            let by = w(&e, 0x2096);
            let h = w(&e, 0x2098);
            let state = w(&e, 0x2080);
            let mut targets: Vec<_> = (1..=n as usize)
                .filter(|&i| w(&e, 0x20c0 + i * 2) == 0)
                .map(|i| (w(&e, 0x20a8 + i * 2), w(&e, 0x20b4 + i * 2)))
                .collect();
            targets.sort_by_key(|&(x, y)| (x - bx).abs() + (y - by).abs());
            let (x, y) = if state != 2 {
                (bx, by)
            } else {
                targets
                    .first()
                    .map(|&(x, y)| (x, y + 3))
                    .unwrap_or((bx, by))
            };
            let mut keys = Vec::new();
            if (x - px).abs() > 3 {
                keys.push(if x > px { 22 } else { 23 });
            } else if (y - py).abs() > 3 {
                keys.push(if y > py { 21 } else { 20 });
            }
            if state == 2 {
                if let Some(&(x, y)) = targets.first()
                    && (y + 3 - by).abs() < 8
                    && ((x < 32 && px <= 35) || (x > 112 && px >= 109))
                {
                    keys = vec![13, if x < px { 23 } else { 22 }];
                }
            } else if (bx - px).abs() < 12 && (by - py).abs() < 12 && h < 10 {
                keys.push(13);
            }
            e.input_mut().set_keys(keys);
            e.run_frame().unwrap();
            if matches!(e.vm().pc(), 0x1e76 | 0x1e77 | 0x1e33 | 0x1e1e) {
                break;
            }
        }
        e.input_mut().release_all();
        advance(&mut e, 120);
        if stage < 2 {
            assert_eq!(w(&e, 0x2082), 3);
            assert_eq!(w(&e, 0x20a0), 5 + 5 * stage);
            assert_eq!(e.vm().pc(), 0x1e33);
            capture(&e, &format!("zelda-lizards-stage-{stage}-success"));
            key(&mut e, 13);
            assert_eq!(e.vm().pc(), 0x1dcd);
            key(&mut e, 13);
        } else {
            assert_eq!(w(&e, 0x2082), 4);
            assert_eq!(e.vm().pc(), 0x1e77);
            capture(&e, "zelda-lizards-loss");
            key(&mut e, 13);
        }
    }
    assert_eq!(e.vm().pc(), 0x2dda);
    let mut chest = load(&fixture_path("文曲星版Zelda.lav"));
    advance(&mut chest, 1200);
    key(&mut chest, 110);
    assert_eq!(chest.vm().pc(), 0x2c8d);
    capture(&chest, "zelda-chest-board");
    let mut timeout = chest.clone();
    let bank = w(&chest, 0x2e9b) as usize;
    let board = chest.vm().memory()[0x20cd + bank * 50..0x20cd + (bank + 1) * 50].to_vec();
    let mut queue = std::collections::VecDeque::from([(0i32, 0i32, Vec::<u8>::new())]);
    let mut visited = std::collections::HashSet::from([(0, 0)]);
    let mut solution = None;
    while let Some((x, y, route)) = queue.pop_front() {
        for (dx, dy, k) in [(0, -1, 20), (0, 1, 21), (-1, 0, 23), (1, 0, 22)] {
            let (xx, yy) = (x + dx, y + dy);
            if !(0..10).contains(&xx) || !(0..5).contains(&yy) {
                continue;
            }
            let tile = board[(yy * 10 + xx) as usize];
            let mut next = route.clone();
            next.push(k);
            if tile == 2 {
                next.push(13);
                solution = Some(next);
                break;
            }
            if tile == 1 && visited.insert((xx, yy)) {
                queue.push_back((xx, yy, next));
            }
        }
        if solution.is_some() {
            break;
        }
    }
    for k in solution.expect("the original chest must be reachable") {
        key(&mut chest, k);
    }
    assert_eq!(chest.vm().pc(), 0x2c1a);
    capture(&chest, "zelda-chest-opened");
    key(&mut chest, 13);
    key(&mut chest, 13);
    assert_eq!(chest.vm().pc(), 0x2dda);
    timeout.input_mut().release_all();
    advance(&mut timeout, 2400);
    key(&mut timeout, 13);
    timeout.input_mut().release_all();
    advance(&mut timeout, 180);
    assert_eq!(timeout.vm().pc(), 0x2c3c);
    capture(&timeout, "zelda-chest-timeout");
    key(&mut timeout, 13);
    assert_eq!(timeout.vm().pc(), 0x2dda);
    key(&mut chest, 109);
    assert_eq!(chest.vm().pc(), 0x17e4);
    let speed = w(&chest, 0x207e);
    key(&mut chest, 23);
    assert_eq!(w(&chest, 0x207e), speed - 5);
    chest.input_mut().release_all();
    for _ in 0..2000 {
        if chest.vm().pc() == 0x17e4 {
            break;
        }
        chest.run_frame().unwrap();
    }
    assert_eq!(chest.vm().pc(), 0x17e4);
    key(&mut chest, 22);
    assert_eq!(w(&chest, 0x207e), speed);
    chest.input_mut().release_all();
    for _ in 0..2000 {
        if chest.vm().pc() == 0x17e4 {
            break;
        }
        chest.run_frame().unwrap();
    }
    assert_eq!(chest.vm().pc(), 0x17e4);
    capture(&chest, "zelda-settings");
    key(&mut chest, 13);
    assert_eq!(chest.vm().pc(), 0x182c);
    capture(&chest, "zelda-help");
    key(&mut chest, 13);
    assert_eq!(chest.vm().pc(), 0x18af);
    capture(&chest, "zelda-credits");
    key(&mut chest, 13);
    chest.input_mut().release_all();
    for _ in 0..2000 {
        if chest.vm().pc() == 0x17e4 {
            break;
        }
        chest.run_frame().unwrap();
    }
    assert_eq!(chest.vm().pc(), 0x17e4);
    key(&mut chest, 27);
    assert_eq!(chest.vm().pc(), 0x2dda);
    key(&mut chest, 103);
    assert_eq!(chest.run_frame().unwrap().status, FrameStatus::Halted(0));
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn strategy_versions_move_attack_change_turns_and_restore_a_campaign() {
    fn keys(e: &mut Emulator, keys: &[u8]) {
        for &key in keys {
            e.input_mut().release_all();
            for frame in 0..24 {
                e.input_mut().set_keys((frame == 0).then_some(key));
                advance(e, 1);
            }
        }
    }
    fn unit(e: &Emulator, team: usize, index: usize) -> &[u8] {
        let start = 0x225d + (team * 48 + index) * 6;
        &e.vm().memory()[start..start + 6]
    }
    for (variant, filename) in [("packaged", "大战略.pac"), ("loose", "dzl.lav")] {
        let path = fixture_path(filename);
        let mut e = load(&path);
        advance(&mut e, 1200);
        assert_eq!(e.vm().pc(), 0xb93);
        keys(&mut e, &[13, 13]);
        assert_eq!(e.vm().pc(), 0x87d5);
        assert_eq!(&e.vm().memory()[0x300a..0x300f], &[9, 8, 0, 0, 4]);
        capture(&e, &format!("strategy-{variant}-field"));

        let mut movement = e.clone();
        keys(&mut movement, &[13, 22, 13, 21, 21, 21, 21, 21, 21, 13]);
        assert_eq!(movement.vm().pc(), 0x87d5);
        assert_eq!(unit(&movement, 0, 5), &[1, 4, 100, 5, 67, 9]);
        capture(&movement, &format!("strategy-{variant}-moved-artillery"));

        keys(&mut e, &[22, 22, 22, 13, 22, 22, 13, 13, 13]);
        e.input_mut().release_all();
        advance(&mut e, 1200);
        assert_eq!(e.vm().pc(), 0x87d5);
        assert_eq!(unit(&e, 0, 2), &[5, 4, 66, 0, 4, 98]);
        assert_eq!(unit(&e, 1, 2), &[6, 4, 62, 0, 10, 98]);
        capture(&e, &format!("strategy-{variant}-battle-result"));
        keys(&mut e, &[27, 21, 21, 13, 13]);
        assert_eq!(e.vm().pc(), 0x87d5);
        let files = e.files_mut().take_dirty_files();
        let saved = files
            .iter()
            .find(|(name, _)| name == "/lavadata/dzl.sav")
            .unwrap();
        assert_eq!(saved.1.len(), 2015);

        keys(&mut e, &[27, 13]);
        assert_eq!(e.vm().pc(), 0x73ca);
        assert_eq!(e.vm().memory()[0x300c], 1);
        capture(&e, &format!("strategy-{variant}-second-player-turn"));
        keys(&mut e, &[13, 13, 23, 13, 21, 21, 21, 21, 21, 21, 13]);
        assert_eq!(e.vm().pc(), 0x87d5);
        assert_eq!(&unit(&e, 1, 11)[..4], &[8, 4, 100, 5]);
        capture(&e, &format!("strategy-{variant}-second-player-moved"));
        keys(&mut e, &[27, 13, 13]);
        assert_eq!(e.vm().pc(), 0x87d5);
        assert_eq!(e.vm().memory()[0x300c], 0);

        let mut restored = load(&path);
        restored.files_mut().import_file(&saved.0, saved.1.clone());
        advance(&mut restored, 1200);
        keys(&mut restored, &[21, 13, 13]);
        assert_eq!(restored.vm().pc(), 0x87d5);
        assert_eq!(&restored.vm().memory()[0x300c..0x300f], &[0, 5, 4]);
        assert_eq!(unit(&restored, 0, 2), &[5, 4, 66, 0, 4, 98]);
        assert_eq!(unit(&restored, 1, 2), &[6, 4, 62, 0, 10, 98]);
        assert_eq!(&unit(&restored, 1, 11)[..4], &[9, 4, 100, 5]);
        assert!(restored.files_mut().take_dirty_files().is_empty());
        capture(&restored, &format!("strategy-{variant}-restored-campaign"));
    }
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn sky_fighting_art_configures_moves_fires_pauses_and_finishes_a_battle() {
    fn key(e: &mut Emulator, key: u8) {
        e.input_mut().release_all();
        for frame in 0..60 {
            e.input_mut().set_keys((frame < 18).then_some(key));
            advance(e, 1);
        }
    }
    fn word(e: &Emulator, address: usize) -> u16 {
        u16::from_le_bytes(e.vm().memory()[address..address + 2].try_into().unwrap())
    }
    let mut title = load(&fixture_path("HFAcs.lav"));
    advance(&mut title, 1200);
    assert_eq!(title.vm().pc(), 0xa58);
    capture(&title, "sky-fighting-title");
    let mut help = title.clone();
    key(&mut help, 22);
    key(&mut help, 13);
    assert_eq!(help.vm().pc(), 0x1a6);
    capture(&help, "sky-fighting-controls");
    for pc in [0x237, 0x2bb, 0x34b, 0x3bc, 0xa58] {
        key(&mut help, 13);
        assert_eq!(help.vm().pc(), pc);
    }
    assert_eq!(help.vm().pc(), 0xa58);

    for mode in 1..=3 {
        let mut e = title.clone();
        key(&mut e, 13);
        for _ in 1..mode {
            key(&mut e, 21);
        }
        key(&mut e, 13);
        assert_eq!(e.vm().pc(), 0x6a8);
        capture(&e, &format!("sky-fighting-mode-{mode}-settings"));
        let mut configured = e.clone();
        for k in [23, 21, 22, 13] {
            key(&mut configured, k);
        }
        assert_eq!(word(&configured, 0x23ed), 90);
        assert_eq!(word(&configured, 0x23f1), 110);
        capture(&configured, &format!("sky-fighting-mode-{mode}-configured"));

        key(&mut e, 13);
        assert_eq!(e.vm().memory()[0x24f5], mode);
        capture(&e, &format!("sky-fighting-mode-{mode}-battle"));
        let mut paused = e.clone();
        key(&mut paused, b'p');
        assert_eq!(paused.vm().pc(), 0x14f);
        let state = paused.vm().memory()[0x2347..0x24f7].to_vec();
        let frame = paused.display().indexed_frame().to_vec();
        advance(&mut paused, 300);
        assert_eq!(&paused.vm().memory()[0x2347..0x24f7], state.as_slice());
        assert_eq!(paused.display().indexed_frame(), frame.as_slice());
        paused.input_mut().set_keys(*b"p");
        advance(&mut paused, 1);
        paused.input_mut().release_all();
        advance(&mut paused, 60);
        assert_ne!(paused.vm().pc(), 0x14f);

        let position = (word(&e, 0x23f9), word(&e, 0x23fb));
        key(&mut e, b'd');
        assert_ne!((word(&e, 0x23f9), word(&e, 0x23fb)), position);
        let weapon = e.vm().memory()[0x2405];
        key(&mut e, 20);
        assert_ne!(e.vm().memory()[0x2405], weapon);
        let shield = word(&e, 0x23f3);
        e.input_mut().set_keys([22]);
        advance(&mut e, 18);
        e.input_mut().release_all();
        assert!(word(&e, 0x23f3) < shield);
        let support = word(&e, 0x23f7);
        e.input_mut().set_keys([14]);
        advance(&mut e, 18);
        e.input_mut().release_all();
        assert!(word(&e, 0x23f7) < support);
        capture(&e, &format!("sky-fighting-mode-{mode}-controls-used"));
        e.input_mut().set_keys([27]);
        let mut exited = false;
        for _ in 0..120 {
            if let FrameStatus::Halted(code) = e.run_frame().unwrap().status {
                assert_eq!(code, 0);
                exited = true;
                break;
            }
        }
        if mode == 3 && e.vm().pc() == 0x4456 {
            assert_eq!(word(&e, 0x23ef), 0);
            capture(&e, "sky-fighting-duel-score");
            key(&mut e, 13);
            assert_eq!(e.vm().pc(), 0xa58);
            e.input_mut().set_keys([27]);
            assert_eq!(e.run_frame().unwrap().status, FrameStatus::Halted(0));
            exited = true;
        }
        assert!(exited, "mode {mode} did not exit: pc={:x}", e.vm().pc());

        if mode == 1 {
            let mut battle = title.clone();
            for _ in 0..3 {
                key(&mut battle, 13);
            }
            key(&mut battle, 20);
            battle.input_mut().set_keys([21]);
            advance(&mut battle, 6000);
            battle.input_mut().release_all();
            assert!(battle.vm().memory()[0x2348] > 0);
            assert_eq!(word(&battle, 0x23ef), 0);
            capture(&battle, "sky-fighting-native-loss");
            key(&mut battle, 27);
            assert_eq!(battle.vm().pc(), 0x4456);
            capture(&battle, "sky-fighting-battle-score");
            key(&mut battle, 13);
            assert_eq!(battle.vm().pc(), 0xa58);
        }
    }
    title.input_mut().set_keys([27]);
    assert_eq!(title.run_frame().unwrap().status, FrameStatus::Halted(0));
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn eagle_versions_aim_fire_pause_finish_and_restore_the_ranking() {
    fn key(e: &mut Emulator, key: u8) {
        e.input_mut().release_all();
        for frame in 0..60 {
            e.input_mut().set_keys((frame < 18).then_some(key));
            advance(e, 1);
        }
    }
    fn word(e: &Emulator, address: usize) -> i16 {
        i16::from_le_bytes(e.vm().memory()[address..address + 2].try_into().unwrap())
    }
    let standard = fixture_path("Eagle.lav");
    let directory = std::path::Path::new(&standard).parent().unwrap();
    for (name, boot, menu, loss, pause, rank, bp) in [
        ("Eagle.lav", 0x14dd, 0x150c, 0x9cd, 0x10ac, 0x90f, 0x6f07),
        ("NC2600.lav", 0x20cd, 0x20fc, 0xd5b, 0x1459, 0xc4e, 0x5c47),
        ("NC3000.lav", 0x2117, 0x2146, 0xda5, 0x14a3, 0xc98, 0x5c47),
        ("TC1000.lav", 0x2117, 0x2146, 0xda5, 0x14a3, 0xc98, 0x5c47),
        ("TC1000S.lav", 0x2117, 0x2146, 0xda5, 0x14a3, 0xc98, 0x5c47),
        ("TC808.lav", 0x1fc0, 0x1fef, 0xc51, 0x134f, 0xb44, 0x5c47),
    ] {
        let path = if name == "Eagle.lav" {
            directory.join(name)
        } else {
            directory.join("VS版").join(name)
        };
        let path = path.to_str().unwrap();
        let label = name.trim_end_matches(".lav");
        let mut e = load(path);
        advance(&mut e, 1200);
        assert_eq!(e.vm().pc(), boot);
        key(&mut e, 13);
        assert_eq!(e.vm().pc(), menu);
        capture(&e, &format!("eagle-{label}-menu"));
        key(&mut e, 13);
        advance(&mut e, 600);
        assert_eq!(e.vm().local_bp() as usize, bp);
        assert!(word(&e, bp + 0x28) > 0);
        capture(&e, &format!("eagle-{label}-battle"));

        let mut controls = e.clone();
        let altitude = controls.vm().memory()[bp + 0x24];
        key(&mut controls, b'w');
        assert!(controls.vm().memory()[bp + 0x24] > altitude);
        let heading = word(&controls, bp + 0x2c);
        key(&mut controls, b'd');
        assert_ne!(word(&controls, bp + 0x2c), heading);
        key(&mut controls, b'p');
        assert_eq!(controls.vm().pc(), pause);
        let state = controls.vm().memory()[bp..bp + 0x47].to_vec();
        let frame = controls.display().indexed_frame().to_vec();
        advance(&mut controls, 300);
        assert_eq!(&controls.vm().memory()[bp..bp + 0x47], state.as_slice());
        assert_eq!(controls.display().indexed_frame(), frame.as_slice());
        capture(&controls, &format!("eagle-{label}-paused"));
        key(&mut controls, b'y');
        assert_ne!(controls.vm().pc(), pause);

        for _ in 0..12000 {
            if word(&e, bp + 0x2a) > 0 || word(&e, bp + 0x28) <= 0 {
                break;
            }
            let target = (0..5)
                .filter(|&i| e.vm().memory()[bp + 0x38 + i] >= 50)
                .min_by_key(|&i| {
                    i32::from(word(&e, bp + 0x10 + i * 2)).abs()
                        + i32::from(word(&e, bp + 0x1a + i * 2)).abs()
                });
            let aim = target.map(|i| (word(&e, bp + 0x10 + i * 2), word(&e, bp + 0x1a + i * 2)));
            let key = match aim {
                Some((x, _)) if x > 16 => b'd',
                Some((x, _)) if x < -16 => b'a',
                Some((_, y)) if y > 5 => b'w',
                Some((_, y)) if y < -5 => b's',
                _ => b'l',
            };
            e.input_mut().set_keys([key]);
            advance(&mut e, 1);
        }
        e.input_mut().release_all();
        assert_eq!(word(&e, bp + 0x2a), 1);
        assert!(word(&e, bp + 0x28) > 0);
        capture(&e, &format!("eagle-{label}-enemy-destroyed"));
        advance(&mut e, 6000);
        assert_eq!(e.vm().pc(), loss);
        assert!(word(&e, bp + 0x28) <= 0);
        capture(&e, &format!("eagle-{label}-game-over"));
        key(&mut e, 13);
        for k in *b"ABC" {
            key(&mut e, k);
        }
        key(&mut e, 13);
        assert_eq!(e.vm().pc(), rank);
        capture(&e, &format!("eagle-{label}-ranking"));
        let files = e.files_mut().take_dirty_files();
        let save_name = if name == "Eagle.lav" {
            "/lavadata/eagle.cfg"
        } else {
            "/lavadata/eagle_vs.cfg"
        };
        let saved = files.iter().find(|(name, _)| name == save_name).unwrap();
        assert_eq!(saved.1.len(), 55);
        assert_eq!(&saved.1[..4], b"ABC\0");
        assert_eq!(&saved.1[9..11], &[1, 0]);

        let mut restored = load(path);
        restored.files_mut().import_file(&saved.0, saved.1.clone());
        advance(&mut restored, 1200);
        assert_eq!(restored.vm().pc(), boot);
        for k in [13, 21, 21, 21, 13] {
            key(&mut restored, k);
        }
        assert_eq!(restored.vm().pc(), rank);
        assert!(restored.files_mut().take_dirty_files().is_empty());
        assert_eq!(
            restored.display().indexed_frame(),
            e.display().indexed_frame()
        );
        capture(&restored, &format!("eagle-{label}-ranking-restored"));
        key(&mut restored, 13);
        assert_eq!(restored.vm().pc(), menu);
        key(&mut restored, 27);
        capture(&restored, &format!("eagle-{label}-farewell"));
        restored.input_mut().set_keys([13]);
        let mut exited = false;
        for _ in 0..300 {
            if let FrameStatus::Halted(code) = restored.run_frame().unwrap().status {
                assert_eq!(code, 0);
                exited = true;
                break;
            }
        }
        assert!(exited);
    }
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn liuye_moves_answers_a_riddle_fights_and_restores_native_progress() {
    fn key(e: &mut Emulator, k: u8, held: usize, frames: usize) {
        e.input_mut().release_all();
        for f in 0..frames {
            e.input_mut().set_keys((f < held).then_some(k));
            advance(e, 1);
        }
    }
    fn start(path: &str, files: &[(String, Vec<u8>)]) -> Emulator {
        let mut e = load(path);
        for (name, bytes) in files {
            e.files_mut().import_file(name, bytes.clone());
        }
        advance(&mut e, 1200);
        key(&mut e, 13, 1, 24);
        assert_eq!(e.vm().pc(), 0x122e);
        key(&mut e, 13, 1, 24);
        for _ in 0..100 {
            key(&mut e, 13, 18, 60);
        }
        assert_eq!(position(&e), (33, 59));
        e
    }
    fn position(e: &Emulator) -> (u8, u8) {
        let m = e.vm().memory();
        (m[0x43b2] + m[0x43b4], m[0x43b3] + m[0x43b5])
    }
    fn word(e: &Emulator, a: usize) -> u16 {
        u16::from_le_bytes(e.vm().memory()[a..a + 2].try_into().unwrap())
    }
    let path = fixture_path("Liuye.lav");
    let baseline = start(&path, &[]);
    capture(&baseline, "liuye-initial-map");
    let mut e = baseline.clone();
    for k in [
        20, 20, 20, 20, 20, 20, 20, 22, 22, 22, 22, 22, 22, 22, 21, 21, 23, 21, 23, 20, 13,
    ] {
        key(&mut e, k, 18, 60);
    }
    assert_eq!(e.vm().pc(), 0x287);
    capture(&e, "liuye-riddle-question");
    key(&mut e, b'y', 3, 60);
    capture(&e, "liuye-riddle-correct");
    for _ in 0..2 {
        key(&mut e, 27, 3, 60);
    }
    assert_eq!(e.vm().memory()[0x3eb3], 1);
    assert_eq!(e.vm().memory()[0x453d], 2);
    assert_eq!(position(&e), (38, 55));
    for k in [25, 22, 22, 22, 22, 13, 13, 13, 27, 27, 27] {
        key(&mut e, k, 3, 120);
    }
    let files = e.files_mut().take_dirty_files();
    assert!(
        files
            .iter()
            .any(|(name, bytes)| name == "/lavadata/liuye_save.dat" && bytes.len() == 2000)
    );
    let mut restored = start(&path, &files);
    assert_eq!(restored.vm().memory()[0x3eb3], 0);
    for k in [25, 22, 22, 22, 22, 13, 21, 13, 13, 27, 27, 27] {
        key(&mut restored, k, 3, 120);
    }
    assert_eq!(position(&restored), (38, 55));
    assert_eq!(restored.vm().memory()[0x3eb3], 1);
    assert_eq!(restored.vm().memory()[0x453d], 2);
    capture(&restored, "liuye-native-progress-restored");
    let mut battle = baseline;
    for k in [
        20, 20, 20, 20, 20, 20, 20, 20, 20, 20, 20, 20, 20, 20, 23, 23, 23, 23, 23, 23, 23, 23, 23,
        23, 23, 23, 23, 23, 23, 20, 20, 20, 20, 20, 23, 23, 23, 23, 23, 23, 23, 23, 23, 23, 23, 21,
        21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 22, 22, 22, 22, 21, 21, 21, 23, 23,
        23, 23, 23, 20,
    ] {
        key(&mut battle, k, 18, 60);
    }
    assert_eq!(position(&battle), (17, 21));
    capture(&battle, "liuye-tavern-dialogue");
    for _ in 0..2 {
        key(&mut battle, 27, 3, 120);
    }
    advance(&mut battle, 120);
    key(&mut battle, b'y', 18, 120);
    for _ in 0..4 {
        key(&mut battle, 27, 3, 120);
    }
    assert_eq!(battle.vm().pc(), 0x1192);
    assert_eq!(word(&battle, 0x4394), 120);
    capture(&battle, "liuye-battle-menu");
    for _ in 0..12 {
        key(&mut battle, 13, 3, 120);
    }
    assert!(word(&battle, 0x4394) < 120);
    assert!(word(&battle, 0x430e) < 150);
    capture(&battle, "liuye-battle-damage");
    for _ in 0..16 {
        key(&mut battle, 13, 3, 120);
    }
    assert_eq!(position(&battle), (17, 21));
    assert_eq!(battle.vm().memory()[0x2014], 1);
    assert_eq!(word(&battle, 0x4394), 58);
    assert_eq!(word(&battle, 0x430e), 13);
    capture(&battle, "liuye-duel-finished");
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn space_treasure_places_mirrors_clears_a_stage_retries_and_exits() {
    fn key(e: &mut Emulator, k: u8, frames: usize) {
        e.input_mut().release_all();
        for f in 0..frames {
            e.input_mut().set_keys((f == 0).then_some(k));
            advance(e, 1);
        }
    }
    let mut e = load(&fixture_path("太空寻宝.lav"));
    advance(&mut e, 1200);
    for _ in 0..40 {
        for f in 0..60 {
            e.input_mut().set_keys((f < 18).then_some(13));
            advance(&mut e, 1);
        }
    }
    e.input_mut().release_all();
    assert_eq!(&e.vm().memory()[0x347c..0x3481], &[1, 1, 3, 1, 0]);
    assert_eq!(&e.vm().memory()[0x3631..0x3637], &[5, 6, 6, 6, 5, 5]);
    capture(&e, "space-treasure-stage-1");
    let mut loss = e.clone();
    key(&mut loss, 22, 120);
    key(&mut loss, 22, 120);
    assert_eq!(loss.vm().pc(), 0x2a11);
    assert_eq!(loss.vm().memory()[0x347f], 0);
    capture(&loss, "space-treasure-ray-collision");
    key(&mut loss, b'y', 120);
    assert_eq!(&loss.vm().memory()[0x347c..0x3481], &[1, 1, 3, 1, 0]);
    capture(&loss, "space-treasure-retry");

    for (i, k) in [
        109, 14, 13, 23, 23, 23, 23, 23, 20, 20, 13, 117, 117, 117, 109, 21, 21, 21, 98, 23, 23,
        23, 98, 21, 109, 117, 14, 14, 13, 23, 23, 23, 23, 21, 21, 21, 21, 21, 13, 22, 22, 109, 109,
        22, 14, 13, 22, 21, 21, 21, 21, 13, 117, 117, 117, 20, 20, 14, 13, 22, 13, 116, 116, 116,
        117, 22, 22, 22, 109, 109, 14, 13, 22, 22, 22, 22, 22, 22, 22, 13, 21, 109, 109, 21, 14,
        13, 22, 22, 22, 22, 22, 22, 21, 21, 21, 21, 13, 98, 98, 23,
    ]
    .into_iter()
    .enumerate()
    {
        key(&mut e, k, 36);
        assert_eq!(e.vm().memory()[0x347f], 1);
        if [10, 38, 51, 60, 79, 96].contains(&i) {
            capture(&e, &format!("space-treasure-mirror-{}", i));
        }
    }
    assert_eq!(e.vm().pc(), 0x31e5);
    assert_eq!(&e.vm().memory()[0x347c..0x3481], &[2, 13, 9, 1, 0]);
    assert_eq!(&e.vm().memory()[0x3631..0x3637], &[0; 6]);
    capture(&e, "space-treasure-stage-cleared");
    key(&mut e, 13, 120);
    assert_eq!(e.vm().memory()[0x347c], 2);
    let board = e.vm().memory()[0x3481..0x3637].to_vec();
    let position = e.vm().memory()[0x347d..0x347f].to_vec();
    capture(&e, "space-treasure-stage-2");
    key(&mut e, 27, 120);
    assert_eq!(e.vm().pc(), 0x3080);
    key(&mut e, b'n', 120);
    assert_eq!(&e.vm().memory()[0x3481..0x3637], board.as_slice());
    assert_eq!(&e.vm().memory()[0x347d..0x347f], position.as_slice());
    for k in [27, b'y', 21, 13] {
        key(&mut e, k, 120);
    }
    assert_eq!(e.vm().pc(), 0x189a);
    capture(&e, "space-treasure-instructions");
    key(&mut e, 13, 120);
    key(&mut e, 27, 120);
    assert_eq!(e.vm().pc(), 0x1556);
    capture(&e, "space-treasure-farewell");
    e.input_mut().set_keys([13]);
    let mut exited = false;
    for _ in 0..120 {
        if let FrameStatus::Halted(code) = e.run_frame().unwrap().status {
            assert_eq!(code, 0);
            exited = true;
            break;
        }
    }
    assert!(exited);
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn space_battleship_paces_turns_moves_fires_pauses_and_finishes_a_battle() {
    fn key(e: &mut Emulator, k: u8, frames: usize) {
        e.input_mut().release_all();
        for f in 0..frames {
            e.input_mut().set_keys((f == 0).then_some(k));
            advance(e, 1);
        }
    }
    fn word(e: &Emulator, a: usize) -> i16 {
        i16::from_le_bytes(e.vm().memory()[a..a + 2].try_into().unwrap())
    }
    let path = fixture_path("太空战舰.lav");
    let mut e = load(&path);
    advance(&mut e, 1200);
    assert_eq!(e.vm().pc(), 0x5c0);
    for _ in 0..3 {
        key(&mut e, 13, 24);
    }
    key(&mut e, 25, 24);
    assert_eq!(e.vm().pc(), 0x5a3);
    capture(&e, "space-battleship-help");
    key(&mut e, 13, 1);
    for _ in 0..100 {
        if e.vm().pc() == 0x135b {
            break;
        }
        advance(&mut e, 1);
    }
    assert_eq!(e.vm().pc(), 0x135b);
    assert_eq!(word(&e, 0x2002), 5);
    capture(&e, "space-battleship-battle");
    let mut idle = e.clone();
    advance(&mut idle, 60);
    assert_eq!(word(&idle, 0x2002), 5);
    assert!(word(&idle, 0x205a) < 60);
    let mut paused = e.clone();
    key(&mut paused, b'p', 30);
    assert_eq!(paused.vm().pc(), 0x531);
    let state = paused.vm().memory()[0x2000..0x2138].to_vec();
    let frame = paused.display().indexed_frame().to_vec();
    advance(&mut paused, 300);
    assert_eq!(&paused.vm().memory()[0x2000..0x2138], state.as_slice());
    assert_eq!(paused.display().indexed_frame(), frame.as_slice());
    key(&mut paused, 13, 30);
    assert_ne!(paused.vm().pc(), 0x531);
    capture(&paused, "space-battleship-resumed");
    let mut finished = false;
    let mut shot_hit = false;
    for f in 0..1200 {
        let i = word(&e, 0x212c).clamp(0, 19) as usize;
        let x = word(&e, 0x2032 + i * 2);
        let y = word(&e, 0x205a + i * 2);
        let player = word(&e, 0x2004);
        let mut target = x + word(&e, 0x2130) * ((69 - y).max(0) / 2);
        while !(42..=112).contains(&target) {
            target = if target < 42 {
                84 - target
            } else {
                224 - target
            };
        }
        let k = if (target - player).abs() < 3 {
            b'a'
        } else if player < target {
            22
        } else {
            23
        };
        e.input_mut().set_keys((f % 6 < 5).then_some(k));
        let lives = word(&e, 0x2002);
        let enemies = word(&e, 0x2030);
        let result = e.run_frame().unwrap();
        assert!(!matches!(result.status, FrameStatus::Halted(_)));
        if word(&e, 0x2030) < enemies && word(&e, 0x2002) == lives {
            shot_hit = true;
            capture(&e, "space-battleship-enemy-shot");
        }
        if e.vm().pc() == 0xdfd {
            finished = true;
            break;
        }
    }
    assert!(finished);
    assert!(shot_hit);
    assert!(word(&e, 0x212c) >= 1);
    assert!(word(&e, 0x2030) < 20);
    assert_eq!(word(&e, 0x2002), 0);
    capture(&e, "space-battleship-native-loss");
    e.input_mut().set_keys([13]);
    assert_eq!(e.run_frame().unwrap().status, FrameStatus::Halted(1));
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn nine_stars_cycles_all_orientation_results_and_exits() {
    fn key(e: &mut Emulator, k: u8) {
        e.input_mut().release_all();
        for frame in 0..36 {
            e.input_mut().set_keys((frame == 0).then_some(k));
            advance(e, 1);
        }
    }
    fn word(e: &Emulator, a: usize) -> u16 {
        u16::from_le_bytes(e.vm().memory()[a..a + 2].try_into().unwrap())
    }
    let matrix = [
        [7, 2, 1, 6, 5, 3, 4, 0],
        [2, 7, 0, 4, 3, 6, 5, 1],
        [0, 1, 7, 5, 6, 3, 4, 2],
        [6, 4, 5, 7, 1, 2, 0, 3],
        [5, 3, 6, 1, 7, 0, 2, 4],
        [4, 6, 3, 0, 2, 7, 1, 5],
        [3, 5, 4, 0, 2, 1, 7, 6],
        [0, 1, 2, 3, 4, 5, 6, 7],
    ];
    let mut e = load(&fixture_path("9stars.lav"));
    advance(&mut e, 1200);
    assert_eq!(e.vm().pc(), 0x1a10);
    capture(&e, "nine-stars-title");
    key(&mut e, 13);
    assert_eq!(word(&e, 0x39b7), 4);
    capture(&e, "nine-stars-orientation");
    for _ in 0..4 {
        key(&mut e, 23);
    }
    assert_eq!(word(&e, 0x39b7), 0);
    let mut seen = [false; 8];
    for (orientation, row) in matrix.into_iter().enumerate() {
        assert_eq!(word(&e, 0x39b7), orientation as u16);
        key(&mut e, 13);
        assert_eq!(word(&e, 0x39b9), orientation as u16);
        for _ in 0..orientation {
            key(&mut e, 23);
        }
        for (direction, expected) in row.into_iter().enumerate() {
            assert_eq!(word(&e, 0x39b9), direction as u16);
            assert_eq!(
                word(&e, 0x3936 + (orientation * 8 + direction) * 2),
                expected
            );
            assert_eq!(e.vm().pc(), 0x1d34);
            if !seen[usize::from(expected)] {
                capture(&e, &format!("nine-stars-result-{expected}"));
                seen[usize::from(expected)] = true;
            }
            key(&mut e, 22);
        }
        assert_eq!(word(&e, 0x39b9), 0);
        key(&mut e, 23);
        assert_eq!(word(&e, 0x39b9), 7);
        key(&mut e, 27);
        assert_eq!(e.vm().pc(), 0x1a4d);
        if orientation < 7 {
            key(&mut e, 22);
        }
    }
    assert!(seen.into_iter().all(|value| value));
    key(&mut e, 22);
    assert_eq!(word(&e, 0x39b7), 0);
    key(&mut e, 23);
    assert_eq!(word(&e, 0x39b7), 7);
    e.input_mut().set_keys([27]);
    assert_eq!(e.run_frame().unwrap().status, FrameStatus::Halted(0));
}
#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn cosmos_moves_pauses_finishes_and_restores_native_records() {
    fn word(e: &Emulator, a: usize) -> i16 {
        i16::from_le_bytes(e.vm().memory()[a..a + 2].try_into().unwrap())
    }
    fn key(e: &mut Emulator, k: u8, frames: usize, held: usize) {
        e.input_mut().release_all();
        for f in 0..frames {
            e.input_mut().set_keys((f < held).then_some(k));
            advance(e, 1);
        }
    }
    let path = fixture_path("宇宙大战.lav");
    let mut e = load(&path);
    advance(&mut e, 1200);
    assert_eq!(e.vm().pc(), 0x1e28);
    let initial = e.files().file("/lavadata/ch.dat").unwrap().to_vec();
    assert_eq!(initial, vec![0; 10]);
    let mut reopened = load(&path);
    reopened
        .files_mut()
        .import_file("/lavadata/ch.dat", initial);
    advance(&mut reopened, 1200);
    assert_eq!(&reopened.vm().memory()[0x205a..0x2064], &[0; 10]);
    capture(&e, "cosmos-title");
    key(&mut e, 13, 1, 1);
    for difficulty in 1..=5 {
        assert_eq!(e.vm().memory()[0x2c01], difficulty);
        key(&mut e, 21, 24, 1);
    }
    assert_eq!(e.vm().memory()[0x2c01], 5);
    for _ in 0..5 {
        key(&mut e, 20, 24, 1);
    }
    assert_eq!(e.vm().memory()[0x2c01], 1);
    capture(&e, "cosmos-difficulty");
    key(&mut e, 13, 60, 1);
    capture(&e, "cosmos-battle");
    assert_eq!((word(&e, 0x2bfd), word(&e, 0x2bff)), (80, 40));
    for (direction, axis, change) in [
        (23, 0x2bfd, -1),
        (22, 0x2bfd, 1),
        (20, 0x2bff, -1),
        (21, 0x2bff, 1),
    ] {
        let mut moved = e.clone();
        key(&mut moved, direction, 60, 18);
        assert!((word(&moved, axis) - word(&e, axis)) * change > 0);
    }
    let mut paused = e.clone();
    key(&mut paused, b'p', 60, 18);
    assert_eq!(paused.vm().pc(), 0x1c81);
    let memory = paused.vm().memory().to_vec();
    let frame = paused.display().indexed_frame().to_vec();
    advance(&mut paused, 300);
    assert_eq!(paused.vm().memory(), memory);
    assert_eq!(paused.display().indexed_frame(), frame);
    capture(&paused, "cosmos-paused");
    key(&mut paused, 13, 36, 1);
    assert_ne!(paused.vm().pc(), 0x1c81);
    let mut help = e.clone();
    key(&mut help, b'h', 36, 18);
    assert_eq!(help.vm().pc(), 0xf9e);
    let first = help.display().indexed_frame().to_vec();
    key(&mut help, 14, 24, 1);
    assert_ne!(first, help.display().indexed_frame());
    capture(&help, "cosmos-help-scrolled");
    key(&mut help, 19, 24, 1);
    assert_eq!(first, help.display().indexed_frame());
    key(&mut help, 27, 24, 1);
    assert_ne!(help.vm().pc(), 0xf9e);
    key(&mut e, 23, 60, 18);
    key(&mut e, 20, 60, 18);
    capture(&e, "cosmos-moved");
    advance(&mut e, 1200);
    assert_eq!(word(&e, 0x2c02), -1);
    assert_eq!(e.vm().pc(), 0x1d27);
    let score = word(&e, 0x205a);
    assert!(score > 0, "evading meteors must earn a native score");
    capture(&e, "cosmos-finished");
    key(&mut e, 27, 24, 1);
    key(&mut e, 21, 24, 1);
    key(&mut e, 13, 24, 1);
    assert_eq!(e.vm().pc(), 0x1f0d);
    capture(&e, "cosmos-records");
    key(&mut e, 13, 24, 1);
    key(&mut e, 21, 24, 1);
    key(&mut e, 21, 24, 1);
    e.input_mut().set_keys([13]);
    assert_eq!(e.run_frame().unwrap().status, FrameStatus::Halted(0));
    let saved = e.files().file("/lavadata/ch.dat").unwrap().to_vec();
    assert_eq!(saved.len(), 10);
    assert_eq!(i16::from(saved[0]) + i16::from(saved[1]) * 200, score);
    let mut restored = load(&path);
    restored.files_mut().import_file("/lavadata/ch.dat", saved);
    advance(&mut restored, 1200);
    assert_eq!(word(&restored, 0x205a), score);
    key(&mut restored, 21, 24, 1);
    key(&mut restored, 13, 24, 1);
    assert_eq!(restored.vm().pc(), 0x1f0d);
    capture(&restored, "cosmos-records-restored");
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn pet_world_80_upgrades_a_pet_saves_and_restores_progress() {
    fn key(e: &mut Emulator, k: u8) {
        e.input_mut().release_all();
        for frame in 0..60 {
            e.input_mut().set_keys((frame < 3).then_some(k));
            advance(e, 1);
        }
    }
    fn potential(e: &Emulator) -> u16 {
        u16::from_le_bytes(e.vm().memory()[0x27db..0x27dd].try_into().unwrap())
    }
    fn max_hp(e: &Emulator) -> u32 {
        u32::from_le_bytes(e.vm().memory()[0x23b6..0x23ba].try_into().unwrap())
    }
    fn check_save(bytes: &[u8]) {
        assert_eq!(bytes.len(), 1072);
        assert_eq!(&bytes[..3], b"PET");
        assert_eq!(&bytes[10..12], &[0, 0]);
        assert_eq!(u32::from_le_bytes(bytes[17..21].try_into().unwrap()), 800);
    }
    let path = fixture_path("宠物世界80.lav");
    let mut e = load(&path);
    advance(&mut e, 300);
    assert_ne!(e.vm().pc(), 0x681e, "virtual storage must allow saving");
    assert_eq!(e.vm().memory()[0x27da], 1);
    assert_eq!((potential(&e), max_hp(&e)), (200, 200));
    capture(&e, "pet-world-80-main");
    key(&mut e, 21);
    assert_eq!(e.vm().pc(), 0x2423);
    capture(&e, "pet-world-80-information");
    key(&mut e, 27);
    key(&mut e, 20);
    assert_eq!(e.vm().pc(), 0x2191);
    capture(&e, "pet-world-80-pet-selection");
    key(&mut e, 13);
    for _ in 0..4 {
        key(&mut e, 22);
    }
    capture(&e, "pet-world-80-allocation-menu");
    key(&mut e, 13);
    key(&mut e, 13);
    capture(&e, "pet-world-80-allocation-amount");
    key(&mut e, 13);
    assert_eq!((potential(&e), max_hp(&e)), (0, 800));
    capture(&e, "pet-world-80-upgraded");
    key(&mut e, 13);
    key(&mut e, 27);
    let mut exiting = e.clone();
    key(&mut exiting, 27);
    assert_eq!(exiting.vm().pc(), 0x1551);
    capture(&exiting, "pet-world-80-exit-confirmation");
    exiting.input_mut().release_all();
    let mut halted = false;
    for frame in 0..180 {
        exiting.input_mut().set_keys((frame < 3).then_some(13));
        if exiting.run_frame().unwrap().status == FrameStatus::Halted(0) {
            halted = true;
            break;
        }
    }
    assert!(halted, "save-and-exit must terminate normally");
    check_save(exiting.files().file("/lavadata/宠世存档.sav").unwrap());
    key(&mut e, 23);
    for _ in 0..4 {
        key(&mut e, 22);
    }
    key(&mut e, 13);
    assert_eq!(e.vm().pc(), 0x145e);
    capture(&e, "pet-world-80-saved");
    let saved = e.files().file("/lavadata/宠世存档.sav").unwrap().to_vec();
    check_save(&saved);
    let mut restored = load(&path);
    restored
        .files_mut()
        .import_file("/lavadata/宠世存档.sav", saved);
    advance(&mut restored, 300);
    assert_eq!(restored.vm().memory()[0x27da], 1);
    assert_eq!((potential(&restored), max_hp(&restored)), (0, 800));
    capture(&restored, "pet-world-80-restored");
    key(&mut restored, 21);
    assert_eq!(restored.vm().pc(), 0x2423);
    capture(&restored, "pet-world-80-restored-information");
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn pet_picture_converter_restores_and_round_trips_native_images_without_growth() {
    fn key(e: &mut Emulator, k: u8) {
        e.input_mut().release_all();
        for frame in 0..24 {
            e.input_mut().set_keys((frame == 0).then_some(k));
            advance(e, 1);
        }
    }
    let path = fixture_path("宠世导出入.lav");
    let mut e = load(&path);
    advance(&mut e, 60);
    assert_eq!(e.vm().pc(), 0x18a);
    capture(&e, "pet-picture-converter-menu");
    key(&mut e, b'c');
    assert_eq!(e.vm().pc(), 0x3af);
    let original = e.files().file("/lavadata/宠世图片.dat").unwrap().to_vec();
    assert_eq!(original.len(), 8 * 32);
    assert!(original.iter().any(|&pixel| pixel != 0));
    capture(&e, "pet-picture-converter-original-restored");
    key(&mut e, 13);
    let mut exported = Vec::new();
    for _ in 0..3 {
        key(&mut e, b'a');
        assert_eq!(e.vm().pc(), 0x29d);
        let album = e.files().file("/album/宠世图片.abm").unwrap();
        assert_eq!(album.len(), 18 + original.len());
        assert_eq!(&album[..4], b"ABM\0");
        assert_eq!(&album[4..10], &[16, 0, 16, 0, 8, 0]);
        assert_eq!(u32::from_le_bytes(album[10..14].try_into().unwrap()), 18);
        assert_eq!(u32::from_le_bytes(album[14..18].try_into().unwrap()), 274);
        assert_eq!(&album[18..], original);
        exported = album.to_vec();
        key(&mut e, 13);
        key(&mut e, b'b');
        assert_eq!(e.vm().pc(), 0x356);
        assert_eq!(e.files().file("/lavadata/宠世图片.dat").unwrap(), original);
        key(&mut e, 13);
    }
    capture(&e, "pet-picture-converter-round-tripped");
    key(&mut e, b'd');
    assert_eq!(e.vm().pc(), 0x42a);
    assert_eq!(e.files().file("/album/宠世图片.abm").unwrap(), exported);
    capture(&e, "pet-picture-converter-album-restored");
    let mut fresh = load(&path);
    fresh
        .files_mut()
        .import_file("/album/宠世图片.abm", exported);
    advance(&mut fresh, 60);
    key(&mut fresh, b'b');
    assert_eq!(
        fresh.files().file("/lavadata/宠世图片.dat").unwrap(),
        original
    );
    capture(&fresh, "pet-picture-converter-fresh-import");
    key(&mut fresh, 13);
    fresh.input_mut().set_keys([27]);
    assert_eq!(fresh.run_frame().unwrap().status, FrameStatus::Halted(0));
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn pixel_album_edits_both_sizes_switches_frames_and_reopens_native_saves() {
    fn key(e: &mut Emulator, k: u8) {
        e.input_mut().release_all();
        for frame in 0..24 {
            e.input_mut().set_keys((frame == 0).then_some(k));
            advance(e, 1);
        }
    }
    fn assert_page(e: &Emulator, text: &[u8]) {
        let bp = e.vm().local_bp() as usize;
        assert_eq!(&e.vm().memory()[bp + 0x5f..bp + 0x5f + text.len()], text);
        let mut expected = e.display().clone();
        expected.draw_text(
            lavaxemu_core::BufferTarget::Front,
            90,
            58,
            text,
            false,
            lavaxemu_core::DrawOperation::Set,
        );
        assert_eq!(
            e.display()
                .capture(lavaxemu_core::BufferTarget::Front, 90, 58, 24, 16),
            expected.capture(lavaxemu_core::BufferTarget::Front, 90, 58, 24, 16)
        );
    }
    fn open(path: &str, guest: &str, album: &[u8]) -> Emulator {
        let mut e = load(path);
        e.files_mut().import_file(guest, album.to_vec());
        advance(&mut e, 60);
        key(&mut e, 21);
        key(&mut e, 13);
        assert_eq!(e.vm().pc(), 0x1677);
        key(&mut e, 13);
        assert_eq!(e.vm().pc(), 0x4340);
        e
    }
    fn exit(e: &mut Emulator) {
        key(e, 27);
        key(e, 21);
        key(e, 21);
        key(e, 13);
        assert_eq!(e.vm().pc(), 0x904);
        e.input_mut().set_keys([27]);
        assert_eq!(e.run_frame().unwrap().status, FrameStatus::Halted(0));
    }
    let path = fixture_path("象素画册.lav");
    let mut converter = load(&fixture_path("宠世导出入.lav"));
    advance(&mut converter, 60);
    key(&mut converter, b'd');
    let album = converter
        .files()
        .file("/album/宠世图片.abm")
        .unwrap()
        .to_vec();
    let mut expected = album[18..].to_vec();
    let mut e = open(&path, "/album/宠世图片.abm", &album);
    assert_page(&e, b"1/8");
    capture(&e, "pixel-album-original");
    for k in [13, b'd', 13, b's', 13] {
        key(&mut e, k);
    }
    expected[0] ^= 0xc0;
    expected[2] ^= 0x40;
    assert_eq!(&e.vm().memory()[0x5200..0x5220], &expected[..32]);
    capture(&e, "pixel-album-first-edited");
    key(&mut e, 21);
    assert_eq!(&e.vm().memory()[0x5200..0x5220], &expected[32..64]);
    key(&mut e, 13);
    expected[34] ^= 0x40;
    assert_page(&e, b"2/8");
    capture(&e, "pixel-album-second-edited");
    key(&mut e, 20);
    assert_eq!(&e.vm().memory()[0x2000..0x2100], expected);
    assert_eq!(&e.vm().memory()[0x5200..0x5220], &expected[..32]);
    key(&mut e, 27);
    key(&mut e, 13);
    let saved = e.files().file("/album/宠世图片.abm").unwrap().to_vec();
    assert_eq!(&saved[..18], &album[..18]);
    assert_eq!(&saved[18..], expected);
    capture(&e, "pixel-album-saved");
    exit(&mut e);
    let mut fresh = open(&path, "/album/宠世图片.abm", &saved);
    assert_eq!(&fresh.vm().memory()[0x2000..0x2100], expected);
    assert_eq!(&fresh.vm().memory()[0x5200..0x5220], &expected[..32]);
    key(&mut fresh, 21);
    assert_eq!(&fresh.vm().memory()[0x5200..0x5220], &expected[32..64]);
    capture(&fresh, "pixel-album-reopened-second");
    exit(&mut fresh);
    for size in [8_usize, 16] {
        let mut e = load(&path);
        advance(&mut e, 60);
        key(&mut e, 13);
        if size == 8 {
            key(&mut e, 21);
        }
        key(&mut e, 13);
        assert_eq!(e.vm().pc(), 0x4340);
        for k in [13, b'd', 13, b's', 13] {
            key(&mut e, k);
        }
        let mut pixels = vec![0; size * size / 8];
        pixels[0] = 0xc0;
        pixels[size / 8] = 0x40;
        key(&mut e, 27);
        key(&mut e, 13);
        assert_eq!(e.vm().pc(), 0x60d);
        key(&mut e, b'y');
        let name = format!("emu{size}");
        for k in name.bytes() {
            key(&mut e, k);
        }
        key(&mut e, 13);
        let guest = format!("/album/{name}.abm");
        let saved = e.files().file(&guest).unwrap().to_vec();
        assert_eq!(saved.len(), 18 + pixels.len());
        assert_eq!(&saved[..4], b"ABM\0");
        assert_eq!(
            u16::from_le_bytes(saved[4..6].try_into().unwrap()),
            size as u16
        );
        assert_eq!(
            u16::from_le_bytes(saved[6..8].try_into().unwrap()),
            size as u16
        );
        assert_eq!(&saved[8..10], &[1, 0]);
        assert_eq!(&saved[18..], pixels);
        capture(&e, &format!("pixel-album-new-{size}-saved"));
        exit(&mut e);
        let mut reopened = open(&path, &guest, &saved);
        assert_eq!(
            &reopened.vm().memory()[0x5200..0x5200 + pixels.len()],
            pixels
        );
        capture(&reopened, &format!("pixel-album-new-{size}-reopened"));
        exit(&mut reopened);
    }
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn locked_room_creates_an_account_moves_triggers_a_scene_and_restores_progress() {
    fn key(e: &mut Emulator, k: u8) {
        e.input_mut().release_all();
        for frame in 0..60 {
            e.input_mut().set_keys((frame < 3).then_some(k));
            advance(e, 1);
        }
    }
    fn account(e: &mut Emulator) {
        assert_eq!(e.vm().pc(), 0x1875);
        for k in [b'a', b'i', 13, 13, 13] {
            key(e, k);
        }
        assert_eq!(e.vm().pc(), 0x7e0f);
        key(e, b'a');
        key(e, 13);
    }
    fn position(e: &Emulator) -> (u8, u8) {
        (e.vm().memory()[0x453c], e.vm().memory()[0x453d])
    }
    fn map_ready(e: &Emulator, map: u8) -> bool {
        e.vm().memory()[0x2025] == map && (0xd559..=0xd56e).contains(&e.vm().pc())
    }
    fn acknowledge(e: &mut Emulator, map: u8) {
        for _ in 0..128 {
            if map_ready(e, map) {
                return;
            }
            key(e, 13);
        }
        panic!("the story must return to the requested map");
    }
    let path = fixture_path("密室杀机.lav");
    let mut e = load(&path);
    advance(&mut e, 1200);
    assert_eq!(e.vm().pc(), 0xa451);
    capture(&e, "locked-room-title");
    key(&mut e, 13);
    capture(&e, "locked-room-menu");
    key(&mut e, 13);
    account(&mut e);
    key(&mut e, b'a');
    key(&mut e, 13);
    capture(&e, "locked-room-opening");
    acknowledge(&mut e, 2);
    assert_eq!(position(&e), (15, 5));
    capture(&e, "locked-room-first-map");
    for (direction, axis, sign) in [
        (23, 0x453c, -1),
        (22, 0x453c, 1),
        (20, 0x453d, -1),
        (21, 0x453d, 1),
    ] {
        let moved = input_branch(&e, direction);
        assert!(
            (i16::from(moved.vm().memory()[axis]) - i16::from(e.vm().memory()[axis])) * sign > 0
        );
        assert_ne!(moved.display().indexed_frame(), e.display().indexed_frame());
    }
    for direction in [23, 23, 21, 21, 21, 23, 23, 21, 21, 21, 21, 21] {
        let before = position(&e);
        for _ in 0..60 {
            e.input_mut().set_keys([direction]);
            advance(&mut e, 1);
            if position(&e) != before {
                break;
            }
        }
        e.input_mut().release_all();
        advance(&mut e, 60);
        assert_ne!(position(&e), before);
    }
    assert_eq!(position(&e), (11, 13));
    assert_eq!(e.vm().pc(), 0x33a9);
    capture(&e, "locked-room-departure");
    acknowledge(&mut e, 3);
    assert_eq!(position(&e), (19, 7));
    capture(&e, "locked-room-next-scene");
    hold(&mut e, b'b', 18);
    assert_eq!(e.vm().pc(), 0x764b);
    capture(&e, "locked-room-status-menu");
    for _ in 0..4 {
        key(&mut e, 21);
    }
    key(&mut e, 13);
    assert_eq!(e.vm().pc(), 0x5808);
    capture(&e, "locked-room-system-menu");
    key(&mut e, 13);
    assert_eq!(e.vm().pc(), 0x764b);
    let saved = e.files().file("/lavadata/挨.dat").unwrap().to_vec();
    assert_eq!(saved.len(), 3891);
    assert_eq!(&saved[..4], b"mss\0");
    assert_eq!(&saved[17..19], &[19, 7]);
    assert_eq!(saved[42], 3);
    let progress = e.vm().memory()[0x65fe..0x69fe].to_vec();
    let mut restored = load(&path);
    restored
        .files_mut()
        .import_file("/lavadata/挨.dat", saved.clone());
    advance(&mut restored, 1200);
    key(&mut restored, 13);
    key(&mut restored, 21);
    key(&mut restored, 13);
    account(&mut restored);
    acknowledge(&mut restored, 3);
    assert_eq!(position(&restored), (19, 7));
    assert_eq!(&restored.vm().memory()[0x65fe..0x69fe], progress);
    assert_eq!(restored.files().file("/lavadata/挨.dat").unwrap(), saved);
    capture(&restored, "locked-room-restored");
    hold(&mut restored, b'b', 18);
    key(&mut restored, 13);
    capture(&restored, "locked-room-restored-party");
    key(&mut restored, 27);
    key(&mut restored, 27);
    if !map_ready(&restored, 3) {
        key(&mut restored, 27);
    }
    assert!(map_ready(&restored, 3));
    hold(&mut restored, b'q', 18);
    assert_eq!(restored.vm().pc(), 0xa609);
    key(&mut restored, b'y');
    assert_eq!(restored.vm().pc(), 0xa4dc);
    restored.input_mut().release_all();
    let mut halted = false;
    for frame in 0..120 {
        restored.input_mut().set_keys((frame < 3).then_some(27));
        if restored.run_frame().unwrap().status == FrameStatus::Halted(0) {
            halted = true;
            break;
        }
    }
    assert!(halted, "exiting the title menu must terminate normally");
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn rich_world_versions_move_take_turns_and_restore_native_saves() {
    fn key(e: &mut Emulator, k: u8, frames: usize) {
        e.input_mut().release_all();
        for f in 0..frames {
            e.input_mut().set_keys((f < 3).then_some(k));
            advance(e, 1);
        }
    }
    fn menu(e: &mut Emulator) {
        advance(e, 1200);
        assert_eq!(e.vm().pc(), 0x19d1);
        key(e, 13, 1200);
        assert_eq!(e.vm().pc(), 0x1b97);
        key(e, 13, 120);
        assert_eq!(e.vm().pc(), 0x1d28);
    }
    fn position(e: &Emulator, player: usize) -> (u8, u8) {
        let a = 0x334c + player * 51 + 48;
        (e.vm().memory()[a], e.vm().memory()[a + 1])
    }
    for (filename, label) in [("富甲天下.lav", "loose"), ("富甲天下.pac", "packaged")] {
        let path = fixture_path(filename);
        let mut e = load(&path);
        menu(&mut e);
        capture(&e, &format!("rich-world-{label}-menu"));
        key(&mut e, 13, 120);
        assert_eq!(e.vm().pc(), 0x35a3);
        capture(&e, &format!("rich-world-{label}-players"));
        key(&mut e, 13, 120);
        assert_eq!(e.vm().pc(), 0x91c3);
        assert_eq!(e.vm().memory()[0x30df], 1);
        capture(&e, &format!("rich-world-{label}-board"));
        for player in 0..2 {
            let before = position(&e, player);
            key(&mut e, 13, 120);
            capture(&e, &format!("rich-world-{label}-roll-{player}"));
            key(&mut e, 13, 180);
            capture(&e, &format!("rich-world-{label}-outcome-{player}"));
            for _ in 0..12 {
                match e.vm().pc() {
                    0x397c | 0x3ab1 => key(&mut e, 27, 180),
                    0x3dbb => key(&mut e, b'n', 180),
                    _ => break,
                }
            }
            assert_eq!(e.vm().pc(), 0x91c3);
            assert_eq!(e.vm().memory()[0x30df], player as u8 + 2);
            assert_ne!(position(&e, player), before);
        }
        capture(&e, &format!("rich-world-{label}-after-turns"));
        for _ in 0..3 {
            key(&mut e, 21, 120);
        }
        key(&mut e, 13, 120);
        capture(&e, &format!("rich-world-{label}-system"));
        key(&mut e, 21, 120);
        key(&mut e, 13, 120);
        key(&mut e, 13, 120);
        assert_eq!(e.vm().pc(), 0x17d8);
        capture(&e, &format!("rich-world-{label}-saved"));
        let saved = e.files().file("/lavadata/rich01.dat").unwrap().to_vec();
        assert_eq!(saved.len(), 3474);
        let players = e.vm().memory()[0x334c..0x334c + 153].to_vec();
        key(&mut e, 13, 120);
        let mut restored = load(&path);
        restored
            .files_mut()
            .import_file("/lavadata/rich01.dat", saved.clone());
        menu(&mut restored);
        key(&mut restored, 21, 120);
        key(&mut restored, 13, 120);
        key(&mut restored, 13, 120);
        assert_eq!(restored.vm().pc(), 0x19b7);
        key(&mut restored, 13, 120);
        assert_eq!(restored.vm().pc(), 0x91c3);
        assert_eq!(restored.vm().memory()[0x30df], 3);
        assert_eq!(&restored.vm().memory()[0x334c..0x334c + 153], players);
        assert_eq!(
            restored.files().file("/lavadata/rich01.dat").unwrap(),
            saved
        );
        capture(&restored, &format!("rich-world-{label}-restored"));
        for _ in 0..3 {
            key(&mut restored, 21, 120);
        }
        key(&mut restored, 13, 120);
        key(&mut restored, 21, 120);
        key(&mut restored, 21, 120);
        restored.input_mut().set_keys([13]);
        let mut halted = false;
        for _ in 0..120 {
            if restored.run_frame().unwrap().status == FrameStatus::Halted(0) {
                halted = true;
                break;
            }
            restored.input_mut().set_keys([]);
        }
        assert!(halted);
    }
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn battle_blocks_versions_control_both_boards_clear_a_line_pause_and_finish() {
    fn run(e: &mut Emulator, keys: &[u8], frames: usize) {
        for _ in 0..frames {
            e.input_mut().set_keys(keys.iter().copied());
            advance(e, 1);
        }
    }
    fn key(e: &mut Emulator, k: u8) {
        e.input_mut().release_all();
        run(e, &[k], 3);
        run(e, &[], 57);
    }
    fn score(e: &Emulator) -> u16 {
        u16::from_le_bytes(e.vm().memory()[0x2523..0x2525].try_into().unwrap())
    }
    let device: &[(usize, u8)] = &[
        (2, 8),
        (0, 7),
        (3, 7),
        (0, 7),
        (0, 8),
        (0, 6),
        (1, 8),
        (0, 8),
        (0, 8),
        (0, 8),
        (0, 5),
        (2, 7),
        (0, 8),
        (0, 7),
        (0, 4),
        (0, 4),
        (2, 5),
        (0, 2),
        (0, 2),
        (2, 5),
        (0, 2),
        (0, 3),
        (2, 5),
        (0, 2),
        (0, 0),
    ];
    let computer: &[(usize, u8)] = &[
        (0, 2),
        (0, 3),
        (0, 1),
        (0, 1),
        (0, 2),
        (0, 3),
        (2, 2),
        (0, 3),
        (0, 3),
        (1, 3),
        (0, 0),
        (0, 2),
        (0, 5),
        (1, 8),
    ];
    for (filename, label, recipe) in [
        ("对战方块.lav", "device", device),
        ("For Computer（模拟器用）.lav", "computer", computer),
    ] {
        let mut e = load(&fixture_path(filename));
        advance(&mut e, 1200);
        assert_eq!(e.vm().pc(), 0x3a5);
        key(&mut e, 13);
        assert_eq!(e.vm().pc(), 0x5d8);
        key(&mut e, 13);
        assert_eq!(e.vm().pc(), 0x8ff);
        capture(&e, &format!("battle-blocks-{label}-controls"));
        key(&mut e, b'y');
        if label == "device" {
            advance(&mut e, 60);
        }
        capture(&e, &format!("battle-blocks-{label}-board"));
        let mut controls = e.clone();
        for _ in 0..1200 {
            if controls.vm().memory()[0x251d] < 16 && controls.vm().memory()[0x251f] < 16 {
                break;
            }
            advance(&mut controls, 1);
        }
        let mut moved = controls.clone();
        run(&mut moved, &[b'a', 22], 18);
        run(&mut moved, &[], 42);
        assert!(moved.vm().memory()[0x251c] < e.vm().memory()[0x251c]);
        assert!(moved.vm().memory()[0x251e] > e.vm().memory()[0x251e]);
        capture(&moved, &format!("battle-blocks-{label}-both-moved"));
        let mut rotated = controls.clone();
        let before = rotated.vm().memory()[0x2518];
        for _ in 0..240 {
            run(&mut rotated, b"w", 1);
            if rotated.vm().memory()[0x2518] != before {
                break;
            }
        }
        rotated.input_mut().release_all();
        assert_ne!(rotated.vm().memory()[0x2518], before);
        let mut right = controls.clone();
        let mut did_rotate = false;
        for _ in 0..6 {
            right = input_branch(&right, 21);
            for _ in 0..1200 {
                if right.vm().memory()[0x251f] < 16 {
                    break;
                }
                advance(&mut right, 1);
            }
            let before = right.vm().memory()[0x2519];
            for _ in 0..240 {
                run(&mut right, &[20], 1);
                if right.vm().memory()[0x2519] != before {
                    break;
                }
            }
            right.input_mut().release_all();
            if right.vm().memory()[0x2519] != before {
                did_rotate = true;
                break;
            }
        }
        assert!(did_rotate, "a visible non-square piece must rotate");
        capture(&right, &format!("battle-blocks-{label}-right-rotated"));
        let mut paused = input_branch(&e, b'p');
        let pause_pc = if label == "device" { 0x4174 } else { 0x4177 };
        assert_eq!(paused.vm().pc(), pause_pc);
        let memory = paused.vm().memory().to_vec();
        let frame = paused.display().indexed_frame().to_vec();
        advance(&mut paused, 300);
        assert_eq!(paused.vm().memory(), memory);
        assert_eq!(paused.display().indexed_frame(), frame);
        capture(&paused, &format!("battle-blocks-{label}-paused"));
        key(&mut paused, 13);
        assert_ne!(paused.vm().pc(), pause_pc);
        assert_eq!(score(&e), 0);
        for &(rotations, target) in recipe {
            for _ in 0..rotations {
                run(&mut e, b"w", 8);
                run(&mut e, &[], 8);
            }
            for _ in 0..100 {
                let x = e.vm().memory()[0x251c];
                if x == target {
                    break;
                }
                run(&mut e, &[if x < target { b'd' } else { b'a' }], 1);
            }
            run(&mut e, &[], 2);
            assert_eq!(e.vm().memory()[0x251c], target);
            let board = e.vm().memory()[0x2204..0x22d6].to_vec();
            let mut locked = false;
            for _ in 0..180 {
                run(&mut e, b"s", 1);
                if e.vm().memory()[0x2204..0x22d6] != board {
                    locked = true;
                    break;
                }
            }
            e.input_mut().release_all();
            assert!(locked, "the piece must lock through actual down input");
            assert_eq!(&e.vm().memory()[0x2512..0x2514], &[0, 0]);
        }
        assert_eq!(score(&e), 1, "the native row clear must award a point");
        assert!(
            e.vm().memory()[0x252b..0x2540]
                .iter()
                .all(|&count| count < 10)
        );
        capture(&e, &format!("battle-blocks-{label}-line-cleared"));
        let mut lost = false;
        for _ in 0..12000 {
            if e.run_frame().unwrap().status == FrameStatus::WaitingForInput {
                lost = true;
                break;
            }
        }
        assert!(lost);
        assert!(e.vm().memory()[0x2512..0x2514].contains(&255));
        capture(&e, &format!("battle-blocks-{label}-loss"));
        key(&mut e, 13);
        key(&mut e, 13);
        capture(&e, &format!("battle-blocks-{label}-result"));
        e.input_mut().set_keys(*b"n");
        assert_eq!(e.run_frame().unwrap().status, FrameStatus::Halted(0));
    }
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn monkey_river_places_roads_retries_and_completes_a_crossing() {
    fn word(e: &Emulator, a: usize) -> u16 {
        u16::from_le_bytes(e.vm().memory()[a..a + 2].try_into().unwrap())
    }
    fn key(e: &mut Emulator, k: u8) {
        e.input_mut().release_all();
        for frame in 0..60 {
            e.input_mut().set_keys((frame < 3).then_some(k));
            advance(e, 1);
        }
    }
    fn place(e: &mut Emulator, x: u16, y: u16, k: u8) {
        while word(e, 0x2176) != x {
            key(e, if word(e, 0x2176) < x { 22 } else { 23 });
        }
        while word(e, 0x2178) != y {
            key(e, if word(e, 0x2178) < y { 21 } else { 20 });
        }
        let before = word(e, 0x217e);
        key(e, k);
        assert_eq!(word(e, 0x217e), before - 1);
    }
    fn finish(e: &mut Emulator, expected: u16) {
        let mut waiting = false;
        for _ in 0..12000 {
            if e.run_frame().unwrap().status == FrameStatus::WaitingForInput {
                waiting = true;
                break;
            }
        }
        assert!(waiting);
        assert_eq!(e.vm().pc(), 0x18f3);
        assert_eq!(word(e, 0x2180), expected);
    }
    fn connected(e: &Emulator) -> bool {
        let mut queue = vec![(1_usize, 2_usize)];
        let mut seen = [[false; 13]; 13];
        seen[1][2] = true;
        let mut cursor = 0;
        while cursor < queue.len() {
            let (x, y) = queue[cursor];
            cursor += 1;
            if word(e, 0x2000 + 2 * (x * 13 + y)) != 1 {
                continue;
            }
            if (x, y) == (12, 11) {
                return true;
            }
            for (a, b) in [
                (x.wrapping_sub(1), y),
                (x + 1, y),
                (x, y.wrapping_sub(1)),
                (x, y + 1),
            ] {
                if a < 13 && b < 13 && !seen[a][b] {
                    seen[a][b] = true;
                    queue.push((a, b));
                }
            }
        }
        false
    }
    let mut e = load(&fixture_path("小猴过河.lav"));
    advance(&mut e, 1200);
    assert_eq!(e.vm().pc(), 0x1ba3);
    capture(&e, "monkey-river-title");
    let mut help = e.clone();
    key(&mut help, 21);
    key(&mut help, 13);
    advance(&mut help, 1200);
    for page in 1..=4 {
        assert_eq!(word(&help, 0x2172), page);
        capture(&help, &format!("monkey-river-help-{page}"));
        key(&mut help, 21);
    }
    assert_eq!(word(&help, 0x2172), 4);
    key(&mut help, 20);
    assert_eq!(word(&help, 0x2172), 3);
    key(&mut help, 27);
    key(&mut e, 13);
    key(&mut e, 13);
    key(&mut e, 13);
    assert_eq!(e.vm().pc(), 0x1741);
    assert_eq!(word(&e, 0x217e), 8);
    key(&mut e, 20);
    key(&mut e, 23);
    assert_eq!((word(&e, 0x2176), word(&e, 0x2178)), (1, 1));
    capture(&e, "monkey-river-initial-board");
    for round in 0..4 {
        for piece in 0..8 {
            place(&mut e, piece % 4 + 1, piece / 4 + 1, b'q');
            if round == 0 && piece == 0 {
                let board = e.vm().memory()[0x2000..0x2152].to_vec();
                key(&mut e, b'q');
                assert_eq!(word(&e, 0x217e), 7);
                assert_eq!(&e.vm().memory()[0x2000..0x2152], board);
                capture(&e, "monkey-river-overlap-rejected");
            }
        }
        finish(&mut e, 2);
        assert!(!connected(&e));
        if round == 0 {
            capture(&e, "monkey-river-failed");
        }
        key(&mut e, 13);
        if round == 0 {
            capture(&e, "monkey-river-retry-menu");
        }
        key(&mut e, 13);
        assert_eq!(word(&e, 0x217e), 8);
    }
    for (piece, (x, y)) in [
        (1, 1),
        (2, 1),
        (3, 2),
        (3, 3),
        (4, 3),
        (4, 1),
        (2, 2),
        (4, 4),
    ]
    .into_iter()
    .enumerate()
    {
        place(&mut e, x, y, b"qaz"[piece % 3]);
    }
    finish(&mut e, 1);
    assert!(connected(&e));
    capture(&e, "monkey-river-crossed");
    key(&mut e, 13);
    key(&mut e, 21);
    key(&mut e, 13);
    assert_eq!(e.vm().pc(), 0x1a86);
    capture(&e, "monkey-river-credits");
    key(&mut e, 13);
    assert_eq!(e.vm().pc(), 0x1b03);
    e.input_mut().set_keys([13]);
    let mut halted = false;
    for _ in 0..600 {
        if e.run_frame().unwrap().status == FrameStatus::Halted(0) {
            halted = true;
            break;
        }
        e.input_mut().set_keys([]);
    }
    assert!(halted);
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn mountain_go_captures_undoes_redoes_and_restores_a_native_record() {
    fn key(e: &mut Emulator, k: u8) {
        e.input_mut().release_all();
        for frame in 0..60 {
            e.input_mut().set_keys((frame < 3).then_some(k));
            advance(e, 1);
        }
    }
    fn stone(e: &Emulator, x: usize, y: usize) -> u8 {
        e.vm().memory()[0x2094 + x * 19 + y]
    }
    fn top_left(e: &mut Emulator) {
        for _ in 0..3 {
            key(e, 23);
        }
        for _ in 0..16 {
            key(e, 20);
        }
        assert_eq!(e.vm().memory()[0x2088], 0);
        assert_eq!(e.vm().memory()[0x2086] + e.vm().memory()[0x2089], 0);
    }
    let path = fixture_path("山尘围棋v2.lav");
    let mut e = load(&path);
    advance(&mut e, 1200);
    capture(&e, "mountain-go-title");
    key(&mut e, 13);
    assert!(e.vm().memory()[0x2094..0x21fd].iter().all(|&v| v == 0));
    top_left(&mut e);
    key(&mut e, 21);
    key(&mut e, 13);
    assert_eq!(stone(&e, 0, 1), 2);
    key(&mut e, 20);
    key(&mut e, 13);
    assert_eq!(stone(&e, 0, 0), 1);
    capture(&e, "mountain-go-before-capture");
    key(&mut e, 22);
    key(&mut e, 13);
    assert_eq!(
        (stone(&e, 0, 0), stone(&e, 0, 1), stone(&e, 1, 0)),
        (0, 2, 2)
    );
    let board = e.vm().memory()[0x2094..0x21fd].to_vec();
    let history = e.vm().memory()[0x2080..0x2086].to_vec();
    capture(&e, "mountain-go-captured");
    key(&mut e, b'i');
    assert_eq!(
        (stone(&e, 0, 0), stone(&e, 0, 1), stone(&e, 1, 0)),
        (1, 2, 0)
    );
    capture(&e, "mountain-go-undone");
    key(&mut e, b'o');
    assert_eq!(&e.vm().memory()[0x2094..0x21fd], board);
    capture(&e, "mountain-go-redone");
    for k in [29, b'y', b'e', b'm', b'u', 13] {
        key(&mut e, k);
    }
    let record = e
        .files()
        .file("/LavaData/EMU.SGO")
        .expect("native save")
        .to_vec();
    assert!(record.starts_with(b"[SCGO_SGO]"));
    capture(&e, "mountain-go-saved");
    let mut restored = load(&path);
    restored
        .files_mut()
        .import_file("/LavaData/EMU.SGO", record);
    advance(&mut restored, 1200);
    for k in [13, 30, b'y', b'e', b'm', b'u', 13] {
        key(&mut restored, k);
    }
    assert_eq!(&restored.vm().memory()[0x2094..0x21fd], board);
    assert_eq!(&restored.vm().memory()[0x2080..0x2086], history);
    top_left(&mut restored);
    capture(&restored, "mountain-go-restored");
    key(&mut restored, 22);
    key(&mut restored, 21);
    key(&mut restored, 13);
    assert_eq!(stone(&restored, 1, 1), 1);
    capture(&restored, "mountain-go-continued");
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn mountain_go_219_versions_capture_and_restore_through_the_file_list() {
    fn key(e: &mut Emulator, k: u8) {
        e.input_mut().release_all();
        for frame in 0..60 {
            e.input_mut().set_keys((frame < 3).then_some(k));
            advance(e, 1);
        }
    }
    fn stone(e: &Emulator, offset: usize, x: usize, y: usize) -> u8 {
        e.vm().memory()[0x2098 + offset + x * 19 + y]
    }
    fn top_left(e: &mut Emulator, offset: usize) {
        for _ in 0..3 {
            key(e, 23);
        }
        for _ in 0..15 {
            key(e, 20);
        }
        assert_eq!(e.vm().memory()[0x208c + offset], 0);
        assert_eq!(
            e.vm().memory()[0x2088 + offset] + e.vm().memory()[0x208d + offset],
            0
        );
    }
    for (version, filename, offset) in [
        ("mono", "SCGO_219.lav", 0),
        ("gray", "SCGO_219_16gray.lav", 0x130),
    ] {
        let path = fixture_path(filename);
        let mut e = load(&path);
        advance(&mut e, 1200);
        capture(&e, &format!("mountain-go-219-{version}-title"));
        key(&mut e, 13);
        assert!(
            e.vm().memory()[0x2098 + offset..0x2201 + offset]
                .iter()
                .all(|&v| v == 0)
        );
        top_left(&mut e, offset);
        key(&mut e, 21);
        key(&mut e, 13);
        assert_eq!(stone(&e, offset, 0, 1), 2);
        key(&mut e, 20);
        key(&mut e, 13);
        assert_eq!(stone(&e, offset, 0, 0), 1);
        capture(&e, &format!("mountain-go-219-{version}-before-capture"));
        key(&mut e, 22);
        key(&mut e, 13);
        assert_eq!(
            (
                stone(&e, offset, 0, 0),
                stone(&e, offset, 0, 1),
                stone(&e, offset, 1, 0)
            ),
            (0, 2, 2)
        );
        let board = e.vm().memory()[0x2098 + offset..0x2201 + offset].to_vec();
        let history = e.vm().memory()[0x2080 + offset..0x2086 + offset].to_vec();
        capture(&e, &format!("mountain-go-219-{version}-captured"));
        key(&mut e, b'i');
        assert_eq!(
            (
                stone(&e, offset, 0, 0),
                stone(&e, offset, 0, 1),
                stone(&e, offset, 1, 0)
            ),
            (1, 2, 0)
        );
        capture(&e, &format!("mountain-go-219-{version}-undone"));
        key(&mut e, b'o');
        assert_eq!(&e.vm().memory()[0x2098 + offset..0x2201 + offset], board);
        capture(&e, &format!("mountain-go-219-{version}-redone"));
        for k in [29, b'y', b'e', b'm', b'u', 13] {
            key(&mut e, k);
        }
        let record = e
            .files()
            .file("/LavaData/SCGO/EMU.SGO")
            .expect("native save")
            .to_vec();
        assert!(record.starts_with(b"[SCGO_SGO]"));
        capture(&e, &format!("mountain-go-219-{version}-saved"));
        let mut restored = load(&path);
        restored
            .files_mut()
            .import_file("/LavaData/SCGO/EMU.SGO", record);
        advance(&mut restored, 1200);
        for k in [13, 30, b'y'] {
            key(&mut restored, k);
        }
        let listed = restored.display().indexed_frame().to_vec();
        capture(&restored, &format!("mountain-go-219-{version}-file-list"));
        key(&mut restored, 21);
        assert_ne!(listed, restored.display().indexed_frame());
        key(&mut restored, 20);
        key(&mut restored, 13);
        assert_eq!(
            &restored.vm().memory()[0x2098 + offset..0x2201 + offset],
            board
        );
        assert_eq!(
            &restored.vm().memory()[0x2080 + offset..0x2086 + offset],
            history
        );
        top_left(&mut restored, offset);
        capture(&restored, &format!("mountain-go-219-{version}-restored"));
        key(&mut restored, 22);
        key(&mut restored, 21);
        key(&mut restored, 13);
        assert_eq!(stone(&restored, offset, 1, 1), 1);
        capture(&restored, &format!("mountain-go-219-{version}-continued"));
    }
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn mountain_go_converter_preserves_a_legacy_record_and_reloads_its_output() {
    fn key(e: &mut Emulator, k: u8) {
        e.input_mut().release_all();
        for frame in 0..60 {
            e.input_mut().set_keys((frame < 3).then_some(k));
            advance(e, 1);
        }
    }
    fn record_name(e: &mut Emulator) {
        for k in [b'e', b'm', b'u', 13] {
            key(e, k);
        }
    }
    let path = fixture_path("山尘围棋v2.lav");
    let mut game = load(&path);
    advance(&mut game, 1200);
    key(&mut game, 13);
    for _ in 0..3 {
        key(&mut game, 23);
    }
    for _ in 0..16 {
        key(&mut game, 20);
    }
    for k in [21, 13, 20, 13, 22, 13, 29, b'y'] {
        key(&mut game, k);
    }
    record_name(&mut game);
    let native = game.files().file("/LavaData/EMU.SGO").unwrap().to_vec();
    assert_eq!(native.len(), 107);
    let board = game.vm().memory()[0x2094..0x21fd].to_vec();
    let history = game.vm().memory()[0x2080..0x2086].to_vec();
    // Encode the genuine short game using the legacy headerless byte counters.
    let mut legacy = native[10..84].to_vec();
    legacy.extend([native[87], native[85], native[89], native[84]]);
    legacy.extend_from_slice(&native[91..]);
    let mut converter = load(&fixture_path("存档转化器_山尘围棋.lav"));
    converter
        .files_mut()
        .import_file("/LavaData/EMU.GOS", legacy.clone());
    advance(&mut converter, 1200);
    capture(&converter, "mountain-go-converter-title");
    for k in [13, 30, b'y'] {
        key(&mut converter, k);
    }
    capture(&converter, "mountain-go-converter-old-filename");
    record_name(&mut converter);
    advance(&mut converter, 300);
    assert_eq!(&converter.vm().memory()[0x2094..0x21fd], board);
    assert_eq!(&converter.vm().memory()[0x2080..0x2086], history);
    for _ in 0..9 {
        key(&mut converter, 20);
    }
    capture(&converter, "mountain-go-converter-loaded-board");
    for k in [29, b'y'] {
        key(&mut converter, k);
    }
    capture(&converter, "mountain-go-converter-new-filename");
    record_name(&mut converter);
    advance(&mut converter, 300);
    let converted = converter.files().file("/LavaData/EMU.GO").unwrap().to_vec();
    assert_eq!(converted, native);
    assert_eq!(
        converter.files().file("/LavaData/EMU.GOS"),
        Some(legacy.as_slice())
    );
    capture(&converter, "mountain-go-converter-saved-board");
    let mut restored = load(&path);
    restored
        .files_mut()
        .import_file("/LavaData/EMU.SGO", converted);
    advance(&mut restored, 1200);
    for k in [13, 30, b'y'] {
        key(&mut restored, k);
    }
    record_name(&mut restored);
    assert_eq!(&restored.vm().memory()[0x2094..0x21fd], board);
    assert_eq!(&restored.vm().memory()[0x2080..0x2086], history);
    for _ in 0..16 {
        key(&mut restored, 20);
    }
    for _ in 0..3 {
        key(&mut restored, 23);
    }
    capture(&restored, "mountain-go-converter-output-reloaded");
    for k in [22, 21, 13] {
        key(&mut restored, k);
    }
    assert_eq!(restored.vm().memory()[0x2094 + 19 + 1], 1);
    capture(&restored, "mountain-go-converter-output-continued");
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn village_gold_versions_explore_and_restore_a_native_diary() {
    fn key(e: &mut Emulator, k: u8, frames: usize, held: usize) {
        e.input_mut().release_all();
        for frame in 0..frames {
            e.input_mut().set_keys((frame < held).then_some(k));
            advance(e, 1);
        }
    }
    fn title(e: &mut Emulator) {
        advance(e, 1200);
        for _ in 0..3 {
            key(e, 13, 120, 3);
        }
        assert_eq!(e.vm().pc(), 0xf078);
    }
    fn position(e: &Emulator) -> (u16, u16) {
        let m = e.vm().memory();
        (
            u16::from(m[0x37bc]) + u16::from(m[0x37be]),
            u16::from(m[0x37bd]) + u16::from(m[0x37bf]),
        )
    }
    for (version, filename) in [("loose", "山村淘金记.lav"), ("packaged", "gold.pac")] {
        let path = fixture_path(filename);
        let mut e = load(&path);
        title(&mut e);
        capture(&e, &format!("village-gold-{version}-title"));
        key(&mut e, 13, 120, 3);
        advance(&mut e, 3600);
        assert_eq!(e.vm().pc(), 0xf193);
        for k in [b'e', b'm', b'u', 13] {
            key(&mut e, k, 120, 3);
        }
        capture(&e, &format!("village-gold-{version}-welcome"));
        for _ in 0..30 {
            if e.vm().pc() == 0x10311 {
                break;
            }
            key(&mut e, 13, 240, 18);
        }
        assert_eq!(e.vm().pc(), 0x10311);
        assert_eq!(position(&e), (12, 15));
        assert_eq!(&e.vm().memory()[0x379a..0x379e], b"emu\0");
        capture(&e, &format!("village-gold-{version}-map"));
        key(&mut e, 22, 120, 18);
        assert_eq!(position(&e), (13, 15));
        key(&mut e, 21, 120, 18);
        assert_eq!(
            position(&e),
            (13, 15),
            "the southern obstacle blocks movement"
        );
        key(&mut e, 22, 120, 18);
        assert_eq!(position(&e), (14, 15));
        capture(&e, &format!("village-gold-{version}-moved"));
        key(&mut e, 28, 120, 18);
        capture(&e, &format!("village-gold-{version}-status"));
        for _ in 0..4 {
            key(&mut e, 14, 120, 3);
        }
        capture(&e, &format!("village-gold-{version}-system"));
        key(&mut e, 13, 120, 3);
        assert_eq!(e.vm().pc(), 0x2471);
        let saved = e.files().file("/LavaData/goldsave0.dat").unwrap().to_vec();
        assert_eq!(saved.len(), 414);
        let saved_position = position(&e);
        let money = e.vm().memory()[0x37c7..0x37cb].to_vec();
        capture(&e, &format!("village-gold-{version}-saved"));
        let mut restored = load(&path);
        restored
            .files_mut()
            .import_file("/LavaData/goldsave0.dat", saved.clone());
        title(&mut restored);
        for k in [22, 13, 13] {
            key(&mut restored, k, 120, 3);
        }
        assert_eq!(restored.vm().pc(), 0x3251);
        capture(
            &restored,
            &format!("village-gold-{version}-load-confirmation"),
        );
        key(&mut restored, 13, 120, 3);
        assert_eq!(position(&restored), saved_position);
        assert_eq!(&restored.vm().memory()[0x379a..0x379e], b"emu\0");
        assert_eq!(&restored.vm().memory()[0x37c7..0x37cb], money);
        assert_eq!(
            restored.files().file("/LavaData/goldsave0.dat"),
            Some(saved.as_slice())
        );
        capture(&restored, &format!("village-gold-{version}-restored"));
        key(&mut restored, 23, 120, 18);
        assert_eq!(position(&restored), (13, 15));
        capture(&restored, &format!("village-gold-{version}-continued"));
    }
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn empire_versions_preserve_timed_construction_costs_and_pending_saves() {
    fn key(e: &mut Emulator, k: u8) {
        e.input_mut().release_all();
        for frame in 0..120 {
            e.input_mut().set_keys((frame < 3).then_some(k));
            advance(e, 1);
        }
    }
    fn number(e: &Emulator, address: usize) -> i32 {
        i32::from_le_bytes(e.vm().memory()[address..address + 4].try_into().unwrap())
    }
    for (version, size, main_pc, message_pc) in [
        ("mono", 35_213, 0x8445, 0x20a9),
        ("gray", 35_313, 0x849f, 0x20f8),
    ] {
        let path = fixture_path_with_size("帝国传.lav", Some(size));
        let mut e = load(&path);
        advance(&mut e, 1200);
        capture(&e, &format!("empire-{version}-fortune"));
        key(&mut e, 13);
        assert_eq!(e.vm().pc(), main_pc);
        capture(&e, &format!("empire-{version}-menu"));
        key(&mut e, 13);
        key(&mut e, 13);
        capture(&e, &format!("empire-{version}-wood-cost"));
        assert_eq!(e.vm().memory()[0x211d], 0);
        let wood = number(&e, 0x2000);
        let iron = number(&e, 0x2008);
        let wood_cost = u16::from_le_bytes(e.vm().memory()[0x2122..0x2124].try_into().unwrap());
        let iron_cost = u16::from_le_bytes(e.vm().memory()[0x2126..0x2128].try_into().unwrap());
        key(&mut e, b'u');
        assert_eq!(number(&e, 0x2000), wood - i32::from(wood_cost));
        assert_eq!(number(&e, 0x2008), iron - i32::from(iron_cost));
        assert_eq!(e.vm().memory()[0x2050], 1);
        assert_eq!(e.vm().memory()[0x211d], 0);
        capture(&e, &format!("empire-{version}-upgrade-started"));
        key(&mut e, 13);
        advance(&mut e, 600);
        assert_eq!(e.vm().memory()[0x2050], 1);
        assert_eq!(
            e.vm().memory()[0x211d],
            0,
            "75-second work must not finish immediately"
        );
        let paid = (number(&e, 0x2000), number(&e, 0x2008));
        assert_eq!(
            paid,
            (wood - i32::from(wood_cost), iron - i32::from(iron_cost))
        );
        for k in [21, 21, 21, 21, 21, 13, 21, 21, 13] {
            key(&mut e, k);
        }
        assert_eq!(e.vm().pc(), message_pc);
        let saved = e.files().file("/LavaData/dgz.sav").unwrap().to_vec();
        assert_eq!(saved.len(), 1433);
        assert_eq!(saved[0x50], 1);
        assert_eq!(saved[0x11d], 0);
        capture(&e, &format!("empire-{version}-saved-pending"));
        key(&mut e, 13);
        capture(&e, &format!("empire-{version}-credits"));
        let mut halted = false;
        e.input_mut().release_all();
        for frame in 0..600 {
            e.input_mut().set_keys((frame < 3).then_some(13));
            if e.run_frame().unwrap().status == FrameStatus::Halted(0) {
                halted = true;
                break;
            }
        }
        assert!(halted);
        let mut restored = load(&path);
        restored
            .files_mut()
            .import_file("/LavaData/dgz.sav", saved.clone());
        advance(&mut restored, 1200);
        assert_eq!(restored.vm().pc(), message_pc);
        assert_eq!(restored.vm().memory()[0x2050], 1);
        assert_eq!(restored.vm().memory()[0x211d], 0);
        assert_eq!((number(&restored, 0x2000), number(&restored, 0x2008)), paid);
        capture(&restored, &format!("empire-{version}-restored-pending"));
        key(&mut restored, 13);
        assert_eq!(restored.vm().pc(), main_pc);
        for _ in 0..7200 {
            if restored.vm().memory()[0x2050] == 0 {
                break;
            }
            advance(&mut restored, 1);
        }
        advance(&mut restored, 120);
        assert_eq!(restored.vm().memory()[0x2050], 0);
        assert_eq!(restored.vm().memory()[0x211d], 1);
        assert_eq!(number(&restored, 0x2034), 8);
        assert_eq!(
            (number(&restored, 0x2000), number(&restored, 0x2008)),
            paid,
            "resource refresh must retain construction costs"
        );
        assert_eq!(
            restored.files().file("/LavaData/dgz.sav"),
            Some(saved.as_slice())
        );
        capture(&restored, &format!("empire-{version}-upgrade-completed"));
        key(&mut restored, 13);
        key(&mut restored, 13);
        key(&mut restored, 13);
        capture(&restored, &format!("empire-{version}-level-one"));
        key(&mut restored, b'u');
        assert_eq!(restored.vm().memory()[0x2050], 1);
        assert_eq!(restored.vm().memory()[0x211d], 1);
        assert!(number(&restored, 0x2000) < paid.0);
        capture(&restored, &format!("empire-{version}-continued-upgrade"));
    }
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn sky_and_land_helper_reads_a_native_account_and_browses_its_catalogs() {
    fn key(e: &mut Emulator, k: u8, frames: usize, held: usize) {
        e.input_mut().release_all();
        for frame in 0..frames {
            e.input_mut().set_keys((frame < held).then_some(k));
            advance(e, 1);
        }
    }
    fn catalog_index(e: &Emulator) -> u16 {
        let address = e.vm().local_bp() as usize + 5;
        u16::from_le_bytes(e.vm().memory()[address..address + 2].try_into().unwrap())
    }
    let mut game = load(&fixture_path("幕天席地.lav"));
    advance(&mut game, 300);
    for k in [13, 13, 20, 13] {
        hold(&mut game, k, 18);
        advance(&mut game, 12);
    }
    for k in b"ylava"
        .iter()
        .copied()
        .chain([13])
        .chain(*b"test")
        .chain([13])
    {
        press(&mut game, k);
    }
    advance(&mut game, 600);
    hold(&mut game, 25, 18);
    press(&mut game, 13);
    let saved = game.files().file("/LavaData/lava.sls").unwrap().to_vec();
    assert_eq!(saved.len(), 88);
    let mut helper = load(&fixture_path("幕天助手.lav"));
    helper
        .files_mut()
        .import_file("/LavaData/lava.sls", saved.clone());
    advance(&mut helper, 1200);
    capture(&helper, "sky-helper-splash");
    key(&mut helper, 13, 600, 180);
    capture(&helper, "sky-helper-title");
    key(&mut helper, 13, 120, 3);
    capture(&helper, "sky-helper-account-prompt");
    for k in b"lava".iter().copied().chain([13]) {
        key(&mut helper, k, 120, 3);
    }
    assert_eq!(&helper.vm().memory()[0x3202..0x3218], &saved[..22]);
    assert_eq!(&helper.vm().memory()[0x3270..0x3284], &saved[22..42]);
    capture(&helper, "sky-helper-menu");
    key(&mut helper, 13, 120, 3);
    assert_eq!(helper.vm().pc(), 0x119f);
    capture(&helper, "sky-helper-stats");
    key(&mut helper, 13, 120, 3);
    capture(&helper, "sky-helper-equipment");
    key(&mut helper, 13, 120, 3);
    key(&mut helper, 21, 120, 3);
    key(&mut helper, 13, 120, 3);
    capture(&helper, "sky-helper-bag");
    key(&mut helper, 27, 120, 3);
    key(&mut helper, 21, 120, 3);
    key(&mut helper, 13, 120, 3);
    capture(&helper, "sky-helper-monster");
    assert_eq!(catalog_index(&helper), 20);
    key(&mut helper, 23, 120, 3);
    assert_eq!(catalog_index(&helper), 20);
    key(&mut helper, 22, 120, 3);
    assert_eq!(catalog_index(&helper), 21);
    capture(&helper, "sky-helper-next-monster");
    for expected in 22..=43 {
        key(&mut helper, 22, 120, 3);
        assert_eq!(catalog_index(&helper), expected);
    }
    key(&mut helper, 22, 120, 3);
    assert_eq!(catalog_index(&helper), 43);
    capture(&helper, "sky-helper-last-monster");
    key(&mut helper, 27, 120, 3);
    assert_eq!(helper.vm().pc(), 0x1ede);
    assert_eq!(helper.vm().memory()[0x36fa], 2);
    capture(&helper, "sky-helper-returned-menu");
    key(&mut helper, 21, 120, 3);
    key(&mut helper, 13, 120, 3);
    capture(&helper, "sky-helper-item");
    assert_eq!(catalog_index(&helper), 0);
    key(&mut helper, 23, 120, 3);
    assert_eq!(catalog_index(&helper), 0);
    key(&mut helper, 22, 120, 3);
    assert_eq!(catalog_index(&helper), 1);
    capture(&helper, "sky-helper-next-item");
    for expected in 2..=49 {
        key(&mut helper, 22, 120, 3);
        assert_eq!(catalog_index(&helper), expected);
    }
    key(&mut helper, 22, 120, 3);
    assert_eq!(catalog_index(&helper), 49);
    capture(&helper, "sky-helper-last-item");
    key(&mut helper, 27, 120, 3);
    assert_eq!(
        helper.files().file("/LavaData/lava.sls"),
        Some(saved.as_slice())
    );
    key(&mut helper, 27, 120, 3);
    capture(&helper, "sky-helper-credits");
    let mut halted = false;
    helper.input_mut().release_all();
    for frame in 0..300 {
        helper.input_mut().set_keys((frame < 3).then_some(13));
        if helper.run_frame().unwrap().status == FrameStatus::Halted(0) {
            halted = true;
            break;
        }
    }
    assert!(halted);
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn sky_and_land_two_explores_defeats_an_enemy_heals_and_restores_progress() {
    fn word(e: &Emulator, a: usize) -> u16 {
        u16::from_le_bytes(e.vm().memory()[a..a + 2].try_into().unwrap())
    }
    fn experience(e: &Emulator) -> u32 {
        u32::from_le_bytes(e.vm().memory()[0x4c0a..0x4c0e].try_into().unwrap())
    }
    fn position(e: &Emulator) -> (u16, u16, u16) {
        (
            word(e, 0x4c12),
            word(e, 0x4c14) + u16::from(e.vm().memory()[0x3f6c]),
            word(e, 0x4c16) + u16::from(e.vm().memory()[0x3f6d]),
        )
    }
    fn key(e: &mut Emulator, k: u8, frames: usize, held: usize) {
        e.input_mut().release_all();
        for frame in 0..frames {
            e.input_mut().set_keys((frame < held).then_some(k));
            advance(e, 1);
        }
    }
    fn step(e: &mut Emulator, k: u8) {
        let before = position(e);
        e.input_mut().release_all();
        for _ in 0..120 {
            e.input_mut().set_keys([k]);
            advance(e, 1);
            if position(e) != before {
                break;
            }
        }
        e.input_mut().release_all();
        advance(e, 60);
        assert_ne!(
            position(e),
            before,
            "the route must advance through actual input"
        );
    }
    let path = fixture_path("幕天席地2.lav");
    let mut e = load(&path);
    advance(&mut e, 1200);
    capture(&e, "sky-two-title");
    key(&mut e, 13, 120, 18);
    key(&mut e, 13, 120, 18);
    advance(&mut e, 1200);
    assert_eq!(e.vm().pc(), 0x2c13);
    capture(&e, "sky-two-new-game-confirmation");
    for k in [b'y', b'e', b'm', b'u', 13, 13, 13] {
        key(&mut e, k, 120, 18);
    }
    assert_eq!(position(&e), (0, 7, 2));
    assert_eq!(word(&e, 0x4c02), 0);
    capture(&e, "sky-two-village");
    for k in [
        22, 21, 21, 21, 21, 21, 22, 22, 22, 21, 21, 23, 23, 23, 23, 21, 21,
    ] {
        step(&mut e, k);
    }
    assert_eq!(position(&e), (1, 4, 3));
    capture(&e, "sky-two-field");
    for _ in 0..3 {
        step(&mut e, 23);
    }
    assert_eq!(position(&e), (1, 1, 3));
    assert_eq!(word(&e, 0x5012), 18);
    capture(&e, "sky-two-before-combat");
    for k in [21, 13, 13, 13] {
        key(&mut e, k, 60, 8);
    }
    assert!(word(&e, 0x5012) as i16 <= 0);
    assert_eq!(experience(&e), 10);
    assert!((1..25).contains(&word(&e, 0x4c18)));
    capture(&e, "sky-two-enemy-defeated");
    for k in [28, 22, 22, 22, 13] {
        key(&mut e, k, 120, 18);
    }
    assert_eq!(e.vm().memory()[0x4ca0], 17);
    capture(&e, "sky-two-potion");
    key(&mut e, 13, 120, 8);
    assert_eq!(word(&e, 0x4c18), word(&e, 0x4c1a));
    assert_eq!(e.vm().memory()[0x4ca0], 18);
    capture(&e, "sky-two-potion-used");
    let inventory = e.vm().memory()[0x4ca0..0x4cb4].to_vec();
    for k in [27, 22, 22, 22, 13] {
        key(&mut e, k, 120, 8);
    }
    capture(&e, "sky-two-system");
    key(&mut e, 13, 120, 8);
    let saved = e.files().file("/LavaData/emu.sl2").unwrap().to_vec();
    assert_eq!(saved.len(), 578);
    capture(&e, "sky-two-saved");
    let mut restored = load(&path);
    restored
        .files_mut()
        .import_file("/LavaData/emu.sl2", saved.clone());
    advance(&mut restored, 1200);
    for k in [13, 21, 13, b'e', b'm', b'u', 13] {
        key(&mut restored, k, 120, 18);
    }
    assert_eq!(position(&restored), (1, 1, 3));
    assert_eq!(experience(&restored), 10);
    assert_eq!(word(&restored, 0x4c18), 25);
    assert_eq!(&restored.vm().memory()[0x4ca0..0x4cb4], inventory);
    assert_eq!(
        restored.files().file("/LavaData/emu.sl2"),
        Some(saved.as_slice())
    );
    capture(&restored, "sky-two-restored");
    step(&mut restored, 20);
    assert_eq!(position(&restored), (1, 1, 2));
    capture(&restored, "sky-two-continued");
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn sky_and_land_three_demo_travels_fights_heals_and_restores_its_account() {
    fn word(e: &Emulator, a: usize) -> u16 {
        u16::from_le_bytes(e.vm().memory()[a..a + 2].try_into().unwrap())
    }
    fn number(e: &Emulator, a: usize) -> u32 {
        u32::from_le_bytes(e.vm().memory()[a..a + 4].try_into().unwrap())
    }
    fn position(e: &Emulator) -> (u16, u16, u16) {
        (word(e, 0x22ae), word(e, 0x54be), word(e, 0x54c0))
    }
    fn key(e: &mut Emulator, k: u8, frames: usize, held: usize) {
        e.input_mut().release_all();
        for frame in 0..frames {
            e.input_mut().set_keys((frame < held).then_some(k));
            advance(e, 1);
        }
    }
    fn step(e: &mut Emulator, k: u8) {
        let before = position(e);
        e.input_mut().release_all();
        for _ in 0..240 {
            e.input_mut().set_keys([k]);
            advance(e, 1);
            if position(e) != before {
                break;
            }
        }
        e.input_mut().release_all();
        advance(e, 60);
        assert_ne!(position(e), before);
    }
    let path = fixture_path("幕天席地3_plus.lav");
    let mut e = load(&path);
    advance(&mut e, 1200);
    capture(&e, "sky-three-splash");
    for k in [13, b'e', b'm', b'u', 13] {
        key(&mut e, k, 180, 18);
    }
    capture(&e, "sky-three-character");
    for k in [b'e', 13, 13, 13, 13] {
        key(&mut e, k, 180, 18);
    }
    assert_eq!(position(&e), (3, 1, 2));
    capture(&e, "sky-three-starting-map");
    for k in [21, 22, 22, 22, 21, 21, 21, 21, 21] {
        step(&mut e, k);
    }
    advance(&mut e, 1200);
    for _ in 0..3 {
        key(&mut e, 13, 180, 18);
    }
    assert_eq!(position(&e), (0, 16, 5));
    capture(&e, "sky-three-village");
    for k in [
        23, 21, 23, 23, 23, 23, 23, 23, 23, 23, 23, 21, 21, 21, 23, 23, 23, 23, 23, 23,
    ] {
        step(&mut e, k);
    }
    advance(&mut e, 600);
    assert_eq!(position(&e), (1, 48, 9));
    capture(&e, "sky-three-field");
    step(&mut e, 23);
    step(&mut e, 23);
    for _ in 0..4 {
        key(&mut e, 13, 120, 18);
    }
    capture(&e, "sky-three-gate-dialogue");
    for _ in 0..3 {
        step(&mut e, 23);
    }
    assert_eq!(position(&e), (1, 43, 9));
    assert_eq!(word(&e, 0x542a), 38);
    assert_eq!(number(&e, 0x22b2), 0);
    let money = number(&e, 0x22b6);
    capture(&e, "sky-three-before-combat");
    for _ in 0..6 {
        key(&mut e, 13, 180, 120);
    }
    assert_eq!(number(&e, 0x22b2), 8);
    assert_eq!(number(&e, 0x22b6), money + 8);
    assert!((1..55).contains(&word(&e, 0x229e)));
    capture(&e, "sky-three-enemy-defeated");
    for k in [28, 22, 22, 22, 13, 13] {
        key(&mut e, k, 120, 18);
    }
    assert_eq!(e.vm().memory()[0x22e0], 87);
    capture(&e, "sky-three-potion-action");
    key(&mut e, 13, 120, 18);
    assert_eq!(e.vm().memory()[0x22e0], 0);
    assert_eq!(word(&e, 0x229e), 55);
    capture(&e, "sky-three-potion-used");
    let inventory = e.vm().memory()[0x22e0..0x22f4].to_vec();
    for k in [27, 22, 22, 22, 13, 13] {
        key(&mut e, k, 120, 18);
    }
    let saved = e.files().file("/LavaData/emu.sl3").unwrap().to_vec();
    assert_eq!(saved.len(), 400);
    capture(&e, "sky-three-saved");
    let mut restored = load(&path);
    restored
        .files_mut()
        .import_file("/LavaData/emu.sl3", saved.clone());
    advance(&mut restored, 1200);
    for k in [21, 13, b'e', b'm', b'u', 13] {
        key(&mut restored, k, 180, 18);
    }
    assert_eq!(position(&restored), (1, 43, 9));
    assert_eq!(number(&restored, 0x22b2), 8);
    assert_eq!(number(&restored, 0x22b6), money + 8);
    assert_eq!(word(&restored, 0x229e), 55);
    assert_eq!(&restored.vm().memory()[0x22e0..0x22f4], inventory);
    assert_eq!(
        restored.files().file("/LavaData/emu.sl3"),
        Some(saved.as_slice())
    );
    capture(&restored, "sky-three-restored");
    step(&mut restored, 20);
    assert_eq!(position(&restored), (1, 43, 8));
    capture(&restored, "sky-three-continued");
    for k in [28, 22, 22, 22, 22, 22, 22, 13, 22, 22, 22, 13] {
        key(&mut restored, k, 120, 18);
    }
    capture(&restored, "sky-three-exit-confirmation");
    restored.input_mut().release_all();
    let mut halted = false;
    for frame in 0..300 {
        restored.input_mut().set_keys((frame < 18).then_some(b'y'));
        if restored.run_frame().unwrap().status == FrameStatus::Halted(0) {
            halted = true;
            break;
        }
    }
    assert!(halted);
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn phantom_fighter_moves_pauses_destroys_enemies_and_reaches_stage_two() {
    fn word(e: &Emulator, a: usize) -> u16 {
        u16::from_le_bytes(e.vm().memory()[a..a + 2].try_into().unwrap())
    }
    fn ticks(e: &Emulator) -> u32 {
        u32::from_le_bytes(e.vm().memory()[0x2e03..0x2e07].try_into().unwrap())
    }
    fn key(e: &mut Emulator, k: u8, frames: usize, held: usize) {
        e.input_mut().release_all();
        for frame in 0..frames {
            e.input_mut().set_keys((frame < held).then_some(k));
            advance(e, 1);
        }
    }
    let mut e = load(&fixture_path("幻影战机.lav"));
    advance(&mut e, 1200);
    capture(&e, "phantom-intro");
    key(&mut e, 13, 120, 18);
    capture(&e, "phantom-title");
    key(&mut e, 13, 120, 18);
    let menu = e.clone();
    capture(&e, "phantom-menu");
    key(&mut e, 13, 120, 18);
    advance(&mut e, 600);
    assert_eq!(e.vm().pc(), 0x3fa8);
    capture(&e, "phantom-stage-one");
    key(&mut e, 13, 120, 18);
    assert_eq!(word(&e, 0x2de3), 600);
    assert_eq!(ticks(&e), 60);
    for (k, address, bound, label) in [
        (22, 0x2de5, 1530, "right"),
        (23, 0x2de5, 120, "left"),
        (20, 0x2de7, 100, "top"),
        (21, 0x2de7, 690, "bottom"),
    ] {
        let mut moved = e.clone();
        key(&mut moved, k, 240, 240);
        assert_eq!(word(&moved, address), bound);
        capture(&moved, &format!("phantom-{label}-bound"));
    }
    let mut paused = e.clone();
    key(&mut paused, b'p', 120, 18);
    assert_eq!(paused.vm().pc(), 0x4423);
    let frozen = paused.vm().memory()[0x2c51..0x2ea7].to_vec();
    advance(&mut paused, 300);
    assert_eq!(&paused.vm().memory()[0x2c51..0x2ea7], frozen);
    capture(&paused, "phantom-paused");
    key(&mut paused, 13, 120, 18);
    assert!(ticks(&paused) > ticks(&e));
    capture(&paused, "phantom-resumed");
    // Each digit holds fire, optionally with up/down/right/left, for 24 frames.
    let actions = b"1003020030001000000002000100002000000000000010000002001004421000020000102013302000001020000100430000214402012330100002000100442000030300010020010204434100023400134200103040200100020100210213402010203040010020103042123412000102033102010002000010424120341002003412000102133020000010402013";
    let mut fired = false;
    let mut destroyed = false;
    let mut damaged = false;
    let mut elapsed = 0;
    for (step, &action) in actions.iter().enumerate() {
        let direction = match action {
            b'0' => None,
            b'1' => Some(20),
            b'2' => Some(21),
            b'3' => Some(22),
            b'4' => Some(23),
            _ => unreachable!(),
        };
        for _ in 0..24 {
            let before: Vec<_> = (0..6)
                .map(|i| (word(&e, 0x2e07 + i * 2), word(&e, 0x2e1b + i * 2) as i16))
                .collect();
            e.input_mut()
                .set_keys(std::iter::once(b'a').chain(direction));
            advance(&mut e, 1);
            fired |= (0..6).any(|i| word(&e, 0x2d7d + i * 2) == 1);
            destroyed |= before.iter().enumerate().any(|(i, &(kind, hp))| {
                (1..=10).contains(&kind) && hp > 0 && word(&e, 0x2e1b + i * 2) as i16 <= 0
            });
            damaged |= word(&e, 0x2de3) < 600;
            elapsed = elapsed.max(ticks(&e));
            assert!(word(&e, 0x2de3) > 0);
        }
        if step == 100 || step == 200 {
            capture(&e, &format!("phantom-combat-{step}"));
        }
    }
    assert!(fired && destroyed && damaged);
    assert!(elapsed > 2000);
    assert_eq!(word(&e, 0x2c4f), 2);
    assert_eq!(word(&e, 0x2de3), 600);
    assert_eq!(word(&e, 0x2df5), 10);
    assert_eq!(word(&e, 0x2df9), 2);
    capture(&e, "phantom-stage-two-upgrade");
    e.input_mut().release_all();
    key(&mut e, 13, 120, 18);
    key(&mut e, b'a', 60, 18);
    assert_eq!(word(&e, 0x2c4f), 2);
    capture(&e, "phantom-stage-two-playing");
    let mut quitter = menu;
    for k in [21, 22] {
        key(&mut quitter, k, 120, 3);
    }
    capture(&quitter, "phantom-abandon-menu");
    quitter.input_mut().release_all();
    let mut halted = false;
    for frame in 0..300 {
        quitter.input_mut().set_keys((frame < 18).then_some(13));
        if quitter.run_frame().unwrap().status == FrameStatus::Halted(0) {
            halted = true;
            break;
        }
    }
    assert!(halted);
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn ghost_house_versions_collect_charms_clear_stage_pause_and_restore_both_save_slots() {
    fn word(e: &Emulator, address: usize) -> u16 {
        u16::from_le_bytes(e.vm().memory()[address..address + 2].try_into().unwrap())
    }
    fn key(e: &mut Emulator, k: u8, frames: usize, held: usize) {
        e.input_mut().release_all();
        for frame in 0..frames {
            e.input_mut()
                .set_keys((k != 0 && frame < held).then_some(k));
            advance(e, 1);
        }
    }
    fn save_and_restore(e: &Emulator, path: &str, slot: u8, prefix: &str) {
        let mut saving = e.clone();
        key(&mut saving, 27 + slot, 60, 6);
        let name = format!("/GVMdata/save{slot}.dat");
        let saved = saving.files().file(&name).unwrap().to_vec();
        assert_eq!(saved.len(), 151);
        assert_eq!(saved[148], 1);
        assert_eq!(saved[149], 5);
        assert_eq!(u16::from(saved[150]), word(e, 0x241c));
        capture(&saving, &format!("{prefix}-slot-{slot}-saved"));
        let mut restored = load(path);
        restored.files_mut().import_file(&name, saved.clone());
        restored.files_mut().import_file(
            "/GVMdata/sys.dat",
            e.files().file("/GVMdata/sys.dat").unwrap().to_vec(),
        );
        advance(&mut restored, 1200);
        for k in [21, 13] {
            key(&mut restored, k, 360, 6);
        }
        if slot == 2 {
            key(&mut restored, 21, 60, 6);
        }
        capture(&restored, &format!("{prefix}-slot-{slot}-selected"));
        key(&mut restored, 13, 60, 6);
        for (address, value) in [
            (0x257c, saved[145]),
            (0x257e, saved[144]),
            (0x258c, saved[146]),
            (0x258e, saved[147]),
            (0x2590, saved[148]),
            (0x2592, saved[149]),
            (0x241c, saved[150]),
        ] {
            assert_eq!(
                word(&restored, address),
                u16::from(value),
                "slot {slot}, {address:x}"
            );
        }
        for x in 1..=24 {
            for y in 1..=6 {
                assert_eq!(
                    word(&restored, 0x241e + (x * 7 + y) * 2),
                    u16::from(saved[(x - 1) * 6 + y - 1])
                );
            }
        }
        advance(&mut restored, 250);
        capture(&restored, &format!("{prefix}-slot-{slot}-restored"));
        let before = word(&restored, 0x2584);
        key(&mut restored, 22, 18, 18);
        assert!(
            word(&restored, 0x2584) > before,
            "slot {slot} before {before} after {} y{} ghost{},{} pc{:x}",
            word(&restored, 0x2584),
            word(&restored, 0x2586),
            word(&restored, 0x2588),
            word(&restored, 0x258a),
            restored.vm().pc()
        );
        assert_eq!(word(&restored, 0x2592), 5);
        assert_eq!(restored.files().file(&name), Some(saved.as_slice()));
        capture(&restored, &format!("{prefix}-slot-{slot}-continued"));
    }
    for (file, prefix) in [
        ("105.lav", "ghost-loose"),
        ("105(打包后的).pac", "ghost-package"),
    ] {
        let path = fixture_path(file);
        let mut e = load(&path);
        advance(&mut e, 1200);
        capture(&e, &format!("{prefix}-title"));
        for k in [21, 21, 21, 13, 21, 23, 23] {
            key(&mut e, k, 360, 6);
        }
        capture(&e, &format!("{prefix}-easy-setting"));
        key(&mut e, 27, 360, 6);
        assert_eq!(e.files().file("/GVMdata/sys.dat"), Some([20, 0].as_slice()));
        for k in [20, 20, 20, 13, 0, 0] {
            key(&mut e, k, 360, 6);
        }
        assert_eq!(word(&e, 0x2590), 1);
        assert_eq!(word(&e, 0x2592), 5);
        capture(&e, &format!("{prefix}-stage-one"));
        let mut caught = e.clone();
        advance(&mut caught, 2400);
        assert!(matches!(caught.vm().pc(), 0x211e | 0x2120));
        capture(&caught, &format!("{prefix}-caught"));
        for _ in 0..2 {
            key(&mut caught, 13, 60, 6);
        }
        advance(&mut caught, 360);
        assert_eq!(word(&caught, 0x2592), 4);
        assert_eq!(word(&caught, 0x2590), 1);
        assert_eq!(word(&caught, 0x241c), 0);
        capture(&caught, &format!("{prefix}-life-lost-restarted"));
        let mut paused = e.clone();
        key(&mut paused, b'p', 60, 6);
        assert_eq!(paused.vm().pc(), 0x095d);
        let frozen = paused.vm().memory()[0x241c..0x2596].to_vec();
        advance(&mut paused, 300);
        assert_eq!(&paused.vm().memory()[0x241c..0x2596], frozen);
        capture(&paused, &format!("{prefix}-paused"));
        key(&mut paused, 13, 60, 6);
        assert_ne!(&paused.vm().memory()[0x241c..0x2596], frozen);
        capture(&paused, &format!("{prefix}-resumed"));
        let route = [
            22, 22, 22, 22, 22, 22, 0, 23, 23, 23, 23, 22, 22, 22, 22, 0, 22, 22, 0, 0, 0, 0, 0,
            22, 22, 22, 22, 22, 22, 22, 0, 0, 0, 0, 0, 22, 22, 22, 22, 22, 22, 22, 22, 22, 0, 22,
            23, 0, 22, 22,
        ];
        for (i, k) in route.into_iter().enumerate() {
            key(&mut e, k, 18, 18);
            if i == 10 {
                assert_eq!(word(&e, 0x241c), 2);
                capture(&e, &format!("{prefix}-two-charms"));
                save_and_restore(&e, &path, 1, prefix);
            }
            if i == 25 {
                assert_eq!(word(&e, 0x241c), 3);
                capture(&e, &format!("{prefix}-three-charms"));
                save_and_restore(&e, &path, 2, prefix);
            }
        }
        assert_eq!(word(&e, 0x2590), 2);
        assert_eq!(word(&e, 0x2592), 5);
        assert_eq!(word(&e, 0x241c), 0);
        capture(&e, &format!("{prefix}-stage-two-announcement"));
        advance(&mut e, 360);
        capture(&e, &format!("{prefix}-stage-two"));
        key(&mut e, 27, 60, 6);
        assert_eq!(e.vm().pc(), 0x10d3);
        capture(&e, &format!("{prefix}-returned-menu"));
        let mut halted = false;
        for frame in 0..120 {
            e.input_mut().set_keys((frame < 6).then_some(27));
            if e.run_frame().unwrap().status == FrameStatus::Halted(0) {
                halted = true;
                break;
            }
        }
        assert!(halted);
    }
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn foreign_land_versions_fight_follow_the_school_story_and_preserve_both_save_slots() {
    fn word(e: &Emulator, a: usize) -> u16 {
        u16::from_le_bytes(e.vm().memory()[a..a + 2].try_into().unwrap())
    }
    fn position(e: &Emulator) -> (u8, u16, u16) {
        let m = e.vm().memory();
        (
            m[0x4972],
            u16::from(m[0x4968]) + u16::from(m[0x496c]),
            u16::from(m[0x4969]) + u16::from(m[0x496d]),
        )
    }
    fn key(e: &mut Emulator, k: u8, frames: usize, held: usize) {
        e.input_mut().release_all();
        for frame in 0..frames {
            e.input_mut().set_keys((frame < held).then_some(k));
            advance(e, 1);
        }
    }
    fn repeated(e: &mut Emulator, k: u8, count: usize, frames: usize, held: usize) {
        for _ in 0..count {
            key(e, k, frames, held);
        }
    }
    fn system_menu(e: &mut Emulator) {
        key(e, 25, 120, 3);
        repeated(e, 22, 4, 120, 3);
        key(e, 13, 120, 3);
    }
    fn save(e: &mut Emulator, slot: usize, label: &str) -> Vec<u8> {
        system_menu(e);
        key(e, 13, 120, 3);
        if slot == 2 {
            key(e, 21, 120, 3);
        }
        capture(e, &format!("{label}-slot-{slot}-save-choice"));
        key(e, 13, 120, 3);
        let file = e.files().file("/LavaData/Land_Save.dat").unwrap().to_vec();
        assert_eq!(file.len(), 4000);
        capture(e, &format!("{label}-slot-{slot}-saved"));
        file
    }
    for (file, label) in [("Land.lav", "land-loose"), ("Land.pac", "land-package")] {
        let path = fixture_path(file);
        let mut e = load(&path);
        advance(&mut e, 1200);
        capture(&e, &format!("{label}-title"));
        for i in 0..10 {
            key(&mut e, 13, 180, 18);
            if i == 0 {
                capture(&e, &format!("{label}-introduction"));
            }
        }
        assert_eq!(position(&e), (1, 46, 4));
        let beginning = e.clone();
        capture(&e, &format!("{label}-outside-home"));
        repeated(&mut e, 23, 20, 40, 1);
        assert_eq!(position(&e), (1, 27, 4));
        repeated(&mut e, 21, 3, 40, 1);
        repeated(&mut e, 23, 22, 40, 1);
        repeated(&mut e, 20, 3, 40, 1);
        key(&mut e, 13, 40, 1);
        assert_eq!(position(&e), (1, 19, 4));
        repeated(&mut e, 21, 3, 120, 1);
        key(&mut e, 23, 120, 1);
        repeated(&mut e, 13, 4, 120, 1);
        assert_eq!(position(&e), (1, 19, 6));
        assert_eq!(e.vm().pc(), 0x10c4);
        assert_eq!(word(&e, 0x4bcd), 10);
        capture(&e, &format!("{label}-first-battle"));
        key(&mut e, 13, 180, 3);
        capture(&e, &format!("{label}-attack"));
        key(&mut e, 13, 180, 3);
        let bp = e.vm().local_bp() as usize;
        assert_eq!(&e.vm().memory()[bp..bp + 3], [0xc5, 0x63, 0]);
        assert_eq!(word(&e, 0x48c8), 200);
        assert_eq!(word(&e, 0x48cc), 0);
        assert_eq!(word(&e, 0x4901), 240);
        capture(&e, &format!("{label}-victory"));
        repeated(&mut e, 13, 10, 180, 3);
        repeated(&mut e, 23, 14, 40, 1);
        repeated(&mut e, 20, 4, 40, 1);
        assert_eq!(position(&e), (1, 2, 3));
        repeated(&mut e, 21, 4, 40, 1);
        repeated(&mut e, 22, 5, 40, 1);
        repeated(&mut e, 20, 3, 40, 1);
        assert_eq!(position(&e), (1, 44, 19));
        capture(&e, &format!("{label}-school-entrance"));
        repeated(&mut e, 20, 5, 120, 1);
        repeated(&mut e, 13, 10, 120, 1);
        assert_eq!(position(&e), (1, 44, 17));
        repeated(&mut e, 23, 4, 40, 1);
        repeated(&mut e, 20, 3, 40, 1);
        repeated(&mut e, 13, 7, 40, 1);
        assert_eq!(position(&e), (1, 31, 18));
        capture(&e, &format!("{label}-school-stairs"));
        repeated(&mut e, 20, 8, 40, 1);
        assert_eq!(position(&e), (1, 11, 18));
        capture(&e, &format!("{label}-classroom"));
        for (i, k) in [22, 22, 22, 22, 20, 20]
            .into_iter()
            .chain(std::iter::repeat_n(13, 16))
            .enumerate()
        {
            key(&mut e, k, 120, 1);
            if [0, 2, 10, 13].contains(&i) {
                capture(&e, &format!("{label}-classroom-dialogue-{i}"));
            }
        }
        assert_eq!(position(&e), (1, 11, 18));
        capture(&e, &format!("{label}-friends-departed"));
        let second = save(&mut e, 2, label);
        assert!(second[..2000].iter().all(|&byte| byte == 0));
        let mut earlier = beginning;
        earlier
            .files_mut()
            .import_file("/LavaData/Land_Save.dat", second.clone());
        let combined = save(&mut earlier, 1, label);
        assert_eq!(&combined[2000..], &second[2000..]);
        assert_ne!(&combined[..2000], &combined[2000..]);
        for (slot, expected) in [(1, (1, 46, 4)), (2, (1, 11, 18))] {
            let mut restored = load(&path);
            restored
                .files_mut()
                .import_file("/LavaData/Land_Save.dat", combined.clone());
            advance(&mut restored, 1200);
            for k in [21, 13] {
                key(&mut restored, k, 120, 3);
            }
            if slot == 2 {
                key(&mut restored, 21, 120, 3);
            }
            capture(&restored, &format!("{label}-slot-{slot}-load-choice"));
            for _ in 0..2 {
                key(&mut restored, 13, 120, 3);
            }
            assert_eq!(position(&restored), expected);
            assert_eq!(word(&restored, 0x48c8), 200);
            assert_eq!(word(&restored, 0x48ca), 60);
            assert_eq!(word(&restored, 0x4901), 240);
            assert_eq!(word(&restored, 0x48ce), 3);
            assert_eq!(
                restored.files().file("/LavaData/Land_Save.dat"),
                Some(combined.as_slice())
            );
            capture(&restored, &format!("{label}-slot-{slot}-restored"));
            repeated(&mut restored, 21, 3, 40, 1);
            assert_ne!(position(&restored), expected);
            capture(&restored, &format!("{label}-slot-{slot}-continued"));
            system_menu(&mut restored);
            repeated(&mut restored, 21, 3, 120, 3);
            capture(&restored, &format!("{label}-slot-{slot}-quit-choice"));
            let mut halted = false;
            for frame in 0..600 {
                restored
                    .input_mut()
                    .set_keys((frame < 3 || (180..183).contains(&frame)).then_some(13));
                if restored.run_frame().unwrap().status == FrameStatus::Halted(0) {
                    halted = true;
                    break;
                }
            }
            assert!(halted);
        }
    }
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn power_driller_versions_clear_training_and_stage_one_and_restore_unlocks() {
    fn word(e: &Emulator, a: usize) -> u16 {
        u16::from_le_bytes(e.vm().memory()[a..a + 2].try_into().unwrap())
    }
    fn key(e: &mut Emulator, k: u8, frames: usize, held: usize) {
        e.input_mut().release_all();
        for frame in 0..frames {
            e.input_mut()
                .set_keys((k != 0 && frame < held).then_some(k));
            advance(e, 1);
        }
    }
    fn menu(e: &mut Emulator) {
        advance(e, 1200);
        for _ in 0..4 {
            key(e, 13, 180, 18);
        }
        assert_eq!(word(e, 0x3e4c), 1);
    }
    fn leave(e: &mut Emulator) {
        key(e, 27, 180, 18);
        assert_eq!(word(e, 0x3e4c), 1);
        assert_eq!(e.vm().memory()[0x3e57], 1);
    }
    for (file, label) in [
        ("强力钻土机(模拟器).lav", "driller-paced"),
        ("强力钻土机(其它机型).lav", "driller-device"),
        ("强力钻土机(Tc800).lav", "driller-tc"),
    ] {
        let path = fixture_path(file);
        let mut e = load(&path);
        menu(&mut e);
        capture(&e, &format!("{label}-level-menu"));
        key(&mut e, b'u', 120, 3);
        capture(&e, &format!("{label}-training-instructions"));
        for _ in 0..8 {
            key(&mut e, 13, 120, 3);
        }
        assert_eq!(e.vm().memory()[0x3e53], 0);
        assert_eq!(word(&e, 0x3e40), 0);
        assert_eq!(word(&e, 0x3e42), 0);
        assert_eq!(word(&e, 0x3e5a), 50);
        assert_eq!(word(&e, 0x3e58), 2000);
        capture(&e, &format!("{label}-training"));
        let mut paced = e.clone();
        let tick = paced.vm().memory()[0x3e50];
        advance(&mut paced, 180);
        assert_eq!(paced.vm().memory()[0x3e50].wrapping_sub(tick), 45);
        let mut moved = e.clone();
        key(&mut moved, b'd', 32, 32);
        assert!(word(&moved, 0x3e40) > 0);
        assert!(word(&moved, 0x3e58) < 2000);
        capture(&moved, &format!("{label}-movement"));
        let training = [
            (100, 31),
            (100, 29),
            (100, 29),
            (100, 29),
            (100, 29),
            (100, 29),
            (100, 29),
            (100, 29),
            (100, 29),
            (100, 29),
            (100, 29),
            (100, 29),
            (100, 29),
            (100, 29),
            (115, 29),
            (115, 29),
            (115, 29),
            (115, 29),
            (97, 29),
            (97, 29),
            (119, 29),
            (97, 29),
            (97, 29),
            (97, 29),
            (97, 29),
            (97, 29),
            (97, 29),
            (97, 29),
            (97, 29),
            (97, 29),
            (97, 29),
            (115, 29),
            (115, 29),
            (97, 29),
            (97, 29),
            (115, 29),
            (115, 29),
            (100, 29),
            (115, 29),
            (115, 29),
            (100, 29),
            (100, 29),
            (100, 29),
            (119, 29),
            (119, 29),
            (119, 29),
            (100, 29),
            (119, 29),
            (100, 29),
            (100, 29),
            (115, 29),
            (115, 29),
            (115, 29),
            (115, 29),
            (100, 29),
            (100, 29),
            (100, 29),
            (100, 29),
            (100, 29),
            (100, 29),
            (100, 41),
            (100, 29),
            (100, 41),
            (97, 29),
            (119, 41),
            (97, 29),
            (119, 41),
            (100, 29),
            (100, 17),
        ];
        for (k, frames) in training {
            key(&mut e, k, frames, frames);
            e.input_mut().release_all();
            advance(&mut e, 3);
        }
        assert_eq!(word(&e, 0x3e5c), 5);
        assert_eq!(e.vm().memory()[0x3e53], 1);
        assert!(word(&e, 0x3e5a) > 0);
        capture(&e, &format!("{label}-training-complete"));
        e.input_mut().release_all();
        advance(&mut e, 180);
        for _ in 0..4 {
            key(&mut e, 13, 180, 3);
        }
        key(&mut e, b'u', 180, 3);
        assert_eq!(e.vm().memory()[0x3e53], 1);
        assert_eq!(word(&e, 0x3e5c), 0);
        capture(&e, &format!("{label}-stage-one"));
        // Fixed real inputs include idle blocks for enemy timing and pickup animations.
        let actions = b"sss0sss0ddd0dss0ssa0aaa0sss0sss0ssd0ddd0ddd0aa0aaa0aww0www0www0ddd0dss0dds0sdd0ddd0ddd0ddd0dww0www0waa0aaa0aww0wa00aww0www0aaa0aas0wddd0ddd0ddd0ddd0ddd0dds0sss0sss0sss0sss0sss0ssa0aaa0aaa0aadddd0ddd0ddd0ddd0ddd0dd0ddawww0wdd0www0www0www0www0aaw0waa0aaa0ass0sss0sdd0sss0sas00";
        let mut gems = 0;
        for &action in actions {
            let k = if action == b'0' { 0 } else { action };
            key(&mut e, k, 12, 12);
            assert!((1..=50).contains(&word(&e, 0x3e5a)));
            let count = word(&e, 0x3e5c);
            if count > gems {
                gems = count;
                capture(&e, &format!("{label}-stage-one-gem-{count}"));
            }
        }
        assert_eq!(gems, 5);
        assert_eq!(e.vm().memory()[0x3e53], 2);
        assert!((1..50).contains(&word(&e, 0x3e5a)));
        e.input_mut().release_all();
        advance(&mut e, 180);
        let saved = e.files().file("/LavaData/diamond.dat").unwrap().to_vec();
        assert_eq!(saved.len(), 25);
        assert_eq!(&saved[..2], &[2, 1]);
        capture(&e, &format!("{label}-stage-one-complete"));
        // Select the level menu on the native result screen.
        key(&mut e, b'd', 60, 3);
        key(&mut e, b'u', 180, 3);
        assert_eq!(word(&e, 0x3e4c), 1);
        capture(&e, &format!("{label}-unlocked-menu"));
        let mut restored = load(&path);
        restored
            .files_mut()
            .import_file("/LavaData/diamond.dat", saved.clone());
        menu(&mut restored);
        assert_eq!(&restored.vm().memory()[0x4afa..0x4b09], &saved[..15]);
        assert_eq!(
            restored.files().file("/LavaData/diamond.dat"),
            Some(saved.as_slice())
        );
        capture(&restored, &format!("{label}-restored-unlocks"));
        for (k, a, target) in [(b'd', 0x3e48, 34), (b's', 0x3e4a, 25)] {
            for _ in 0..600 {
                if word(&restored, a) >= target {
                    break;
                }
                restored.input_mut().set_keys([k]);
                advance(&mut restored, 1);
            }
            restored.input_mut().release_all();
            advance(&mut restored, 3);
            assert!(word(&restored, a) >= target);
        }
        capture(&restored, &format!("{label}-stage-two-selected"));
        key(&mut restored, b'u', 180, 3);
        assert_eq!(restored.vm().memory()[0x3e53], 2);
        assert_eq!(word(&restored, 0x3e5c), 0);
        assert!(word(&restored, 0x3e5a) > 0);
        capture(&restored, &format!("{label}-stage-two"));
        leave(&mut restored);
        capture(&restored, &format!("{label}-returned-menu"));
        let mut halted = false;
        for frame in 0..300 {
            restored.input_mut().set_keys((frame < 18).then_some(27));
            if restored.run_frame().unwrap().status == FrameStatus::Halted(0) {
                halted = true;
                break;
            }
        }
        assert!(halted);
    }
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn ninja_versions_keep_the_default_name_win_a_battle_and_restore_both_slots() {
    fn word(e: &Emulator, a: usize) -> u16 {
        u16::from_le_bytes(e.vm().memory()[a..a + 2].try_into().unwrap())
    }
    fn position(e: &Emulator) -> (u8, u16, u16) {
        let m = e.vm().memory();
        (
            m[0x43b4],
            u16::from(m[0x43ae]) + u16::from(m[0x43b0]),
            u16::from(m[0x43af]) + u16::from(m[0x43b1]),
        )
    }
    fn key(e: &mut Emulator, k: u8, frames: usize, held: usize) {
        e.input_mut().release_all();
        for frame in 0..frames {
            e.input_mut().set_keys((frame < held).then_some(k));
            advance(e, 1);
        }
    }
    fn repeated(e: &mut Emulator, k: u8, count: usize, frames: usize, held: usize) {
        for _ in 0..count {
            key(e, k, frames, held);
        }
    }
    fn system_menu(e: &mut Emulator) {
        key(e, 25, 120, 3);
        repeated(e, 22, 4, 120, 3);
        key(e, 13, 120, 3);
    }
    fn save(e: &mut Emulator, slot: usize, label: &str) -> Vec<u8> {
        system_menu(e);
        key(e, 13, 120, 3);
        if slot == 2 {
            key(e, 21, 120, 3);
        }
        capture(e, &format!("{label}-slot-{slot}-save-choice"));
        key(e, 13, 120, 3);
        let bytes = e.files().file("/GVMdata/RZ_Save.dat").unwrap().to_vec();
        assert_eq!(bytes.len(), 4000);
        capture(e, &format!("{label}-slot-{slot}-saved"));
        key(e, 13, 120, 3);
        bytes
    }
    for (file, label) in [
        ("忍う者.lav", "ninja-loose"),
        ("忍う者.pac", "ninja-package"),
    ] {
        let path = fixture_path(file);
        let mut e = load(&path);
        advance(&mut e, 1200);
        capture(&e, &format!("{label}-title"));
        for i in 0..8 {
            key(&mut e, 13, 180, 3);
            if i == 0 {
                capture(&e, &format!("{label}-name-entry"));
            }
        }
        assert_eq!(
            &e.vm().memory()[0x42fd..0x4304],
            &[0xce, 0xde, 0xa4, 0xa6, 0xb5, 0xd0, 0]
        );
        assert_eq!(position(&e), (1, 3, 68));
        capture(&e, &format!("{label}-cemetery"));
        repeated(&mut e, 20, 6, 60, 1);
        repeated(&mut e, 13, 2, 60, 1);
        capture(&e, &format!("{label}-goddess-dialogue"));
        repeated(&mut e, 13, 24, 180, 3);
        assert_eq!(position(&e), (1, 31, 7));
        assert_eq!(word(&e, 0x4320), 1000);
        capture(&e, &format!("{label}-town-reward"));
        let beginning = e.clone();
        for k in [
            22, 22, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 23, 23, 23, 23, 23, 23, 23, 23, 23, 21,
            21, 21, 21, 23, 23, 23, 23, 23, 23, 20, 20, 23, 23,
        ] {
            key(&mut e, k, 60, 1);
        }
        assert_eq!(position(&e), (1, 18, 18));
        key(&mut e, 23, 60, 1);
        assert_eq!(position(&e), (1, 1, 30));
        capture(&e, &format!("{label}-forest-entrance"));
        for (k, count) in [(22, 6), (21, 12), (22, 6), (20, 8)] {
            repeated(&mut e, k, count, 60, 1);
        }
        for (k, count) in [(22, 4), (21, 16), (22, 10), (20, 10)] {
            repeated(&mut e, k, count, 90, 1);
        }
        assert_eq!(position(&e), (1, 6, 32));
        assert_eq!(e.vm().pc(), 0x118b);
        assert_ne!(e.vm().memory()[0x4365], 0);
        capture(&e, &format!("{label}-battle-menu"));
        for i in 0..20 {
            key(&mut e, 13, 180, 3);
            if [0, 3, 10, 18].contains(&i) {
                capture(&e, &format!("{label}-battle-{i}"));
            }
        }
        assert_eq!(e.vm().memory()[0x4365], 0);
        assert_eq!(e.vm().memory()[0x4310], 1);
        assert_eq!(word(&e, 0x430a), 202);
        assert_eq!(word(&e, 0x4320), 1007);
        capture(&e, &format!("{label}-victory-returned-field"));
        let second = save(&mut e, 2, label);
        assert!(second[..2000].iter().all(|&byte| byte == 0));
        let mut earlier = beginning;
        earlier
            .files_mut()
            .import_file("/GVMdata/RZ_Save.dat", second.clone());
        let combined = save(&mut earlier, 1, label);
        assert_eq!(&combined[2000..], &second[2000..]);
        assert_ne!(&combined[..2000], &combined[2000..]);
        for (slot, expected, gold, hp, level) in
            [(1, (1, 31, 7), 1000, 200, 0), (2, (1, 6, 32), 1007, 202, 1)]
        {
            let mut restored = load(&path);
            restored
                .files_mut()
                .import_file("/GVMdata/RZ_Save.dat", combined.clone());
            advance(&mut restored, 1200);
            for k in [21, 13] {
                key(&mut restored, k, 120, 3);
            }
            if slot == 2 {
                key(&mut restored, 21, 120, 3);
            }
            capture(&restored, &format!("{label}-slot-{slot}-load-choice"));
            repeated(&mut restored, 13, 2, 120, 3);
            assert_eq!(position(&restored), expected);
            assert_eq!(word(&restored, 0x4320), gold);
            assert_eq!(word(&restored, 0x430a), hp);
            assert_eq!(restored.vm().memory()[0x4310], level);
            assert_eq!(
                &restored.vm().memory()[0x42fd..0x4304],
                &[0xce, 0xde, 0xa4, 0xa6, 0xb5, 0xd0, 0]
            );
            assert_eq!(
                restored.files().file("/GVMdata/RZ_Save.dat"),
                Some(combined.as_slice())
            );
            capture(&restored, &format!("{label}-slot-{slot}-restored"));
            repeated(&mut restored, if slot == 1 { 22 } else { 20 }, 3, 60, 1);
            assert_ne!(position(&restored), expected);
            capture(&restored, &format!("{label}-slot-{slot}-continued"));
            system_menu(&mut restored);
            repeated(&mut restored, 21, 2, 120, 3);
            capture(&restored, &format!("{label}-slot-{slot}-quit-choice"));
            let mut halted = false;
            for frame in 0..600 {
                restored
                    .input_mut()
                    .set_keys((frame < 3 || (180..183).contains(&frame)).then_some(13));
                if restored.run_frame().unwrap().status == FrameStatus::Halted(0) {
                    halted = true;
                    break;
                }
            }
            assert!(halted);
        }
    }
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn love_versions_save_and_restore_both_native_slots_from_an_empty_file() {
    fn position(e: &Emulator) -> (u8, u16, u16) {
        let m = e.vm().memory();
        (
            m[0x4972],
            u16::from(m[0x4968]) + u16::from(m[0x496c]),
            u16::from(m[0x4969]) + u16::from(m[0x496d]),
        )
    }
    fn key(e: &mut Emulator, k: u8) {
        e.input_mut().release_all();
        for frame in 0..120 {
            e.input_mut().set_keys((frame < 3).then_some(k));
            advance(e, 1);
        }
    }
    fn open_field(e: &mut Emulator) {
        advance(e, 1200);
        for _ in 0..20 {
            key(e, 13);
        }
        assert_eq!(position(e), (1, 5, 26));
    }
    fn system_menu(e: &mut Emulator) {
        key(e, 25);
        for _ in 0..4 {
            key(e, 22);
        }
        key(e, 13);
    }
    fn save(e: &mut Emulator, slot: usize) -> Vec<u8> {
        system_menu(e);
        key(e, 13);
        if slot == 2 {
            key(e, 21);
        }
        key(e, 13);
        let data = e.files().file("/LavaData/Love_Save.dat").unwrap().to_vec();
        assert_eq!(data.len(), 4000);
        capture(e, &format!("love-slot-{slot}-saved"));
        data
    }
    for (file, label) in [("Love_I.lav", "love-loose"), ("Love.pac", "love-package")] {
        let path = if file == "Love.pac" {
            fixture_path_with_size(file, Some(219_064))
        } else {
            fixture_path_with_size(file, Some(35_867))
        };
        let mut e = load(&path);
        open_field(&mut e);
        capture(&e, &format!("{label}-opening-field"));
        let first_point = e.clone();
        let second = save(&mut e, 2);
        assert!(second[..2000].iter().all(|&byte| byte == 0));
        assert!(second[2000..].iter().any(|&byte| byte != 0));

        let mut earlier = first_point;
        key(&mut earlier, 23);
        key(&mut earlier, 21);
        assert_eq!(position(&earlier), (1, 4, 27));
        capture(&earlier, &format!("{label}-moved-field"));
        earlier
            .files_mut()
            .import_file("/LavaData/Love_Save.dat", second.clone());
        let combined = save(&mut earlier, 1);
        assert_eq!(&combined[2000..], &second[2000..]);
        assert_ne!(&combined[..2000], &combined[2000..]);

        for (slot, expected) in [(1, (1, 4, 27)), (2, (1, 5, 26))] {
            let mut restored = load(&path);
            restored
                .files_mut()
                .import_file("/LavaData/Love_Save.dat", combined.clone());
            open_field(&mut restored);
            system_menu(&mut restored);
            key(&mut restored, 21);
            key(&mut restored, 13);
            if slot == 2 {
                key(&mut restored, 21);
            }
            key(&mut restored, 13);
            assert_eq!(position(&restored), expected);
            assert_eq!(
                restored.files().file("/LavaData/Love_Save.dat"),
                Some(combined.as_slice())
            );
            capture(&restored, &format!("{label}-slot-{slot}-restored"));
            key(&mut restored, 23);
            assert_ne!(position(&restored), expected);
            capture(&restored, &format!("{label}-slot-{slot}-continued"));
        }
    }
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn my_world_other_realm_reaches_the_map_and_moves() {
    let mut emulator = load("LAVA1 我的世界异界版/我的世界.lav");
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
    let map = emulator.display().indexed_frame().to_vec();
    assert!(
        map.iter().any(|&pixel| pixel != map[0]),
        "map must be visible"
    );
    capture(&emulator, "my-world-other-realm-map");

    let mut idle = emulator.clone();
    advance(&mut idle, 60);
    capture(&idle, "my-world-other-realm-idle");
    for (key, direction) in [(21, "down"), (22, "left"), (23, "right")] {
        let moved = input_branch(&emulator, key);
        assert_ne!(
            moved.display().indexed_frame(),
            idle.display().indexed_frame(),
            "{direction} must move the created character"
        );
        capture(&moved, &format!("my-world-other-realm-{direction}"));
    }
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn strategy_one_versions_show_the_battlefield_and_accept_orders() {
    fn status_panel(e: &Emulator) -> Vec<u8> {
        let width = usize::from(e.display().width());
        e.display()
            .indexed_frame()
            .chunks_exact(width)
            .flat_map(|row| row[width * 3 / 5..].iter().copied())
            .collect()
    }
    fn start_game(e: &mut Emulator) {
        e.input_mut().release_all();
        for frame in 0..24 {
            e.input_mut().set_keys((frame == 0).then_some(13));
            advance(e, 1);
        }
        advance(e, 1200);
    }
    for (path, label) in [
        (
            fixture_path_with_size("战略1号.pac", Some(26_786)),
            "packaged",
        ),
        (fixture_path_with_size("战略1号.lav", Some(19_888)), "loose"),
    ] {
        let mut e = load(&path);
        advance(&mut e, 1200);
        start_game(&mut e);
        start_game(&mut e);
        assert_eq!(e.vm().pc(), 0x30cf);
        capture(&e, &format!("strategy-one-{label}-battlefield"));
        let mut idle_selection = e.clone();
        advance(&mut idle_selection, 60);
        let selected = input_branch(&e, b'a');
        assert_ne!(
            status_panel(&idle_selection),
            status_panel(&selected),
            "A must select the available combat units"
        );
        capture(&selected, &format!("strategy-one-{label}-selected"));
        let mut selected_idle = selected.clone();
        advance(&mut selected_idle, 360);
        let commanded = input_branch(&selected, 14);
        let mut commanded = commanded;
        advance(&mut commanded, 300);
        assert_ne!(
            &selected_idle.vm().memory()[0x2020..0x2050],
            &commanded.vm().memory()[0x2020..0x2050],
            "Page Down must apply an order to the selected units"
        );
        assert_ne!(
            selected_idle.display().indexed_frame(),
            commanded.display().indexed_frame(),
            "ordered units must produce a visible battlefield update"
        );
        capture(&commanded, &format!("strategy-one-{label}-commanded"));
        let mut direction_idle = selected.clone();
        advance(&mut direction_idle, 60);
        for (index, key) in [20, 21, 22, 23].into_iter().enumerate() {
            let moved = input_branch(&selected, key);
            assert_ne!(
                direction_idle.display().indexed_frame(),
                moved.display().indexed_frame(),
                "direction keys must move the battlefield pointer"
            );
            capture(&moved, &format!("strategy-one-{label}-direction-{index}"));
        }
    }
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn poker_battle_bets_replaces_a_card_and_settles_a_hand() {
    fn press(e: &mut Emulator, key: u8) {
        e.input_mut().release_all();
        for frame in 0..60 {
            e.input_mut().set_keys((frame == 0).then_some(key));
            advance(e, 1);
        }
    }
    fn region(e: &Emulator, x0: usize, x1: usize, y0: usize, y1: usize) -> Vec<u8> {
        let width = usize::from(e.display().width());
        e.display()
            .indexed_frame()
            .chunks_exact(width)
            .skip(y0)
            .take(y1 - y0)
            .flat_map(|row| row[x0..x1].iter().copied())
            .collect()
    }

    let path = fixture_path_with_size("扑克大战.lav", Some(10_765));
    let mut game = load(&path);
    advance(&mut game, 1200);
    assert_eq!(game.vm().pc(), 0x28ce);
    assert!(
        game.display()
            .indexed_frame()
            .iter()
            .any(|&pixel| pixel != game.display().indexed_frame()[0])
    );
    capture(&game, "poker-battle-title");

    press(&mut game, 13);
    assert_eq!(game.vm().pc(), 0x294c);
    capture(&game, "poker-battle-instructions");
    press(&mut game, 13);
    assert_eq!(game.vm().pc(), 0x1690);
    capture(&game, "poker-battle-betting");

    let mut idle_bet = game.clone();
    advance(&mut idle_bet, 60);
    let mut raised_bet = game.clone();
    press(&mut raised_bet, 20);
    assert_eq!(raised_bet.vm().pc(), 0x1690);
    assert_ne!(
        region(&idle_bet, 0, 160, 0, 16),
        region(&raised_bet, 0, 160, 0, 16),
        "Up must update the displayed betting terms"
    );
    capture(&raised_bet, "poker-battle-raised-bet");

    press(&mut raised_bet, 13);
    assert_eq!(raised_bet.vm().pc(), 0x1aec);
    capture(&raised_bet, "poker-battle-hand");
    let mut idle_hand = raised_bet.clone();
    advance(&mut idle_hand, 60);
    for (index, key) in b"bnmgh".iter().copied().enumerate() {
        let mut selected = raised_bet.clone();
        press(&mut selected, key);
        assert_eq!(selected.vm().pc(), 0x1aec);
        assert_ne!(
            region(&idle_hand, 38, 116, 16, 68),
            region(&selected, 38, 116, 16, 68),
            "number key {} must select its corresponding card",
            index + 1
        );
        capture(
            &selected,
            &format!("poker-battle-card-{}-selected", index + 1),
        );
    }

    let mut selected = raised_bet.clone();
    press(&mut selected, b'b');
    assert_ne!(
        region(&idle_hand, 38, 116, 16, 68),
        region(&selected, 38, 116, 16, 68),
        "number key 1 must mark the first card for replacement"
    );
    capture(&selected, "poker-battle-card-marked");
    press(&mut selected, 13);
    assert_eq!(selected.vm().pc(), 0x29d4);
    capture(&selected, "poker-battle-round-result");
    press(&mut selected, 13);
    assert_eq!(selected.vm().pc(), 0x1690);
    capture(&selected, "poker-battle-next-bet");
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn plane_shooter_enters_practice_and_accepts_board_controls() {
    fn tap(e: &mut Emulator, key: u8) {
        e.input_mut().release_all();
        e.input_mut().set_keys([key]);
        advance(e, 1);
        e.input_mut().set_keys([]);
        advance(e, 59);
    }
    fn region(e: &Emulator, x0: usize, x1: usize, y0: usize, y1: usize) -> Vec<u8> {
        let width = usize::from(e.display().width());
        e.display()
            .indexed_frame()
            .chunks_exact(width)
            .skip(y0)
            .take(y1 - y0)
            .flat_map(|row| row[x0..x1].iter().copied())
            .collect()
    }
    let path = fixture_path_with_size("打飞机.lav", Some(11_229));
    let mut title = load(&path);
    advance(&mut title, 1200);
    assert_eq!(title.vm().pc(), 0x2758);
    assert!(
        title
            .display()
            .indexed_frame()
            .iter()
            .any(|&pixel| pixel != title.display().indexed_frame()[0]),
        "the title and mode choices must be visible"
    );
    capture(&title, "plane-shooter-title");

    let mut game = title.clone();
    tap(&mut game, 13);
    assert_eq!(game.vm().pc(), 0x1554);
    capture(&game, "plane-shooter-practice-board");

    let mut idle = game.clone();
    advance(&mut idle, 60);
    let idle_board = region(&idle, 80, 145, 12, 76);
    let mut directional_frames = Vec::new();
    for (index, key) in [20, 21, 22, 23].into_iter().enumerate() {
        let mut moved = game.clone();
        tap(&mut moved, key);
        let board = region(&moved, 80, 145, 12, 76);
        assert_ne!(
            board, idle_board,
            "direction key {key} must move the visible board cursor"
        );
        directional_frames.push(board);
        capture(&moved, &format!("plane-shooter-direction-{index}"));
    }
    directional_frames.sort();
    directional_frames.dedup();
    assert_eq!(
        directional_frames.len(),
        4,
        "each direction must select a distinct board position"
    );

    let mut selected = game.clone();
    tap(&mut selected, 13);
    assert_ne!(
        selected.display().indexed_frame(),
        idle.display().indexed_frame(),
        "Enter must perform a visible board action"
    );
    capture(&selected, "plane-shooter-board-action");

    let mut exit_prompt = game;
    tap(&mut exit_prompt, 27);
    assert_eq!(exit_prompt.vm().pc(), 0x165e);
    assert_ne!(
        region(&exit_prompt, 45, 115, 22, 56),
        region(&idle, 45, 115, 22, 56),
        "Escape must show the game's exit confirmation"
    );
    capture(&exit_prompt, "plane-shooter-exit-confirmation");
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn minesweeper_opens_cells_marks_and_moves_on_the_board() {
    fn tap(e: &mut Emulator, key: u8) {
        e.input_mut().release_all();
        e.input_mut().set_keys([key]);
        advance(e, 1);
        e.input_mut().release_all();
        advance(e, 59);
    }
    fn region(e: &Emulator, x0: usize, x1: usize, y0: usize, y1: usize) -> Vec<u8> {
        let width = usize::from(e.display().width());
        e.display()
            .indexed_frame()
            .chunks_exact(width)
            .skip(y0)
            .take(y1 - y0)
            .flat_map(|row| row[x0..x1].iter().copied())
            .collect()
    }
    let path = fixture_path_with_size("扫雷.lav", Some(33_229));
    let mut splash = load(&path);
    advance(&mut splash, 1200);
    assert_eq!(splash.vm().pc(), 0x8153);
    assert!(
        splash
            .display()
            .indexed_frame()
            .iter()
            .any(|&pixel| pixel != splash.display().indexed_frame()[0]),
        "the opening screen must render"
    );
    capture(&splash, "minesweeper-opening-screen");

    let mut menu = splash;
    tap(&mut menu, 13);
    assert_eq!(menu.vm().pc(), 0x7ce4);
    capture(&menu, "minesweeper-main-menu");

    let mut difficulty = menu.clone();
    tap(&mut difficulty, 13);
    assert_eq!(difficulty.vm().pc(), 0x7ec3);
    capture(&difficulty, "minesweeper-difficulty");

    let mut board = difficulty.clone();
    tap(&mut board, 13);
    assert_eq!(board.vm().pc(), 0x56cd);
    capture(&board, "minesweeper-board");

    let mut idle = board.clone();
    advance(&mut idle, 60);
    let idle_board = region(&idle, 0, 72, 0, 72);
    let mut directions = Vec::new();
    for (index, key) in [20, 21, 22, 23].into_iter().enumerate() {
        let mut moved = board.clone();
        tap(&mut moved, key);
        let board_image = region(&moved, 0, 72, 0, 72);
        assert_ne!(
            board_image, idle_board,
            "direction key {key} must move the selected cell"
        );
        directions.push(board_image);
        capture(&moved, &format!("minesweeper-direction-{index}"));
    }
    directions.sort();
    directions.dedup();
    assert_eq!(
        directions.len(),
        4,
        "direction keys must select distinct cells"
    );

    let mut revealed = board.clone();
    tap(&mut revealed, 13);
    assert_ne!(
        region(&revealed, 0, 72, 0, 72),
        idle_board,
        "Enter must reveal the selected cell"
    );
    capture(&revealed, "minesweeper-cell-revealed");

    let mut marked = board.clone();
    tap(&mut marked, 14);
    assert_ne!(
        region(&marked, 0, 72, 0, 72),
        idle_board,
        "the mark key must change the selected cell"
    );
    capture(&marked, "minesweeper-cell-marked");

    let mut unmarked = marked;
    tap(&mut unmarked, 14);
    let mut idle_after_two = board.clone();
    advance(&mut idle_after_two, 120);
    assert_eq!(
        region(&unmarked, 0, 72, 0, 72),
        region(&idle_after_two, 0, 72, 0, 72),
        "marking the selected cell twice must clear its marker"
    );
    capture(&unmarked, "minesweeper-cell-unmarked-after-toggle");

    let mut returned = board;
    tap(&mut returned, 27);
    assert_eq!(returned.vm().pc(), 0x7ec3);
    assert_ne!(
        region(&returned, 35, 125, 8, 72),
        region(&idle, 35, 125, 8, 72),
        "Escape must return to the difficulty menu"
    );
    capture(&returned, "minesweeper-back-to-difficulty");
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn wrist_wrestling_selects_a_fighter_and_responds_to_gameplay_input() {
    fn tap(e: &mut Emulator, key: u8) {
        e.input_mut().release_all();
        e.input_mut().set_keys([key]);
        advance(e, 1);
        e.input_mut().set_keys([]);
        advance(e, 59);
    }
    fn region(e: &Emulator, x0: usize, x1: usize, y0: usize, y1: usize) -> Vec<u8> {
        let width = usize::from(e.display().width());
        e.display()
            .indexed_frame()
            .chunks_exact(width)
            .skip(y0)
            .take(y1 - y0)
            .flat_map(|row| row[x0..x1].iter().copied())
            .collect()
    }

    let path = fixture_path_with_size("扳手腕.lav", Some(8_744));
    let mut splash = load(&path);
    advance(&mut splash, 1200);
    assert_eq!(splash.vm().pc(), 0x0d84);
    assert!(
        splash
            .display()
            .indexed_frame()
            .iter()
            .any(|&pixel| pixel != splash.display().indexed_frame()[0]),
        "the studio logo must render"
    );
    capture(&splash, "wrist-wrestling-splash");

    let mut menu = splash;
    tap(&mut menu, 13);
    advance(&mut menu, 1200);
    assert_eq!(menu.vm().pc(), 0x0edc);
    capture(&menu, "wrist-wrestling-mode-menu");

    let mut character = menu.clone();
    tap(&mut character, 13);
    assert_eq!(character.vm().pc(), 0x1371);
    capture(&character, "wrist-wrestling-character-selection");

    let mut character_idle = character.clone();
    advance(&mut character_idle, 60);
    let mut next_character = character.clone();
    tap(&mut next_character, 22);
    assert_ne!(
        region(&next_character, 10, 150, 8, 70),
        region(&character_idle, 10, 150, 8, 70),
        "the character selector must respond to a horizontal direction"
    );
    capture(&next_character, "wrist-wrestling-next-character");

    let mut previous_character = next_character;
    tap(&mut previous_character, 23);
    let mut character_after_two_inputs = character.clone();
    advance(&mut character_after_two_inputs, 120);
    assert_eq!(
        region(&previous_character, 10, 150, 8, 70),
        region(&character_after_two_inputs, 10, 150, 8, 70),
        "the opposite direction must return to the original fighter"
    );
    capture(&previous_character, "wrist-wrestling-previous-character");

    let mut difficulty = character;
    tap(&mut difficulty, 13);
    assert_eq!(difficulty.vm().pc(), 0x1531);
    capture(&difficulty, "wrist-wrestling-difficulty");

    let mut paused_at_start = difficulty;
    tap(&mut paused_at_start, 13);
    assert_eq!(paused_at_start.vm().pc(), 0x1860);
    capture(&paused_at_start, "wrist-wrestling-ready");

    let mut active = paused_at_start.clone();
    tap(&mut active, 32);
    assert_eq!(active.vm().pc(), 0x17d8);
    assert_ne!(
        region(&active, 50, 115, 24, 52),
        region(&paused_at_start, 50, 115, 24, 52),
        "Space must start the paused match"
    );
    capture(&active, "wrist-wrestling-active");

    let mut idle = active.clone();
    advance(&mut idle, 60);
    let mut paused = active.clone();
    tap(&mut paused, b'p');
    assert_eq!(paused.vm().pc(), 0x1860);
    assert_ne!(
        region(&paused, 50, 115, 24, 52),
        region(&idle, 50, 115, 24, 52),
        "P must display the pause state"
    );
    capture(&paused, "wrist-wrestling-paused");

    let mut resumed = paused;
    tap(&mut resumed, 32);
    assert_eq!(resumed.vm().pc(), 0x17d8);
    capture(&resumed, "wrist-wrestling-resumed");

    let mut mashing = active;
    for _ in 0..30 {
        mashing.input_mut().set_keys([13]);
        advance(&mut mashing, 1);
        mashing.input_mut().set_keys([]);
        advance(&mut mashing, 1);
    }
    assert_ne!(
        region(&mashing, 15, 112, 55, 72),
        region(&idle, 15, 112, 55, 72),
        "rapid Enter presses must move the visible strength bar"
    );
    capture(&mashing, "wrist-wrestling-enter-mashing");
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn spot_the_difference_finds_all_three_differences_and_advances() {
    fn word(emulator: &Emulator, address: usize) -> u16 {
        u16::from_le_bytes(
            emulator.vm().memory()[address..address + 2]
                .try_into()
                .unwrap(),
        )
    }
    fn region(emulator: &Emulator, x0: usize, x1: usize, y0: usize, y1: usize) -> Vec<u8> {
        let width = usize::from(emulator.display().width());
        emulator
            .display()
            .indexed_frame()
            .chunks_exact(width)
            .skip(y0)
            .take(y1 - y0)
            .flat_map(|row| row[x0..x1].iter().copied())
            .collect()
    }
    fn wait_for_input(emulator: &mut Emulator) {
        for _ in 0..1200 {
            let result = emulator
                .run_frame()
                .expect("game must execute without errors");
            match result.status {
                FrameStatus::WaitingForInput => return,
                FrameStatus::Halted(_) => panic!("game exited while waiting for input"),
                _ => {}
            }
        }
        panic!("the game did not wait for input");
    }
    fn step(emulator: &mut Emulator, horizontal: bool, target: u16) {
        let address = if horizontal { 0x26d2 } else { 0x26d4 };
        let before = word(emulator, address);
        assert_ne!(before, target, "the cursor is already at the target");
        let key = match (horizontal, before < target) {
            (true, true) => b'd',
            (true, false) => b'a',
            (false, true) => b's',
            (false, false) => b'w',
        };
        emulator.input_mut().set_keys([key]);
        for _ in 0..30 {
            advance(emulator, 1);
            if word(emulator, address) != before {
                break;
            }
        }
        emulator.input_mut().release_all();
        advance(emulator, 10);
        assert_ne!(
            word(emulator, address),
            before,
            "key {key} must move the cursor"
        );
    }
    fn goto(emulator: &mut Emulator, x: u16, y: u16) {
        let mut guard = 0;
        while word(emulator, 0x26d2) != x {
            step(emulator, true, x);
            guard += 1;
            assert!(guard < 200, "the horizontal cursor must reach {x}");
        }
        guard = 0;
        while word(emulator, 0x26d4) != y {
            step(emulator, false, y);
            guard += 1;
            assert!(guard < 200, "the vertical cursor must reach {y}");
        }
    }
    fn check(emulator: &mut Emulator, flag: usize) {
        emulator.input_mut().set_keys([13]);
        let mut found = false;
        for _ in 0..300 {
            advance(emulator, 1);
            if emulator.vm().memory()[flag] != 0 {
                found = true;
                break;
            }
            if emulator.vm().pc() == 0x92c {
                break;
            }
        }
        emulator.input_mut().release_all();
        assert!(found, "Enter over a difference must mark it");
        advance(emulator, 240);
    }

    for (path, name) in [
        ("LAVA1 找茬/大家来找茬.lav", "spot-lav"),
        ("LAVA1 找茬/找茬.pac", "spot-pac"),
    ] {
        let mut emulator = load(path);
        wait_for_input(&mut emulator);
        assert_eq!(
            emulator.vm().pc(),
            0x8a1,
            "{path} must show the level image"
        );
        capture(&emulator, &format!("{name}-level-image"));

        press(&mut emulator, 13);
        advance(&mut emulator, 540);
        assert_eq!(word(&emulator, 0x26d8), 1, "{path} must start on level 1");
        assert_eq!(
            (word(&emulator, 0x26d2), word(&emulator, 0x26d4)),
            (80, 40),
            "{path} must center the cursor"
        );
        let coordinates: Vec<u16> = (0..6).map(|i| word(&emulator, 0x26c0 + i * 2)).collect();
        assert_eq!(
            coordinates,
            [111, 100, 70, 25, 30, 78],
            "{path} must load the first level's differences"
        );
        assert!(
            emulator
                .display()
                .indexed_frame()
                .iter()
                .any(|&pixel| pixel != emulator.display().indexed_frame()[0]),
            "{path} must draw the comparison board"
        );
        capture(&emulator, &format!("{name}-board"));

        let start = region(&emulator, 70, 100, 30, 55);
        let mut idle = emulator.clone();
        advance(&mut idle, 60);
        step(&mut emulator, true, 82);
        assert_eq!(word(&emulator, 0x26d2), 82);
        assert_ne!(
            region(&emulator, 70, 100, 30, 55),
            start,
            "{path} cursor must move right on D"
        );
        step(&mut emulator, false, 42);
        assert_eq!(word(&emulator, 0x26d4), 42);
        assert_ne!(
            region(&emulator, 70, 100, 30, 55),
            region(&idle, 70, 100, 30, 55),
            "{path} cursor must move down on S"
        );
        capture(&emulator, &format!("{name}-moved"));

        for (label, x, y, flag) in [
            ("pair1", 112, 26, 0x26ccusize),
            ("pair2", 100, 30, 0x26ce),
            ("pair3", 70, 78, 0x26d0),
        ] {
            goto(&mut emulator, x, y);
            check(&mut emulator, flag);
            assert_ne!(
                emulator.display().indexed_frame(),
                idle.display().indexed_frame(),
                "{path} must mark difference {label}"
            );
            capture(&emulator, &format!("{name}-{label}"));
        }

        wait_for_input(&mut emulator);
        assert_eq!(emulator.vm().pc(), 0x8a1, "{path} must show the next level");
        assert_eq!(word(&emulator, 0x26d8), 2, "{path} must advance to level 2");
        let coordinates: Vec<u16> = (0..6).map(|i| word(&emulator, 0x26c0 + i * 2)).collect();
        assert_eq!(
            coordinates,
            [5, 60, 105, 62, 61, 61],
            "{path} must load the second level's differences"
        );
        assert_eq!(
            emulator.vm().memory()[0x26cc..0x26d2],
            [0, 0, 0, 0, 0, 0],
            "{path} must reset the found flags"
        );
        assert_eq!(word(&emulator, 0x26d6), 0, "{path} must reset the timer");
        capture(&emulator, &format!("{name}-level2"));
    }
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn battleship_advances_menus_and_battle_and_returns_after_game_over() {
    fn word(emulator: &Emulator, address: usize) -> u16 {
        u16::from_le_bytes(
            emulator.vm().memory()[address..address + 2]
                .try_into()
                .unwrap(),
        )
    }
    fn score(emulator: &Emulator) -> u32 {
        u32::from_le_bytes(emulator.vm().memory()[0x3380..0x3384].try_into().unwrap())
    }
    fn wait_for_input(emulator: &mut Emulator) -> usize {
        for _ in 0..2400 {
            let result = emulator
                .run_frame()
                .expect("game must execute without errors");
            match result.status {
                FrameStatus::WaitingForInput => return emulator.vm().pc(),
                FrameStatus::Halted(_) => panic!("game exited while waiting for input"),
                _ => {}
            }
        }
        panic!("the game did not wait for input");
    }
    for (path, name) in [
        ("LAVA1 护卫战舰/护卫战舰.pac", "battleship-pac"),
        ("LAVA1 护卫战舰/非PAC/护卫战舰.lav", "battleship-lav"),
    ] {
        let mut emulator = load(path);
        assert_eq!(wait_for_input(&mut emulator), 0x16b, "{path} intro screen");
        capture(&emulator, &format!("{name}-intro"));
        for _ in 0..4 {
            press(&mut emulator, 13);
            assert_eq!(wait_for_input(&mut emulator), 0x1ec, "{path} splash chain");
        }
        press(&mut emulator, 13);
        assert_eq!(wait_for_input(&mut emulator), 0xb26, "{path} main menu");
        let menu = emulator.display().indexed_frame().to_vec();
        capture(&emulator, &format!("{name}-menu"));
        press(&mut emulator, 21);
        assert_ne!(
            emulator.display().indexed_frame(),
            menu.as_slice(),
            "{path} down must move the menu cursor"
        );
        press(&mut emulator, 20);
        assert_eq!(
            emulator.display().indexed_frame(),
            menu.as_slice(),
            "{path} up must restore the menu cursor"
        );
        press(&mut emulator, 13);
        assert_eq!(
            wait_for_input(&mut emulator),
            0x1aa9,
            "{path} must open the level briefing"
        );
        capture(&emulator, &format!("{name}-briefing"));
        press(&mut emulator, 27);
        assert_eq!(
            wait_for_input(&mut emulator),
            0xb26,
            "{path} escape must cancel the briefing"
        );
        press(&mut emulator, 13);
        assert_eq!(wait_for_input(&mut emulator), 0x1aa9);
        press(&mut emulator, 13);
        let mut started = false;
        for _ in 0..600 {
            advance(&mut emulator, 1);
            if emulator.vm().pc() == 0x2b14
                && word(&emulator, 0x2fd0) == 0
                && word(&emulator, 0x2fca) > 0
            {
                started = true;
                break;
            }
        }
        assert!(started, "{path} must enter the battle loop");
        assert_eq!(word(&emulator, 0x2fd2), 65, "{path} initial ship position");
        capture(&emulator, &format!("{name}-battle"));
        emulator.input_mut().set_keys(*b"z");
        advance(&mut emulator, 1);
        emulator.input_mut().release_all();
        advance(&mut emulator, 2);
        assert_eq!(word(&emulator, 0x2fbc), 2, "{path} fire must consume ammo");
        assert_eq!(word(&emulator, 0x2fdc), 1, "{path} fire must spawn a shell");
        assert_eq!(
            word(&emulator, 0x2ff0),
            65,
            "{path} shell must launch from the ship"
        );
        capture(&emulator, &format!("{name}-shot"));
        advance(&mut emulator, 20);
        assert_eq!(score(&emulator), 50, "{path} the shot must destroy a ship");
        assert_eq!(word(&emulator, 0x2fdc), 0, "{path} the shell must be spent");
        emulator.input_mut().set_keys([22]);
        advance(&mut emulator, 3);
        emulator.input_mut().release_all();
        advance(&mut emulator, 1);
        assert_eq!(
            word(&emulator, 0x2fd2),
            86,
            "{path} right must move the ship"
        );
        capture(&emulator, &format!("{name}-right"));
        emulator.input_mut().set_keys([23]);
        advance(&mut emulator, 2);
        emulator.input_mut().release_all();
        advance(&mut emulator, 1);
        assert_eq!(
            word(&emulator, 0x2fd2),
            72,
            "{path} left must move the ship"
        );
        capture(&emulator, &format!("{name}-left"));
        let mut game_over = false;
        for _ in 0..600 {
            let result = emulator
                .run_frame()
                .expect("game must execute without errors");
            if result.status == FrameStatus::WaitingForInput {
                game_over = true;
                break;
            }
            assert!(
                !matches!(result.status, FrameStatus::Halted(_)),
                "{path} game exited before the game-over screen"
            );
        }
        assert!(game_over, "{path} must reach the game-over screen");
        assert_eq!(emulator.vm().pc(), 0x197f, "{path} game-over wait");
        assert_eq!(word(&emulator, 0x2fb8), 0, "{path} all ships must be lost");
        assert_eq!(score(&emulator), 50, "{path} the score must be preserved");
        capture(&emulator, &format!("{name}-game-over"));
        press(&mut emulator, 27);
        assert_eq!(
            wait_for_input(&mut emulator),
            0xb26,
            "{path} escape must return to the main menu"
        );
        capture(&emulator, &format!("{name}-back-to-menu"));
    }
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn boxing_resolves_punches_and_finishes_a_match() {
    fn byte(emulator: &Emulator, address: usize) -> u8 {
        emulator.vm().memory()[address]
    }
    fn wait_for_input(emulator: &mut Emulator) -> usize {
        for _ in 0..2400 {
            let result = emulator
                .run_frame()
                .expect("game must execute without errors");
            match result.status {
                FrameStatus::WaitingForInput => return emulator.vm().pc(),
                FrameStatus::Halted(_) => panic!("game exited while waiting for input"),
                _ => {}
            }
        }
        panic!("the game did not wait for input");
    }
    fn tap(emulator: &mut Emulator, key: u8) {
        emulator.input_mut().set_keys([key]);
        advance(emulator, 1);
        emulator.input_mut().release_all();
    }
    let mut emulator = load("LAVA1 拳击/boxing.lav");
    assert_eq!(wait_for_input(&mut emulator), 0x40f1, "title screen");
    capture(&emulator, "boxing-title");
    tap(&mut emulator, 13);
    assert_eq!(wait_for_input(&mut emulator), 0x40c0, "key guide");
    capture(&emulator, "boxing-keys");
    tap(&mut emulator, 13);
    assert_eq!(wait_for_input(&mut emulator), 0x3bef, "fight loop");
    assert_eq!(byte(&emulator, 0x2008), 52, "player stamina");
    assert_eq!(byte(&emulator, 0x200b), 52, "enemy stamina");
    assert_eq!(byte(&emulator, 0x2007), 21, "first enemy move");
    capture(&emulator, "boxing-fight");
    let moves = [21u8, 112, 23, 105, 105, 21, 112, 112, 112, 22, 23, 105, 105];
    let stamina = [
        (52u8, 52u8),
        (50, 47),
        (50, 47),
        (48, 42),
        (48, 42),
        (48, 42),
        (46, 37),
        (46, 32),
        (40, 17),
        (40, 17),
        (40, 17),
        (34, 2),
        (34, 0),
    ];
    let keys = [23u8, 105, 21, 112, 22];
    for round in 0..12 {
        tap(&mut emulator, keys[round % keys.len()]);
        assert_eq!(
            wait_for_input(&mut emulator),
            0x3bef,
            "round {round} must stay in the fight"
        );
        assert_eq!(
            byte(&emulator, 0x2007),
            moves[round],
            "round {round} enemy move"
        );
        assert_eq!(
            (byte(&emulator, 0x2008), byte(&emulator, 0x200b)),
            stamina[round],
            "round {round} stamina"
        );
        if round == 8 {
            capture(&emulator, "boxing-exchange");
        }
    }
    tap(&mut emulator, keys[12 % keys.len()]);
    advance(&mut emulator, 40);
    capture(&emulator, "boxing-knockout");
    assert_eq!(
        wait_for_input(&mut emulator),
        0x40f1,
        "a knockout must return to the title"
    );
    assert_eq!(byte(&emulator, 0x200b), 0, "enemy stamina after knockout");
    capture(&emulator, "boxing-after-knockout");
    tap(&mut emulator, 13);
    assert_eq!(wait_for_input(&mut emulator), 0x40c0);
    tap(&mut emulator, 13);
    assert_eq!(wait_for_input(&mut emulator), 0x3bef, "rematch");
    assert_eq!(byte(&emulator, 0x2008), 52, "rematch player stamina");
    assert_eq!(byte(&emulator, 0x200b), 52, "rematch enemy stamina");
    capture(&emulator, "boxing-rematch");
    tap(&mut emulator, 27);
    assert_eq!(
        wait_for_input(&mut emulator),
        0x40f1,
        "escape must leave the fight"
    );
    capture(&emulator, "boxing-escape");
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn matchstick_game_takes_turns_and_restarts_or_quits() {
    fn word(emulator: &Emulator, address: usize) -> u16 {
        u16::from_le_bytes(
            emulator.vm().memory()[address..address + 2]
                .try_into()
                .unwrap(),
        )
    }
    fn wait_for_input(emulator: &mut Emulator) -> usize {
        for _ in 0..2400 {
            match emulator.run_frame().unwrap().status {
                FrameStatus::WaitingForInput => return emulator.vm().pc(),
                FrameStatus::Halted(_) => panic!("program halted while waiting for input"),
                _ => {}
            }
        }
        panic!("no input prompt within 2400 frames");
    }
    fn tap(emulator: &mut Emulator, key: u8) {
        emulator.input_mut().release_all();
        emulator.input_mut().set_keys([key]);
        emulator.run_frame().unwrap();
        emulator.input_mut().release_all();
    }
    fn take_one(emulator: &mut Emulator) {
        tap(emulator, b'1');
        tap(emulator, 13);
    }

    let mut emulator = load("LAVA1 拿火柴/拿火柴.lav");
    assert_eq!(wait_for_input(&mut emulator), 0xee4, "rules screen");
    capture(&emulator, "matchstick-rules");

    tap(&mut emulator, 13);
    assert_eq!(wait_for_input(&mut emulator), 0xd31, "number prompt");
    assert_eq!(word(&emulator, 0x2c8b), 186, "initial matches");
    assert_eq!(word(&emulator, 0x2c8d), 10, "maximum take");
    capture(&emulator, "matchstick-prompt");

    tap(&mut emulator, b'1');
    tap(&mut emulator, b'1');
    tap(&mut emulator, 13);
    assert_eq!(wait_for_input(&mut emulator), 0xff4, "input error screen");
    assert_eq!(word(&emulator, 0x2c8b), 186, "rejected take keeps matches");
    capture(&emulator, "matchstick-invalid");
    tap(&mut emulator, 13);
    assert_eq!(wait_for_input(&mut emulator), 0xd31);

    take_one(&mut emulator);
    assert_eq!(wait_for_input(&mut emulator), 0xd31);
    assert_eq!(word(&emulator, 0x2c8b), 176, "after first exchange");
    assert_eq!(word(&emulator, 0x2c8f), 9, "computer takes the remainder");

    for expected in [
        165, 154, 143, 132, 121, 110, 99, 88, 77, 66, 55, 44, 33, 22, 11,
    ] {
        take_one(&mut emulator);
        assert_eq!(wait_for_input(&mut emulator), 0xd31);
        assert_eq!(word(&emulator, 0x2c8b), expected);
        assert_eq!(word(&emulator, 0x2c8f), 10);
    }

    take_one(&mut emulator);
    assert_eq!(
        wait_for_input(&mut emulator),
        0x1142,
        "computer takes the last match"
    );
    assert_eq!(word(&emulator, 0x2c8b), 0);
    capture(&emulator, "matchstick-loss");

    tap(&mut emulator, b'y');
    assert_eq!(wait_for_input(&mut emulator), 0xd31, "rematch prompt");
    assert_eq!(word(&emulator, 0x2c8b), 156);
    assert_eq!(word(&emulator, 0x2c8d), 17);
    capture(&emulator, "matchstick-rematch");

    for _ in 0..10 {
        take_one(&mut emulator);
        let pc = wait_for_input(&mut emulator);
        if word(&emulator, 0x2c8b) == 0 {
            assert_eq!(pc, 0x1142);
            break;
        }
        assert_eq!(pc, 0xd31);
    }
    assert_eq!(word(&emulator, 0x2c8b), 0, "second game reaches zero");
    capture(&emulator, "matchstick-loss-again");

    tap(&mut emulator, b'n');
    assert_eq!(
        wait_for_input(&mut emulator),
        0xd31,
        "after a rematch, N is ignored and another game starts"
    );
    assert_eq!(word(&emulator, 0x2c8b), 155);
    assert_eq!(word(&emulator, 0x2c8d), 15);
    capture(&emulator, "matchstick-answer-n-restarts");

    tap(&mut emulator, 27);
    for _ in 0..600 {
        if !emulator.vm().is_running() {
            break;
        }
        emulator.run_frame().unwrap();
    }
    assert!(!emulator.vm().is_running(), "escape must exit the game");

    let mut quitting = load("LAVA1 拿火柴/拿火柴.lav");
    assert_eq!(wait_for_input(&mut quitting), 0xee4);
    tap(&mut quitting, 13);
    assert_eq!(wait_for_input(&mut quitting), 0xd31);
    for _ in 0..16 {
        take_one(&mut quitting);
        assert_eq!(wait_for_input(&mut quitting), 0xd31);
    }
    take_one(&mut quitting);
    assert_eq!(wait_for_input(&mut quitting), 0x1142);
    tap(&mut quitting, b'n');
    for _ in 0..600 {
        if !quitting.vm().is_running() {
            break;
        }
        quitting.run_frame().unwrap();
    }
    assert!(
        !quitting.vm().is_running(),
        "n must quit after the first game"
    );
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn pipe_game_places_pipes_counts_down_pauses_and_quits() {
    fn byte(emulator: &Emulator, address: usize) -> u8 {
        emulator.vm().memory()[address]
    }
    fn cursor(emulator: &Emulator) -> (usize, usize) {
        (
            byte(emulator, 0x3434).wrapping_add(byte(emulator, 0x3432)) as usize,
            byte(emulator, 0x3435).wrapping_add(byte(emulator, 0x3433)) as usize,
        )
    }
    fn cell(emulator: &Emulator) -> usize {
        let (x, y) = cursor(emulator);
        (x - 1) * 36 + (y - 1)
    }
    fn seconds(emulator: &Emulator) -> u8 {
        byte(emulator, 0x342a)
    }
    fn tap(emulator: &mut Emulator, key: u8) {
        emulator.input_mut().set_keys([key]);
        advance(emulator, 2);
        emulator.input_mut().release_all();
        advance(emulator, 20);
    }
    fn hold(emulator: &mut Emulator, key: u8, frames: usize) {
        for _ in 0..frames {
            emulator.input_mut().set_keys([key]);
            advance(emulator, 1);
        }
        emulator.input_mut().release_all();
        advance(emulator, 30);
    }
    let mut emulator = load("LAVA1 接水管/接水管.lav");
    advance(&mut emulator, 30);
    assert_eq!(emulator.vm().pc(), 0x10ed, "title must wait for a key");
    capture(&emulator, "pipe-title");
    tap(&mut emulator, 13);
    advance(&mut emulator, 100);
    assert_eq!(
        emulator.vm().pc(),
        0x2ff4,
        "start screen must wait before the round begins"
    );
    capture(&emulator, "pipe-start-screen");
    for _ in 0..300 {
        if emulator.vm().pc() == 0x2a75 {
            break;
        }
        advance(&mut emulator, 1);
    }
    assert_eq!(emulator.vm().pc(), 0x2a75, "the round starts automatically");
    assert_eq!(cursor(&emulator), (19, 25), "cursor starts on the source");
    assert_eq!(cell(&emulator), 672, "cursor starts on the source cell");
    assert_eq!(byte(&emulator, 0x2f08 + 672), 14, "water source cell");
    assert_eq!(seconds(&emulator), 180, "three-minute time limit");
    capture(&emulator, "pipe-play");
    advance(&mut emulator, 150);
    assert!(
        seconds(&emulator) < 180,
        "the clock must count down during play"
    );
    tap(&mut emulator, 23);
    assert_eq!(cursor(&emulator), (18, 25), "one key narrows the column");
    tap(&mut emulator, 22);
    assert_eq!(cursor(&emulator), (19, 25), "the opposite key restores it");
    tap(&mut emulator, 21);
    assert_eq!(cursor(&emulator), (19, 26), "one key advances the row");
    tap(&mut emulator, 20);
    assert_eq!(cursor(&emulator), (19, 25), "the opposite key restores it");
    tap(&mut emulator, 23);
    assert_eq!(cell(&emulator), 636, "cursor moves onto an empty cell");
    tap(&mut emulator, b'a');
    assert_eq!(byte(&emulator, 0x2f08 + 636), 6, "A places a pipe");
    capture(&emulator, "pipe-placed");
    tap(&mut emulator, b'a');
    assert_eq!(
        byte(&emulator, 0x2f08 + 636),
        6,
        "A must not overwrite an existing pipe"
    );
    tap(&mut emulator, b's');
    assert_eq!(
        byte(&emulator, 0x2f08 + 636),
        0,
        "S removes the pipe under the cursor"
    );
    assert_eq!(byte(&emulator, 0x2f08 + 672), 14, "the source is untouched");
    capture(&emulator, "pipe-removed");
    hold(&mut emulator, b'p', 40);
    let paused = seconds(&emulator);
    capture(&emulator, "pipe-paused");
    advance(&mut emulator, 180);
    assert_eq!(
        seconds(&emulator),
        paused,
        "the clock must stop while paused"
    );
    hold(&mut emulator, b'p', 45);
    advance(&mut emulator, 120);
    assert!(
        seconds(&emulator) < paused,
        "P must resume the countdown after the pause loop polls again"
    );
    hold(&mut emulator, 27, 60);
    advance(&mut emulator, 90);
    assert_eq!(
        byte(&emulator, 0x3444),
        0,
        "escape returns to the title menu"
    );
    capture(&emulator, "pipe-title-again");
    for index in 1..=4u8 {
        for _ in 0..3 {
            emulator.input_mut().set_keys([21]);
            advance(&mut emulator, 1);
        }
        emulator.input_mut().release_all();
        advance(&mut emulator, 30);
        assert_eq!(byte(&emulator, 0x3444), index, "menu selection moves down");
    }
    tap(&mut emulator, 13);
    for _ in 0..600 {
        if emulator.vm().pc() == 0x2f17 {
            break;
        }
        advance(&mut emulator, 1);
    }
    assert_eq!(emulator.vm().pc(), 0x2f17, "credits must wait for a key");
    capture(&emulator, "pipe-credits");
    emulator.input_mut().set_keys([13]);
    for _ in 0..120 {
        let result = emulator
            .run_frame()
            .expect("game must execute without errors");
        if matches!(result.status, FrameStatus::Halted(_)) {
            break;
        }
    }
    emulator.input_mut().release_all();
    assert!(
        !emulator.vm().is_running(),
        "a key at the credits must leave the game"
    );
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn sokoban_slides_boxes_pairs_glyphs_and_advances_levels() {
    fn word(emulator: &Emulator, address: usize) -> u16 {
        let memory = emulator.vm().memory();
        u16::from_le_bytes([memory[address], memory[address + 1]])
    }
    fn glyph(emulator: &Emulator, x: usize, y: usize) -> u8 {
        emulator.vm().memory()[0x2010 + x * 11 + y]
    }
    fn player(emulator: &Emulator) -> (u16, u16) {
        (word(emulator, 0x2000), word(emulator, 0x2002))
    }
    fn tap(emulator: &mut Emulator, key: u8) {
        emulator.input_mut().set_keys([key]);
        advance(emulator, 2);
        emulator.input_mut().release_all();
        advance(emulator, 30);
    }
    fn route(emulator: &mut Emulator, keys: &[u8]) {
        for &key in keys {
            tap(emulator, key);
        }
    }
    for path in [
        "LAVA1 推箱达人/推箱达人.pac",
        "LAVA1 推箱达人/未打包的/推箱达人.lav",
    ] {
        let mut emulator = load(path);
        advance(&mut emulator, 90);
        assert_eq!(emulator.vm().pc(), 0x491, "the board must wait for a key");
        assert_eq!(player(&emulator), (5, 5), "level 1 starts in the centre");
        assert_eq!(word(&emulator, 0x2004), 80, "pixel x = 45 + 7 * 5");
        assert_eq!(word(&emulator, 0x2006), 36, "pixel y = 1 + 7 * 5");
        assert_eq!(word(&emulator, 0x2008), 45, "tile origin x");
        assert_eq!(word(&emulator, 0x200a), 1, "tile origin y");
        assert_eq!(word(&emulator, 0x200c), 2, "level 1 counts two glyphs");
        assert_eq!(word(&emulator, 0x200e), 21, "the data holds 21 levels");
        assert_eq!(glyph(&emulator, 2, 5), 2);
        assert_eq!(glyph(&emulator, 8, 4), 2);
        capture(&emulator, "sokoban-level1");

        tap(&mut emulator, 23);
        assert_eq!(player(&emulator), (4, 5), "left moves one tile");
        tap(&mut emulator, 22);
        assert_eq!(player(&emulator), (5, 5), "right moves one tile");
        tap(&mut emulator, 20);
        assert_eq!(player(&emulator), (5, 4), "up moves one tile");
        tap(&mut emulator, 20);
        assert_eq!(player(&emulator), (5, 4), "the wall row blocks movement");
        tap(&mut emulator, 21);
        assert_eq!(player(&emulator), (5, 5));

        route(&mut emulator, &[21, 23, 23, 23, 20]);
        assert_eq!(
            glyph(&emulator, 2, 4),
            2,
            "the free glyph slides to the wall"
        );
        assert_eq!(glyph(&emulator, 2, 5), 0);
        assert_eq!(word(&emulator, 0x200c), 2, "a slide alone does not score");
        capture(&emulator, "sokoban-pushed");
        route(&mut emulator, &[22, 22, 22, 22, 22, 21, 22, 22, 20, 20, 23]);
        assert_eq!(word(&emulator, 0x200c), 0, "pairing removes both glyphs");
        assert_eq!(glyph(&emulator, 2, 4), 0);
        assert_eq!(glyph(&emulator, 3, 4), 0);
        assert_eq!(emulator.vm().pc(), 0x8c0, "All Clean waits for a key");
        capture(&emulator, "sokoban-clean");

        tap(&mut emulator, 13);
        assert_eq!(word(&emulator, 0x200c), 2, "level 2 loads its own count");
        assert_eq!(player(&emulator), (5, 5));
        assert_eq!(glyph(&emulator, 2, 5), 9);
        assert_eq!(glyph(&emulator, 8, 5), 9);
        capture(&emulator, "sokoban-level2");

        route(&mut emulator, &[23, 23, 23]);
        assert_eq!(
            glyph(&emulator, 1, 5),
            9,
            "the left glyph stops at the wall"
        );
        route(&mut emulator, &[22, 22, 22, 22, 22, 21, 22, 22, 20]);
        assert_eq!(player(&emulator), (9, 5));
        tap(&mut emulator, 23);
        assert_eq!(
            glyph(&emulator, 7, 5),
            9,
            "nine-glyphs move one tile per push"
        );
        assert_eq!(glyph(&emulator, 2, 5), 0, "nine-glyphs do not slide ahead");
        assert_eq!(player(&emulator), (8, 5));
        assert_eq!(word(&emulator, 0x200c), 2);
        capture(&emulator, "sokoban-single-step");
        route(&mut emulator, &[23, 23, 23, 23, 23]);
        assert_eq!(word(&emulator, 0x200c), 0, "the twin glyphs annihilate");
        assert_eq!(glyph(&emulator, 1, 5), 0);
        assert_eq!(glyph(&emulator, 2, 5), 0);
        assert_eq!(emulator.vm().pc(), 0x8c0);
        capture(&emulator, "sokoban-clean-again");

        tap(&mut emulator, 13);
        assert_eq!(word(&emulator, 0x200c), 12);
        assert_eq!(player(&emulator), (6, 5));
        assert_eq!(glyph(&emulator, 2, 2), 2);
        assert_eq!(glyph(&emulator, 1, 8), 2);
        assert_eq!(glyph(&emulator, 3, 3), 3);
        assert_eq!(glyph(&emulator, 2, 6), 9);
        assert_eq!(glyph(&emulator, 8, 6), 9);
        capture(&emulator, "sokoban-level3");

        tap(&mut emulator, 110);
        assert_eq!(word(&emulator, 0x200c), 2, "N loads the next level");
        assert_eq!(player(&emulator), (5, 2));
        assert_eq!(glyph(&emulator, 6, 5), 2);
        assert_eq!(glyph(&emulator, 8, 2), 2);
        capture(&emulator, "sokoban-level4");

        route(&mut emulator, &[22, 22, 22]);
        assert_eq!(player(&emulator), (8, 2));
        assert_eq!(
            glyph(&emulator, 9, 2),
            2,
            "the glyph slides before the wall"
        );
        assert_eq!(glyph(&emulator, 8, 2), 0);
        tap(&mut emulator, 28);
        assert_eq!(player(&emulator), (5, 2), "F1 replays the current level");
        assert_eq!(glyph(&emulator, 8, 2), 2);
        assert_eq!(glyph(&emulator, 9, 2), 0);
        assert_eq!(word(&emulator, 0x200c), 2);
        capture(&emulator, "sokoban-replayed");

        emulator.input_mut().set_keys([27]);
        for _ in 0..120 {
            let result = emulator
                .run_frame()
                .expect("game must execute without errors");
            if matches!(result.status, FrameStatus::Halted(_)) {
                break;
            }
        }
        emulator.input_mut().release_all();
        assert!(!emulator.vm().is_running(), "escape leaves the game");
    }
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn porter_pushes_crates_uses_a_bomb_and_restores_board_state() {
    fn cell(emulator: &Emulator, x: usize, y: usize) -> u8 {
        emulator.vm().memory()[0x2050 + x * 12 + y]
    }
    fn player(emulator: &Emulator) -> (u8, u8) {
        let memory = emulator.vm().memory();
        (memory[0x20f6] - 6, memory[0x20f7] - 2)
    }
    fn tap(emulator: &mut Emulator, key: u8) {
        emulator.input_mut().set_keys([key]);
        advance(emulator, 3);
        emulator.input_mut().release_all();
        advance(emulator, 45);
    }
    let mut emulator = load("LAVA1 搬运工/搬运工.lav");
    advance(&mut emulator, 300);
    tap(&mut emulator, 13);
    assert_eq!(
        emulator.vm().pc(),
        0x2f1,
        "Enter opens the level confirmation"
    );
    tap(&mut emulator, 121);
    advance(&mut emulator, 60);
    assert_eq!(emulator.vm().pc(), 0x985, "Y starts the first level");
    assert_eq!(player(&emulator), (7, 1));
    assert_eq!(cell(&emulator, 1, 3), 4, "the goal column has three pads");
    assert_eq!(cell(&emulator, 1, 4), 4);
    assert_eq!(cell(&emulator, 1, 5), 4);
    assert_eq!(cell(&emulator, 5, 2), 5);
    assert_eq!(cell(&emulator, 6, 2), 5);
    assert_eq!(cell(&emulator, 6, 3), 5);
    assert_eq!(
        cell(&emulator, 6, 4),
        2,
        "indestructible walls guard the corridor"
    );
    assert_eq!(cell(&emulator, 4, 7), 7, "a bomb lies in the lower pocket");
    assert_eq!(emulator.vm().memory()[0x20fe], 0);
    assert_eq!(emulator.vm().memory()[0x20ff], 0);
    capture(&emulator, "porter-level1");

    tap(&mut emulator, 22);
    assert_eq!(
        player(&emulator),
        (7, 1),
        "the outer wall blocks the corridor"
    );
    tap(&mut emulator, 23);
    tap(&mut emulator, 23);
    assert_eq!(player(&emulator), (5, 1));
    tap(&mut emulator, 21);
    assert_eq!(player(&emulator), (5, 2), "the crate yields one tile");
    assert_eq!(cell(&emulator, 5, 2), 0);
    assert_eq!(cell(&emulator, 5, 3), 5);
    capture(&emulator, "porter-pushed");

    tap(&mut emulator, 104);
    assert_eq!(player(&emulator), (5, 1), "H undoes the last push");
    assert_eq!(cell(&emulator, 5, 2), 5);
    assert_eq!(cell(&emulator, 5, 3), 0);
    tap(&mut emulator, 21);
    tap(&mut emulator, 114);
    assert_eq!(player(&emulator), (7, 1), "R restarts the level");
    assert_eq!(cell(&emulator, 5, 2), 5);
    assert_eq!(cell(&emulator, 5, 3), 0);
    assert_eq!(cell(&emulator, 6, 2), 5);
    assert_eq!(cell(&emulator, 6, 3), 5);
    capture(&emulator, "porter-restarted");

    for key in [
        23, 23, 21, 21, 21, 21, 21, 23, 21, 20, 22, 20, 20, 20, 20, 20, 22, 22, 21, 21, 23, 20, 23,
        21, 21,
    ] {
        tap(&mut emulator, key);
    }
    assert_eq!(
        player(&emulator),
        (5, 4),
        "the crate reaches the wall pocket"
    );
    assert_eq!(cell(&emulator, 5, 5), 5);
    assert_eq!(
        cell(&emulator, 4, 7),
        0,
        "walking over the pickup collects it"
    );
    assert_eq!(
        emulator.vm().memory()[0x20fe],
        1,
        "the level grants one bomb"
    );
    assert_eq!(emulator.vm().memory()[0x20ff], 1);

    tap(&mut emulator, 98);
    assert_eq!(emulator.vm().pc(), 0xa93, "B enters bomb aiming");
    tap(&mut emulator, 22);
    tap(&mut emulator, 21);
    assert_eq!(
        (
            emulator.vm().memory()[0x20f8],
            emulator.vm().memory()[0x20f9]
        ),
        (12, 7),
        "arrows move the aim cursor across cells"
    );
    capture(&emulator, "porter-aim");
    tap(&mut emulator, 13);
    assert_eq!(
        emulator.vm().pc(),
        0x985,
        "Enter fires and returns to the board"
    );
    assert_eq!(cell(&emulator, 6, 5), 0, "the bomb clears the inner wall");
    assert_eq!(emulator.vm().memory()[0x20ff], 0, "the bomb is spent");
    assert_eq!(emulator.vm().memory()[0x2101], 1);

    for key in [
        21, 22, 21, 21, 23, 20, 22, 20, 23, 23, 23, 23, 21, 23, 20, 20,
    ] {
        tap(&mut emulator, key);
    }
    assert_eq!(player(&emulator), (1, 4));
    assert_eq!(cell(&emulator, 1, 3), 6, "a crate lands on the goal pad");
    assert_eq!(cell(&emulator, 1, 4), 4);
    assert_eq!(cell(&emulator, 1, 5), 4);
    assert_eq!(
        cell(&emulator, 6, 1),
        5,
        "the middle crate waits above its slot"
    );
    assert_eq!(cell(&emulator, 6, 2), 0);
    assert_eq!(cell(&emulator, 6, 3), 0);
    assert_eq!(
        cell(&emulator, 4, 7),
        5,
        "the last crate rests in the pocket"
    );
    capture(&emulator, "porter-filled");

    emulator.input_mut().set_keys([111]);
    for _ in 0..3 {
        emulator.run_frame().expect("the exit prompt must execute");
    }
    emulator.input_mut().release_all();
    let mut halted = false;
    for _ in 0..400 {
        let result = emulator.run_frame().expect("the exit fade must execute");
        if matches!(result.status, FrameStatus::Halted(_)) {
            halted = true;
            break;
        }
    }
    assert!(halted, "O fades out and leaves the game");
    assert!(!emulator.vm().is_running());
    capture(&emulator, "porter-exit");
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn billiards_racks_aims_fires_and_quits_from_the_title() {
    fn word(emulator: &Emulator, address: usize) -> u16 {
        let memory = emulator.vm().memory();
        u16::from_le_bytes([memory[address], memory[address + 1]])
    }
    fn tap(emulator: &mut Emulator, key: u8) {
        emulator.input_mut().set_keys([key]);
        advance(emulator, 3);
        emulator.input_mut().release_all();
        advance(emulator, 120);
    }
    let mut emulator = load("LAVA1 撞球高手/撞球高手.lav");
    advance(&mut emulator, 360);
    tap(&mut emulator, 13);
    assert_eq!(
        emulator.vm().pc(),
        0x25f8,
        "the level banner waits for a key"
    );
    assert_eq!(emulator.vm().memory()[0x20a4], 11);
    capture(&emulator, "billiards-banner");

    tap(&mut emulator, 13);
    for (index, (x, y)) in [
        (0x1064, 0x134b),
        (0x2580, 0x2361),
        (0x6e0a, 0x3bfb),
        (0x551f, 0x1394),
        (0x20e3, 0x21bb),
        (0x2929, 0x2fdc),
        (0x1e6a, 0x179d),
    ]
    .iter()
    .enumerate()
    {
        assert_eq!(word(&emulator, 0x2000 + index * 8), *x);
        assert_eq!(word(&emulator, 0x2004 + index * 8), *y);
    }
    assert_eq!(word(&emulator, 0x20b0), 149, "the shot clock starts full");
    assert_eq!(
        word(&emulator, 0x20b6),
        16,
        "the aim starts on the cue ball"
    );
    assert_eq!(word(&emulator, 0x20b8), 19);
    assert_eq!(
        emulator.vm().memory()[0x20c5],
        b'0',
        "the score starts at zero"
    );
    capture(&emulator, "billiards-rack");

    tap(&mut emulator, 22);
    assert_eq!(word(&emulator, 0x20b6), 18, "right turns the aim");
    tap(&mut emulator, 23);
    assert_eq!(word(&emulator, 0x20b6), 16);
    tap(&mut emulator, 20);
    assert_eq!(word(&emulator, 0x20b8), 17, "up raises the aim");
    tap(&mut emulator, 21);
    assert_eq!(word(&emulator, 0x20b8), 19);
    tap(&mut emulator, b'd');
    assert_eq!(word(&emulator, 0x20b6), 18, "D mirrors the right arrow");
    tap(&mut emulator, b'a');
    assert_eq!(word(&emulator, 0x20b6), 16);
    tap(&mut emulator, b'w');
    assert_eq!(word(&emulator, 0x20b8), 17, "W mirrors the up arrow");
    tap(&mut emulator, b's');
    assert_eq!(word(&emulator, 0x20b8), 19);
    capture(&emulator, "billiards-aim");

    tap(&mut emulator, 22);
    tap(&mut emulator, 22);
    assert_eq!(word(&emulator, 0x20b6), 20);
    let clock = word(&emulator, 0x20b0);
    tap(&mut emulator, 32);
    assert_eq!(
        (word(&emulator, 0x2000), word(&emulator, 0x2004)),
        (0x1564, 0x134b),
        "the cue ball rolls five pixels to the right"
    );
    assert_eq!(word(&emulator, 0x2098), 0x3ebb);
    assert_eq!(emulator.vm().memory()[0x209c], 9, "the shot spends power");
    assert_eq!(
        word(&emulator, 0x20b0),
        clock - 2,
        "each action costs two seconds"
    );
    capture(&emulator, "billiards-shot");

    tap(&mut emulator, 13);
    assert_eq!(
        word(&emulator, 0x20b0),
        clock - 4,
        "the clock keeps ticking"
    );
    assert_eq!(
        emulator.vm().memory()[0x209c],
        1,
        "a key press finishes the shot animation"
    );
    capture(&emulator, "billiards-settled");

    tap(&mut emulator, 27);
    assert_eq!(
        emulator.vm().memory()[0x20a4],
        14,
        "escape returns to the title"
    );
    assert!(emulator.vm().is_running());
    capture(&emulator, "billiards-title");

    emulator.input_mut().set_keys([27]);
    let mut halted = false;
    for _ in 0..120 {
        let result = emulator.run_frame().expect("the title screen must execute");
        if matches!(result.status, FrameStatus::Halted(_)) {
            halted = true;
            break;
        }
    }
    emulator.input_mut().release_all();
    assert!(halted, "escape on the title leaves the program");
    assert!(!emulator.vm().is_running());
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn sudoku_plays_a_level_fills_digits_and_switches_levels() {
    fn tap(emulator: &mut Emulator, key: u8) {
        emulator.input_mut().set_keys([key]);
        advance(emulator, 3);
        emulator.input_mut().release_all();
        advance(emulator, 40);
    }
    fn cell(emulator: &Emulator, index: usize) -> u8 {
        emulator.vm().memory()[0x2383 + index]
    }
    const GIVEN: [u8; 81] = [
        4, 0, 0, 1, 0, 8, 2, 9, 3, 9, 2, 0, 3, 6, 0, 5, 0, 1, 0, 7, 1, 0, 0, 5, 4, 0, 0, 2, 3, 0,
        0, 8, 0, 9, 5, 4, 0, 0, 7, 5, 0, 9, 0, 3, 2, 0, 0, 0, 6, 3, 0, 0, 1, 8, 7, 0, 0, 2, 0, 6,
        0, 0, 0, 0, 4, 9, 8, 1, 0, 3, 0, 5, 1, 5, 2, 0, 9, 0, 0, 0, 0,
    ];

    let mut emulator = load("LAVA1 数独1.3/sudoku.lav");
    advance(&mut emulator, 60);
    assert_eq!(emulator.vm().pc(), 0x26f3, "the title menu waits for input");
    capture(&emulator, "sudoku-menu");

    tap(&mut emulator, 21);
    assert_eq!(emulator.vm().memory()[0x262e], 2, "down moves the cursor");
    tap(&mut emulator, 20);
    assert_eq!(emulator.vm().memory()[0x262e], 1, "up restores the cursor");
    tap(&mut emulator, 13);
    assert_eq!(emulator.vm().pc(), 0x1823, "enter starts the first level");
    assert_eq!(emulator.vm().memory()[0x237d], 1);
    for (index, value) in GIVEN.iter().enumerate() {
        assert_eq!(cell(&emulator, index), *value, "given cell {index}");
    }
    capture(&emulator, "sudoku-level-1");

    assert_eq!(
        emulator.vm().memory()[0x2008],
        4,
        "the cursor starts centered"
    );
    assert_eq!(emulator.vm().memory()[0x200a], 4);
    for expected in [5, 6, 7, 8, 0] {
        tap(&mut emulator, 22);
        assert_eq!(emulator.vm().memory()[0x2008], expected, "right wraps");
    }
    for expected in [5, 6, 7, 8, 0] {
        tap(&mut emulator, 21);
        assert_eq!(emulator.vm().memory()[0x200a], expected, "down wraps");
    }
    for _ in 0..5 {
        tap(&mut emulator, 22);
        tap(&mut emulator, 21);
    }
    assert_eq!(
        (
            emulator.vm().memory()[0x2008],
            emulator.vm().memory()[0x200a]
        ),
        (5, 5)
    );
    tap(&mut emulator, 104);
    assert_eq!(cell(&emulator, 5 * 9 + 5), 5, "the number keys fill a cell");
    capture(&emulator, "sudoku-digit");

    tap(&mut emulator, 0x1c);
    assert_eq!(emulator.vm().memory()[0x200c], 1, "F1 enables hints");
    tap(&mut emulator, 0x1c);
    assert_eq!(emulator.vm().memory()[0x200c], 0, "F1 disables hints");

    tap(&mut emulator, 13);
    assert_eq!(
        emulator.vm().pc(),
        0x51e,
        "enter checks an unfinished board"
    );
    tap(&mut emulator, 13);
    assert_eq!(
        emulator.vm().pc(),
        0x1823,
        "the unfinished dialog dismisses"
    );
    assert_eq!(cell(&emulator, 5 * 9 + 5), 5);

    tap(&mut emulator, 0x1e);
    assert_eq!(emulator.vm().pc(), 0x4b6, "F3 asks before the next level");
    tap(&mut emulator, 121);
    assert_eq!(emulator.vm().pc(), 0x1823);
    assert_eq!(emulator.vm().memory()[0x237d], 2, "F3 loads level two");
    let second: Vec<u8> = (0..9).map(|index| cell(&emulator, index)).collect();
    assert_eq!(second, [6, 0, 9, 4, 0, 0, 8, 0, 0]);
    capture(&emulator, "sudoku-level-2");

    tap(&mut emulator, 0x1f);
    assert_eq!(
        emulator.vm().pc(),
        0x4b6,
        "F4 asks before the previous level"
    );
    tap(&mut emulator, 121);
    assert_eq!(emulator.vm().pc(), 0x1823);
    assert_eq!(emulator.vm().memory()[0x237d], 1, "F4 returns to level one");
    for (index, value) in GIVEN.iter().enumerate() {
        assert_eq!(cell(&emulator, index), *value, "level one cell {index}");
    }

    tap(&mut emulator, 0x1d);
    assert_eq!(emulator.vm().pc(), 0x4b6, "F2 asks before showing answers");
    tap(&mut emulator, 110);
    assert_eq!(emulator.vm().pc(), 0x1823, "n cancels the answer prompt");
    assert_eq!(
        cell(&emulator, 5 * 9 + 5),
        0,
        "n leaves the reloaded board untouched"
    );
    tap(&mut emulator, 0x1d);
    tap(&mut emulator, 121);
    assert_eq!(emulator.vm().pc(), 0x1823);
    let answer: Vec<u8> = (0..9).map(|index| cell(&emulator, index)).collect();
    assert_eq!(
        answer,
        [6, 7, 8, 1, 9, 5, 4, 0, 8],
        "the answer overlay comes from the headerless answer file"
    );
    assert_eq!(
        cell(&emulator, 5 * 9 + 5),
        6,
        "the answer overwrites entries"
    );
    capture(&emulator, "sudoku-answer");

    tap(&mut emulator, 27);
    assert_eq!(emulator.vm().pc(), 0x26f3, "escape returns to the menu");
    for expected in 2..=6 {
        tap(&mut emulator, 21);
        assert_eq!(emulator.vm().memory()[0x262e], expected);
    }
    tap(&mut emulator, 13);
    assert_eq!(emulator.vm().pc(), 0x1bd7, "the menu opens the help screen");
    assert!(emulator.vm().is_running());
    capture(&emulator, "sudoku-help");
    tap(&mut emulator, 27);
    assert_eq!(emulator.vm().pc(), 0x26f3, "escape leaves the help screen");
    tap(&mut emulator, 21);
    assert_eq!(emulator.vm().memory()[0x262e], 7);

    emulator.input_mut().set_keys([13]);
    let mut halted = false;
    for _ in 0..120 {
        let result = emulator.run_frame().expect("the menu must execute");
        if matches!(result.status, FrameStatus::Halted(_)) {
            halted = true;
            break;
        }
    }
    emulator.input_mut().release_all();
    assert!(halted, "the exit item leaves the program");
    assert!(!emulator.vm().is_running());
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn circuit_simulator_browses_chips_edits_waveforms_and_exits() {
    fn tap(emulator: &mut Emulator, key: u8, settle: usize) {
        emulator.input_mut().set_keys([key]);
        advance(emulator, 1);
        emulator.input_mut().release_all();
        advance(emulator, settle);
    }
    fn tap_until(
        emulator: &mut Emulator,
        key: u8,
        settle: usize,
        ready: impl Fn(&Emulator) -> bool,
    ) {
        for _ in 0..60 {
            tap(emulator, key, settle);
            if ready(emulator) {
                return;
            }
        }
        panic!("key {key} did not reach the expected screen");
    }
    for (name, path) in [
        ("unpacked", "LAVA1 数电模拟器/未打包的文件/数电模拟器.lav"),
        ("packed", "LAVA1 数电模拟器/已打包的文件/数电模拟器.pac"),
    ] {
        let mut emulator = load(path);
        advance(&mut emulator, 600);
        assert_eq!(emulator.vm().pc(), 0x4a81, "the intro reaches the menu");
        assert!(emulator.vm().is_running());
        capture(&emulator, &format!("circuit-{name}-menu"));

        tap(&mut emulator, 21, 30);
        assert_eq!(emulator.vm().memory()[0x25bd], 1, "down moves the menu");
        tap(&mut emulator, 21, 30);
        assert_eq!(emulator.vm().memory()[0x25bd], 2);
        tap(&mut emulator, 21, 30);
        assert_eq!(emulator.vm().memory()[0x25bd], 0, "down wraps");
        tap(&mut emulator, 20, 30);
        assert_eq!(emulator.vm().memory()[0x25bd], 2, "up wraps");
        tap(&mut emulator, 21, 30);
        tap(&mut emulator, 22, 30);
        assert_eq!(
            emulator.vm().memory()[0x25bc],
            1,
            "right selects the triggers"
        );
        tap(&mut emulator, 23, 30);
        assert_eq!(emulator.vm().memory()[0x25bc], 0, "left selects the gates");

        let menu = emulator.display().indexed_frame().to_vec();
        tap(&mut emulator, 13, 400);
        assert_ne!(
            menu,
            emulator.display().indexed_frame(),
            "the chip viewer replaces the menu"
        );
        assert_eq!(
            emulator.vm().memory()[0x25d4],
            0,
            "the list opens on 74LS00"
        );
        tap_until(&mut emulator, 21, 200, |emulator| {
            emulator.vm().memory()[0x25d4] == 1
        });
        assert_eq!(emulator.vm().memory()[0x25d4], 1, "down selects 74LS08");
        capture(&emulator, &format!("circuit-{name}-chip"));
        tap_until(&mut emulator, 21, 200, |emulator| {
            emulator.vm().memory()[0x25d4] == 6
        });
        assert_eq!(
            emulator.vm().memory()[0x25d4],
            6,
            "the list reaches 74LS283"
        );
        tap_until(&mut emulator, 20, 200, |emulator| {
            emulator.vm().memory()[0x25d4] == 4
        });
        assert_eq!(emulator.vm().memory()[0x25d4], 4, "up moves back");
        tap_until(&mut emulator, 27, 200, |emulator| {
            emulator.vm().pc() == 0x4a81
        });

        tap(&mut emulator, 21, 30);
        tap(&mut emulator, 21, 30);
        tap(&mut emulator, 13, 60);
        assert_eq!(emulator.vm().pc(), 0x5338, "enter opens the notes");
        capture(&emulator, &format!("circuit-{name}-notes"));
        tap(&mut emulator, 20, 60);
        assert_eq!(emulator.vm().pc(), 0x4a81, "a key leaves the notes");
        assert_eq!(emulator.vm().memory()[0x25bd], 0, "the menu resets");

        tap(&mut emulator, 22, 30);
        tap(&mut emulator, 13, 60);
        assert_eq!(emulator.vm().pc(), 0x54df, "enter opens the trigger list");
        let first = emulator.display().indexed_frame().to_vec();
        for _ in 0..5 {
            tap(&mut emulator, 21, 100);
        }
        let last = emulator.display().indexed_frame().to_vec();
        assert_ne!(first, last, "down moves the trigger selection");
        capture(&emulator, &format!("circuit-{name}-triggers"));
        tap(&mut emulator, 21, 100);
        assert_eq!(
            last,
            emulator.display().indexed_frame(),
            "the trigger list clamps at the last entry"
        );
        tap(&mut emulator, 20, 100);
        assert_ne!(
            last,
            emulator.display().indexed_frame(),
            "up moves the trigger selection"
        );
        tap(&mut emulator, 20, 100);
        tap(&mut emulator, 27, 60);
        assert_eq!(emulator.vm().pc(), 0x4a81, "escape leaves the trigger list");

        tap(&mut emulator, 22, 30);
        tap(&mut emulator, 21, 30);
        tap(&mut emulator, 13, 60);
        assert_eq!(emulator.vm().pc(), 0x7e27, "the editor shows its intro");
        capture(&emulator, &format!("circuit-{name}-editor-intro"));
        tap(&mut emulator, 13, 60);
        assert_eq!(emulator.vm().pc(), 0x54df, "a key opens the trigger list");
        tap(&mut emulator, 13, 60);
        assert_eq!(
            emulator.vm().pc(),
            0x56be,
            "enter opens the waveform editor"
        );
        capture(&emulator, &format!("circuit-{name}-editor"));

        for _ in 0..16 {
            tap(&mut emulator, 22, 30);
        }
        assert_eq!(emulator.vm().memory()[0x25c3], 0, "the column cursor wraps");
        tap(&mut emulator, 23, 30);
        assert_eq!(emulator.vm().memory()[0x25c3], 15, "left wraps backwards");
        tap(&mut emulator, 22, 30);
        tap(&mut emulator, 21, 30);
        assert_eq!(emulator.vm().memory()[0x25c4], 1, "down selects the R row");
        tap(&mut emulator, 21, 30);
        assert_eq!(emulator.vm().memory()[0x25c4], 1, "the row clamps");
        tap(&mut emulator, 20, 30);
        assert_eq!(emulator.vm().memory()[0x25c4], 0, "up returns to the S row");

        let idle = emulator.display().indexed_frame().to_vec();
        tap(&mut emulator, 48, 30);
        assert_ne!(
            idle,
            emulator.display().indexed_frame(),
            "a digit drives the input low"
        );
        tap(&mut emulator, 13, 120);
        assert_eq!(emulator.vm().pc(), 0x5b9d, "enter simulates the latch");
        capture(&emulator, &format!("circuit-{name}-result"));
        tap(&mut emulator, 27, 60);
        assert_eq!(emulator.vm().pc(), 0x5bdf, "escape asks to return");
        capture(&emulator, &format!("circuit-{name}-return"));
        tap(&mut emulator, 121, 60);
        assert_eq!(emulator.vm().pc(), 0x56be, "y returns to the editor");
        tap(&mut emulator, 27, 60);
        assert_eq!(
            emulator.vm().pc(),
            0x586d,
            "escape asks to leave the editor"
        );
        tap(&mut emulator, 121, 60);
        assert_eq!(emulator.vm().pc(), 0x4a81, "y leaves the editor");

        tap(&mut emulator, 27, 60);
        assert_eq!(emulator.vm().pc(), 0x48a5, "escape shows the credits");
        capture(&emulator, &format!("circuit-{name}-credits"));
        emulator.input_mut().set_keys([27]);
        let mut halted = false;
        for _ in 0..120 {
            let result = emulator.run_frame().expect("the credits must execute");
            if matches!(result.status, FrameStatus::Halted(_)) {
                halted = true;
                break;
            }
        }
        emulator.input_mut().release_all();
        assert!(halted, "escape leaves the program");
        assert!(!emulator.vm().is_running());
    }
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn slot_machine_places_bets_spins_and_exits() {
    fn tap(emulator: &mut Emulator, key: u8, settle: usize) {
        emulator.input_mut().set_keys([key]);
        advance(emulator, 3);
        emulator.input_mut().release_all();
        advance(emulator, settle);
    }
    fn tap_until(
        emulator: &mut Emulator,
        key: u8,
        settle: usize,
        ready: impl Fn(&Emulator) -> bool,
    ) {
        for _ in 0..80 {
            tap(emulator, key, settle);
            if ready(emulator) {
                return;
            }
        }
        panic!("key {key} did not reach the expected screen");
    }
    fn bet(emulator: &Emulator, column: usize) -> u16 {
        let memory = emulator.vm().memory();
        u16::from_le_bytes([memory[0x414e + column * 2], memory[0x414f + column * 2]])
    }
    let mut emulator = load("LAVA1 旋转老虎机/旋转老虎机.lav");
    advance(&mut emulator, 300);
    assert_eq!(emulator.vm().pc(), 0x21fd, "the splash waits for a key");
    capture(&emulator, "slot-splash");
    tap_until(&mut emulator, 13, 90, |emulator| {
        emulator.vm().pc() == 0x2348
    });
    assert_eq!(
        emulator.vm().memory()[0x4132],
        7,
        "the menu opens on the first item"
    );
    capture(&emulator, "slot-menu");
    for expected in [21, 35, 49] {
        tap(&mut emulator, 21, 60);
        assert_eq!(emulator.vm().memory()[0x4132], expected);
    }
    tap(&mut emulator, 21, 60);
    assert_eq!(emulator.vm().memory()[0x4132], 7, "down wraps the menu");
    tap(&mut emulator, 20, 60);
    assert_eq!(emulator.vm().memory()[0x4132], 49, "up wraps the menu");
    tap(&mut emulator, 20, 60);
    assert_eq!(emulator.vm().memory()[0x4132], 35);

    tap(&mut emulator, 13, 120);
    assert_eq!(emulator.vm().pc(), 0x24f3, "enter opens the instructions");
    capture(&emulator, "slot-instructions");
    tap(&mut emulator, 27, 90);
    assert_eq!(emulator.vm().pc(), 0x2524, "escape shows the second page");
    capture(&emulator, "slot-instructions-2");
    tap(&mut emulator, 21, 90);
    assert_eq!(emulator.vm().pc(), 0x2348, "a key returns to the menu");

    tap(&mut emulator, 20, 60);
    assert_eq!(emulator.vm().memory()[0x4132], 21);
    tap(&mut emulator, 13, 120);
    assert_eq!(emulator.vm().pc(), 0x2400, "enter opens the version page");
    capture(&emulator, "slot-version");
    tap(&mut emulator, 27, 90);
    assert_eq!(
        emulator.vm().pc(),
        0x2480,
        "escape shows the version details"
    );
    capture(&emulator, "slot-version-2");
    tap(&mut emulator, 21, 90);
    assert_eq!(emulator.vm().pc(), 0x21fd, "a key returns to the title");

    tap_until(&mut emulator, 13, 90, |emulator| {
        emulator.vm().pc() == 0x2348
    });
    assert_eq!(emulator.vm().memory()[0x4132], 7);
    tap(&mut emulator, 13, 120);
    assert_eq!(emulator.vm().pc(), 0x2720, "enter starts a round");
    assert_eq!(
        emulator.vm().memory()[0x4168..0x416a],
        [20, 0],
        "a round starts with 20 credits"
    );
    assert_eq!(emulator.vm().memory()[0x4166], 0, "no bet is placed");
    for column in 0..7 {
        assert_eq!(bet(&emulator, column), 0);
    }
    capture(&emulator, "slot-game");

    tap(&mut emulator, 20, 60);
    assert_eq!(bet(&emulator, 0), 1, "up bets on the first column");
    assert_eq!(emulator.vm().memory()[0x4166], 1, "the total bet follows");
    assert_eq!(emulator.vm().memory()[0x4168..0x416a], [19, 0]);
    tap(&mut emulator, 22, 60);
    assert_eq!(
        emulator.vm().memory()[0x4130],
        41,
        "right moves to the next column"
    );
    tap(&mut emulator, 20, 60);
    assert_eq!(bet(&emulator, 1), 1, "up bets on the second column");
    assert_eq!(emulator.vm().memory()[0x4166], 2);
    assert_eq!(emulator.vm().memory()[0x4168..0x416a], [18, 0]);
    tap(&mut emulator, 21, 60);
    assert_eq!(bet(&emulator, 1), 0, "down withdraws the bet");
    assert_eq!(emulator.vm().memory()[0x4166], 1);
    assert_eq!(emulator.vm().memory()[0x4168..0x416a], [19, 0]);
    tap(&mut emulator, 23, 60);
    assert_eq!(
        emulator.vm().memory()[0x4130],
        25,
        "left returns to the first column"
    );
    capture(&emulator, "slot-bet");

    tap(&mut emulator, 13, 60);
    assert_ne!(emulator.vm().pc(), 0x2720, "enter spins the reels");
    let mut reached = false;
    for _ in 0..3000 {
        advance(&mut emulator, 1);
        if emulator.vm().pc() == 0x2ead {
            reached = true;
            break;
        }
    }
    assert!(reached, "the spin must reach the result screen");
    assert_eq!(
        emulator.vm().memory()[0x4164],
        6,
        "the result column is fixed"
    );
    assert_eq!(
        emulator.vm().memory()[0x416a],
        16,
        "the result payout is fixed"
    );
    assert_eq!(
        emulator.vm().memory()[0x4168..0x416a],
        [19, 0],
        "a losing round keeps the remaining credits"
    );
    capture(&emulator, "slot-result");

    tap(&mut emulator, 20, 120);
    assert_eq!(emulator.vm().pc(), 0x2720, "a key returns to the table");
    for column in 0..7 {
        assert_eq!(bet(&emulator, column), 0, "the table clears its bets");
    }
    assert_eq!(emulator.vm().memory()[0x4166], 0);
    assert_eq!(emulator.vm().memory()[0x4168..0x416a], [19, 0]);

    tap(&mut emulator, 27, 120);
    assert_eq!(emulator.vm().pc(), 0x3089, "escape asks to quit");
    capture(&emulator, "slot-quit");
    tap(&mut emulator, 22, 90);
    assert_eq!(emulator.vm().memory()[0x4130], 91, "right selects cancel");
    tap(&mut emulator, 13, 90);
    assert_eq!(emulator.vm().pc(), 0x2720, "enter cancels the quit");
    tap(&mut emulator, 27, 120);
    assert_eq!(emulator.vm().pc(), 0x3089);
    tap(&mut emulator, 13, 120);
    assert_eq!(
        emulator.vm().pc(),
        0x21fd,
        "confirming returns to the title"
    );
    assert_eq!(
        emulator.vm().memory()[0x4168..0x416a],
        [20, 0],
        "leaving a round restores the starting credits"
    );
    capture(&emulator, "slot-title");

    tap_until(&mut emulator, 13, 90, |emulator| {
        emulator.vm().pc() == 0x2348
    });
    for _ in 0..8 {
        if emulator.vm().memory()[0x4132] == 49 {
            break;
        }
        tap(&mut emulator, 21, 60);
    }
    assert_eq!(
        emulator.vm().memory()[0x4132],
        49,
        "the exit item is selected"
    );
    emulator.input_mut().set_keys([13]);
    let mut halted = false;
    for _ in 0..120 {
        let result = emulator.run_frame().expect("the menu must execute");
        if matches!(result.status, FrameStatus::Halted(_)) {
            halted = true;
            break;
        }
        emulator.input_mut().release_all();
    }
    emulator.input_mut().release_all();
    assert!(halted, "the exit item leaves the program");
    assert!(!emulator.vm().is_running());
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn bottomless_pit_selects_difficulty_descends_saves_and_replays() {
    fn finish(emulator: &mut Emulator) {
        for _ in 0..400 {
            if emulator.vm().pc() == 0x762 {
                return;
            }
            advance(emulator, 1);
        }
        panic!("the fall must finish at the result screen");
    }
    fn running(emulator: &Emulator) -> bool {
        matches!(emulator.vm().pc(), 0xeee | 0xbac)
    }
    let mut emulator = load("LAVA1 无底洞/无底洞.lav");
    advance(&mut emulator, 300);
    assert_eq!(emulator.vm().pc(), 0x762, "the title waits for a key");
    capture(&emulator, "pit-title");

    for expected in [1, 2, 0] {
        press(&mut emulator, b'l');
        assert_eq!(
            emulator.vm().memory()[0x2143],
            expected,
            "L cycles the difficulty"
        );
    }
    capture(&emulator, "pit-difficulty");

    press(&mut emulator, 13);
    assert!(running(&emulator), "enter starts a run");
    let start_x = emulator.vm().memory()[0x323a];
    press(&mut emulator, 20);
    assert_eq!(
        emulator.vm().memory()[0x323a],
        start_x,
        "up does not move the falling player"
    );
    press(&mut emulator, 22);
    assert_eq!(
        emulator.vm().memory()[0x323a],
        start_x + 2,
        "right moves the player"
    );
    press(&mut emulator, 23);
    assert_eq!(
        emulator.vm().memory()[0x323a],
        start_x,
        "left moves the player back"
    );
    capture(&emulator, "pit-run");

    finish(&mut emulator);
    assert_eq!(
        emulator.vm().memory()[0x2145],
        1,
        "the fall counts one floor"
    );
    capture(&emulator, "pit-result");

    press(&mut emulator, b's');
    assert_eq!(emulator.vm().pc(), 0x7ec, "S asks to save the record");
    capture(&emulator, "pit-save-prompt");
    press(&mut emulator, b'y');
    assert_eq!(emulator.vm().pc(), 0x762);
    let records = emulator.files_mut().take_dirty_files();
    let (_, record) = records
        .iter()
        .find(|(path, _)| path == "/lavadata/hole.sav")
        .expect("the record is saved");
    assert_eq!(record.len(), 4080, "the record holds the full replay");
    assert_eq!(record[0], 1, "the record is marked as used");
    capture(&emulator, "pit-saved");

    press(&mut emulator, b'r');
    assert_ne!(emulator.vm().pc(), 0x762, "R replays the record");
    capture(&emulator, "pit-replay");
    finish(&mut emulator);
    assert_eq!(
        emulator.vm().memory()[0x2145],
        1,
        "the replay repeats the fall without input"
    );
    capture(&emulator, "pit-replay-result");

    press(&mut emulator, 13);
    assert!(running(&emulator), "enter starts another run");
    press(&mut emulator, 27);
    assert_eq!(emulator.vm().pc(), 0x762, "escape leaves the run");
    capture(&emulator, "pit-escape");
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn star_legend_package_registration_writes_a_usable_record() {
    let mut package = load("LAVA1 星剑传奇/打包的[星剑传奇]/星剑传奇.pac");
    advance(&mut package, 300);
    for key in [13]
        .into_iter()
        .chain(*b"lavaxx")
        .chain([13])
        .chain(*b"testxx")
        .chain([13, b'y', 13])
        .chain(*b"lava")
        .chain([13])
    {
        press(&mut package, key);
    }
    capture(&package, "star-legend-pac-registration");
    let (record_path, record) = package
        .files_mut()
        .take_dirty_files()
        .into_iter()
        .find(|(path, bytes)| path.ends_with("s-lavaxx.dat") && bytes.len() == 194)
        .expect("the packaged registration entry writes the account record");

    let mut emulator = load("LAVA1 星剑传奇/未打包的[星剑传奇]/星剑传奇.lav");
    emulator.files_mut().import_file(&record_path, record);
    advance(&mut emulator, 600);
    press(&mut emulator, 13);
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
    capture(&emulator, "star-legend-pac-character");
    press(&mut emulator, 13);
    advance(&mut emulator, 600);
    capture(&emulator, "star-legend-pac-map");
    let mut idle = emulator.clone();
    advance(&mut idle, 60);
    let moved = input_branch(&emulator, 20);
    capture(&moved, "star-legend-pac-movement");
    assert!(
        moved.display().indexed_frame() != idle.display().indexed_frame(),
        "the packaged record must load a character that moves on the map"
    );
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn down_hundred_floors_steers_and_reaches_game_over() {
    let mut emulator = load("LAVA1 是男人就下100层（模拟器可能不能运行）/是男人就下100层.lav");
    advance(&mut emulator, 300);
    assert_eq!(emulator.vm().pc(), 0x1189, "the title waits for a key");
    capture(&emulator, "down100-title");
    press(&mut emulator, 13);
    assert_eq!(emulator.vm().pc(), 0x12b5, "the first credit screen waits");
    capture(&emulator, "down100-first-splash");
    press(&mut emulator, 13);
    assert_eq!(emulator.vm().pc(), 0x1328, "the second credit screen waits");
    capture(&emulator, "down100-second-splash");
    press(&mut emulator, 13);
    assert_eq!(emulator.vm().pc(), 0x19f7, "a demonstration run starts");
    capture(&emulator, "down100-run");

    let mut still = emulator.clone();
    advance(&mut still, 33);
    for key in [20, 21] {
        let mut other = emulator.clone();
        press(&mut other, key);
        assert_eq!(
            other.display().indexed_frame(),
            still.display().indexed_frame(),
            "up and down do not steer the character in this scene"
        );
    }
    let mut right = emulator.clone();
    press(&mut right, 22);
    capture(&right, "down100-right");
    assert!(
        right.display().indexed_frame() != still.display().indexed_frame(),
        "right steers the falling character"
    );
    let mut left = emulator.clone();
    press(&mut left, 23);
    capture(&left, "down100-left");
    assert!(
        left.display().indexed_frame() != still.display().indexed_frame()
            && left.display().indexed_frame() != right.display().indexed_frame(),
        "left steers the falling character in the other direction"
    );

    let mut idle = emulator.clone();
    for _ in 0..400 {
        if idle.vm().pc() == 0x1a1a {
            break;
        }
        advance(&mut idle, 1);
    }
    assert_eq!(idle.vm().pc(), 0x1a1a, "the run ends in game over");
    capture(&idle, "down100-game-over");
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn hold_twenty_seconds_moves_the_player_and_shows_the_result() {
    let mut emulator = load("LAVA1 是男人就撑20秒正式版/是男人就撑20秒正式版.lav");
    advance(&mut emulator, 300);
    assert!(
        matches!(emulator.vm().pc(), 0x1ec6 | 0x1fcd),
        "the title menu waits for a key"
    );
    capture(&emulator, "hold20-title");
    press(&mut emulator, 13);
    assert_eq!(emulator.vm().pc(), 0x1bdb, "Enter starts a survival run");
    capture(&emulator, "hold20-run");

    let mut still = emulator.clone();
    advance(&mut still, 50);
    let mut positions = Vec::new();
    for key in [20, 21, 22, 23] {
        let mut steered = emulator.clone();
        hold(&mut steered, key, 20);
        capture(&steered, &format!("hold20-{key}"));
        let frame = steered.display().indexed_frame().to_vec();
        assert!(
            frame != still.display().indexed_frame(),
            "each arrow key moves the player away from its idle position"
        );
        positions.push(frame);
    }
    for (index, frame) in positions.iter().enumerate() {
        for other in &positions[index + 1..] {
            assert_ne!(
                frame, other,
                "the four arrow keys move the player to different positions"
            );
        }
    }

    let mut run = emulator.clone();
    for _ in 0..500 {
        if run.vm().pc() == 0x1665 {
            break;
        }
        run.input_mut().set_keys([20]);
        advance(&mut run, 1);
    }
    run.input_mut().set_keys([]);
    assert_eq!(
        run.vm().pc(),
        0x1665,
        "steering into the falling field ends the run"
    );
    capture(&run, "hold20-result");
    press(&mut run, 13);

    let mut resumed = false;
    for _ in 0..600 {
        if run.vm().pc() == 0x1bdb {
            resumed = true;
            break;
        }
        advance(&mut run, 1);
    }
    assert!(
        resumed,
        "a new run starts after the result is shown (pc {:#x})",
        run.vm().pc()
    );
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn fly_ten_thousand_meters_adjusts_speed_and_climbs() {
    let mut emulator = load("LAVA1 是男人就飞10000米/是男人就飞10000米.lav");
    advance(&mut emulator, 300);
    assert_eq!(emulator.vm().pc(), 0x9a7, "the title waits for a key");
    capture(&emulator, "fly10000-title");

    let mut setup = emulator.clone();
    press(&mut setup, 31);
    assert_eq!(setup.vm().pc(), 0x8d0, "F4 opens the speed adjustment");
    capture(&setup, "fly10000-settings");

    let initial = setup.display().indexed_frame().to_vec();
    let mut faster = setup.clone();
    hold(&mut faster, 20, 3);
    capture(&faster, "fly10000-speed-up");
    assert_ne!(
        faster.display().indexed_frame(),
        initial,
        "Up raises the speed value"
    );
    let mut restored = faster.clone();
    hold(&mut restored, 21, 3);
    assert_eq!(
        restored.display().indexed_frame(),
        initial,
        "Down lowers the speed value back"
    );

    press(&mut emulator, 13);
    assert_eq!(emulator.vm().pc(), 0xaab, "Enter starts the flight");
    capture(&emulator, "fly10000-run");

    let mut still = emulator.clone();
    advance(&mut still, 90);
    let mut climbed = emulator.clone();
    hold(&mut climbed, 20, 60);
    capture(&climbed, "fly10000-climb");
    assert!(
        climbed.display().indexed_frame() != still.display().indexed_frame(),
        "holding a key lifts the plane"
    );

    let mut first = emulator.clone();
    advance(&mut first, 60);
    let mut later = first.clone();
    advance(&mut later, 60);
    assert!(
        first.display().indexed_frame() != later.display().indexed_frame(),
        "the flight keeps moving without input"
    );

    let mut crashed = emulator.clone();
    let mut credits = false;
    for _ in 0..1200 {
        if crashed.vm().pc() == 0xbc6 {
            credits = true;
            break;
        }
        advance(&mut crashed, 1);
    }
    assert!(credits, "an unattended flight ends on the credits screen");
    capture(&crashed, "fly10000-credits");

    let mut halted = false;
    for _ in 0..60 {
        crashed.input_mut().set_keys([27]);
        if matches!(
            crashed
                .run_frame()
                .expect("game must execute without errors")
                .status,
            FrameStatus::Halted(_)
        ) {
            halted = true;
            break;
        }
    }
    assert!(halted, "Escape exits from the credits screen");
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn moon_city_reaches_the_village_and_handles_menu_items() {
    let mut emulator = load("LAVA1 月光城/Moon.pac");
    advance(&mut emulator, 300);
    assert_eq!(emulator.vm().pc(), 0x286, "the title menu waits");
    capture(&emulator, "moon-title");
    let title = emulator.clone();

    let mut loose = load("LAVA1 月光城/非pac/Moon.lav");
    advance(&mut loose, 300);
    assert_eq!(
        loose.display().indexed_frame(),
        title.display().indexed_frame(),
        "the packaged and loose entries render the same title"
    );

    let mut loading = title.clone();
    press(&mut loading, 21);
    press(&mut loading, 13);
    assert_eq!(loading.vm().pc(), 0x1191, "the load item opens two slots");
    capture(&loading, "moon-load-slots");
    press(&mut loading, 13);
    capture(&loading, "moon-load-failure");
    let mut back = false;
    for _ in 0..600 {
        if loading.vm().pc() == 0x286 {
            back = true;
            break;
        }
        advance(&mut loading, 1);
    }
    assert!(back, "a missing record returns to the title");

    press(&mut emulator, 13);
    assert_eq!(emulator.vm().pc(), 0x122d, "a new game asks for a name");
    capture(&emulator, "moon-name");
    press(&mut emulator, 13);
    assert_ne!(
        emulator.vm().pc(),
        0x122d,
        "an empty name enters the village"
    );
    capture(&emulator, "moon-village");

    let mut still = emulator.clone();
    advance(&mut still, 70);
    let mut views = Vec::new();
    for key in [20, 21, 22, 23] {
        let mut moved = emulator.clone();
        hold(&mut moved, key, 40);
        capture(&moved, &format!("moon-{key}"));
        let frame = moved.display().indexed_frame().to_vec();
        assert!(
            frame != still.display().indexed_frame(),
            "each arrow key changes the village view"
        );
        views.push(frame);
    }
    for (index, frame) in views.iter().enumerate() {
        for other in &views[index + 1..] {
            assert_ne!(frame, other, "the four arrow keys move in four directions");
        }
    }

    let mut quitter = title.clone();
    for _ in 0..3 {
        press(&mut quitter, 21);
    }
    press(&mut quitter, 13);
    let mut halted = false;
    for _ in 0..300 {
        if matches!(
            quitter
                .run_frame()
                .expect("game must execute without errors")
                .status,
            FrameStatus::Halted(_)
        ) {
            halted = true;
            break;
        }
    }
    assert!(halted, "the exit item halts the program");
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn unnamed_village_map_walks_and_scrolls() {
    let mut emulator = load("LAVA1 未知/game.lav");
    advance(&mut emulator, 300);
    capture(&emulator, "unknown-spawn");

    let mut still = emulator.clone();
    advance(&mut still, 70);
    let mut right = emulator.clone();
    hold(&mut right, 22, 40);
    capture(&right, "unknown-right");
    assert!(
        right.display().indexed_frame() != still.display().indexed_frame(),
        "Right walks and scrolls the map"
    );
    for key in [20, 23] {
        let mut turned = emulator.clone();
        hold(&mut turned, key, 40);
        capture(&turned, &format!("unknown-{key}"));
        assert!(
            turned.display().indexed_frame() != still.display().indexed_frame(),
            "Up and Left turn the character at the spawn"
        );
    }
    let mut blocked = emulator.clone();
    hold(&mut blocked, 21, 40);
    assert_eq!(
        blocked.display().indexed_frame(),
        still.display().indexed_frame(),
        "Down is blocked by the fence at the spawn"
    );

    let mut walking = emulator.clone();
    hold(&mut walking, 22, 300);
    capture(&walking, "unknown-east-edge");
    let mut resting = walking.clone();
    advance(&mut resting, 90);
    for key in [20, 21, 23] {
        let mut moved = walking.clone();
        hold(&mut moved, key, 60);
        capture(&moved, &format!("unknown-east-{key}"));
        assert!(
            moved.display().indexed_frame() != resting.display().indexed_frame(),
            "the character can leave the eastern edge in another direction"
        );
    }
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn dream_twenty_one_places_cards_into_the_selected_pile() {
    let mut emulator = load("LAVA1 梦幻21点/梦幻21点.lav");
    advance(&mut emulator, 300);
    assert_eq!(emulator.vm().pc(), 0x18f4, "the splash waits for a key");
    capture(&emulator, "21point-splash");

    press(&mut emulator, 13);
    assert_eq!(emulator.vm().pc(), 0x198c, "Enter starts the first round");
    capture(&emulator, "21point-round");

    let state = |emulator: &Emulator| {
        let memory = emulator.vm().memory();
        (
            memory[0x2036],
            [
                memory[0x2037],
                memory[0x2038],
                memory[0x2039],
                memory[0x203a],
            ],
            [
                memory[0x203b],
                memory[0x203c],
                memory[0x203d],
                memory[0x203e],
            ],
            u32::from_le_bytes([
                memory[0x203f],
                memory[0x2040],
                memory[0x2041],
                memory[0x2042],
            ]),
        )
    };
    assert_eq!(
        state(&emulator),
        (0, [0; 4], [0; 4], 0),
        "a new round starts with an empty deck and no score"
    );

    let mut idle = emulator.clone();
    advance(&mut idle, 120);
    assert_eq!(idle.vm().pc(), 0x198c, "the round waits for input");
    assert_eq!(
        idle.display().indexed_frame(),
        emulator.display().indexed_frame(),
        "the round does not advance on its own"
    );

    let mut selected = emulator.clone();
    press(&mut selected, 22);
    assert_eq!(state(&selected).0, 1, "Right moves the pile cursor");
    assert_ne!(
        selected.display().indexed_frame(),
        idle.display().indexed_frame(),
        "the cursor is visible"
    );
    capture(&selected, "21point-cursor");

    let mut far = selected.clone();
    press(&mut far, 22);
    assert_eq!(state(&far).0, 2, "Right moves to the third pile");
    assert_ne!(
        far.display().indexed_frame(),
        selected.display().indexed_frame()
    );

    let mut back = far.clone();
    press(&mut back, 23);
    press(&mut back, 23);
    assert_eq!(state(&back).0, 0, "Left returns to the first pile");
    assert_eq!(
        back.display().indexed_frame(),
        idle.display().indexed_frame(),
        "moving back restores the round view"
    );

    let mut left = emulator.clone();
    press(&mut left, 23);
    assert_eq!(state(&left).0, 0, "Left at the first pile keeps the cursor");
    assert_eq!(
        left.display().indexed_frame(),
        idle.display().indexed_frame()
    );

    let mut placed = emulator.clone();
    press(&mut placed, 13);
    press(&mut placed, 13);
    let (cursor, counts, totals, score) = state(&placed);
    assert_eq!(cursor, 0, "placing cards keeps the cursor");
    assert_eq!(counts, [2, 0, 0, 0], "two presses place two cards");
    assert_eq!(totals, [12, 0, 0, 0], "the two aces total 12");
    assert_eq!(score, 100, "every placed card scores 50 points");
    assert_ne!(
        placed.display().indexed_frame(),
        idle.display().indexed_frame()
    );
    capture(&placed, "21point-pile");

    let mut other = emulator.clone();
    press(&mut other, 22);
    press(&mut other, 13);
    let (cursor, counts, totals, score) = state(&other);
    assert_eq!(cursor, 1, "the cursor stays on the chosen pile");
    assert_eq!(counts, [0, 1, 0, 0], "the card goes to the second pile");
    assert_eq!(totals, [0, 11, 0, 0], "the ace is worth 11");
    assert_eq!(score, 50);
    capture(&other, "21point-other-pile");

    let mut over = placed.clone();
    over.input_mut().set_keys([27]);
    let mut waiting = false;
    for _ in 0..120 {
        let result = over.run_frame().expect("game must execute without errors");
        if over.vm().pc() == 0x1681 {
            waiting = true;
            break;
        }
        assert!(
            !matches!(result.status, FrameStatus::Halted(_)),
            "Escape reaches the game-over screen before leaving"
        );
    }
    over.input_mut().set_keys([]);
    assert!(
        waiting,
        "the game-over screen waits for a key (pc {:#x})",
        over.vm().pc()
    );
    assert_eq!(state(&over).3, 100, "the game-over screen keeps the score");
    capture(&over, "21point-game-over");

    over.run_frame().expect("game must execute without errors");
    over.input_mut().set_keys([13]);
    let mut halted = false;
    for _ in 0..60 {
        let result = over.run_frame().expect("game must execute without errors");
        if matches!(result.status, FrameStatus::Halted(_)) {
            halted = true;
            break;
        }
    }
    assert!(
        halted,
        "a key on the game-over screen exits (pc {:#x})",
        over.vm().pc()
    );
    assert_eq!(over.vm().pc(), 0x1686);
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn dream_mario_walks_jumps_and_ignores_unbound_keys() {
    let mut emulator = load("LAVA1 梦幻mario/梦幻mario.lav");
    advance(&mut emulator, 300);
    assert_eq!(
        emulator.vm().pc(),
        0x3598,
        "the intro starts with its logo animation"
    );
    let mut early = emulator.clone();
    press(&mut early, 13);
    assert_eq!(early.vm().pc(), 0x359b, "an early Enter is ignored");
    assert_eq!(
        early.vm().memory()[0x3881],
        76,
        "the player does not move during the intro"
    );

    advance(&mut emulator, 60);
    assert_eq!(emulator.vm().pc(), 0x359b);
    capture(&emulator, "mario-title");
    let title = emulator.display().indexed_frame().to_vec();

    let mut help = emulator.clone();
    press(&mut help, 21);
    assert_ne!(
        help.display().indexed_frame(),
        title,
        "Down selects the help entry"
    );
    press(&mut help, 13);
    assert_eq!(help.vm().pc(), 0x3598, "the help screen stays on the title");
    capture(&help, "mario-help");

    press(&mut emulator, 13);
    assert_eq!(
        emulator.vm().pc(),
        0x3626,
        "Enter selects Start and shows the map prompt"
    );
    assert_ne!(emulator.display().indexed_frame(), title);
    capture(&emulator, "mario-map-prompt");

    press(&mut emulator, 13);
    advance(&mut emulator, 60);
    assert_eq!(
        emulator.vm().pc(),
        0x3092,
        "the first map waits at its game loop"
    );
    assert_eq!(emulator.vm().memory()[0x3880], 0x80);
    assert_eq!(emulator.vm().memory()[0x3881], 76);
    assert_eq!(emulator.vm().memory()[0x3882], 26);
    capture(&emulator, "mario-map");

    let idle = emulator.clone();
    let mut still = emulator.clone();
    advance(&mut still, 120);
    assert_eq!(still.vm().pc(), 0x3092, "the map waits for input");
    assert_eq!(
        still.display().indexed_frame(),
        emulator.display().indexed_frame(),
        "the map does not animate while waiting"
    );

    let mut right = emulator.clone();
    press(&mut right, b'd');
    assert_eq!(right.vm().memory()[0x3881], 80, "D walks right");
    assert_eq!(
        right.vm().memory()[0x3882],
        26,
        "walking keeps the player on the ground"
    );
    assert_ne!(
        right.display().indexed_frame(),
        idle.display().indexed_frame()
    );
    capture(&right, "mario-right");

    let mut left = emulator.clone();
    press(&mut left, b'a');
    assert_eq!(left.vm().memory()[0x3881], 72, "A walks left");
    assert_ne!(
        left.display().indexed_frame(),
        idle.display().indexed_frame()
    );
    capture(&left, "mario-left");

    let mut up = emulator.clone();
    press(&mut up, b'w');
    assert_eq!(up.vm().memory()[0x3881], 76, "W is not a movement key");
    assert_eq!(up.vm().memory()[0x3882], 26);
    assert_eq!(up.display().indexed_frame(), idle.display().indexed_frame());
    let mut down = emulator.clone();
    press(&mut down, b's');
    assert_eq!(down.vm().memory()[0x3881], 76, "S is not a movement key");
    assert_eq!(
        down.display().indexed_frame(),
        idle.display().indexed_frame()
    );

    let mut jump = emulator.clone();
    hold(&mut jump, b'l', 20);
    assert_eq!(
        jump.vm().memory()[0x3881],
        76,
        "jumping does not shift the player sideways"
    );
    assert_eq!(jump.vm().memory()[0x3882], 22, "L jumps");
    assert_ne!(
        jump.display().indexed_frame(),
        idle.display().indexed_frame()
    );
    capture(&jump, "mario-jump");
    advance(&mut jump, 60);
    assert_eq!(
        jump.vm().memory()[0x3882],
        26,
        "the jump lands back on the ground"
    );

    let mut walk = emulator.clone();
    for _ in 0..3 {
        press(&mut walk, b'd');
    }
    assert_eq!(
        walk.vm().memory()[0x3881],
        88,
        "three steps move the player right"
    );
    assert_eq!(walk.vm().memory()[0x3882], 26);
    capture(&walk, "mario-walk");
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn rubber_house_menu_settings_and_level_one() {
    let mut menu = load("LAVA1 橡皮屋/橡皮屋.lav");
    advance(&mut menu, 1800);
    assert_eq!(menu.vm().pc(), 0x377a, "the title menu waits for input");
    capture(&menu, "rubber-menu");

    let mut help = menu.clone();
    press(&mut help, 22);
    assert_ne!(
        help.display().indexed_frame(),
        menu.display().indexed_frame(),
        "Right moves the menu selection"
    );
    press(&mut help, 22);
    press(&mut help, 13);
    assert_eq!(help.vm().pc(), 0x2991, "the help entry opens the rules");
    capture(&help, "rubber-help");
    for (page, expected) in [0x29d7, 0x2a5e, 0x2ae7, 0x2b76, 0x2c0b, 0x2c8f, 0x2d24]
        .into_iter()
        .enumerate()
    {
        press(&mut help, 27);
        assert_eq!(
            help.vm().pc(),
            expected,
            "Escape advances to help page {}",
            page + 1
        );
    }
    press(&mut help, 27);
    assert_eq!(help.vm().pc(), 0x377a, "Escape on the last page returns");

    let mut settings = menu.clone();
    press(&mut settings, 22);
    press(&mut settings, 13);
    assert_eq!(settings.vm().pc(), 0x259a, "the settings entry opens");
    capture(&settings, "rubber-settings");
    press(&mut settings, 27);
    assert_eq!(settings.vm().pc(), 0x377a);

    let mut bind = menu.clone();
    press(&mut bind, 22);
    press(&mut bind, 13);
    press(&mut bind, 13);
    assert_eq!(bind.vm().pc(), 0x26da, "the key binding screen opens");
    capture(&bind, "rubber-bind");
    for key in [13, b'z', b'x', b'y'] {
        press(&mut bind, key);
    }
    assert_eq!(
        bind.vm().pc(),
        0x1472,
        "four keys fill the binding and ask for confirmation"
    );
    capture(&bind, "rubber-bind-confirm");
    press(&mut bind, b'n');
    assert_eq!(bind.vm().pc(), 0x259a, "N cancels the new bindings");

    let mut locked = menu.clone();
    for _ in 0..3 {
        press(&mut locked, 22);
    }
    press(&mut locked, 13);
    assert_eq!(locked.vm().pc(), 0x1472);
    capture(&locked, "rubber-locked");
    press(&mut locked, 13);
    assert_eq!(locked.vm().pc(), 0x377a);

    let mut exit = menu.clone();
    for _ in 0..4 {
        press(&mut exit, 22);
    }
    press(&mut exit, 13);
    assert_eq!(exit.vm().pc(), 0x1472);
    capture(&exit, "rubber-exit");
    exit.input_mut().set_keys([13]);
    let mut halted = false;
    for _ in 0..60 {
        let result = exit.run_frame().expect("game must execute without errors");
        if matches!(result.status, FrameStatus::Halted(_)) {
            halted = true;
            break;
        }
    }
    assert!(halted, "the exit item ends the program");
    assert_eq!(exit.vm().pc(), 0x2d3f);

    let mut emulator = menu.clone();
    press(&mut emulator, 13);
    assert_eq!(emulator.vm().pc(), 0x1472, "Start shows the level prompt");
    capture(&emulator, "rubber-level-prompt");
    let mut prompt = emulator.clone();
    press(&mut prompt, 22);
    assert_eq!(prompt.vm().pc(), 0x1472, "the prompt ignores Right");
    press(&mut emulator, 13);
    advance(&mut emulator, 300);
    assert_eq!(emulator.vm().pc(), 0x24dd, "level one waits for input");
    assert_eq!(emulator.vm().memory()[0x34db], 23, "level one has 23 beads");
    assert_eq!(
        emulator.vm().memory()[0x34e1],
        8,
        "the sprite starts at x 8"
    );
    assert_eq!(
        emulator.vm().memory()[0x34e2],
        8,
        "the sprite starts at y 8"
    );
    assert_eq!(
        emulator.vm().memory()[0x343f],
        5,
        "a bead sits at the first tile"
    );
    capture(&emulator, "rubber-level");

    let idle = emulator.clone();
    let mut still = emulator.clone();
    advance(&mut still, 60);
    assert_eq!(still.vm().memory()[0x34e1], 8);
    assert_eq!(still.vm().memory()[0x34e2], 8);
    assert_eq!(still.vm().memory()[0x34db], 23);
    assert_eq!(
        still.display().indexed_frame(),
        emulator.display().indexed_frame(),
        "the level does not animate while waiting"
    );

    let mut right = emulator.clone();
    hold(&mut right, 22, 30);
    assert_eq!(right.vm().memory()[0x34e1], 18, "Right walks right");
    assert_eq!(
        right.vm().memory()[0x34e2],
        8,
        "the sprite stays on its ledge"
    );
    assert_eq!(right.vm().memory()[0x34db], 22, "one bead is collected");
    assert_eq!(right.vm().memory()[0x343f], 0, "the bead tile is cleared");
    assert_ne!(
        right.display().indexed_frame(),
        idle.display().indexed_frame()
    );
    capture(&right, "rubber-right");

    let mut far = emulator.clone();
    hold(&mut far, 22, 60);
    assert_eq!(far.vm().memory()[0x34e1], 28, "a longer walk reaches x 28");
    assert_eq!(far.vm().memory()[0x34db], 21, "a second bead is collected");

    let mut left = emulator.clone();
    hold(&mut left, 23, 30);
    assert_eq!(left.vm().memory()[0x34e1], 0, "Left stops at the wall");
    assert_eq!(
        left.vm().memory()[0x34e2],
        20,
        "the sprite drops off its ledge"
    );
    assert_eq!(left.vm().memory()[0x34db], 23, "the drop collects no bead");
    capture(&left, "rubber-left");

    for key in [20, 21] {
        let mut vertical = emulator.clone();
        hold(&mut vertical, key, 60);
        assert_eq!(vertical.vm().memory()[0x34e1], 8, "vertical keys keep x");
        assert_eq!(vertical.vm().memory()[0x34e2], 8, "vertical keys keep y");
        assert_eq!(vertical.vm().memory()[0x34db], 23);
        assert_eq!(
            vertical.display().indexed_frame(),
            idle.display().indexed_frame(),
            "only the left and right keys move the sprite"
        );
    }

    let mut pause = emulator.clone();
    press(&mut pause, 27);
    assert_eq!(pause.vm().pc(), 0x1472, "Escape asks whether to quit");
    capture(&pause, "rubber-pause");
    let mut resume = pause.clone();
    press(&mut resume, b'n');
    assert_eq!(resume.vm().pc(), 0x24dd, "N resumes the level");
    let mut quit = pause.clone();
    press(&mut quit, b'y');
    assert_eq!(quit.vm().pc(), 0x377a, "Y returns to the title menu");
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn happy_gomoku_menu_places_stones_undoes_restarts_and_detects_a_win() {
    let stone = |emulator: &Emulator, col: usize, row: usize| {
        emulator.vm().memory()[0x21b4 + col * 30 + row * 2]
    };

    let mut menu = load("LAVA1 欢乐五子棋/欢乐五子棋.lav");
    advance(&mut menu, 300);
    assert_eq!(menu.vm().pc(), 0x1269, "the title menu waits for input");
    capture(&menu, "gomoku-menu");

    let mut second = menu.clone();
    press(&mut second, 21);
    assert_ne!(
        second.display().indexed_frame(),
        menu.display().indexed_frame(),
        "Down moves the selection to the second mode"
    );
    let mut up = menu.clone();
    press(&mut up, 20);
    assert_eq!(
        up.display().indexed_frame(),
        second.display().indexed_frame(),
        "Up selects the same second mode"
    );
    press(&mut second, 21);
    assert_eq!(
        second.display().indexed_frame(),
        menu.display().indexed_frame(),
        "Down wraps back to the first mode"
    );
    for key in [22, 23] {
        let mut sideways = menu.clone();
        press(&mut sideways, key);
        assert_eq!(
            sideways.display().indexed_frame(),
            menu.display().indexed_frame(),
            "Left and Right do not change the menu selection"
        );
    }

    let mut solo = menu.clone();
    press(&mut solo, 13);
    assert_eq!(solo.vm().pc(), 0x71f, "the solo board waits for the cursor");
    assert_eq!(solo.vm().memory()[0x21ac], 42, "the cursor starts at x 42");
    assert_eq!(solo.vm().memory()[0x21ae], 36, "the cursor starts at y 36");
    capture(&solo, "gomoku-board");

    for (key, expected) in [
        (23u8, (78u8, 36u8)),
        (22, (6, 36)),
        (20, (42, 72)),
        (21, (42, 0)),
    ] {
        let mut moved = solo.clone();
        for _ in 0..7 {
            press(&mut moved, key);
        }
        let memory = moved.vm().memory();
        assert_eq!(
            (memory[0x21ac], memory[0x21ae]),
            expected,
            "seven steps wrap the cursor around the 13-point board"
        );
        assert_ne!(
            moved.display().indexed_frame(),
            solo.display().indexed_frame()
        );
    }

    let mut first = solo.clone();
    press(&mut first, 13);
    assert_eq!(
        first.vm().memory()[0x21b6],
        8,
        "the stone lands in column 8"
    );
    assert_eq!(first.vm().memory()[0x21b8], 8, "the stone lands in row 8");
    assert_eq!(stone(&first, 8, 8), 1, "the player places a black stone");
    assert_eq!(stone(&first, 7, 7), 2, "the AI answers up-left");
    capture(&first, "gomoku-first");

    let mut occupied = first.clone();
    press(&mut occupied, 13);
    assert_eq!(
        occupied.display().indexed_frame(),
        first.display().indexed_frame(),
        "an occupied point rejects another stone"
    );

    let mut undo_first = first.clone();
    press(&mut undo_first, b'h');
    assert_eq!(stone(&undo_first, 8, 8), 0, "undo removes the player stone");
    assert_eq!(stone(&undo_first, 7, 7), 0, "undo removes the AI reply too");
    capture(&undo_first, "gomoku-undo");

    let mut second_round = first.clone();
    press(&mut second_round, 22);
    press(&mut second_round, 13);
    assert_eq!(
        stone(&second_round, 8, 7),
        1,
        "the second player stone follows the AI cursor"
    );
    assert_eq!(stone(&second_round, 8, 6), 2, "the AI answers above it");
    let mut undo_second = second_round.clone();
    press(&mut undo_second, b'h');
    assert_eq!(stone(&undo_second, 8, 8), 1, "undo keeps the earlier round");
    assert_eq!(stone(&undo_second, 7, 7), 2);
    assert_eq!(stone(&undo_second, 8, 7), 0);
    assert_eq!(stone(&undo_second, 8, 6), 0);

    let mut restart = second_round.clone();
    press(&mut restart, b'r');
    assert_eq!(
        restart.display().indexed_frame(),
        solo.display().indexed_frame(),
        "R clears the board back to the fresh position"
    );
    assert_eq!(restart.vm().memory()[0x21ac], 42);

    let mut duo = menu.clone();
    press(&mut duo, 21);
    press(&mut duo, 13);
    assert_eq!(
        duo.vm().pc(),
        0x71f,
        "the two-player board waits for the cursor"
    );
    let moves: [&[u8]; 9] = [
        &[23, 23, 23, 23, 13],
        &[20, 20, 20, 20, 13],
        &[21, 21, 21, 21, 22, 13],
        &[20, 20, 20, 20, 13],
        &[21, 21, 21, 21, 22, 13],
        &[20, 20, 20, 20, 13],
        &[21, 21, 21, 21, 22, 13],
        &[20, 20, 20, 20, 13],
        &[21, 21, 21, 21, 22, 13],
    ];
    for keys in moves {
        for &key in keys {
            press(&mut duo, key);
        }
    }
    assert_eq!(duo.vm().pc(), 0x288d, "five black stones end the game");
    assert_eq!(stone(&duo, 8, 8), 1, "the winning stone stays on the board");
    let text = duo.vm().memory();
    assert_eq!(
        &text[0x1c7a..0x1c82],
        b"\xba\xda\xc6\xe5win!",
        "the board announces a black win"
    );
    capture(&duo, "gomoku-win");

    advance(&mut duo, 200);
    press(&mut duo, 13);
    assert_eq!(
        duo.vm().pc(),
        0x71f,
        "a key after the win starts a new game"
    );
    assert_eq!(stone(&duo, 8, 8), 0, "the new game starts empty");
    assert_eq!(
        &duo.vm().memory()[0x1c83..0x1c88],
        b"win:1",
        "the black win counter is kept for the rematch"
    );
    press(&mut duo, 13);
    assert_eq!(stone(&duo, 8, 8), 2, "white starts the next game");
    capture(&duo, "gomoku-rematch");
}

#[test]
#[ignore = "requires the local LavaX game corpus"]
fn wuxia_title_menu_save_slots_and_story_intro() {
    let frame = |emulator: &Emulator| emulator.display().indexed_frame().to_vec();
    let green = |emulator: &Emulator| {
        emulator
            .display()
            .indexed_frame()
            .iter()
            .filter(|&&pixel| pixel == 0)
            .count()
    };

    let mut menu = load("LAVA1 武侠/武侠.lav");
    advance(&mut menu, 90);
    let title = frame(&menu);
    capture(&menu, "wuxia-title");

    let mut navigation = menu.clone();
    press(&mut navigation, 21);
    let second = frame(&navigation);
    assert_ne!(second, title, "Down highlights the second entry");
    capture(&navigation, "wuxia-title-continue");
    press(&mut navigation, 21);
    let third = frame(&navigation);
    assert_ne!(third, second, "Down highlights the third entry");
    press(&mut navigation, 21);
    assert_eq!(
        frame(&navigation),
        title,
        "Down wraps back to the first entry"
    );
    press(&mut navigation, 20);
    assert_eq!(
        frame(&navigation),
        third,
        "Up wraps back to the third entry"
    );

    let mut save = menu.clone();
    press(&mut save, 21);
    assert_eq!(frame(&save), second, "the title keeps 游龙再现 selected");
    press(&mut save, 13);
    assert_ne!(frame(&save), second, "Enter opens the load menu");
    let load_menu = frame(&save);
    capture(&save, "wuxia-load-menu");
    press(&mut save, 13);
    advance(&mut save, 60);
    assert_ne!(
        frame(&save),
        load_menu,
        "an empty slot reports a load failure"
    );
    let failure = frame(&save);
    capture(&save, "wuxia-load-failed");
    advance(&mut save, 120);
    assert_eq!(frame(&save), failure, "the failure message stays on screen");
    press(&mut save, 27);
    advance(&mut save, 30);
    assert_eq!(
        frame(&save),
        second,
        "Escape returns to the title with the same entry"
    );

    let mut story = menu.clone();
    press(&mut story, 13);
    let night = frame(&story);
    assert_eq!(green(&story), 63, "the night scene only shows the tree");
    assert_ne!(night, title, "初涉武林 leaves the title screen");
    capture(&story, "wuxia-night");
    press(&mut story, 21);
    let first_page = frame(&story);
    assert_ne!(first_page, night, "Down starts the story text");
    assert!(
        green(&story) > 3_000,
        "the night scene gains the dialog box"
    );
    capture(&story, "wuxia-story-night");
    for _ in 0..5 {
        press(&mut story, 21);
    }
    let room = frame(&story);
    assert_ne!(room, first_page, "the story changes the scene");
    assert!(green(&story) > 7_000, "the room and the portrait are drawn");
    capture(&story, "wuxia-room");
    press(&mut story, 21);
    assert_ne!(frame(&story), room, "Down advances the room dialog");
    capture(&story, "wuxia-room-dialog");

    let mut quit = menu.clone();
    press(&mut quit, 21);
    press(&mut quit, 21);
    for _ in 0..3 {
        quit.input_mut().set_keys([13]);
        let _ = quit.run_frame().unwrap();
    }
    quit.input_mut().set_keys([]);
    let mut exit_code = None;
    for _ in 0..400 {
        let result = quit.run_frame().unwrap();
        if let FrameStatus::Halted(code) = result.status {
            exit_code = Some(code);
            break;
        }
    }
    assert_eq!(exit_code, Some(0), "豹隐山林 exits the program");
}

#[test]
#[ignore = "requires the local LavaX game corpus"]
fn pipe_works_menu_screens_gameplay_and_exit() {
    let frame = |emulator: &Emulator| emulator.display().indexed_frame().to_vec();
    let region = |emulator: &Emulator, x0: usize, y0: usize, x1: usize, y1: usize| {
        let display = emulator.display();
        let frame = display.indexed_frame();
        let width = display.width() as usize;
        let mut pixels = Vec::with_capacity((x1 - x0) * (y1 - y0));
        for row in y0..y1 {
            pixels.extend_from_slice(&frame[row * width + x0..row * width + x1]);
        }
        pixels
    };
    let selected = |emulator: &mut Emulator| {
        let cells = [
            (47u32, 25u32),
            (100, 25),
            (47, 41),
            (100, 41),
            (47, 57),
            (100, 57),
        ];
        let mut darkest = [0usize; 6];
        for _ in 0..14 {
            {
                let display = emulator.display();
                let frame = display.indexed_frame();
                let width = display.width() as u32;
                for (index, (x, y)) in cells.iter().enumerate() {
                    let mut black = 0;
                    for row in *y..y + 11 {
                        for column in *x..x + 47 {
                            let pixel = frame[(row * width + column) as usize];
                            if display.palette()[pixel as usize] == [0, 0, 0] {
                                black += 1;
                            }
                        }
                    }
                    darkest[index] = darkest[index].max(black);
                }
            }
            let _ = emulator.run_frame().unwrap();
        }
        let mut best = 0;
        for index in 1..6 {
            if darkest[index] > darkest[best] {
                best = index;
            }
        }
        best
    };

    let mut menu = load("LAVA1 水管工/PC模拟器端/水管工.lav");
    advance(&mut menu, 90);
    assert_eq!(menu.vm().pc(), 0x479f, "the title waits in the menu loop");
    let title = frame(&menu);
    capture(&menu, "pipe-title");
    press(&mut menu, 27);
    assert_eq!(menu.vm().pc(), 0x479f, "Escape keeps the title open");

    let mut navigation = menu.clone();
    assert_eq!(
        selected(&mut navigation),
        0,
        "开始游戏 is highlighted first"
    );
    press(&mut navigation, 21);
    assert_eq!(selected(&mut navigation), 2, "Down moves to 游戏手册");
    press(&mut navigation, 21);
    assert_eq!(selected(&mut navigation), 4, "Down moves to 榜上有名");
    press(&mut navigation, 21);
    assert_eq!(selected(&mut navigation), 4, "Down stops on 榜上有名");
    press(&mut navigation, 22);
    assert_eq!(selected(&mut navigation), 5, "Right moves to 结束游戏");
    press(&mut navigation, 20);
    assert_eq!(selected(&mut navigation), 3, "Up moves to 关于游戏");
    press(&mut navigation, 23);
    assert_eq!(selected(&mut navigation), 2, "Left moves to 游戏手册");
    press(&mut navigation, 22);
    assert_eq!(selected(&mut navigation), 3, "Right moves to 关于游戏");
    press(&mut navigation, 21);
    assert_eq!(selected(&mut navigation), 5, "Down moves to 结束游戏");
    press(&mut navigation, 22);
    assert_eq!(selected(&mut navigation), 4, "Right wraps to 榜上有名");
    press(&mut navigation, 20);
    assert_eq!(selected(&mut navigation), 2, "Up moves to 游戏手册");
    press(&mut navigation, 23);
    assert_eq!(selected(&mut navigation), 1, "Left moves to 游戏设定");
    press(&mut navigation, 23);
    assert_eq!(selected(&mut navigation), 0, "Left moves back to 开始游戏");
    press(&mut navigation, 20);
    assert_eq!(selected(&mut navigation), 0, "Up stops on 开始游戏");

    let mut manual = menu.clone();
    press(&mut manual, 21);
    press(&mut manual, 13);
    assert_eq!(manual.vm().pc(), 0x444b, "游戏手册 opens the manual");
    assert_ne!(frame(&manual), title, "the manual replaces the title");
    capture(&manual, "pipe-manual");
    press(&mut manual, 21);
    assert_eq!(manual.vm().pc(), 0x479f, "Down leaves the manual");
    assert_eq!(selected(&mut manual), 0, "the title restarts on 开始游戏");

    let mut hall = menu.clone();
    press(&mut hall, 21);
    press(&mut hall, 21);
    press(&mut hall, 13);
    assert_eq!(hall.vm().pc(), 0x2d26, "榜上有名 opens the high score list");
    assert_ne!(
        frame(&hall),
        title,
        "the high score list replaces the title"
    );
    capture(&hall, "pipe-hall");
    press(&mut hall, 13);
    assert_eq!(
        hall.vm().pc(),
        0x479f,
        "Enter returns from the high score list"
    );

    let mut settings = menu.clone();
    press(&mut settings, 22);
    press(&mut settings, 13);
    assert!(
        (0x4380..=0x43ff).contains(&settings.vm().pc()),
        "游戏设定 opens the difficulty selector"
    );
    let easy = frame(&settings);
    capture(&settings, "pipe-settings");
    press(&mut settings, 21);
    assert!(
        (0x4380..=0x43ff).contains(&settings.vm().pc()),
        "Down stays in the difficulty selector"
    );
    let medium = frame(&settings);
    assert_ne!(medium, easy, "Down moves the difficulty marker");
    capture(&settings, "pipe-settings-medium");
    press(&mut settings, 21);
    assert_ne!(frame(&settings), medium, "Down moves the marker to 困难");
    press(&mut settings, 13);
    assert_eq!(settings.vm().pc(), 0x479f, "Enter confirms the difficulty");

    let mut about = menu.clone();
    press(&mut about, 22);
    press(&mut about, 21);
    press(&mut about, 13);
    assert_eq!(about.vm().pc(), 0x453b, "关于游戏 opens the about page");
    let first_page = frame(&about);
    capture(&about, "pipe-about");
    press(&mut about, 21);
    assert_eq!(about.vm().pc(), 0x4632, "Down shows the second about page");
    let second_page = frame(&about);
    assert_ne!(second_page, first_page, "the second page changes the text");
    capture(&about, "pipe-about-page2");
    press(&mut about, 21);
    assert_eq!(about.vm().pc(), 0x479f, "Down leaves the about screen");

    let mut game = menu.clone();
    press(&mut game, 13);
    let splash = frame(&game);
    assert!(
        matches!(game.vm().pc(), 0x2803 | 0x2826),
        "开始游戏 shows the studio splash"
    );
    assert_ne!(splash, title, "the studio splash replaces the title");
    capture(&game, "pipe-splash");
    advance(&mut game, 240);
    assert!(game.vm().is_running(), "the board keeps running");
    assert_ne!(game.vm().pc(), 0x479f, "the splash gives way to the board");
    assert!(
        !matches!(game.vm().pc(), 0x2803 | 0x2826),
        "the splash has finished"
    );
    let board = frame(&game);
    assert_ne!(board, splash, "the pipe board replaces the splash");
    capture(&game, "pipe-game");

    let timer = region(&game, 139, 48, 157, 74);
    advance(&mut game, 40);
    assert_ne!(
        region(&game, 139, 48, 157, 74),
        timer,
        "the plumber timer keeps animating"
    );

    let cursor = region(&game, 56, 0, 132, 28);
    advance(&mut game, 40);
    assert_eq!(
        region(&game, 56, 0, 132, 28),
        cursor,
        "the board cursor stays put without input"
    );
    press(&mut game, 22);
    let after_right = region(&game, 56, 0, 132, 28);
    assert_ne!(after_right, cursor, "Right moves the cursor");
    capture(&game, "pipe-game-cursor");
    press(&mut game, 21);
    assert_ne!(
        region(&game, 56, 0, 132, 28),
        after_right,
        "Down moves the cursor again"
    );

    press(&mut game, 27);
    assert_eq!(game.vm().pc(), 0x479f, "Escape returns to the title");
    assert_eq!(selected(&mut game), 0, "the title resets to 开始游戏");

    let mut quit = menu.clone();
    press(&mut quit, 22);
    press(&mut quit, 21);
    press(&mut quit, 21);
    press(&mut quit, 13);
    assert!(
        matches!(quit.vm().pc(), 0x2803 | 0x2826),
        "结束游戏 shows the studio splash"
    );
    let mut exit_code = None;
    for _ in 0..400 {
        let result = quit.run_frame().unwrap();
        if let FrameStatus::Halted(code) = result.status {
            exit_code = Some(code);
            break;
        }
    }
    assert_eq!(exit_code, Some(0), "结束游戏 exits the program");
}

fn pipe_works_pixels(emulator: &Emulator, x0: usize, y0: usize, x1: usize, y1: usize) -> Vec<u8> {
    let display = emulator.display();
    let frame = display.indexed_frame();
    let width = display.width() as usize;
    let mut pixels = Vec::with_capacity((x1 - x0) * (y1 - y0));
    for row in y0..y1 {
        pixels.extend_from_slice(&frame[row * width + x0..row * width + x1]);
    }
    pixels
}

fn pipe_works_selected(emulator: &mut Emulator) -> usize {
    let cells = [
        (47u32, 25u32),
        (100, 25),
        (47, 41),
        (100, 41),
        (47, 57),
        (100, 57),
    ];
    let mut darkest = [0usize; 6];
    for _ in 0..14 {
        {
            let display = emulator.display();
            let frame = display.indexed_frame();
            let width = display.width() as u32;
            for (index, (x, y)) in cells.iter().enumerate() {
                let mut black = 0;
                for row in *y..y + 11 {
                    for column in *x..x + 47 {
                        let pixel = frame[(row * width + column) as usize];
                        if display.palette()[pixel as usize] == [0, 0, 0] {
                            black += 1;
                        }
                    }
                }
                darkest[index] = darkest[index].max(black);
            }
        }
        let _ = emulator.run_frame().unwrap();
    }
    let mut best = 0;
    for index in 1..6 {
        if darkest[index] > darkest[best] {
            best = index;
        }
    }
    best
}

#[test]
#[ignore = "requires the local LavaX game corpus"]
fn pipe_works_wqx_menu_screens_gameplay_and_exit() {
    let frame = |emulator: &Emulator| emulator.display().indexed_frame().to_vec();

    let mut menu = load("LAVA1 水管工/文曲星端/水管工.lav");
    advance(&mut menu, 90);
    assert_eq!(menu.vm().pc(), 0x4797, "the title waits in the menu loop");
    let title = frame(&menu);
    capture(&menu, "pipe-wqx-title");
    press(&mut menu, 27);
    assert_eq!(menu.vm().pc(), 0x4797, "Escape keeps the title open");

    let mut navigation = menu.clone();
    assert_eq!(pipe_works_selected(&mut navigation), 0, "开始游戏 is first");
    press(&mut navigation, 21);
    assert_eq!(
        pipe_works_selected(&mut navigation),
        2,
        "Down moves to 游戏手册"
    );
    press(&mut navigation, 21);
    assert_eq!(
        pipe_works_selected(&mut navigation),
        4,
        "Down moves to 榜上有名"
    );
    press(&mut navigation, 21);
    assert_eq!(
        pipe_works_selected(&mut navigation),
        4,
        "Down stops on 榜上有名"
    );
    press(&mut navigation, 22);
    assert_eq!(
        pipe_works_selected(&mut navigation),
        5,
        "Right moves to 结束游戏"
    );
    press(&mut navigation, 20);
    assert_eq!(
        pipe_works_selected(&mut navigation),
        3,
        "Up moves to 关于游戏"
    );
    press(&mut navigation, 23);
    assert_eq!(
        pipe_works_selected(&mut navigation),
        2,
        "Left moves to 游戏手册"
    );
    press(&mut navigation, 22);
    assert_eq!(
        pipe_works_selected(&mut navigation),
        3,
        "Right moves to 关于游戏"
    );
    press(&mut navigation, 21);
    assert_eq!(
        pipe_works_selected(&mut navigation),
        5,
        "Down moves to 结束游戏"
    );
    press(&mut navigation, 22);
    assert_eq!(
        pipe_works_selected(&mut navigation),
        4,
        "Right wraps to 榜上有名"
    );
    press(&mut navigation, 20);
    assert_eq!(
        pipe_works_selected(&mut navigation),
        2,
        "Up moves to 游戏手册"
    );

    let mut manual = menu.clone();
    press(&mut manual, 21);
    press(&mut manual, 13);
    assert_eq!(manual.vm().pc(), 0x4443, "游戏手册 opens the manual");
    assert_ne!(frame(&manual), title, "the manual replaces the title");
    capture(&manual, "pipe-wqx-manual");
    press(&mut manual, 21);
    assert_eq!(manual.vm().pc(), 0x4797, "Down leaves the manual");

    let mut hall = menu.clone();
    press(&mut hall, 21);
    press(&mut hall, 21);
    press(&mut hall, 13);
    assert_eq!(hall.vm().pc(), 0x2d1f, "榜上有名 opens the high score list");
    assert_ne!(
        frame(&hall),
        title,
        "the high score list replaces the title"
    );
    capture(&hall, "pipe-wqx-hall");
    press(&mut hall, 13);
    assert_eq!(
        hall.vm().pc(),
        0x4797,
        "Enter returns from the high score list"
    );

    let mut settings = menu.clone();
    press(&mut settings, 22);
    press(&mut settings, 13);
    assert!(
        (0x4380..=0x4430).contains(&settings.vm().pc()),
        "游戏设定 opens the difficulty selector"
    );
    let easy = frame(&settings);
    capture(&settings, "pipe-wqx-settings");
    press(&mut settings, 21);
    let medium = frame(&settings);
    assert_ne!(medium, easy, "Down moves the difficulty marker");
    press(&mut settings, 21);
    assert_ne!(frame(&settings), medium, "Down moves the marker to 困难");
    press(&mut settings, 13);
    assert_eq!(settings.vm().pc(), 0x4797, "Enter confirms the difficulty");

    let mut about = menu.clone();
    press(&mut about, 22);
    press(&mut about, 21);
    press(&mut about, 13);
    assert_eq!(about.vm().pc(), 0x4533, "关于游戏 opens the about page");
    let first_page = frame(&about);
    capture(&about, "pipe-wqx-about");
    press(&mut about, 21);
    assert_eq!(about.vm().pc(), 0x462a, "Down shows the second about page");
    assert_ne!(
        frame(&about),
        first_page,
        "the second page changes the text"
    );
    press(&mut about, 21);
    assert_eq!(about.vm().pc(), 0x4797, "Down leaves the about screen");

    let mut game = menu.clone();
    press(&mut game, 13);
    assert!(
        matches!(game.vm().pc(), 0x27fc | 0x281f),
        "开始游戏 shows the studio splash"
    );
    let splash = frame(&game);
    capture(&game, "pipe-wqx-splash");
    advance(&mut game, 240);
    assert!(game.vm().is_running(), "the board keeps running");
    assert_ne!(game.vm().pc(), 0x4797, "the splash gives way to the board");
    assert!(
        !matches!(game.vm().pc(), 0x27fc | 0x281f),
        "the splash has finished"
    );
    assert_ne!(frame(&game), splash, "the pipe board replaces the splash");
    capture(&game, "pipe-wqx-game");

    let timer = pipe_works_pixels(&game, 139, 48, 157, 74);
    advance(&mut game, 40);
    assert_ne!(
        pipe_works_pixels(&game, 139, 48, 157, 74),
        timer,
        "the plumber timer keeps animating"
    );
    let cursor = pipe_works_pixels(&game, 56, 0, 132, 28);
    advance(&mut game, 40);
    assert_eq!(
        pipe_works_pixels(&game, 56, 0, 132, 28),
        cursor,
        "the board cursor stays put without input"
    );
    press(&mut game, 22);
    let after_right = pipe_works_pixels(&game, 56, 0, 132, 28);
    assert_ne!(after_right, cursor, "Right moves the cursor");
    capture(&game, "pipe-wqx-game-cursor");
    press(&mut game, 21);
    assert_ne!(
        pipe_works_pixels(&game, 56, 0, 132, 28),
        after_right,
        "Down moves the cursor again"
    );
    press(&mut game, 27);
    assert_eq!(game.vm().pc(), 0x4797, "Escape returns to the title");

    let mut quit = menu.clone();
    press(&mut quit, 22);
    press(&mut quit, 21);
    press(&mut quit, 21);
    quit.input_mut().set_keys([13]);
    let mut exit_code = None;
    for _ in 0..400 {
        let result = quit.run_frame().unwrap();
        if let FrameStatus::Halted(code) = result.status {
            exit_code = Some(code);
            break;
        }
    }
    quit.input_mut().release_all();
    assert_eq!(exit_code, Some(0), "结束游戏 exits the program");
}

fn changed_region(before: &[u8], after: &[u8], width: usize) -> (usize, usize, usize) {
    let mut count = 0;
    let mut top = usize::MAX;
    let mut bottom = 0;
    for (index, (left, right)) in before.iter().zip(after).enumerate() {
        if left != right {
            count += 1;
            let row = index / width;
            top = top.min(row);
            bottom = bottom.max(row);
        }
    }
    (count, top, bottom)
}

#[test]
#[ignore = "requires the local LavaX game corpus"]
fn jianghu_pirate_title_intro_field_and_episode_branches() {
    let frame = |emulator: &Emulator| emulator.display().indexed_frame().to_vec();
    let mut menu = load("LAVA1 江湖--海贼王传说/world.lav");
    advance(&mut menu, 400);
    assert_eq!(menu.vm().pc(), 0x39e, "the title waits for a key");
    let width = usize::from(menu.display().width());
    let title = frame(&menu);
    capture(&menu, "jianghu-pirate-title");

    let mut navigation = menu.clone();
    press(&mut navigation, 21);
    let second = frame(&navigation);
    assert_ne!(second, title, "Down selects 渔龙再现");
    press(&mut navigation, 21);
    let third = frame(&navigation);
    assert_ne!(third, second, "Down selects 豹隐山林");
    press(&mut navigation, 21);
    assert_eq!(frame(&navigation), title, "Down wraps to 初涉江湖");
    press(&mut navigation, 20);
    assert_eq!(frame(&navigation), third, "Up wraps to 豹隐山林");
    press(&mut navigation, 23);
    assert_eq!(frame(&navigation), second, "Left returns to 渔龙再现");
    press(&mut navigation, 22);
    assert_eq!(frame(&navigation), third, "Right returns to 豹隐山林");

    let mut loading = menu.clone();
    press(&mut loading, 21);
    press(&mut loading, 13);
    advance(&mut loading, 120);
    assert_eq!(
        loading.vm().pc(),
        0x39e,
        "the failed load returns to the title"
    );
    let failure = frame(&loading);
    assert_ne!(
        failure, second,
        "the missing save is reported over the menu"
    );
    capture(&loading, "jianghu-pirate-save-failure");
    press(&mut loading, 13);
    assert_eq!(frame(&loading), second, "a key dismisses the report");
    press(&mut loading, 13);
    assert_eq!(
        frame(&loading),
        failure,
        "choosing 渔龙再现 retries the load"
    );

    let mut quit = menu.clone();
    press(&mut quit, 21);
    press(&mut quit, 21);
    for _ in 0..3 {
        quit.input_mut().set_keys([13]);
        quit.run_frame().unwrap();
    }
    quit.input_mut().set_keys([]);
    let mut exit_code = None;
    for _ in 0..200 {
        let result = quit.run_frame().unwrap();
        if let FrameStatus::Halted(code) = result.status {
            exit_code = Some(code);
            break;
        }
    }
    assert_eq!(exit_code, Some(0), "豹隐山林 fades out and exits");

    let mut game = menu.clone();
    press(&mut game, 13);
    assert_eq!(game.vm().pc(), 0x39e, "the intro dialog waits for a key");
    let first_page = frame(&game);
    assert_ne!(first_page, title, "初涉江湖 opens the intro scene");
    capture(&game, "jianghu-pirate-dialog");
    press(&mut game, 13);
    assert_ne!(frame(&game), first_page, "Enter advances the dialog");
    let mut presses = 2;
    while game.vm().pc() == 0x39e {
        assert!(presses < 12, "the intro must reach the field");
        press(&mut game, 13);
        presses += 1;
    }
    assert_eq!(game.vm().pc(), 0x8b7e, "the intro reaches the field");
    let field = frame(&game);
    capture(&game, "jianghu-pirate-field");

    let mut idle = game.clone();
    advance(&mut idle, 60);
    assert_eq!(frame(&idle), field, "the field waits for input");

    let mut moved = game.clone();
    moved.input_mut().set_keys([22]);
    advance(&mut moved, 20);
    moved.input_mut().set_keys([]);
    advance(&mut moved, 10);
    let right = frame(&moved);
    let (count, top, bottom) = changed_region(&field, &right, width);
    assert!(count > 0 && count < 400, "Right moves the player sprite");
    assert!(
        top >= 30 && bottom <= 50,
        "Right keeps the player on the same rows"
    );
    capture(&moved, "jianghu-pirate-right");

    let mut up = game.clone();
    up.input_mut().set_keys([20]);
    advance(&mut up, 20);
    up.input_mut().set_keys([]);
    advance(&mut up, 10);
    let (count, top, _) = changed_region(&field, &frame(&up), width);
    assert!(count > 0 && count < 400, "Up moves the player sprite");
    assert!(top < 30, "Up moves the player to an upper row");
    capture(&up, "jianghu-pirate-up");
}

#[test]
#[ignore = "requires the local LavaX game corpus"]
fn jianghu_chaos_title_intro_field_and_save_branches() {
    let frame = |emulator: &Emulator| emulator.display().indexed_frame().to_vec();
    let mut menu = load("LAVA1 江湖-混沌初开/world.lav");
    advance(&mut menu, 400);
    assert_eq!(menu.vm().pc(), 0x2ef, "the title waits for a key");
    let width = usize::from(menu.display().width());
    let title = frame(&menu);
    capture(&menu, "jianghu-chaos-title");

    let mut navigation = menu.clone();
    press(&mut navigation, 21);
    let second = frame(&navigation);
    assert_ne!(second, title, "Down selects 再续前缘");
    press(&mut navigation, 21);
    let third = frame(&navigation);
    assert_ne!(third, second, "Down selects 退隐江湖");
    press(&mut navigation, 21);
    assert_eq!(frame(&navigation), title, "Down wraps to 新的征程");
    press(&mut navigation, 20);
    assert_eq!(frame(&navigation), third, "Up wraps to 退隐江湖");
    press(&mut navigation, 23);
    assert_eq!(frame(&navigation), second, "Left returns to 再续前缘");
    press(&mut navigation, 22);
    assert_eq!(frame(&navigation), third, "Right returns to 退隐江湖");

    let mut loading = menu.clone();
    press(&mut loading, 21);
    press(&mut loading, 13);
    assert_eq!(loading.vm().pc(), 0x11fa, "再续前缘 opens the save slots");
    let slots = frame(&loading);
    assert_ne!(slots, second, "the slot menu replaces the title");
    press(&mut loading, 13);
    let failure = frame(&loading);
    assert_ne!(failure, slots, "the missing save is reported over the menu");
    capture(&loading, "jianghu-chaos-save-failure");
    advance(&mut loading, 180);
    assert_eq!(frame(&loading), failure, "the report waits for a key");
    press(&mut loading, 13);
    assert_eq!(frame(&loading), second, "a key returns to the title");

    let mut quit = menu.clone();
    press(&mut quit, 21);
    press(&mut quit, 21);
    for _ in 0..3 {
        quit.input_mut().set_keys([13]);
        quit.run_frame().unwrap();
    }
    quit.input_mut().set_keys([]);
    let mut exit_code = None;
    for _ in 0..400 {
        let result = quit.run_frame().unwrap();
        if let FrameStatus::Halted(code) = result.status {
            exit_code = Some(code);
            break;
        }
    }
    assert_eq!(exit_code, Some(0), "退隐江湖 fades out and exits");

    let mut game = menu.clone();
    press(&mut game, 13);
    assert_eq!(game.vm().pc(), 0x2ef, "the intro dialog waits for a key");
    let first_page = frame(&game);
    assert_ne!(first_page, title, "新的征程 opens the intro scene");
    capture(&game, "jianghu-chaos-dialog");
    press(&mut game, 13);
    assert_ne!(frame(&game), first_page, "Enter advances the dialog");
    let mut presses = 2;
    while game.vm().pc() == 0x2ef {
        assert!(presses < 12, "the intro must reach the field");
        press(&mut game, 13);
        presses += 1;
    }
    assert_eq!(game.vm().pc(), 0x8b20, "the intro reaches the field");
    let field = frame(&game);
    capture(&game, "jianghu-chaos-field");

    let mut idle = game.clone();
    advance(&mut idle, 60);
    assert_eq!(frame(&idle), field, "the field waits for input");

    let mut moved = game.clone();
    moved.input_mut().set_keys([22]);
    advance(&mut moved, 20);
    moved.input_mut().set_keys([]);
    advance(&mut moved, 10);
    let right = frame(&moved);
    let (count, top, bottom) = changed_region(&field, &right, width);
    assert!(count > 0 && count < 400, "Right moves the player sprite");
    assert!(
        top >= 30 && bottom <= 50,
        "Right keeps the player on the same rows"
    );
    capture(&moved, "jianghu-chaos-right");

    let mut up = game.clone();
    up.input_mut().set_keys([20]);
    advance(&mut up, 20);
    up.input_mut().set_keys([]);
    advance(&mut up, 10);
    let (count, top, _) = changed_region(&field, &frame(&up), width);
    assert!(count > 0 && count < 400, "Up moves the player sprite");
    assert!(top < 30, "Up moves the player to an upper row");
    capture(&up, "jianghu-chaos-up");
}

#[test]
#[ignore = "requires the local LavaX game corpus"]
fn wangwang_tycoon_title_settings_slots_and_board_turns() {
    let frame = |emulator: &Emulator| emulator.display().indexed_frame().to_vec();
    let mut menu = load("LAVA1 汪汪大富翁/wangwang.lav");
    advance(&mut menu, 400);
    assert_eq!(menu.vm().pc(), 0x227a, "the title waits for a key");
    let title = frame(&menu);
    capture(&menu, "wangwang-title");

    let mut navigation = menu.clone();
    press(&mut navigation, 21);
    let settings_title = frame(&navigation);
    assert_ne!(settings_title, title, "Down selects 系统设置");
    press(&mut navigation, 21);
    let manual_title = frame(&navigation);
    assert_ne!(manual_title, settings_title, "Down selects 软件说明");
    press(&mut navigation, 21);
    let exit_title = frame(&navigation);
    assert_ne!(exit_title, manual_title, "Down selects 退出游戏");
    press(&mut navigation, 21);
    assert_eq!(frame(&navigation), title, "Down wraps to 进入游戏");
    press(&mut navigation, 20);
    assert_eq!(frame(&navigation), exit_title, "Up wraps to 退出游戏");

    let mut arrows = menu.clone();
    press(&mut arrows, 22);
    assert_eq!(frame(&arrows), title, "Right leaves the title unchanged");
    press(&mut arrows, 23);
    assert_eq!(frame(&arrows), title, "Left leaves the title unchanged");

    let mut manual = menu.clone();
    press(&mut manual, 21);
    press(&mut manual, 21);
    press(&mut manual, 13);
    let manual_page = frame(&manual);
    assert_ne!(manual_page, manual_title, "软件说明 opens the manual");
    capture(&manual, "wangwang-manual");
    press(&mut manual, 13);
    assert_eq!(frame(&manual), manual_title, "Enter closes the manual");

    let mut settings = menu.clone();
    press(&mut settings, 21);
    press(&mut settings, 13);
    let settings_page = frame(&settings);
    assert_ne!(settings_page, settings_title, "系统设置 opens the settings");

    let mut character = settings.clone();
    press(&mut character, 13);
    let rows = frame(&character);
    assert_ne!(rows, settings_page, "角色设定 opens the character list");
    let mut mode = character.clone();
    press(&mut mode, 22);
    let toggled = frame(&mode);
    assert_ne!(toggled, rows, "Right changes the selected mode");
    press(&mut mode, 23);
    assert_eq!(frame(&mode), rows, "Left restores the mode");
    press(&mut mode, 21);
    assert_ne!(frame(&mode), rows, "Down selects 张经理");
    capture(&mode, "wangwang-characters");
    press(&mut mode, 13);
    assert_eq!(frame(&mode), settings_title, "Enter returns to the title");

    let mut advanced = settings.clone();
    press(&mut advanced, 21);
    let advanced_selected = frame(&advanced);
    assert_ne!(advanced_selected, settings_page, "Down selects 高级选项");
    press(&mut advanced, 13);
    let economy = frame(&advanced);
    assert_ne!(
        economy, advanced_selected,
        "高级选项 shows the economy values"
    );
    capture(&advanced, "wangwang-economy");
    press(&mut advanced, 13);
    assert_eq!(
        frame(&advanced),
        settings_title,
        "Enter returns to the title"
    );

    let mut slots = menu.clone();
    press(&mut slots, 13);
    let submenu = frame(&slots);
    assert_ne!(submenu, title, "进入游戏 opens the start submenu");
    press(&mut slots, 21);
    let load_selected = frame(&slots);
    assert_ne!(load_selected, submenu, "Down selects 读取游戏");
    press(&mut slots, 13);
    let slot_menu = frame(&slots);
    assert_ne!(slot_menu, load_selected, "读取游戏 lists four save slots");
    capture(&slots, "wangwang-slots");
    press(&mut slots, 21);
    let second_slot = frame(&slots);
    assert_ne!(second_slot, slot_menu, "Down selects the second slot");
    press(&mut slots, 13);
    assert_eq!(
        frame(&slots),
        second_slot,
        "an empty slot stays on the list"
    );
    press(&mut slots, 27);
    assert_eq!(frame(&slots), title, "Escape returns to the title");

    let mut board = menu.clone();
    press(&mut board, 13);
    press(&mut board, 13);
    advance(&mut board, 300);
    assert_eq!(board.vm().pc(), 0xa33f, "开始游戏 reaches the board");
    let board_frame = frame(&board);
    assert_ne!(board_frame, title, "the board replaces the title");
    capture(&board, "wangwang-board");
    advance(&mut board, 120);
    assert_eq!(frame(&board), board_frame, "the board waits for input");

    let mut turn = board.clone();
    press(&mut turn, 13);
    assert_ne!(frame(&turn), board_frame, "Enter takes a turn");
    advance(&mut turn, 120);
    let prompt = frame(&turn);
    assert_ne!(prompt, board_frame, "the turn reaches a purchase prompt");
    capture(&turn, "wangwang-purchase-prompt");
    press(&mut turn, 13);
    advance(&mut turn, 300);
    assert_ne!(
        frame(&turn),
        prompt,
        "answering the prompt advances the game"
    );
    capture(&turn, "wangwang-next-turn");

    let mut quit = menu.clone();
    press(&mut quit, 21);
    press(&mut quit, 21);
    press(&mut quit, 21);
    quit.input_mut().set_keys([13]);
    let mut exit_code = None;
    for _ in 0..120 {
        let result = quit.run_frame().unwrap();
        if let FrameStatus::Halted(code) = result.status {
            exit_code = Some(code);
            break;
        }
    }
    assert_eq!(exit_code, Some(0), "退出游戏 exits the program");

    let mut escape = menu.clone();
    escape.input_mut().set_keys([27]);
    let mut escape_code = None;
    for _ in 0..120 {
        let result = escape.run_frame().unwrap();
        if let FrameStatus::Halted(code) = result.status {
            escape_code = Some(code);
            break;
        }
    }
    assert_eq!(escape_code, Some(0), "Escape exits from the title");
}

#[test]
#[ignore = "requires the local LavaX game corpus"]
fn meteor_butterfly_sword_title_pages_difficulty_and_arena() {
    let frame = |emulator: &Emulator| emulator.display().indexed_frame().to_vec();
    let mut splash = load("LAVA1 流星蝴蝶剑/流星蝴蝶剑.lav");
    advance(&mut splash, 2000);
    assert_eq!(splash.vm().pc(), 0x51de, "the loading splash waits");
    capture(&splash, "meteor-splash");

    let mut menu = splash.clone();
    press(&mut menu, 13);
    assert_eq!(menu.vm().pc(), 0x4fce, "the title waits for a key");
    let width = usize::from(menu.display().width());
    let title = frame(&menu);
    capture(&menu, "meteor-title");

    let mut navigation = menu.clone();
    press(&mut navigation, 22);
    let second = frame(&navigation);
    assert_ne!(second, title, "Right selects the second entry");
    press(&mut navigation, 22);
    let third = frame(&navigation);
    assert_ne!(third, second, "Right selects the third entry");
    press(&mut navigation, 22);
    assert_eq!(frame(&navigation), third, "Right stops at the third entry");
    press(&mut navigation, 23);
    assert_eq!(
        frame(&navigation),
        second,
        "Left returns to the second entry"
    );
    press(&mut navigation, 23);
    assert_eq!(frame(&navigation), title, "Left returns to the first entry");
    press(&mut navigation, 23);
    assert_eq!(frame(&navigation), title, "Left stops at the first entry");

    let mut about = menu.clone();
    press(&mut about, 22);
    press(&mut about, 22);
    press(&mut about, 13);
    let about_page = frame(&about);
    assert_ne!(about_page, third, "the third entry opens the about page");
    capture(&about, "meteor-about");
    press(&mut about, 13);
    assert_eq!(frame(&about), third, "Enter returns to the title");

    let mut credits = menu.clone();
    press(&mut credits, 27);
    assert_eq!(credits.vm().pc(), 0x2db2, "Escape opens the credits");
    assert_ne!(frame(&credits), title, "the credits replace the title");
    capture(&credits, "meteor-credits");
    credits.input_mut().set_keys([13]);
    let mut credits_code = None;
    for _ in 0..120 {
        let result = credits.run_frame().unwrap();
        if let FrameStatus::Halted(code) = result.status {
            credits_code = Some(code);
            break;
        }
    }
    assert_eq!(credits_code, Some(0), "Enter on the credits halts");

    let mut selector = menu.clone();
    press(&mut selector, 22);
    press(&mut selector, 13);
    assert_eq!(selector.vm().pc(), 0x4f03, "the difficulty selector waits");
    let default_bar = frame(&selector);
    capture(&selector, "meteor-difficulty");
    press(&mut selector, 22);
    let harder = frame(&selector);
    assert_ne!(harder, default_bar, "Right raises the difficulty");
    press(&mut selector, 23);
    assert_eq!(
        frame(&selector),
        default_bar,
        "Left restores the difficulty"
    );
    press(&mut selector, 22);
    let adjusted = frame(&selector);
    press(&mut selector, 27);
    assert_eq!(
        selector.vm().pc(),
        0x4f03,
        "Escape is ignored after an adjustment"
    );
    assert_eq!(frame(&selector), adjusted, "the adjusted selector stays");

    let mut escape = menu.clone();
    press(&mut escape, 22);
    press(&mut escape, 13);
    press(&mut escape, 27);
    assert_eq!(escape.vm().pc(), 0x4fce, "Escape leaves the selector");
    assert_eq!(
        frame(&escape),
        second,
        "the title keeps the second entry selected"
    );

    let mut game = menu.clone();
    press(&mut game, 13);
    assert_eq!(game.vm().pc(), 0x4ddb, "the first entry opens the controls");
    let controls = frame(&game);
    capture(&game, "meteor-controls");
    press(&mut game, 32);
    assert_eq!(game.vm().pc(), 0x480b, "any key starts the arena");
    assert_ne!(
        frame(&game),
        controls,
        "the arena replaces the controls page"
    );
    capture(&game, "meteor-arena");

    let mut base = game.clone();
    advance(&mut base, 60);
    let mut idle = base.clone();
    advance(&mut idle, 120);
    let idle_frame = frame(&idle);

    for (key, name) in [(100, "D"), (119, "W"), (97, "A"), (115, "S")] {
        let mut moved = base.clone();
        moved.input_mut().set_keys([key]);
        advance(&mut moved, 120);
        moved.input_mut().set_keys([]);
        let (count, _, _) = changed_region(&idle_frame, &frame(&moved), width);
        assert!(count > 100, "{name} changes the arena");
    }
    let mut moved = base.clone();
    moved.input_mut().set_keys([100]);
    advance(&mut moved, 120);
    moved.input_mut().set_keys([]);
    capture(&moved, "meteor-move");

    let mut attack = base.clone();
    attack.input_mut().set_keys([106]);
    advance(&mut attack, 120);
    attack.input_mut().set_keys([]);
    let (count, _, _) = changed_region(&idle_frame, &frame(&attack), width);
    assert!(count > 50, "the attack key animates the fighter");

    let mut quit = base.clone();
    quit.input_mut().set_keys([113]);
    let mut quit_code = None;
    for _ in 0..120 {
        let result = quit.run_frame().unwrap();
        if let FrameStatus::Halted(code) = result.status {
            quit_code = Some(code);
            break;
        }
    }
    assert_eq!(quit_code, Some(0), "q quits the arena");
}

#[test]
#[ignore = "requires the local LavaX game corpus"]
fn haidian_floating_life_name_menu_actions_and_save_round_trip() {
    let frame = |emulator: &Emulator| emulator.display().indexed_frame().to_vec();
    let money = |emulator: &Emulator| {
        u32::from_le_bytes(emulator.vm().memory()[0x2488..0x248c].try_into().unwrap())
    };
    let debt = |emulator: &Emulator| {
        u32::from_le_bytes(emulator.vm().memory()[0x248c..0x2490].try_into().unwrap())
    };
    let health = |emulator: &Emulator| {
        u16::from_le_bytes(emulator.vm().memory()[0x2490..0x2492].try_into().unwrap())
    };
    let morality = |emulator: &Emulator| {
        u16::from_le_bytes(emulator.vm().memory()[0x2492..0x2494].try_into().unwrap())
    };
    let happiness = |emulator: &Emulator| {
        u16::from_le_bytes(emulator.vm().memory()[0x2494..0x2496].try_into().unwrap())
    };
    let day = |emulator: &Emulator| {
        u16::from_le_bytes(emulator.vm().memory()[0x2496..0x2498].try_into().unwrap())
    };
    let inventory = |emulator: &Emulator| {
        u16::from_le_bytes(emulator.vm().memory()[0x24ae..0x24b0].try_into().unwrap())
    };

    let mut title = load("LAVA1 海淀浮生记/海淀浮生记.lav");
    advance(&mut title, 300);
    assert_eq!(title.vm().pc(), 0x61d, "the title waits for a key");
    let title_frame = frame(&title);
    capture(&title, "haidian-title");

    let mut menu = title.clone();
    press(&mut menu, 13);
    assert_eq!(menu.vm().pc(), 0x761, "Enter opens the name prompt");
    assert_ne!(
        frame(&menu),
        title_frame,
        "the name prompt replaces the title"
    );
    capture(&menu, "haidian-name");
    for key in *b"abc" {
        press(&mut menu, key);
    }
    assert_eq!(menu.vm().pc(), 0x761, "the name prompt waits");
    press(&mut menu, 13);
    assert_eq!(menu.vm().pc(), 0x153a, "the menu waits for a selection");
    assert_eq!(
        &menu.vm().memory()[0x247f..0x2488],
        b"abc\r\0\0\0\0\0",
        "the entered name is stored for saving"
    );
    assert_eq!(money(&menu), 5000, "a new life starts with cash");
    assert_eq!(debt(&menu), 5000, "a new life starts in debt");
    assert_eq!(health(&menu), 85, "a new life starts healthy");
    assert_eq!(morality(&menu), 80, "a new life starts moral");
    assert_eq!(happiness(&menu), 75, "a new life starts content");
    assert_eq!(day(&menu), 0, "a new life starts on the first day");
    capture(&menu, "haidian-menu");

    // Number keys arrive as their LavaX letter codes: 1..8 = b, n, m, g, h, j, t, y.
    let mut status = menu.clone();
    press(&mut status, b'b');
    assert_eq!(
        status.vm().pc(),
        0xb5e,
        "the first entry opens the status page"
    );
    assert_ne!(
        frame(&status),
        frame(&menu),
        "the status page replaces the menu"
    );
    capture(&status, "haidian-status");
    press(&mut status, 32);
    assert_eq!(status.vm().pc(), 0x153a, "any key leaves the status page");

    let mut market = menu.clone();
    press(&mut market, b'm');
    assert_eq!(market.vm().pc(), 0xc66, "the third entry opens the market");
    let market_frame = frame(&market);
    capture(&market, "haidian-market");
    press(&mut market, b'j');
    assert_eq!(
        market.vm().pc(),
        0xcaa,
        "the sixth item opens its trade prompt"
    );
    let trade_frame = frame(&market);
    assert_ne!(
        trade_frame, market_frame,
        "the trade prompt replaces the list"
    );
    press(&mut market, b'b');
    assert_eq!(market.vm().pc(), 0x6a7, "buying asks for a quantity");
    assert_eq!(money(&market), 5000, "asking does not charge yet");
    capture(&market, "haidian-market-buy");
    press(&mut market, 50);
    press(&mut market, 13);
    assert_eq!(market.vm().pc(), 0xd4e, "the purchase reports its result");
    assert_eq!(money(&market), 4834, "two discs cost 166");
    assert_eq!(inventory(&market), 2, "two discs are held");
    press(&mut market, 32);
    assert_eq!(market.vm().pc(), 0x153a, "the menu returns after buying");

    let mut sell = market.clone();
    press(&mut sell, b'm');
    press(&mut sell, b'j');
    press(&mut sell, b'n');
    assert_eq!(sell.vm().pc(), 0x6a7, "selling asks for a quantity");
    press(&mut sell, 49);
    press(&mut sell, 13);
    assert_eq!(sell.vm().pc(), 0xe00, "the sale reports its result");
    assert_eq!(money(&sell), 4917, "one disc sells for its full price");
    assert_eq!(inventory(&sell), 1, "one disc remains");
    press(&mut sell, 32);
    assert_eq!(sell.vm().pc(), 0x153a, "the menu returns after selling");

    let mut donate = sell.clone();
    press(&mut donate, b'h');
    assert_eq!(
        donate.vm().pc(),
        0xe20,
        "the fifth entry opens the hospital"
    );
    capture(&donate, "haidian-hospital");
    press(&mut donate, b'b');
    assert_eq!(donate.vm().pc(), 0xe76, "donation shows the blood drive");
    press(&mut donate, 32);
    assert_eq!(donate.vm().pc(), 0xe7f, "donation reports its effect");
    assert_eq!(health(&donate), 80, "donation costs health");
    assert_eq!(morality(&donate), 85, "donation raises morality");
    assert_eq!(money(&donate), 4917, "donation is free");
    press(&mut donate, 32);
    assert_eq!(donate.vm().pc(), 0x153a, "the menu returns after donating");

    let mut treat = donate.clone();
    press(&mut treat, b'h');
    press(&mut treat, b'n');
    assert_eq!(treat.vm().pc(), 0xecf, "treatment shows the nurse");
    press(&mut treat, 32);
    assert_eq!(treat.vm().pc(), 0xee3, "treatment reports its effect");
    assert_eq!(health(&treat), 85, "treatment restores health");
    assert_eq!(money(&treat), 3917, "treatment costs 1000");
    assert_eq!(morality(&treat), 85, "treatment leaves morality unchanged");
    press(&mut treat, 32);
    assert_eq!(treat.vm().pc(), 0x153a, "the menu returns after treatment");

    let mut bar = treat.clone();
    press(&mut bar, b'j');
    assert_eq!(bar.vm().pc(), 0xefb, "the sixth entry opens the bar");
    let bar_art = frame(&bar);
    capture(&bar, "haidian-bar");
    press(&mut bar, 32);
    assert_eq!(bar.vm().pc(), 0xf69, "the first drink reports the bill");
    assert_eq!(money(&bar), 2917, "a night at the bar costs 1000");
    assert_eq!(health(&bar), 84, "drinking costs health");
    assert_eq!(happiness(&bar), 80, "drinking raises happiness");
    assert_ne!(frame(&bar), bar_art, "the bill replaces the bar scene");
    press(&mut bar, 32);
    assert_eq!(bar.vm().pc(), 0x153a, "the menu returns after the bill");

    let mut expensive = bar.clone();
    press(&mut expensive, b'm');
    press(&mut expensive, b'b');
    press(&mut expensive, b'b');
    assert_eq!(
        expensive.vm().pc(),
        0x6a7,
        "the computer asks for a quantity"
    );
    press(&mut expensive, 49);
    press(&mut expensive, 13);
    assert_eq!(expensive.vm().pc(), 0xd68, "the purchase is rejected");
    assert_eq!(money(&expensive), 2917, "a rejected purchase is free");
    press(&mut expensive, 32);
    assert_eq!(
        expensive.vm().pc(),
        0x153a,
        "the menu returns after rejection"
    );

    let mut bank = expensive.clone();
    press(&mut bank, b'n');
    assert_eq!(bank.vm().pc(), 0xb71, "the second entry opens the bank");
    capture(&bank, "haidian-bank");
    press(&mut bank, b'b');
    assert_eq!(bank.vm().pc(), 0x6a7, "a loan asks for an amount");
    capture(&bank, "haidian-loan");
    for key in [53, 48, 48, 48] {
        press(&mut bank, key);
    }
    press(&mut bank, 13);
    assert_eq!(bank.vm().pc(), 0x153a, "the menu returns after the loan");
    assert_eq!(money(&bank), 7917, "the loan adds 5000");
    assert_eq!(debt(&bank), 10000, "the loan adds 5000 of debt");

    let mut repay = bank.clone();
    press(&mut repay, b'n');
    press(&mut repay, b'n');
    assert_eq!(repay.vm().pc(), 0x6a7, "a repayment asks for an amount");
    for key in [50, 48, 48, 48] {
        press(&mut repay, key);
    }
    press(&mut repay, 13);
    assert_eq!(repay.vm().pc(), 0x153a, "the menu returns after repayment");
    assert_eq!(money(&repay), 5917, "repayment subtracts 2000");
    assert_eq!(debt(&repay), 8000, "repayment subtracts 2000 of debt");

    let mut casino = repay.clone();
    press(&mut casino, b'g');
    assert_eq!(
        casino.vm().pc(),
        0x1007,
        "the casino turns away a small stake"
    );
    capture(&casino, "haidian-casino");
    press(&mut casino, 32);
    assert_eq!(
        casino.vm().pc(),
        0x153a,
        "the menu returns after the refusal"
    );

    let mut night = casino.clone();
    let evening = frame(&night);
    press(&mut night, b't');
    advance(&mut night, 150);
    assert_eq!(night.vm().pc(), 0x153a, "the menu returns after the night");
    assert_ne!(frame(&night), evening, "the calendar redraws");
    assert_eq!(money(&night), 917, "a night costs 5000");
    assert_eq!(debt(&night), 8800, "the night adds ten percent interest");
    assert_eq!(health(&night), 79, "a night costs health");
    assert_eq!(day(&night), 1, "the calendar advances to the second day");
    capture(&night, "haidian-sleep");

    let mut save = night.clone();
    press(&mut save, b'y');
    assert_eq!(
        save.vm().pc(),
        0x12d5,
        "the eighth entry opens the system menu"
    );
    press(&mut save, b'b');
    assert_eq!(save.vm().pc(), 0x1322, "saving shows progress");
    advance(&mut save, 60);
    press(&mut save, 32);
    assert_eq!(save.vm().pc(), 0x132b, "saving reports completion");
    capture(&save, "haidian-saved");
    let dirty = save.files_mut().take_dirty_files();
    let (_, bytes) = dirty
        .iter()
        .find(|(path, _)| path == "/lavadata/hd.sav")
        .expect("the save record is written");
    assert_eq!(bytes.len(), 31, "the save record has a fixed size");
    assert_eq!(&bytes[..9], b"abc\r\0\0\0\0\0", "the record keeps the name");
    assert_eq!(
        u32::from_le_bytes(bytes[9..13].try_into().unwrap()),
        917,
        "the record keeps the cash"
    );
    assert_eq!(
        u32::from_le_bytes(bytes[13..17].try_into().unwrap()),
        8800,
        "the record keeps the debt"
    );
    assert_eq!(
        u16::from_le_bytes(bytes[17..19].try_into().unwrap()),
        79,
        "the record keeps health"
    );
    assert_eq!(
        u16::from_le_bytes(bytes[19..21].try_into().unwrap()),
        85,
        "the record keeps morality"
    );
    assert_eq!(
        u16::from_le_bytes(bytes[21..23].try_into().unwrap()),
        80,
        "the record keeps happiness"
    );
    assert_eq!(
        u16::from_le_bytes(bytes[23..25].try_into().unwrap()),
        1,
        "the record keeps the day"
    );
    press(&mut save, 32);
    assert_eq!(save.vm().pc(), 0x153a, "the menu returns after saving");

    let mut reload = save.clone();
    press(&mut reload, b'h');
    press(&mut reload, b'b');
    advance(&mut reload, 60);
    press(&mut reload, 32);
    advance(&mut reload, 60);
    assert_eq!(health(&reload), 74, "the second donation costs health");
    assert_eq!(morality(&reload), 90, "the second donation raises morality");
    press(&mut reload, 32);
    assert_eq!(reload.vm().pc(), 0x153a, "the menu returns after donating");

    press(&mut reload, b'y');
    press(&mut reload, b'n');
    assert_eq!(reload.vm().pc(), 0x137b, "loading shows progress");
    advance(&mut reload, 60);
    press(&mut reload, 32);
    assert_eq!(reload.vm().pc(), 0x1384, "loading reports completion");
    capture(&reload, "haidian-loaded");
    advance(&mut reload, 60);
    press(&mut reload, 32);
    assert_eq!(reload.vm().pc(), 0x153a, "the menu returns after loading");
    assert_eq!(health(&reload), 79, "loading restores health");
    assert_eq!(morality(&reload), 85, "loading restores morality");
    assert_eq!(money(&reload), 917, "loading restores cash");
    assert_eq!(debt(&reload), 8800, "loading restores debt");
    assert_eq!(day(&reload), 1, "loading restores the day");

    let mut exit = reload.clone();
    press(&mut exit, b'y');
    assert_eq!(exit.vm().pc(), 0x12d5, "the system menu waits again");
    exit.input_mut().set_keys(*b"g");
    let mut exit_code = None;
    for _ in 0..120 {
        let result = exit.run_frame().unwrap();
        if let FrameStatus::Halted(code) = result.status {
            exit_code = Some(code);
            break;
        }
    }
    assert_eq!(exit_code, Some(0), "the exit entry halts");
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn pirate_ship_menus_help_credits_difficulty_and_sailing() {
    let frame = |emulator: &Emulator| emulator.display().indexed_frame().to_vec();
    let word = |emulator: &Emulator, address: usize| {
        u16::from_le_bytes(
            emulator.vm().memory()[address..address + 2]
                .try_into()
                .unwrap(),
        )
    };
    let position = |emulator: &Emulator| (word(emulator, 0x2f10), word(emulator, 0x2f12));
    let first_enemy = |emulator: &Emulator| (word(emulator, 0x2f28), word(emulator, 0x2f2a));

    for (path, tag) in [
        ("LAVA1 海盗船/打包/海盗船.pac", "pac"),
        ("LAVA1 海盗船/未打包/海盗船.lav", "lav"),
    ] {
        let mut title = load(path);
        advance(&mut title, 300);
        assert_eq!(title.vm().pc(), 0x7d1, "the splash waits for a key");
        capture(&title, &format!("pirate-splash-{tag}"));
        press(&mut title, 13);
        assert_eq!(title.vm().pc(), 0x83b, "the studio logo waits for a key");
        press(&mut title, 13);
        advance(&mut title, 300);
        assert_eq!(title.vm().pc(), 0x8a1, "the title menu waits for a key");
        capture(&title, &format!("pirate-menu-{tag}"));
        assert_eq!(word(&title, 0x2f76), 1, "the first menu entry is selected");

        for expected in [2, 3, 4, 1] {
            press(&mut title, 21);
            assert_eq!(word(&title, 0x2f76), expected, "down wraps the menu");
        }
        for expected in [4, 3, 2, 1] {
            press(&mut title, 20);
            assert_eq!(word(&title, 0x2f76), expected, "up wraps the menu");
        }

        press(&mut title, 21);
        press(&mut title, 13);
        advance(&mut title, 150);
        assert_eq!(title.vm().pc(), 0x6be, "the help screen waits for a key");
        capture(&title, &format!("pirate-help-{tag}"));
        let mut page = frame(&title);
        for _ in 0..9 {
            press(&mut title, 21);
            assert_ne!(frame(&title), page, "down scrolls the help text");
            page = frame(&title);
        }
        press(&mut title, 21);
        assert_eq!(frame(&title), page, "the help text stops at the end");
        capture(&title, &format!("pirate-help-end-{tag}"));
        press(&mut title, 27);
        assert_eq!(title.vm().pc(), 0x12f, "escape leaves the help screen");
        advance(&mut title, 300);
        assert_eq!(title.vm().pc(), 0x8a1, "the title menu returns after help");

        press(&mut title, 21);
        press(&mut title, 21);
        press(&mut title, 13);
        advance(&mut title, 150);
        assert_eq!(title.vm().pc(), 0x9a0, "the credits wait for a key");
        capture(&title, &format!("pirate-credits-{tag}"));
        for (pc, name) in [(0x9c8, "second"), (0x9f0, "third"), (0xa18, "fourth")] {
            press(&mut title, 13);
            assert_eq!(title.vm().pc(), pc, "the credits advance page by page");
            capture(&title, &format!("pirate-credits-{name}-{tag}"));
        }
        press(&mut title, 13);
        assert_eq!(title.vm().pc(), 0x12f, "the credits leave to the menu");
        advance(&mut title, 300);
        assert_eq!(
            title.vm().pc(),
            0x8a1,
            "the title menu returns after credits"
        );

        for _ in 0..3 {
            press(&mut title, 21);
        }
        press(&mut title, 13);
        advance(&mut title, 150);
        assert_eq!(title.vm().pc(), 0xa4d, "the exit screen waits for a key");
        capture(&title, &format!("pirate-exit-{tag}"));
        title.input_mut().set_keys([13]);
        let mut exit_code = None;
        for _ in 0..120 {
            let result = title.run_frame().unwrap();
            if let FrameStatus::Halted(code) = result.status {
                exit_code = Some(code);
                break;
            }
        }
        assert_eq!(exit_code, Some(0), "the exit entry halts");
    }

    let navigate_to_difficulty = |emulator: &mut Emulator| {
        advance(emulator, 300);
        press(emulator, 13);
        press(emulator, 13);
        advance(emulator, 300);
        press(emulator, 13);
        advance(emulator, 150);
    };

    let mut difficulty = load("LAVA1 海盗船/打包/海盗船.pac");
    navigate_to_difficulty(&mut difficulty);
    assert_eq!(
        difficulty.vm().pc(),
        0xab4,
        "the difficulty screen waits for a key"
    );
    assert_eq!(word(&difficulty, 0x2f76), 1, "the easiest ship is selected");
    for expected in [2, 3, 1] {
        press(&mut difficulty, 21);
        assert_eq!(
            word(&difficulty, 0x2f76),
            expected,
            "down wraps the difficulty choices"
        );
    }
    for expected in [3, 2, 1] {
        press(&mut difficulty, 20);
        assert_eq!(
            word(&difficulty, 0x2f76),
            expected,
            "up wraps the difficulty choices"
        );
    }

    let mut normal = difficulty.clone();
    press(&mut normal, 21);
    press(&mut normal, 13);
    advance(&mut normal, 240);
    assert_eq!(normal.vm().pc(), 0x133d, "normal difficulty starts sailing");
    assert_eq!(word(&normal, 0x2f4e), 2, "normal difficulty is recorded");
    assert_eq!(word(&normal, 0x2f52), 3, "normal places three pirate ships");
    assert_eq!(word(&normal, 0x2f54), 2, "normal places two islands");
    assert_eq!(word(&normal, 0x2f58), 3, "normal keeps three harbors");
    assert_eq!(word(&normal, 0x2f64), 3, "normal starts with three lives");
    assert_eq!(word(&normal, 0x2f66), 1, "normal starts on level one");

    let mut hard = difficulty.clone();
    press(&mut hard, 21);
    press(&mut hard, 21);
    press(&mut hard, 13);
    advance(&mut hard, 240);
    assert_eq!(hard.vm().pc(), 0x133d, "hard difficulty starts sailing");
    assert_eq!(word(&hard, 0x2f4e), 3, "hard difficulty is recorded");
    assert_eq!(word(&hard, 0x2f52), 3, "hard places three pirate ships");
    assert_eq!(word(&hard, 0x2f54), 3, "hard places three islands");
    assert_eq!(word(&hard, 0x2f58), 2, "hard keeps two harbors");
    assert_eq!(word(&hard, 0x2f64), 3, "hard starts with three lives");
    assert_eq!(word(&hard, 0x2f66), 1, "hard starts on level one");
    capture(&hard, "pirate-hard");
    assert_ne!(
        frame(&hard),
        frame(&normal),
        "each difficulty lays out its own sea"
    );

    let mut easy = difficulty.clone();
    press(&mut easy, 13);
    advance(&mut easy, 240);
    assert_eq!(easy.vm().pc(), 0x133d, "easy difficulty starts sailing");
    assert_eq!(word(&easy, 0x2f4e), 1, "easy difficulty is recorded");
    assert_eq!(word(&easy, 0x2f52), 1, "easy places one pirate ship");
    assert_eq!(word(&easy, 0x2f54), 1, "easy places one island");
    assert_eq!(word(&easy, 0x2f58), 4, "easy keeps four harbors");
    assert_eq!(word(&easy, 0x2f64), 3, "easy starts with three lives");
    assert_eq!(word(&easy, 0x2f66), 1, "easy starts on level one");
    assert_eq!(
        position(&easy),
        (4, 4),
        "the main ship starts in the middle"
    );
    assert_eq!(first_enemy(&easy), (4, 1), "the pirate ship starts above");
    capture(&easy, "pirate-easy");

    for (key, expected) in [
        (22, (5, 4)),
        (23, (3, 4)),
        (20, (4, 3)),
        (21, (4, 5)),
        (b'a', (3, 3)),
        (b'z', (3, 5)),
        (b's', (5, 3)),
        (b'x', (5, 5)),
    ] {
        let mut moved = easy.clone();
        press(&mut moved, key);
        assert_eq!(moved.vm().pc(), 0x133d, "sailing keeps running");
        assert_eq!(position(&moved), expected, "the key steers the main ship");
    }

    let mut fired = easy.clone();
    press(&mut fired, 13);
    assert_eq!(fired.vm().pc(), 0x133d, "firing keeps running");
    assert_eq!(
        position(&fired),
        (4, 4),
        "firing does not move the main ship"
    );
    assert_eq!(
        first_enemy(&fired),
        (4, 2),
        "the pirate ship chases after a turn"
    );

    let mut quit = easy.clone();
    press(&mut quit, 27);
    assert_eq!(quit.vm().pc(), 0x1990, "escape asks to leave the game");
    capture(&quit, "pirate-quit");
    press(&mut quit, b'n');
    assert_eq!(quit.vm().pc(), 0x133d, "declining resumes sailing");
    press(&mut quit, 27);
    press(&mut quit, b'y');
    assert_eq!(quit.vm().pc(), 0x12f, "accepting leaves the game");
    advance(&mut quit, 300);
    assert_eq!(
        quit.vm().pc(),
        0xab4,
        "leaving returns to the difficulty screen"
    );

    let mut death = easy.clone();
    for _ in 0..3 {
        press(&mut death, 20);
    }
    assert_eq!(
        death.vm().pc(),
        0x1e4f,
        "sailing into a wreck sinks the ship"
    );
    assert_eq!(position(&death).1, 1, "the ship stopped on the wreck");
    press(&mut death, 20);
    assert_eq!(death.vm().pc(), 0x1e71, "the sinking animation runs");
    press(&mut death, 20);
    assert_eq!(death.vm().pc(), 0x12f, "the sunk message waits for a key");
    capture(&death, "pirate-sunk");
    advance(&mut death, 120);
    press(&mut death, 13);
    assert_eq!(death.vm().pc(), 0x133d, "the next ship starts sailing");
    assert_eq!(word(&death, 0x2f64), 2, "a sunk ship costs one life");
    assert_eq!(
        position(&death),
        (4, 4),
        "the next ship starts in the middle"
    );

    for _ in 0..3 {
        press(&mut death, 20);
    }
    assert_eq!(death.vm().pc(), 0x1e4f, "the wreck sinks the next ship");
    press(&mut death, 20);
    assert_eq!(death.vm().pc(), 0x1e71, "the second sinking animation runs");
    press(&mut death, 20);
    assert_eq!(death.vm().pc(), 0x12f, "the second sunk message waits");
    advance(&mut death, 120);
    press(&mut death, 13);
    assert_eq!(death.vm().pc(), 0x133d, "the last ship starts sailing");
    assert_eq!(word(&death, 0x2f64), 1, "the second loss leaves one life");

    for _ in 0..3 {
        press(&mut death, 20);
    }
    assert_eq!(death.vm().pc(), 0x1e4f, "the last ship hits the wreck");
    press(&mut death, 20);
    assert_eq!(death.vm().pc(), 0x1e71, "the last sinking animation runs");
    press(&mut death, 20);
    assert_eq!(
        death.vm().pc(),
        0x1e8d,
        "the last loss starts the game over"
    );
    advance(&mut death, 120);
    press(&mut death, 13);
    assert_eq!(death.vm().pc(), 0x12f, "the game over screen waits");
    assert_eq!(word(&death, 0x2f64), 0, "no lives remain");
    capture(&death, "pirate-game-over");
    advance(&mut death, 180);
    assert_eq!(
        death.vm().pc(),
        0xab4,
        "game over returns to the difficulty screen"
    );

    let mut unpacked = load("LAVA1 海盗船/未打包/海盗船.lav");
    navigate_to_difficulty(&mut unpacked);
    press(&mut unpacked, 13);
    advance(&mut unpacked, 240);
    assert_eq!(
        unpacked.vm().pc(),
        0x133d,
        "the unpacked game starts sailing"
    );
    assert_eq!(
        frame(&unpacked),
        frame(&easy),
        "both packages share the sea"
    );
    assert_eq!(
        word(&unpacked, 0x2f28),
        word(&easy, 0x2f28),
        "same pirate ship"
    );
    assert_eq!(word(&unpacked, 0x2f64), word(&easy, 0x2f64), "same lives");
    assert_eq!(word(&unpacked, 0x2f66), word(&easy, 0x2f66), "same level");
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn spirit_sword_menu_help_arrows_scoring_level_and_rank() {
    const PATH: &str = "LAVA1 灵剑射击/灵剑射击.lav";
    let frame = |emulator: &Emulator| emulator.display().indexed_frame().to_vec();
    let word = |emulator: &Emulator, address: usize| {
        u16::from_be_bytes(
            emulator.vm().memory()[address..address + 2]
                .try_into()
                .unwrap(),
        )
    };
    let open_menu = |emulator: &mut Emulator| {
        advance(emulator, 300);
        assert_eq!(emulator.vm().pc(), 0x2259, "the splash waits for a key");
        press(emulator, 13);
        assert_eq!(
            emulator.vm().pc(),
            0x23b5,
            "the first-run message waits for a key"
        );
        press(emulator, 13);
        advance(emulator, 120);
        press(emulator, 13);
        advance(emulator, 120);
        assert_eq!(emulator.vm().pc(), 0x1b66, "the title menu waits for a key");
    };

    let mut menu = load(PATH);
    open_menu(&mut menu);
    capture(&menu, "spirit-menu");
    let first = frame(&menu);
    press(&mut menu, 21);
    let second = frame(&menu);
    assert_ne!(second, first, "down highlights the second entry");
    press(&mut menu, 21);
    let third = frame(&menu);
    assert_ne!(third, second, "down highlights the third entry");
    press(&mut menu, 21);
    assert_eq!(frame(&menu), third, "the menu stops at the last entry");
    press(&mut menu, 20);
    assert_eq!(frame(&menu), second, "up returns to the second entry");
    press(&mut menu, 20);
    assert_eq!(frame(&menu), first, "up returns to the first entry");
    press(&mut menu, 20);
    assert_eq!(frame(&menu), first, "the menu stops at the first entry");

    press(&mut menu, 25);
    assert_eq!(menu.vm().pc(), 0x1ab8, "F5 opens the help page");
    capture(&menu, "spirit-help");
    press(&mut menu, 21);
    assert_eq!(menu.vm().pc(), 0x1b66, "down leaves the help page");
    assert_eq!(frame(&menu), first, "the title returns at the first entry");

    press(&mut menu, 21);
    press(&mut menu, 13);
    advance(&mut menu, 120);
    assert_eq!(menu.vm().pc(), 0x24f3, "the empty leaderboard opens");
    capture(&menu, "spirit-rank-empty");
    press(&mut menu, 13);
    assert_eq!(menu.vm().pc(), 0x1b66, "enter returns from the leaderboard");

    let mut game = load(PATH);
    open_menu(&mut game);
    press(&mut game, 13);
    advance(&mut game, 300);
    assert!(
        matches!(game.vm().pc(), 0x215a | 0x215b),
        "the start entry begins the first level"
    );
    capture(&game, "spirit-level-one");
    assert_eq!(game.vm().memory()[0x3391], 1, "the first level is shown");
    assert_eq!(
        game.vm().memory()[0x338b],
        10,
        "the level starts with ten arrows"
    );
    assert_eq!(
        &game.vm().memory()[0x3397..0x33a6],
        b"\xb7\xd6\xca\xfd\xa3\xba0 \xd7\xdc\xb7\xd6\xa3\xba0",
        "the score starts at zero"
    );
    assert_eq!(
        &game.vm().memory()[0x33b5..0x33ba],
        b"\xb5\xda\x31\xb9\xd8",
        "the HUD shows level one"
    );
    assert_eq!(
        word(&game, 0x3382),
        150,
        "the target sits at the right edge"
    );
    assert_eq!(word(&game, 0x3384), 53, "the arrow flies at a fixed height");
    assert_eq!(word(&game, 0x3386), 18, "the arrow waits at the left edge");
    let target = word(&game, 0x3380);
    advance(&mut game, 60);
    assert_ne!(word(&game, 0x3380), target, "the target moves up and down");

    for shot in 0..10 {
        let mut waited = 0;
        while word(&game, 0x3380) != 60 {
            advance(&mut game, 1);
            waited += 1;
            assert!(waited < 4000, "the target must reach the shot height");
        }
        hold(&mut game, b'a', 3);
        advance(&mut game, 150);
        if shot == 0 {
            assert_eq!(game.vm().memory()[0x338b], 9, "firing spends one arrow");
            assert_eq!(
                &game.vm().memory()[0x3397..0x33a8],
                b"\xb7\xd6\xca\xfd\xa3\xba13 \xd7\xdc\xb7\xd6\xa3\xba13",
                "a hit scores thirteen points"
            );
            capture(&game, "spirit-hit");
        }
    }
    advance(&mut game, 300);
    assert!(
        matches!(game.vm().pc(), 0x215a | 0x215b),
        "the second level starts"
    );
    assert_eq!(game.vm().memory()[0x3391], 2, "the second level is shown");
    assert_eq!(
        game.vm().memory()[0x338b],
        10,
        "the second level restores the arrows"
    );
    assert_eq!(
        &game.vm().memory()[0x3397..0x33a7],
        b"\xb7\xd6\xca\xfd\xa3\xba0 \xd7\xdc\xb7\xd6\xa3\xba65",
        "the total keeps the level-one score"
    );
    assert_eq!(
        &game.vm().memory()[0x33b5..0x33ba],
        b"\xb5\xda\x32\xb9\xd8",
        "the HUD shows level two"
    );
    capture(&game, "spirit-level-two");

    let target = word(&game, 0x3380);
    press(&mut game, 112);
    assert_eq!(game.vm().pc(), 0x206e, "P pauses the round");
    let paused = frame(&game);
    advance(&mut game, 120);
    assert_eq!(frame(&game), paused, "the paused frame stays still");
    assert_eq!(word(&game, 0x3380), target, "the target freezes");
    capture(&game, "spirit-paused");
    press(&mut game, 32);
    assert!(
        matches!(game.vm().pc(), 0x215a | 0x215b),
        "any key resumes the round"
    );
    advance(&mut game, 90);
    assert_ne!(word(&game, 0x3380), target, "the target moves again");

    press(&mut game, 27);
    advance(&mut game, 180);
    assert_eq!(game.vm().pc(), 0x1479, "escape asks for a leaderboard name");
    capture(&game, "spirit-rank-name");
    for key in b"xy" {
        press(&mut game, *key);
    }
    press(&mut game, 13);
    advance(&mut game, 120);
    assert_eq!(
        game.vm().pc(),
        0x1b66,
        "the registered name returns to the menu"
    );
    press(&mut game, 21);
    press(&mut game, 13);
    advance(&mut game, 120);
    assert_eq!(
        game.vm().pc(),
        0x24f3,
        "the leaderboard opens after a round"
    );
    capture(&game, "spirit-rank");
    let rank = frame(&game);
    let files = game.files_mut().take_dirty_files();
    let (rank_path, rank_bytes) = files
        .iter()
        .find(|(path, _)| path.ends_with("shoop.dat"))
        .expect("the leaderboard must write its record");
    assert_eq!(rank_path.as_str(), "/lavadata/shoop.dat");
    assert_eq!(rank_bytes.len(), 48, "the leaderboard stores four entries");
    assert_eq!(&rank_bytes[..2], b"xy", "the first row keeps the name");
    assert_eq!(rank_bytes[40], 65, "the first row keeps the total score");

    let mut reloaded = load(PATH);
    for (path, bytes) in files {
        reloaded.files_mut().import_file(&path, bytes);
    }
    advance(&mut reloaded, 300);
    assert_eq!(
        reloaded.vm().pc(),
        0x2259,
        "the splash still waits for a key"
    );
    press(&mut reloaded, 13);
    assert!(
        matches!(reloaded.vm().pc(), 0x1962 | 0x1a06),
        "the stored record skips the first-run message"
    );
    advance(&mut reloaded, 120);
    press(&mut reloaded, 13);
    advance(&mut reloaded, 120);
    assert_eq!(reloaded.vm().pc(), 0x1b66, "the title menu waits for a key");
    press(&mut reloaded, 21);
    press(&mut reloaded, 13);
    advance(&mut reloaded, 120);
    assert_eq!(reloaded.vm().pc(), 0x24f3, "the stored leaderboard opens");
    assert_eq!(frame(&reloaded), rank, "the stored record is read back");

    let mut quitting = load(PATH);
    open_menu(&mut quitting);
    press(&mut quitting, 21);
    press(&mut quitting, 21);
    press(&mut quitting, 13);
    advance(&mut quitting, 120);
    assert_eq!(quitting.vm().pc(), 0x2519, "the exit entry asks to leave");
    press(&mut quitting, b'n');
    advance(&mut quitting, 60);
    assert_eq!(quitting.vm().pc(), 0x1b66, "declining returns to the menu");
    press(&mut quitting, 21);
    press(&mut quitting, 21);
    press(&mut quitting, 13);
    advance(&mut quitting, 120);
    press(&mut quitting, b'y');
    assert_eq!(
        quitting.vm().pc(),
        0x253e,
        "the second question keeps asking"
    );
    press(&mut quitting, b'n');
    advance(&mut quitting, 60);
    assert_eq!(
        quitting.vm().pc(),
        0x1b66,
        "declining the second question returns to the menu"
    );
    press(&mut quitting, 21);
    press(&mut quitting, 21);
    press(&mut quitting, 13);
    advance(&mut quitting, 120);
    press(&mut quitting, b'y');
    assert_eq!(
        quitting.vm().pc(),
        0x253e,
        "the second question appears again"
    );
    press(&mut quitting, b'y');
    assert_eq!(quitting.vm().pc(), 0x256e, "the third question appears");
    press(&mut quitting, b'y');
    assert_eq!(quitting.vm().pc(), 0x2593, "the last question appears");
    capture(&quitting, "spirit-exit");
    press(&mut quitting, b'y');
    let mut exit_code = None;
    for _ in 0..120 {
        if let FrameStatus::Halted(code) = quitting.run_frame().unwrap().status {
            exit_code = Some(code);
            break;
        }
    }
    assert_eq!(exit_code, Some(0), "the last answer halts the program");
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn bomb_submarine_menu_help_level_select_scoring_pause_and_exit() {
    const PATH: &str = "LAVA1 炸潜艇/炸潜艇v1.1 2007.7.6.lav";
    let frame = |emulator: &Emulator| emulator.display().indexed_frame().to_vec();
    let word = |emulator: &Emulator, address: usize| {
        u16::from_le_bytes([
            emulator.vm().memory()[address],
            emulator.vm().memory()[address + 1],
        ])
    };
    let ship = |emulator: &Emulator| {
        i16::from_le_bytes([
            emulator.vm().memory()[0x2006],
            emulator.vm().memory()[0x2007],
        ])
    };
    let start_level = |emulator: &mut Emulator| {
        advance(emulator, 60);
        assert_eq!(emulator.vm().pc(), 0x1718, "the title menu waits for a key");
        press(emulator, 13);
        assert_eq!(emulator.vm().pc(), 0x1146, "the level selector opens");
        advance(emulator, 120);
        press(emulator, 13);
        advance(emulator, 30);
        assert_eq!(emulator.vm().pc(), 0x14de, "the first level runs");
    };

    let mut menu = load(PATH);
    advance(&mut menu, 60);
    capture(&menu, "bomb-menu");
    let first = frame(&menu);
    press(&mut menu, 21);
    let second = frame(&menu);
    assert_ne!(second, first, "down highlights the help entry");
    press(&mut menu, 21);
    let third = frame(&menu);
    assert_ne!(third, second, "down highlights the background entry");
    press(&mut menu, 21);
    let fourth = frame(&menu);
    assert_ne!(fourth, third, "down highlights the exit entry");
    press(&mut menu, 21);
    assert_eq!(frame(&menu), first, "down wraps to the first entry");
    press(&mut menu, 20);
    assert_eq!(frame(&menu), fourth, "up wraps to the exit entry");
    press(&mut menu, 20);
    assert_eq!(frame(&menu), third, "up returns to the background entry");
    press(&mut menu, 20);
    assert_eq!(frame(&menu), second, "up returns to the help entry");

    let mut help = load(PATH);
    advance(&mut help, 60);
    press(&mut help, 21);
    press(&mut help, 13);
    advance(&mut help, 60);
    assert_eq!(help.vm().pc(), 0x4f7, "the help page opens");
    capture(&help, "bomb-help");
    let mut pages = vec![frame(&help)];
    for _ in 0..10 {
        press(&mut help, 21);
        let page = frame(&help);
        assert_ne!(*pages.last().unwrap(), page, "down scrolls the help text");
        pages.push(page);
    }
    press(&mut help, 21);
    assert_eq!(
        frame(&help),
        pages[10],
        "the help text stops at its last page"
    );
    capture(&help, "bomb-help-end");
    press(&mut help, 27);
    advance(&mut help, 60);
    assert_eq!(help.vm().pc(), 0x1718, "escape returns to the title menu");

    let mut story = load(PATH);
    advance(&mut story, 60);
    press(&mut story, 21);
    press(&mut story, 21);
    press(&mut story, 13);
    advance(&mut story, 60);
    assert_eq!(story.vm().pc(), 0xef6, "the background story opens");
    capture(&story, "bomb-story");
    let opening = frame(&story);
    press(&mut story, 21);
    let after_one = frame(&story);
    assert_ne!(after_one, opening, "down scrolls the story");
    press(&mut story, 21);
    assert_ne!(frame(&story), after_one, "down scrolls the story again");
    press(&mut story, 27);
    let mut story_exit = None;
    for _ in 0..12000 {
        if let FrameStatus::Halted(code) = story.run_frame().unwrap().status {
            story_exit = Some(code);
            break;
        }
    }
    assert_eq!(
        story_exit,
        Some(0),
        "escape leaves the story into the ending"
    );
    assert_eq!(
        story.vm().pc(),
        0x176f,
        "the story ending halts at its own page"
    );

    let mut exit = load(PATH);
    advance(&mut exit, 60);
    press(&mut exit, 21);
    press(&mut exit, 21);
    press(&mut exit, 21);
    press(&mut exit, 13);
    advance(&mut exit, 60);
    assert_eq!(
        exit.vm().pc(),
        0xef6,
        "the exit entry opens its credits page"
    );
    capture(&exit, "bomb-credits");
    let mut exit_code = None;
    for _ in 0..6000 {
        if let FrameStatus::Halted(code) = exit.run_frame().unwrap().status {
            exit_code = Some(code);
            break;
        }
    }
    assert_eq!(exit_code, Some(0), "the exit credits halt the program");
    assert_eq!(exit.vm().pc(), 0x1626, "the exit credits end on their page");

    let mut levels = load(PATH);
    advance(&mut levels, 60);
    press(&mut levels, 13);
    assert_eq!(levels.vm().pc(), 0x1146, "the level selector opens");
    assert_eq!(word(&levels, 0x2320), 1, "the first level is selected");
    press(&mut levels, 23);
    assert_eq!(word(&levels, 0x2320), 1, "left stops at the first level");
    for _ in 0..8 {
        press(&mut levels, 22);
    }
    assert_eq!(word(&levels, 0x2320), 9, "right reaches the ninth level");
    press(&mut levels, 22);
    assert_eq!(word(&levels, 0x2320), 9, "right stops at the ninth level");
    capture(&levels, "bomb-level-select");
    press(&mut levels, 27);
    advance(&mut levels, 60);
    assert_eq!(levels.vm().pc(), 0x1718, "escape returns to the title menu");
    press(&mut levels, 13);
    assert_eq!(levels.vm().pc(), 0x1146, "the level selector opens again");
    assert_eq!(
        word(&levels, 0x2320),
        1,
        "the selector restarts at the first level"
    );
    for _ in 0..8 {
        press(&mut levels, 22);
    }
    assert_eq!(
        word(&levels, 0x2320),
        9,
        "the ninth level is selected again"
    );
    press(&mut levels, 13);
    advance(&mut levels, 90);
    assert_eq!(levels.vm().pc(), 0x14de, "the ninth level runs");
    assert_eq!(word(&levels, 0x2000), 9, "the level counter shows nine");
    assert_eq!(word(&levels, 0x2002), 15, "the timer starts at fifteen");
    assert_eq!(ship(&levels), 75, "the ship starts in the middle");
    capture(&levels, "bomb-level-nine");

    let mut game = load(PATH);
    start_level(&mut game);
    assert_eq!(word(&game, 0x2000), 1, "the first level starts");
    assert_eq!(word(&game, 0x2002), 15, "the timer starts at fifteen");
    assert_eq!(ship(&game), 75, "the ship starts in the middle");
    assert_eq!(word(&game, 0x2008), 0, "the score starts at zero");
    capture(&game, "bomb-level-one");
    hold(&mut game, 22, 15);
    assert_eq!(
        ship(&game),
        105,
        "right moves the ship two pixels per frame"
    );
    hold(&mut game, 22, 15);
    assert_eq!(ship(&game), 135, "right moves the ship to the right edge");
    hold(&mut game, 22, 15);
    assert_eq!(ship(&game), 135, "the ship stops at the right edge");
    hold(&mut game, 23, 70);
    assert_eq!(ship(&game), -1, "left moves the ship to the left edge");
    hold(&mut game, 23, 15);
    assert_eq!(ship(&game), -1, "the ship stops at the left edge");

    let mut shots = load(PATH);
    advance(&mut shots, 60);
    press(&mut shots, 13);
    advance(&mut shots, 120);
    press(&mut shots, 13);
    advance(&mut shots, 30);
    for _ in 0..20 {
        press(&mut shots, b'z');
    }
    assert_eq!(word(&shots, 0x2008), 25, "twenty z charges score 25");
    capture(&shots, "bomb-hits");
    let timer = word(&shots, 0x2002);
    press(&mut shots, 112);
    assert_eq!(shots.vm().pc(), 0x5ef, "p pauses the round");
    capture(&shots, "bomb-paused");
    advance(&mut shots, 240);
    assert_eq!(shots.vm().pc(), 0x5ef, "the pause holds");
    assert_eq!(word(&shots, 0x2002), timer, "the pause freezes the timer");
    press(&mut shots, 32);
    assert_eq!(shots.vm().pc(), 0x14de, "any key resumes the round");
    let mut resumed = 0;
    while word(&shots, 0x2002) >= timer && resumed < 1200 {
        advance(&mut shots, 120);
        resumed += 120;
    }
    assert!(
        word(&shots, 0x2002) < timer,
        "the timer runs again after the pause"
    );

    let mut x_shots = load(PATH);
    advance(&mut x_shots, 60);
    press(&mut x_shots, 13);
    advance(&mut x_shots, 120);
    press(&mut x_shots, 13);
    advance(&mut x_shots, 30);
    for _ in 0..20 {
        press(&mut x_shots, b'x');
    }
    assert_eq!(word(&x_shots, 0x2008), 22, "twenty x charges score 22");

    let mut timed = load(PATH);
    start_level(&mut timed);
    let mut frames = 0;
    while timed.vm().pc() != 0x1146 {
        timed.run_frame().unwrap();
        frames += 1;
        assert!(frames < 6000, "the level timer must expire");
    }
    assert_eq!(
        timed.vm().pc(),
        0x1146,
        "the timer returns to the level selector"
    );
    assert_eq!(word(&timed, 0x2002), 0, "the timer is spent");
    assert_eq!(word(&timed, 0x2008), 0, "the round score is cleared");
    assert_eq!(word(&timed, 0x2320), 1, "the selector keeps the level");
    capture(&timed, "bomb-time-up");
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn war_sword_menu_options_random_map_turn_editor_help_and_load_errors() {
    const PATH: &str = "LAVA1 煞魔剑/mywar1.lav";
    let byte = |emulator: &Emulator, address: usize| emulator.vm().memory()[address];
    let word = |emulator: &Emulator, address: usize| {
        u16::from_le_bytes([
            emulator.vm().memory()[address],
            emulator.vm().memory()[address + 1],
        ])
    };
    let cell = |emulator: &Emulator, x: usize, y: usize| {
        let address = 0x2170 + (y * 20 + x) * 4;
        let memory = emulator.vm().memory();
        [
            memory[address],
            memory[address + 1],
            memory[address + 2],
            memory[address + 3],
        ]
    };
    let at =
        |emulator: &Emulator, low: usize, high: usize| (low..=high).contains(&emulator.vm().pc());
    let cursor = |emulator: &Emulator, x: u8, y: u8| {
        byte(emulator, 0x20e3) == x && byte(emulator, 0x20e4) == y
    };
    let start_map = |emulator: &mut Emulator| {
        advance(emulator, 60);
        press(emulator, 13);
        assert_eq!(
            byte(emulator, 0x44b7),
            1,
            "the first menu entry is selected"
        );
        press(emulator, 13);
        press(emulator, 21);
        press(emulator, 21);
        press(emulator, 13);
        advance(emulator, 60);
        assert_eq!(word(emulator, 0x44b7), 20, "the map width starts at twenty");
        assert_eq!(
            word(emulator, 0x44b9),
            15,
            "the map height starts at fifteen"
        );
        press(emulator, 13);
        advance(emulator, 240);
        assert!(at(emulator, 0x6dc7, 0x6e06), "the map view runs");
        assert!(cursor(emulator, 9, 3), "the cursor starts on the temple");
    };

    let mut menu = load(PATH);
    advance(&mut menu, 60);
    assert!(
        at(&menu, 0x72f6, 0x7315),
        "the intro screen waits for a key"
    );
    capture(&menu, "war-intro");
    press(&mut menu, 13);
    assert_eq!(byte(&menu, 0x44b7), 1, "the first menu entry is selected");
    capture(&menu, "war-menu");
    for expected in [2, 3, 4] {
        press(&mut menu, 22);
        assert_eq!(
            byte(&menu, 0x44b7),
            expected,
            "right selects the next entry"
        );
    }
    press(&mut menu, 22);
    assert_eq!(byte(&menu, 0x44b7), 4, "right stops at the help entry");
    for expected in [3, 2, 1] {
        press(&mut menu, 23);
        assert_eq!(
            byte(&menu, 0x44b7),
            expected,
            "left selects the previous entry"
        );
    }
    press(&mut menu, 23);
    assert_eq!(byte(&menu, 0x44b7), 1, "left stops at the start entry");
    press(&mut menu, 13);
    advance(&mut menu, 60);
    assert_eq!(byte(&menu, 0x44bd), 0, "the map row is selected");
    assert_eq!(byte(&menu, 0x20f5), 1, "the random map is selected");
    assert_eq!(
        byte(&menu, 0x20e5),
        2,
        "the standard difficulty is selected"
    );
    capture(&menu, "war-options");
    press(&mut menu, 22);
    assert_eq!(byte(&menu, 0x20f5), 2, "right selects the downloaded map");
    press(&mut menu, 22);
    assert_eq!(byte(&menu, 0x20f5), 2, "right stops at the downloaded map");
    press(&mut menu, 23);
    assert_eq!(byte(&menu, 0x20f5), 1, "left selects the random map");
    press(&mut menu, 23);
    assert_eq!(byte(&menu, 0x20f5), 1, "left stops at the random map");
    press(&mut menu, 21);
    assert_eq!(byte(&menu, 0x44bd), 1, "down selects the difficulty row");
    press(&mut menu, 22);
    assert_eq!(byte(&menu, 0x20e5), 3, "right selects the hard difficulty");
    press(&mut menu, 22);
    assert_eq!(byte(&menu, 0x20e5), 3, "right stops at the hard difficulty");
    press(&mut menu, 23);
    assert_eq!(
        byte(&menu, 0x20e5),
        2,
        "left returns to the standard difficulty"
    );
    press(&mut menu, 23);
    assert_eq!(byte(&menu, 0x20e5), 1, "left selects the easy difficulty");
    press(&mut menu, 23);
    assert_eq!(byte(&menu, 0x20e5), 1, "left stops at the easy difficulty");
    press(&mut menu, 21);
    assert_eq!(byte(&menu, 0x44bd), 2, "down selects the name row");
    press(&mut menu, 21);
    assert_eq!(byte(&menu, 0x44bd), 2, "down stops at the name row");
    for key in *b"play" {
        press(&mut menu, key);
    }
    assert_eq!(
        &menu.vm().memory()[0x208c..0x2090],
        b"PLAY",
        "the typed name is stored"
    );
    assert_eq!(byte(&menu, 0x44bf), 4, "the name length is four");
    capture(&menu, "war-name");
    press(&mut menu, 13);
    advance(&mut menu, 60);
    assert_eq!(word(&menu, 0x44b7), 20, "the map width starts at twenty");
    assert_eq!(word(&menu, 0x44b9), 15, "the map height starts at fifteen");
    capture(&menu, "war-size");
    press(&mut menu, 13);
    advance(&mut menu, 240);
    assert!(at(&menu, 0x6dc7, 0x6e06), "the map view runs");
    assert!(cursor(&menu, 9, 3), "the cursor starts on the temple");
    assert_eq!(byte(&menu, 0x20db), 0, "the turn counter starts at zero");
    assert_eq!(
        cell(&menu, 9, 3),
        [0, 3, 0x25, 0],
        "the temple map is generated"
    );
    assert_eq!(
        cell(&menu, 9, 1),
        [0, 2, 0x64, 0],
        "the terrain row is generated"
    );
    assert_eq!(cell(&menu, 10, 3), [0, 0, 0, 0], "the map border is empty");
    capture(&menu, "war-map");

    press(&mut menu, 22);
    assert!(cursor(&menu, 10, 3), "right moves east");
    press(&mut menu, 22);
    assert!(
        cursor(&menu, 10, 3),
        "the empty border blocks the eastern step"
    );
    for expected in [9, 8, 7, 6] {
        press(&mut menu, 23);
        assert!(cursor(&menu, expected, 3), "left moves west");
    }
    for expected in [4, 5, 5] {
        press(&mut menu, 21);
        assert!(cursor(&menu, 6, expected), "down moves south");
    }
    for expected in [4, 3, 2, 1, 1] {
        press(&mut menu, 20);
        assert!(cursor(&menu, 6, expected), "up moves north");
    }
    for expected in [2, 3, 4, 5] {
        press(&mut menu, 21);
        assert!(cursor(&menu, 6, expected), "down returns south");
    }
    for expected in [7, 8, 9] {
        press(&mut menu, 22);
        assert!(cursor(&menu, expected, 5), "right returns east");
    }
    for expected in [4, 3] {
        press(&mut menu, 20);
        assert!(cursor(&menu, 9, expected), "up returns north");
    }
    press(&mut menu, 13);
    assert!(at(&menu, 0x1c39, 0x1c4c), "the temple info panel opens");
    assert_eq!(
        byte(&menu, 0x2004),
        0x35,
        "the temple info text is selected"
    );
    capture(&menu, "war-temple-info");
    press(&mut menu, 27);
    assert!(at(&menu, 0x6dc7, 0x6e06), "escape returns to the map");
    press(&mut menu, 23);
    assert!(cursor(&menu, 8, 3), "left moves onto the field");
    press(&mut menu, 13);
    assert!(
        at(&menu, 0x6001, 0x6014),
        "the build menu opens on the field"
    );
    assert_eq!(byte(&menu, 0x2004), 0x31, "the build menu text is selected");
    capture(&menu, "war-build-menu");
    press(&mut menu, 27);
    assert!(at(&menu, 0x6dc7, 0x6e06), "escape returns to the map");
    for expected in [1, 2, 3] {
        press(&mut menu, b'p');
        assert_eq!(byte(&menu, 0x20db), expected, "p advances the turn counter");
    }

    let mut panels = load(PATH);
    start_map(&mut panels);
    press(&mut panels, 27);
    assert!(at(&panels, 0x1c39, 0x1c4c), "the system menu opens");
    capture(&panels, "war-system-menu");
    press(&mut panels, 27);
    assert!(at(&panels, 0x6dc7, 0x6e06), "escape returns to the map");
    press(&mut panels, 32);
    assert!(at(&panels, 0x1c39, 0x1c4c), "the memory point panel opens");
    capture(&panels, "war-memory-points");
    press(&mut panels, 27);
    assert!(at(&panels, 0x6dc7, 0x6e06), "escape returns to the map");

    let mut missing = load(PATH);
    advance(&mut missing, 60);
    press(&mut missing, 13);
    press(&mut missing, 22);
    assert_eq!(byte(&missing, 0x44b7), 2, "the load entry is selected");
    press(&mut missing, 13);
    advance(&mut missing, 60);
    assert!(
        at(&missing, 0x21b0, 0x21c3),
        "the missing document error opens"
    );
    capture(&missing, "war-load-error");
    press(&mut missing, 13);
    assert_eq!(
        byte(&missing, 0x44b7),
        1,
        "the error returns to the first entry"
    );

    let mut help = load(PATH);
    advance(&mut help, 60);
    press(&mut help, 13);
    for _ in 0..3 {
        press(&mut help, 22);
    }
    assert_eq!(byte(&help, 0x44b7), 4, "the help entry is selected");
    press(&mut help, 13);
    advance(&mut help, 60);
    assert_eq!(help.vm().pc(), 0x4168, "the help page opens");
    capture(&help, "war-help");
    press(&mut help, 27);
    advance(&mut help, 60);
    assert_eq!(byte(&help, 0x44b7), 1, "escape returns to the first entry");

    let mut sizing = load(PATH);
    advance(&mut sizing, 60);
    press(&mut sizing, 13);
    press(&mut sizing, 13);
    press(&mut sizing, 21);
    press(&mut sizing, 21);
    press(&mut sizing, 13);
    advance(&mut sizing, 60);
    assert_eq!(word(&sizing, 0x44b7), 20, "the width starts at twenty");
    assert_eq!(word(&sizing, 0x44b9), 15, "the height starts at fifteen");
    press(&mut sizing, 20);
    assert_eq!(word(&sizing, 0x44b9), 16, "up grows the height");
    press(&mut sizing, 20);
    assert_eq!(word(&sizing, 0x44b9), 17, "up grows the height again");
    press(&mut sizing, 21);
    assert_eq!(word(&sizing, 0x44b9), 16, "down shrinks the height");
    press(&mut sizing, 21);
    assert_eq!(word(&sizing, 0x44b9), 15, "down shrinks the height again");
    press(&mut sizing, 21);
    assert_eq!(word(&sizing, 0x44b9), 15, "down stops at fifteen");
    press(&mut sizing, 22);
    assert_eq!(word(&sizing, 0x44b7), 21, "right grows the width");
    press(&mut sizing, 23);
    assert_eq!(word(&sizing, 0x44b7), 20, "left shrinks the width");
    press(&mut sizing, 23);
    assert_eq!(word(&sizing, 0x44b7), 20, "left stops at twenty");
    capture(&sizing, "war-size-adjust");

    let mut editor = load(PATH);
    advance(&mut editor, 60);
    press(&mut editor, 13);
    press(&mut editor, 22);
    press(&mut editor, 22);
    assert_eq!(byte(&editor, 0x44b7), 3, "the map entry is selected");
    press(&mut editor, 13);
    advance(&mut editor, 60);
    assert_eq!(
        word(&editor, 0x44b7),
        20,
        "the editor map starts twenty wide"
    );
    assert_eq!(
        word(&editor, 0x44b9),
        15,
        "the editor map starts fifteen high"
    );
    press(&mut editor, 13);
    advance(&mut editor, 60);
    assert_eq!(editor.vm().pc(), 0x4168, "the editor help page opens");
    press(&mut editor, 13);
    advance(&mut editor, 60);
    assert!(at(&editor, 0x56f1, 0x5704), "the editor canvas runs");
    assert!(cursor(&editor, 1, 1), "the cursor starts in the first cell");
    assert_eq!(
        cell(&editor, 1, 1),
        [0, 0, 0, 1],
        "the first cell starts empty"
    );
    capture(&editor, "war-editor");
    press(&mut editor, 13);
    advance(&mut editor, 60);
    assert!(at(&editor, 0x21b0, 0x21c3), "saving without a start warns");
    capture(&editor, "war-editor-warning");
    press(&mut editor, 13);
    advance(&mut editor, 60);
    assert!(
        at(&editor, 0x56f1, 0x5704),
        "the warning returns to the canvas"
    );
    assert_eq!(
        cell(&editor, 1, 1),
        [0, 0, 0, 1],
        "the warning leaves the cell unchanged"
    );
    press(&mut editor, b't');
    assert_eq!(cell(&editor, 1, 1), [0, 2, 0x64, 1], "t places a temple");
    press(&mut editor, b'a');
    assert_eq!(cell(&editor, 1, 1), [0, 7, 0x64, 1], "a places its terrain");
    press(&mut editor, b'g');
    assert_eq!(cell(&editor, 1, 1), [0, 3, 0xC8, 1], "g places its terrain");
    press(&mut editor, b'e');
    assert_eq!(cell(&editor, 1, 1), [0, 0, 0, 1], "e clears the cell");
    press(&mut editor, b'k');
    assert_eq!(cell(&editor, 1, 1), [0, 0x11, 0, 1], "k places its terrain");
    press(&mut editor, b's');
    assert_eq!(cell(&editor, 1, 1), [0, 0x10, 0x64, 1], "s marks the start");
    assert_eq!(byte(&editor, 0x20df), 1, "the start column is stored");
    assert_eq!(byte(&editor, 0x20e0), 1, "the start row is stored");
    press(&mut editor, b'c');
    assert_eq!(cell(&editor, 1, 1), [0, 1, 0x64, 1], "c marks the city");
    assert_eq!(byte(&editor, 0x20e1), 1, "the city column is stored");
    assert_eq!(byte(&editor, 0x20e2), 1, "the city row is stored");
    capture(&editor, "war-editor-city");
    press(&mut editor, 22);
    press(&mut editor, 22);
    press(&mut editor, 21);
    assert!(cursor(&editor, 3, 2), "the editor cursor moves");
    press(&mut editor, b'q');
    advance(&mut editor, 60);
    assert_eq!(
        byte(&editor, 0x44b7),
        1,
        "q returns to the first menu entry"
    );
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn war_sword_a_splash_stub_menus_options_map_and_field_messages() {
    const PATH: &str = "LAVA1 煞魔剑A/煞魔剑A.lav";
    let byte = |emulator: &Emulator, address: usize| emulator.vm().memory()[address];
    let cell = |emulator: &Emulator, x: usize, y: usize| {
        let address = 0x2cc6 + (y * 20 + x) * 4;
        let memory = emulator.vm().memory();
        [
            memory[address],
            memory[address + 1],
            memory[address + 2],
            memory[address + 3],
        ]
    };
    let start = || {
        let mut emulator = load(PATH);
        advance(&mut emulator, 60);
        press(&mut emulator, 13);
        advance(&mut emulator, 30);
        press(&mut emulator, 13);
        advance(&mut emulator, 30);
        press(&mut emulator, 21);
        press(&mut emulator, 21);
        advance(&mut emulator, 30);
        press(&mut emulator, 112);
        press(&mut emulator, 108);
        press(&mut emulator, 97);
        press(&mut emulator, 121);
        advance(&mut emulator, 30);
        press(&mut emulator, 13);
        advance(&mut emulator, 120);
        press(&mut emulator, 13);
        advance(&mut emulator, 240);
        emulator
    };

    let mut menu = load(PATH);
    advance(&mut menu, 60);
    capture(&menu, "war-a-splash");
    press(&mut menu, 13);
    assert!(
        (0x47da..=0x488f).contains(&menu.vm().pc()),
        "the title menu runs"
    );
    assert_eq!(byte(&menu, 0x1b0c), 1, "the start entry is selected");
    capture(&menu, "war-a-menu");
    for expected in [2, 3, 4, 1] {
        press(&mut menu, 22);
        assert_eq!(
            byte(&menu, 0x1b0c),
            expected,
            "right moves through the menu entries"
        );
    }
    press(&mut menu, 23);
    assert_eq!(byte(&menu, 0x1b0c), 1, "left stops at the first menu entry");

    for (moves, shot) in [
        (1usize, "war-a-stub-load"),
        (2, "war-a-stub-map"),
        (3, "war-a-stub-help"),
    ] {
        let mut stub = load(PATH);
        advance(&mut stub, 60);
        press(&mut stub, 13);
        for _ in 0..moves {
            press(&mut stub, 22);
        }
        press(&mut stub, 13);
        assert_eq!(
            stub.vm().pc(),
            0x3c35,
            "the unfinished entry shows its notice"
        );
        capture(&stub, shot);
        press(&mut stub, 13);
        assert!(
            (0x47da..=0x488f).contains(&stub.vm().pc()),
            "the notice returns to the menu"
        );
        assert_eq!(
            byte(&stub, 0x1b0c),
            1,
            "dismissing the notice resets the selection"
        );
    }

    let mut walk = start();
    assert!(
        (0x6830..=0x685b).contains(&walk.vm().pc()),
        "the generated map runs"
    );
    assert_eq!(byte(&walk, 0x3ae0), 2, "standard difficulty is the default");
    assert_eq!(byte(&walk, 0x3af0), 1, "the random map type persists");
    assert_eq!(byte(&walk, 0x3ae5), 20, "the map is twenty wide");
    assert_eq!(byte(&walk, 0x3ae6), 15, "the map is fifteen high");
    assert_eq!(byte(&walk, 0x3ae9), 124, "standard difficulty health");
    assert_eq!(byte(&walk, 0x3aeb), 200, "standard difficulty mana");
    assert_eq!(byte(&walk, 0x3ade), 9, "the hero starts in column nine");
    assert_eq!(byte(&walk, 0x3adf), 3, "the hero starts in row three");
    for x in 0..20 {
        assert_eq!(cell(&walk, x, 0), [0, 2, 0x64, 0], "the top row is a wall");
    }
    assert_eq!(
        cell(&walk, 9, 3),
        [0, 3, 0x43, 0],
        "the start tile is a city"
    );
    assert_eq!(
        cell(&walk, 10, 3),
        [0, 0, 0, 0],
        "the tile to the right is open"
    );
    assert_eq!(
        cell(&walk, 11, 3),
        [0, 2, 0x81, 0],
        "the next tile is a wall"
    );
    assert_eq!(
        cell(&walk, 10, 2),
        [0, 3, 0x22, 0],
        "the tile above is a city"
    );
    assert_eq!(cell(&walk, 10, 1), [0, 2, 1, 0], "the next tile is terrain");
    capture(&walk, "war-a-map");

    press(&mut walk, 22);
    assert_eq!(
        (byte(&walk, 0x3ade), byte(&walk, 0x3adf)),
        (10, 3),
        "right moves the hero"
    );
    press(&mut walk, 22);
    assert_eq!(
        (byte(&walk, 0x3ade), byte(&walk, 0x3adf)),
        (10, 3),
        "the wall blocks the hero"
    );
    press(&mut walk, 20);
    assert_eq!(
        (byte(&walk, 0x3ade), byte(&walk, 0x3adf)),
        (10, 2),
        "up moves the hero"
    );
    press(&mut walk, 20);
    assert_eq!(
        (byte(&walk, 0x3ade), byte(&walk, 0x3adf)),
        (10, 1),
        "up moves the hero again"
    );
    press(&mut walk, 23);
    assert_eq!(
        (byte(&walk, 0x3ade), byte(&walk, 0x3adf)),
        (9, 1),
        "left moves the hero"
    );
    press(&mut walk, 21);
    assert_eq!(
        (byte(&walk, 0x3ade), byte(&walk, 0x3adf)),
        (9, 2),
        "down moves the hero"
    );
    capture(&walk, "war-a-walk");

    let mut empty = start();
    press(&mut empty, 13);
    assert!(
        (0x1ceb..=0x1cfd).contains(&empty.vm().pc()),
        "enter opens a field message"
    );
    assert_eq!(
        &empty.vm().memory()[0x1c93..0x1c99],
        &[0xCE, 0xB4, 0xCD, 0xEA, 0xB3, 0xC9],
        "the unfinished notice is shown"
    );
    capture(&empty, "war-a-message-empty");
    press(&mut empty, 13);
    assert!(
        (0x6830..=0x685b).contains(&empty.vm().pc()),
        "the message returns to the map"
    );

    let mut resource = start();
    press(&mut resource, 22);
    press(&mut resource, 20);
    assert_eq!(
        (byte(&resource, 0x3ade), byte(&resource, 0x3adf)),
        (10, 2),
        "the hero stands on the resource tile"
    );
    press(&mut resource, 13);
    assert!(
        (0x1ceb..=0x1cfd).contains(&resource.vm().pc()),
        "enter opens the resource message"
    );
    assert_eq!(
        &resource.vm().memory()[0x1c93..0x1c9b],
        &[0xBF, 0xF3, 0xB2, 0xFA, 0xD7, 0xCA, 0xD4, 0xB4],
        "the resource notice is shown"
    );
    capture(&resource, "war-a-message-resource");
    press(&mut resource, 13);
    assert!(
        (0x6830..=0x685b).contains(&resource.vm().pc()),
        "the resource message returns to the map"
    );

    let mut system = start();
    press(&mut system, 27);
    assert!(
        (0x1ceb..=0x1cfd).contains(&system.vm().pc()),
        "escape opens the system menu"
    );
    assert_eq!(
        &system.vm().memory()[0x1c93..0x1ca0],
        &[
            0xCF, 0xB5, 0xCD, 0xB3, 0xB2, 0xCB, 0xB5, 0xA5, 0x3A, 0x20, 0x31, 0xB4, 0xE6
        ],
        "the system menu lists its entries"
    );
    capture(&system, "war-a-system-menu");
    for key in [49, 50, 51, 52] {
        press(&mut system, key);
        assert!(
            (0x6830..=0x685b).contains(&system.vm().pc()),
            "the system entry returns to the map"
        );
        assert_eq!(
            (byte(&system, 0x3ade), byte(&system, 0x3adf)),
            (9, 3),
            "the hero stays put"
        );
        if key != 52 {
            press(&mut system, 27);
            assert!(
                (0x1ceb..=0x1cfd).contains(&system.vm().pc()),
                "escape reopens the system menu"
            );
        }
    }
    assert!(
        system.files_mut().take_dirty_files().is_empty(),
        "the stub save entries write nothing"
    );

    let mut sizing = load(PATH);
    advance(&mut sizing, 60);
    press(&mut sizing, 13);
    press(&mut sizing, 13);
    press(&mut sizing, 21);
    press(&mut sizing, 21);
    press(&mut sizing, 13);
    assert!(
        (0x1e43..=0x1e56).contains(&sizing.vm().pc()),
        "the size screen opens"
    );
    assert_eq!(byte(&sizing, 0x3b29), 20, "the width starts at twenty");
    assert_eq!(byte(&sizing, 0x3b2b), 15, "the height starts at fifteen");
    capture(&sizing, "war-a-size");
    for _ in 0..12 {
        press(&mut sizing, 23);
    }
    assert_eq!(byte(&sizing, 0x3b29), 10, "the width stops at ten");
    for _ in 0..2 {
        press(&mut sizing, 23);
    }
    assert_eq!(byte(&sizing, 0x3b29), 10, "the width stays at ten");
    for _ in 0..12 {
        press(&mut sizing, 21);
    }
    assert_eq!(byte(&sizing, 0x3b2b), 5, "the height stops at five");
    for _ in 0..2 {
        press(&mut sizing, 21);
    }
    assert_eq!(byte(&sizing, 0x3b2b), 5, "the height stays at five");
    assert!(
        (0x1e43..=0x1e56).contains(&sizing.vm().pc()),
        "the size screen keeps running"
    );
    capture(&sizing, "war-a-size-min");
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn war_sword_full_splash_menu_options_editor_and_field_messages() {
    const PATH: &str = "LAVA1 煞魔剑正式版/【煞魔剑】 正式版.lav";
    let byte = |emulator: &Emulator, address: usize| emulator.vm().memory()[address];
    let cell = |emulator: &Emulator, x: usize, y: usize| {
        let address = 0x2d66 + (y * 20 + x) * 4;
        let memory = emulator.vm().memory();
        [
            memory[address],
            memory[address + 1],
            memory[address + 2],
            memory[address + 3],
        ]
    };
    let at =
        |emulator: &Emulator, low: usize, high: usize| (low..=high).contains(&emulator.vm().pc());
    let hero = |emulator: &Emulator| (byte(emulator, 0x3ade), byte(emulator, 0x3adf));
    let start = || {
        let mut emulator = load(PATH);
        advance(&mut emulator, 60);
        press(&mut emulator, 13);
        press(&mut emulator, 13);
        press(&mut emulator, 21);
        press(&mut emulator, 21);
        press(&mut emulator, 13);
        advance(&mut emulator, 60);
        press(&mut emulator, 13);
        advance(&mut emulator, 240);
        emulator
    };

    let mut menu = load(PATH);
    advance(&mut menu, 60);
    assert!(
        at(&menu, 0x6cee, 0x6cfd),
        "the splash screen waits for a key"
    );
    capture(&menu, "war-full-splash");
    press(&mut menu, 13);
    assert!(at(&menu, 0x4790, 0x488f), "the title menu runs");
    assert_eq!(byte(&menu, 0x1b0c), 1, "the start entry is selected");
    capture(&menu, "war-full-menu");
    for expected in [2, 3, 4] {
        press(&mut menu, 22);
        assert_eq!(
            byte(&menu, 0x1b0c),
            expected,
            "right selects the next entry"
        );
    }
    press(&mut menu, 22);
    assert_eq!(byte(&menu, 0x1b0c), 4, "right stops at the help entry");
    for expected in [3, 2, 1] {
        press(&mut menu, 23);
        assert_eq!(
            byte(&menu, 0x1b0c),
            expected,
            "left selects the previous entry"
        );
    }
    press(&mut menu, 23);
    assert_eq!(byte(&menu, 0x1b0c), 1, "left stops at the start entry");

    let mut load_entry = load(PATH);
    advance(&mut load_entry, 60);
    press(&mut load_entry, 13);
    press(&mut load_entry, 22);
    assert_eq!(byte(&load_entry, 0x1b0c), 2, "the load entry is selected");
    press(&mut load_entry, 13);
    advance(&mut load_entry, 60);
    assert!(
        at(&load_entry, 0x4590, 0x45ab),
        "a missing save falls back to the options"
    );
    assert_eq!(
        byte(&load_entry, 0x1b14),
        0,
        "the map row is selected by default"
    );
    assert_eq!(
        byte(&load_entry, 0x3af0),
        1,
        "the random map is selected by default"
    );
    assert_eq!(
        byte(&load_entry, 0x3ae0),
        2,
        "the standard difficulty is selected by default"
    );
    capture(&load_entry, "war-full-load-options");
    press(&mut load_entry, 27);
    advance(&mut load_entry, 60);
    assert!(
        at(&load_entry, 0x4790, 0x488f),
        "escape returns to the title menu"
    );
    assert_eq!(
        byte(&load_entry, 0x1b0c),
        1,
        "the start entry is selected again"
    );

    let mut options = load(PATH);
    advance(&mut options, 60);
    press(&mut options, 13);
    press(&mut options, 13);
    advance(&mut options, 60);
    assert!(at(&options, 0x4590, 0x45ab), "the options screen runs");
    assert_eq!(byte(&options, 0x1b14), 0, "the map row is selected");
    assert_eq!(byte(&options, 0x3af0), 1, "the random map is selected");
    assert_eq!(
        byte(&options, 0x3ae0),
        2,
        "the standard difficulty is selected"
    );
    capture(&options, "war-full-options");
    press(&mut options, 22);
    assert_eq!(
        byte(&options, 0x3af0),
        2,
        "right selects the downloaded map"
    );
    press(&mut options, 22);
    assert_eq!(
        byte(&options, 0x3af0),
        2,
        "right stops at the downloaded map"
    );
    press(&mut options, 23);
    assert_eq!(byte(&options, 0x3af0), 1, "left selects the random map");
    press(&mut options, 23);
    assert_eq!(byte(&options, 0x3af0), 1, "left stops at the random map");
    press(&mut options, 21);
    assert_eq!(byte(&options, 0x1b14), 1, "down selects the difficulty row");
    press(&mut options, 22);
    assert_eq!(
        byte(&options, 0x3ae0),
        3,
        "right selects the hard difficulty"
    );
    press(&mut options, 22);
    assert_eq!(
        byte(&options, 0x3ae0),
        3,
        "right stops at the hard difficulty"
    );
    press(&mut options, 23);
    assert_eq!(
        byte(&options, 0x3ae0),
        2,
        "left returns to the standard difficulty"
    );
    press(&mut options, 23);
    assert_eq!(
        byte(&options, 0x3ae0),
        1,
        "left selects the easy difficulty"
    );
    press(&mut options, 23);
    assert_eq!(
        byte(&options, 0x3ae0),
        1,
        "left stops at the easy difficulty"
    );
    press(&mut options, 21);
    assert_eq!(byte(&options, 0x1b14), 2, "down selects the name row");
    press(&mut options, 21);
    assert_eq!(byte(&options, 0x1b14), 2, "down stops at the name row");
    for key in *b"play" {
        press(&mut options, key);
    }
    assert_eq!(
        &options.vm().memory()[0x2c67..0x2c6b],
        &b"PLAY"[..],
        "the name is stored in upper case"
    );
    assert_eq!(
        byte(&options, 0x2c6b),
        0,
        "the name buffer ends after four characters"
    );
    press(&mut options, 13);
    advance(&mut options, 60);
    assert!(at(&options, 0x1e17, 0x1e2a), "the size screen opens");
    assert_eq!(byte(&options, 0x3b29), 20, "the width starts at twenty");
    assert_eq!(byte(&options, 0x3b2b), 15, "the height starts at fifteen");
    capture(&options, "war-full-size");
    press(&mut options, 23);
    assert_eq!(byte(&options, 0x3b29), 20, "the width stops at twenty");
    press(&mut options, 21);
    assert_eq!(byte(&options, 0x3b2b), 15, "the height stops at fifteen");
    press(&mut options, 22);
    assert_eq!(byte(&options, 0x3b29), 21, "right widens the map");
    press(&mut options, 20);
    assert_eq!(byte(&options, 0x3b2b), 16, "up heightens the map");
    capture(&options, "war-full-size-custom");
    press(&mut options, 13);
    advance(&mut options, 300);
    assert!(at(&options, 0x67c4, 0x67ef), "the generated map runs");
    assert_eq!(byte(&options, 0x3ae5), 21, "the custom width is used");
    assert_eq!(byte(&options, 0x3ae6), 16, "the custom height is used");
    assert_eq!(hero(&options), (9, 3), "the hero starts on the temple");
    capture(&options, "war-full-custom-map");

    let mut map = start();
    assert!(at(&map, 0x67c4, 0x67ef), "the generated map runs");
    assert_eq!(byte(&map, 0x3ae0), 2, "the standard difficulty is used");
    assert_eq!(byte(&map, 0x3af0), 1, "the random map is used");
    assert_eq!(byte(&map, 0x3ae5), 20, "the map is twenty wide");
    assert_eq!(byte(&map, 0x3ae6), 15, "the map is fifteen high");
    assert_eq!(
        byte(&map, 0x3ae9),
        124,
        "the player city has one hundred twenty-four"
    );
    assert_eq!(byte(&map, 0x3aeb), 200, "the computer city has two hundred");
    assert_eq!(hero(&map), (9, 3), "the hero starts on the temple");
    for x in 0..20 {
        assert_eq!(
            cell(&map, x, 0),
            [0, 2, 0x64, 0],
            "the top border is a wall"
        );
    }
    assert_eq!(
        cell(&map, 9, 3),
        [0, 3, 0x43, 0],
        "the temple sits under the hero"
    );
    assert_eq!(cell(&map, 10, 3), [0, 0, 0, 0], "the tile east is empty");
    assert_eq!(
        cell(&map, 11, 3),
        [0, 2, 0x81, 0],
        "the forest sits east of the empty tile"
    );
    assert_eq!(
        cell(&map, 10, 2),
        [0, 3, 0x22, 0],
        "the resource sits north of the empty tile"
    );
    assert_eq!(
        cell(&map, 10, 1),
        [0, 2, 1, 0],
        "the forest sits north of the resource"
    );
    capture(&map, "war-full-map");
    press(&mut map, 22);
    assert_eq!(hero(&map), (10, 3), "right moves the hero");
    press(&mut map, 22);
    assert_eq!(hero(&map), (10, 3), "the forest blocks the hero");
    press(&mut map, 20);
    assert_eq!(hero(&map), (10, 2), "up moves onto the resource");
    press(&mut map, 20);
    assert_eq!(hero(&map), (10, 1), "up moves onto the northern forest");
    press(&mut map, 23);
    assert_eq!(hero(&map), (9, 1), "left moves back to the temple column");
    press(&mut map, 21);
    assert_eq!(hero(&map), (9, 2), "down returns towards the temple");
    capture(&map, "war-full-walk");

    let mut city = start();
    press(&mut city, 13);
    assert!(at(&city, 0x1cbf, 0x1cd0), "the field prompt runs");
    assert_eq!(
        &city.vm().memory()[0x1c97..0x1c9d],
        &[0xce, 0xb4, 0xcd, 0xea, 0xb3, 0xc9],
        "the temple reports the unfinished game"
    );
    capture(&city, "war-full-message-city");
    press(&mut city, 13);
    assert!(at(&city, 0x67c4, 0x67ef), "the prompt returns to the map");

    let mut resource = start();
    press(&mut resource, 22);
    press(&mut resource, 20);
    assert_eq!(hero(&resource), (10, 2), "the hero reaches the resource");
    press(&mut resource, 13);
    assert!(at(&resource, 0x1cbf, 0x1cd0), "the resource prompt runs");
    assert_eq!(
        &resource.vm().memory()[0x1c97..0x1c9f],
        &[0xbf, 0xf3, 0xb2, 0xfa, 0xd7, 0xca, 0xd4, 0xb4],
        "the resource is named"
    );
    assert_eq!(
        &resource.vm().memory()[0x1ca0..0x1caf],
        &[
            0xc8, 0xc3, 0xd3, 0xa2, 0xd0, 0xdb, 0xc0, 0xb4, 0xbf, 0xaa, 0xb7, 0xa2, 0xb0, 0xc9,
            0x7e
        ],
        "the resource asks for a hero"
    );
    capture(&resource, "war-full-message-resource");
    press(&mut resource, 13);
    assert!(
        at(&resource, 0x67c4, 0x67ef),
        "the resource prompt returns to the map"
    );

    let mut system = start();
    press(&mut system, 27);
    assert!(at(&system, 0x1cbf, 0x1cd0), "the system prompt runs");
    assert_eq!(
        &system.vm().memory()[0x1c97..0x1ca0],
        &[0xcf, 0xb5, 0xcd, 0xb3, 0xb2, 0xcb, 0xb5, 0xa5, 0x3a],
        "the system menu is labelled"
    );
    capture(&system, "war-full-system-menu");
    for key in [49, 50, 51, 52] {
        press(&mut system, key);
        assert!(
            at(&system, 0x67c4, 0x67ef),
            "the system entry returns to the map"
        );
        assert_eq!(hero(&system), (9, 3), "the hero stays put");
        if key != 52 {
            press(&mut system, 27);
            assert!(
                at(&system, 0x1cbf, 0x1cd0),
                "escape reopens the system menu"
            );
        }
    }
    assert!(
        system.files_mut().take_dirty_files().is_empty(),
        "the system entries write nothing"
    );

    let mut help = load(PATH);
    advance(&mut help, 60);
    press(&mut help, 13);
    for _ in 0..3 {
        press(&mut help, 22);
    }
    assert_eq!(byte(&help, 0x1b0c), 4, "the help entry is selected");
    press(&mut help, 13);
    advance(&mut help, 60);
    assert_eq!(help.vm().pc(), 0x3bed, "the help notice waits for a key");
    capture(&help, "war-full-help-notice");
    press(&mut help, 13);
    advance(&mut help, 60);
    assert!(
        at(&help, 0x4790, 0x488f),
        "the notice returns to the title menu"
    );
    assert_eq!(byte(&help, 0x1b0c), 1, "the start entry is selected again");

    let mut editor = load(PATH);
    advance(&mut editor, 60);
    press(&mut editor, 13);
    press(&mut editor, 22);
    press(&mut editor, 22);
    assert_eq!(byte(&editor, 0x1b0c), 3, "the map entry is selected");
    press(&mut editor, 13);
    advance(&mut editor, 60);
    assert!(at(&editor, 0x1e17, 0x1e2a), "the editor asks for the size");
    assert_eq!(byte(&editor, 0x3b29), 20, "the editor starts twenty wide");
    assert_eq!(byte(&editor, 0x3b2b), 15, "the editor starts fifteen high");
    press(&mut editor, 13);
    advance(&mut editor, 60);
    assert_eq!(editor.vm().pc(), 0x3bed, "the editor shows the help notice");
    press(&mut editor, 13);
    advance(&mut editor, 120);
    assert!(at(&editor, 0x50f3, 0x5106), "the map editor runs");
    capture(&editor, "war-full-editor");
    press(&mut editor, 13);
    advance(&mut editor, 60);
    assert!(at(&editor, 0x223c, 0x2249), "the editor hint dialog runs");
    assert_eq!(
        &editor.vm().memory()[0x1c81..0x1c91],
        &[
            0xb1, 0xd8, 0xd0, 0xeb, 0xc8, 0xb7, 0xb6, 0xa8, 0xbf, 0xaa, 0xca, 0xbc, 0xce, 0xbb,
            0xd6, 0xc3
        ],
        "the hint asks for the start positions"
    );
    assert_eq!(
        &editor.vm().memory()[0x1c92..0x1ca4],
        &[
            0xb0, 0xb4, 0x73, 0x28, 0xcd, 0xe6, 0xbc, 0xd2, 0x29, 0x63, 0x28, 0xb5, 0xe7, 0xc4,
            0xd4, 0x29, 0xbc, 0xfc
        ],
        "the hint names the marker keys"
    );
    capture(&editor, "war-full-editor-dialog");
    press(&mut editor, 27);
    advance(&mut editor, 60);
    assert!(at(&editor, 0x50f3, 0x5106), "escape closes the hint");
    assert_eq!(hero(&editor), (1, 1), "the cursor starts in the corner");
    press(&mut editor, 22);
    assert_eq!(hero(&editor), (2, 1), "right moves the cursor");
    press(&mut editor, 21);
    assert_eq!(hero(&editor), (2, 2), "down moves the cursor");
    press(&mut editor, 115);
    assert_eq!(
        (byte(&editor, 0x3ada), byte(&editor, 0x3adb)),
        (2, 2),
        "s marks the player start"
    );
    press(&mut editor, 99);
    assert_eq!(
        (byte(&editor, 0x3adc), byte(&editor, 0x3add)),
        (2, 2),
        "c marks the computer start"
    );
    press(&mut editor, 27);
    assert!(
        at(&editor, 0x50f3, 0x5106),
        "escape does not leave the editor"
    );
    press(&mut editor, 113);
    advance(&mut editor, 60);
    assert!(at(&editor, 0x4790, 0x488f), "q returns to the title menu");
    assert_eq!(
        byte(&editor, 0x1b0c),
        1,
        "the start entry is selected again"
    );
    assert!(
        editor.files_mut().take_dirty_files().is_empty(),
        "the editor writes nothing on exit"
    );

    let mut download = load(PATH);
    advance(&mut download, 60);
    press(&mut download, 13);
    press(&mut download, 13);
    press(&mut download, 22);
    assert_eq!(byte(&download, 0x3af0), 2, "the downloaded map is selected");
    press(&mut download, 21);
    press(&mut download, 21);
    press(&mut download, 13);
    advance(&mut download, 60);
    assert!(at(&download, 0x1e17, 0x1e2a), "the size screen opens");
    press(&mut download, 13);
    advance(&mut download, 240);
    assert!(
        at(&download, 0x67c4, 0x67ef),
        "the map runs without a map file"
    );
    assert_eq!(
        byte(&download, 0x3af0),
        2,
        "the downloaded map entry is kept"
    );
    assert_eq!(
        byte(&download, 0x3ae5),
        20,
        "the fallback map is twenty wide"
    );
    assert_eq!(
        byte(&download, 0x3ae6),
        15,
        "the fallback map is fifteen high"
    );
    assert_eq!(hero(&download), (9, 3), "the hero starts on the temple");
    assert_eq!(
        cell(&download, 11, 3),
        [0, 2, 0x81, 0],
        "the fallback map matches the random map"
    );
    capture(&download, "war-full-download-map");
}

#[test]
#[ignore = "requires LAVAX_GAME_ROOT with the game corpus"]
fn crawler_chronicle_intro_pages_logo_and_halt() {
    const PATH: &str = "LAVA1 爬虫大陆编年史/爬虫大陆.lav";

    let mut emulator = load(PATH);
    advance(&mut emulator, 60);
    assert_eq!(emulator.vm().pc(), 0x1f31, "the loading screen runs");
    capture(&emulator, "crawler-loading");

    advance(&mut emulator, 720);
    assert_eq!(emulator.vm().pc(), 0x1fea, "the prologue waits for a key");
    let byte = |emulator: &Emulator, address: usize| emulator.vm().memory()[address];
    assert_eq!(byte(&emulator, 0x1b08), 0xa0, "the script starts at zero");
    capture(&emulator, "crawler-prologue");

    press(&mut emulator, 13);
    assert_eq!(
        emulator.vm().pc(),
        0x1dfb,
        "enter opens the introduction dialogue"
    );
    assert_eq!(byte(&emulator, 0x1b08), 0x41, "the script advances");
    assert_eq!(byte(&emulator, 0x1b0c), 1, "the speaker is selected");
    assert_eq!(byte(&emulator, 0x1b10), 1, "the line is selected");
    capture(&emulator, "crawler-dialog");

    for expected in [0x5a, 0x73, 0x8c] {
        press(&mut emulator, 13);
        assert_eq!(
            byte(&emulator, 0x1b08),
            expected,
            "enter advances one dialogue page"
        );
    }
    capture(&emulator, "crawler-dialog-next");

    press(&mut emulator, 27);
    assert!(
        (0x1ea7..=0x1eab).contains(&emulator.vm().pc()),
        "escape skips to the title logo"
    );
    capture(&emulator, "crawler-logo");

    advance(&mut emulator, 1800);
    assert!(
        (0x1ea7..=0x1eab).contains(&emulator.vm().pc()),
        "the logo keeps waiting"
    );
    let mut halted = None;
    for _ in 0..20 {
        emulator.input_mut().set_keys([13]);
        if let FrameStatus::Halted(code) = emulator.run_frame().unwrap().status {
            halted = Some(code);
            break;
        }
    }
    emulator.input_mut().set_keys([]);
    assert_eq!(halted, Some(0), "a held key ends the preview");
    assert_eq!(emulator.vm().pc(), 0x1ffd, "the logo exit runs");
    capture(&emulator, "crawler-logo-exit");
    assert!(
        emulator.files_mut().take_dirty_files().is_empty(),
        "the preview writes nothing"
    );
}

#[test]
#[ignore = "requires LAVAX_GAME_ROOT with the game corpus"]
fn bomb_man_splash_menus_solo_failure_and_duel_win() {
    const PATH: &str = "LAVA1 版炸弹人/BomB.lav";

    let mut emulator = load(PATH);
    advance(&mut emulator, 1020);
    assert_eq!(
        emulator.vm().pc(),
        0x1a97,
        "the intro splash waits for a key"
    );
    capture(&emulator, "bomb-man-splash");

    press(&mut emulator, 13);
    assert_eq!(emulator.vm().pc(), 0x1d96, "enter opens the title menu");
    capture(&emulator, "bomb-man-title");

    let byte = |emulator: &Emulator, address: usize| emulator.vm().memory()[address];

    press(&mut emulator, 13);
    assert_eq!(emulator.vm().pc(), 0x16cf, "the mode screen opens");
    assert_eq!(byte(&emulator, 0x211f), 0, "two players are selected first");
    press(&mut emulator, 22);
    assert_eq!(emulator.vm().pc(), 0x16cf, "the mode screen keeps running");
    assert_eq!(byte(&emulator, 0x211f), 1, "right selects single player");
    capture(&emulator, "bomb-man-mode");

    press(&mut emulator, 13);
    assert_eq!(emulator.vm().pc(), 0x17a7, "the map screen opens");
    assert_eq!(byte(&emulator, 0x2130), b'1', "the first map is selected");
    press(&mut emulator, 23);
    assert_eq!(byte(&emulator, 0x2130), b'4', "left wraps to the last map");
    press(&mut emulator, 22);
    assert_eq!(
        byte(&emulator, 0x2130),
        b'1',
        "right wraps back to the first map"
    );
    capture(&emulator, "bomb-man-map");

    press(&mut emulator, 13);
    assert_eq!(emulator.vm().pc(), 0x2bdf, "the solo goal screen shows");
    capture(&emulator, "bomb-man-goal");
    advance(&mut emulator, 120);
    assert_eq!(emulator.vm().pc(), 0x2be3, "the goal screen keeps running");

    press(&mut emulator, 13);
    assert_eq!(emulator.vm().pc(), 0x1b38, "the board loading logo shows");
    advance(&mut emulator, 120);
    assert_eq!(emulator.vm().pc(), 0x2f4b, "the solo board runs");
    capture(&emulator, "bomb-man-solo-board");

    press(&mut emulator, 13);
    assert_eq!(byte(&emulator, 0x20c5), 0x14, "enter places a bomb");
    capture(&emulator, "bomb-man-solo-bomb");

    advance(&mut emulator, 180);
    assert_eq!(emulator.vm().pc(), 0x2f4b, "the bomb is still ticking");
    advance(&mut emulator, 60);
    assert_eq!(emulator.vm().pc(), 0x325d, "the bomb explodes");
    assert_eq!(byte(&emulator, 0x20c5), 0x01, "the bomb cell is burnt");
    assert_eq!(
        byte(&emulator, 0x20c4),
        0x5b,
        "the fire spreads beside the bomb"
    );
    capture(&emulator, "bomb-man-boom");
    advance(&mut emulator, 60);
    assert_eq!(emulator.vm().pc(), 0x3274, "the failure message shows");
    capture(&emulator, "bomb-man-solo-failure");

    advance(&mut emulator, 30);
    assert_eq!(emulator.vm().pc(), 0x32a0, "the credits show");
    capture(&emulator, "bomb-man-credits");
    let mut halted = None;
    for _ in 0..120 {
        if let FrameStatus::Halted(code) = emulator.run_frame().unwrap().status {
            halted = Some(code);
            break;
        }
    }
    assert_eq!(halted, Some(0), "the credits end the program");
    assert_eq!(emulator.vm().pc(), 0x32a6, "the exit path runs");

    let mut emulator = load(PATH);
    advance(&mut emulator, 1020);
    press(&mut emulator, 13);
    press(&mut emulator, 13);
    assert_eq!(byte(&emulator, 0x211f), 0, "two players stay selected");
    press(&mut emulator, 13);
    press(&mut emulator, 13);
    assert_eq!(
        emulator.vm().pc(),
        0x1b38,
        "the duel board loading logo shows"
    );
    advance(&mut emulator, 120);
    assert_eq!(emulator.vm().pc(), 0x22e3, "the duel board runs");
    capture(&emulator, "bomb-man-duel-board");

    press(&mut emulator, 13);
    assert_eq!(byte(&emulator, 0x20a6), 0x14, "player one places a bomb");
    advance(&mut emulator, 240);
    assert_eq!(emulator.vm().pc(), 0x22e3, "the duel bomb is still ticking");
    advance(&mut emulator, 60);
    assert_eq!(emulator.vm().pc(), 0x281b, "the duel bomb explodes");
    assert_eq!(byte(&emulator, 0x20a6), 0x01, "the duel bomb cell is burnt");
    assert_eq!(
        byte(&emulator, 0x20a5),
        0x5b,
        "the fire spreads beside the bomb"
    );
    advance(&mut emulator, 60);
    assert_eq!(emulator.vm().pc(), 0x287c, "the winner message shows");
    capture(&emulator, "bomb-man-duel-win");
    advance(&mut emulator, 60);
    assert_eq!(emulator.vm().pc(), 0x28a8, "the duel credits show");
    capture(&emulator, "bomb-man-duel-credits");
    let mut halted = None;
    for _ in 0..3000 {
        if let FrameStatus::Halted(code) = emulator.run_frame().unwrap().status {
            halted = Some(code);
            break;
        }
    }
    assert_eq!(halted, Some(0), "the credits end the duel");
    assert_eq!(emulator.vm().pc(), 0x28ae, "the duel exit path runs");

    assert!(
        emulator.files_mut().take_dirty_files().is_empty(),
        "the game writes nothing"
    );
}

#[test]
#[ignore = "requires LAVAX_GAME_ROOT with the game corpus"]
fn cat_dog_war_2_menus_battle_and_credits() {
    const PATH: &str = "LAVA1 猫狗大战2/猫狗大战Ⅱ.lav";

    let mut emulator = load(PATH);
    let byte = |emulator: &Emulator, address: usize| emulator.vm().memory()[address];
    advance(&mut emulator, 300);
    assert_eq!(emulator.vm().pc(), 0x3985, "the title screen runs");
    capture(&emulator, "cat-dog-war-2-title");

    press(&mut emulator, 109);
    assert_eq!(emulator.vm().pc(), 0x21b2, "the help page opens");
    capture(&emulator, "cat-dog-war-2-help");
    press(&mut emulator, 13);
    assert_eq!(emulator.vm().pc(), 0x225b, "the key guide page opens");
    capture(&emulator, "cat-dog-war-2-keys");
    press(&mut emulator, 13);
    assert_eq!(emulator.vm().pc(), 0x3985, "the title returns");

    press(&mut emulator, 98);
    assert_eq!(emulator.vm().pc(), 0x20d4, "the difficulty screen opens");
    capture(&emulator, "cat-dog-war-2-difficulty");
    press(&mut emulator, 98);
    assert_eq!(emulator.vm().pc(), 0x3203, "the easy arena waits for a key");
    assert_eq!(
        byte(&emulator, 0x2004),
        98,
        "the easy difficulty is recorded"
    );
    assert_eq!(byte(&emulator, 0x2005), 0x14, "Tom starts at full HP");
    assert_eq!(byte(&emulator, 0x2007), 0x14, "Jerry starts at full HP");
    assert_eq!(byte(&emulator, 0x2027), 0x5a, "the easy CPU power is 90");
    capture(&emulator, "cat-dog-war-2-easy-arena");

    press(&mut emulator, 13);
    assert_eq!(emulator.vm().pc(), 0x2f28, "the combat menu opens");
    capture(&emulator, "cat-dog-war-2-combat");
    press(&mut emulator, 21);
    assert_eq!(emulator.vm().pc(), 0x24cc, "the item menu opens");
    assert_eq!(byte(&emulator, 0x202d), 1, "the first item is selected");
    press(&mut emulator, 21);
    assert_eq!(byte(&emulator, 0x202d), 2, "down selects the next item");
    press(&mut emulator, 20);
    assert_eq!(byte(&emulator, 0x202d), 1, "up selects the previous item");
    press(&mut emulator, 20);
    assert_eq!(byte(&emulator, 0x202d), 4, "up wraps to the last item");
    capture(&emulator, "cat-dog-war-2-items");
    press(&mut emulator, 13);
    assert_eq!(
        emulator.vm().pc(),
        0x3203,
        "confirming the item returns to the arena"
    );
    assert_eq!(byte(&emulator, 0x2025), 1, "the turn passes");
    assert_eq!(byte(&emulator, 0x2009), 4, "one turn has been taken");

    press(&mut emulator, 13);
    assert_eq!(emulator.vm().pc(), 0x2f0a, "the second combat menu opens");
    hold(&mut emulator, 32, 60);
    assert_eq!(
        emulator.vm().pc(),
        0x2f0a,
        "the strike animation keeps the menu"
    );
    assert_eq!(
        byte(&emulator, 0x200b),
        0x66,
        "holding space raises the power"
    );
    assert_eq!(
        byte(&emulator, 0x2021),
        0x24,
        "the strike meter is charging"
    );
    capture(&emulator, "cat-dog-war-2-strike");
    advance(&mut emulator, 340);
    assert_eq!(
        emulator.vm().pc(),
        0x3203,
        "the strike returns to the arena"
    );
    assert_eq!(byte(&emulator, 0x2007), 0x12, "Jerry loses two HP");
    assert_eq!(byte(&emulator, 0x2005), 0x14, "Tom keeps full HP");
    assert_eq!(byte(&emulator, 0x2009), 5, "the exchange advances the turn");
    assert_eq!(byte(&emulator, 0x2021), 0x78, "the strike meter refills");
    capture(&emulator, "cat-dog-war-2-hit");

    press(&mut emulator, 27);
    assert_eq!(emulator.vm().pc(), 0x3404, "escape leaves the match");
    capture(&emulator, "cat-dog-war-2-exit");
    press(&mut emulator, 13);
    assert_eq!(emulator.vm().pc(), 0x3985, "leaving returns to the title");

    press(&mut emulator, 98);
    assert_eq!(
        emulator.vm().pc(),
        0x20d4,
        "the difficulty screen opens again"
    );
    press(&mut emulator, 109);
    assert_eq!(emulator.vm().pc(), 0x3203, "the hard arena waits for a key");
    assert_eq!(
        byte(&emulator, 0x2004),
        109,
        "the hard difficulty is recorded"
    );
    assert_eq!(byte(&emulator, 0x2027), 0x28, "the hard CPU power is 40");
    capture(&emulator, "cat-dog-war-2-hard-arena");
    press(&mut emulator, 27);
    assert_eq!(emulator.vm().pc(), 0x3404, "escape leaves the hard match");
    press(&mut emulator, 13);
    assert_eq!(emulator.vm().pc(), 0x3985, "the title returns again");
    press(&mut emulator, 27);
    assert_eq!(emulator.vm().pc(), 0x206c, "the credits show");
    capture(&emulator, "cat-dog-war-2-credits");
    let mut halted = None;
    emulator.input_mut().set_keys([13]);
    for _ in 0..30 {
        if let FrameStatus::Halted(code) = emulator.run_frame().unwrap().status {
            halted = Some(code);
            break;
        }
    }
    emulator.input_mut().set_keys([]);
    assert_eq!(halted, Some(0), "the credits end the program");
    assert_eq!(emulator.vm().pc(), 0x2070, "the exit path runs");

    let mut emulator = load(PATH);
    advance(&mut emulator, 300);
    press(&mut emulator, 110);
    assert_eq!(
        emulator.vm().pc(),
        0x3651,
        "the two-player arena waits for a key"
    );
    assert_eq!(byte(&emulator, 0x2005), 0x14, "Tom starts at full HP");
    assert_eq!(byte(&emulator, 0x2007), 0x14, "Jerry starts at full HP");
    capture(&emulator, "cat-dog-war-2-duel-arena");
    press(&mut emulator, 13);
    assert_eq!(
        emulator.vm().pc(),
        0x2f28,
        "the first player's combat menu opens"
    );
    press(&mut emulator, 21);
    assert_eq!(
        emulator.vm().pc(),
        0x24cc,
        "the first player's item menu opens"
    );
    assert_eq!(byte(&emulator, 0x202d), 1, "the first item is selected");
    press(&mut emulator, 13);
    assert_eq!(
        emulator.vm().pc(),
        0x3651,
        "confirming the item passes the turn"
    );
    assert_eq!(
        byte(&emulator, 0x2025),
        1,
        "the second player's turn starts"
    );
    press(&mut emulator, 13);
    assert_eq!(
        emulator.vm().pc(),
        0x2f75,
        "the second player's combat menu opens"
    );
    capture(&emulator, "cat-dog-war-2-duel-menu");
    press(&mut emulator, 120);
    assert_eq!(
        emulator.vm().pc(),
        0x2add,
        "the second player's item menu opens"
    );
    assert_eq!(byte(&emulator, 0x202d), 4, "the last item is selected");
    capture(&emulator, "cat-dog-war-2-duel-items");
    press(&mut emulator, 97);
    assert_eq!(
        emulator.vm().pc(),
        0x3651,
        "confirming the item passes the turn back"
    );
    assert_eq!(byte(&emulator, 0x2025), 0, "the first player's turn starts");
    press(&mut emulator, 27);
    assert_eq!(emulator.vm().pc(), 0x3848, "escape leaves the duel");
    capture(&emulator, "cat-dog-war-2-duel-exit");
    press(&mut emulator, 13);
    assert_eq!(emulator.vm().pc(), 0x3985, "the duel returns to the title");

    assert!(
        emulator.files_mut().take_dirty_files().is_empty(),
        "the game writes nothing"
    );
}

#[test]
#[ignore = "requires LAVAX_GAME_ROOT with the game corpus"]
fn biorhythm_dates_help_results_and_exit() {
    const PATH: &str = "LAVA1 生物钟/生物钟.lav";
    const BIRTH: [u8; 9] = [98, 117, 117, 48, 98, 110, 98, 116, 13];

    let mut emulator = load(PATH);
    let byte = |emulator: &Emulator, address: usize| emulator.vm().memory()[address];
    let word = |emulator: &Emulator, address: usize| {
        u16::from_le_bytes(
            emulator.vm().memory()[address..address + 2]
                .try_into()
                .unwrap(),
        )
    };
    advance(&mut emulator, 300);
    assert_eq!(
        emulator.vm().pc(),
        0x414,
        "the birth date form waits for a key"
    );
    assert_eq!(
        &emulator.vm().memory()[0x2000..0x200b],
        b"2000\x0001\x0001\x00",
        "the form starts at 2000/01/01"
    );
    capture(&emulator, "biorhythm-birth");

    press(&mut emulator, 25);
    assert_eq!(emulator.vm().pc(), 0x14f, "the first help page opens");
    capture(&emulator, "biorhythm-help-1");
    press(&mut emulator, 22);
    assert_eq!(emulator.vm().pc(), 0x21c, "the second help page opens");
    capture(&emulator, "biorhythm-help-2");
    press(&mut emulator, 22);
    assert_eq!(emulator.vm().pc(), 0x2f9, "the third help page opens");
    capture(&emulator, "biorhythm-help-3");
    press(&mut emulator, 22);
    assert_eq!(emulator.vm().pc(), 0x2f9, "the last help page stays");
    press(&mut emulator, 23);
    assert_eq!(emulator.vm().pc(), 0x21c, "the second help page returns");
    press(&mut emulator, 23);
    assert_eq!(emulator.vm().pc(), 0x14f, "the first help page returns");
    press(&mut emulator, 23);
    assert_eq!(
        emulator.vm().pc(),
        0x14f,
        "the first page has no previous page"
    );
    press(&mut emulator, 27);
    assert_eq!(
        emulator.vm().pc(),
        0x414,
        "escape returns to the birth form"
    );

    for key in [98, 121, 117, 117] {
        press(&mut emulator, key);
    }
    assert_eq!(byte(&emulator, 0x2028), 0, "a year before 1900 is rejected");
    for key in [98, 117, 117, 48] {
        press(&mut emulator, key);
    }
    assert_eq!(
        byte(&emulator, 0x2028),
        7,
        "a valid year moves to the month"
    );
    for key in [98, 109] {
        press(&mut emulator, key);
    }
    assert_eq!(byte(&emulator, 0x2028), 7, "a month above 12 is rejected");
    for key in [98, 110] {
        press(&mut emulator, key);
    }
    assert_eq!(
        byte(&emulator, 0x2028),
        12,
        "a valid month moves to the day"
    );
    for key in [109, 110] {
        press(&mut emulator, key);
    }
    assert_eq!(byte(&emulator, 0x2028), 12, "a day above 31 is rejected");
    for key in [98, 116] {
        press(&mut emulator, key);
    }
    assert_eq!(
        byte(&emulator, 0x2028),
        13,
        "a valid day completes the form"
    );
    assert_eq!(byte(&emulator, 0x202a), 1, "the birth date is accepted");
    assert_eq!(
        &emulator.vm().memory()[0x2000..0x200b],
        b"1990\x0012\x0017\x00",
        "the birth date is stored"
    );
    press(&mut emulator, 13);
    assert_eq!(emulator.vm().pc(), 0x4a0, "the query form waits for a key");
    assert_eq!(
        &emulator.vm().memory()[0x2000..0x200b],
        b"2000\x0001\x0001\x00",
        "the query form starts at 2000/01/01"
    );
    capture(&emulator, "biorhythm-query");

    for key in BIRTH {
        press(&mut emulator, key);
    }
    assert_eq!(
        emulator.vm().pc(),
        0xb0,
        "a query on the birth date is refused"
    );
    capture(&emulator, "biorhythm-refused");
    let mut halted = None;
    emulator.input_mut().set_keys([32]);
    for _ in 0..60 {
        if let FrameStatus::Halted(code) = emulator.run_frame().unwrap().status {
            halted = Some(code);
            break;
        }
    }
    emulator.input_mut().set_keys([]);
    assert_eq!(halted, Some(0), "the refusal ends the program");
    assert_eq!(emulator.vm().pc(), 0x1edd, "the exit path runs");

    let mut emulator = load(PATH);
    advance(&mut emulator, 300);
    for key in BIRTH {
        press(&mut emulator, key);
    }
    assert_eq!(emulator.vm().pc(), 0x4a0, "the query form opens");
    for key in [98, 117, 117, 48, 98, 110, 98, 121, 13] {
        press(&mut emulator, key);
    }
    assert_eq!(
        emulator.vm().pc(),
        0x5b4,
        "the result screen waits for a key"
    );
    assert_eq!(
        &emulator.vm().memory()[0x2017..0x201c],
        b"25%\0\0",
        "the physical cycle is 25 percent"
    );
    assert_eq!(
        &emulator.vm().memory()[0x201c..0x2021],
        b"64%\0\0",
        "the emotional cycle is 64 percent"
    );
    assert_eq!(
        &emulator.vm().memory()[0x2021..0x2026],
        b"68%\0\0",
        "the intellectual cycle is 68 percent"
    );
    assert_eq!(
        &emulator.vm().memory()[0x2000..0x200b],
        b"1990\x0012\x0018\x00",
        "the query date is shown"
    );
    assert_eq!(word(&emulator, 0x2032), 1991, "the next birthday year");
    assert_eq!(word(&emulator, 0x2034), 12, "the next birthday month");
    assert_eq!(word(&emulator, 0x2036), 17, "the next birthday day");
    assert_eq!(word(&emulator, 0x2038), 1990, "the query year is stored");
    assert_eq!(word(&emulator, 0x203a), 12, "the query month is stored");
    assert_eq!(word(&emulator, 0x203c), 18, "the query day is stored");
    assert_eq!(word(&emulator, 0x203e), 366, "the elapsed day count");
    capture(&emulator, "biorhythm-result-18");

    press(&mut emulator, 31);
    assert_eq!(emulator.vm().pc(), 0x38a, "the about screen opens");
    capture(&emulator, "biorhythm-about");
    press(&mut emulator, 13);
    assert_eq!(
        emulator.vm().pc(),
        0x1eb8,
        "the result loop resumes behind the about screen"
    );
    let mut halted = None;
    emulator.input_mut().set_keys([27]);
    for _ in 0..60 {
        if let FrameStatus::Halted(code) = emulator.run_frame().unwrap().status {
            halted = Some(code);
            break;
        }
    }
    emulator.input_mut().set_keys([]);
    assert_eq!(halted, Some(0), "escape on the result ends the program");
    assert_eq!(emulator.vm().pc(), 0x1edd, "the exit path runs");

    let mut emulator = load(PATH);
    advance(&mut emulator, 300);
    for key in BIRTH {
        press(&mut emulator, key);
    }
    for key in [98, 117, 117, 48, 98, 110, 110, 103, 13] {
        press(&mut emulator, key);
    }
    assert_eq!(
        emulator.vm().pc(),
        0x5b4,
        "the second result screen waits for a key"
    );
    assert_eq!(&emulator.vm().memory()[0x2017..0x201c], b"83%\0\0");
    assert_eq!(&emulator.vm().memory()[0x201c..0x2021], b"92%\0\0");
    assert_eq!(&emulator.vm().memory()[0x2021..0x2026], b"93%\0\0");
    assert_eq!(word(&emulator, 0x203e), 372, "the elapsed day count grows");
    capture(&emulator, "biorhythm-result-24");

    let mut emulator = load(PATH);
    advance(&mut emulator, 300);
    for _ in 0..3 {
        emulator.input_mut().set_keys([27]);
        emulator.run_frame().unwrap();
    }
    emulator.input_mut().set_keys([]);
    let mut halted = None;
    for _ in 0..60 {
        if let FrameStatus::Halted(code) = emulator.run_frame().unwrap().status {
            halted = Some(code);
            break;
        }
    }
    assert_eq!(halted, Some(0), "escape on the birth form ends the program");
    assert_eq!(emulator.vm().pc(), 0x1edd, "the exit path runs");
}

#[test]
#[ignore = "requires LAVAX_GAME_ROOT with the game corpus"]
fn crazy_map_editor_menus_maps_and_saves() {
    const PATH: &str = "LAVA1 疯狂弹球/非pac/mapedit.lav";
    const HEADER: [u8; 20] = [
        0x00, b'M', b'A', b'I', b'N', 0x00, 0xcd, 0xbc, 0x00, 0xbc, 0xd3, 0xd4, 0xd8, 0xb5, 0xd8,
        0xcd, 0xbc, 0x00, 0xca, 0xb9,
    ];

    let byte = |emulator: &Emulator, address: usize| emulator.vm().memory()[address];
    let word = |emulator: &Emulator, address: usize| {
        u16::from_le_bytes(
            emulator.vm().memory()[address..address + 2]
                .try_into()
                .unwrap(),
        )
    };

    let mut emulator = load(PATH);
    advance(&mut emulator, 300);
    assert_eq!(emulator.vm().pc(), 0x1e6, "the main menu waits for a key");
    assert_eq!(word(&emulator, 0x2194), 1, "create map starts selected");
    capture(&emulator, "crazy-map-editor-menu");

    press(&mut emulator, 20);
    assert_eq!(word(&emulator, 0x2194), 0, "up selects exit editing");
    capture(&emulator, "crazy-map-editor-menu-exit");
    press(&mut emulator, 20);
    assert_eq!(word(&emulator, 0x2194), 3, "the selection wraps to help");
    press(&mut emulator, 21);
    assert_eq!(
        word(&emulator, 0x2194),
        0,
        "down wraps back to exit editing"
    );
    press(&mut emulator, 21);
    press(&mut emulator, 21);
    assert_eq!(word(&emulator, 0x2194), 2, "down selects load map");
    capture(&emulator, "crazy-map-editor-menu-load");
    press(&mut emulator, 21);
    assert_eq!(word(&emulator, 0x2194), 3, "down selects help");
    capture(&emulator, "crazy-map-editor-menu-help");

    press(&mut emulator, 13);
    assert_eq!(emulator.vm().pc(), 0xfb1, "the first help page opens");
    capture(&emulator, "crazy-map-editor-help-1");
    for (page, name) in [
        (0x1016, "crazy-map-editor-help-2"),
        (0x107c, "crazy-map-editor-help-3"),
        (0x10ea, "crazy-map-editor-help-4"),
        (0x1169, "crazy-map-editor-help-5"),
        (0x11ef, "crazy-map-editor-help-6"),
        (0x1244, "crazy-map-editor-help-7"),
    ] {
        press(&mut emulator, 32);
        assert_eq!(emulator.vm().pc(), page, "the help pages advance in order");
        capture(&emulator, name);
    }
    press(&mut emulator, 32);
    assert_eq!(
        emulator.vm().pc(),
        0x1e6,
        "the last help page returns to the menu"
    );
    assert_eq!(word(&emulator, 0x2194), 1, "the menu selection resets");

    press(&mut emulator, 13);
    assert_eq!(
        emulator.vm().pc(),
        0x34b,
        "the map name form waits for a key"
    );
    capture(&emulator, "crazy-map-editor-create");
    assert_eq!(byte(&emulator, 0x2188), 0, "the name starts empty");
    for key in *b"bcd" {
        press(&mut emulator, key);
    }
    assert_eq!(
        &emulator.vm().memory()[0x211f..0x2122],
        b"bcd",
        "the typed name is stored"
    );
    assert_eq!(byte(&emulator, 0x2188), 3, "the name length is tracked");
    press(&mut emulator, 13);
    assert_eq!(emulator.vm().pc(), 0xebe, "the level list waits for a key");
    let created = emulator.files_mut().take_dirty_files();
    assert_eq!(created.len(), 1, "creating a map writes one file");
    assert_eq!(
        created[0].0, "/cbmap/bcd.dat",
        "the map file uses the typed name"
    );
    assert_eq!(
        created[0].1.as_slice(),
        HEADER.as_slice(),
        "the created map holds only the header"
    );
    assert_eq!(byte(&emulator, 0x211c), 1, "the first stage is selected");
    assert_eq!(byte(&emulator, 0x211e), 0, "the new map has no entries");
    assert_eq!(
        &emulator.vm().memory()[0x2120..0x212c],
        b"       1   \0",
        "the stage number is formatted into the level list buffer"
    );
    assert_eq!(
        &emulator.vm().memory()[0x214c..0x215b],
        b"/CBMap/bcd.dat\0",
        "the map path is built"
    );
    capture(&emulator, "crazy-map-editor-level-list");
    press(&mut emulator, 22);
    assert_eq!(
        &emulator.vm().memory()[0x2120..0x212c],
        b"       1   \0",
        "right cannot leave the only stage"
    );

    press(&mut emulator, 121);
    assert_eq!(
        emulator.vm().pc(),
        0x1721,
        "confirming a stage opens the editor"
    );
    assert!(
        emulator.vm().memory()[0x2089..0x2102]
            .iter()
            .all(|&value| value == 0),
        "a created map starts empty"
    );
    assert_eq!(
        word(&emulator, 0x216a),
        0,
        "the cursor starts at the first column"
    );
    assert_eq!(
        word(&emulator, 0x216c),
        0,
        "the cursor starts at the first row"
    );
    capture(&emulator, "crazy-map-editor-empty");

    press(&mut emulator, b'b');
    assert_eq!(byte(&emulator, 0x2089), 1, "b paints a class one brick");
    press(&mut emulator, 21);
    press(&mut emulator, b'n');
    assert_eq!(byte(&emulator, 0x208a), 2, "n paints a class two brick");
    press(&mut emulator, 21);
    press(&mut emulator, b'm');
    assert_eq!(byte(&emulator, 0x208b), 3, "m paints an iron plate");
    assert_eq!(word(&emulator, 0x216c), 2, "the cursor follows the edits");
    assert_eq!(
        word(&emulator, 0x2178),
        2,
        "the mirrored row follows the edits"
    );
    assert_eq!(
        word(&emulator, 0x216a),
        0,
        "the column stays at the first cell"
    );
    capture(&emulator, "crazy-map-editor-painted");

    press(&mut emulator, b'c');
    assert_eq!(byte(&emulator, 0x208b), 0, "c clears the current cell");
    press(&mut emulator, b'd');
    assert!(
        emulator.vm().memory()[0x2089..0x2102]
            .iter()
            .all(|&value| value == 0),
        "d clears the whole map"
    );
    press(&mut emulator, b'l');
    assert!(
        emulator.vm().memory()[0x2089..0x2102]
            .iter()
            .all(|&value| value == 0xff),
        "l fills the whole map"
    );
    press(&mut emulator, b'd');
    assert_eq!(
        word(&emulator, 0x216a),
        0,
        "clearing the map resets the cursor column"
    );
    assert_eq!(
        word(&emulator, 0x216c),
        0,
        "clearing the map resets the cursor row"
    );
    for (key, tile) in [
        (b'b', 1),
        (b'n', 2),
        (b'm', 3),
        (b't', 4),
        (b'u', 7),
        (b'y', 8),
        (b'g', 9),
        (b'j', 10),
    ] {
        press(&mut emulator, key);
        assert_eq!(
            byte(&emulator, 0x2089),
            tile,
            "each tile key paints its tile"
        );
        press(&mut emulator, b'd');
    }
    for _ in 0..11 {
        press(&mut emulator, 21);
    }
    assert_eq!(word(&emulator, 0x216c), 0, "the cursor wraps vertically");
    for _ in 0..11 {
        press(&mut emulator, 22);
    }
    assert_eq!(word(&emulator, 0x216a), 0, "the cursor wraps horizontally");

    press(&mut emulator, b'b');
    press(&mut emulator, 21);
    press(&mut emulator, b'n');
    press(&mut emulator, 21);
    press(&mut emulator, b'm');
    press(&mut emulator, 13);
    assert_eq!(emulator.vm().pc(), 0xe32, "the save notice takes over");
    capture(&emulator, "crazy-map-editor-saved");
    advance(&mut emulator, 60);
    press(&mut emulator, 32);
    assert_eq!(
        emulator.vm().pc(),
        0x1721,
        "dismissing the notice returns to the editor"
    );
    let saved = emulator.files_mut().take_dirty_files();
    assert_eq!(saved.len(), 1, "saving writes one file");
    assert_eq!(saved[0].0, "/cbmap/bcd.dat", "the save keeps the map path");
    assert_eq!(
        saved[0].1.len(),
        141,
        "the save holds the header and every cell"
    );
    assert_eq!(
        &saved[0].1[..20],
        &HEADER[..],
        "the saved header is preserved"
    );
    assert_eq!(
        &saved[0].1[20..23],
        &[1_u8, 2, 3],
        "the painted cells are saved"
    );
    assert!(
        saved[0].1[23..].iter().all(|&value| value == 0),
        "the remaining cells are empty"
    );

    press(&mut emulator, b'b');
    press(&mut emulator, 27);
    assert_eq!(
        emulator.vm().pc(),
        0x19ef,
        "leaving an edited map asks to save"
    );
    capture(&emulator, "crazy-map-editor-save-prompt");
    press(&mut emulator, b'n');
    assert_eq!(
        emulator.vm().pc(),
        0xebe,
        "discarding returns to the level list"
    );
    assert!(
        emulator.files_mut().take_dirty_files().is_empty(),
        "discarding writes nothing"
    );
    press(&mut emulator, 121);
    assert_eq!(emulator.vm().pc(), 0x1721, "the stage reopens");
    assert_eq!(
        &emulator.vm().memory()[0x2089..0x208c],
        &[1_u8, 2, 3],
        "the saved map is reloaded"
    );
    assert_eq!(
        word(&emulator, 0x216a),
        0,
        "reopening resets the cursor column"
    );
    assert_eq!(
        word(&emulator, 0x216c),
        0,
        "reopening resets the cursor row"
    );
    capture(&emulator, "crazy-map-editor-reloaded");

    press(&mut emulator, b'h');
    assert_eq!(
        emulator.vm().pc(),
        0xfb1,
        "editor help opens the first page"
    );
    for _ in 0..7 {
        press(&mut emulator, 32);
    }
    assert_eq!(
        emulator.vm().pc(),
        0x1721,
        "editor help returns to the editor"
    );
    press(&mut emulator, 27);
    assert_eq!(
        emulator.vm().pc(),
        0xebe,
        "an unchanged map leaves without prompting"
    );
    press(&mut emulator, 27);
    assert_eq!(
        emulator.vm().pc(),
        0x1e6,
        "a populated level list returns to the menu"
    );
    let rewritten = emulator.files_mut().take_dirty_files();
    assert_eq!(
        rewritten.len(),
        1,
        "leaving the level list writes the level pack"
    );
    assert_eq!(
        rewritten[0].0, "/cbmap/bcd.dat",
        "the level pack keeps the path"
    );
    assert_eq!(
        rewritten[0].1.len(),
        262,
        "the level pack holds the saved map"
    );
    assert_eq!(
        &rewritten[0].1[20..23],
        &[1_u8, 2, 3],
        "the saved map is written back"
    );

    let mut emulator = load(PATH);
    advance(&mut emulator, 300);
    press(&mut emulator, 20);
    assert_eq!(word(&emulator, 0x2194), 0, "exit editing is selected");
    let mut halted = None;
    for _ in 0..60 {
        emulator.input_mut().set_keys([13]);
        if let FrameStatus::Halted(code) = emulator.run_frame().unwrap().status {
            halted = Some(code);
            break;
        }
    }
    emulator.input_mut().set_keys([]);
    assert_eq!(halted, Some(0), "exit editing ends the program");
    assert_eq!(emulator.vm().pc(), 0x1327, "the menu exit path runs");

    let mut emulator = load(PATH);
    advance(&mut emulator, 300);
    press(&mut emulator, 21);
    assert_eq!(word(&emulator, 0x2194), 2, "load map is selected");
    press(&mut emulator, 13);
    assert_eq!(
        emulator.vm().pc(),
        0xebe,
        "the level list shows the built-in map"
    );
    assert_eq!(byte(&emulator, 0x211c), 1, "the first stage is selected");
    assert_eq!(byte(&emulator, 0x211e), 15, "the built-in map has an entry");
    assert_eq!(
        &emulator.vm().memory()[0x214c..0x2160],
        b"/CBMap/beta2map.dat\0",
        "the built-in map path is used"
    );
    press(&mut emulator, 121);
    assert_eq!(emulator.vm().pc(), 0x1721, "the built-in map opens");
    let cells = &emulator.vm().memory()[0x2089..0x2102];
    assert_eq!(cells[12], 1, "the built-in map has a raised border corner");
    assert!(
        cells[34..43].iter().all(|&value| value == 1),
        "the built-in map has a raised border edge"
    );
    assert_eq!(
        &cells[45..54],
        &[1_u8, 2, 1, 2, 1, 2, 1, 2, 1],
        "the built-in map has an alternating inner wall"
    );
    assert!(
        emulator.files_mut().take_dirty_files().is_empty(),
        "loading the built-in map writes nothing"
    );
    capture(&emulator, "crazy-map-editor-built-in-map");
}

#[test]
#[ignore = "requires LAVAX_GAME_ROOT with the game corpus"]
fn crazy_checkers_versions_walk_menus_start_both_modes_and_trade_moves() {
    for path in [
        "LAVA1 疯狂跳棋/打包的文件/fktq.pac",
        "LAVA1 疯狂跳棋/疯狂跳棋.lav",
    ] {
        let byte = |emulator: &Emulator, address: usize| emulator.vm().memory()[address];
        let board =
            |emulator: &Emulator, cell: usize| emulator.vm().memory()[0x2008 + cell * 7 + 5];
        let finish = |emulator: &mut Emulator| {
            for _ in 0..3 {
                emulator.input_mut().set_keys([13]);
                emulator.run_frame().unwrap();
            }
            emulator.input_mut().set_keys([]);
            let mut halted = None;
            for _ in 0..60 {
                if let FrameStatus::Halted(code) = emulator.run_frame().unwrap().status {
                    halted = Some(code);
                    break;
                }
            }
            assert_eq!(halted, Some(0), "a key on the credits ends the program");
            assert_eq!(emulator.vm().pc(), 0x1090, "the exit path runs");
        };

        let mut emulator = load(path);
        advance(&mut emulator, 300);
        assert_eq!(emulator.vm().pc(), 0xacd, "the splash waits for a key");
        capture(&emulator, "crazy-checkers-splash");
        press(&mut emulator, 13);
        assert_eq!(emulator.vm().pc(), 0x1545, "the title waits for a key");
        capture(&emulator, "crazy-checkers-title");
        assert_eq!(
            byte(&emulator, 0x2961),
            0,
            "the first menu item starts selected"
        );
        press(&mut emulator, 21);
        assert_eq!(byte(&emulator, 0x2961), 1, "down selects two players");
        press(&mut emulator, 21);
        assert_eq!(byte(&emulator, 0x2961), 2, "down selects the score table");
        press(&mut emulator, 21);
        assert_eq!(
            byte(&emulator, 0x2961),
            2,
            "the selection stops on the last item"
        );
        press(&mut emulator, 20);
        press(&mut emulator, 20);
        assert_eq!(byte(&emulator, 0x2961), 0, "up returns to one player");
        press(&mut emulator, 20);
        assert_eq!(
            byte(&emulator, 0x2961),
            0,
            "the selection stops on the first item"
        );
        press(&mut emulator, 21);
        press(&mut emulator, 21);
        press(&mut emulator, 13);
        advance(&mut emulator, 120);
        assert_eq!(emulator.vm().pc(), 0xf5a, "the score table waits for a key");
        assert_eq!(
            &emulator.vm().memory()[0x2295..0x22bd],
            b"mbx\0\0mbx\0\0mbx\0\0mbx\0\0mbx\0\0mbx\0\0mbx\0\0mbx\0\0",
            "the score table lists eight names"
        );
        assert_eq!(
            emulator.vm().memory()[0x22bd..0x22c1],
            [1, 1, 1, 1],
            "the score table holds four recorded results"
        );
        assert_eq!(
            &emulator.vm().memory()[0x2d7b..0x2d80],
            b"31:30",
            "the score text shows a full board"
        );
        capture(&emulator, "crazy-checkers-scores");
        press(&mut emulator, 27);
        assert_eq!(emulator.vm().pc(), 0x1545, "escape closes the score table");
        press(&mut emulator, 27);
        advance(&mut emulator, 120);
        assert_eq!(
            emulator.vm().pc(),
            0x1080,
            "escape leaves the title for the credits"
        );
        capture(&emulator, "crazy-checkers-credits");
        finish(&mut emulator);

        let mut emulator = load(path);
        advance(&mut emulator, 300);
        press(&mut emulator, 13);
        press(&mut emulator, 13);
        assert_eq!(
            emulator.vm().pc(),
            0xce6,
            "the one-player name form waits for a key"
        );
        assert_eq!(byte(&emulator, 0x2235), 0, "the player name starts empty");
        assert_eq!(
            &emulator.vm().memory()[0x223d..0x2241],
            b"comp",
            "the computer is named"
        );
        assert_eq!(byte(&emulator, 0x2d7c), 0, "the name length starts empty");
        press(&mut emulator, 13);
        assert_eq!(
            emulator.vm().pc(),
            0x1545,
            "an empty name returns to the title"
        );
        assert_eq!(byte(&emulator, 0x2d7e), 1, "the empty name is rejected");
        assert_eq!(
            byte(&emulator, 0x2d7f),
            13,
            "the rejection records the Enter key"
        );
        press(&mut emulator, 13);
        assert_eq!(emulator.vm().pc(), 0xce6, "the name form reopens");
        assert_eq!(byte(&emulator, 0x2d7e), 0, "reopening clears the rejection");
        for key in *b"abc" {
            press(&mut emulator, key);
        }
        assert_eq!(
            &emulator.vm().memory()[0x2235..0x2238],
            b"abc",
            "the typed name is stored"
        );
        assert_eq!(
            byte(&emulator, 0x2d7c),
            3,
            "the name length follows the typing"
        );
        press(&mut emulator, 13);
        assert_eq!(
            emulator.vm().pc(),
            0x1fc9,
            "a three-letter name starts the board"
        );
        assert_eq!(
            byte(&emulator, 0x2234),
            1,
            "the board runs in one-player mode"
        );
        assert_eq!(byte(&emulator, 0x2232), 3, "white starts with three pieces");
        assert_eq!(byte(&emulator, 0x2233), 3, "black starts with three pieces");
        assert_eq!(
            byte(&emulator, 0x2230),
            2,
            "the cursor starts on the second cell"
        );
        for (cell, owner) in [(0, 1), (4, 2), (26, 2), (34, 1), (56, 1), (60, 2)] {
            assert_eq!(board(&emulator, cell), owner, "cell {cell} starts occupied");
        }
        capture(&emulator, "crazy-checkers-board");

        press(&mut emulator, 21);
        assert_eq!(byte(&emulator, 0x2230), 3);
        press(&mut emulator, 21);
        assert_eq!(byte(&emulator, 0x2230), 4);
        press(&mut emulator, 21);
        assert_eq!(
            byte(&emulator, 0x2230),
            5,
            "the cursor reaches the column bottom"
        );
        press(&mut emulator, 13);
        assert_eq!(
            byte(&emulator, 0x2231),
            0,
            "a black piece cannot be selected"
        );
        assert_eq!(board(&emulator, 4), 2, "the black piece stays put");
        press(&mut emulator, 21);
        assert_eq!(
            byte(&emulator, 0x2230),
            5,
            "down stops at the column bottom"
        );
        for (step, cursor) in [(1, 4), (2, 3), (3, 2), (4, 1)] {
            press(&mut emulator, 20);
            assert_eq!(byte(&emulator, 0x2230), cursor, "up step {step}");
        }
        press(&mut emulator, 20);
        assert_eq!(byte(&emulator, 0x2230), 1, "up stops at the column top");

        for (step, cursor) in [6, 12, 19, 27, 36, 44, 51, 57, 57].into_iter().enumerate() {
            press(&mut emulator, 22);
            assert_eq!(byte(&emulator, 0x2230), cursor, "right step {}", step + 1);
        }
        for (step, cursor) in [52, 46, 39, 31, 23, 16, 10, 5, 5].into_iter().enumerate() {
            press(&mut emulator, 23);
            assert_eq!(byte(&emulator, 0x2230), cursor, "left step {}", step + 1);
        }
        capture(&emulator, "crazy-checkers-cursor");
        for _ in 0..4 {
            press(&mut emulator, 20);
        }
        assert_eq!(
            byte(&emulator, 0x2230),
            1,
            "the cursor returns to the top corner"
        );

        press(&mut emulator, 13);
        assert_eq!(byte(&emulator, 0x2231), 1, "the white piece is selected");
        for (cell, marker) in [
            (1, 8),
            (2, 9),
            (5, 8),
            (6, 8),
            (7, 9),
            (11, 9),
            (12, 9),
            (13, 9),
        ] {
            assert_eq!(
                board(&emulator, cell),
                marker,
                "cell {cell} shows a move marker"
            );
        }
        assert_eq!(board(&emulator, 3), 0, "unmarked cells stay empty");
        capture(&emulator, "crazy-checkers-markers");

        press(&mut emulator, 22);
        assert_eq!(byte(&emulator, 0x2230), 6, "right moves onto a marked cell");
        press(&mut emulator, 13);
        let mut settled = false;
        for _ in 0..600 {
            let result = emulator.run_frame().unwrap();
            assert!(
                !matches!(result.status, FrameStatus::Halted(_)),
                "the game continues"
            );
            if emulator.vm().pc() == 0x1fc9 && matches!(result.status, FrameStatus::WaitingForInput)
            {
                settled = true;
                break;
            }
        }
        assert!(settled, "the computer reply returns control to the board");
        assert_eq!(
            byte(&emulator, 0x2230),
            36,
            "the cursor follows the computer move"
        );
        assert_eq!(byte(&emulator, 0x2231), 0, "the selection is cleared");
        assert_eq!(byte(&emulator, 0x2232), 4, "white gains a piece");
        assert_eq!(byte(&emulator, 0x2233), 4, "black answers with a piece");
        assert_eq!(board(&emulator, 5), 1, "white occupies the marked cell");
        assert_eq!(board(&emulator, 35), 2, "black occupies its answer cell");
        for cell in [1, 2, 6, 7, 11, 12, 13] {
            assert_eq!(board(&emulator, cell), 0, "the move markers are cleared");
        }
        capture(&emulator, "crazy-checkers-moved");
        assert!(
            emulator.files_mut().take_dirty_files().is_empty(),
            "a move writes no files"
        );

        press(&mut emulator, 27);
        assert_eq!(emulator.vm().pc(), 0x227e, "escape asks before leaving");
        capture(&emulator, "crazy-checkers-confirm");
        press(&mut emulator, 110);
        assert_eq!(
            emulator.vm().pc(),
            0x1fc9,
            "answering no returns to the board"
        );
        press(&mut emulator, 27);
        press(&mut emulator, 121);
        advance(&mut emulator, 120);
        assert_eq!(
            emulator.vm().pc(),
            0x1080,
            "answering yes leaves for the credits"
        );
        finish(&mut emulator);

        let mut emulator = load(path);
        advance(&mut emulator, 300);
        press(&mut emulator, 13);
        press(&mut emulator, 21);
        assert_eq!(
            byte(&emulator, 0x2961),
            1,
            "the two-player item is selected"
        );
        press(&mut emulator, 13);
        assert_eq!(
            emulator.vm().pc(),
            0xce6,
            "the two-player name form waits for a key"
        );
        assert_eq!(byte(&emulator, 0x2235), 0, "the first name starts empty");
        assert_eq!(byte(&emulator, 0x223d), 0, "the second name starts empty");
        press(&mut emulator, 13);
        assert_eq!(
            emulator.vm().pc(),
            0xce6,
            "an empty first name advances to the second name"
        );
        assert_eq!(byte(&emulator, 0x2d7e), 1, "the first entry is recorded");
        press(&mut emulator, 13);
        assert_eq!(
            emulator.vm().pc(),
            0x1545,
            "an empty second name returns to the title"
        );
        assert_eq!(byte(&emulator, 0x2d7e), 2, "the empty name is rejected");
        assert_eq!(
            byte(&emulator, 0x2d7f),
            13,
            "the rejection records the Enter key"
        );
        press(&mut emulator, 13);
        assert_eq!(emulator.vm().pc(), 0xce6, "the name form reopens");
        assert_eq!(byte(&emulator, 0x2d7e), 0, "reopening clears the rejection");
        for key in *b"abcd" {
            press(&mut emulator, key);
        }
        assert_eq!(
            emulator.vm().pc(),
            0xce6,
            "four letters advance to the second name"
        );
        assert_eq!(
            &emulator.vm().memory()[0x2235..0x2239],
            b"abcd",
            "the first name is kept"
        );
        assert_eq!(byte(&emulator, 0x223d), 0, "the second name is still empty");
        assert_eq!(byte(&emulator, 0x2d7e), 1, "the first name is accepted");
        for key in *b"defg" {
            press(&mut emulator, key);
        }
        assert_eq!(
            emulator.vm().pc(),
            0x1fc9,
            "four letters start the two-player board"
        );
        assert_eq!(
            &emulator.vm().memory()[0x223d..0x2241],
            b"defg",
            "the second name is kept"
        );
        assert_eq!(
            byte(&emulator, 0x2234),
            0,
            "the board runs in two-player mode"
        );
        assert_eq!(byte(&emulator, 0x2232), 3, "white starts with three pieces");
        assert_eq!(byte(&emulator, 0x2233), 3, "black starts with three pieces");
        assert_eq!(
            byte(&emulator, 0x2230),
            2,
            "the cursor starts on the second cell"
        );
        for (cell, owner) in [(0, 1), (4, 2), (26, 2), (34, 1), (56, 1), (60, 2)] {
            assert_eq!(board(&emulator, cell), owner, "cell {cell} starts occupied");
        }
        capture(&emulator, "crazy-checkers-two-players");

        press(&mut emulator, 27);
        assert_eq!(emulator.vm().pc(), 0x227e, "escape asks before leaving");
        press(&mut emulator, 121);
        advance(&mut emulator, 120);
        assert_eq!(
            emulator.vm().pc(),
            0x1080,
            "answering yes leaves for the credits"
        );
        finish(&mut emulator);
    }
}

#[test]
#[ignore = "requires LAVAX_GAME_ROOT with the game corpus"]
fn bai_bai_kuai_pao_versions_walk_menus_start_the_stage_and_scroll_back() {
    for path in [
        "LAVA1 白白快跑/已打包的/白白快跑.pac",
        "LAVA1 白白快跑/未打包的/LAVA/白白快跑.lav",
    ] {
        let byte = |emulator: &Emulator, address: usize| emulator.vm().memory()[address];
        let frame = |emulator: &Emulator| emulator.display().indexed_frame().to_vec();
        let frame_difference =
            |left: &[u8], right: &[u8]| left.iter().zip(right).filter(|(a, b)| a != b).count();

        let mut emulator = load(path);
        advance(&mut emulator, 300);
        assert_eq!(emulator.vm().pc(), 0x80c, "the splash waits for a key");
        capture(&emulator, "bai-bai-kuai-pao-splash");

        press(&mut emulator, 13);
        advance(&mut emulator, 60);
        assert_eq!(emulator.vm().pc(), 0x1d49, "the level list waits for a key");
        assert_eq!(
            &emulator.vm().memory()[0x1400..0x1410],
            &[
                0xc9, 0xcf, 0xcf, 0xc2, 0xd1, 0xa1, 0xd4, 0xf1, 0xb5, 0xda, 0x20, 0x31, 0x20, 0xb9,
                0xd8, 0x20,
            ],
            "the level list offers the first level"
        );
        assert_eq!(
            byte(&emulator, 0x429c),
            1,
            "the only unlocked level is selected"
        );
        assert_eq!(byte(&emulator, 0x429e), 1, "the level mirror is selected");
        let levels = frame(&emulator);
        capture(&emulator, "bai-bai-kuai-pao-levels");
        let expected_progress = if path.contains("已打包的") {
            vec![("/lavadata/bbkpv.dat".to_owned(), vec![0u8])]
        } else {
            Vec::new()
        };
        assert_eq!(
            emulator.files_mut().take_dirty_files(),
            expected_progress,
            "the level list creates a missing progress file and reuses an existing one"
        );
        press(&mut emulator, 20);
        assert_eq!(frame(&emulator), levels, "up keeps the only level");
        assert_eq!(byte(&emulator, 0x429c), 1, "up keeps the level selected");
        press(&mut emulator, 21);
        assert_eq!(frame(&emulator), levels, "down keeps the only level");
        assert_eq!(byte(&emulator, 0x429e), 1, "down keeps the level mirror");

        press(&mut emulator, 27);
        advance(&mut emulator, 60);
        assert_eq!(emulator.vm().pc(), 0x1bcb, "escape opens the main menu");
        assert_eq!(
            byte(&emulator, 0x429c),
            1,
            "the first menu item starts selected"
        );
        let menu = frame(&emulator);
        capture(&emulator, "bai-bai-kuai-pao-menu");

        for expected in [2, 3, 4, 1] {
            press(&mut emulator, 21);
            assert_eq!(
                byte(&emulator, 0x429c),
                expected,
                "down moves the menu cursor"
            );
            assert_eq!(
                byte(&emulator, 0x429e),
                expected,
                "the menu mirror follows down"
            );
        }
        for expected in [4, 3, 2, 1] {
            press(&mut emulator, 20);
            assert_eq!(
                byte(&emulator, 0x429c),
                expected,
                "up moves the menu cursor"
            );
            assert_eq!(
                byte(&emulator, 0x429e),
                expected,
                "the menu mirror follows up"
            );
        }

        press(&mut emulator, 21);
        press(&mut emulator, 13);
        advance(&mut emulator, 60);
        assert_eq!(
            emulator.vm().pc(),
            0x1d7d,
            "the locked map editor explains itself"
        );
        assert_eq!(
            &emulator.vm().memory()[0x1400..0x1411],
            &[
                0xc7, 0xeb, 0xcf, 0xc8, 0xcd, 0xa8, 0xb9, 0xfd, 0xcb, 0xf9, 0xd3, 0xd0, 0xb9, 0xd8,
                0xbf, 0xa8, 0x2e,
            ],
            "the message asks for every level to be cleared first"
        );
        capture(&emulator, "bai-bai-kuai-pao-locked-editor");
        press(&mut emulator, 97);
        advance(&mut emulator, 60);
        assert_eq!(frame(&emulator), menu, "a key returns to the menu");
        assert_eq!(
            byte(&emulator, 0x429c),
            1,
            "returning resets the menu cursor"
        );

        press(&mut emulator, 21);
        press(&mut emulator, 21);
        press(&mut emulator, 13);
        advance(&mut emulator, 60);
        assert_eq!(emulator.vm().pc(), 0x1ef8, "the about page waits for a key");
        assert!(frame(&emulator) != menu, "the about page replaces the menu");
        capture(&emulator, "bai-bai-kuai-pao-about");
        press(&mut emulator, 97);
        advance(&mut emulator, 60);
        assert_eq!(frame(&emulator), menu, "a key leaves the about page");

        press(&mut emulator, 21);
        press(&mut emulator, 21);
        press(&mut emulator, 21);
        press(&mut emulator, 13);
        advance(&mut emulator, 60);
        assert_eq!(
            emulator.vm().pc(),
            0x1fa8,
            "the author page waits for a key"
        );
        assert!(
            frame(&emulator) != menu,
            "the author page replaces the menu"
        );
        capture(&emulator, "bai-bai-kuai-pao-author");
        press(&mut emulator, 97);
        advance(&mut emulator, 60);
        assert_eq!(frame(&emulator), menu, "a key leaves the author page");

        assert!(
            emulator.files_mut().take_dirty_files().is_empty(),
            "the sub pages write nothing"
        );

        press(&mut emulator, 13);
        advance(&mut emulator, 60);
        assert_eq!(frame(&emulator), levels, "start returns to the level list");
        press(&mut emulator, 13);
        advance(&mut emulator, 60);
        assert_eq!(emulator.vm().pc(), 0x1063, "the first level starts");
        assert_eq!(byte(&emulator, 0x4248), 5, "the player starts with five HP");
        let start = frame(&emulator);
        capture(&emulator, "bai-bai-kuai-pao-stage");

        for _ in 0..5 {
            press(&mut emulator, 100);
        }
        assert_eq!(
            byte(&emulator, 0x4238),
            5,
            "five right steps scroll the stage"
        );
        let scrolled = frame(&emulator);
        assert!(
            frame_difference(&scrolled, &start) > start.len() / 10,
            "walking right changes the stage"
        );
        for _ in 0..5 {
            press(&mut emulator, 97);
        }
        assert_eq!(
            byte(&emulator, 0x4238),
            0x71,
            "five left steps bring the view back"
        );
        let restored = frame(&emulator);
        assert!(
            frame_difference(&restored, &start) < start.len() / 20,
            "walking back restores the scenery"
        );
        assert!(
            frame_difference(&restored, &scrolled) > start.len() / 10,
            "the scrolled stage is not the restored stage"
        );
        assert_eq!(byte(&emulator, 0x4248), 5, "walking keeps five HP");
        capture(&emulator, "bai-bai-kuai-pao-returned");

        press(&mut emulator, 25);
        advance(&mut emulator, 60);
        assert_eq!(emulator.vm().pc(), 0x411, "help waits for a key");
        assert_eq!(
            &emulator.vm().memory()[0x1400..0x1404],
            b"WSAD",
            "help lists the movement keys"
        );
        assert_eq!(
            &emulator.vm().memory()[0x1468..0x1473],
            b"HP:5  KEY:0",
            "help shows the starting HP and key count"
        );
        capture(&emulator, "bai-bai-kuai-pao-help");
        press(&mut emulator, 48);
        advance(&mut emulator, 60);
        assert_eq!(emulator.vm().pc(), 0x1063, "zero closes help");
        assert_eq!(
            frame(&emulator),
            restored,
            "closing help restores the stage"
        );

        press(&mut emulator, 27);
        advance(&mut emulator, 60);
        assert_eq!(emulator.vm().pc(), 0x12b0, "escape asks before leaving");
        let prompt = frame(&emulator);
        assert!(
            frame_difference(&prompt, &restored) > 0,
            "the prompt replaces the stage"
        );
        capture(&emulator, "bai-bai-kuai-pao-exit");
        press(&mut emulator, 110);
        advance(&mut emulator, 60);
        assert_eq!(emulator.vm().pc(), 0x1063, "answering no resumes the stage");
        assert_eq!(
            frame(&emulator),
            restored,
            "answering no restores the stage"
        );

        press(&mut emulator, 27);
        advance(&mut emulator, 60);
        assert_eq!(emulator.vm().pc(), 0x12b0, "escape asks again");
        press(&mut emulator, 121);
        advance(&mut emulator, 60);
        assert_eq!(frame(&emulator), menu, "answering yes returns to the menu");
        assert_eq!(
            byte(&emulator, 0x429c),
            1,
            "the menu cursor restarts on the first item"
        );
        assert!(
            emulator.files_mut().take_dirty_files().is_empty(),
            "the menus and the first stage write nothing else"
        );
    }
}

#[test]
#[ignore = "requires LAVAX_GAME_ROOT with the game corpus"]
fn platinum_snake_walks_menus_sets_speed_steers_pauses_and_saves_a_score() {
    let path = "LAVA1 白金贪吃蛇/白金贪吃蛇.lav";
    let byte = |emulator: &Emulator, address: usize| emulator.vm().memory()[address];
    let word = |emulator: &Emulator, address: usize| {
        u16::from_le_bytes([byte(emulator, address), byte(emulator, address + 1)])
    };
    let region = |emulator: &Emulator, x0: usize, y0: usize, x1: usize, y1: usize| {
        let stride = usize::from(emulator.display().width());
        let frame = emulator.display().indexed_frame();
        (y0..y1)
            .flat_map(|y| frame[y * stride + x0..y * stride + x1].to_vec())
            .collect::<Vec<u8>>()
    };
    let playing = |emulator: &Emulator| matches!(emulator.vm().pc(), 0x3219 | 0x3262);
    let contains = |emulator: &Emulator, needle: &[u8]| {
        emulator
            .vm()
            .memory()
            .windows(needle.len())
            .any(|window| window == needle)
    };
    let score = |emulator: &Emulator| emulator.vm().memory()[0x3368..0x336c].to_vec();
    let start = |emulator: &mut Emulator| {
        advance(emulator, 30);
        assert_eq!(emulator.vm().pc(), 0x17f7, "the title waits for input");
        press(emulator, 13);
        advance(emulator, 180);
        assert_eq!(emulator.vm().pc(), 0x3219, "the new game starts");
    };
    let eat_five = |emulator: &mut Emulator| {
        press(emulator, 20);
        advance(emulator, 12);
        press(emulator, 23);
        advance(emulator, 12);
        press(emulator, 21);
        advance(emulator, 12);
        press(emulator, 22);
        advance(emulator, 12);
        advance(emulator, 80);
    };

    let mut emulator = load(path);
    advance(&mut emulator, 30);
    assert_eq!(emulator.vm().pc(), 0x17f7, "the title waits for input");
    assert_eq!(
        &emulator.vm().memory()[0x1c00..0x1c13],
        b"/LavaData/snake.sav",
        "the title names the score file"
    );
    assert_eq!(
        &emulator.vm().memory()[0x1c16..0x1c30],
        &[
            0x20, 0x2d, 0x2d, 0x2d, 0x2d, 0xcc, 0xb0, 0x20, 0xb3, 0xd4, 0x20, 0xc9, 0xdf, 0x20,
            0xb0, 0xd7, 0x20, 0xbd, 0xf0, 0x20, 0xb0, 0xe6, 0x2d, 0x2d, 0x2d, 0x2d,
        ],
        "the title banner names the game"
    );
    assert_eq!(
        &emulator.vm().memory()[0x1c31..0x1c3b],
        &[0x31, 0x2e, 0xd0, 0xc2, 0xd3, 0xce, 0xcf, 0xb7, 0x20, 0x20],
        "the menu lists a new game"
    );
    assert_eq!(
        &emulator.vm().memory()[0x1c3c..0x1c46],
        &[0x32, 0x2e, 0xcb, 0xd9, 0xb6, 0xc8, 0xc9, 0xe8, 0xd6, 0xc3],
        "the menu lists the speed setting"
    );
    assert_eq!(
        &emulator.vm().memory()[0x1c47..0x1c51],
        &[0x33, 0x2e, 0xd3, 0xa2, 0xd0, 0xdb, 0xb0, 0xf1, 0x20, 0x20],
        "the menu lists the score board"
    );
    assert_eq!(
        word(&emulator, 0x336e),
        1,
        "the first menu item starts selected"
    );
    capture(&emulator, "platinum-snake-title");
    for expected in [2, 3, 1] {
        press(&mut emulator, 21);
        assert_eq!(
            word(&emulator, 0x336e),
            expected,
            "down walks the menu and wraps"
        );
    }
    for expected in [3, 2, 1] {
        press(&mut emulator, 20);
        assert_eq!(
            word(&emulator, 0x336e),
            expected,
            "up walks the menu and wraps"
        );
    }

    press(&mut emulator, 21);
    press(&mut emulator, 13);
    advance(&mut emulator, 180);
    assert_eq!(
        emulator.vm().pc(),
        0x1874,
        "the speed screen waits for input"
    );
    assert_eq!(byte(&emulator, 0x3337), 5, "the speed starts at five");
    capture(&emulator, "platinum-snake-speed");
    for expected in [6, 7, 8, 9, 10, 1] {
        press(&mut emulator, 21);
        assert_eq!(
            byte(&emulator, 0x3337),
            expected,
            "down raises the speed and wraps"
        );
    }
    for expected in [10, 9, 8, 7, 6, 5] {
        press(&mut emulator, 20);
        assert_eq!(byte(&emulator, 0x3337), expected, "up lowers the speed");
    }
    press(&mut emulator, 13);
    assert_eq!(emulator.vm().pc(), 0x17f7, "enter returns to the title");
    assert_eq!(word(&emulator, 0x336e), 2, "the speed item stays selected");
    assert_eq!(byte(&emulator, 0x3337), 5, "the chosen speed is kept");

    press(&mut emulator, 20);
    press(&mut emulator, 13);
    advance(&mut emulator, 180);
    assert_eq!(emulator.vm().pc(), 0x3219, "the new game starts");
    assert_eq!(byte(&emulator, 0x3337), 5, "the game uses the stored speed");
    assert_eq!(word(&emulator, 0x3343), 7, "the move counter starts here");
    assert_eq!(word(&emulator, 0x333d), 0x16, "the snake starts mid board");
    assert_eq!(
        word(&emulator, 0x333f),
        0,
        "the snake starts on the top row"
    );
    assert_eq!(score(&emulator), [0, 0, 0, 0], "the score starts empty");
    capture(&emulator, "platinum-snake-start");
    advance(&mut emulator, 60);
    assert_eq!(
        word(&emulator, 0x3343),
        16,
        "speed five advances eight steps a second"
    );
    assert_eq!(
        word(&emulator, 0x333d),
        0x1f,
        "the snake crosses to the right half"
    );
    advance(&mut emulator, 60);
    assert_eq!(
        word(&emulator, 0x3343),
        24,
        "the move counter keeps the speed"
    );

    let mut emulator = load(path);
    advance(&mut emulator, 30);
    press(&mut emulator, 21);
    press(&mut emulator, 13);
    advance(&mut emulator, 180);
    assert_eq!(emulator.vm().pc(), 0x1874, "the speed screen reopens");
    for _ in 0..5 {
        press(&mut emulator, 21);
    }
    assert_eq!(byte(&emulator, 0x3337), 10, "the speed can reach ten");
    press(&mut emulator, 13);
    press(&mut emulator, 20);
    press(&mut emulator, 13);
    advance(&mut emulator, 240);
    assert!(playing(&emulator), "the fast game starts");
    assert_eq!(
        word(&emulator, 0x3343),
        118,
        "speed ten counts moves every frame"
    );
    advance(&mut emulator, 60);
    assert_eq!(
        word(&emulator, 0x3343),
        178,
        "speed ten advances sixty moves a second"
    );

    let mut emulator = load(path);
    start(&mut emulator);
    advance(&mut emulator, 60);
    assert_eq!(
        word(&emulator, 0x333d),
        0x1f,
        "the snake reaches the right edge"
    );
    assert_eq!(word(&emulator, 0x333f), 0, "the snake stays on the top row");
    capture(&emulator, "platinum-snake-right-edge");
    hold(&mut emulator, 21, 30);
    assert_eq!(word(&emulator, 0x333d), 0x1f, "down keeps the column");
    assert_eq!(
        word(&emulator, 0x333f),
        8,
        "down moves the snake down the board"
    );
    hold(&mut emulator, 22, 30);
    assert_eq!(
        word(&emulator, 0x333d),
        0,
        "right wraps the snake to the left edge"
    );
    assert_eq!(word(&emulator, 0x333f), 8, "right keeps the row");
    hold(&mut emulator, 20, 30);
    assert_eq!(word(&emulator, 0x333d), 0, "up keeps the column");
    assert_eq!(
        word(&emulator, 0x333f),
        0,
        "up returns the snake to the top row"
    );
    assert!(playing(&emulator), "the snake survives the turns");
    assert_eq!(score(&emulator), [0, 0, 0, 0], "the snake scores nothing");
    assert!(
        emulator.files_mut().take_dirty_files().is_empty(),
        "steering writes nothing"
    );
    capture(&emulator, "platinum-snake-steered");

    let mut emulator = load(path);
    start(&mut emulator);
    eat_five(&mut emulator);
    assert!(playing(&emulator), "the snake keeps playing after eating");
    assert_eq!(score(&emulator), [5, 0, 0, 0], "one food is worth five");
    assert_eq!(word(&emulator, 0x333d), 0x22, "the snake ate at the cell");
    assert_eq!(word(&emulator, 0x333f), 0x13, "the snake ate at the row");
    capture(&emulator, "platinum-snake-after-eating");
    assert!(
        !contains(&emulator, b"PAUSE"),
        "the pause label is not written before pausing"
    );

    press(&mut emulator, 112);
    assert_eq!(emulator.vm().pc(), 0x172b, "p pauses the game");
    let frozen_field = region(&emulator, 2, 22, 120, 76);
    advance(&mut emulator, 90);
    assert_eq!(emulator.vm().pc(), 0x172b, "the paused screen waits");
    assert_eq!(
        word(&emulator, 0x3343),
        0x2c,
        "the pause freezes the move counter"
    );
    assert_eq!(word(&emulator, 0x333d), 0x22, "the pause freezes the snake");
    assert_eq!(word(&emulator, 0x333f), 0x13, "the pause freezes the snake");
    assert_eq!(
        score(&emulator),
        [5, 0, 0, 0],
        "the pause freezes the score"
    );
    assert!(contains(&emulator, b"PAUSE"), "the pause label is visible");
    assert_eq!(
        region(&emulator, 2, 22, 120, 76),
        frozen_field,
        "the board is frozen"
    );
    capture(&emulator, "platinum-snake-paused");
    press(&mut emulator, 112);
    assert!(playing(&emulator), "p resumes the game");
    advance(&mut emulator, 60);
    assert!(
        word(&emulator, 0x3343) > 0x2c,
        "the game moves again after the pause"
    );

    press(&mut emulator, 25);
    assert_eq!(
        emulator.vm().pc(),
        0x161c,
        "help opens on the controls page"
    );
    assert_eq!(
        &emulator.vm().memory()[0x1407..0x1412],
        &[
            0xd3, 0xce, 0x20, 0xcf, 0xb7, 0x20, 0xb0, 0xef, 0x20, 0xd6, 0xfa
        ],
        "help shows the direction keys"
    );
    assert_eq!(score(&emulator), [5, 0, 0, 0], "help freezes the score");
    capture(&emulator, "platinum-snake-help-controls");
    press(&mut emulator, 97);
    assert_eq!(emulator.vm().pc(), 0x168f, "a key opens the rules page");
    assert_eq!(
        &emulator.vm().memory()[0x1418..0x1424],
        &[
            0x20, 0x20, 0xd4, 0xda, 0xd3, 0xce, 0xcf, 0xb7, 0xd6, 0xd0, 0xb2, 0xbb,
        ],
        "the rules warn about the snake body"
    );
    capture(&emulator, "platinum-snake-help-rules");
    press(&mut emulator, 97);
    assert_eq!(emulator.vm().pc(), 0x1711, "a key opens the author page");
    assert_eq!(
        &emulator.vm().memory()[0x1437..0x1441],
        &[0xd6, 0xc6, 0xd7, 0xf7, 0x3a, 0xba, 0xa3, 0xb3, 0xc9, 0x20],
        "the author page names the author"
    );
    assert_eq!(
        &emulator.vm().memory()[0x1441..0x144a],
        &[0xb2, 0xdf, 0xbb, 0xae, 0x3a, 0x64, 0x75, 0x64, 0x75],
        "the author page names the designer"
    );
    capture(&emulator, "platinum-snake-help-author");
    press(&mut emulator, 97);
    assert!(playing(&emulator), "a key closes the help");

    let mut emulator = load(path);
    start(&mut emulator);
    eat_five(&mut emulator);
    assert_eq!(score(&emulator), [5, 0, 0, 0], "the snake ate once");
    press(&mut emulator, 27);
    assert_eq!(emulator.vm().pc(), 0x3254, "escape asks to leave");
    press(&mut emulator, 32);
    assert_eq!(emulator.vm().pc(), 0x10ae, "space starts the farewell");
    advance(&mut emulator, 60);
    assert_eq!(
        emulator.vm().pc(),
        0x1114,
        "the game congratulates the player"
    );
    assert_eq!(
        &emulator.vm().memory()[0x1c6b..0x1c7a],
        &[
            0xb9, 0xa7, 0xcf, 0xb2, 0xc4, 0xfa, 0x2c, 0xc9, 0xcf, 0xb0, 0xf1, 0xc1, 0xcb, 0x21,
            0x00,
        ],
        "the game announces a high score"
    );
    capture(&emulator, "platinum-snake-congratulations");
    press(&mut emulator, 32);
    assert_eq!(emulator.vm().pc(), 0xe2c, "the name form waits for input");
    assert_eq!(
        &emulator.vm().memory()[0x1400..0x1415],
        &[
            0xc4, 0xfa, 0xb5, 0xc4, 0xb4, 0xf3, 0xc3, 0xfb, 0x28, 0x38, 0xd7, 0xd6, 0xbd, 0xda,
            0xd2, 0xd4, 0xc4, 0xda, 0x29, 0x3a, 0x5f,
        ],
        "the form asks for an eight letter name"
    );
    capture(&emulator, "platinum-snake-name-entry");
    for key in b"ABCDEFGH" {
        press(&mut emulator, *key);
        assert_eq!(
            emulator.vm().pc(),
            0xe2c,
            "the form keeps accepting letters"
        );
    }
    press(&mut emulator, 13);
    assert_eq!(emulator.vm().pc(), 0x14f7, "enter opens the score board");
    assert_eq!(
        &emulator.vm().memory()[0x3384..0x338c],
        b"ABCDEFGH",
        "the board stores the name"
    );
    assert_eq!(
        &emulator.vm().memory()[0x337a..0x337e],
        [5, 0, 0, 0],
        "the board stores the score"
    );
    let mut saved = vec![5, 0, 0, 0];
    saved.extend([0u8; 6]);
    saved.extend(b"ABCDEFGH");
    saved.extend([0u8; 37]);
    assert_eq!(
        emulator.files_mut().take_dirty_files(),
        vec![("/lavadata/snake.sav".to_owned(), saved)],
        "the board writes one saved score"
    );
    capture(&emulator, "platinum-snake-score-board");
    press(&mut emulator, 27);
    assert_eq!(emulator.vm().pc(), 0x10ae, "escape leaves the board");
    advance(&mut emulator, 60);
    assert_eq!(emulator.vm().pc(), 0x17f7, "the title returns");
    assert!(
        emulator.files_mut().take_dirty_files().is_empty(),
        "leaving the board writes nothing"
    );

    press(&mut emulator, 27);
    assert_eq!(emulator.vm().pc(), 0x159a, "escape opens the credits");
    assert!(
        contains(&emulator, &[0xd4, 0xd9, 0x20, 0x20, 0xbc, 0xfb]),
        "the credits say goodbye"
    );
    assert!(
        contains(
            &emulator,
            &[0xd6, 0xc6, 0xd7, 0xf7, 0x3a, 0xba, 0xa3, 0xb3, 0xc9]
        ),
        "the credits name the author"
    );
    assert!(
        contains(
            &emulator,
            &[0xb2, 0xdf, 0xbb, 0xae, 0x3a, 0x64, 0x75, 0x64, 0x75]
        ),
        "the credits name the designer"
    );
    capture(&emulator, "platinum-snake-credits");
    let mut halted = None;
    emulator.input_mut().set_keys([97]);
    for frame in 0..90 {
        if frame == 3 {
            emulator.input_mut().set_keys([]);
        }
        if let FrameStatus::Halted(code) = emulator.run_frame().unwrap().status {
            halted = Some(code);
            break;
        }
    }
    assert_eq!(halted, Some(1), "a key on the credits ends the program");
    assert_eq!(emulator.vm().pc(), 0x15a2, "the exit path runs");

    let mut emulator = load(path);
    advance(&mut emulator, 30);
    press(&mut emulator, 21);
    press(&mut emulator, 21);
    press(&mut emulator, 13);
    advance(&mut emulator, 240);
    assert_eq!(
        emulator.vm().pc(),
        0x14f7,
        "the score board waits for input"
    );
    assert_eq!(
        &emulator.vm().memory()[0x1cb1..0x1cb7],
        &[0xb5, 0xda, 0x31, 0xc3, 0xfb, 0x3a],
        "the board starts at rank one"
    );
    assert_eq!(
        &emulator.vm().memory()[0x1d15..0x1d1b],
        &[0xb5, 0xda, 0x35, 0xc3, 0xfb, 0x3a],
        "the board ends at rank five"
    );
    capture(&emulator, "platinum-snake-empty-board");
    press(&mut emulator, 27);
    assert_eq!(emulator.vm().pc(), 0x10ae, "escape leaves the board");
    advance(&mut emulator, 60);
    assert_eq!(emulator.vm().pc(), 0x17f7, "the title returns");
}

#[test]
#[ignore = "requires LAVAX_GAME_ROOT with the game corpus"]
fn instant_worms_builds_walk_menus_edit_maps_and_fight_duels() {
    let std_path = "LAVA1 百战天虫即时版/天虫即时.lav";
    let vs_path = "LAVA1 百战天虫即时版/VS版/GVmaker/天虫即时vs.lav";
    let frame = |emulator: &Emulator| emulator.display().indexed_frame().to_vec();
    let worms = |emulator: &Emulator| {
        let stride = usize::from(emulator.display().width());
        let frame = emulator.display().indexed_frame();
        (3..157)
            .filter(|&x| frame[35 * stride + x] != 0)
            .collect::<Vec<usize>>()
    };
    let banner = [
        0xb0, 0xd9, 0xd5, 0xbd, 0xcc, 0xec, 0xb3, 0xe6, 0xbc, 0xb4, 0xca, 0xb1, 0xb0, 0xe6,
    ];
    let contains = |emulator: &Emulator, needle: &[u8]| {
        emulator
            .vm()
            .memory()
            .windows(needle.len())
            .any(|window| window == needle)
    };
    let open_editor = |emulator: &mut Emulator| {
        advance(emulator, 300);
        assert_eq!(emulator.vm().pc(), 0x2063, "the title waits for input");
        press(emulator, 109);
        assert_eq!(emulator.vm().pc(), 0x1e41, "m opens the map manager");
        press(emulator, 98);
        assert_eq!(emulator.vm().pc(), 0x1920, "a new map opens the editor");
    };

    let mut emulator = load(std_path);
    advance(&mut emulator, 300);
    assert_eq!(emulator.vm().pc(), 0x2063, "the title waits for input");
    assert_eq!(
        &emulator.vm().memory()[0x1c06..0x1c14],
        &banner,
        "the banner names the game"
    );
    assert_eq!(
        &emulator.vm().memory()[0x1c1e..0x1c26],
        &[0xcb, 0xab, 0xc8, 0xcb, 0xb6, 0xd4, 0xd5, 0xbd],
        "the first entry starts a duel"
    );
    assert_eq!(
        &emulator.vm().memory()[0x1c2e..0x1c36],
        &[0xc8, 0xcb, 0xbb, 0xfa, 0xb6, 0xd4, 0xd5, 0xbd],
        "the second entry duels the computer"
    );
    assert_eq!(
        &emulator.vm().memory()[0x1c49..0x1c51],
        &[0xb5, 0xd8, 0xcd, 0xbc, 0xb9, 0xdc, 0xc0, 0xed],
        "the third entry manages maps"
    );
    assert_eq!(
        &emulator.vm().memory()[0x1c59..0x1c5d],
        &[0xc7, 0xf3, 0xd6, 0xfa],
        "the last entry asks for help"
    );
    assert_eq!(
        emulator
            .files()
            .file("/wormmap/default.wmap")
            .map(|map| map.len()),
        Some(1340),
        "the stock map is mounted"
    );
    capture(&emulator, "instant-worms-title");

    let title = frame(&emulator);
    press(&mut emulator, 25);
    assert_eq!(emulator.vm().pc(), 0x2148, "F5 opens the first help page");
    let help = frame(&emulator);
    assert_ne!(help, title, "the help covers the title");
    capture(&emulator, "instant-worms-help");
    press(&mut emulator, 13);
    assert_eq!(emulator.vm().pc(), 0x21ce, "a key turns to the second page");
    assert_ne!(
        frame(&emulator),
        help,
        "the second page differs from the first"
    );
    press(&mut emulator, 13);
    assert_eq!(emulator.vm().pc(), 0x2063, "a key closes the help");
    assert_eq!(frame(&emulator), title, "the title returns unchanged");

    press(&mut emulator, 98);
    assert_eq!(emulator.vm().pc(), 0xce2, "the map browser opens");
    capture(&emulator, "instant-worms-browser");
    press(&mut emulator, 13);
    assert_eq!(emulator.vm().pc(), 0x1476, "the duel replays the map");
    advance(&mut emulator, 60);
    assert_eq!(emulator.vm().pc(), 0x1423, "the duel waits for orders");
    let battle = frame(&emulator);
    advance(&mut emulator, 60);
    assert_eq!(
        frame(&emulator),
        battle,
        "the duel is idle until orders arrive"
    );
    capture(&emulator, "instant-worms-duel");
    assert_eq!(
        worms(&emulator),
        vec![19, 21, 105, 107],
        "both teams stand on the ground"
    );

    hold(&mut emulator, 100, 40);
    assert_eq!(
        worms(&emulator),
        vec![30, 32, 105, 107],
        "d walks the left worm forward"
    );
    hold(&mut emulator, 97, 40);
    assert_eq!(worms(&emulator), vec![19, 21, 105, 107], "a walks it back");
    hold(&mut emulator, 22, 40);
    assert_eq!(
        worms(&emulator),
        vec![19, 21, 111, 113],
        "right walks the right worm forward"
    );
    hold(&mut emulator, 23, 40);
    assert_eq!(frame(&emulator), battle, "walking back restores the stand");

    let mut aimed = emulator.clone();
    hold(&mut aimed, 119, 8);
    assert_ne!(frame(&aimed), battle, "w raises the aiming cross");
    let mut lowered = emulator.clone();
    hold(&mut lowered, 115, 8);
    assert_ne!(frame(&lowered), battle, "s lowers the aiming cross");
    let mut jumped = emulator.clone();
    hold(&mut jumped, 113, 20);
    assert_eq!(jumped.vm().pc(), 0x1423, "q launches the worm");
    assert_ne!(frame(&jumped), battle, "the worm leaves the ground");

    let mut fired = emulator.clone();
    hold(&mut fired, 101, 60);
    assert_eq!(fired.vm().pc(), 0x1476, "e charges and fires");
    assert_ne!(frame(&fired), battle, "the shot replaces the field");
    advance(&mut fired, 120);
    assert_eq!(fired.vm().pc(), 0x1423, "the field waits after the shot");
    assert_ne!(frame(&fired), battle, "the shot changes the ground");
    capture(&fired, "instant-worms-shot");

    let mut paused = emulator.clone();
    press(&mut paused, 27);
    assert_eq!(paused.vm().pc(), 0x13af, "escape pauses the duel");
    assert_ne!(frame(&paused), battle, "the pause dialog covers the field");
    assert!(
        contains(
            &paused,
            &[
                0x59, 0xbc, 0xcc, 0xd0, 0xf8, 0x2f, 0x52, 0xd6, 0xd8, 0xc0, 0xb4, 0x2f, 0x4e, 0xb2,
                0xbb, 0xcd, 0xe6, 0x00,
            ]
        ),
        "the dialog offers continue, restart and quit"
    );
    capture(&paused, "instant-worms-pause");
    press(&mut paused, 121);
    assert_eq!(paused.vm().pc(), 0x1423, "y resumes the duel");
    assert_eq!(frame(&paused), battle, "continuing restores the field");

    let mut restarted = emulator.clone();
    press(&mut restarted, 27);
    assert_eq!(restarted.vm().pc(), 0x13af, "escape pauses again");
    press(&mut restarted, 114);
    assert_eq!(restarted.vm().pc(), 0xce2, "r returns to the map browser");
    press(&mut restarted, 27);
    assert_eq!(restarted.vm().pc(), 0x2063, "escape leaves the browser");

    let mut quit = emulator.clone();
    press(&mut quit, 27);
    assert_eq!(quit.vm().pc(), 0x13af, "escape pauses the duel");
    press(&mut quit, 110);
    assert_eq!(quit.vm().pc(), 0x2063, "n returns to the title");
    assert!(
        quit.files_mut().take_dirty_files().is_empty(),
        "the duel writes nothing"
    );

    let mut ai = load(std_path);
    advance(&mut ai, 300);
    press(&mut ai, 110);
    assert_eq!(ai.vm().pc(), 0xce2, "the computer duel picks a map");
    press(&mut ai, 13);
    assert_eq!(ai.vm().pc(), 0x1476, "the computer duel replays the map");
    advance(&mut ai, 600);
    assert_eq!(
        ai.vm().pc(),
        0x1484,
        "the computer duel waits for the player"
    );
    capture(&ai, "instant-worms-ai-duel");
    press(&mut ai, 27);
    assert_eq!(ai.vm().pc(), 0x14de, "escape leaves the computer duel");
    assert!(
        ai.files_mut().take_dirty_files().is_empty(),
        "the computer duel writes nothing"
    );

    let mut blank = load(std_path);
    advance(&mut blank, 300);
    press(&mut blank, 109);
    assert_eq!(blank.vm().pc(), 0x1e41, "m opens the map manager");
    assert!(
        contains(
            &blank,
            &[
                0xd0, 0xc2, 0xbd, 0xa8, 0xd2, 0xbb, 0xd5, 0xc5, 0xb5, 0xd8, 0xcd, 0xbc,
            ]
        ),
        "the manager offers a new map"
    );
    assert!(
        contains(
            &blank,
            &[
                0xb4, 0xd3, 0xbe, 0xc9, 0xb5, 0xd8, 0xcd, 0xbc, 0xb1, 0xe0, 0xbc, 0xad,
            ]
        ),
        "the manager offers an old map"
    );
    capture(&blank, "instant-worms-map-menu");
    press(&mut blank, 98);
    assert_eq!(blank.vm().pc(), 0x1920, "a new map opens the editor");
    assert!(
        contains(&blank, &[0xcc, 0xa7, 0xb1, 0xca]),
        "the toolbar starts on the pen-up tool"
    );
    assert!(
        contains(&blank, &[0xcf, 0xb8, 0xb1, 0xca]),
        "the toolbar lists the thin pen"
    );
    assert!(
        contains(&blank, &[0xbb, 0xad, 0xb1, 0xca]),
        "the toolbar lists the pen"
    );
    capture(&blank, "instant-worms-editor");
    press(&mut blank, 13);
    assert_eq!(blank.vm().pc(), 0x1f7c, "enter saves the blank map");
    capture(&blank, "instant-worms-saved");
    let saved = blank.files_mut().take_dirty_files();
    assert_eq!(saved.len(), 1, "the save writes one file");
    assert_eq!(
        saved[0].0, "/wormmap/aatbum`li.wmap",
        "the map is stored in the WormMap folder"
    );
    assert_eq!(saved[0].1.len(), 1340, "the saved map keeps the stock size");
    assert!(
        saved[0].1[..0x3c].iter().all(|&byte| byte == 0),
        "the map starts with padding"
    );
    assert_eq!(
        &saved[0].1[0x3c..0x50],
        &[
            0xc4, 0x1a, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x3c, 0x05, 0x00, 0x00, 0x80, 0x00,
            0x00, 0x00, 0x01, 0x00, 0x00, 0x00,
        ],
        "the header records the map size"
    );
    press(&mut blank, 13);
    assert_eq!(blank.vm().pc(), 0x1fae, "enter shows the saved path");
    press(&mut blank, 13);
    assert_eq!(blank.vm().pc(), 0x1e41, "enter returns to the manager");
    press(&mut blank, 27);
    assert_eq!(blank.vm().pc(), 0x2063, "escape leaves the manager");
    assert!(
        blank.files_mut().take_dirty_files().is_empty(),
        "leaving writes nothing"
    );

    let mut drawn = load(std_path);
    open_editor(&mut drawn);
    assert!(
        contains(&drawn, &[0xcc, 0xa7, 0xb1, 0xca]),
        "the editor opens with the same toolbar"
    );
    press(&mut drawn, 27);
    assert_eq!(drawn.vm().pc(), 0x1c56, "escape asks before leaving");
    assert!(
        contains(
            &drawn,
            &[
                0xb1, 0xa3, 0xb4, 0xe6, 0xc2, 0xf0, 0xa3, 0xbf, 0x59, 0xca, 0xc7, 0x2f, 0x4e, 0xb7,
                0xf1, 0x2f, 0x52, 0xb7, 0xb5, 0xbb, 0xd8, 0xb1, 0xe0, 0xbc, 0xad,
            ]
        ),
        "the prompt offers yes, no and return"
    );
    capture(&drawn, "instant-worms-quit-prompt");
    press(&mut drawn, 110);
    assert_eq!(drawn.vm().pc(), 0x1e41, "no discards the map");
    assert!(
        drawn.files_mut().take_dirty_files().is_empty(),
        "declining writes nothing"
    );
    press(&mut drawn, 110);
    assert_eq!(drawn.vm().pc(), 0xce2, "the manager browses saved maps");
    press(&mut drawn, 13);
    assert_eq!(drawn.vm().pc(), 0x1920, "the stock map opens in the editor");
    hold(&mut drawn, 22, 8);
    hold(&mut drawn, 21, 8);
    press(&mut drawn, 103);
    press(&mut drawn, 27);
    assert_eq!(drawn.vm().pc(), 0x1c56, "escape asks again");
    press(&mut drawn, 121);
    assert_eq!(drawn.vm().pc(), 0x1f7c, "yes saves the edited map");
    let edited = drawn.files_mut().take_dirty_files();
    assert_eq!(edited.len(), 1, "the edited map is written once");
    assert_eq!(
        edited[0].0, saved[0].0,
        "the edited map reuses the save path"
    );
    assert_eq!(
        edited[0].1.len(),
        1340,
        "the edited map keeps the stock size"
    );
    assert_eq!(
        &edited[0].1[0x3c..0x50],
        &saved[0].1[0x3c..0x50],
        "the header stays intact"
    );
    assert_ne!(
        edited[0].1, saved[0].1,
        "the drawn point changes the saved map"
    );
    press(&mut drawn, 13);
    assert_eq!(drawn.vm().pc(), 0x1fae, "enter shows the saved path");
    press(&mut drawn, 13);
    assert_eq!(drawn.vm().pc(), 0x1e41, "enter returns to the manager");
    press(&mut drawn, 27);
    assert_eq!(drawn.vm().pc(), 0x2063, "escape leaves the manager");

    let mut halted = None;
    drawn.input_mut().set_keys([27]);
    for frame in 0..90 {
        if frame == 3 {
            drawn.input_mut().set_keys([]);
        }
        if let FrameStatus::Halted(code) = drawn.run_frame().unwrap().status {
            halted = Some(code);
            break;
        }
    }
    assert_eq!(halted, Some(0), "escape on the title ends the program");
    assert_eq!(drawn.vm().pc(), 0x21ee, "the exit path runs");

    let mut emulator = load(vs_path);
    advance(&mut emulator, 300);
    assert_eq!(
        emulator.vm().pc(),
        0x118e,
        "the community build waits on its title"
    );
    assert_eq!(
        &emulator.vm().memory()[0x1c06..0x1c14],
        &banner,
        "the banner names the same game"
    );
    assert_eq!(
        &emulator.vm().memory()[0x1c1e..0x1c26],
        &[0xcb, 0xab, 0xc8, 0xcb, 0xb6, 0xd4, 0xd5, 0xbd],
        "the first entry starts a duel"
    );
    assert_eq!(
        &emulator.vm().memory()[0x1c2e..0x1c36],
        &[0xb5, 0xd8, 0xcd, 0xbc, 0xb9, 0xdc, 0xc0, 0xed],
        "the second entry names the map manager"
    );
    assert_eq!(
        &emulator.vm().memory()[0x1c37..0x1c3d],
        &[0xce, 0xb4, 0xbf, 0xaa, 0xb7, 0xc5],
        "the map manager is marked closed"
    );
    assert_eq!(
        &emulator.vm().memory()[0x1c49..0x1c51],
        &[0xd3, 0xc3, 0xbb, 0xa7, 0xb9, 0xdc, 0xc0, 0xed],
        "the third entry names the user manager"
    );
    assert_eq!(
        emulator
            .files()
            .file("/lavadata/default.wmap")
            .map(|map| map.len()),
        Some(1340),
        "the stock map is mounted"
    );
    capture(&emulator, "instant-worms-vs-title");

    let title = frame(&emulator);
    for key in [110, 109, 25] {
        press(&mut emulator, key);
        assert_eq!(
            emulator.vm().pc(),
            0x118e,
            "the closed entries stay on the title"
        );
        assert_eq!(frame(&emulator), title, "the closed entries draw nothing");
    }
    press(&mut emulator, 98);
    assert_eq!(
        emulator.vm().pc(),
        0x105f,
        "the duel starts without a map browser"
    );
    assert_eq!(
        worms(&emulator),
        vec![39, 41, 119, 121],
        "both teams stand closer together"
    );
    let battle = frame(&emulator);
    advance(&mut emulator, 60);
    assert_eq!(
        frame(&emulator),
        battle,
        "the duel is idle until orders arrive"
    );
    capture(&emulator, "instant-worms-vs-duel");

    hold(&mut emulator, 100, 40);
    assert_eq!(
        worms(&emulator),
        vec![50, 52, 119, 121],
        "d walks the left worm forward"
    );
    hold(&mut emulator, 97, 40);
    assert_eq!(worms(&emulator), vec![39, 41, 119, 121], "a walks it back");
    hold(&mut emulator, 22, 40);
    assert_eq!(
        worms(&emulator),
        vec![39, 41, 125, 127],
        "right walks the right worm forward"
    );
    hold(&mut emulator, 23, 40);
    assert_eq!(
        worms(&emulator),
        vec![39, 41, 119, 121],
        "walking back restores the stand"
    );

    press(&mut emulator, 27);
    assert_eq!(emulator.vm().pc(), 0x10ed, "escape pauses the duel");
    press(&mut emulator, 13);
    assert_eq!(emulator.vm().pc(), 0x11c8, "a key offers a rematch");
    assert_ne!(
        frame(&emulator),
        battle,
        "the rematch prompt covers the field"
    );
    assert!(
        contains(
            &emulator,
            &[
                0xd4, 0xd9, 0xc0, 0xb4, 0xd2, 0xbb, 0xbe, 0xd6, 0xa3, 0xbf, 0x59, 0x2f, 0x4e, 0x00,
            ]
        ),
        "the prompt asks for another round"
    );
    capture(&emulator, "instant-worms-vs-rematch");
    press(&mut emulator, 121);
    assert_eq!(emulator.vm().pc(), 0x105f, "yes restarts the duel");
    assert_eq!(
        worms(&emulator),
        vec![39, 41, 119, 121],
        "the restart resets the field"
    );
    press(&mut emulator, 27);
    assert_eq!(emulator.vm().pc(), 0x10ed, "escape pauses again");
    press(&mut emulator, 13);
    assert_eq!(emulator.vm().pc(), 0x11c8, "the rematch prompt returns");
    press(&mut emulator, 110);
    assert_eq!(emulator.vm().pc(), 0x118e, "no returns to the title");
    assert!(
        emulator.files_mut().take_dirty_files().is_empty(),
        "the community build writes nothing"
    );

    let mut halted = None;
    emulator.input_mut().set_keys([27]);
    for frame in 0..90 {
        if frame == 3 {
            emulator.input_mut().set_keys([]);
        }
        if let FrameStatus::Halted(code) = emulator.run_frame().unwrap().status {
            halted = Some(code);
            break;
        }
    }
    assert_eq!(halted, Some(0), "escape on the title ends the program");
    assert_eq!(emulator.vm().pc(), 0x11dc, "the exit path runs");
}

#[test]
#[ignore = "requires LAVAX_GAME_ROOT with the game corpus"]
fn yo_games_collection_plays_all_minigames_and_scores_them() {
    let pc_path = "LAVA1 礼物大作战/YoGamesPC.lav";
    let wqx_path = "LAVA1 礼物大作战/YoGamesWQX.lav";
    let word = |emulator: &Emulator, address: usize| {
        let memory = emulator.vm().memory();
        u16::from_le_bytes([memory[address], memory[address + 1]])
    };
    let selection = |emulator: &Emulator| word(emulator, 0x1b04);
    let scores = |emulator: &Emulator| {
        (0..4)
            .map(|index| word(emulator, 0x46a7 + index * 2))
            .collect::<Vec<u16>>()
    };
    let lives = |emulator: &Emulator| {
        (0..4)
            .map(|index| word(emulator, 0x46dd + index * 2))
            .collect::<Vec<u16>>()
    };
    let walk = |emulator: &mut Emulator, key: u8, expected: u16, message: &str| {
        press(emulator, key);
        advance(emulator, 1200);
        assert_eq!(selection(emulator), expected, "{message}");
    };
    let menu = |path: &str, menu_pc: usize| {
        let mut emulator = load(path);
        advance(&mut emulator, 300);
        press(&mut emulator, 13);
        advance(&mut emulator, 1200);
        assert_eq!(emulator.vm().pc(), menu_pc, "any key opens the menu");
        emulator
    };

    let mut emulator = load(pc_path);
    advance(&mut emulator, 300);
    assert_eq!(emulator.vm().pc(), 0x5674, "the pc title waits for input");
    for (address, text) in [
        (
            0x2000,
            &[0xc0, 0xf1, 0xce, 0xef, 0xb4, 0xf3, 0xd7, 0xf7, 0xd5, 0xbd],
        ),
        (
            0x200c,
            &[0xb9, 0xa6, 0xb7, 0xf2, 0xc0, 0xde, 0xcc, 0xa8, 0xc8, 0xfc],
        ),
        (
            0x2018,
            &[0xb2, 0xb6, 0xd3, 0xe3, 0xd0, 0xa1, 0xc4, 0xdc, 0xca, 0xd6],
        ),
        (
            0x2024,
            &[0xd6, 0xc6, 0xd7, 0xf7, 0xd7, 0xe9, 0xc3, 0xfb, 0xb5, 0xa5],
        ),
    ] {
        assert_eq!(
            &emulator.vm().memory()[address..address + text.len()],
            text,
            "the menu lists all four entries"
        );
    }
    press(&mut emulator, 13);
    advance(&mut emulator, 1200);
    assert_eq!(emulator.vm().pc(), 0x5cef, "any key opens the menu");
    assert_eq!(selection(&emulator), 1, "the first entry starts selected");
    capture(&emulator, "yo-games-menu");
    for expected in [2, 3, 4, 1] {
        walk(&mut emulator, 21, expected, "down walks the menu and wraps");
    }
    for expected in [4, 3, 2, 1] {
        walk(&mut emulator, 20, expected, "up walks the menu and wraps");
    }
    press(&mut emulator, 20);
    advance(&mut emulator, 1200);
    assert_eq!(selection(&emulator), 4, "up selects the credits");
    press(&mut emulator, 13);
    assert_eq!(emulator.vm().pc(), 0x5e57, "the credits open");
    press(&mut emulator, 32);
    advance(&mut emulator, 1200);
    assert_eq!(emulator.vm().pc(), 0x5cef, "a key returns to the menu");
    press(&mut emulator, 27);
    assert_eq!(emulator.vm().pc(), 0x5e8b, "escape shows the farewell");
    let mut halted = None;
    emulator.input_mut().set_keys([32]);
    for frame in 0..90 {
        if frame == 3 {
            emulator.input_mut().set_keys([]);
        }
        if let FrameStatus::Halted(code) = emulator.run_frame().unwrap().status {
            halted = Some(code);
            break;
        }
    }
    assert_eq!(halted, Some(0), "the farewell ends the pc build");
    assert_eq!(emulator.vm().pc(), 0x5e8f, "the pc exit path runs");
    assert!(
        emulator.files_mut().take_dirty_files().is_empty(),
        "the pc build writes nothing"
    );

    let mut gift = menu(pc_path, 0x5cef);
    press(&mut gift, 13);
    advance(&mut gift, 1200);
    assert_eq!(gift.vm().pc(), 0x308c, "the gift game explains its rules");
    press(&mut gift, 13);
    advance(&mut gift, 1200);
    assert_eq!(gift.vm().pc(), 0x31a8, "a key shows the gift values");
    press(&mut gift, 32);
    advance(&mut gift, 1200);
    assert_eq!(gift.vm().pc(), 0x31d4, "a key seats the four players");
    capture(&gift, "yo-games-gift-board");
    press(&mut gift, 22);
    assert_eq!(
        gift.vm().pc(),
        0x3205,
        "the first right throw starts the rounds"
    );
    for _ in 0..299 {
        press(&mut gift, 22);
    }
    advance(&mut gift, 600);
    assert_eq!(gift.vm().pc(), 0x2eb1, "three hundred rounds end the game");
    assert_eq!(
        scores(&gift),
        vec![0, 3625, 2175, 3025],
        "the right pile scores the gift game"
    );
    capture(&gift, "yo-games-gift-result");
    press(&mut gift, 13);
    advance(&mut gift, 1200);
    assert_eq!(gift.vm().pc(), 0x5cef, "enter returns to the menu");
    assert!(
        gift.files_mut().take_dirty_files().is_empty(),
        "the gift game writes nothing"
    );

    let mut gift_pause = menu(pc_path, 0x5cef);
    press(&mut gift_pause, 13);
    advance(&mut gift_pause, 1200);
    press(&mut gift_pause, 13);
    advance(&mut gift_pause, 1200);
    press(&mut gift_pause, 32);
    advance(&mut gift_pause, 1200);
    assert_eq!(gift_pause.vm().pc(), 0x31d4, "the gift board reopens");
    press(&mut gift_pause, 13);
    advance(&mut gift_pause, 1200);
    assert_eq!(gift_pause.vm().pc(), 0x3205, "enter starts a gift round");
    press(&mut gift_pause, 112);
    advance(&mut gift_pause, 1200);
    assert_eq!(gift_pause.vm().pc(), 0x2b3a, "p pauses the gift game");
    press(&mut gift_pause, 13);
    advance(&mut gift_pause, 1200);
    assert_eq!(gift_pause.vm().pc(), 0x3205, "enter resumes the gift game");
    press(&mut gift_pause, 112);
    advance(&mut gift_pause, 1200);
    assert_eq!(gift_pause.vm().pc(), 0x2b3a, "p pauses the gift game again");
    press(&mut gift_pause, 27);
    advance(&mut gift_pause, 1200);
    assert_eq!(
        gift_pause.vm().pc(),
        0x2eb1,
        "escape from the pause ends the game"
    );
    press(&mut gift_pause, 13);
    advance(&mut gift_pause, 1200);
    assert_eq!(gift_pause.vm().pc(), 0x5cef, "enter returns to the menu");

    let mut gift_quit = menu(pc_path, 0x5cef);
    press(&mut gift_quit, 13);
    advance(&mut gift_quit, 1200);
    press(&mut gift_quit, 32);
    advance(&mut gift_quit, 1200);
    press(&mut gift_quit, 13);
    advance(&mut gift_quit, 1200);
    assert_eq!(
        gift_quit.vm().pc(),
        0x31d4,
        "the gift board waits before the first round"
    );
    press(&mut gift_quit, 112);
    advance(&mut gift_quit, 1200);
    assert_eq!(
        gift_quit.vm().pc(),
        0x2b3a,
        "p pauses before the first round"
    );
    press(&mut gift_quit, 27);
    advance(&mut gift_quit, 1200);
    assert_eq!(
        gift_quit.vm().pc(),
        0x2eb1,
        "escape before the first round ends the game"
    );
    assert_eq!(
        scores(&gift_quit),
        vec![0, 0, 0, 0],
        "an unplayed gift game scores nothing"
    );
    press(&mut gift_quit, 13);
    advance(&mut gift_quit, 1200);
    assert_eq!(gift_quit.vm().pc(), 0x5cef, "enter returns to the menu");

    let mut kung_fu = menu(pc_path, 0x5cef);
    press(&mut kung_fu, 21);
    advance(&mut kung_fu, 1200);
    assert_eq!(selection(&kung_fu), 2, "down selects the kung fu game");
    press(&mut kung_fu, 13);
    advance(&mut kung_fu, 1200);
    assert_eq!(
        kung_fu.vm().pc(),
        0x4515,
        "the kung fu game explains its rules"
    );
    press(&mut kung_fu, 13);
    advance(&mut kung_fu, 1200);
    assert_eq!(kung_fu.vm().pc(), 0x4565, "a key lists the controls");
    press(&mut kung_fu, 13);
    advance(&mut kung_fu, 1200);
    assert_eq!(kung_fu.vm().pc(), 0x4574, "a key enters the arena");
    assert_eq!(
        lives(&kung_fu),
        vec![5, 5, 5, 5],
        "each fighter has five lives"
    );
    advance(&mut kung_fu, 1200);
    capture(&kung_fu, "yo-games-kung-fu");
    for _ in 0..20 {
        press(&mut kung_fu, 113);
    }
    advance(&mut kung_fu, 1200);
    assert_eq!(kung_fu.vm().pc(), 0x432f, "twenty punches end the match");
    assert_eq!(
        lives(&kung_fu),
        vec![0, 2, 0, 0],
        "the second fighter wins the match"
    );
    capture(&kung_fu, "yo-games-kung-fu-result");
    press(&mut kung_fu, 13);
    advance(&mut kung_fu, 1200);
    assert_eq!(kung_fu.vm().pc(), 0x5cef, "enter returns to the menu");
    assert!(
        kung_fu.files_mut().take_dirty_files().is_empty(),
        "the kung fu game writes nothing"
    );

    let mut kung_fu_pause = menu(pc_path, 0x5cef);
    press(&mut kung_fu_pause, 21);
    advance(&mut kung_fu_pause, 1200);
    press(&mut kung_fu_pause, 13);
    advance(&mut kung_fu_pause, 1200);
    press(&mut kung_fu_pause, 13);
    advance(&mut kung_fu_pause, 1200);
    press(&mut kung_fu_pause, 13);
    advance(&mut kung_fu_pause, 1200);
    assert_eq!(kung_fu_pause.vm().pc(), 0x4574, "the arena reopens");
    press(&mut kung_fu_pause, 112);
    advance(&mut kung_fu_pause, 1200);
    assert_eq!(kung_fu_pause.vm().pc(), 0x35cf, "p pauses the match");
    press(&mut kung_fu_pause, 13);
    advance(&mut kung_fu_pause, 1200);
    assert_eq!(kung_fu_pause.vm().pc(), 0x45a0, "enter resumes the match");
    press(&mut kung_fu_pause, 112);
    advance(&mut kung_fu_pause, 1200);
    assert_eq!(kung_fu_pause.vm().pc(), 0x35cf, "p pauses the match again");
    press(&mut kung_fu_pause, 27);
    advance(&mut kung_fu_pause, 1200);
    assert_eq!(kung_fu_pause.vm().pc(), 0x4243, "escape aborts the match");
    press(&mut kung_fu_pause, 13);
    advance(&mut kung_fu_pause, 1200);
    assert_eq!(kung_fu_pause.vm().pc(), 0x5cef, "enter returns to the menu");

    let mut fishing = menu(pc_path, 0x5cef);
    press(&mut fishing, 21);
    advance(&mut fishing, 1200);
    press(&mut fishing, 21);
    advance(&mut fishing, 1200);
    assert_eq!(selection(&fishing), 3, "down selects the fishing game");
    press(&mut fishing, 13);
    advance(&mut fishing, 1200);
    assert_eq!(
        fishing.vm().pc(),
        0x5587,
        "the fishing game explains its rules"
    );
    press(&mut fishing, 13);
    advance(&mut fishing, 1200);
    assert_eq!(fishing.vm().pc(), 0x5596, "a key shows the pond");
    press(&mut fishing, 13);
    advance(&mut fishing, 1200);
    assert_eq!(fishing.vm().pc(), 0x55e8, "a key casts the first line");
    advance(&mut fishing, 1200);
    capture(&fishing, "yo-games-fishing");
    for _ in 0..300 {
        press(&mut fishing, 22);
    }
    advance(&mut fishing, 1200);
    assert_eq!(
        fishing.vm().pc(),
        0x53d7,
        "three hundred casts end the round"
    );
    assert_eq!(
        scores(&fishing),
        vec![0, 122, 143, 170],
        "the right casts score the fishing game"
    );
    press(&mut fishing, 13);
    advance(&mut fishing, 1200);
    assert_eq!(fishing.vm().pc(), 0x5cef, "enter returns to the menu");
    assert!(
        fishing.files_mut().take_dirty_files().is_empty(),
        "the fishing game writes nothing"
    );

    let mut wqx = load(wqx_path);
    advance(&mut wqx, 300);
    assert_eq!(wqx.vm().pc(), 0x566e, "the wqx title waits for input");
    press(&mut wqx, 13);
    advance(&mut wqx, 1200);
    assert_eq!(wqx.vm().pc(), 0x5ce9, "any key opens the wqx menu");
    assert_eq!(selection(&wqx), 1, "the wqx menu starts selected");
    capture(&wqx, "yo-games-wqx-menu");
    press(&mut wqx, 20);
    advance(&mut wqx, 1200);
    assert_eq!(selection(&wqx), 4, "up selects the wqx credits");
    press(&mut wqx, 13);
    assert_eq!(wqx.vm().pc(), 0x5e51, "the wqx credits open");
    press(&mut wqx, 32);
    advance(&mut wqx, 1200);
    assert_eq!(wqx.vm().pc(), 0x5ce9, "a key returns to the wqx menu");
    press(&mut wqx, 27);
    assert_eq!(wqx.vm().pc(), 0x5e85, "escape shows the wqx farewell");
    let mut halted = None;
    wqx.input_mut().set_keys([32]);
    for frame in 0..90 {
        if frame == 3 {
            wqx.input_mut().set_keys([]);
        }
        if let FrameStatus::Halted(code) = wqx.run_frame().unwrap().status {
            halted = Some(code);
            break;
        }
    }
    assert_eq!(halted, Some(0), "the farewell ends the wqx build");
    assert_eq!(wqx.vm().pc(), 0x5e89, "the wqx exit path runs");
    assert!(
        wqx.files_mut().take_dirty_files().is_empty(),
        "the wqx build writes nothing"
    );

    let mut wqx_gift = menu(wqx_path, 0x5ce9);
    press(&mut wqx_gift, 13);
    advance(&mut wqx_gift, 1200);
    assert_eq!(
        wqx_gift.vm().pc(),
        0x308c,
        "the wqx gift game explains its rules"
    );
    press(&mut wqx_gift, 13);
    advance(&mut wqx_gift, 1200);
    assert_eq!(wqx_gift.vm().pc(), 0x31a8, "a key shows the gift values");
    press(&mut wqx_gift, 32);
    advance(&mut wqx_gift, 1200);
    assert_eq!(wqx_gift.vm().pc(), 0x31d4, "a key seats the players");
    press(&mut wqx_gift, 22);
    assert_eq!(
        wqx_gift.vm().pc(),
        0x31fe,
        "the first right throw starts the wqx rounds"
    );
    for _ in 0..299 {
        press(&mut wqx_gift, 22);
    }
    advance(&mut wqx_gift, 600);
    assert_eq!(wqx_gift.vm().pc(), 0x2eb1, "the wqx gift game ends");
    assert_eq!(
        scores(&wqx_gift),
        vec![0, 3625, 2175, 3025],
        "the wqx gift game scores the same"
    );
    press(&mut wqx_gift, 13);
    advance(&mut wqx_gift, 1200);
    assert_eq!(wqx_gift.vm().pc(), 0x5ce9, "enter returns to the wqx menu");
    assert!(
        wqx_gift.files_mut().take_dirty_files().is_empty(),
        "the wqx gift game writes nothing"
    );

    let mut wqx_kung_fu = menu(wqx_path, 0x5ce9);
    press(&mut wqx_kung_fu, 21);
    advance(&mut wqx_kung_fu, 1200);
    press(&mut wqx_kung_fu, 13);
    advance(&mut wqx_kung_fu, 1200);
    assert_eq!(
        wqx_kung_fu.vm().pc(),
        0x4512,
        "the wqx kung fu game explains its rules"
    );
    press(&mut wqx_kung_fu, 13);
    advance(&mut wqx_kung_fu, 1200);
    assert_eq!(wqx_kung_fu.vm().pc(), 0x4562, "a key lists the controls");
    press(&mut wqx_kung_fu, 13);
    advance(&mut wqx_kung_fu, 1200);
    assert_eq!(wqx_kung_fu.vm().pc(), 0x4571, "a key enters the wqx arena");
    assert_eq!(
        lives(&wqx_kung_fu),
        vec![5, 5, 5, 5],
        "each wqx fighter has five lives"
    );
    advance(&mut wqx_kung_fu, 1200);
    for _ in 0..20 {
        press(&mut wqx_kung_fu, 113);
    }
    advance(&mut wqx_kung_fu, 1200);
    assert_eq!(wqx_kung_fu.vm().pc(), 0x432c, "the wqx match ends");
    assert_eq!(
        lives(&wqx_kung_fu),
        vec![0, 2, 0, 0],
        "the second wqx fighter wins"
    );
    press(&mut wqx_kung_fu, 13);
    advance(&mut wqx_kung_fu, 1200);
    assert_eq!(
        wqx_kung_fu.vm().pc(),
        0x5ce9,
        "enter returns to the wqx menu"
    );
    assert!(
        wqx_kung_fu.files_mut().take_dirty_files().is_empty(),
        "the wqx kung fu game writes nothing"
    );

    let mut wqx_fishing = menu(wqx_path, 0x5ce9);
    press(&mut wqx_fishing, 21);
    advance(&mut wqx_fishing, 1200);
    press(&mut wqx_fishing, 21);
    advance(&mut wqx_fishing, 1200);
    press(&mut wqx_fishing, 13);
    advance(&mut wqx_fishing, 1200);
    assert_eq!(
        wqx_fishing.vm().pc(),
        0x5581,
        "the wqx fishing game explains its rules"
    );
    press(&mut wqx_fishing, 13);
    advance(&mut wqx_fishing, 1200);
    assert_eq!(wqx_fishing.vm().pc(), 0x5590, "a key shows the wqx pond");
    press(&mut wqx_fishing, 13);
    advance(&mut wqx_fishing, 1200);
    assert_eq!(wqx_fishing.vm().pc(), 0x55e2, "a key casts the wqx line");
    advance(&mut wqx_fishing, 1200);
    for _ in 0..300 {
        press(&mut wqx_fishing, 22);
    }
    advance(&mut wqx_fishing, 1200);
    assert_eq!(wqx_fishing.vm().pc(), 0x53d1, "the wqx fishing round ends");
    assert_eq!(
        scores(&wqx_fishing),
        vec![0, 122, 143, 170],
        "the wqx fishing game scores the same"
    );
    press(&mut wqx_fishing, 13);
    advance(&mut wqx_fishing, 1200);
    assert_eq!(
        wqx_fishing.vm().pc(),
        0x5ce9,
        "enter returns to the wqx menu"
    );
    assert!(
        wqx_fishing.files_mut().take_dirty_files().is_empty(),
        "the wqx fishing game writes nothing"
    );

    let mut wqx_pause = menu(wqx_path, 0x5ce9);
    press(&mut wqx_pause, 21);
    advance(&mut wqx_pause, 1200);
    press(&mut wqx_pause, 13);
    advance(&mut wqx_pause, 1200);
    press(&mut wqx_pause, 13);
    advance(&mut wqx_pause, 1200);
    press(&mut wqx_pause, 13);
    advance(&mut wqx_pause, 1200);
    assert_eq!(wqx_pause.vm().pc(), 0x4571, "the wqx arena reopens");
    press(&mut wqx_pause, 112);
    advance(&mut wqx_pause, 1200);
    assert_eq!(wqx_pause.vm().pc(), 0x35cc, "p pauses the wqx match");
    press(&mut wqx_pause, 13);
    assert_eq!(wqx_pause.vm().pc(), 0x4586, "enter resumes the wqx match");
    press(&mut wqx_pause, 112);
    assert_eq!(wqx_pause.vm().pc(), 0x35cc, "p pauses the wqx match again");
    press(&mut wqx_pause, 27);
    assert_eq!(wqx_pause.vm().pc(), 0x4240, "escape aborts the wqx match");
    press(&mut wqx_pause, 13);
    advance(&mut wqx_pause, 1200);
    assert_eq!(wqx_pause.vm().pc(), 0x5ce9, "enter returns to the wqx menu");
}

#[test]
#[ignore = "requires LAVAX_GAME_ROOT with the game corpus"]
fn secret_realm_rebirth_escapes_ten_generated_mazes() {
    let path = "LAVA1 秘境还生/秘境还生.lav";
    let frame = |e: &Emulator| e.display().indexed_frame().to_vec();
    let step = |e: &mut Emulator, key: u8| {
        for _ in 0..3 {
            e.input_mut().set_keys([key]);
            advance(e, 1);
        }
        e.input_mut().set_keys([]);
        advance(e, 10);
    };
    let enter_maze = |e: &mut Emulator| {
        advance(e, 600);
        assert_eq!(e.vm().pc(), 0x10df, "the title waits for a key");
        press(e, 13);
        advance(e, 60);
        assert_eq!(e.vm().pc(), 0x11ef, "the key help screen appears");
        press(e, 13);
        advance(e, 60);
        assert_eq!(e.vm().pc(), 0x1c68, "the first maze starts");
    };
    let halt_with = |e: &mut Emulator, key: u8| {
        let mut halted = None;
        for frame in 0..90 {
            e.input_mut().set_keys([key]);
            if frame == 3 {
                e.input_mut().set_keys([]);
            }
            if let FrameStatus::Halted(code) = e.run_frame().unwrap().status {
                halted = Some(code);
                break;
            }
        }
        halted
    };

    let mut movement = load(path);
    enter_maze(&mut movement);
    let entry = frame(&movement);
    step(&mut movement, 20);
    advance(&mut movement, 30);
    assert_eq!(frame(&movement), entry, "the border blocks movement up");
    step(&mut movement, 22);
    advance(&mut movement, 30);
    assert_ne!(frame(&movement), entry, "moving right redraws the maze");
    assert!(
        movement.files_mut().take_dirty_files().is_empty(),
        "the maze writes nothing"
    );

    let mut death = load(path);
    enter_maze(&mut death);
    advance(&mut death, 175);
    assert_eq!(death.vm().pc(), 0x1c4f, "the level timer expires");
    assert_eq!(
        &death.vm().memory()[0x1400..0x1412],
        &[
            0xc4, 0xe3, 0xc6, 0xe6, 0xbc, 0xa3, 0xb0, 0xe3, 0xcb, 0xc0, 0xd3, 0xda, 0xb5, 0xda,
            0x31, 0xb9, 0xd8, 0x21,
        ],
        "the death screen reports the level"
    );
    assert_eq!(halt_with(&mut death, 13), Some(0), "a key ends the game");
    assert_eq!(death.vm().pc(), 0x1c53, "the game stops after the death");

    let mut escape = load(path);
    enter_maze(&mut escape);
    assert_eq!(
        halt_with(&mut escape, 27),
        Some(0),
        "escape leaves the maze"
    );
    assert_eq!(escape.vm().pc(), 0x1c97, "escape ends the game");

    let paths = [
        "DDDDDDDDDDRRRRRRRRUURRDDRRRR",
        "DDDDRRRRRRDDLLLLLLDDRRRRDDRRUURRDDRRRRRRRR",
        "RRDDDDRRRRRRDDRRDDLLLLUULLDDDDRRRRRRRRUURRDDRR",
        "RRRRDDDDLLDDLLDDRRRRUURRUURRRRDDLLDDDDRRRRRRRRRR",
        "DDDDDDRRRRRRUULLUUUURRDDRRRRUURRRRDDRRUURRDDDDDDDDDDDD",
        "DDDDDDDDDDDDRRRRUURRUURRRRRRDDDDRRUURRDDRRRR",
        "RRRRRRRRDDDDLLUULLDDDDLLDDDDRRDDRRRRRRRRRRRRRRRRRR",
        "RRDDLLDDDDDDRRRRDDLLDDLLDDRRRRUURRDDRRUURRUURRDDRRRRRRRRUURRUUUURRUUUURRDDDDDDDDDDDD",
        "DDDDDDDDDDRRUUUURRDDDDRRDDDDRRRRRRRRUURRDDRRUURRRRUURRRRUURRRRDDDDDD",
        "DDDDRRRRRRDDDDRRDDLLDDRRRRUUUURRRRRRRRUURRUURRUURRRRDDLLDDDDLLLLDDLLDDRRRRRRRRRRUURRUURRRRDDDDDDDD",
    ];
    let mut cleared = load(path);
    enter_maze(&mut cleared);
    for (level, path) in paths.iter().enumerate() {
        for direction in path.bytes() {
            let key = match direction {
                b'U' => 20,
                b'D' => 21,
                b'R' => 22,
                b'L' => 23,
                _ => unreachable!("the solved path only uses directions"),
            };
            step(&mut cleared, key);
        }
        if level + 1 < paths.len() {
            assert_eq!(
                cleared.vm().pc(),
                0x1c68,
                "maze level {} hands over to the next maze",
                level + 1
            );
        }
    }
    assert_eq!(
        cleared.vm().pc(),
        0x14d6,
        "clearing the tenth maze starts the ending"
    );
    assert_eq!(
        &cleared.vm().memory()[0x1400..0x1410],
        &[
            0xbe, 0xad, 0xb9, 0xfd, 0xbf, 0xd6, 0xb2, 0xc0, 0xb5, 0xc4, 0xcc, 0xd3, 0xc9, 0xfa,
            0xa3, 0xac,
        ],
        "the ending describes the escape"
    );
    let expected = [
        (1, 0x14f1),
        (2, 0x150e),
        (3, 0x1527),
        (4, 0x153c),
        (5, 0x1557),
        (6, 0x1c68),
    ];
    for (presses, pc) in expected {
        press(&mut cleared, 13);
        assert_eq!(
            cleared.vm().pc(),
            pc,
            "ending page {} advances after enter",
            presses
        );
    }
    assert!(
        cleared.files_mut().take_dirty_files().is_empty(),
        "the ending writes nothing"
    );
}

#[test]
#[ignore = "requires LAVAX_GAME_ROOT with the game corpus"]
fn freecell_deals_moves_cards_and_walks_the_menus() {
    let path = "LAVA1 空当接龙/空当接龙.lav";
    let frame = |e: &Emulator| e.display().indexed_frame().to_vec();
    let card = |e: &Emulator, row: usize, column: usize| e.vm().memory()[0x2092 + row * 8 + column];
    let cursor = |e: &Emulator| e.vm().memory()[0x211a];
    let selection = |e: &Emulator| e.vm().memory()[0x211d];
    let deal =
        |e: &Emulator| u16::from_le_bytes([e.vm().memory()[0x210d], e.vm().memory()[0x210e]]);
    let halt_with = |e: &mut Emulator, key: u8| {
        let mut halted = None;
        for frame in 0..90 {
            e.input_mut().set_keys([key]);
            if frame == 3 {
                e.input_mut().set_keys([]);
            }
            if let FrameStatus::Halted(code) = e.run_frame().unwrap().status {
                halted = Some(code);
                break;
            }
        }
        halted
    };

    let mut board = load(path);
    advance(&mut board, 30);
    assert_eq!(board.vm().pc(), 0x17df, "the deal starts ready for input");
    assert_eq!(deal(&board), 346, "the game opens its fixed deal");
    assert_eq!(
        &board.vm().memory()[0x2092..0x20c6],
        &[
            0xa2, 0x11, 0x12, 0x22, 0x82, 0xb0, 0xb2, 0x23, 0x71, 0x43, 0xc2, 0x91, 0x93, 0x80,
            0x81, 0xd0, 0x53, 0x70, 0x40, 0x51, 0xb3, 0x20, 0x21, 0x32, 0x92, 0x31, 0xc0, 0x61,
            0x73, 0x10, 0xc3, 0xa1, 0xa0, 0xd3, 0x33, 0x50, 0xd1, 0x62, 0xc1, 0x13, 0xa3, 0x42,
            0x52, 0x90, 0x72, 0xd2, 0x63, 0x41, 0x30, 0xb1, 0x60, 0x83,
        ],
        "the deal fills the tableau"
    );
    assert_eq!(
        &board.vm().memory()[0x20ee..0x20f6],
        &[0, 0, 0, 0, 0, 0, 0, 0],
        "the foundations and cells start empty"
    );
    assert_eq!(
        &board.vm().memory()[0x20f6..0x20fe],
        &[6, 6, 6, 6, 5, 5, 5, 5]
    );
    assert_eq!(
        &board.vm().memory()[0x20fe..0x2106],
        &[6, 6, 6, 5, 5, 5, 5, 5]
    );
    assert_eq!(
        board.vm().memory()[0x2106],
        200,
        "the times value starts at 200"
    );
    assert_eq!(cursor(&board), 0);
    assert_eq!(selection(&board), 0);
    capture(&board, "freecell-board");

    press(&mut board, 23);
    assert_eq!(cursor(&board), 7, "left wraps from the first column");
    press(&mut board, 23);
    assert_eq!(cursor(&board), 6);
    press(&mut board, 22);
    assert_eq!(cursor(&board), 7);
    press(&mut board, 22);
    assert_eq!(cursor(&board), 0, "right wraps from the last column");

    let idle = frame(&board);
    press(&mut board, 13);
    assert_eq!(selection(&board), 13, "enter selects the bottom card");
    assert_ne!(frame(&board), idle, "the selection inverts the card");
    capture(&board, "freecell-selected");
    press(&mut board, 13);
    assert_eq!(selection(&board), 0, "enter again drops the selection");
    assert_eq!(
        frame(&board),
        idle,
        "dropping the selection restores the frame"
    );

    press(&mut board, 13);
    press(&mut board, 22);
    let before = board.vm().memory()[0x2092..0x20c6].to_vec();
    press(&mut board, 13);
    assert_eq!(
        &board.vm().memory()[0x2092..0x20c6],
        before.as_slice(),
        "moving a three onto a jack is refused"
    );
    assert_eq!(selection(&board), 13, "the refused card stays selected");
    press(&mut board, 23);
    press(&mut board, 13);
    assert_eq!(
        selection(&board),
        0,
        "returning to the source drops the selection"
    );
    assert!(
        board.files_mut().take_dirty_files().is_empty(),
        "the board writes nothing"
    );

    let mut moved = load(path);
    advance(&mut moved, 30);
    press(&mut moved, 13);
    for _ in 0..7 {
        press(&mut moved, 22);
    }
    press(&mut moved, 13);
    assert_eq!(card(&moved, 6, 0), 0xff, "the three leaves its column");
    assert_eq!(card(&moved, 6, 7), 0x30, "the three lands on the four");
    assert_eq!(card(&moved, 5, 7), 0x41);
    assert_eq!(
        &moved.vm().memory()[0x20f6..0x20fe],
        &[5, 6, 6, 6, 5, 5, 5, 6]
    );
    assert_eq!(selection(&moved), 0);
    capture(&moved, "freecell-tableau-move");

    let mut cells = load(path);
    advance(&mut cells, 30);
    for _ in 0..7 {
        press(&mut cells, 22);
    }
    press(&mut cells, 19);
    assert_eq!(
        cells.vm().memory()[0x20f2],
        0x41,
        "key 19 parks the bottom card"
    );
    assert_eq!(
        card(&cells, 4, 7),
        0xff,
        "the exposed ace leaves the column"
    );
    assert_eq!(
        cells.vm().memory()[0x20f1],
        0x13,
        "the exposed ace reaches its foundation"
    );
    assert_eq!(&cells.vm().memory()[0x20ee..0x20f1], &[0, 0, 0]);
    assert_eq!(cells.vm().memory()[0x20f6 + 7], 3);
    assert_eq!(cells.vm().memory()[0x20fe + 7], 3);
    press(&mut cells, 19);
    assert_eq!(
        cells.vm().memory()[0x20f3],
        0xa1,
        "the next card fills the second cell"
    );
    assert_eq!(cells.vm().memory()[0x20f6 + 7], 2);
    assert_eq!(
        cells.vm().memory()[0x20f1],
        0x13,
        "the foundation stays on the ace"
    );
    capture(&cells, "freecell-foundation");
    let before = cells.vm().memory()[0x2092..0x20f6].to_vec();
    press(&mut cells, 14);
    assert_eq!(
        &cells.vm().memory()[0x2092..0x20f6],
        before.as_slice(),
        "key 14 refuses a card that is not next on its foundation"
    );
    assert!(
        cells.files_mut().take_dirty_files().is_empty(),
        "the moves write nothing"
    );

    let mut pickup = load(path);
    advance(&mut pickup, 30);
    press(&mut pickup, 19);
    assert_eq!(
        pickup.vm().memory()[0x20f2],
        0x30,
        "the three waits in the first cell"
    );
    press(&mut pickup, 20);
    assert_eq!(pickup.vm().pc(), 0x13e1, "up opens the free-cell row");
    press(&mut pickup, 13);
    assert_eq!(pickup.vm().pc(), 0x17df, "enter returns to the tableau");
    assert_eq!(selection(&pickup), 20, "the cell card is held");
    for _ in 0..7 {
        press(&mut pickup, 22);
    }
    assert_eq!(
        selection(&pickup),
        20,
        "moving the cursor keeps the held card"
    );
    press(&mut pickup, 13);
    assert_eq!(pickup.vm().memory()[0x20f2], 0, "the cell is released");
    assert_eq!(card(&pickup, 6, 7), 0x30, "the held card lands on the four");
    assert_eq!(card(&pickup, 5, 7), 0x41);
    assert_eq!(pickup.vm().memory()[0x20f6 + 7], 6);
    assert_eq!(selection(&pickup), 0);
    capture(&pickup, "freecell-cell-move");

    let mut quit = load(path);
    advance(&mut quit, 30);
    press(&mut quit, 27);
    assert_eq!(quit.vm().pc(), 0x15a3, "escape opens the menu");
    capture(&quit, "freecell-menu");
    press(&mut quit, 13);
    assert_eq!(quit.vm().pc(), 0x17df, "enter resumes the game");
    press(&mut quit, 27);
    for _ in 0..4 {
        press(&mut quit, 21);
    }
    press(&mut quit, 13);
    assert_eq!(
        quit.vm().pc(),
        0x353,
        "the exit entry asks for confirmation"
    );
    capture(&quit, "freecell-exit-confirm");
    press(&mut quit, 22);
    press(&mut quit, 13);
    assert_eq!(quit.vm().pc(), 0x17df, "cancel returns to the game");
    press(&mut quit, 27);
    for _ in 0..4 {
        press(&mut quit, 21);
    }
    press(&mut quit, 13);
    assert_eq!(
        halt_with(&mut quit, 13),
        Some(0),
        "confirming the exit halts"
    );
    assert_eq!(quit.vm().pc(), 0x1d02, "the game stops after the exit");

    let mut replay = load(path);
    advance(&mut replay, 30);
    press(&mut replay, 28);
    assert_eq!(replay.vm().pc(), 0x353, "replay asks to abandon the game");
    capture(&replay, "freecell-abandon");
    press(&mut replay, 23);
    press(&mut replay, 13);
    assert_eq!(replay.vm().pc(), 0x17df);
    assert_eq!(deal(&replay), 346, "cancel keeps the deal");
    press(&mut replay, 28);
    press(&mut replay, 13);
    assert_eq!(deal(&replay), 346, "replay keeps the same deal");
    assert_eq!(
        card(&replay, 6, 0),
        0x30,
        "replay restores the original tableau"
    );
    press(&mut replay, 29);
    press(&mut replay, 13);
    assert_ne!(deal(&replay), 346, "new game draws another deal");
    assert_ne!(card(&replay, 6, 0), 0x30, "new game replaces the tableau");

    let mut deals = load(path);
    advance(&mut deals, 30);
    press(&mut deals, 30);
    assert_eq!(
        deals.vm().pc(),
        0x353,
        "choosing a deal asks to abandon the game"
    );
    press(&mut deals, 13);
    assert_eq!(deals.vm().pc(), 0x1df, "the deal selector opens");
    capture(&deals, "freecell-deal-select");
    press(&mut deals, 27);
    assert_eq!(deals.vm().pc(), 0x17df, "escape cancels the deal selector");
    assert_eq!(deal(&deals), 346);
    press(&mut deals, 30);
    press(&mut deals, 13);
    press(&mut deals, 49);
    assert_eq!(deals.vm().pc(), 0x2c0, "a digit is accepted");
    press(&mut deals, 13);
    assert_eq!(deals.vm().pc(), 0x17df);
    assert_eq!(deal(&deals), 1, "the typed deal is loaded");
    assert_eq!(
        card(&deals, 6, 0),
        0xd3,
        "the first deal has its own tableau"
    );

    let mut times = load(path);
    advance(&mut times, 30);
    assert_eq!(times.vm().memory()[0x2106], 200);
    press(&mut times, 99);
    assert_eq!(times.vm().pc(), 0x1df, "c opens the times input");
    capture(&times, "freecell-times");
    press(&mut times, 49);
    press(&mut times, 50);
    press(&mut times, 13);
    assert_eq!(times.vm().pc(), 0x17df);
    assert_eq!(
        times.vm().memory()[0x2106],
        12,
        "the typed times value is stored"
    );
    press(&mut times, 99);
    press(&mut times, 27);
    assert_eq!(
        times.vm().memory()[0x2106],
        12,
        "escape keeps the times value"
    );
    assert!(
        times.files_mut().take_dirty_files().is_empty(),
        "the dialogs write nothing"
    );
}

#[test]
#[ignore = "requires LAVAX_GAME_ROOT with the game corpus"]
fn space_war_walks_the_speed_menu_and_fights_the_first_wave() {
    let path = "LAVA1 空间战争/SpaceWar.lav";
    let stage = |e: &Emulator| e.vm().memory()[0x2099];
    let lives = |e: &Emulator| e.vm().memory()[0x209b];
    let score = |e: &Emulator| e.vm().memory()[0x209d];
    let ship = |e: &Emulator| (e.vm().memory()[0x2027], e.vm().memory()[0x2029]);
    let bullets = |e: &Emulator| e.vm().memory()[0x202b];
    let start = |e: &mut Emulator| {
        advance(e, 30);
        press(e, 13);
        press(e, 13);
        advance(e, 80);
        assert_eq!(e.vm().pc(), 0x1164, "the first stage is running");
    };

    let mut title = load(path);
    advance(&mut title, 30);
    assert_eq!(title.vm().pc(), 0x180a, "the splash waits for a key");
    press(&mut title, 13);
    assert_eq!(title.vm().pc(), 0x1843, "the key opens the speed menu");
    assert_eq!(title.vm().memory()[0x2000], 0, "the default speed is zero");
    press(&mut title, 21);
    assert_eq!(title.vm().memory()[0x2000], 1, "down raises the speed");
    press(&mut title, 22);
    assert_eq!(title.vm().memory()[0x2000], 2, "right also raises it");
    press(&mut title, 21);
    assert_eq!(title.vm().memory()[0x2000], 0, "raising wraps to zero");
    press(&mut title, 20);
    assert_eq!(title.vm().memory()[0x2000], 2, "up lowers it with wrap");
    press(&mut title, 23);
    assert_eq!(title.vm().memory()[0x2000], 1, "left also lowers it");
    let mut halt = None;
    for frame in 0..90 {
        title.input_mut().set_keys([27]);
        if frame == 3 {
            title.input_mut().set_keys([]);
        }
        if let FrameStatus::Halted(code) = title.run_frame().unwrap().status {
            halt = Some(code);
            break;
        }
    }
    assert_eq!(halt, Some(0), "escape exits from the speed menu");
    assert_eq!(title.vm().pc(), 0x18c8);

    let mut wave = load(path);
    start(&mut wave);
    assert_eq!(stage(&wave), 1);
    assert_eq!(lives(&wave), 5);
    assert_eq!(ship(&wave), (40, 60));
    assert_eq!(bullets(&wave), 0, "no bullet is in flight");
    assert_eq!(
        &wave.vm().memory()[0x2018..0x201f],
        b"STAGE 1",
        "the hud names the stage"
    );

    let mut left = load(path);
    start(&mut left);
    hold(&mut left, 97, 70);
    assert_eq!(ship(&left), (10, 60), "the ship stops at the left edge");

    let mut right = load(path);
    start(&mut right);
    hold(&mut right, 115, 100);
    assert_eq!(ship(&right), (124, 60), "the ship stops at the right edge");

    let mut up = load(path);
    start(&mut up);
    hold(&mut up, 113, 80);
    assert_eq!(ship(&up), (40, 20), "the ship stops at the top edge");

    let mut down = load(path);
    start(&mut down);
    hold(&mut down, 122, 40);
    assert_eq!(ship(&down), (40, 62), "the ship stops at the bottom edge");

    let mut shot = load(path);
    start(&mut shot);
    press(&mut shot, 13);
    assert_eq!(shot.vm().pc(), 0xf47);
    assert_eq!(bullets(&shot), 1);
    assert_eq!(shot.vm().memory()[0x202d], 1, "the first slot fires");
    assert_eq!(
        (shot.vm().memory()[0x2035], shot.vm().memory()[0x203d]),
        (46, 28),
        "the bullet leaves the ship's nose"
    );
    advance(&mut shot, 10);
    assert_eq!(shot.vm().memory()[0x203d], 26, "the bullet climbs");
    advance(&mut shot, 50);
    assert_eq!(bullets(&shot), 0, "the bullet leaves the screen");
    assert_eq!(shot.vm().memory()[0x202d], 0, "the slot is freed");

    let mut three = load(path);
    start(&mut three);
    for _ in 0..3 {
        press(&mut three, 13);
    }
    assert_eq!(three.vm().pc(), 0x116e);
    assert_eq!(score(&three), 1, "three shots down one enemy");
    assert_eq!(stage(&three), 1);
    assert_eq!(
        &three.vm().memory()[0x2011..0x2016],
        b"00001",
        "the hud shows the kill"
    );
    let mut five = load(path);
    start(&mut five);
    for _ in 0..5 {
        press(&mut five, 13);
    }
    assert_eq!(score(&five), 2, "five shots down two enemies");
    assert_eq!(five.vm().memory()[0x2015], b'2');

    let mut six = load(path);
    start(&mut six);
    for _ in 0..6 {
        press(&mut six, 13);
    }
    assert_eq!(score(&six), 3, "six shots down three enemies");
    assert_eq!(lives(&six), 0, "the last life is already gone");

    let mut over = load(path);
    start(&mut over);
    advance(&mut over, 400);
    assert_eq!(over.vm().pc(), 0x19a3, "the wave ends in game over");
    assert_eq!(lives(&over), 0);
    press(&mut over, 13);
    assert_eq!(over.vm().pc(), 0x19a5, "the first key is acknowledged");
    press(&mut over, 13);
    assert_eq!(over.vm().pc(), 0x180a, "the splash returns");
    press(&mut over, 13);
    assert_eq!(over.vm().pc(), 0x1843, "the speed menu returns");
    assert_eq!(over.vm().memory()[0x2000], 0, "the speed resets");
    assert!(
        over.files_mut().take_dirty_files().is_empty(),
        "the game writes no files"
    );
}

#[test]
#[ignore = "requires LAVAX_GAME_ROOT with the game corpus"]
fn allout_solves_the_first_board_and_advances_to_the_next() {
    let path = "LAVA1 类似十全十美/allout.lav";
    let cell = |e: &Emulator, index: usize| e.vm().memory()[0x2a54 + index * 2];
    let lit =
        |e: &Emulator| -> Vec<usize> { (0..25).filter(|index| cell(e, *index) == 1).collect() };
    let state = |e: &Emulator| {
        (
            e.vm().memory()[0x2a86],
            e.vm().memory()[0x2a88],
            e.vm().memory()[0x2a8a],
            e.vm().memory()[0x2a8c],
        )
    };
    let start = |e: &mut Emulator| {
        advance(e, 30);
        assert_eq!(e.vm().pc(), 0xd33, "the intro waits for enter");
        press(e, 13);
        assert_eq!(e.vm().pc(), 0xfee, "enter starts the first board");
    };
    let quit_with_yes = |e: &mut Emulator| -> Option<i32> {
        for frame in 0..90 {
            e.input_mut().set_keys([121]);
            if frame == 3 {
                e.input_mut().set_keys([]);
            }
            if let FrameStatus::Halted(code) = e.run_frame().unwrap().status {
                return Some(code);
            }
        }
        None
    };

    let mut intro = load(path);
    advance(&mut intro, 30);
    assert_eq!(intro.vm().pc(), 0xd33, "the intro waits for enter");
    assert_eq!(state(&intro), (0, 0, 0, 0), "the first board starts fresh");
    assert_eq!(
        lit(&intro),
        vec![2, 6, 8, 10, 14, 16, 18, 22],
        "board zero lights eight cells"
    );
    assert_eq!(
        intro.vm().memory()[0x2a95],
        b'1',
        "the intro names the level"
    );
    press(&mut intro, 27);
    assert_eq!(intro.vm().pc(), 0xd96, "escape prompts from the intro");
    press(&mut intro, 110);
    assert_eq!(intro.vm().pc(), 0xd33, "no returns to the intro");
    press(&mut intro, 13);
    assert_eq!(intro.vm().pc(), 0xfee, "enter starts the board");
    press(&mut intro, 23);
    assert_eq!(
        (state(&intro).0, state(&intro).1),
        (0, 0),
        "left is clamped"
    );
    press(&mut intro, 20);
    assert_eq!((state(&intro).0, state(&intro).1), (0, 0), "up is clamped");
    press(&mut intro, 22);
    assert_eq!((state(&intro).0, state(&intro).1), (0, 1));
    press(&mut intro, 21);
    assert_eq!((state(&intro).0, state(&intro).1), (1, 1));
    for _ in 0..3 {
        press(&mut intro, 22);
    }
    assert_eq!(state(&intro).1, 4, "right reaches the last column");
    press(&mut intro, 22);
    assert_eq!(state(&intro).1, 4, "right stays clamped");
    for _ in 0..3 {
        press(&mut intro, 21);
    }
    assert_eq!(state(&intro).0, 4, "down reaches the last row");
    press(&mut intro, 21);
    assert_eq!(state(&intro).0, 4, "down stays clamped");
    assert_eq!(state(&intro).2, 0, "steering spends no move");
    press(&mut intro, 27);
    assert_eq!(intro.vm().pc(), 0x11a1, "escape prompts during play");
    press(&mut intro, 110);
    assert_eq!(intro.vm().pc(), 0xfee, "no resumes the board");
    assert_eq!(state(&intro), (4, 4, 0, 0), "the board is unchanged");
    press(&mut intro, 27);
    assert_eq!(intro.vm().pc(), 0x11a1);
    assert_eq!(quit_with_yes(&mut intro), Some(0), "yes quits from play");

    let mut exit = load(path);
    advance(&mut exit, 30);
    press(&mut exit, 27);
    assert_eq!(exit.vm().pc(), 0xd96);
    assert_eq!(
        quit_with_yes(&mut exit),
        Some(0),
        "yes quits from the intro"
    );

    let mut toggle = load(path);
    start(&mut toggle);
    press(&mut toggle, 13);
    assert_eq!(toggle.vm().pc(), 0xfee, "one press does not win");
    assert_eq!(state(&toggle).2, 1, "a press spends one move");
    assert_eq!(
        lit(&toggle),
        vec![0, 1, 2, 5, 6, 8, 10, 14, 16, 18, 22],
        "the cursor cross flips its three in-bounds cells"
    );
    assert_eq!(
        toggle.vm().memory()[0x2a8f],
        b'1',
        "the hud counts the move"
    );

    let mut centre = load(path);
    start(&mut centre);
    for _ in 0..2 {
        press(&mut centre, 21);
    }
    for _ in 0..2 {
        press(&mut centre, 22);
    }
    press(&mut centre, 13);
    assert_eq!(
        lit(&centre),
        vec![2, 6, 7, 8, 10, 11, 12, 13, 14, 16, 17, 18, 22],
        "the centre press flips the full cross"
    );

    let mut solve = load(path);
    start(&mut solve);
    press(&mut solve, 13);
    for _ in 0..4 {
        press(&mut solve, 22);
    }
    press(&mut solve, 13);
    for _ in 0..2 {
        press(&mut solve, 21);
    }
    for _ in 0..2 {
        press(&mut solve, 23);
    }
    press(&mut solve, 13);
    for _ in 0..2 {
        press(&mut solve, 21);
    }
    for _ in 0..2 {
        press(&mut solve, 23);
    }
    press(&mut solve, 13);
    for _ in 0..4 {
        press(&mut solve, 22);
    }
    press(&mut solve, 13);
    assert_eq!(solve.vm().pc(), 0x11c8, "the win waits for a key");
    assert_eq!(state(&solve), (4, 4, 5, 0), "five moves solve the board");
    assert_eq!(
        lit(&solve),
        (0..25).collect::<Vec<_>>(),
        "all cells are lit"
    );
    assert_eq!(
        solve.vm().memory()[0x2a8f],
        b'5',
        "the hud shows five moves"
    );
    press(&mut solve, 13);
    assert_eq!(solve.vm().pc(), 0xfee, "the next board starts at once");
    assert_eq!(state(&solve), (0, 0, 0, 1), "the second board starts fresh");
    assert_eq!(
        lit(&solve),
        vec![0, 2, 4, 7, 10, 12, 14, 16, 18, 21, 23],
        "board one is loaded"
    );
    assert_eq!(solve.vm().memory()[0x2a8f], b'0', "the move counter resets");
    assert_eq!(solve.vm().memory()[0x2a95], b'2', "the hud names the level");
    assert!(
        solve.files_mut().take_dirty_files().is_empty(),
        "the game writes no files"
    );
}

#[test]
#[ignore = "requires LAVAX_GAME_ROOT with the game corpus"]
fn sprite_island_five_runs_the_menu_settings_saves_and_first_level() {
    struct Build {
        path: &'static str,
        splash: usize,
        credits: usize,
        menu: usize,
        intro: usize,
        play: usize,
        caught: usize,
        resume: usize,
        settings: usize,
        list: usize,
        empty: usize,
        manual: [usize; 5],
        about: usize,
        halt: usize,
        halted_pc: usize,
    }
    const BUILDS: [Build; 2] = [
        Build {
            path: "LAVA1 精灵岛5-勇者之旅/精灵岛5.lav",
            splash: 0x30af,
            credits: 0x30c1,
            menu: 0x2ddb,
            intro: 0x2fab,
            play: 0x3886,
            caught: 0x3652,
            resume: 0x38cc,
            settings: 0x2086,
            list: 0x2805,
            empty: 0x2341,
            manual: [0x2969, 0x2a06, 0x2aa0, 0x2b36, 0x2bca],
            about: 0x2c6d,
            halt: 0x2c6d,
            halted_pc: 0x2f6f,
        },
        Build {
            path: "LAVA1 精灵岛5-勇者之旅/精灵岛5.pac",
            splash: 0x30ca,
            credits: 0x30dc,
            menu: 0x2df6,
            intro: 0x2fc6,
            play: 0x38a1,
            caught: 0x366d,
            resume: 0x38e7,
            settings: 0x20a1,
            list: 0x2820,
            empty: 0x235c,
            manual: [0x2984, 0x2a21, 0x2abb, 0x2b51, 0x2be5],
            about: 0x2c88,
            halt: 0x2c88,
            halted_pc: 0x2f8a,
        },
    ];
    let word = |emulator: &Emulator, index: usize| {
        let memory = emulator.vm().memory();
        i16::from_le_bytes([memory[0x2096 + index * 2], memory[0x2096 + index * 2 + 1]])
    };
    for build in BUILDS {
        let path = build.path;

        let mut settings = load(path);
        advance(&mut settings, 30);
        assert_eq!(settings.vm().pc(), build.splash, "the splash waits");
        press(&mut settings, 13);
        assert_eq!(settings.vm().pc(), build.credits, "the second splash waits");
        press(&mut settings, 13);
        assert_eq!(settings.vm().pc(), build.menu, "the menu waits");
        press(&mut settings, 22);
        press(&mut settings, 13);
        assert_eq!(
            settings.vm().pc(),
            build.settings,
            "the settings screen waits"
        );
        assert_eq!(
            &settings.vm().memory()[0x2086..0x2096],
            &[0, 0, 0, 0, 0, 0, 10, 0, 5, 0, 1, 0, 0, 0, 13, 0],
            "the settings start from their defaults"
        );
        press(&mut settings, 20);
        press(&mut settings, 22);
        assert_eq!(
            settings.vm().memory()[0x2090],
            2,
            "up wraps to the difficulty row"
        );
        press(&mut settings, 21);
        press(&mut settings, 23);
        assert_eq!(
            &settings.vm().memory()[0x2088..0x208c],
            &[3, 0, 3, 0],
            "left wraps the enemy choice and mirrors it"
        );
        press(&mut settings, 21);
        press(&mut settings, 22);
        press(&mut settings, 22);
        assert_eq!(
            settings.vm().memory()[0x208c],
            0,
            "right stops at the fastest speed"
        );
        press(&mut settings, 23);
        assert_eq!(settings.vm().memory()[0x208c], 20);
        press(&mut settings, 13);
        assert_eq!(settings.vm().pc(), build.menu, "enter stores the settings");
        let files = settings.files_mut().take_dirty_files();
        let config = files
            .iter()
            .find(|(path, _)| path == "/lavadata/sprcfig.dat")
            .expect("the settings must be saved")
            .1
            .as_slice();
        assert_eq!(config, [3, 20, 2], "the file stores the three choices");

        let mut seeded = load(path);
        seeded
            .files_mut()
            .import_file("/LavaData/SprCfig.dat", vec![2, 60, 2]);
        advance(&mut seeded, 30);
        press(&mut seeded, 13);
        press(&mut seeded, 13);
        press(&mut seeded, 22);
        press(&mut seeded, 13);
        assert_eq!(seeded.vm().pc(), build.settings);
        assert_eq!(
            &seeded.vm().memory()[0x2088..0x208c],
            &[2, 0, 2, 0],
            "the stored enemy choice loads at startup"
        );
        assert_eq!(seeded.vm().memory()[0x208c], 60, "the stored speed loads");
        assert_eq!(
            seeded.vm().memory()[0x2090],
            2,
            "the stored difficulty loads"
        );

        let mut level = load(path);
        advance(&mut level, 30);
        assert_eq!(level.vm().pc(), build.splash);
        press(&mut level, 13);
        press(&mut level, 13);
        press(&mut level, 13);
        assert_eq!(level.vm().pc(), build.intro, "enter opens the level intro");
        press(&mut level, 13);
        assert_eq!(level.vm().pc(), build.play, "enter starts the first level");
        assert_eq!(
            [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17]
                .map(|index| word(&level, index)),
            [
                -1, -1, -1, -1, -1, 9, -1, 9, -1, -1, -1, 1, 1, 1, 1, -1, 1, -1
            ],
            "the level map loads into the tile grid"
        );
        assert_eq!(&level.vm().memory()[0x2366..0x236c], &[0, 0, 0, 0, 4, 0]);
        advance(&mut level, 400);
        assert_eq!(
            level.vm().pc(),
            build.caught,
            "idling gets the player caught"
        );
        assert_eq!(level.vm().memory()[0x208e], 4, "one life is lost");
        assert_eq!(&level.vm().memory()[0x2366..0x236a], &[0, 0, 0, 0]);
        press(&mut level, 13);
        assert_eq!(level.vm().pc(), build.play, "enter restarts the level");
        hold(&mut level, 22, 40);
        assert_eq!(
            &level.vm().memory()[0x2366..0x236c],
            &[48, 0, 0, 0, 3, 0],
            "holding right moves three tiles"
        );
        hold(&mut level, 112, 10);
        assert_eq!(level.vm().pc(), 0x169c, "p pauses the level");
        assert_eq!(
            &level.vm().memory()[0x2366..0x236a],
            &[48, 0, 0, 0],
            "pausing keeps the position"
        );
        press(&mut level, 13);
        assert_eq!(level.vm().pc(), build.resume, "any key resumes the level");
        hold(&mut level, 115, 10);
        assert_eq!(level.vm().pc(), build.list, "s opens the save list");
        press(&mut level, 13);
        assert_eq!(level.vm().pc(), build.resume, "enter returns to the level");
        let files = level.files_mut().take_dirty_files();
        let save = files
            .iter()
            .find(|(path, _)| path == "/lavadata/sprsav1.dat")
            .expect("the first slot must be written")
            .1
            .as_slice();
        assert_eq!(save.len(), 29, "the slot record is twenty-nine bytes");
        assert_eq!(
            [save[0], save[6], save[7], save[8], save[10], save[11]],
            [0, 4, 48, 0, 3, 3],
            "the record stores score, lives, position and facing"
        );

        press(&mut level, 27);
        assert_eq!(level.vm().pc(), build.menu, "escape leaves the level");
        assert_eq!(
            level.vm().memory()[0x208e],
            5,
            "escaping restores the lives"
        );
        press(&mut level, 21);
        press(&mut level, 13);
        assert_eq!(level.vm().pc(), build.list, "the journey list opens");
        press(&mut level, 13);
        assert_eq!(level.vm().pc(), build.play, "enter loads the first slot");
        assert_eq!(
            &level.vm().memory()[0x2366..0x236a],
            &[48, 0, 0, 0],
            "loading restores the saved position"
        );
        assert_eq!(level.vm().memory()[0x208e], 4, "loading restores the lives");

        press(&mut level, 27);
        press(&mut level, 21);
        press(&mut level, 13);
        press(&mut level, 22);
        press(&mut level, 13);
        assert_eq!(
            level.vm().pc(),
            build.empty,
            "an empty slot reports missing data"
        );
        assert_eq!(
            level.vm().memory()[0x208e],
            5,
            "the failed load changes nothing"
        );
        press(&mut level, 13);
        assert_eq!(
            level.vm().pc(),
            build.menu,
            "the report returns to the menu"
        );

        press(&mut level, 22);
        press(&mut level, 13);
        assert_eq!(
            level.vm().pc(),
            build.manual[0],
            "the manual opens at its first page"
        );
        for page in &build.manual[1..] {
            press(&mut level, 13);
            assert_eq!(level.vm().pc(), *page, "each key turns one page");
        }
        press(&mut level, 13);
        assert_eq!(level.vm().pc(), build.menu, "the last page returns");

        press(&mut level, 22);
        press(&mut level, 13);
        assert_eq!(level.vm().pc(), build.about, "about shows the credits");
        press(&mut level, 13);
        assert_eq!(level.vm().pc(), build.menu, "the credits return");

        press(&mut level, 22);
        press(&mut level, 13);
        assert_eq!(level.vm().pc(), build.halt, "quitting shows the credits");
        level.input_mut().set_keys([13]);
        let mut halted = None;
        for _ in 0..10 {
            if let FrameStatus::Halted(code) = level.run_frame().unwrap().status {
                halted = Some(code);
                break;
            }
        }
        level.input_mut().set_keys([]);
        assert_eq!(halted, Some(0), "the credits exit the program");
        assert_eq!(
            level.vm().pc(),
            build.halted_pc,
            "the program stops cleanly"
        );
    }
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn absolute_fall_scrolls_help_saves_and_solves_the_first_level() {
    let path = "LAVA1 绝对下落/分步安装/绝对下落.lav";
    let tap = |emulator: &mut Emulator, key: u8| {
        emulator.input_mut().set_keys([key]);
        let mut status = emulator.run_frame().unwrap().status;
        emulator.input_mut().set_keys([]);
        let mut frames = 1;
        while !matches!(
            status,
            FrameStatus::WaitingForInput | FrameStatus::Halted(_)
        ) {
            status = emulator.run_frame().unwrap().status;
            frames += 1;
            assert!(frames < 600, "the game must return to a key wait");
        }
    };
    let cell =
        |emulator: &Emulator, x: usize, y: usize| emulator.vm().memory()[0x2048 + x * 11 + y];
    let help_offset = |emulator: &Emulator| {
        let bp = emulator.vm().local_bp() as usize;
        emulator.vm().memory()[bp + 7]
    };

    let mut menu = load(path);
    advance(&mut menu, 30);
    assert_eq!(menu.vm().pc(), 0x1ea4, "the splash shows first");
    advance(&mut menu, 270);
    assert_eq!(menu.vm().pc(), 0x1d31, "the menu waits after the splash");
    {
        let files = menu.files().files();
        let size = |name: &str| {
            files
                .iter()
                .find(|info| info.path == name)
                .map(|info| info.size)
        };
        assert_eq!(
            size("/xl_map.dat"),
            Some(1950),
            "the level map is available"
        );
        assert_eq!(
            size("/xl_pic.dat"),
            Some(9600),
            "the graphics are available"
        );
    }
    assert!(
        menu.files_mut().take_dirty_files().is_empty(),
        "booting does not write files"
    );

    press(&mut menu, 20);
    press(&mut menu, 23);
    assert_eq!(
        menu.vm().memory()[0x5566],
        0,
        "up and left stop at the first item"
    );
    press(&mut menu, 21);
    assert_eq!(menu.vm().memory()[0x5566], 1, "down selects continue");
    press(&mut menu, 22);
    assert_eq!(menu.vm().memory()[0x5566], 2, "right selects help");
    press(&mut menu, 21);
    press(&mut menu, 21);
    assert_eq!(menu.vm().memory()[0x5566], 3, "down stops at quit");
    press(&mut menu, 23);
    assert_eq!(menu.vm().memory()[0x5566], 2, "left backs up");

    press(&mut menu, 13);
    assert_eq!(menu.vm().pc(), 0x768, "enter opens the help");
    press(&mut menu, 20);
    press(&mut menu, 20);
    assert_eq!(help_offset(&menu), 0, "up stops at the first line");
    press(&mut menu, 21);
    press(&mut menu, 21);
    assert_eq!(help_offset(&menu), 2, "down scrolls the text");
    press(&mut menu, 27);
    assert_eq!(menu.vm().pc(), 0x1d31, "escape returns to the menu");
    assert_eq!(
        menu.vm().memory()[0x5566],
        2,
        "the menu keeps its selection"
    );

    press(&mut menu, 21);
    assert_eq!(menu.vm().memory()[0x5566], 3, "quit is selected");
    press(&mut menu, 13);
    assert_eq!(menu.vm().pc(), 0x914, "quitting shows the farewell");
    menu.input_mut().set_keys([13]);
    let mut halted = None;
    for _ in 0..10 {
        if let FrameStatus::Halted(code) = menu.run_frame().unwrap().status {
            halted = Some(code);
            break;
        }
    }
    menu.input_mut().set_keys([]);
    assert_eq!(halted, Some(1), "the farewell exits the program");
    assert_eq!(menu.vm().pc(), 0x918, "the program stops cleanly");

    let mut game = load(path);
    advance(&mut game, 300);
    assert_eq!(game.vm().pc(), 0x1d31, "the menu is ready");
    tap(&mut game, 13);
    assert_eq!(game.vm().pc(), 0x149d, "enter starts the first level");
    assert_eq!(
        &game.vm().memory()[0x2002..0x2004],
        &[1, 1],
        "the cursor starts at the corner"
    );
    assert_eq!(game.vm().memory()[0x2008], 0, "the first level is loaded");
    assert_eq!(game.vm().memory()[0x2009], 0, "no later level is known");
    assert_eq!(game.vm().memory()[0x200a], 4, "four stones remain");
    assert_eq!(game.vm().memory()[0x2014], 5, "the player has five lives");
    assert_eq!(cell(&game, 5, 6), 3, "the first three stone loads");
    assert_eq!(cell(&game, 7, 6), 3, "the matching three stone loads");
    assert_eq!(cell(&game, 9, 6), 4, "the first four stone loads");
    assert_eq!(cell(&game, 11, 6), 4, "the matching four stone loads");
    assert_eq!(cell(&game, 3, 4), 2, "the top wall loads");
    assert_eq!(cell(&game, 13, 7), 2, "the bottom wall loads");
    assert_eq!(cell(&game, 1, 1), 1, "the start cell is empty");

    tap(&mut game, 23);
    assert_eq!(
        game.vm().memory()[0x2002],
        15,
        "left wraps to the last column"
    );
    tap(&mut game, 20);
    assert_eq!(game.vm().memory()[0x2003], 10, "up wraps to the last row");
    tap(&mut game, 22);
    assert_eq!(
        game.vm().memory()[0x2002],
        1,
        "right wraps to the first column"
    );
    tap(&mut game, 21);
    assert_eq!(game.vm().memory()[0x2003], 1, "down wraps to the first row");
    tap(&mut game, 13);
    assert_eq!(
        game.vm().pc(),
        0x149d,
        "enter on an empty cell does nothing"
    );
    assert_eq!(
        game.vm().memory()[0x200a],
        4,
        "the stone count is unchanged"
    );

    for _ in 0..5 {
        tap(&mut game, 21);
    }
    for _ in 0..4 {
        tap(&mut game, 22);
    }
    assert_eq!(
        &game.vm().memory()[0x2002..0x2004],
        &[5, 6],
        "the cursor reaches the first pair"
    );
    tap(&mut game, 13);
    assert_eq!(game.vm().pc(), 0x167f, "enter picks up a stone");
    assert_eq!(
        &game.vm().memory()[0x2004..0x2006],
        &[5, 6],
        "the stone is held"
    );
    tap(&mut game, 22);
    assert_eq!(game.vm().pc(), 0x149d, "moving the stone resolves the pair");
    assert_eq!(game.vm().memory()[0x200a], 2, "the three pair vanishes");
    assert_eq!(
        &game.vm().memory()[0x2002..0x2004],
        &[6, 6],
        "the cursor follows the stone"
    );
    assert_eq!(cell(&game, 5, 6), 1, "the source cell empties");
    assert_eq!(cell(&game, 6, 6), 1, "the target cell empties");
    assert_eq!(cell(&game, 7, 6), 1, "the matched stone empties");

    for _ in 0..3 {
        tap(&mut game, 22);
    }
    assert_eq!(
        &game.vm().memory()[0x2002..0x2004],
        &[9, 6],
        "the cursor reaches the second pair"
    );
    tap(&mut game, 13);
    assert_eq!(game.vm().pc(), 0x167f, "enter picks up the second stone");
    assert_eq!(
        &game.vm().memory()[0x2004..0x2006],
        &[9, 6],
        "the stone is held"
    );
    tap(&mut game, 22);
    assert_eq!(game.vm().pc(), 0x145d, "the second pair wins the level");
    assert_eq!(game.vm().memory()[0x200a], 0, "no stones remain");
    assert_eq!(cell(&game, 9, 6), 1, "the last matched stones empty");
    assert_eq!(cell(&game, 11, 6), 1, "the last matched stones empty");
    assert!(
        game.files_mut().take_dirty_files().is_empty(),
        "winning alone does not write files"
    );

    tap(&mut game, 13);
    assert_eq!(game.vm().pc(), 0x149d, "a key loads the second level");
    assert_eq!(
        &game.vm().memory()[0x2008..0x200a],
        &[1, 1],
        "the second level is reached"
    );
    assert_eq!(
        game.vm().memory()[0x200a],
        7,
        "the second level stones load"
    );
    assert_eq!(
        &game.vm().memory()[0x2002..0x2004],
        &[1, 1],
        "the cursor resets"
    );
    let saved = game.files_mut().take_dirty_files();
    assert_eq!(saved.len(), 1, "only the progress file changes");
    assert_eq!(
        saved[0].0, "/lavadata/xl_user.dat",
        "the progress file is named"
    );
    assert_eq!(saved[0].1, [1, 1], "the save records the furthest level");

    tap(&mut game, 21);
    assert_eq!(game.vm().memory()[0x2003], 2, "the cursor moves");
    tap(&mut game, 28);
    assert_eq!(game.vm().pc(), 0x149d, "F1 restarts the level");
    assert_eq!(
        &game.vm().memory()[0x2002..0x2004],
        &[1, 1],
        "F1 resets the cursor"
    );
    assert_eq!(game.vm().memory()[0x200a], 7, "F1 restores the stones");

    tap(&mut game, 21);
    assert_eq!(game.vm().memory()[0x2003], 2, "the cursor moves again");
    tap(&mut game, 25);
    assert_eq!(game.vm().pc(), 0x768, "F5 opens the help in game");
    assert_eq!(game.vm().memory()[0x2003], 2, "the level is paused");
    press(&mut game, 21);
    assert_eq!(help_offset(&game), 1, "the in-game help scrolls");
    press(&mut game, 27);
    assert_eq!(game.vm().pc(), 0x149d, "escape resumes the level");
    assert_eq!(
        &game.vm().memory()[0x2002..0x2004],
        &[1, 2],
        "the cursor is kept"
    );
    assert_eq!(game.vm().memory()[0x200a], 7, "the stones are kept");

    tap(&mut game, 30);
    assert_eq!(game.vm().pc(), 0xf80, "F3 opens the level select");
    assert_eq!(
        game.vm().memory()[0x2011],
        2,
        "the current level is selected"
    );
    press(&mut game, 20);
    assert_eq!(game.vm().memory()[0x2011], 1, "up moves to the first level");
    press(&mut game, 20);
    assert_eq!(
        game.vm().memory()[0x2011],
        2,
        "up wraps to the furthest level"
    );
    press(&mut game, 21);
    assert_eq!(
        game.vm().memory()[0x2011],
        2,
        "down stops at the furthest level"
    );
    press(&mut game, 20);
    assert_eq!(
        game.vm().memory()[0x2011],
        1,
        "up returns to the first level"
    );
    tap(&mut game, 13);
    assert_eq!(game.vm().pc(), 0x149d, "enter starts the chosen level");
    assert_eq!(game.vm().memory()[0x2008], 0, "the first level reloads");
    assert_eq!(game.vm().memory()[0x200a], 4, "its stones reload");

    tap(&mut game, 30);
    assert_eq!(game.vm().pc(), 0xf80, "F3 reopens the level select");
    assert_eq!(
        game.vm().memory()[0x2011],
        1,
        "the current level is selected"
    );
    press(&mut game, 21);
    assert_eq!(
        game.vm().memory()[0x2011],
        2,
        "down moves to the furthest level"
    );
    tap(&mut game, 13);
    assert_eq!(game.vm().pc(), 0x149d, "enter returns to the second level");
    assert_eq!(game.vm().memory()[0x2008], 1, "the second level reloads");
    assert_eq!(game.vm().memory()[0x200a], 7, "its stones reload");

    press(&mut game, 27);
    assert_eq!(
        game.vm().pc(),
        0x1d31,
        "escape leaves the level for the menu"
    );
    assert_eq!(game.vm().memory()[0x5566], 0, "the menu starts over");
    let saved = game.files_mut().take_dirty_files();
    assert_eq!(saved.len(), 1, "leaving writes the progress file");
    assert_eq!(saved[0].1, [1, 1], "the furthest level stays saved");

    let mut resumed = load(path);
    assert!(
        resumed
            .files_mut()
            .import_file("/LavaData/xl_user.dat", vec![1, 1]),
        "the save seed is imported"
    );
    advance(&mut resumed, 300);
    assert_eq!(resumed.vm().pc(), 0x1d31, "the menu waits");
    press(&mut resumed, 21);
    assert_eq!(resumed.vm().memory()[0x5566], 1, "continue is selected");
    tap(&mut resumed, 13);
    assert_eq!(resumed.vm().pc(), 0x149d, "continue loads the saved level");
    assert_eq!(
        &resumed.vm().memory()[0x2008..0x200a],
        &[1, 1],
        "the save is read"
    );
    assert_eq!(
        resumed.vm().memory()[0x200a],
        7,
        "the saved level stones load"
    );
    assert_eq!(
        &resumed.vm().memory()[0x2002..0x2004],
        &[1, 1],
        "the cursor resets"
    );

    let mut missing = load(path);
    advance(&mut missing, 300);
    press(&mut missing, 21);
    tap(&mut missing, 13);
    assert_eq!(
        missing.vm().pc(),
        0x149d,
        "continue without a save starts over"
    );
    assert_eq!(missing.vm().memory()[0x2008], 0, "the first level loads");
    let created = missing.files_mut().take_dirty_files();
    assert_eq!(created.len(), 1, "a missing save is created");
    assert_eq!(
        created[0].0, "/lavadata/xl_user.dat",
        "the progress file is named"
    );
    assert_eq!(
        created[0].1,
        [0, 0],
        "the new save starts at the first level"
    );
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn wing_sword_pages_the_prologue_and_walks_the_opening_map() {
    let builds = [
        "LAVA1 翼剑天殇/翼剑天殇.pac",
        "LAVA1 翼剑天殇/非pac/Wing.lav",
    ];
    let mut all_frames = Vec::new();
    for path in builds {
        let mut game = load(path);
        advance(&mut game, 300);
        assert_eq!(game.vm().pc(), 0x286, "the title waits");
        assert_eq!(
            &game.vm().memory()[0x43ad..0x43af],
            &[0, 0],
            "no player is drawn on the title"
        );
        assert!(
            game.files_mut().take_dirty_files().is_empty(),
            "the title writes nothing"
        );
        let mut frames = vec![game.display().indexed_frame().to_vec()];

        press(&mut game, 13);
        assert_ne!(
            game.display().indexed_frame(),
            frames[0].as_slice(),
            "the first key opens a text page"
        );
        assert_ne!(
            &game.vm().memory()[0x1c0c..0x1c12],
            &[0, 0, 0, 0, 0, 0],
            "the text page names the data file"
        );
        frames.push(game.display().indexed_frame().to_vec());

        press(&mut game, 13);
        assert_eq!(
            &game.vm().memory()[0x43ad..0x43af],
            &[5, 3],
            "the opening map loads with the hero in place"
        );
        frames.push(game.display().indexed_frame().to_vec());

        for _ in 2..20 {
            press(&mut game, 13);
        }
        assert_eq!(
            &game.vm().memory()[0x43ad..0x43af],
            &[5, 3],
            "the dialogue leaves the hero at the start"
        );
        assert_eq!(game.vm().memory()[0x43b5], 1, "the hero faces down");
        assert_eq!(game.vm().memory()[0x5b38], 1, "the world counter resets");
        assert_ne!(
            game.display().indexed_frame(),
            frames[1].as_slice(),
            "the dialogue box is gone once control starts"
        );
        frames.push(game.display().indexed_frame().to_vec());

        press(&mut game, 22);
        assert_eq!(
            &game.vm().memory()[0x43ad..0x43af],
            &[6, 3],
            "right steps one column"
        );
        assert_eq!(game.vm().memory()[0x43b5], 2, "right faces right");
        frames.push(game.display().indexed_frame().to_vec());

        press(&mut game, 22);
        assert_eq!(
            &game.vm().memory()[0x43ad..0x43af],
            &[7, 3],
            "right steps again"
        );
        frames.push(game.display().indexed_frame().to_vec());

        press(&mut game, 21);
        assert_eq!(
            &game.vm().memory()[0x43ad..0x43af],
            &[7, 3],
            "a wall blocks the downward step"
        );
        assert_eq!(
            game.vm().memory()[0x43b5],
            1,
            "the blocked step still faces down"
        );
        frames.push(game.display().indexed_frame().to_vec());

        press(&mut game, 20);
        assert_eq!(
            &game.vm().memory()[0x43ad..0x43af],
            &[7, 2],
            "up steps one row"
        );
        assert_eq!(game.vm().memory()[0x43b5], 0, "up faces up");
        frames.push(game.display().indexed_frame().to_vec());

        press(&mut game, 20);
        assert_eq!(
            &game.vm().memory()[0x43ad..0x43af],
            &[7, 1],
            "up steps again"
        );
        frames.push(game.display().indexed_frame().to_vec());

        press(&mut game, 23);
        assert_eq!(
            &game.vm().memory()[0x43ad..0x43af],
            &[6, 1],
            "left steps one column"
        );
        assert_eq!(game.vm().memory()[0x43b5], 3, "left faces left");
        frames.push(game.display().indexed_frame().to_vec());

        press(&mut game, 23);
        assert_eq!(
            &game.vm().memory()[0x43ad..0x43af],
            &[5, 1],
            "left steps again"
        );
        frames.push(game.display().indexed_frame().to_vec());

        press(&mut game, 21);
        assert_eq!(
            &game.vm().memory()[0x43ad..0x43af],
            &[5, 2],
            "down steps one row"
        );
        assert_eq!(game.vm().memory()[0x43b5], 1, "down faces down");
        frames.push(game.display().indexed_frame().to_vec());

        press(&mut game, 21);
        assert_eq!(
            &game.vm().memory()[0x43ad..0x43af],
            &[5, 3],
            "down returns to the start"
        );
        frames.push(game.display().indexed_frame().to_vec());

        assert!(
            game.files_mut().take_dirty_files().is_empty(),
            "the opening map writes nothing"
        );
        all_frames.push(frames);
    }

    assert_eq!(
        all_frames[0].len(),
        all_frames[1].len(),
        "both builds take the same steps"
    );
    for (index, (packaged, loose)) in all_frames[0].iter().zip(&all_frames[1]).enumerate() {
        assert_eq!(
            packaged, loose,
            "both builds render the same frame at step {index}"
        );
    }
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn flower_spirit_collects_flowers_and_restores_a_save() {
    let builds = [
        "LAVA1 花之精灵2.0/flower2.0.pac",
        "LAVA1 花之精灵2.0/非pac/花之精灵.lav",
    ];
    let mut all_frames = Vec::new();
    for path in builds {
        let mut game = load(path);
        let score = |game: &Emulator| {
            u16::from_le_bytes(game.vm().memory()[0x380d..0x380f].try_into().unwrap())
        };
        let tile = |game: &Emulator, row: usize, column: usize| {
            game.vm().memory()[0x3000 + row * 17 + column]
        };
        advance(&mut game, 300);
        assert_eq!(game.vm().pc(), 0x23bc, "the title waits for a key");
        assert!(
            game.files_mut().take_dirty_files().is_empty(),
            "the title writes nothing"
        );
        let mut frames = vec![game.display().indexed_frame().to_vec()];

        press(&mut game, 13);
        assert_eq!(game.vm().pc(), 0x24c6, "the first key opens the menu");
        let menu_bp = game.vm().local_bp() as usize;
        assert_eq!(
            game.vm().memory()[menu_bp + 0x647],
            0,
            "the menu starts on the first item"
        );
        frames.push(game.display().indexed_frame().to_vec());

        press(&mut game, 21);
        assert_eq!(
            game.vm().memory()[menu_bp + 0x647],
            1,
            "down advances the menu selection"
        );
        press(&mut game, 20);
        assert_eq!(
            game.vm().memory()[menu_bp + 0x647],
            0,
            "up returns the menu selection"
        );
        press(&mut game, 20);
        assert_eq!(
            game.vm().memory()[menu_bp + 0x647],
            0,
            "the menu clamps at the first item"
        );
        for _ in 0..3 {
            press(&mut game, 21);
        }
        assert_eq!(
            game.vm().memory()[menu_bp + 0x647],
            3,
            "three downs reach the last item"
        );
        press(&mut game, 21);
        assert_eq!(
            game.vm().memory()[menu_bp + 0x647],
            3,
            "the menu clamps at the last item"
        );

        press(&mut game, 20);
        press(&mut game, 13);
        assert_eq!(game.vm().pc(), 0x25e1, "the help page waits for a key");
        assert_eq!(
            &game.vm().memory()[0x1c34..0x1c3d],
            &[0xb9, 0xa6, 0x20, 0xc4, 0xdc, 0x20, 0xbc, 0xfc, 0x3a],
            "the help page lists the function keys"
        );
        frames.push(game.display().indexed_frame().to_vec());
        press(&mut game, 13);
        assert_eq!(game.vm().pc(), 0x24c6, "the help page returns to the menu");
        assert_eq!(
            game.vm().memory()[menu_bp + 0x647],
            0,
            "the menu selection resets after a page"
        );

        for _ in 0..3 {
            press(&mut game, 21);
        }
        press(&mut game, 13);
        assert_eq!(
            game.vm().pc(),
            0x2b46,
            "the last item opens the leaderboard"
        );
        assert!(
            game.files_mut().take_dirty_files().is_empty(),
            "a missing high score list writes nothing"
        );
        frames.push(game.display().indexed_frame().to_vec());
        press(&mut game, 13);
        assert_eq!(
            game.vm().pc(),
            0x24c6,
            "the leaderboard returns to the menu"
        );

        press(&mut game, 13);
        assert_eq!(
            game.vm().pc(),
            0x2f92,
            "a new game redraws the map before play"
        );
        press(&mut game, 13);
        assert_eq!(game.vm().pc(), 0x311d, "the map waits for movement");
        assert_eq!(
            &game.vm().memory()[0x200a..0x200f],
            &[10, 20, 0, 0, 0],
            "the new game starts at the top left of the clearing"
        );
        assert_eq!(game.vm().memory()[0x200f], 1, "the start is safe");
        assert_eq!(score(&game), 0, "the new game starts without flowers");
        assert_eq!(tile(&game, 2, 2), 0, "the hero stands on an empty tile");
        frames.push(game.display().indexed_frame().to_vec());

        for _ in 0..3 {
            press(&mut game, 22);
        }
        assert_eq!(game.vm().memory()[0x200a], 40, "right walks one tile");
        assert_eq!(score(&game), 1, "the first flower is picked up");
        assert_eq!(tile(&game, 2, 5), 0, "the first flower tile is cleared");
        for _ in 0..7 {
            press(&mut game, 22);
        }
        assert_eq!(game.vm().memory()[0x200a], 110, "the row ends by the wall");
        assert_eq!(
            score(&game),
            3,
            "all three flowers on the row are picked up"
        );
        assert_eq!(tile(&game, 2, 6), 0, "the second flower tile is cleared");
        assert_eq!(tile(&game, 2, 12), 0, "the third flower tile is cleared");
        frames.push(game.display().indexed_frame().to_vec());

        press(&mut game, 115);
        assert_eq!(game.vm().pc(), 0x266d, "the save picker waits for a slot");
        let picker_bp = game.vm().local_bp() as usize;
        assert_eq!(
            game.vm().memory()[picker_bp + 0x0a],
            0,
            "the picker starts on the first slot"
        );
        press(&mut game, 20);
        assert_eq!(
            game.vm().memory()[picker_bp + 0x0a],
            0,
            "up clamps the first slot"
        );
        press(&mut game, 21);
        assert_eq!(
            game.vm().memory()[picker_bp + 0x0a],
            1,
            "down moves to the second slot"
        );
        press(&mut game, 20);
        press(&mut game, 13);
        assert_eq!(game.vm().pc(), 0x311d, "saving returns to the map");
        assert_eq!(game.vm().memory()[0x200a], 110, "saving keeps the position");
        let saved = game.files_mut().take_dirty_files();
        assert_eq!(saved.len(), 1, "saving writes one file");
        assert_eq!(
            saved[0].0, "/lavadata/flowersave0.dat",
            "the first slot has its own file"
        );
        assert_eq!(saved[0].1, [0, 3], "the save stores progress and flowers");
        frames.push(game.display().indexed_frame().to_vec());

        press(&mut game, 27);
        assert_eq!(game.vm().pc(), 0x24c6, "escape returns to the menu");
        assert_eq!(
            game.vm().memory()[menu_bp + 0x647],
            0,
            "the menu restarts on the first item"
        );
        press(&mut game, 21);
        press(&mut game, 13);
        assert_eq!(game.vm().pc(), 0x266d, "the load item opens the picker");
        press(&mut game, 13);
        assert_eq!(
            game.vm().pc(),
            0x2f92,
            "loading redraws the map before play"
        );
        assert_eq!(score(&game), 3, "loading restores the flower count");
        assert_eq!(
            &game.vm().memory()[0x200a..0x200d],
            &[10, 20, 0],
            "loading restarts the hero at the clearing"
        );
        assert_eq!(tile(&game, 2, 5), 11, "loading restores the first flower");
        assert_eq!(tile(&game, 2, 6), 11, "loading restores the second flower");
        assert_eq!(tile(&game, 2, 12), 11, "loading restores the third flower");
        frames.push(game.display().indexed_frame().to_vec());
        press(&mut game, 13);
        assert_eq!(game.vm().pc(), 0x311d, "the loaded map starts");

        press(&mut game, 21);
        press(&mut game, 21);
        press(&mut game, 22);
        assert_eq!(
            game.vm().pc(),
            0x3f5,
            "walking into the pit shows the death prompt"
        );
        assert_eq!(game.vm().memory()[0x200b], 40, "the death stays in place");
        assert_eq!(
            game.vm().memory()[0x200f],
            0,
            "the pit clears the safe flag"
        );
        assert_eq!(
            score(&game),
            0xfff9,
            "the pit costs ten flowers from the loaded score"
        );
        frames.push(game.display().indexed_frame().to_vec());
        press(&mut game, 13);
        assert_eq!(game.vm().pc(), 0x24c6, "losing a life returns to the menu");
        assert_eq!(score(&game), 0, "the menu resets the flower count");
        frames.push(game.display().indexed_frame().to_vec());

        press(&mut game, 21);
        press(&mut game, 13);
        assert_eq!(
            game.vm().pc(),
            0x266d,
            "the empty slot check opens the picker"
        );
        press(&mut game, 21);
        press(&mut game, 21);
        press(&mut game, 13);
        assert_eq!(game.vm().pc(), 0x24c6, "an empty slot returns to the menu");
        assert_eq!(
            game.vm().memory()[menu_bp + 0x647],
            0,
            "the menu restarts after an empty slot"
        );
        frames.push(game.display().indexed_frame().to_vec());

        game.input_mut().set_keys([27]);
        let mut exit = None;
        for _ in 0..3 {
            if let FrameStatus::Halted(code) = game.run_frame().unwrap().status {
                exit = Some(code);
                break;
            }
        }
        game.input_mut().set_keys([]);
        assert_eq!(exit, Some(0), "escape quits the menu");
        assert_eq!(game.vm().pc(), 0x24d7, "the menu exits through its halt");
        all_frames.push(frames);
    }

    assert_eq!(
        all_frames[0].len(),
        all_frames[1].len(),
        "both builds take the same steps"
    );
    for (index, (packaged, loose)) in all_frames[0].iter().zip(&all_frames[1]).enumerate() {
        assert_eq!(
            packaged, loose,
            "both builds render the same frame at step {index}"
        );
    }
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn hero2_perfect_edition_mines_recruits_and_fights() {
    let mut game = load("LAVA1 英雄无敌2完美版/《英雄无敌2》完美版.lav");
    let word = |game: &Emulator, address: usize| {
        u16::from_le_bytes(game.vm().memory()[address..address + 2].try_into().unwrap())
    };
    let cursor = |game: &Emulator| (word(game, 0x2011), word(game, 0x2013));
    let gold = |game: &Emulator| word(game, 0x58cf);
    let action = |game: &Emulator| word(game, 0x58eb);

    advance(&mut game, 300);
    assert_eq!(game.vm().pc(), 0x114b2, "the title waits for a key");
    assert!(
        game.files_mut().take_dirty_files().is_empty(),
        "the title writes nothing"
    );
    assert_eq!(
        game.files().file("/h2-data.dat").map(|data| data.len()),
        Some(14352),
        "the companion data is loaded"
    );
    let title_frame = game.display().indexed_frame().to_vec();

    let mut title_exit = game.clone();
    title_exit.input_mut().set_keys(*b"m");
    let mut halted = None;
    for _ in 0..3 {
        if let FrameStatus::Halted(code) = title_exit.run_frame().unwrap().status {
            halted = Some(code);
            break;
        }
    }
    title_exit.input_mut().set_keys([]);
    assert_eq!(halted, Some(0), "the third title entry quits");
    assert_eq!(
        title_exit.vm().pc(),
        0x114b1,
        "the title exits through its halt"
    );

    press(&mut game, b'n');
    assert_eq!(game.vm().pc(), 0x114a2, "the second entry is a placeholder");
    press(&mut game, 13);
    assert_eq!(
        game.vm().pc(),
        0x114b2,
        "a key returns from the placeholder"
    );
    press(&mut game, b'b');
    assert_eq!(
        game.vm().pc(),
        0x114f2,
        "the first entry asks for the player faction"
    );
    press(&mut game, 13);
    assert_eq!(
        game.vm().pc(),
        0x11765,
        "the faction grid waits for a choice"
    );
    assert_eq!(cursor(&game), (0, 0), "the grid starts on the first entry");
    press(&mut game, b'd');
    assert_eq!(cursor(&game), (56, 0), "right advances the column");
    press(&mut game, b'd');
    assert_eq!(cursor(&game), (112, 0));
    press(&mut game, b'd');
    assert_eq!(cursor(&game), (0, 0), "the row wraps");
    press(&mut game, b'w');
    assert_eq!(cursor(&game), (0, 40), "up advances the row");
    press(&mut game, b'w');
    assert_eq!(cursor(&game), (0, 0), "the column wraps vertically");
    press(&mut game, 13);
    assert_eq!(game.vm().pc(), 0x11814, "the grid asks for the AI faction");
    press(&mut game, 13);
    assert_eq!(game.vm().pc(), 0x11a99, "the AI grid waits for a choice");
    assert_eq!(
        cursor(&game),
        (0, 0),
        "the AI grid starts on the first entry"
    );
    press(&mut game, b'w');
    assert_eq!(cursor(&game), (0, 40));
    press(&mut game, b'd');
    assert_eq!(cursor(&game), (56, 40));
    press(&mut game, 13);
    assert_eq!(game.vm().pc(), 0x1128c, "the choices open the main menu");
    assert_eq!(word(&game, 0x205d), 0, "the player faction is stored");
    assert_eq!(word(&game, 0x205f), 4, "the AI faction is stored");
    assert_eq!(cursor(&game), (1, 30), "the main menu starts on mining");
    let main_frame = game.display().indexed_frame().to_vec();
    assert_ne!(
        main_frame, title_frame,
        "the main menu renders a different screen"
    );

    let mut recruit = game.clone();

    press(&mut game, b'a');
    assert_eq!(cursor(&game), (1, 30), "left clamps on the first column");
    press(&mut game, b'd');
    assert_eq!(cursor(&game), (53, 30));
    press(&mut game, b'd');
    assert_eq!(cursor(&game), (105, 30));
    press(&mut game, b'd');
    assert_eq!(cursor(&game), (105, 30), "right clamps on the last column");
    press(&mut game, b'a');
    assert_eq!(cursor(&game), (53, 30));
    press(&mut game, b'a');
    assert_eq!(cursor(&game), (1, 30));
    press(&mut game, b'w');
    assert_eq!(cursor(&game), (1, 30), "up clamps on the first row");
    press(&mut game, b's');
    assert_eq!(cursor(&game), (1, 69));
    press(&mut game, b'w');
    assert_eq!(cursor(&game), (1, 30));
    press(&mut game, b's');
    assert_eq!(cursor(&game), (1, 69));
    press(&mut game, b'w');
    assert_eq!(cursor(&game), (1, 30));

    press(&mut game, 13);
    assert_eq!(
        game.vm().pc(),
        0x10da6,
        "the first entry opens the mine menu"
    );
    assert_eq!(cursor(&game), (1, 30), "the mine menu starts on mining");

    let mut upgrade = game.clone();
    press(&mut upgrade, b'd');
    assert_eq!(cursor(&upgrade), (53, 30));
    press(&mut upgrade, 13);
    assert_eq!(upgrade.vm().pc(), 0xfdf3, "the upgrade page shows its cost");
    assert_eq!(gold(&upgrade), 1000, "the page reads the treasury");
    press(&mut upgrade, b'y');
    assert_eq!(upgrade.vm().pc(), 0xfe5d, "gold pays for the upgrade");
    assert_eq!(gold(&upgrade), 700, "the upgrade costs three hundred");
    assert_eq!(word(&upgrade, 0x2001), 1, "the mine tier is upgraded");
    assert_eq!(action(&upgrade), 5, "the upgrade does not spend an action");
    advance(&mut upgrade, 60);
    assert_eq!(upgrade.vm().pc(), 0x85d8, "the result message appears");
    press(&mut upgrade, 13);
    assert_eq!(upgrade.vm().pc(), 0xfea0, "the message starts closing");
    advance(&mut upgrade, 60);
    assert_eq!(upgrade.vm().pc(), 0x10da6, "the mine menu returns");
    assert_eq!(cursor(&upgrade), (53, 30), "the cursor stays on upgrade");

    let mut build = game.clone();
    press(&mut build, b'd');
    press(&mut build, b'd');
    assert_eq!(cursor(&build), (105, 30));
    press(&mut build, 13);
    assert_eq!(build.vm().pc(), 0xff67, "the mine page shows its cost");
    assert_eq!(gold(&build), 1000);
    press(&mut build, b'y');
    assert_eq!(build.vm().pc(), 0xffc3, "gold pays for the mine");
    assert_eq!(gold(&build), 500, "the mine costs five hundred");
    assert_eq!(word(&build, 0x2002), 1, "the mine is built");
    assert_eq!(action(&build), 5, "building does not spend an action");
    advance(&mut build, 60);
    assert_eq!(build.vm().pc(), 0x85d8, "the result message appears");
    press(&mut build, 13);
    assert_eq!(build.vm().pc(), 0x10006, "the message starts closing");
    advance(&mut build, 60);
    assert_eq!(build.vm().pc(), 0x10da6, "the mine menu returns");
    assert_eq!(cursor(&build), (105, 30), "the cursor stays on the mine");

    let mut explore = game.clone();
    press(&mut explore, b's');
    assert_eq!(cursor(&explore), (1, 69));
    press(&mut explore, 13);
    assert_eq!(
        explore.vm().pc(),
        0x10024,
        "the exploration page waits for a key"
    );
    press(&mut explore, 13);
    assert_eq!(explore.vm().pc(), 0x10253, "the exploration finds gold");
    assert_eq!(gold(&explore), 1090, "the find is added to the treasury");
    assert_eq!(word(&explore, 0x58e7), 1, "the find is recorded");
    assert_eq!(word(&explore, 0x58e9), 90, "the find is ninety gold");
    assert_eq!(action(&explore), 4, "exploration spends one action");
    advance(&mut explore, 60);
    assert_eq!(explore.vm().pc(), 0x10253, "the find stays on screen");
    press(&mut explore, 13);
    assert_eq!(explore.vm().pc(), 0x85d8, "the find starts closing");
    advance(&mut explore, 60);
    assert_eq!(explore.vm().pc(), 0x85d9, "the closing message animates");
    press(&mut explore, 13);
    assert_eq!(explore.vm().pc(), 0x10da6, "the mine menu returns");
    assert_eq!(cursor(&explore), (1, 69), "the cursor stays on exploration");

    let mut view = game.clone();
    press(&mut view, b'd');
    press(&mut view, b's');
    assert_eq!(cursor(&view), (53, 69));
    press(&mut view, 13);
    assert_eq!(view.vm().pc(), 0x10364, "the status page shows resources");
    press(&mut view, 13);
    assert_eq!(view.vm().pc(), 0x10463, "the status page shows the hero");
    press(&mut view, 13);
    assert_eq!(view.vm().pc(), 0x10da6, "the status pages return");
    assert_eq!(cursor(&view), (53, 69), "the cursor stays on status");

    let mut back = game.clone();
    press(&mut back, b'd');
    press(&mut back, b'd');
    press(&mut back, b's');
    press(&mut back, 13);
    assert_eq!(back.vm().pc(), 0x1128c, "the last entry returns");
    assert_eq!(cursor(&back), (1, 30), "the main menu restarts");

    press(&mut game, 13);
    assert_eq!(game.vm().pc(), 0xfd15, "the first entry starts mining");
    assert_eq!(gold(&game), 1044, "the haul is added to the treasury");
    assert_eq!(word(&game, 0x58e9), 44, "the haul is forty-four gold");
    assert_eq!(action(&game), 4, "mining spends one action");
    press(&mut game, 13);
    assert_eq!(game.vm().pc(), 0x85d8, "the haul message appears");
    advance(&mut game, 40);
    assert_eq!(game.vm().pc(), 0x85d9, "the haul message animates");
    press(&mut game, 13);
    assert_eq!(game.vm().pc(), 0x10da6, "mining returns to the mine menu");
    assert_eq!(cursor(&game), (1, 30), "the mine menu restarts");
    press(&mut game, b'd');
    press(&mut game, b'd');
    press(&mut game, b's');
    press(&mut game, 13);
    assert_eq!(
        game.vm().pc(),
        0x1128c,
        "the mine menu returns to the main menu"
    );
    assert_eq!(cursor(&game), (1, 30), "the main menu restarts");

    press(&mut recruit, b'd');
    press(&mut recruit, b'd');
    assert_eq!(cursor(&recruit), (105, 30));
    press(&mut recruit, 13);
    assert_eq!(recruit.vm().pc(), 0x10a77, "the recruit list opens");
    assert_eq!(
        cursor(&recruit),
        (1, 30),
        "the recruit list starts on the first troop"
    );
    press(&mut recruit, 13);
    assert_eq!(
        recruit.vm().pc(),
        0x10834,
        "the first troop shows its stats"
    );
    assert_eq!(gold(&recruit), 1000);
    press(&mut recruit, b'y');
    assert_eq!(recruit.vm().pc(), 0x108f2, "gold pays for the troop");
    press(&mut recruit, 13);
    assert_eq!(recruit.vm().pc(), 0x85d8, "the recruit message appears");
    advance(&mut recruit, 60);
    assert_eq!(recruit.vm().pc(), 0x10a77, "the recruit list returns");
    assert_eq!(gold(&recruit), 850, "the troop costs one hundred fifty");
    assert_eq!(recruit.vm().memory()[0x2722], 3, "the troop joins the army");
    assert_eq!(action(&recruit), 4, "recruiting spends one action");
    press(&mut recruit, 27);
    assert_eq!(recruit.vm().pc(), 0x1128c, "escape leaves the recruit list");
    assert_eq!(cursor(&recruit), (1, 30));

    press(&mut game, b'd');
    press(&mut game, 13);
    assert_eq!(game.vm().pc(), 0x10ca7, "the army page lists the troops");
    press(&mut game, 13);
    assert_eq!(game.vm().pc(), 0x1128c, "the army page returns");
    assert_eq!(cursor(&game), (1, 30));

    press(&mut game, b's');
    press(&mut game, 13);
    assert_eq!(
        game.vm().pc(),
        0x10364,
        "the status page opens from the menu"
    );
    press(&mut game, 13);
    assert_eq!(game.vm().pc(), 0x10463);
    press(&mut game, 13);
    assert_eq!(game.vm().pc(), 0x1128c);
    assert_eq!(cursor(&game), (1, 69), "the cursor stays on status");

    press(&mut game, b'd');
    press(&mut game, b'd');
    assert_eq!(cursor(&game), (105, 69));
    press(&mut game, 13);
    assert_eq!(game.vm().pc(), 0x10565, "the system menu opens");
    assert_eq!(
        cursor(&game),
        (1, 30),
        "the system menu starts on its first entry"
    );

    press(&mut game, b'd');
    press(&mut game, b'd');
    press(&mut game, 13);
    assert_eq!(
        game.vm().pc(),
        0xfbfc,
        "the first help page waits for a key"
    );
    press(&mut game, 13);
    assert_eq!(
        game.vm().pc(),
        0xfc96,
        "the second help page waits for a key"
    );
    press(&mut game, 13);
    assert_eq!(game.vm().pc(), 0x10565, "the help pages return");
    assert_eq!(cursor(&game), (105, 30), "the cursor stays on help");

    press(&mut game, b's');
    press(&mut game, b'a');
    assert_eq!(cursor(&game), (53, 69));
    press(&mut game, 13);
    assert_eq!(game.vm().pc(), 0xfb9b, "the about page waits for a key");
    press(&mut game, 13);
    assert_eq!(game.vm().pc(), 0x10565, "the about page returns");
    assert_eq!(cursor(&game), (53, 69), "the cursor stays on about");

    press(&mut game, b'w');
    assert_eq!(cursor(&game), (53, 30));
    press(&mut game, 13);
    assert_eq!(game.vm().pc(), 0x10565, "save and quit is not implemented");
    assert_eq!(cursor(&game), (53, 30));
    assert!(
        game.files_mut().take_dirty_files().is_empty(),
        "save and quit writes nothing"
    );

    press(&mut game, b'a');
    assert_eq!(cursor(&game), (1, 30));
    press(&mut game, 13);
    assert_eq!(
        game.vm().pc(),
        0x111b7,
        "ending the turn starts the transition"
    );
    let mut frames = 0;
    loop {
        let status = game.run_frame().unwrap().status;
        frames += 1;
        if matches!(status, FrameStatus::WaitingForInput) {
            break;
        }
        assert!(frames < 600, "the turn transition settles");
    }
    assert_eq!(game.vm().pc(), 0x1128c, "the new turn opens the main menu");
    assert_eq!(cursor(&game), (1, 30));
    assert_eq!(action(&game), 5, "a new turn refills the action points");
    assert_eq!(word(&game, 0x2015), 6, "the turn counter advances");

    let mut quit = game.clone();
    press(&mut quit, b'd');
    press(&mut quit, b'd');
    press(&mut quit, b's');
    press(&mut quit, 13);
    assert_eq!(quit.vm().pc(), 0x10565);
    press(&mut quit, b's');
    assert_eq!(cursor(&quit), (1, 69));
    quit.input_mut().set_keys([13]);
    let mut exited = None;
    for _ in 0..3 {
        if let FrameStatus::Halted(code) = quit.run_frame().unwrap().status {
            exited = Some(code);
            break;
        }
    }
    quit.input_mut().set_keys([]);
    assert_eq!(exited, Some(0), "the quit entry exits");
    assert_eq!(
        quit.vm().pc(),
        0x1065d,
        "the system menu exits through its halt"
    );

    press(&mut game, b'd');
    press(&mut game, b's');
    assert_eq!(cursor(&game), (53, 69));
    press(&mut game, 13);
    assert_eq!(
        game.vm().pc(),
        0xe8d5,
        "attack asks for the attacking troops"
    );
    assert_eq!(cursor(&game), (53, 69));
    press(&mut game, 13);
    assert_eq!(game.vm().pc(), 0xecc6, "the battle draft waits for picks");

    let mut unprepared = game.clone();
    press(&mut unprepared, b'y');
    assert_eq!(
        unprepared.vm().pc(),
        0xecc6,
        "a battle needs at least one troop pick"
    );

    press(&mut game, 13);
    assert_eq!(game.vm().pc(), 0xecc6, "the draft takes the pick");
    press(&mut game, b'y');
    assert_eq!(game.vm().pc(), 0xdb60, "the battle begins");
    advance(&mut game, 60);
    assert_eq!(game.vm().pc(), 0xe386, "the battle scene is shown");
    let battle_frame = game.display().indexed_frame().to_vec();
    assert_ne!(
        battle_frame, main_frame,
        "the battle renders a different screen"
    );
    press(&mut game, 32);
    assert_eq!(game.vm().pc(), 0xe440, "space shows the round result");
    press(&mut game, 13);
    assert_eq!(game.vm().pc(), 0xe386, "enter starts the next round");
    assert!(
        game.files_mut().take_dirty_files().is_empty(),
        "the battle writes nothing"
    );
    assert_eq!(
        game.files().file("/h2-data.dat").map(|data| data.len()),
        Some(14352),
        "the companion data is unchanged"
    );
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn hero_battle_edition_drafts_units_and_fights() {
    let mut game = load("LAVA1 英雄无敌对战版/英雄无敌v1.0.lav");
    let word = |game: &Emulator, address: usize| {
        u16::from_le_bytes(game.vm().memory()[address..address + 2].try_into().unwrap())
    };
    let cursor = |game: &Emulator| word(game, 0x2006);

    advance(&mut game, 600);
    assert_eq!(game.vm().pc(), 0x110ff, "the splash waits for a key");
    assert!(
        game.files_mut().take_dirty_files().is_empty(),
        "the splash writes nothing"
    );
    assert_eq!(
        game.files().file("/h2-data.dat").map(|data| data.len()),
        Some(14352),
        "the companion data is loaded"
    );
    assert_eq!(
        game.files()
            .file("/英雄无敌帮助.txt")
            .map(|data| data.len()),
        Some(986),
        "the help text is loaded"
    );
    let splash_frame = game.display().indexed_frame().to_vec();

    press(&mut game, 32);
    assert_eq!(game.vm().pc(), 0x110b8, "space opens the title");
    let title_frame = game.display().indexed_frame().to_vec();
    assert_ne!(
        title_frame, splash_frame,
        "the title renders a different screen"
    );

    let mut title_exit = game.clone();
    title_exit.input_mut().set_keys(*b"m");
    let mut halted = None;
    for _ in 0..3 {
        if let FrameStatus::Halted(code) = title_exit.run_frame().unwrap().status {
            halted = Some(code);
            break;
        }
    }
    title_exit.input_mut().set_keys([]);
    assert_eq!(halted, Some(0), "the third title entry quits");
    assert_eq!(
        title_exit.vm().pc(),
        0x110b7,
        "the title exits through its halt"
    );

    let mut two_player = game.clone();
    press(&mut two_player, b'n');
    assert_eq!(
        two_player.vm().pc(),
        0x111b5,
        "the second entry starts a two player battle"
    );
    assert_eq!(word(&two_player, 0x2004), 2, "two players are stored");
    press(&mut two_player, 13);
    press(&mut two_player, 13);
    press(&mut two_player, 13);
    assert_eq!(
        two_player.vm().pc(),
        0x118da,
        "the second player drafts troops"
    );
    assert_eq!(cursor(&two_player), 0);
    press(&mut two_player, 13);
    assert_eq!(word(&two_player, 0x2046), 1, "the pick is counted");
    press(&mut two_player, b'y');
    assert_eq!(
        two_player.vm().pc(),
        0x11950,
        "the second player picks a faction"
    );
    assert_eq!(
        word(&two_player, 0x2004),
        2,
        "both players stay in the battle"
    );

    press(&mut game, b'b');
    assert_eq!(
        game.vm().pc(),
        0x111b5,
        "the first entry starts a single battle"
    );
    assert_eq!(word(&game, 0x2004), 1, "one player is stored");
    press(&mut game, 13);
    assert_eq!(
        game.vm().pc(),
        0x11419,
        "the faction grid waits for a choice"
    );
    press(&mut game, 13);
    assert_eq!(
        game.vm().pc(),
        0x1148f,
        "the grid asks for the player troops"
    );
    press(&mut game, 13);
    assert_eq!(game.vm().pc(), 0x118da, "the troop draft waits for picks");
    assert_eq!(cursor(&game), 0, "the draft starts on the first troop");

    press(&mut game, b'd');
    assert_eq!(cursor(&game), 20);
    press(&mut game, b'd');
    assert_eq!(cursor(&game), 40);
    press(&mut game, b'd');
    assert_eq!(cursor(&game), 60);
    press(&mut game, b'd');
    assert_eq!(cursor(&game), 80);
    press(&mut game, b'd');
    assert_eq!(cursor(&game), 100);
    press(&mut game, b'd');
    assert_eq!(cursor(&game), 0, "right wraps to the first troop");
    press(&mut game, b'a');
    assert_eq!(cursor(&game), 100, "left wraps to the last troop");
    press(&mut game, b'a');
    assert_eq!(cursor(&game), 80);

    let mut no_pick = game.clone();
    press(&mut no_pick, b'y');
    assert_eq!(
        no_pick.vm().pc(),
        0x118da,
        "a battle needs at least one troop"
    );
    assert_eq!(word(&no_pick, 0x2046), 0);

    assert_eq!(
        game.vm().memory()[0x271b],
        6,
        "the fifth troop has six copies"
    );
    press(&mut game, 13);
    assert_eq!(game.vm().memory()[0x271b], 5, "the pick removes one copy");
    assert_eq!(word(&game, 0x2046), 1, "the pick is counted");
    press(&mut game, b'y');
    assert_eq!(game.vm().pc(), 0x11950, "the AI drafts its troops");
    assert_eq!(word(&game, 0x2004), 1);
    press(&mut game, 13);
    assert_eq!(
        game.vm().pc(),
        0x11c11,
        "the AI faction grid waits for a choice"
    );
    press(&mut game, 13);
    assert_eq!(game.vm().pc(), 0x11c87, "the grid asks for the AI troops");
    press(&mut game, 13);
    assert_eq!(
        game.vm().pc(),
        0x120d6,
        "the AI troop draft waits for picks"
    );
    assert_eq!(word(&game, 0x2046), 0, "the AI draft resets the picks");
    press(&mut game, 13);
    assert_eq!(word(&game, 0x2046), 1, "the AI pick is counted");
    press(&mut game, b'y');
    assert_eq!(game.vm().pc(), 0x10321, "both sides ready the battle");
    advance(&mut game, 60);
    assert_eq!(game.vm().pc(), 0x10b52, "the battle scene is shown");
    let battle_frame = game.display().indexed_frame().to_vec();
    assert_ne!(
        battle_frame, title_frame,
        "the battle renders a different screen"
    );
    press(&mut game, 32);
    assert_eq!(game.vm().pc(), 0x10c0c, "space shows the round result");
    press(&mut game, 13);
    assert_eq!(game.vm().pc(), 0x10b52, "enter starts the next round");
    assert!(
        game.files_mut().take_dirty_files().is_empty(),
        "the battle writes nothing"
    );
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn luobo_dazuo_zhan_package_walks_the_menus_and_uses_side_keys() {
    let mut game = load("LAVA1 萝卜大作战修改版/萝卜大作战.pac");
    let byte = |game: &Emulator, address: usize| game.vm().memory()[address];
    let word = |game: &Emulator, address: usize| {
        u16::from_le_bytes(game.vm().memory()[address..address + 2].try_into().unwrap())
    };

    advance(&mut game, 60);
    assert_eq!(game.vm().pc(), 0x56b5, "the menu waits for a key");
    assert_eq!(byte(&game, 0x3c94), 0, "the menu starts on the first entry");
    press(&mut game, 21);
    assert_eq!(byte(&game, 0x3c94), 1, "down advances the menu");
    press(&mut game, 21);
    assert_eq!(byte(&game, 0x3c94), 2);
    press(&mut game, 21);
    assert_eq!(byte(&game, 0x3c94), 3);
    press(&mut game, 21);
    assert_eq!(byte(&game, 0x3c94), 0, "down wraps to the first entry");
    press(&mut game, 20);
    assert_eq!(byte(&game, 0x3c94), 3, "up wraps to the last entry");
    press(&mut game, 20);
    assert_eq!(byte(&game, 0x3c94), 2);
    press(&mut game, 20);
    assert_eq!(byte(&game, 0x3c94), 1);
    press(&mut game, 20);
    assert_eq!(byte(&game, 0x3c94), 0);

    press(&mut game, 13);
    assert_eq!(
        game.vm().pc(),
        0x530a,
        "the first entry opens the mode select"
    );
    press(&mut game, 21);
    press(&mut game, 21);
    press(&mut game, 21);
    press(&mut game, 20);
    press(&mut game, 13);
    assert_eq!(byte(&game, 0x3c8f), 3, "the third mode is selected");
    assert_eq!(game.vm().pc(), 0x553e, "the stage select follows the mode");
    press(&mut game, 13);
    assert_eq!(
        game.vm().pc(),
        0x2d81,
        "the empty custom stage reports an error"
    );
    advance(&mut game, 120);
    assert_eq!(game.vm().pc(), 0x56b5, "the error returns to the menu");
    assert_eq!(byte(&game, 0x3c8e), 10, "the custom stage is stored");

    press(&mut game, 13);
    assert_eq!(
        game.vm().pc(),
        0x530a,
        "the mode select keeps the last mode"
    );
    press(&mut game, 21);
    press(&mut game, 13);
    assert_eq!(byte(&game, 0x3c8f), 1, "down wraps to the first mode");
    assert_eq!(game.vm().pc(), 0x553e, "the stage select opens stage 24");
    press(&mut game, 19);
    press(&mut game, 14);
    press(&mut game, 13);
    assert_eq!(game.vm().pc(), 0x4fc8, "enter starts the level banner");
    assert_eq!(byte(&game, 0x3c8b), 24, "the selected stage is 24");
    assert_eq!(byte(&game, 0x3c8c), 24, "the mode remembers stage 24");
    advance(&mut game, 60);
    assert_eq!(game.vm().pc(), 0x3029, "the game loop waits for input");
    assert_eq!(byte(&game, 0x3c8f), 1, "mode one is active");
    assert_eq!(word(&game, 0x4071), 11, "the level has eleven carrots");
    assert_eq!(
        (word(&game, 0x4083), word(&game, 0x4085)),
        (10, 8),
        "the dog starts on the entrance"
    );
    assert_eq!(
        (word(&game, 0x4077), word(&game, 0x4079)),
        (5, 6),
        "the camera starts at the spawn"
    );

    hold(&mut game, 37, 20);
    assert_eq!(
        (word(&game, 0x4083), word(&game, 0x4085)),
        (10, 6),
        "the side up key walks north"
    );
    assert_eq!(byte(&game, 0x3662), 17, "the one-way wall is still intact");
    hold(&mut game, 39, 15);
    assert_eq!(
        (word(&game, 0x4083), word(&game, 0x4085)),
        (9, 6),
        "the side left key walks onto the carrot"
    );
    assert_eq!(word(&game, 0x406f), 1, "the carrot is collected");
    assert_eq!(byte(&game, 0x3661), 7, "the carrot tile becomes empty");
    assert_eq!(byte(&game, 0x3662), 18, "the one-way wall becomes a hazard");
    assert_eq!(
        (word(&game, 0x4077), word(&game, 0x4079)),
        (4, 4),
        "the camera follows the dog"
    );
    assert!(
        game.files_mut().take_dirty_files().is_empty(),
        "the run writes nothing"
    );
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn luobo_dazuo_zhan_unpacked_blocks_walls_and_resumes_after_idle() {
    let mut game = load("LAVA1 萝卜大作战修改版/非pac/萝卜大作战.lav");
    let word = |game: &Emulator, address: usize| {
        u16::from_le_bytes(game.vm().memory()[address..address + 2].try_into().unwrap())
    };
    let dog = |game: &Emulator| (word(game, 0x4083), word(game, 0x4085));

    advance(&mut game, 60);
    assert_eq!(game.vm().pc(), 0x56b5, "the menu waits for a key");
    press(&mut game, 13);
    assert_eq!(game.vm().pc(), 0x530a, "enter opens the mode select");
    press(&mut game, 13);
    assert_eq!(game.vm().pc(), 0x553e, "enter opens the stage select");
    press(&mut game, 13);
    assert_eq!(game.vm().pc(), 0x4fc8, "enter starts the level banner");
    advance(&mut game, 60);
    assert_eq!(game.vm().pc(), 0x3029, "the game loop waits for input");
    assert_eq!(dog(&game), (10, 8), "the dog starts on the entrance");

    hold(&mut game, 39, 15);
    assert_eq!(dog(&game), (10, 8), "the wall blocks the side left key");
    hold(&mut game, 23, 15);
    assert_eq!(dog(&game), (10, 8), "the standard left arrow is unused");
    hold(&mut game, 20, 15);
    assert_eq!(dog(&game), (10, 8), "the standard up arrow is unused");

    advance(&mut game, 700);
    let pc = game.vm().pc();
    assert!(
        (0x520c..=0x5218).contains(&pc),
        "the game pauses after ten idle seconds, pc=0x{pc:x}"
    );
    hold(&mut game, 37, 20);
    assert_eq!(dog(&game), (10, 6), "a side key resumes and walks");

    press(&mut game, 27);
    assert!(
        (0x35c1..=0x35ca).contains(&game.vm().pc()),
        "escape asks before leaving, pc=0x{:x}",
        game.vm().pc()
    );
    press(&mut game, b'n');
    assert_eq!(game.vm().pc(), 0x3029, "n returns to the game");
    press(&mut game, 27);
    assert!(
        (0x35c1..=0x35ca).contains(&game.vm().pc()),
        "escape asks again, pc=0x{:x}",
        game.vm().pc()
    );
    press(&mut game, b'y');
    assert_eq!(game.vm().pc(), 0x56b5, "y returns to the menu");
    assert!(
        game.files_mut().take_dirty_files().is_empty(),
        "the run writes nothing"
    );
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn luobo_dazuo_zhan_map_editor_edits_and_saves_stage_two() {
    let mut game = load("LAVA1 萝卜大作战修改版/非pac/MapEditor.lav");
    let byte = |game: &Emulator, address: usize| game.vm().memory()[address];

    advance(&mut game, 60);
    assert_eq!(game.vm().pc(), 0x1aa, "the editor menu waits for a key");
    press(&mut game, 21);
    assert_eq!(
        game.vm().pc(),
        0x1aa,
        "the stage selector stays on the menu"
    );
    press(&mut game, 13);
    assert_eq!(game.vm().pc(), 0x3dd, "enter opens the map editor");
    assert_eq!(byte(&game, 0x2000), 2, "the second custom stage is loaded");
    assert_eq!(
        (byte(&game, 0x25f5), byte(&game, 0x25f6)),
        (0, 0),
        "the cursor starts at the origin"
    );
    press(&mut game, 22);
    assert_eq!(byte(&game, 0x25f5), 1, "right moves the cursor");
    press(&mut game, 21);
    assert_eq!(byte(&game, 0x25f6), 1, "down moves the cursor");
    press(&mut game, b'a');
    assert_eq!(byte(&game, 0x2000 + 21), 0x0b, "a letter stamps its tile");
    press(&mut game, 13);
    assert_eq!(game.vm().pc(), 0x853, "the editor asks about saving");
    press(&mut game, 27);
    assert_eq!(game.vm().pc(), 0x3d8, "escape returns to the editor");
    press(&mut game, 13);
    assert_eq!(game.vm().pc(), 0x853);
    press(&mut game, 13);
    assert_eq!(game.vm().pc(), 0x8f3, "enter saves the map");
    let mut files = game.files_mut().take_dirty_files();
    assert_eq!(files.len(), 1, "saving writes one file");
    let (path, bytes) = files.remove(0);
    assert_eq!(path, "/tzbb_mp.dat");
    assert_eq!(bytes.len(), 36453);
    assert_eq!(
        bytes[24421], 0x0b,
        "the stamped tile is stored in stage two"
    );
    press(&mut game, 113);
    assert_eq!(game.vm().pc(), 0x1aa, "q returns to the editor menu");

    let mut protected = load("LAVA1 萝卜大作战修改版/非pac/MapEditor.lav");
    advance(&mut protected, 60);
    press(&mut protected, 19);
    press(&mut protected, 13);
    assert_eq!(protected.vm().pc(), 0x2d6, "a protected mode is refused");
    assert_eq!(
        byte(&protected, 0x2000),
        0,
        "no map is loaded for a protected mode"
    );
    press(&mut protected, 13);
    assert_eq!(
        protected.vm().pc(),
        0x1aa,
        "the warning returns to the menu"
    );
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn wawa_monopoly_package_walks_menus_writes_settings_and_exits() {
    let builds = [
        "LAVA1 蛙蛙大富翁/蛙蛙大富翁打包.pac",
        "LAVA1 蛙蛙大富翁/非pac/蛙蛙大富翁.lav",
    ];
    let mut all_frames = Vec::new();
    let mut all_configs = Vec::new();
    for path in builds {
        let mut game = load(path);
        let local = |game: &Emulator, offset: usize| {
            game.vm().memory()[game.vm().local_bp() as usize + offset]
        };

        advance(&mut game, 300);
        assert_eq!(game.vm().pc(), 0x13fb9, "the splash waits for a key");
        assert!(
            game.files_mut().take_dirty_files().is_empty(),
            "the splash writes nothing"
        );
        let mut frames = vec![game.display().indexed_frame().to_vec()];

        for _ in 0..4 {
            press(&mut game, 13);
        }
        assert_eq!(game.vm().pc(), 0xe3f0, "the splash opens the main menu");
        assert_eq!(local(&game, 9), 0, "the menu starts on new game");
        frames.push(game.display().indexed_frame().to_vec());

        press(&mut game, 22);
        assert_eq!(local(&game, 9), 1, "right selects save game");
        press(&mut game, 23);
        assert_eq!(local(&game, 9), 0, "left returns to new game");
        press(&mut game, 21);
        assert_eq!(local(&game, 9), 2, "down selects load game");
        press(&mut game, 13);
        assert_eq!(game.vm().pc(), 0xe900, "load opens the slot list");
        press(&mut game, 13);
        assert_eq!(game.vm().pc(), 0x5b13, "an empty slot reports missing data");
        assert!(
            game.files_mut().take_dirty_files().is_empty(),
            "an empty slot writes nothing"
        );
        frames.push(game.display().indexed_frame().to_vec());
        press(&mut game, 13);
        assert_eq!(game.vm().pc(), 0xe3f0, "the message returns to the menu");
        assert_eq!(local(&game, 9), 2, "the menu keeps the load entry");

        press(&mut game, 22);
        assert_eq!(local(&game, 9), 3, "right selects the settings");
        press(&mut game, 13);
        assert_eq!(game.vm().pc(), 0xdb52, "enter opens the settings");
        assert_eq!(local(&game, 5), 0, "the settings start on the first row");
        press(&mut game, 22);
        assert_eq!(local(&game, 5), 1, "right steps the row forward");
        press(&mut game, 23);
        assert_eq!(local(&game, 5), 0, "left steps the row back");
        press(&mut game, 21);
        assert_eq!(local(&game, 5), 1, "down steps the row forward");
        press(&mut game, 20);
        assert_eq!(local(&game, 5), 0, "up steps the row back");
        press(&mut game, 23);
        assert_eq!(local(&game, 5), 2, "left wraps to the last row");
        press(&mut game, 21);
        assert_eq!(local(&game, 5), 0, "down wraps to the first row");
        press(&mut game, 22);
        assert_eq!(local(&game, 5), 1, "right selects the speed row");
        press(&mut game, 13);
        assert_eq!(game.vm().pc(), 0xd98f, "enter opens the speed slider");
        assert_eq!(local(&game, 5), 5, "the slider starts in the middle");
        press(&mut game, 22);
        assert_eq!(local(&game, 5), 6, "right speeds the slider up");
        press(&mut game, 22);
        assert_eq!(local(&game, 5), 7, "the slider reaches its top step");
        frames.push(game.display().indexed_frame().to_vec());
        press(&mut game, 27);
        assert_eq!(game.vm().pc(), 0xdb52, "escape returns to the settings");
        let configs = game.files_mut().take_dirty_files();
        assert_eq!(configs.len(), 1, "leaving the slider writes the config");
        assert!(
            configs[0].0.to_lowercase().ends_with("richcfig.dat"),
            "the config is written next to the resource that was read"
        );
        assert_eq!(configs[0].1.len(), 127, "the config keeps its fixed size");
        assert_eq!(configs[0].1[126], 0x46, "the config stores the top speed");
        press(&mut game, 27);
        assert_eq!(game.vm().pc(), 0xe3f0, "escape returns to the menu");
        assert_eq!(local(&game, 9), 3, "the menu keeps the settings entry");
        frames.push(game.display().indexed_frame().to_vec());

        press(&mut game, b'z');
        press(&mut game, 23);
        assert_eq!(local(&game, 9), 2, "left moves back to the load column");
        press(&mut game, 21);
        assert_eq!(local(&game, 9), 4, "down selects the leaderboard");
        press(&mut game, 21);
        assert_eq!(local(&game, 9), 6, "down selects the copyright notice");
        press(&mut game, 13);
        assert_eq!(game.vm().pc(), 0xde91, "enter opens the copyright notice");
        for expected in [
            0xdf1f, 0xdfa5, 0xe02a, 0xe0bd, 0xe14f, 0xe1e4, 0xe275, 0xe2e3, 0xe3f0,
        ] {
            press(&mut game, 13);
            assert_eq!(
                game.vm().pc(),
                expected,
                "enter advances the copyright notice"
            );
        }
        assert_eq!(local(&game, 9), 6, "the notice keeps the copyright entry");
        frames.push(game.display().indexed_frame().to_vec());

        press(&mut game, b'z');
        press(&mut game, 23);
        assert_eq!(local(&game, 9), 5, "left selects the thanks page");
        press(&mut game, 13);
        assert_eq!(game.vm().pc(), 0xe5c4, "enter opens the thanks page");
        for expected in [0xe658, 0xe6eb, 0xe73c, 0xe7c6, 0xe3f0] {
            press(&mut game, 13);
            assert_eq!(game.vm().pc(), expected, "enter advances the thanks page");
        }
        assert_eq!(local(&game, 9), 5, "the thanks page keeps its entry");
        frames.push(game.display().indexed_frame().to_vec());

        press(&mut game, b'z');
        press(&mut game, 21);
        assert_eq!(local(&game, 9), 7, "down selects the exit entry");
        press(&mut game, 13);
        assert_eq!(game.vm().pc(), 0xdccd, "enter opens the exit prompt");
        press(&mut game, 13);
        assert_eq!(game.vm().pc(), 0xdd61, "the first prompt confirms once");
        press(&mut game, 13);
        assert_eq!(game.vm().pc(), 0xdde8, "the second prompt confirms again");
        game.input_mut().set_keys([13]);
        let mut exit = None;
        for _ in 0..120 {
            if let FrameStatus::Halted(code) = game.run_frame().unwrap().status {
                exit = Some(code);
                break;
            }
        }
        game.input_mut().set_keys([]);
        assert_eq!(exit, Some(0), "the final confirm halts normally");
        assert_eq!(game.vm().pc(), 0xddfc, "the exit halts from its last page");
        frames.push(game.display().indexed_frame().to_vec());
        assert!(
            game.files_mut().take_dirty_files().is_empty(),
            "the exit writes nothing"
        );
        all_frames.push(frames);
        all_configs.push(configs);
    }

    assert_eq!(all_frames[0].len(), all_frames[1].len());
    for (index, (packaged, loose)) in all_frames[0].iter().zip(&all_frames[1]).enumerate() {
        assert_eq!(
            packaged, loose,
            "both builds render the same frame at step {index}"
        );
    }
    assert_eq!(
        all_configs[0][0].1.len(),
        all_configs[1][0].1.len(),
        "both builds write the same config size"
    );
    let packaged = &all_configs[0][0].1;
    let loose = &all_configs[1][0].1;
    assert_eq!(
        packaged[126], 0x46,
        "the packaged build stores the top speed"
    );
    assert_eq!(loose[126], 0x46, "the loose build stores the top speed");
    // The loose build ships a config with a zero first byte; the packaged build
    // starts from a missing file and leaves that byte at the default 0xff.
    let differing: Vec<usize> = packaged
        .iter()
        .zip(loose)
        .enumerate()
        .filter(|(_, (left, right))| left != right)
        .map(|(index, _)| index)
        .collect();
    assert_eq!(
        differing,
        vec![0],
        "only the missing-file default differs in the config"
    );
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn wawa_monopoly_package_rolls_saves_and_loads() {
    let builds = [
        "LAVA1 蛙蛙大富翁/蛙蛙大富翁打包.pac",
        "LAVA1 蛙蛙大富翁/非pac/蛙蛙大富翁.lav",
    ];
    let mut all_frames = Vec::new();
    let mut all_saves = Vec::new();
    for path in builds {
        let mut game = load(path);
        let word = |game: &Emulator, address: usize| {
            u16::from_le_bytes(game.vm().memory()[address..address + 2].try_into().unwrap())
        };
        let dword = |game: &Emulator, address: usize| {
            u32::from_le_bytes(game.vm().memory()[address..address + 4].try_into().unwrap())
        };
        let local = |game: &Emulator, offset: usize| {
            game.vm().memory()[game.vm().local_bp() as usize + offset]
        };
        let player = |game: &Emulator| (word(game, 0x590c), word(game, 0x590e));

        advance(&mut game, 300);
        assert_eq!(game.vm().pc(), 0x13fb9, "the splash waits for a key");
        for _ in 0..4 {
            press(&mut game, 13);
        }
        assert_eq!(game.vm().pc(), 0xe3f0, "the splash opens the main menu");
        let mut frames = vec![game.display().indexed_frame().to_vec()];

        press(&mut game, 13);
        assert_eq!(game.vm().pc(), 0xd29c, "new game opens the map list");
        press(&mut game, 21);
        assert_eq!(local(&game, 7), 1);
        press(&mut game, 20);
        assert_eq!(local(&game, 7), 0);
        press(&mut game, 14);
        assert_eq!(local(&game, 7), 4);
        assert_eq!(local(&game, 9), 4, "page down jumps a whole page");
        press(&mut game, 19);
        assert_eq!(local(&game, 7), 0);
        assert_eq!(local(&game, 9), 0, "page up returns to the first page");
        press(&mut game, 19);
        assert_eq!(local(&game, 7), 0, "page up clamps on the first page");
        frames.push(game.display().indexed_frame().to_vec());

        press(&mut game, 13);
        assert_eq!(game.vm().pc(), 0x5321, "enter opens the map preview");
        press(&mut game, 13);
        assert_eq!(
            game.vm().pc(),
            0x50fb,
            "the preview asks about player count"
        );
        press(&mut game, b'n');
        assert_eq!(game.vm().pc(), 0x11384, "declining starts the board");
        assert_eq!(
            player(&game),
            (15, 10),
            "the first player spawns on the start"
        );
        assert_eq!(dword(&game, 0x5902), 60000, "the first player keeps 60000");
        frames.push(game.display().indexed_frame().to_vec());

        press(&mut game, 13);
        assert_eq!(game.vm().pc(), 0x363b, "enter rolls the dice");
        advance(&mut game, 90);
        assert_eq!(game.vm().pc(), 0x815b, "the roll lands on a tile prompt");
        press(&mut game, b'n');
        advance(&mut game, 200);
        assert_eq!(game.vm().pc(), 0x11384, "declining returns to the board");
        assert_eq!(player(&game), (14, 11), "the first roll moves the player");
        assert_eq!(
            dword(&game, 0x5902),
            60000,
            "the unowned tile costs nothing"
        );
        frames.push(game.display().indexed_frame().to_vec());

        press(&mut game, 25);
        assert_eq!(game.vm().pc(), 0xcb77, "f5 opens the in-game menu");
        for _ in 0..4 {
            press(&mut game, 14);
            advance(&mut game, 60);
        }
        assert_eq!(game.vm().pc(), 0xe3f0, "paging reaches the main menu");
        press(&mut game, 22);
        assert_eq!(local(&game, 9), 1);
        press(&mut game, 13);
        assert_eq!(game.vm().pc(), 0xe900, "save opens the slot list");
        press(&mut game, 13);
        assert_eq!(game.vm().pc(), 0xe3f0, "saving returns to the menu");
        let saves = game.files_mut().take_dirty_files();
        assert_eq!(saves.len(), 1, "saving writes one file");
        assert_eq!(saves[0].0, "/lavadata/richsav1.dat");
        assert_eq!(saves[0].1.len(), 2685, "the save keeps its fixed size");
        let bytes = &saves[0].1;
        assert_eq!(
            u32::from_le_bytes(bytes[1549..1553].try_into().unwrap()),
            60000,
            "the save stores the money"
        );
        assert_eq!(
            u32::from_le_bytes(bytes[1561..1565].try_into().unwrap()),
            14,
            "the save stores the column"
        );
        assert_eq!(
            u32::from_le_bytes(bytes[1565..1569].try_into().unwrap()),
            11,
            "the save stores the row"
        );
        frames.push(game.display().indexed_frame().to_vec());

        press(&mut game, 27);
        assert_eq!(game.vm().pc(), 0x11384, "escape returns to the board");
        assert_eq!(player(&game), (14, 11), "the board keeps the position");

        press(&mut game, 25);
        for _ in 0..4 {
            press(&mut game, 14);
            advance(&mut game, 60);
        }
        assert_eq!(game.vm().pc(), 0xe3f0, "paging reaches the main menu again");
        press(&mut game, 21);
        assert_eq!(local(&game, 9), 2);
        press(&mut game, 13);
        assert_eq!(game.vm().pc(), 0xe900, "load opens the slot list");
        press(&mut game, 13);
        assert_eq!(game.vm().pc(), 0x61ae, "the slot reports a successful load");
        press(&mut game, 13);
        assert_eq!(game.vm().pc(), 0x11384, "loading returns to the board");
        assert_eq!(player(&game), (14, 11), "loading restores the position");
        assert_eq!(dword(&game, 0x5902), 60000, "loading restores the money");
        frames.push(game.display().indexed_frame().to_vec());

        press(&mut game, 25);
        for _ in 0..4 {
            press(&mut game, 14);
            advance(&mut game, 60);
        }
        press(&mut game, 21);
        press(&mut game, 13);
        press(&mut game, 21);
        press(&mut game, 13);
        assert_eq!(game.vm().pc(), 0x5b13, "a second slot reports missing data");
        assert!(
            game.files_mut().take_dirty_files().is_empty(),
            "reading an empty slot writes nothing"
        );
        press(&mut game, 13);
        assert_eq!(game.vm().pc(), 0xe3f0, "the message returns to the menu");
        frames.push(game.display().indexed_frame().to_vec());

        all_frames.push(frames);
        all_saves.push(saves);
    }

    assert_eq!(all_frames[0].len(), all_frames[1].len());
    for (index, (packaged, loose)) in all_frames[0].iter().zip(&all_frames[1]).enumerate() {
        assert_eq!(
            packaged, loose,
            "both builds render the same frame at step {index}"
        );
    }
    assert_eq!(
        all_saves[0][0].0, all_saves[1][0].0,
        "both builds write the same save path"
    );
    let packaged = &all_saves[0][0].1;
    let loose = &all_saves[1][0].1;
    assert_eq!(packaged.len(), loose.len(), "both saves keep the same size");
    // The loose build ships a RichCfig.dat that the packaged build lacks, so the
    // settings byte shadows one saved field even though the runtime steps match.
    let differing: Vec<usize> = packaged
        .iter()
        .zip(loose)
        .enumerate()
        .filter(|(_, (left, right))| left != right)
        .map(|(index, _)| index)
        .collect();
    assert_eq!(
        differing,
        (2317..2321).collect::<Vec<_>>(),
        "only the config-derived field differs"
    );
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn wawa_monopoly_package_adjusts_players_before_the_first_roll() {
    let builds = [
        "LAVA1 蛙蛙大富翁/蛙蛙大富翁打包.pac",
        "LAVA1 蛙蛙大富翁/非pac/蛙蛙大富翁.lav",
    ];
    let mut all_frames = Vec::new();
    for path in builds {
        let mut game = load(path);
        let word = |game: &Emulator, address: usize| {
            u16::from_le_bytes(game.vm().memory()[address..address + 2].try_into().unwrap())
        };
        let row = |game: &Emulator| game.vm().memory()[game.vm().local_bp() as usize + 7];

        advance(&mut game, 300);
        for _ in 0..4 {
            press(&mut game, 13);
        }
        assert_eq!(game.vm().pc(), 0xe3f0, "the splash opens the main menu");
        let mut frames = vec![game.display().indexed_frame().to_vec()];
        press(&mut game, 13);
        press(&mut game, 13);
        press(&mut game, 13);
        assert_eq!(
            game.vm().pc(),
            0x50fb,
            "the preview asks about player count"
        );
        press(&mut game, b'y');
        assert_eq!(game.vm().pc(), 0x4f36, "yes opens the player setup");
        assert_eq!(row(&game), 0, "the setup starts on the first player");

        press(&mut game, 22);
        assert_eq!(word(&game, 0x5978), 0, "the human player cannot be toggled");
        press(&mut game, 23);
        assert_eq!(word(&game, 0x5978), 0, "left leaves the human player alone");
        press(&mut game, 21);
        assert_eq!(row(&game), 1, "down selects the second player");
        press(&mut game, 22);
        assert_eq!(
            word(&game, 0x59fc),
            1,
            "right makes the second player human"
        );
        press(&mut game, 22);
        assert_eq!(word(&game, 0x59fc), 1, "right clamps at human");
        press(&mut game, 23);
        assert_eq!(
            word(&game, 0x59fc),
            0,
            "left returns the second player to computer"
        );
        press(&mut game, 21);
        assert_eq!(row(&game), 2, "down selects the third player");
        press(&mut game, 22);
        assert_eq!(word(&game, 0x5a80), 1, "right makes the third player human");
        press(&mut game, 23);
        assert_eq!(
            word(&game, 0x5a80),
            0,
            "left returns the third player to computer"
        );
        press(&mut game, 21);
        assert_eq!(row(&game), 0, "down wraps to the first player");
        press(&mut game, 20);
        assert_eq!(row(&game), 2, "up wraps to the third player");
        frames.push(game.display().indexed_frame().to_vec());

        press(&mut game, 13);
        assert_eq!(game.vm().pc(), 0x363b, "enter starts play and rolls");
        advance(&mut game, 90);
        assert_eq!(game.vm().pc(), 0x815b, "the opening roll lands on a tile");
        assert_eq!(
            (word(&game, 0x590c), word(&game, 0x590e)),
            (14, 11),
            "the opening roll moves the first player"
        );
        assert_eq!(
            word(&game, 0x5902),
            0xea60,
            "the first player keeps the starting deposit"
        );
        frames.push(game.display().indexed_frame().to_vec());
        all_frames.push(frames);
    }

    assert_eq!(all_frames[0].len(), all_frames[1].len());
    for (index, (packaged, loose)) in all_frames[0].iter().zip(&all_frames[1]).enumerate() {
        assert_eq!(
            packaged, loose,
            "both builds render the same frame at step {index}"
        );
    }
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn wawa_circus_runs_menus_settings_and_three_shows() {
    let path = "LAVA1 蛙蛙马戏团 1.2/蛙蛙马戏团.lav";
    let byte = |game: &Emulator, address: usize| game.vm().memory()[address];
    let word = |game: &Emulator, address: usize| {
        u16::from_le_bytes(game.vm().memory()[address..address + 2].try_into().unwrap())
    };
    let dword = |game: &Emulator, address: usize| {
        u32::from_le_bytes(game.vm().memory()[address..address + 4].try_into().unwrap())
    };
    let local =
        |game: &Emulator, offset: usize| game.vm().memory()[game.vm().local_bp() as usize + offset];
    let open_menu = || {
        let mut game = load(path);
        advance(&mut game, 240);
        assert_eq!(game.vm().pc(), 0x5b47, "the splash waits for a key");
        for _ in 0..3 {
            press(&mut game, 13);
        }
        assert_eq!(game.vm().pc(), 0x39af, "the splash opens the main menu");
        game
    };
    let open_stage = || {
        let mut game = open_menu();
        press(&mut game, 13);
        assert_eq!(game.vm().pc(), 0x5a68, "start opens the show preparation");
        press(&mut game, 13);
        assert_eq!(game.vm().pc(), 0x58db, "enter starts the first show");
        game
    };

    let mut game = open_menu();
    assert!(
        game.files_mut().take_dirty_files().is_empty(),
        "the splash writes nothing"
    );
    assert_eq!(local(&game, 5), 0, "the menu starts on the first entry");
    press(&mut game, 23);
    assert_eq!(local(&game, 5), 0, "left clamps on the first entry");
    press(&mut game, 20);
    assert_eq!(local(&game, 5), 0, "up clamps on the first entry");
    press(&mut game, 21);
    assert_eq!(local(&game, 5), 2, "down selects the second row");
    press(&mut game, 21);
    assert_eq!(local(&game, 5), 4, "down selects the third row");
    press(&mut game, 21);
    assert_eq!(local(&game, 5), 4, "down clamps on the third row");
    press(&mut game, 22);
    assert_eq!(local(&game, 5), 5, "right selects the second column");
    press(&mut game, 22);
    assert_eq!(local(&game, 5), 5, "right clamps on the second column");
    press(&mut game, 23);
    press(&mut game, 20);
    press(&mut game, 20);
    assert_eq!(local(&game, 5), 0, "up returns to the first entry");

    press(&mut game, 22);
    assert_eq!(local(&game, 5), 1, "right selects the speed entry");
    press(&mut game, 13);
    assert_eq!(game.vm().pc(), 0x3073, "enter opens the speed dialog");
    assert_eq!(word(&game, 0x267a), 0, "the speed starts at the minimum");
    press(&mut game, 23);
    assert_eq!(word(&game, 0x267a), 20, "left raises the speed");
    press(&mut game, 23);
    assert_eq!(word(&game, 0x267a), 40, "left raises the speed again");
    press(&mut game, 23);
    assert_eq!(word(&game, 0x267a), 60, "left reaches the maximum");
    press(&mut game, 22);
    assert_eq!(word(&game, 0x267a), 40, "right lowers the speed");
    press(&mut game, 20);
    assert_eq!(word(&game, 0x267a), 40, "up does not change the speed");
    press(&mut game, 27);
    assert_eq!(game.vm().pc(), 0x39af, "escape returns to the menu");
    assert_eq!(local(&game, 5), 1, "the menu keeps the speed entry");
    assert!(
        game.files_mut().take_dirty_files().is_empty(),
        "the speed dialog writes nothing"
    );

    let mut game = open_menu();
    press(&mut game, 21);
    press(&mut game, 21);
    assert_eq!(local(&game, 5), 4, "down reaches the story entry");
    press(&mut game, 13);
    assert_eq!(game.vm().pc(), 0x31b2, "enter opens the story pages");
    for expected in [0x3259, 0x32fb, 0x339c, 0x344c] {
        press(&mut game, 13);
        assert_eq!(game.vm().pc(), expected, "enter advances the story");
    }
    press(&mut game, 13);
    assert_eq!(
        game.vm().pc(),
        0x39af,
        "the last story page returns to the menu"
    );
    press(&mut game, 27);
    assert_eq!(game.vm().pc(), 0x39af, "escape returns from the story");

    let mut game = open_menu();
    press(&mut game, 21);
    press(&mut game, 22);
    assert_eq!(local(&game, 5), 3, "down and right reach the ranking entry");
    press(&mut game, 13);
    assert_eq!(game.vm().pc(), 0x381e, "enter opens the ranking");
    press(&mut game, 13);
    assert_eq!(game.vm().pc(), 0x39af, "the ranking returns to the menu");
    assert!(
        game.files_mut().take_dirty_files().is_empty(),
        "the ranking writes nothing"
    );

    let mut game = open_menu();
    press(&mut game, 21);
    press(&mut game, 13);
    assert_eq!(game.vm().pc(), 0x2d81, "enter opens the diary slots");
    assert_eq!(local(&game, 9), 0, "the diary starts on the first slot");
    press(&mut game, 21);
    assert_eq!(local(&game, 9), 1, "down selects the second slot");
    press(&mut game, 21);
    assert_eq!(local(&game, 9), 2, "down selects the third slot");
    press(&mut game, 21);
    assert_eq!(local(&game, 9), 3, "down selects the fourth slot");
    press(&mut game, 21);
    assert_eq!(local(&game, 9), 0, "down wraps on the fourth slot");
    press(&mut game, 22);
    assert_eq!(local(&game, 9), 1, "right moves to the next slot");
    press(&mut game, 23);
    assert_eq!(local(&game, 9), 0, "left moves to the previous slot");
    press(&mut game, 23);
    assert_eq!(local(&game, 9), 3, "left wraps on the first slot");
    press(&mut game, 13);
    assert_eq!(game.vm().pc(), 0x2101, "an empty slot reports missing data");
    assert!(
        game.files_mut().take_dirty_files().is_empty(),
        "an empty slot writes nothing"
    );
    press(&mut game, 13);
    assert_eq!(game.vm().pc(), 0x39af, "the message returns to the menu");

    let mut game = open_menu();
    press(&mut game, 13);
    assert_eq!(game.vm().pc(), 0x5a68, "start opens the show preparation");
    assert_eq!(word(&game, 0x2899), 1, "the first show is selected");
    press(&mut game, 22);
    assert_eq!(word(&game, 0x2899), 2, "right selects the second show");
    press(&mut game, 22);
    assert_eq!(word(&game, 0x2899), 3, "right selects the third show");
    press(&mut game, 22);
    assert_eq!(word(&game, 0x2899), 1, "right wraps to the first show");
    press(&mut game, 23);
    assert_eq!(word(&game, 0x2899), 3, "left wraps to the last show");
    press(&mut game, 23);
    assert_eq!(word(&game, 0x2899), 2, "left selects the second show");
    press(&mut game, 23);
    assert_eq!(word(&game, 0x2899), 1, "left returns to the first show");
    press(&mut game, 27);
    assert_eq!(
        game.vm().pc(),
        0x5a68,
        "escape is ignored during preparation"
    );
    press(&mut game, 22);
    assert_eq!(
        word(&game, 0x2899),
        2,
        "right selects the second show again"
    );
    press(&mut game, 13);
    assert_eq!(
        game.vm().pc(),
        0x51de,
        "the second show has its own routine"
    );
    assert_eq!(word(&game, 0x27a5), 30, "the second show places the player");
    assert_eq!(
        word(&game, 0x27a7),
        55,
        "the second show places the player high"
    );
    press(&mut game, 27);
    assert_eq!(game.vm().pc(), 0x39af, "escape leaves the second show");

    let mut game = open_menu();
    press(&mut game, 13);
    press(&mut game, 22);
    press(&mut game, 22);
    assert_eq!(word(&game, 0x2899), 3, "the third show is selected");
    press(&mut game, 13);
    assert_eq!(game.vm().pc(), 0x4910, "the third show has its own routine");
    assert_eq!(word(&game, 0x27a5), 35, "the third show places the player");
    assert_eq!(
        word(&game, 0x27a7),
        52,
        "the third show places the player high"
    );
    let mut frames = 0;
    while game.vm().pc() != 0x3b6e && frames < 60 {
        advance(&mut game, 1);
        frames += 1;
    }
    assert_eq!(game.vm().pc(), 0x3b6e, "the third show fails without input");
    assert_eq!(word(&game, 0x2897), 2, "the third show costs a life");

    let mut game = open_stage();
    assert_eq!(word(&game, 0x27a5), 30, "the first show places the player");
    assert_eq!(
        word(&game, 0x27a7),
        25,
        "the first show places the player down"
    );
    assert_eq!(
        dword(&game, 0x288f),
        2000,
        "the act starts at the left edge"
    );
    assert_eq!(dword(&game, 0x2893), 2627, "the show timer starts counting");
    assert_eq!(word(&game, 0x2899), 1, "the first show is running");
    game.input_mut().set_keys(*b"a");
    advance(&mut game, 1);
    game.input_mut().set_keys([]);
    assert_eq!(word(&game, 0x27a7), 22, "the jump starts on the next frame");
    for expected in [19, 16, 13, 10, 7, 4, 7, 10, 13, 16, 19, 22, 25] {
        advance(&mut game, 1);
        assert_eq!(word(&game, 0x27a7), expected, "the jump follows its arc");
    }
    assert_eq!(byte(&game, 0x28b5), 0, "the jump avoids the first lion");
    assert_eq!(game.vm().pc(), 0x58db, "the stage keeps running");

    let mut game = open_stage();
    for _ in 0..10 {
        game.input_mut().set_keys([22]);
        advance(&mut game, 1);
    }
    game.input_mut().set_keys([]);
    assert_eq!(dword(&game, 0x288f), 1990, "right scrolls the act forward");
    assert_eq!(game.vm().pc(), 0x58db, "the scroll does not end the show");
    assert_eq!(byte(&game, 0x28b5), 0, "the scroll avoids the first lion");

    let mut game = open_stage();
    press(&mut game, b's');
    assert_eq!(game.vm().pc(), 0x2d81, "s opens the diary during the show");
    press(&mut game, 21);
    press(&mut game, 13);
    let mut files = game.files_mut().take_dirty_files();
    assert_eq!(files.len(), 1, "saving the diary writes one file");
    let (path, bytes) = files.remove(0);
    assert_eq!(path, "/lavadata/tansav2.dat", "the diary keeps its slot");
    assert_eq!(bytes.len(), 512, "the diary keeps its fixed size");
    let saved = |offset: usize| u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap());
    assert_eq!(saved(0), 30, "the diary stores the player column");
    assert_eq!(saved(4), 25, "the diary stores the player height");
    assert_eq!(saved(0x1d4), 2000, "the diary stores the scroll");
    assert_eq!(saved(0x1d8), 2627, "the diary stores the show timer");
    assert_eq!(saved(0x1f4), 2000, "the diary stores the starting scroll");
    assert_eq!(saved(0x1f8), 2660, "the diary stores the timer start");

    let mut game = open_stage();
    assert_eq!(byte(&game, 0x28b5), 0, "the show starts without a miss");
    let mut frames = 0;
    while byte(&game, 0x28b5) == 0 && frames < 40 {
        advance(&mut game, 1);
        frames += 1;
    }
    assert_eq!(
        byte(&game, 0x28b5),
        1,
        "the first lion reaches the idle player"
    );
    assert_eq!(game.vm().pc(), 0x58db, "the hit frame keeps the stage");
    advance(&mut game, 1);
    assert_eq!(
        game.vm().pc(),
        0x3b6e,
        "the collision enters the result routine"
    );
    let mut frames = 0;
    while game.vm().pc() != 0x3be1 && frames < 200 {
        advance(&mut game, 1);
        frames += 1;
    }
    assert_eq!(game.vm().pc(), 0x3be1, "the result screen waits for a key");
    assert_eq!(word(&game, 0x2897), 2, "the collision costs one life");
    press(&mut game, 27);
    assert_eq!(
        game.vm().pc(),
        0x5a68,
        "escape returns to the show preparation"
    );
    assert_eq!(word(&game, 0x2899), 2, "the next show is selected");
    assert_eq!(word(&game, 0x2897), 2, "the lost life is kept");
    press(&mut game, 13);
    assert_eq!(
        game.vm().pc(),
        0x51de,
        "the second show starts after the miss"
    );
    assert_eq!(
        word(&game, 0x27a7),
        55,
        "the second show places the player high"
    );
    press(&mut game, 27);
    assert_eq!(game.vm().pc(), 0x39af, "escape leaves the second show");

    let mut game = open_stage();
    let taps: [usize; 109] = [
        21, 43, 62, 90, 124, 150, 169, 190, 216, 235, 260, 282, 311, 330, 350, 375, 402, 436, 454,
        479, 501, 521, 550, 581, 599, 623, 645, 665, 689, 710, 733, 754, 775, 799, 820, 842, 863,
        887, 909, 930, 952, 976, 1003, 1026, 1052, 1076, 1097, 1118, 1142, 1168, 1189, 1209, 1235,
        1269, 1295, 1315, 1336, 1361, 1382, 1405, 1427, 1466, 1490, 1510, 1534, 1555, 1577, 1601,
        1621, 1645, 1666, 1694, 1717, 1736, 1760, 1797, 1821, 1839, 1865, 1888, 1912, 1937, 1962,
        1988, 2026, 2051, 2074, 2094, 2138, 2160, 2184, 2209, 2234, 2255, 2275, 2297, 2337, 2361,
        2383, 2406, 2428, 2450, 2480, 2500, 2521, 2544, 2586, 2608, 2626,
    ];
    let mut elapsed = 0;
    for tap in taps {
        advance(&mut game, tap - elapsed - 1);
        game.input_mut().set_keys(*b"a");
        advance(&mut game, 1);
        game.input_mut().set_keys([]);
        elapsed = tap;
    }
    advance(&mut game, 62);
    assert_eq!(
        game.vm().pc(),
        0x3be1,
        "the full show reaches the result screen"
    );
    assert_eq!(dword(&game, 0x2893), 0, "the show timer runs out");
    assert_eq!(byte(&game, 0x28b5), 0, "the full show avoids every lion");
    assert_eq!(word(&game, 0x27a5), 30, "the player keeps the pad");
    assert_eq!(word(&game, 0x27a7), 19, "the last jump is still in the air");
    press(&mut game, 13);
    assert_eq!(game.vm().pc(), 0x3c79, "enter opens the score board");
    press(&mut game, 13);
    assert_eq!(
        game.vm().pc(),
        0x39af,
        "the score board returns to the menu"
    );
    assert_eq!(word(&game, 0x2899), 2, "the next show is selected");
    assert_eq!(word(&game, 0x2897), 3, "the next show restores the lives");

    press(&mut game, 21);
    press(&mut game, 21);
    press(&mut game, 22);
    assert_eq!(local(&game, 5), 5, "down and right reach the exit entry");
    press(&mut game, 13);
    assert_eq!(game.vm().pc(), 0x34f0, "enter opens the credits");
    game.input_mut().set_keys([13]);
    let mut exit = None;
    for _ in 0..120 {
        if let FrameStatus::Halted(code) = game.run_frame().unwrap().status {
            exit = Some(code);
            break;
        }
    }
    game.input_mut().set_keys([]);
    assert_eq!(exit, Some(0), "enter exits the program from the credits");
    assert_eq!(game.vm().pc(), 0x3b43, "the program halts from the credits");
    assert!(
        game.files_mut().take_dirty_files().is_empty(),
        "the exit writes nothing"
    );
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn roach_adventure_runs_menus_levels_and_chase() {
    let path = "LAVA1 蟑螂历险记/测试版/蟑螂历险记 V1.0.lav";
    let byte = |game: &Emulator, address: usize| game.vm().memory()[address];
    let word = |game: &Emulator, address: usize| {
        u16::from_le_bytes(game.vm().memory()[address..address + 2].try_into().unwrap())
    };
    let open_levels = || {
        let mut game = load(path);
        advance(&mut game, 4300);
        assert_eq!(game.vm().pc(), 0x2935, "the title waits for enter");
        press(&mut game, 13);
        advance(&mut game, 600);
        assert_eq!(game.vm().pc(), 0x2fc5, "the menu animates in");
        press(&mut game, 13);
        advance(&mut game, 80);
        assert_eq!(game.vm().pc(), 0x305a, "the menu settles");
        press(&mut game, 13);
        advance(&mut game, 200);
        assert_eq!(game.vm().pc(), 0x35bc, "start opens the level select");
        assert_eq!(
            byte(&game, 0x45e7),
            0,
            "the level select starts on the first stage"
        );
        game
    };
    let open_stage = || {
        let mut game = open_levels();
        press(&mut game, 13);
        game
    };

    let mut game = open_levels();
    for _ in 0..3 {
        press(&mut game, 22);
    }
    assert_eq!(byte(&game, 0x45e7), 3, "right walks the stage cursor up");
    press(&mut game, 23);
    press(&mut game, 23);
    assert_eq!(byte(&game, 0x45e7), 1, "left walks the stage cursor back");
    press(&mut game, 21);
    assert_eq!(byte(&game, 0x45e7), 0, "down walks the stage cursor back");
    press(&mut game, 20);
    assert_eq!(byte(&game, 0x45e7), 1, "up walks the stage cursor up");
    press(&mut game, 21);
    assert_eq!(byte(&game, 0x45e7), 0, "down returns to the first stage");
    for _ in 0..12 {
        press(&mut game, 22);
    }
    assert_eq!(
        byte(&game, 0x45e7),
        9,
        "the stage cursor clamps on the last stage"
    );
    for _ in 0..12 {
        press(&mut game, 23);
    }
    assert_eq!(
        byte(&game, 0x45e7),
        0,
        "the stage cursor clamps on the first stage"
    );
    assert!(
        game.files_mut().take_dirty_files().is_empty(),
        "the menus write nothing"
    );

    let mut game = open_stage();
    assert_eq!(word(&game, 0x4583), 5, "the stage starts with five lives");
    assert_eq!(
        word(&game, 0x4585),
        4,
        "the player marker row comes from the map"
    );
    assert_eq!(
        word(&game, 0x4587),
        9,
        "the player marker column comes from the map"
    );
    assert_eq!(
        word(&game, 0x4589),
        1,
        "the chaser marker row comes from the map"
    );
    assert_eq!(
        word(&game, 0x458b),
        18,
        "the chaser marker column comes from the map"
    );
    assert_eq!(
        word(&game, 0x457f),
        64,
        "the roach starts on its marker cell"
    );
    assert_eq!(
        word(&game, 0x4581),
        24,
        "the roach starts on its marker cell"
    );
    assert_eq!(
        word(&game, 0x4577),
        135,
        "the chaser starts on its marker cell"
    );
    assert_eq!(
        word(&game, 0x4579),
        1,
        "the chaser starts on its marker cell"
    );
    assert_eq!(word(&game, 0x45e9), 40, "the exit comes from the map");
    assert_eq!(word(&game, 0x45eb), 32, "the exit comes from the map");
    assert_eq!(byte(&game, 0x45e7), 0, "the stage keeps the selected level");

    press(&mut game, 21);
    assert_eq!(byte(&game, 0x45e5), 2, "down faces the roach down");
    assert_eq!(word(&game, 0x4581), 26, "down moves the roach one cell");
    press(&mut game, 20);
    assert_eq!(byte(&game, 0x45e5), 7, "up faces the roach up");
    assert_eq!(word(&game, 0x4581), 24, "up moves the roach back");
    press(&mut game, 22);
    assert_eq!(byte(&game, 0x45e5), 4, "right faces the roach right");
    assert_eq!(word(&game, 0x457f), 64, "the wall blocks the roach right");
    press(&mut game, 23);
    assert_eq!(byte(&game, 0x45e5), 5, "left faces the roach left");
    assert_eq!(word(&game, 0x457f), 64, "the wall blocks the roach left");
    assert_eq!(
        word(&game, 0x4583),
        5,
        "the chaser has not reached the roach"
    );

    let mut game = open_stage();
    let (start_x, start_y) = (word(&game, 0x4577), word(&game, 0x4579));
    advance(&mut game, 200);
    let (chase_x, chase_y) = (word(&game, 0x4577), word(&game, 0x4579));
    assert!(chase_x < start_x, "the chaser closes in horizontally");
    assert!(chase_y > start_y, "the chaser closes in vertically");
    assert_eq!(word(&game, 0x457f), 64, "the idle roach keeps its cell");
    assert_eq!(word(&game, 0x4581), 24, "the idle roach keeps its cell");

    let mut game = open_stage();
    let mut frames = 0;
    while word(&game, 0x4583) == 5 && frames < 1500 {
        advance(&mut game, 1);
        frames += 1;
    }
    assert_eq!(word(&game, 0x4583), 4, "the chaser takes one life");
    advance(&mut game, 30);
    assert!(
        word(&game, 0x4577) > 120,
        "the chaser restarts on its marker cell"
    );
    assert!(
        word(&game, 0x4579) < 16,
        "the chaser restarts on its marker cell"
    );
    assert_eq!(word(&game, 0x457f), 64, "the roach keeps its start cell");
    assert_eq!(word(&game, 0x4581), 24, "the roach keeps its start cell");
    assert_eq!(byte(&game, 0x45e7), 0, "the stage stays selected");
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn roach_adventure_platinum_runs_levels_and_chase() {
    let path = "LAVA1 蟑螂历险记/白金版/蟑螂历险记V3.0.lav";
    let byte = |game: &Emulator, address: usize| game.vm().memory()[address];
    let word = |game: &Emulator, address: usize| {
        u16::from_le_bytes(game.vm().memory()[address..address + 2].try_into().unwrap())
    };
    let open_levels = || {
        let mut game = load(path);
        advance(&mut game, 4300);
        assert_eq!(game.vm().pc(), 0x2935, "the title waits for enter");
        press(&mut game, 13);
        advance(&mut game, 600);
        assert_eq!(game.vm().pc(), 0x2fc8, "the menu animates in");
        press(&mut game, 13);
        advance(&mut game, 80);
        assert_eq!(game.vm().pc(), 0x305d, "the menu settles");
        press(&mut game, 13);
        advance(&mut game, 2000);
        assert_eq!(game.vm().pc(), 0x370d, "the level select waits for a key");
        assert_eq!(
            byte(&game, 0x47c7),
            0,
            "the level select starts on the first stage"
        );
        game
    };
    let open_stage = || {
        let mut game = open_levels();
        hold(&mut game, 13, 3);
        game
    };

    let mut game = open_levels();
    for _ in 0..3 {
        press(&mut game, 22);
    }
    assert_eq!(byte(&game, 0x47c7), 3, "right walks the stage cursor up");
    press(&mut game, 23);
    press(&mut game, 23);
    assert_eq!(byte(&game, 0x47c7), 1, "left walks the stage cursor back");
    press(&mut game, 21);
    assert_eq!(byte(&game, 0x47c7), 0, "down walks the stage cursor back");
    press(&mut game, 20);
    assert_eq!(byte(&game, 0x47c7), 1, "up walks the stage cursor up");
    press(&mut game, 21);
    assert_eq!(byte(&game, 0x47c7), 0, "down returns to the first stage");
    for _ in 0..12 {
        press(&mut game, 22);
    }
    assert_eq!(
        byte(&game, 0x47c7),
        9,
        "the stage cursor clamps on the last stage"
    );
    for _ in 0..12 {
        press(&mut game, 23);
    }
    assert_eq!(
        byte(&game, 0x47c7),
        0,
        "the stage cursor clamps on the first stage"
    );
    assert!(
        game.files_mut().take_dirty_files().is_empty(),
        "the menus write nothing"
    );

    let mut game = open_stage();
    assert_eq!(word(&game, 0x4763), 5, "the stage starts with five lives");
    assert_eq!(
        word(&game, 0x4765),
        5,
        "the player marker row comes from the map"
    );
    assert_eq!(
        word(&game, 0x4767),
        10,
        "the player marker column comes from the map"
    );
    assert_eq!(
        word(&game, 0x4769),
        1,
        "the chaser marker row comes from the map"
    );
    assert_eq!(
        word(&game, 0x476b),
        18,
        "the chaser marker column comes from the map"
    );
    assert_eq!(
        word(&game, 0x475f),
        72,
        "the roach starts on its marker cell"
    );
    assert_eq!(
        word(&game, 0x4761),
        32,
        "the roach starts on its marker cell"
    );
    assert!(
        (130..=136).contains(&word(&game, 0x4757)),
        "the chaser starts on its marker cell"
    );
    assert!(
        word(&game, 0x4759) <= 4,
        "the chaser starts on its marker cell"
    );
    assert_eq!(word(&game, 0x47c9), 56, "the exit comes from the map");
    assert_eq!(word(&game, 0x47cb), 32, "the exit comes from the map");
    assert_eq!(word(&game, 0x47b9), 2, "the map holds two extra markers");
    assert_eq!(byte(&game, 0x47c7), 0, "the stage keeps the selected level");

    press(&mut game, 20);
    assert_eq!(word(&game, 0x4761), 32, "the wall blocks the roach up");
    assert_eq!(word(&game, 0x475f), 72, "the wall keeps the roach column");
    press(&mut game, 21);
    assert_eq!(word(&game, 0x4761), 32, "the wall blocks the roach down");
    press(&mut game, 23);
    assert_eq!(word(&game, 0x475f), 72, "the wall blocks the roach left");
    press(&mut game, 22);
    assert_eq!(word(&game, 0x475f), 76, "right moves the roach four pixels");
    assert_eq!(word(&game, 0x4761), 32, "right keeps the roach row");

    let mut game = open_stage();
    let (start_x, start_y) = (word(&game, 0x4757), word(&game, 0x4759));
    advance(&mut game, 200);
    let (chase_x, chase_y) = (word(&game, 0x4757), word(&game, 0x4759));
    assert!(chase_x < start_x, "the chaser closes in horizontally");
    assert!(chase_y > start_y, "the chaser closes in vertically");
    assert_eq!(word(&game, 0x475f), 72, "the idle roach keeps its cell");
    assert_eq!(word(&game, 0x4761), 32, "the idle roach keeps its cell");
    assert_eq!(
        word(&game, 0x4763),
        5,
        "the chaser has not reached the roach"
    );

    let mut game = open_stage();
    let mut frames = 0;
    while word(&game, 0x4763) == 5 && frames < 1400 {
        advance(&mut game, 1);
        frames += 1;
    }
    assert_eq!(word(&game, 0x4763), 4, "the chaser takes one life");
    advance(&mut game, 10);
    assert!(
        word(&game, 0x4757) > 130,
        "the chaser restarts on its marker cell"
    );
    assert!(
        word(&game, 0x4759) < 8,
        "the chaser restarts on its marker cell"
    );
    assert_eq!(word(&game, 0x475f), 72, "the roach keeps its start cell");
    assert_eq!(word(&game, 0x4761), 32, "the roach keeps its start cell");
    assert_eq!(byte(&game, 0x47c7), 0, "the stage stays selected");
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn roach_adventure_compact_menus_help_and_missing_map() {
    let path = "LAVA1 蟑螂历险记/精简版/蟑螂历险记V2.0.lav";
    let byte = |game: &Emulator, address: usize| game.vm().memory()[address];
    let local =
        |game: &Emulator, index: usize| game.vm().memory()[game.vm().local_bp() as usize + index];
    let open_title = || {
        let mut game = load(path);
        advance(&mut game, 120);
        assert!(
            matches!(game.vm().pc(), 0x8df | 0x8e2),
            "the title waits in its delay loop"
        );
        assert_eq!(byte(&game, 0x23fb), 1, "the menu starts on start game");
        game
    };

    let mut game = open_title();
    press(&mut game, 22);
    assert_eq!(byte(&game, 0x23fb), 2, "right selects hotline help");
    press(&mut game, 22);
    assert_eq!(byte(&game, 0x23fb), 2, "the second column ignores right");
    press(&mut game, 21);
    assert_eq!(byte(&game, 0x23fb), 4, "down selects version info");
    press(&mut game, 20);
    assert_eq!(byte(&game, 0x23fb), 2, "up returns to hotline help");
    press(&mut game, 23);
    assert_eq!(byte(&game, 0x23fb), 1, "left returns to start game");
    press(&mut game, 21);
    assert_eq!(byte(&game, 0x23fb), 3, "down selects quit");
    press(&mut game, 23);
    assert_eq!(byte(&game, 0x23fb), 3, "the first column ignores left");
    press(&mut game, 20);
    assert_eq!(byte(&game, 0x23fb), 1, "up returns to start game");

    let mut game = open_title();
    press(&mut game, 13);
    advance(&mut game, 700);
    assert!(
        matches!(game.vm().pc(), 0xc7d | 0xc80),
        "start game opens the level select"
    );
    assert_eq!(local(&game, 5), 1, "the level select starts on stage one");
    press(&mut game, 22);
    assert_eq!(local(&game, 5), 2, "right advances one stage");
    press(&mut game, 23);
    assert_eq!(local(&game, 5), 1, "left returns one stage");
    for _ in 0..12 {
        press(&mut game, 22);
    }
    assert_eq!(local(&game, 5), 10, "the level select stops at stage ten");
    for _ in 0..9 {
        press(&mut game, 23);
    }
    assert_eq!(local(&game, 5), 1, "the level select stops at stage one");
    assert!(
        game.files_mut().take_dirty_files().is_empty(),
        "the level select writes no files"
    );

    press(&mut game, 13);
    advance(&mut game, 60);
    assert_eq!(
        game.vm().pc(),
        0xd2b,
        "the stage reports that the renamed map file is missing"
    );
    press(&mut game, 13);
    advance(&mut game, 600);
    assert!(
        matches!(game.vm().pc(), 0x5a4 | 0x5a7),
        "the missing map page leads to the credits page"
    );

    let mut game = open_title();
    press(&mut game, 22);
    press(&mut game, 13);
    advance(&mut game, 1200);
    assert_eq!(game.vm().pc(), 0x297, "the help page waits for a key");
    press(&mut game, 13);
    assert_eq!(game.vm().pc(), 0x299, "the help page asks for a second key");
    press(&mut game, 13);
    advance(&mut game, 1500);
    assert!(
        matches!(game.vm().pc(), 0x8df | 0x8e2),
        "the help returns to the title"
    );
    assert_eq!(byte(&game, 0x23fb), 1, "the title resets the menu");

    let mut game = open_title();
    press(&mut game, 21);
    press(&mut game, 22);
    press(&mut game, 13);
    advance(&mut game, 1200);
    assert!(
        matches!(game.vm().pc(), 0x5a4 | 0x5a7),
        "version info opens the credits page"
    );
    press(&mut game, 27);
    advance(&mut game, 1500);
    assert!(
        matches!(game.vm().pc(), 0x8df | 0x8e2),
        "escape returns from the credits to the title"
    );

    let mut game = open_title();
    press(&mut game, 21);
    press(&mut game, 13);
    advance(&mut game, 1200);
    assert!(
        matches!(game.vm().pc(), 0x5a4 | 0x5a7),
        "quit opens the credits page"
    );
    press(&mut game, 27);
    let mut frames = 0;
    let mut halted = None;
    while frames < 1200 {
        if let FrameStatus::Halted(code) = game.run_frame().unwrap().status {
            halted = Some(code);
            break;
        }
        frames += 1;
    }
    assert_eq!(halted, Some(0), "escape exits from the quit credits");
}

#[test]
#[ignore = "requires the local game corpus via LAVAX_GAME_ROOT"]
fn roach_adventure_final_menus_levels_and_chase() {
    let byte = |game: &Emulator, address: usize| game.vm().memory()[address];
    let word = |game: &Emulator, address: usize| {
        u16::from_le_bytes(game.vm().memory()[address..address + 2].try_into().unwrap())
    };
    let cell =
        |game: &Emulator, row: usize, column: usize| word(game, 0x44dc + 2 * (row * 20 + column));
    for path in [
        "LAVA1 蟑螂历险记/终结完美版/蟑螂历险记V3.2.lav",
        "LAVA1 蟑螂历险记3.2/蟑螂历险记V3.2.lav",
    ] {
        let open_menu = || {
            let mut game = load(path);
            advance(&mut game, 2300);
            assert_eq!(game.vm().pc(), 0x2935, "the title waits for enter");
            press(&mut game, 13);
            advance(&mut game, 700);
            assert_eq!(game.vm().pc(), 0x32e6, "the 2x2 menu waits for a key");
            assert_eq!(byte(&game, 0x47cb), 1, "the menu starts on start game");
            game
        };
        let open_levels = || {
            let mut game = open_menu();
            press(&mut game, 13);
            advance(&mut game, 600);
            assert_eq!(game.vm().pc(), 0x38ec, "start game opens the level select");
            assert_eq!(
                byte(&game, 0x47eb),
                1,
                "the level select starts on stage one"
            );
            game
        };
        let open_stage = || {
            let mut game = open_levels();
            hold(&mut game, 13, 3);
            assert_eq!(game.vm().pc(), 0x3ae4, "the first stage enters its loop");
            game
        };

        let mut game = open_menu();
        press(&mut game, 20);
        assert_eq!(byte(&game, 0x47cb), 1, "up clamps on the top row");
        press(&mut game, 21);
        assert_eq!(byte(&game, 0x47cb), 3, "down selects quit");
        press(&mut game, 21);
        assert_eq!(byte(&game, 0x47cb), 3, "down clamps on the bottom row");
        press(&mut game, 22);
        assert_eq!(byte(&game, 0x47cb), 4, "right selects version info");
        press(&mut game, 22);
        assert_eq!(byte(&game, 0x47cb), 4, "right clamps on the second column");
        press(&mut game, 20);
        assert_eq!(byte(&game, 0x47cb), 2, "up selects hotline help");
        press(&mut game, 20);
        assert_eq!(byte(&game, 0x47cb), 2, "up clamps on the top row");
        press(&mut game, 23);
        assert_eq!(byte(&game, 0x47cb), 1, "left returns to start game");

        let mut game = open_levels();
        press(&mut game, 22);
        assert_eq!(byte(&game, 0x47eb), 2, "right advances one stage");
        press(&mut game, 21);
        assert_eq!(byte(&game, 0x47eb), 1, "down returns one stage");
        press(&mut game, 20);
        assert_eq!(byte(&game, 0x47eb), 2, "up advances one stage");
        press(&mut game, 23);
        assert_eq!(byte(&game, 0x47eb), 1, "left returns one stage");
        press(&mut game, 23);
        assert_eq!(byte(&game, 0x47eb), 1, "left clamps on stage one");
        for _ in 0..12 {
            press(&mut game, 20);
        }
        assert_eq!(byte(&game, 0x47eb), 10, "up clamps on stage ten");
        for _ in 0..12 {
            press(&mut game, 21);
        }
        assert_eq!(byte(&game, 0x47eb), 1, "down walks back to stage one");
        assert!(
            game.files_mut().take_dirty_files().is_empty(),
            "the menus write nothing"
        );

        let mut game = open_stage();
        assert_eq!(word(&game, 0x4763), 5, "the stage starts with five lives");
        assert_eq!(
            word(&game, 0x4765),
            10,
            "the player marker row comes from the map"
        );
        assert_eq!(
            word(&game, 0x4767),
            1,
            "the player marker column comes from the map"
        );
        assert_eq!(
            word(&game, 0x4769),
            1,
            "the chaser marker row comes from the map"
        );
        assert_eq!(
            word(&game, 0x476b),
            18,
            "the chaser marker column comes from the map"
        );
        assert_eq!(
            word(&game, 0x475f),
            0,
            "the roach starts on its marker cell"
        );
        assert_eq!(
            word(&game, 0x4761),
            72,
            "the roach starts on its marker cell"
        );
        assert_eq!(cell(&game, 0, 0), 1, "the map border is a wall");
        assert_eq!(cell(&game, 0, 19), 1, "the map border is a wall");
        assert_eq!(cell(&game, 1, 0), 1, "the map border is a wall");
        assert_eq!(
            cell(&game, 1, 18),
            0,
            "the chaser marker cell is open floor"
        );
        assert_eq!(cell(&game, 10, 0), 1, "the map border is a wall");
        assert_eq!(
            cell(&game, 10, 1),
            0,
            "the player marker cell is open floor"
        );
        assert_eq!(cell(&game, 10, 2), 2, "a wall sits right of the start cell");
        assert_eq!(cell(&game, 10, 19), 1, "the map border is a wall");

        press(&mut game, 20);
        assert_eq!(word(&game, 0x4761), 70, "up moves the roach two pixels");
        press(&mut game, 20);
        assert_eq!(word(&game, 0x4761), 68, "up moves the roach two pixels");
        press(&mut game, 21);
        assert_eq!(word(&game, 0x4761), 72, "down moves the roach two pixels");
        press(&mut game, 22);
        assert_eq!(word(&game, 0x475f), 0, "the wall blocks the roach right");
        press(&mut game, 23);
        assert_eq!(word(&game, 0x475f), 0, "the wall blocks the roach left");
        assert_eq!(word(&game, 0x4761), 72, "the roach keeps its row");

        let mut game = open_stage();
        advance(&mut game, 200);
        assert_eq!(
            word(&game, 0x4757),
            123,
            "the chaser closes in horizontally"
        );
        assert_eq!(word(&game, 0x4759), 14, "the chaser closes in vertically");
        assert_eq!(word(&game, 0x475f), 0, "the idle roach keeps its cell");
        assert_eq!(word(&game, 0x4761), 72, "the idle roach keeps its cell");
        assert_eq!(
            word(&game, 0x4763),
            5,
            "the chaser has not reached the roach"
        );

        let mut game = open_stage();
        let mut frames = 0;
        while word(&game, 0x4763) == 5 && frames < 2400 {
            advance(&mut game, 1);
            frames += 1;
        }
        assert_eq!(word(&game, 0x4763), 4, "the chaser takes one life");
        assert_eq!(
            word(&game, 0x4757),
            136,
            "the chaser restarts on its marker cell"
        );
        assert_eq!(
            word(&game, 0x4759),
            0,
            "the chaser restarts on its marker cell"
        );
        assert_eq!(word(&game, 0x475f), 0, "the roach keeps its start cell");
        assert_eq!(word(&game, 0x4761), 72, "the roach keeps its start cell");

        let mut game = open_menu();
        press(&mut game, 22);
        assert_eq!(byte(&game, 0x47cb), 2, "right selects hotline help");
        press(&mut game, 13);
        advance(&mut game, 600);
        assert_eq!(game.vm().pc(), 0x297c, "the help page waits for a key");
        press(&mut game, 13);
        assert_eq!(
            game.vm().pc(),
            0x297e,
            "the help page asks for a second key"
        );
        press(&mut game, 13);
        assert_eq!(game.vm().pc(), 0x29b8, "the help page starts to close");
        advance(&mut game, 1200);
        assert_eq!(game.vm().pc(), 0x32e6, "the help returns to the menu");
        assert_eq!(byte(&game, 0x47cb), 1, "the menu resets its selection");

        let mut game = open_menu();
        press(&mut game, 22);
        press(&mut game, 21);
        assert_eq!(byte(&game, 0x47cb), 4, "down selects version info");
        press(&mut game, 13);
        assert_eq!(game.vm().pc(), 0x2a31, "version info opens the credits");
        advance(&mut game, 600);
        assert_eq!(game.vm().pc(), 0x2d77, "the credits wait for escape");
        press(&mut game, 27);
        advance(&mut game, 1200);
        assert_eq!(game.vm().pc(), 0x32e6, "escape returns to the menu");
        assert_eq!(byte(&game, 0x47cb), 1, "the menu resets its selection");

        let mut game = open_menu();
        press(&mut game, 21);
        press(&mut game, 13);
        advance(&mut game, 600);
        assert_eq!(game.vm().pc(), 0x2d77, "the quit entry opens the credits");
        press(&mut game, 27);
        let mut frames = 0;
        let mut halted = None;
        while frames < 600 {
            if let FrameStatus::Halted(code) = game.run_frame().unwrap().status {
                halted = Some(code);
                break;
            }
            frames += 1;
        }
        assert_eq!(halted, Some(0), "escape exits from the quit credits");
    }
}
