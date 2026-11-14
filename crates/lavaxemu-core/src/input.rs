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
                self.queued.clear();
                self.queued.push_back(key);
            }
        } else {
            self.pressed.remove(&key);
        }
    }

    pub fn set_keys(&mut self, keys: impl IntoIterator<Item = u8>) {
        let next: BTreeSet<u8> = keys.into_iter().map(|key| key & 0x7f).collect();
        if next.difference(&self.pressed).next().is_some() {
            self.queued.clear();
        }
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
        let key = key & 0x7f;
        self.pressed.remove(&key);
        self.queued.retain(|queued| *queued != key);
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
    fn guest_release_clears_only_its_pending_key_and_physical_release_keeps_it() {
        let mut input = InputState::default();
        input.set_keys([13]);
        input.set_keys([]);
        assert_eq!(input.pop_key(), Some(13));
        input.set_keys([13]);
        input.set_keys([]);
        input.set_keys([25]);
        assert_eq!(input.pop_key(), Some(25));
        assert_eq!(input.pop_key(), None);
        input.set_keys([]);
        input.set_keys([13, 25]);
        input.release(13);
        assert!(!input.is_pressed(13));
        assert!(input.is_pressed(25));
        assert_eq!(input.pop_key(), Some(25));
        assert_eq!(input.pop_key(), None);
    }

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
