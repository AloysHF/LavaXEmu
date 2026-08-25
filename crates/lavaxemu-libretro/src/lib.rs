mod ffi;

use std::{
    ffi::{CStr, c_char, c_void},
    fs,
    path::{Path, PathBuf},
    ptr,
    sync::{Mutex, MutexGuard},
};

use ffi::*;
use lavaxemu_core::{Emulator, PointerState, Program};

const FRAMES_PER_SECOND: f64 = 60.0;
const AUDIO_SAMPLE_RATE: f64 = 44_100.0;
const AUDIO_FRAMES_PER_VIDEO_FRAME: usize = 735;
const SERIALIZED_STATE_CAPACITY: usize = 64 * 1024 * 1024 + 14;

#[derive(Default)]
struct Callbacks {
    environment: EnvironmentCallback,
    video_refresh: VideoRefreshCallback,
    audio_sample: AudioSampleCallback,
    audio_sample_batch: AudioSampleBatchCallback,
    input_poll: InputPollCallback,
    input_state: InputStateCallback,
}

#[derive(Default)]
struct Core {
    callbacks: Callbacks,
    emulator: Option<Emulator>,
    video: Vec<u16>,
}

static CORE: Mutex<Core> = Mutex::new(Core {
    callbacks: Callbacks {
        environment: None,
        video_refresh: None,
        audio_sample: None,
        audio_sample_batch: None,
        input_poll: None,
        input_state: None,
    },
    emulator: None,
    video: Vec::new(),
});

fn core() -> MutexGuard<'static, Core> {
    CORE.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[unsafe(no_mangle)]
pub extern "C" fn retro_set_environment(callback: EnvironmentCallback) {
    core().callbacks.environment = callback;
    let mut support_no_game = false;
    call_environment(
        callback,
        ENVIRONMENT_SET_SUPPORT_NO_GAME,
        ptr::from_mut(&mut support_no_game).cast(),
    );
}

#[unsafe(no_mangle)]
pub extern "C" fn retro_set_video_refresh(callback: VideoRefreshCallback) {
    core().callbacks.video_refresh = callback;
}

#[unsafe(no_mangle)]
pub extern "C" fn retro_set_audio_sample(callback: AudioSampleCallback) {
    core().callbacks.audio_sample = callback;
}

#[unsafe(no_mangle)]
pub extern "C" fn retro_set_audio_sample_batch(callback: AudioSampleBatchCallback) {
    core().callbacks.audio_sample_batch = callback;
}

#[unsafe(no_mangle)]
pub extern "C" fn retro_set_input_poll(callback: InputPollCallback) {
    core().callbacks.input_poll = callback;
}

#[unsafe(no_mangle)]
pub extern "C" fn retro_set_input_state(callback: InputStateCallback) {
    core().callbacks.input_state = callback;
}

#[unsafe(no_mangle)]
pub extern "C" fn retro_init() {}

#[unsafe(no_mangle)]
pub extern "C" fn retro_deinit() {
    let mut core = core();
    core.emulator = None;
    core.video.clear();
}

#[unsafe(no_mangle)]
pub extern "C" fn retro_api_version() -> u32 {
    API_VERSION
}

#[unsafe(no_mangle)]
/// # Safety
/// `info` must be null or point to writable `SystemInfo` storage.
pub unsafe extern "C" fn retro_get_system_info(info: *mut SystemInfo) {
    let Some(info) = (unsafe { info.as_mut() }) else {
        return;
    };
    *info = SystemInfo {
        library_name: c"LavaXEmu".as_ptr(),
        library_version: c"0.1.0".as_ptr(),
        valid_extensions: c"lav".as_ptr(),
        need_fullpath: true,
        block_extract: false,
    };
}

#[unsafe(no_mangle)]
/// # Safety
/// `info` must be null or point to writable `SystemAvInfo` storage.
pub unsafe extern "C" fn retro_get_system_av_info(info: *mut SystemAvInfo) {
    let Some(info) = (unsafe { info.as_mut() }) else {
        return;
    };
    let core = core();
    let (width, height) = core
        .emulator
        .as_ref()
        .map(|emulator| {
            (
                u32::from(emulator.display().width()),
                u32::from(emulator.display().height()),
            )
        })
        .unwrap_or((240, 160));
    *info = SystemAvInfo {
        geometry: GameGeometry {
            base_width: width,
            base_height: height,
            max_width: 320,
            max_height: 240,
            aspect_ratio: width as f32 / height as f32,
        },
        timing: SystemTiming {
            fps: FRAMES_PER_SECOND,
            sample_rate: AUDIO_SAMPLE_RATE,
        },
    };
}

