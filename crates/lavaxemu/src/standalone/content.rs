use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use lavaxemu_core::{Emulator, Program, VirtualFileSystem};

pub fn load_emulator(program_path: &Path, arguments: &[String]) -> Result<Emulator> {
    let data = fs::read(program_path)
        .with_context(|| format!("failed to read {}", program_path.display()))?;
    let program = Program::load(&data)
        .with_context(|| format!("failed to load {}", program_path.display()))?;
    let mut emulator = Emulator::new(program);
    emulator.set_command_line(arguments.join(" ").into_bytes());

    let root = content_root(program_path);
    import_directory(&root, &root, program_path, emulator.files_mut())?;
    Ok(emulator)
}

pub fn flush_dirty_files(program_path: &Path, files: &mut VirtualFileSystem) -> Result<()> {
    let root = content_root(program_path);
    for (virtual_path, data) in files.take_dirty_files() {
        let relative = virtual_path.trim_start_matches('/');
        let mut destination = root.clone();
        for component in relative.split('/') {
            let existing = fs::read_dir(&destination).ok().and_then(|entries| {
                entries
                    .filter_map(Result::ok)
                    .find(|entry| entry.file_name().eq_ignore_ascii_case(component))
            });
            destination =
                existing.map_or_else(|| destination.join(component), |entry| entry.path());
        }
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        fs::write(&destination, data)
            .with_context(|| format!("failed to write {}", destination.display()))?;
    }
    Ok(())
}

fn content_root(program_path: &Path) -> PathBuf {
    let parent = program_path.parent().unwrap_or_else(|| Path::new("."));
    parent
        .ancestors()
        .take(4)
        .find(|ancestor| {
            let device_folder = program_path
                .strip_prefix(ancestor)
                .ok()
                .and_then(|relative| relative.components().next());
            if !device_folder.is_some_and(|folder| {
                folder.as_os_str().eq_ignore_ascii_case("GVmaker")
                    || folder.as_os_str().eq_ignore_ascii_case("LAVA")
            }) {
                return false;
            }
            fs::read_dir(ancestor).is_ok_and(|entries| {
                entries.filter_map(Result::ok).any(|entry| {
                    entry.file_type().is_ok_and(|kind| kind.is_dir())
                        && entry.file_name().eq_ignore_ascii_case("LavaData")
                })
            })
        })
        .unwrap_or(parent)
        .to_path_buf()
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
