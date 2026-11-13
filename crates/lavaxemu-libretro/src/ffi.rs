use std::ffi::{c_char, c_void};

pub const API_VERSION: u32 = 1;
pub const REGION_NTSC: u32 = 0;

pub const ENVIRONMENT_SET_MESSAGE: u32 = 6;
pub const ENVIRONMENT_SET_PERFORMANCE_LEVEL: u32 = 8;
pub const ENVIRONMENT_SET_PIXEL_FORMAT: u32 = 10;
pub const ENVIRONMENT_SET_INPUT_DESCRIPTORS: u32 = 11;
pub const ENVIRONMENT_GET_VARIABLE: u32 = 15;
pub const ENVIRONMENT_GET_VARIABLE_UPDATE: u32 = 17;
pub const ENVIRONMENT_SET_SUPPORT_NO_GAME: u32 = 18;
#[cfg(not(target_os = "emscripten"))]
pub const ENVIRONMENT_GET_LOG_INTERFACE: u32 = 27;
pub const ENVIRONMENT_SET_CORE_OPTIONS: u32 = 56;
pub const ENVIRONMENT_SET_CORE_OPTIONS_V2: u32 = 60;

pub const LOG_DEBUG: u32 = 0;
pub const LOG_INFO: u32 = 1;
pub const LOG_WARN: u32 = 2;
pub const LOG_ERROR: u32 = 3;

pub const PIXEL_FORMAT_RGB565: u32 = 2;
pub const DEVICE_NONE: u32 = 0;
pub const DEVICE_JOYPAD: u32 = 1;
pub const DEVICE_KEYBOARD: u32 = 3;
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
/// Frontend log sink obtained through `RETRO_ENVIRONMENT_GET_LOG_INTERFACE`.
///
/// libretro declares it C-variadic, but over wasm the callee signature carries
/// a hidden varargs pointer, so calling it from Rust traps with "function
/// signature mismatch". Native builds call it with the message as the format
/// string and no arguments; Emscripten builds log to stderr instead and never
/// touch the callback.
#[cfg(not(target_os = "emscripten"))]
pub type LogCallback = Option<unsafe extern "C" fn(u32, *const c_char)>;

/// Sink the frontend fills in for `RETRO_ENVIRONMENT_GET_LOG_INTERFACE`.
#[cfg(not(target_os = "emscripten"))]
#[repr(C)]
pub struct LogCallbackStruct {
    pub log: LogCallback,
}

#[repr(C)]
pub struct Message {
    pub msg: *const c_char,
    /// Duration in frames.
    pub frames: u32,
}

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

/// Number of value slots per core option definition (libretro constant).
pub const CORE_OPTION_VALUES_MAX: usize = 128;

// The core-options structs are plain-data FFI records. The core only ever
// hands the frontend pointers to `'static` instances of them, so sharing them
// across threads is sound; the raw pointers would otherwise block `static`.
unsafe impl Sync for CoreOptionValue {}
unsafe impl Sync for CoreOptionDefinition {}
unsafe impl Sync for CoreOptionV2Category {}
unsafe impl Sync for CoreOptionV2Definition {}
unsafe impl Sync for CoreOptions {}
unsafe impl Sync for CoreOptionsV2 {}

/// Key/value pair queried through `RETRO_ENVIRONMENT_GET_VARIABLE`.
#[repr(C)]
pub struct Variable {
    pub key: *const c_char,
    pub value: *const c_char,
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct CoreOptionValue {
    pub value: *const c_char,
    pub label: *const c_char,
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct CoreOptionDefinition {
    pub key: *const c_char,
    pub desc: *const c_char,
    pub info: *const c_char,
    pub values: [CoreOptionValue; CORE_OPTION_VALUES_MAX],
    pub default_value: *const c_char,
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct CoreOptionV2Category {
    pub key: *const c_char,
    pub desc: *const c_char,
    pub info: *const c_char,
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct CoreOptionV2Definition {
    pub key: *const c_char,
    pub desc: *const c_char,
    pub info: *const c_char,
    pub category_key: *const c_char,
    pub values: [CoreOptionValue; CORE_OPTION_VALUES_MAX],
    pub default_value: *const c_char,
}

/// `retro_core_options`: the v1 core-options interface.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct CoreOptions {
    pub definitions: *const CoreOptionDefinition,
    pub num_definitions: usize,
}

/// `retro_core_options_v2`: the v2 core-options interface.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct CoreOptionsV2 {
    pub categories: *const CoreOptionV2Category,
    pub definitions: *const CoreOptionV2Definition,
    pub num_categories: usize,
    pub num_definitions: usize,
}