#[unsafe(no_mangle)]
pub extern "C" fn retro_set_controller_port_device(port: u32, device: u32) {
    if port == 0 && device != DEVICE_NONE && device != DEVICE_JOYPAD {
        log::warn!("unsupported controller device {device} on port {port}");
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn retro_get_region() -> u32 {
    REGION_NTSC
}

#[unsafe(no_mangle)]
/// # Safety
/// `info` and its referenced fields must remain valid for this call.
pub unsafe extern "C" fn retro_load_game(info: *const GameInfo) -> bool {
    let Some(info) = (unsafe { info.as_ref() }) else {
        return false;
    };
    let path = game_path(info.path);
    let data = match game_data(info, path.as_deref()) {
        Ok(data) => data,
        Err(error) => {
            log::error!("failed to read content: {error}");
            return false;
        }
    };
    let program = match Program::load(&data) {
        Ok(program) => program,
        Err(error) => {
            log::error!("failed to load content: {error}");
            return false;
        }
    };

    let mut emulator = Emulator::new(program);
    if let Some(path) = path.as_deref()
        && let Err(error) = import_companion_files(path, emulator.files_mut())
    {
        log::warn!("failed to import companion files: {error}");
    }

    let mut core = core();
    if !configure_frontend(core.callbacks.environment) {
        return false;
    }
    core.video.clear();
    core.emulator = Some(emulator);
    true
}

#[unsafe(no_mangle)]
pub extern "C" fn retro_load_game_special(
    _game_type: u32,
    _info: *const GameInfo,
    _num_info: usize,
) -> bool {
    false
}

#[unsafe(no_mangle)]
pub extern "C" fn retro_unload_game() {
    let mut core = core();
    core.emulator = None;
    core.video.clear();
}

#[unsafe(no_mangle)]
pub extern "C" fn retro_run() {
    let mut core = core();
    if let Some(callback) = core.callbacks.input_poll {
        unsafe { callback() };
    }
    let input_state = core.callbacks.input_state;
    let keys = read_joypad(input_state);

    if core.emulator.is_none() {
        return;
    }
    let mut video = std::mem::take(&mut core.video);
    let emulator = core.emulator.as_mut().expect("emulator was checked");
    emulator.input_mut().set_keys(keys);
    let pointer = read_pointer(
        input_state,
        emulator.display().width(),
        emulator.display().height(),
    );
    emulator.input_mut().set_pointer(pointer);
    if let Err(error) = emulator.run_frame() {
        log::error!("frame execution failed: {error}");
    }

    let width = u32::from(emulator.display().width());
    let height = u32::from(emulator.display().height());
    let pixels = usize::try_from(width * height).expect("display dimensions fit usize");
    if video.len() != pixels {
        video.resize(pixels, 0);
    }
    emulator.display().to_rgb565(&mut video);
    core.video = video;

    if let Some(callback) = core.callbacks.video_refresh {
        unsafe {
            callback(
                core.video.as_ptr().cast(),
                width,
                height,
                width as usize * size_of::<u16>(),
            )
        };
    }
    emit_silence(&core.callbacks);
}

#[unsafe(no_mangle)]
pub extern "C" fn retro_reset() {
    if let Some(emulator) = core().emulator.as_mut() {
        emulator.reset();
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn retro_serialize_size() -> usize {
    if core().emulator.is_some() {
        SERIALIZED_STATE_CAPACITY
    } else {
        0
    }
}

#[unsafe(no_mangle)]
/// # Safety
/// `data` must point to at least `size` writable bytes.
pub unsafe extern "C" fn retro_serialize(data: *mut c_void, size: usize) -> bool {
    if data.is_null() || size < SERIALIZED_STATE_CAPACITY {
        return false;
    }
    let core = core();
    let Some(emulator) = core.emulator.as_ref() else {
        return false;
    };
    let state = match emulator.save_state() {
        Ok(state) => state,
        Err(error) => {
            log::error!("failed to serialize state: {error}");
            return false;
        }
    };
    if state.len() > size {
        return false;
    }
    let output = unsafe { std::slice::from_raw_parts_mut(data.cast::<u8>(), size) };
    output.fill(0);
    output[..state.len()].copy_from_slice(&state);
    true
}

#[unsafe(no_mangle)]
/// # Safety
/// `data` must point to at least `size` readable bytes.
pub unsafe extern "C" fn retro_unserialize(data: *const c_void, size: usize) -> bool {
    if data.is_null() || size < 14 {
        return false;
    }
    let input = unsafe { std::slice::from_raw_parts(data.cast::<u8>(), size) };
    let mut core = core();
    let Some(emulator) = core.emulator.as_mut() else {
        return false;
    };
    emulator.load_state(input).is_ok()
}

#[unsafe(no_mangle)]
pub extern "C" fn retro_cheat_reset() {}

#[unsafe(no_mangle)]
pub extern "C" fn retro_cheat_set(_index: u32, _enabled: bool, _code: *const c_char) {}

#[unsafe(no_mangle)]
pub extern "C" fn retro_get_memory_data(id: u32) -> *mut c_void {
    if id != MEMORY_SYSTEM_RAM {
        return ptr::null_mut();
    }
    core()
        .emulator
        .as_mut()
        .map_or(ptr::null_mut(), |emulator| {
            emulator.vm_mut().memory_mut().as_mut_ptr().cast()
        })
}

#[unsafe(no_mangle)]
pub extern "C" fn retro_get_memory_size(id: u32) -> usize {
    if id != MEMORY_SYSTEM_RAM {
        return 0;
    }
    core()
        .emulator
        .as_ref()
        .map_or(0, |emulator| emulator.vm().memory().len())
}

fn configure_frontend(environment: EnvironmentCallback) -> bool {
    let mut pixel_format = PIXEL_FORMAT_RGB565;
    if !call_environment(
        environment,
        ENVIRONMENT_SET_PIXEL_FORMAT,
        ptr::from_mut(&mut pixel_format).cast(),
    ) {
        log::error!("frontend rejected the required RGB565 pixel format");
        return false;
    }

    let mut performance_level = 1_u32;
    call_environment(
        environment,
        ENVIRONMENT_SET_PERFORMANCE_LEVEL,
        ptr::from_mut(&mut performance_level).cast(),
    );
    register_input_descriptors(environment);
    true
}

fn register_input_descriptors(environment: EnvironmentCallback) {
    let descriptors = [
        descriptor(JOYPAD_UP, c"Up"),
        descriptor(JOYPAD_DOWN, c"Down"),
        descriptor(JOYPAD_LEFT, c"Left"),
        descriptor(JOYPAD_RIGHT, c"Right"),
        descriptor(JOYPAD_A, c"B"),
        descriptor(JOYPAD_B, c"N"),
        descriptor(JOYPAD_X, c"M"),
        descriptor(JOYPAD_Y, c"G"),
        descriptor(JOYPAD_SELECT, c"H"),
        descriptor(JOYPAD_START, c"J"),
        descriptor(JOYPAD_L, c"Page Up"),
        descriptor(JOYPAD_R, c"Page Down"),
        InputDescriptor {
            port: 0,
            device: DEVICE_POINTER,
            index: 0,
            id: POINTER_PRESSED,
            description: c"Pointer".as_ptr(),
        },
        InputDescriptor {
            port: 0,
            device: 0,
            index: 0,
            id: 0,
            description: ptr::null(),
        },
    ];
    call_environment(
        environment,
        ENVIRONMENT_SET_INPUT_DESCRIPTORS,
        descriptors.as_ptr().cast_mut().cast(),
    );
}

fn descriptor(id: u32, description: &'static CStr) -> InputDescriptor {
    InputDescriptor {
        port: 0,
        device: DEVICE_JOYPAD,
        index: 0,
        id,
        description: description.as_ptr(),
    }
}

fn call_environment(callback: EnvironmentCallback, command: u32, data: *mut c_void) -> bool {
    callback.is_some_and(|callback| unsafe { callback(command, data) })
}

fn read_joypad(callback: InputStateCallback) -> Vec<u8> {
    let mappings = [
        (JOYPAD_UP, 20),
        (JOYPAD_DOWN, 21),
        (JOYPAD_RIGHT, 22),
        (JOYPAD_LEFT, 23),
        (JOYPAD_A, b'b'),
        (JOYPAD_B, b'n'),
        (JOYPAD_X, b'm'),
        (JOYPAD_Y, b'g'),
        (JOYPAD_SELECT, b'h'),
        (JOYPAD_START, b'j'),
        (JOYPAD_L, 19),
        (JOYPAD_R, 14),
    ];
    mappings
        .into_iter()
        .filter_map(|(id, key)| {
            let pressed =
                callback.is_some_and(|callback| unsafe { callback(0, DEVICE_JOYPAD, 0, id) != 0 });
            pressed.then_some(key)
        })
        .collect()
}

fn read_pointer(callback: InputStateCallback, width: u16, height: u16) -> Option<PointerState> {
    let callback = callback?;
    let pressed = unsafe { callback(0, DEVICE_POINTER, 0, POINTER_PRESSED) != 0 };
    let raw_x = unsafe { callback(0, DEVICE_POINTER, 0, POINTER_X) };
    let raw_y = unsafe { callback(0, DEVICE_POINTER, 0, POINTER_Y) };
    if !pressed && raw_x == 0 && raw_y == 0 {
        return None;
    }

    let scale = |value: i16, extent: u16| {
        let normalized = i32::from(value) + 32_768;
        (normalized * i32::from(extent) / 65_536).clamp(0, i32::from(extent.saturating_sub(1)))
            as i16
    };
    Some(PointerState {
        x: scale(raw_x, width),
        y: scale(raw_y, height),
        pressed,
    })
}

fn emit_silence(callbacks: &Callbacks) {
    static SILENCE: [i16; AUDIO_FRAMES_PER_VIDEO_FRAME * 2] = [0; AUDIO_FRAMES_PER_VIDEO_FRAME * 2];
    if let Some(callback) = callbacks.audio_sample_batch {
        unsafe { callback(SILENCE.as_ptr(), AUDIO_FRAMES_PER_VIDEO_FRAME) };
    } else if let Some(callback) = callbacks.audio_sample {
        for _ in 0..AUDIO_FRAMES_PER_VIDEO_FRAME {
            unsafe { callback(0, 0) };
        }
    }
}

fn game_path(path: *const c_char) -> Option<PathBuf> {
    if path.is_null() {
        return None;
    }
    Some(PathBuf::from(
        unsafe { CStr::from_ptr(path) }.to_string_lossy().as_ref(),
    ))
}

fn game_data(info: &GameInfo, path: Option<&Path>) -> std::io::Result<Vec<u8>> {
    if !info.data.is_null() && info.size != 0 {
        return Ok(
            unsafe { std::slice::from_raw_parts(info.data.cast::<u8>(), info.size) }.to_vec(),
        );
    }
    fs::read(path.ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "content has no path or data",
        )
    })?)
}

fn import_companion_files(
    program_path: &Path,
    files: &mut lavaxemu_core::VirtualFileSystem,
) -> std::io::Result<()> {
    let root = program_path.parent().unwrap_or_else(|| Path::new("."));
    import_directory(root, root, program_path, files)
}

fn import_directory(
    root: &Path,
    directory: &Path,
    program_path: &Path,
    files: &mut lavaxemu_core::VirtualFileSystem,
) -> std::io::Result<()> {
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            import_directory(root, &path, program_path, files)?;
        } else if file_type.is_file() && path != program_path {
            let relative = path
                .strip_prefix(root)
                .expect("entry must be below content root");
            let virtual_path = format!("/{}", relative.to_string_lossy().replace('\\', "/"));
            files.import_file(&virtual_path, fs::read(path)?);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    static VIDEO_WIDTH: AtomicU32 = AtomicU32::new(0);

    unsafe extern "C" fn environment(command: u32, _data: *mut c_void) -> bool {
        command == ENVIRONMENT_SET_PIXEL_FORMAT
            || command == ENVIRONMENT_SET_INPUT_DESCRIPTORS
            || command == ENVIRONMENT_SET_PERFORMANCE_LEVEL
    }

    unsafe extern "C" fn video(_data: *const c_void, width: u32, _height: u32, _pitch: usize) {
        VIDEO_WIDTH.store(width, Ordering::Relaxed);
    }

    #[test]
    fn loads_runs_and_serializes_content() {
        let mut image = [0_u8; 17];
        image[..4].copy_from_slice(b"LAV\x12");
        image[9] = 10;
        image[10] = 5;
        image[16] = 4;
        let info = GameInfo {
            path: ptr::null(),
            data: image.as_ptr().cast(),
            size: image.len(),
            meta: ptr::null(),
        };

        retro_set_environment(Some(environment));
        retro_set_video_refresh(Some(video));
        assert!(unsafe { retro_load_game(&info) });
        retro_run();
        assert_eq!(VIDEO_WIDTH.load(Ordering::Relaxed), 160);
        assert_eq!(retro_get_memory_size(MEMORY_SYSTEM_RAM), 0x0100_0000);

        let mut state = vec![0_u8; retro_serialize_size()];
        assert!(unsafe { retro_serialize(state.as_mut_ptr().cast(), state.len()) });
        assert!(unsafe { retro_unserialize(state.as_ptr().cast(), state.len()) });
        retro_unload_game();
    }
}
