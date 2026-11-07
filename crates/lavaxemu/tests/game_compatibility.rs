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
