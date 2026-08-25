use std::path::PathBuf;

use clap::Parser;

#[derive(Debug, Parser)]
#[command(version, about = "Standalone LavaX virtual machine")]
pub struct Cli {
    /// LAV program to load.
    pub program: PathBuf,

    /// Parse the program and print its metadata without running it.
    #[arg(long)]
    pub info: bool,

    /// Run without opening a window.
    #[arg(long)]
    pub headless: bool,

    /// Number of frames to run in headless mode.
    #[arg(long, default_value_t = 600)]
    pub frames: usize,

    /// Save the last frame to a PNG file.
    #[arg(long)]
    pub screenshot: Option<PathBuf>,

    /// Initial integer window scale.
    #[arg(long, default_value_t = 3, value_parser = parse_scale)]
    pub scale: usize,

    /// Do not write virtual file changes back to disk.
    #[arg(long)]
    pub read_only: bool,

    /// Arguments exposed to the guest program.
    #[arg(last = true)]
    pub arguments: Vec<String>,
}

fn parse_scale(value: &str) -> Result<usize, String> {
    let scale = value
        .parse::<usize>()
        .map_err(|_| "scale must be an integer".to_owned())?;
    if (1..=8).contains(&scale) {
        Ok(scale)
    } else {
        Err("scale must be between 1 and 8".to_owned())
    }
}
