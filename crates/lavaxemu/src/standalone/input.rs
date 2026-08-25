use lavaxemu_core::{InputState, PointerState};
use minifb::{Key, MouseButton, MouseMode, Window};

const LETTERS: &[(Key, u8)] = &[
    (Key::A, b'a'),
    (Key::B, b'b'),
    (Key::C, b'c'),
    (Key::D, b'd'),
    (Key::E, b'e'),
    (Key::F, b'f'),
    (Key::G, b'g'),
    (Key::H, b'h'),
    (Key::I, b'i'),
    (Key::J, b'j'),
    (Key::K, b'k'),
    (Key::L, b'l'),
    (Key::M, b'm'),
    (Key::N, b'n'),
    (Key::O, b'o'),
    (Key::P, b'p'),
    (Key::Q, b'q'),
    (Key::R, b'r'),
    (Key::S, b's'),
    (Key::T, b't'),
    (Key::U, b'u'),
    (Key::V, b'v'),
    (Key::W, b'w'),
    (Key::X, b'x'),
    (Key::Y, b'y'),
    (Key::Z, b'z'),
];

const SPECIAL_KEYS: &[(Key, u8)] = &[
    (Key::Up, 20),
    (Key::Down, 21),
    (Key::Right, 22),
    (Key::Left, 23),
    (Key::PageUp, 19),
    (Key::PageDown, 14),
    (Key::Enter, 13),
    (Key::Escape, 27),
    (Key::Space, b'b'),
    (Key::Tab, 19),
    (Key::Backspace, 14),
    (Key::LeftShift, 26),
    (Key::F1, 0x1c),
    (Key::F2, 0x1d),
    (Key::F3, 0x1e),
    (Key::F4, 0x1f),
    (Key::F5, 25),
    (Key::F6, 18),
];

const NUMBER_KEYS: &[(Key, u8)] = &[
    (Key::Key1, b'b'),
    (Key::Key2, b'n'),
    (Key::Key3, b'm'),
    (Key::Key4, b'g'),
    (Key::Key5, b'h'),
    (Key::Key6, b'j'),
    (Key::Key7, b't'),
    (Key::Key8, b'y'),
    (Key::Key9, b'u'),
];

pub fn update_input(window: &Window, input: &mut InputState, width: usize, height: usize) {
    let mut keys = Vec::new();
    for &(physical, guest) in LETTERS.iter().chain(NUMBER_KEYS).chain(SPECIAL_KEYS) {
        if window.is_key_down(physical) {
            keys.push(guest);
        }
    }
    input.set_keys(keys);

    if let Some((x, y)) = window.get_mouse_pos(MouseMode::Clamp) {
        let (window_width, window_height) = window.get_size();
        input.set_pointer(Some(PointerState {
            x: ((x * width as f32 / window_width as f32) as usize).min(width - 1) as i16,
            y: ((y * height as f32 / window_height as f32) as usize).min(height - 1) as i16,
            pressed: window.get_mouse_down(MouseButton::Left),
        }));
    } else {
        input.set_pointer(None);
    }
}
