use std::{fs, path::Path};

use anyhow::{Context, Result};
use lavaxemu_core::{Emulator, Program, VirtualFileSystem};

pub fn load_emulator(program_path: &Path, arguments: &[String]) -> Result<Emulator> {
    let data = fs::read(program_path)
        .with_context(|| format!("failed to read {}", program_path.display()))?;
    let program = Program::load(&data)
        .with_context(|| format!("failed to load {}", program_path.display()))?;
    let mut emulator = Emulator::new(program);
    emulator.set_command_line(arguments.join(" ").into_bytes());

    if let Some(root) = program_path.parent() {
        import_directory(root, root, program_path, emulator.files_mut())?;
    }
    Ok(emulator)
}

pub fn flush_dirty_files(program_path: &Path, files: &mut VirtualFileSystem) -> Result<()> {
    let root = program_path.parent().unwrap_or_else(|| Path::new("."));
    for (virtual_path, data) in files.take_dirty_files() {
        let relative = virtual_path.trim_start_matches('/');
        let destination = root.join(relative.replace('/', std::path::MAIN_SEPARATOR_STR));
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        fs::write(&destination, data)
            .with_context(|| format!("failed to write {}", destination.display()))?;
    }
    Ok(())
}

fn import_directory(
    root: &Path,
    directory: &Path,
    program_path: &Path,
    files: &mut VirtualFileSystem,
) -> Result<()> {
    for entry in fs::read_dir(directory)
        .with_context(|| format!("failed to read directory {}", directory.display()))?
    {
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
            let data = fs::read(&path)
                .with_context(|| format!("failed to read companion file {}", path.display()))?;
            files.import_file(&virtual_path, data);
        }
    }
    Ok(())
}
