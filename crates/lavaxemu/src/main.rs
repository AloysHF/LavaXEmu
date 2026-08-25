use anyhow::{Context, Result};
use clap::Parser;
use image::{ColorType, ImageFormat};
use lavaxemu_core::{Emulator, FrameStatus};
use minifb::{Key, KeyRepeat, ScaleMode, Window, WindowOptions};

use crate::standalone::{
    cli::Cli,
    content::{flush_dirty_files, load_emulator},
    input::update_input,
};

mod standalone;

fn main() -> Result<()> {
    env_logger::init();
    let cli = Cli::parse();
    let mut emulator = load_emulator(&cli.program, &cli.arguments)?;

    if cli.info {
        println!("{:#?}", emulator.vm().program().header());
        return Ok(());
    }

    if cli.headless || cli.screenshot.is_some() {
        run_headless(&mut emulator, cli.frames)?;
    } else {
        run_windowed(&mut emulator, cli.scale)?;
    }

    if let Some(path) = cli.screenshot.as_deref() {
        save_screenshot(&emulator, path)?;
    }

    if !cli.read_only {
        flush_dirty_files(&cli.program, emulator.files_mut())?;
    }
    Ok(())
}

fn run_headless(emulator: &mut Emulator, frame_count: usize) -> Result<()> {
    for _ in 0..frame_count {
        if matches!(emulator.run_frame()?.status, FrameStatus::Halted(_)) {
            break;
        }
    }
    Ok(())
}

fn run_windowed(emulator: &mut Emulator, scale: usize) -> Result<()> {
    let width = usize::from(emulator.display().width());
    let height = usize::from(emulator.display().height());
    let mut window = Window::new(
        "LavaXEmu",
        width * scale,
        height * scale,
        WindowOptions {
            resize: true,
            scale_mode: ScaleMode::AspectRatioStretch,
            ..WindowOptions::default()
        },
    )
    .context("failed to create the emulator window")?;
    window.set_target_fps(60);

    let mut frame = vec![0_u32; width * height];
    let mut paused = false;
    while window.is_open() && !window.is_key_down(Key::F12) {
        if window.is_key_pressed(Key::F9, KeyRepeat::No) {
            paused = !paused;
            window.set_title(if paused {
                "LavaXEmu - Paused"
            } else {
                "LavaXEmu"
            });
        }
        if window.is_key_pressed(Key::F10, KeyRepeat::No) {
            emulator.reset();
        }
        update_input(&window, emulator.input_mut(), width, height);
        let status = if paused {
            FrameStatus::Running
        } else {
            emulator.run_frame()?.status
        };
        emulator.display().to_xrgb8888(&mut frame);
        window
            .update_with_buffer(&frame, width, height)
            .context("failed to update the emulator window")?;
        if matches!(status, FrameStatus::Halted(_)) {
            break;
        }
    }
    Ok(())
}

fn save_screenshot(emulator: &Emulator, path: &std::path::Path) -> Result<()> {
    let display = emulator.display();
    let rgb: Vec<u8> = display
        .indexed_frame()
        .iter()
        .flat_map(|&color| display.palette()[usize::from(color)])
        .collect();
    image::save_buffer_with_format(
        path,
        &rgb,
        u32::from(display.width()),
        u32::from(display.height()),
        ColorType::Rgb8,
        ImageFormat::Png,
    )
    .with_context(|| format!("failed to save screenshot to {}", path.display()))
}
