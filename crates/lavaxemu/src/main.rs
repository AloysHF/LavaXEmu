use std::{fs, path::PathBuf};

use anyhow::{Context, Result};
use clap::Parser;
use lavaxemu_core::Program;

#[derive(Debug, Parser)]
#[command(version, about = "Standalone LavaX virtual machine")]
struct Cli {
    /// LAV program to load.
    program: PathBuf,

    /// Parse the program and print its metadata without opening a window.
    #[arg(long)]
    info: bool,
}

fn main() -> Result<()> {
    env_logger::init();
    let cli = Cli::parse();
    let data = fs::read(&cli.program)
        .with_context(|| format!("failed to read {}", cli.program.display()))?;
    let program = Program::load(&data)
        .with_context(|| format!("failed to load {}", cli.program.display()))?;

    if cli.info {
        println!("{:#?}", program.header());
        return Ok(());
    }

    anyhow::bail!(
        "the standalone run loop is not implemented yet; use --info to inspect a LAV file"
    )
}
