use std::ffi::{c_char, c_void};

pub const API_VERSION: u32 = 1;
pub const REGION_NTSC: u32 = 0;

pub const ENVIRONMENT_SET_PERFORMANCE_LEVEL: u32 = 8;
pub const ENVIRONMENT_SET_PIXEL_FORMAT: u32 = 10;
pub const ENVIRONMENT_SET_INPUT_DESCRIPTORS: u32 = 11;
pub const ENVIRONMENT_SET_SUPPORT_NO_GAME: u32 = 18;

pub const PIXEL_FORMAT_RGB565: u32 = 2;
pub const DEVICE_NONE: u32 = 0;
pub const DEVICE_JOYPAD: u32 = 1;
pub const DEVICE_POINTER: u32 = 6;

pub const JOYPAD_B: u32 = 0;
pub const JOYPAD_Y: u32 = 1;
pub const JOYPAD_SELECT: u32 = 2;
pub const JOYPAD_START: u32 = 3;
pub const JOYPAD_UP: u32 = 4;
pub const JOYPAD_DOWN: u32 = 5;
pub const JOYPAD_LEFT: u32 = 6;
pub const JOYPAD_RIGHT: u32 = 7;
pub const JOYPAD_A: u32 = 8;
pub const JOYPAD_X: u32 = 9;
pub const JOYPAD_L: u32 = 10;
pub const JOYPAD_R: u32 = 11;

pub const POINTER_X: u32 = 0;
pub const POINTER_Y: u32 = 1;
pub const POINTER_PRESSED: u32 = 2;

pub const MEMORY_SYSTEM_RAM: u32 = 2;

pub type EnvironmentCallback = Option<unsafe extern "C" fn(u32, *mut c_void) -> bool>;
pub type VideoRefreshCallback = Option<unsafe extern "C" fn(*const c_void, u32, u32, usize)>;
pub type AudioSampleCallback = Option<unsafe extern "C" fn(i16, i16)>;
pub type AudioSampleBatchCallback = Option<unsafe extern "C" fn(*const i16, usize) -> usize>;
pub type InputPollCallback = Option<unsafe extern "C" fn()>;
pub type InputStateCallback = Option<unsafe extern "C" fn(u32, u32, u32, u32) -> i16>;

#[repr(C)]
pub struct SystemInfo {
    pub library_name: *const c_char,
    pub library_version: *const c_char,
    pub valid_extensions: *const c_char,
    pub need_fullpath: bool,
    pub block_extract: bool,
}

#[repr(C)]
pub struct GameGeometry {
    pub base_width: u32,
    pub base_height: u32,
    pub max_width: u32,
    pub max_height: u32,
    pub aspect_ratio: f32,
}

#[repr(C)]
pub struct SystemTiming {
    pub fps: f64,
    pub sample_rate: f64,
}

#[repr(C)]
pub struct SystemAvInfo {
    pub geometry: GameGeometry,
    pub timing: SystemTiming,
}

#[repr(C)]
pub struct GameInfo {
    pub path: *const c_char,
    pub data: *const c_void,
    pub size: usize,
    pub meta: *const c_char,
}

#[repr(C)]
pub struct InputDescriptor {
    pub port: u32,
    pub device: u32,
    pub index: u32,
    pub id: u32,
    pub description: *const c_char,
}
