//! The libretro ABI frontend is implemented after the platform-independent VM.

#[unsafe(no_mangle)]
pub extern "C" fn retro_api_version() -> u32 {
    1
}
