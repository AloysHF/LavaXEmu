use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileInfo {
    pub path: String,
    pub size: usize,
    pub dirty: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FileEntry {
    data: Vec<u8>,
    dirty: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OpenFile {
    path: String,
    offset: usize,
    readable: bool,
    writable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VirtualFileSystem {
    files: BTreeMap<String, FileEntry>,
    directories: BTreeSet<String>,
    cwd: String,
    handles: [Option<OpenFile>; 3],
}

impl Default for VirtualFileSystem {
    fn default() -> Self {
        Self {
            files: BTreeMap::new(),
            directories: BTreeSet::from(["/".to_owned()]),
            cwd: "/".to_owned(),
            handles: std::array::from_fn(|_| None),
        }
    }
}

impl VirtualFileSystem {
    pub fn import_file(&mut self, path: &str, data: Vec<u8>) -> bool {
        let Some(path) = self.normalize(path) else {
            return false;
        };
        self.create_parent_directories(&path);
        self.files.insert(path, FileEntry { data, dirty: false });
        true
    }

    pub fn file(&self, path: &str) -> Option<&[u8]> {
        let path = self.normalize(path)?;
        self.files.get(&path).map(|entry| entry.data.as_slice())
    }

    pub fn files(&self) -> Vec<FileInfo> {
        self.files
            .iter()
            .map(|(path, entry)| FileInfo {
                path: path.clone(),
                size: entry.data.len(),
                dirty: entry.dirty,
            })
            .collect()
    }

    pub fn take_dirty_files(&mut self) -> Vec<(String, Vec<u8>)> {
        let mut output = Vec::new();
        for (path, entry) in &mut self.files {
            if entry.dirty {
                output.push((path.clone(), entry.data.clone()));
                entry.dirty = false;
            }
        }
        output
    }

    pub fn current_directory(&self) -> &str {
        &self.cwd
    }

    pub fn change_directory(&mut self, path: &str) -> bool {
        let Some(path) = self.normalize(path) else {
            return false;
        };
        if self.directories.contains(&path) {
            self.cwd = path;
            true
        } else {
            false
        }
    }

    pub fn create_directory(&mut self, path: &str) -> bool {
        let Some(path) = self.normalize(path) else {
            return false;
        };
        self.create_parent_directories(&path);
        self.directories.insert(path)
    }

    pub fn delete(&mut self, path: &str) -> bool {
        let Some(path) = self.normalize(path) else {
            return false;
        };
        if self.files.remove(&path).is_some() {
            return true;
        }
        if path == "/" || self.files.keys().any(|file| parent_path(file) == path) {
            return false;
        }
        self.directories.remove(&path)
    }

    pub fn count(&self, path: &str) -> i32 {
        let Some(path) = self.normalize(path) else {
            return -1;
        };
        if !self.directories.contains(&path) {
            return -1;
        }
        let files = self
            .files
            .keys()
            .filter(|file| parent_path(file) == path)
            .count();
        let directories = self
            .directories
            .iter()
            .filter(|directory| directory.as_str() != path && parent_path(directory) == path)
            .count();
        (files + directories) as i32
    }

    pub fn list(&self, path: &str) -> Vec<String> {
        let Some(path) = self.normalize(path) else {
            return Vec::new();
        };
        let mut entries: Vec<String> = self
            .directories
            .iter()
            .filter(|directory| directory.as_str() != path && parent_path(directory) == path)
            .filter_map(|directory| file_name(directory).map(ToOwned::to_owned))
            .collect();
        entries.extend(
            self.files
                .keys()
                .filter(|file| parent_path(file) == path)
                .filter_map(|file| file_name(file).map(ToOwned::to_owned)),
        );
        entries.sort();
        entries
    }

    pub fn open(&mut self, path: &str, mode: &str) -> Option<u8> {
        let path = self.normalize(path)?;
        let slot = self.handles.iter().position(Option::is_none)?;
        let plus = mode.contains('+');
        let readable = mode.starts_with('r') || plus;
        let writable = mode.starts_with('w') || mode.starts_with('a') || plus;
        let append = mode.starts_with('a');
        if mode.starts_with('r') && !self.files.contains_key(&path) {
            return None;
        }
        if mode.starts_with('w') || (append && !self.files.contains_key(&path)) {
            self.create_parent_directories(&path);
            self.files.insert(
                path.clone(),
                FileEntry {
                    data: Vec::new(),
                    dirty: true,
                },
            );
        }
        if !self.files.contains_key(&path) || (!readable && !writable) {
            return None;
        }
        let offset = if append {
            self.files[&path].data.len()
        } else {
            0
        };
        self.handles[slot] = Some(OpenFile {
            path,
            offset,
            readable,
            writable,
        });
        Some(0x80 | slot as u8)
    }

    pub fn close(&mut self, handle: u8) -> bool {
        let Some(slot) = handle_slot(handle) else {
            return false;
        };
        self.handles[slot].take().is_some()
    }

    pub fn read(&mut self, handle: u8, length: usize) -> Option<Vec<u8>> {
        let slot = handle_slot(handle)?;
        let open = self.handles[slot].as_mut()?;
        if !open.readable {
            return Some(Vec::new());
        }
        let entry = self.files.get(&open.path)?;
        let end = open.offset.saturating_add(length).min(entry.data.len());
        let data = entry.data[open.offset..end].to_vec();
        open.offset = end;
        Some(data)
    }

    pub fn write(&mut self, handle: u8, data: &[u8]) -> Option<usize> {
        let slot = handle_slot(handle)?;
        let open = self.handles[slot].as_mut()?;
        if !open.writable {
            return Some(0);
        }
        let entry = self.files.get_mut(&open.path)?;
        let end = open.offset.checked_add(data.len())?;
        if end > entry.data.len() {
            entry.data.resize(end, 0);
        }
        entry.data[open.offset..end].copy_from_slice(data);
        open.offset = end;
        entry.dirty = true;
        Some(data.len())
    }

    pub fn seek(&mut self, handle: u8, offset: i32, origin: u8) -> Option<usize> {
        let slot = handle_slot(handle)?;
        let open = self.handles[slot].as_mut()?;
        let length = self.files.get(&open.path)?.data.len() as i64;
        let base = match origin {
            0 => 0,
            1 => open.offset as i64,
            2 => length,
            _ => return None,
        };
        let position = base + i64::from(offset);
        if !(0..=length).contains(&position) {
            return None;
        }
        open.offset = position as usize;
        Some(open.offset)
    }

    pub fn tell(&self, handle: u8) -> Option<usize> {
        let slot = handle_slot(handle)?;
        Some(self.handles[slot].as_ref()?.offset)
    }

    pub fn eof(&self, handle: u8) -> Option<bool> {
        let slot = handle_slot(handle)?;
        let open = self.handles[slot].as_ref()?;
        Some(open.offset == self.files.get(&open.path)?.data.len())
    }

    pub fn rewind(&mut self, handle: u8) -> bool {
        let Some(slot) = handle_slot(handle) else {
            return false;
        };
        let Some(open) = self.handles[slot].as_mut() else {
            return false;
        };
        open.offset = 0;
        true
    }

    fn normalize(&self, path: &str) -> Option<String> {
        let path = path.replace('\\', "/").to_lowercase();
        let mut components = if path.starts_with('/') {
            Vec::new()
        } else {
            self.cwd
                .split('/')
                .filter(|component| !component.is_empty())
                .map(ToOwned::to_owned)
                .collect()
        };
        for component in path.split('/') {
            match component {
                "" | "." => {}
                ".." => {
                    components.pop()?;
                }
                value if value.contains(':') || value.contains('\0') => return None,
                value => components.push(value.to_owned()),
            }
        }
        Some(format!("/{}", components.join("/")))
    }

    fn create_parent_directories(&mut self, path: &str) {
        let mut current = String::new();
        let components: Vec<&str> = path.split('/').filter(|part| !part.is_empty()).collect();
        for component in components.iter().take(components.len().saturating_sub(1)) {
            current.push('/');
            current.push_str(component);
            self.directories.insert(current.clone());
        }
    }
}

fn handle_slot(handle: u8) -> Option<usize> {
    if (0x80..=0x82).contains(&handle) {
        Some(usize::from(handle & 3))
    } else {
        None
    }
}

fn parent_path(path: &str) -> &str {
    path.rsplit_once('/').map_or(
        "/",
        |(parent, _)| {
            if parent.is_empty() { "/" } else { parent }
        },
    )
}

fn file_name(path: &str) -> Option<&str> {
    path.rsplit('/').find(|part| !part.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn confines_paths_to_the_virtual_root() {
        let mut fs = VirtualFileSystem::default();
        assert!(fs.create_directory("/games/save"));
        assert!(fs.change_directory("/games/save"));
        assert!(!fs.import_file("../../../escape.dat", vec![1]));
        assert!(fs.import_file("slot.dat", vec![1, 2, 3]));
        assert_eq!(fs.file("/games/save/slot.dat"), Some([1, 2, 3].as_slice()));
    }

    #[test]
    fn supports_read_write_and_append_modes() {
        let mut fs = VirtualFileSystem::default();
        fs.import_file("save.dat", vec![1, 2, 3]);
        let handle = fs.open("save.dat", "r+").unwrap();
        assert_eq!(fs.read(handle, 2).unwrap(), [1, 2]);
        assert_eq!(fs.write(handle, &[9, 8]).unwrap(), 2);
        assert!(fs.close(handle));
        assert_eq!(fs.file("save.dat"), Some([1, 2, 9, 8].as_slice()));

        let handle = fs.open("save.dat", "ab").unwrap();
        fs.write(handle, &[7]).unwrap();
        assert_eq!(fs.file("save.dat"), Some([1, 2, 9, 8, 7].as_slice()));
    }

    #[test]
    fn lists_immediate_children() {
        let mut fs = VirtualFileSystem::default();
        fs.import_file("/one/a.dat", vec![]);
        fs.import_file("/one/two/b.dat", vec![]);
        assert_eq!(fs.count("/one"), 2);
        assert_eq!(fs.list("/one"), ["a.dat", "two"]);
    }
}
