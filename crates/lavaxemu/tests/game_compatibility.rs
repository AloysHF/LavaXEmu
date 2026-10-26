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
    advance(emulator, 1);
    emulator.input_mut().set_key(key, false);
    advance(emulator, 30);
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
