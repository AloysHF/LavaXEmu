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
    emulator.input_mut().set_key(key, true);
    advance(emulator, 3);
    emulator.input_mut().set_key(key, false);
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
