use std::collections::{BTreeSet, VecDeque};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PointerState {
    pub x: i16,
    pub y: i16,
    pub pressed: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InputState {
    pressed: BTreeSet<u8>,
    queued: VecDeque<u8>,
    pointer: Option<PointerState>,
}

impl InputState {
    pub fn set_key(&mut self, key: u8, pressed: bool) {
        let key = key & 0x7f;
        if pressed {
            if self.pressed.insert(key) {
                self.queued.push_back(key);
            }
        } else {
            self.pressed.remove(&key);
        }
    }

    pub fn set_keys(&mut self, keys: impl IntoIterator<Item = u8>) {
        let next: BTreeSet<u8> = keys.into_iter().map(|key| key & 0x7f).collect();
        for &key in next.difference(&self.pressed) {
            self.queued.push_back(key);
        }
        self.pressed = next;
    }

    pub fn is_pressed(&self, key: u8) -> bool {
        self.pressed.contains(&(key & 0x7f))
    }

    pub fn first_pressed(&self) -> Option<u8> {
        self.pressed.first().copied()
    }

    pub fn pop_key(&mut self) -> Option<u8> {
        self.queued.pop_front()
    }

    pub fn release(&mut self, key: u8) {
        self.pressed.remove(&(key & 0x7f));
    }

    pub fn release_all(&mut self) {
        self.pressed.clear();
        self.queued.clear();
    }

    pub fn set_pointer(&mut self, pointer: Option<PointerState>) {
        self.pointer = pointer;
    }

    pub const fn pointer(&self) -> Option<PointerState> {
        self.pointer
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queues_only_new_key_presses() {
        let mut input = InputState::default();
        input.set_keys(*b"AB");
        assert_eq!(input.pop_key(), Some(b'A'));
        assert_eq!(input.pop_key(), Some(b'B'));
        input.set_keys(*b"AB");
        assert_eq!(input.pop_key(), None);
        input.set_keys(*b"BC");
        assert_eq!(input.pop_key(), Some(b'C'));
    }
}
