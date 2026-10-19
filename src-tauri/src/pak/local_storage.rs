use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use sha1::Sha1;
use sha2::{Digest, Sha256};

use super::model::{Entry, FileHashes, FileStat, PakResult, MOUNT_FILENAME, PATCH_DIRECTORY};
use super::storage::Storage;

/// Filesystem-backed adapter for isolated tests and local smoke fixtures only.
/// Native desktop commands must not select this as a fallback storage backend.
pub struct LocalStorage {
    root: PathBuf,
}

impl LocalStorage {
    /// Opens an existing resource-tree directory as the adapter's containment root.
    pub fn new(root: impl AsRef<Path>) -> PakResult<Self> {
        let root = fs::canonicalize(root.as_ref())
            .map_err(|error| format!("Could not resolve local resource root: {error}"))?;
        let metadata = fs::symlink_metadata(&root)
            .map_err(|error| format!("Could not inspect local resource root: {error}"))?;
        if !metadata.file_type().is_dir() {
            return Err("Local resource root is not a directory".to_owned());
        }
        Ok(Self { root })
    }

    fn checked_path(&self, path: &str, allow_root: bool) -> PakResult<PathBuf> {
        if path.is_empty() {
            return if allow_root {
                Ok(self.root.clone())
            } else {
                Err("An empty path is only valid for the resource root".to_owned())
            };
        }
        if path.starts_with('/') || path.contains('\\') || path.contains('\0') {
            return Err(format!("Unsafe resource path: {path}"));
        }

        let segments: Vec<_> = path.split('/').collect();
        if segments
            .iter()
            .any(|segment| segment.is_empty() || *segment == "." || *segment == "..")
        {
            return Err(format!("Unsafe resource path: {path}"));
        }

        let mut current = self.root.clone();
        let mut missing_parent = false;
        for (index, segment) in segments.iter().enumerate() {
            current.push(*segment);
            if missing_parent {
                continue;
            }
            match fs::symlink_metadata(&current) {
                Ok(metadata) => {
                    if metadata.file_type().is_symlink() {
                        return Err(format!("Symbolic links are not allowed: {path}"));
                    }
                    if index + 1 != segments.len() && !metadata.is_dir() {
                        return Err(format!("A path parent is not a directory: {path}"));
                    }
                }
                Err(error) if error.kind() == io::ErrorKind::NotFound => missing_parent = true,
                Err(error) => {
                    return Err(format!("Could not inspect resource path {path}: {error}"))
                }
            }
        }
        if !current.starts_with(&self.root) {
            return Err(format!("Resource path escaped its root: {path}"));
        }
        Ok(current)
    }

    fn checked_write_path(&self, path: &str) -> PakResult<PathBuf> {
        if !is_allowed_write(path) {
            return Err(format!("Writes are not allowed for resource path: {path}"));
        }
        self.checked_path(path, false)
    }

    fn ensure_absent(path: &Path, label: &str) -> PakResult<()> {
        match fs::symlink_metadata(path) {
            Ok(_) => Err(format!("{label} already exists")),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(format!("Could not inspect {label}: {error}")),
        }
    }

    fn write_new(path: &Path, write: impl FnOnce(&mut File) -> io::Result<()>) -> PakResult<()> {
        let mut created = false;
        let result = (|| {
            let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
            created = true;
            write(&mut file)?;
            file.sync_all()
        })();
        match result {
            Ok(()) => Ok(()),
            Err(error) => {
                if created {
                    let _ = fs::remove_file(path);
                }
                Err(format!("Could not write local resource file: {error}"))
            }
        }
    }
}

impl Storage for LocalStorage {
    fn list(&self, path: &str) -> PakResult<Vec<Entry>> {
        let directory = self.checked_path(path, true)?;
        let metadata = fs::metadata(&directory)
            .map_err(|error| format!("Could not list resource directory {path}: {error}"))?;
        if !metadata.is_dir() {
            return Err(format!("Resource path is not a directory: {path}"));
        }
        let mut entries = Vec::new();
        for entry in fs::read_dir(&directory)
            .map_err(|error| format!("Could not list resource directory {path}: {error}"))?
        {
            let entry = entry
                .map_err(|error| format!("Could not read resource directory entry: {error}"))?;
            let name = entry
                .file_name()
                .into_string()
                .map_err(|_| format!("Resource directory {path} contains a non-UTF-8 name"))?;
            let metadata = fs::symlink_metadata(entry.path())
                .map_err(|error| format!("Could not inspect resource entry {name}: {error}"))?;
            if metadata.file_type().is_symlink() {
                return Err(format!("Symbolic links are not allowed: {path}/{name}"));
            }
            entries.push(Entry {
                name,
                is_dir: metadata.is_dir(),
            });
        }
        entries.sort_by(|left, right| left.name.cmp(&right.name));
        Ok(entries)
    }

    fn stat(&self, path: &str) -> PakResult<FileStat> {
        let path_on_disk = self.checked_path(path, true)?;
        match fs::symlink_metadata(&path_on_disk) {
            Ok(metadata) => Ok(FileStat {
                exists: true,
                is_dir: metadata.is_dir(),
                size: if metadata.is_file() {
                    metadata.len()
                } else {
                    0
                },
            }),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(FileStat {
                exists: false,
                is_dir: false,
                size: 0,
            }),
            Err(error) => Err(format!("Could not inspect resource path {path}: {error}")),
        }
    }

    fn stat_many(&self, paths: &[String]) -> PakResult<Vec<FileStat>> {
        paths.iter().map(|path| self.stat(path)).collect()
    }

    fn read_text(&self, path: &str) -> PakResult<String> {
        let path_on_disk = self.checked_path(path, true)?;
        fs::read_to_string(&path_on_disk)
            .map_err(|error| format!("Could not read resource text {path}: {error}"))
    }

    fn hashes(&self, path: &str, with_sha1: bool) -> PakResult<FileHashes> {
        let path_on_disk = self.checked_path(path, false)?;
        let before = fs::symlink_metadata(&path_on_disk).map_err(|e| e.to_string())?;
        if !before.is_file() {
            return Err(format!("Resource is not a regular file: {path}"));
        }
        let mut file = File::open(&path_on_disk).map_err(|e| e.to_string())?;
        let mut sha256 = Sha256::new();
        let mut sha1 = with_sha1.then(Sha1::new);
        let mut buffer = [0u8; 64 * 1024];
        let mut size = 0;
        loop {
            let count = file.read(&mut buffer).map_err(|e| e.to_string())?;
            if count == 0 {
                break;
            }
            sha256.update(&buffer[..count]);
            if let Some(sha1) = &mut sha1 {
                sha1.update(&buffer[..count]);
            }
            size += count as u64;
        }
        let after =
            fs::symlink_metadata(self.checked_path(path, false)?).map_err(|e| e.to_string())?;
        if !after.is_file()
            || size != before.len()
            || size != after.len()
            || before.modified().ok() != after.modified().ok()
        {
            return Err(format!("Resource changed while hashing: {path}"));
        }
        Ok(FileHashes {
            size,
            sha256: hex::encode(sha256.finalize()),
            sha1: sha1.map(|hash| hex::encode(hash.finalize())),
        })
    }

    fn mkdir(&self, path: &str) -> PakResult<()> {
        if !is_allowed_patch_directory(path) {
            return Err(format!(
                "Directory creation is not allowed for resource path: {path}"
            ));
        }
        let version = path
            .split('/')
            .next()
            .ok_or_else(|| "Invalid patch directory".to_owned())?;
        let version_root = self.checked_path(version, false)?;
        let metadata = fs::symlink_metadata(version_root)
            .map_err(|error| format!("Could not inspect resource version {version}: {error}"))?;
        if !metadata.is_dir() {
            return Err(format!("Resource version is not a directory: {version}"));
        }
        let directory = self.checked_path(path, false)?;
        fs::create_dir_all(directory)
            .map_err(|error| format!("Could not create installer directory {path}: {error}"))
    }

    fn copy(&self, source: &str, target: &str) -> PakResult<()> {
        let source = self.checked_path(source, false)?;
        let target = self.checked_write_path(target)?;
        let metadata = fs::symlink_metadata(&source)
            .map_err(|error| format!("Could not inspect copy source: {error}"))?;
        if !metadata.is_file() {
            return Err("Copy source is not a regular file".to_owned());
        }
        Self::ensure_absent(&target, "Copy target")?;
        Self::write_new(&target, |output| {
            let mut input = File::open(source)?;
            io::copy(&mut input, output)?;
            Ok(())
        })
    }

    fn move_file(&self, source: &str, target: &str) -> PakResult<()> {
        let source = self.checked_write_path(source)?;
        let target = self.checked_write_path(target)?;
        let metadata = fs::symlink_metadata(&source)
            .map_err(|error| format!("Could not inspect move source: {error}"))?;
        if !metadata.is_file() {
            return Err("Move source is not a regular file".to_owned());
        }
        fs::hard_link(&source, &target).map_err(|error| {
            format!("Could not create non-replacing installer move target: {error}")
        })?;
        fs::remove_file(&source)
            .map_err(|error| format!("Move target was published but its source remains: {error}"))
    }

    fn remove(&self, path: &str) -> PakResult<()> {
        let path_on_disk = self.checked_write_path(path)?;
        match fs::symlink_metadata(&path_on_disk) {
            Ok(metadata) if metadata.is_file() => fs::remove_file(path_on_disk)
                .map_err(|error| format!("Could not remove installer file {path}: {error}")),
            Ok(_) => Err(format!("Refusing to remove non-file resource path: {path}")),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(format!("Could not inspect installer file {path}: {error}")),
        }
    }

    fn write_text(&self, path: &str, content: &str) -> PakResult<()> {
        let path_on_disk = self.checked_write_path(path)?;
        Self::ensure_absent(&path_on_disk, "Text target")?;
        Self::write_new(&path_on_disk, |file| file.write_all(content.as_bytes()))
    }

    fn write_from_file(&self, path: &str, source: &Path) -> PakResult<()> {
        let target = self.checked_write_path(path)?;
        let source_metadata = fs::symlink_metadata(source)
            .map_err(|error| format!("Could not inspect local source file: {error}"))?;
        if !source_metadata.is_file() {
            return Err("Local source is not a regular file".to_owned());
        }
        Self::ensure_absent(&target, "File target")?;
        Self::write_new(&target, |output| {
            let mut input = File::open(source)?;
            io::copy(&mut input, output)?;
            Ok(())
        })
    }

    // The adapter owns only an isolated local fixture; there is no game process to stop.
    fn force_stop(&self) -> PakResult<()> {
        Ok(())
    }
}

fn is_allowed_patch_directory(path: &str) -> bool {
    let parts: Vec<_> = path.split('/').collect();
    parts.len() == 2 && is_resource_version(parts[0]) && parts[1] == PATCH_DIRECTORY
}

fn is_allowed_write(path: &str) -> bool {
    let parts: Vec<_> = path.split('/').collect();
    if parts.len() != 3 || !is_resource_version(parts[0]) {
        return false;
    }
    if parts[1] == PATCH_DIRECTORY {
        return !parts[2].is_empty()
            && !parts[2]
                .chars()
                .any(|character| matches!(character, '/' | '\\'));
    }
    parts[1] == "Mount" && is_mount_write_name(parts[2])
}

fn is_mount_write_name(name: &str) -> bool {
    name == MOUNT_FILENAME
        || name == format!("{MOUNT_FILENAME}.new")
        || name == format!("{MOUNT_FILENAME}.bak")
}

fn is_resource_version(value: &str) -> bool {
    let mut parts = value.split('.');
    let Some(major) = parts.next() else {
        return false;
    };
    let Some(minor) = parts.next() else {
        return false;
    };
    let Some(patch) = parts.next() else {
        return false;
    };
    parts.next().is_none()
        && [major, minor, patch].iter().all(|part| {
            !part.is_empty()
                && (part.len() == 1 || !part.starts_with('0'))
                && part.bytes().all(|byte| byte.is_ascii_digit())
        })
}

#[cfg(test)]
mod tests {
    use super::LocalStorage;
    use crate::pak::storage::Storage;
    use std::fs;

    #[test]
    fn refuses_mutations_outside_installer_owned_paths() {
        let temporary = tempfile::tempdir().expect("temporary directory");
        fs::create_dir(temporary.path().join("3.7.0")).expect("version directory");
        let storage = LocalStorage::new(temporary.path()).expect("local storage");
        assert!(storage.mkdir("3.7.0/wuwaid").is_ok());
        assert!(storage.stat("../outside").is_err());
        assert!(storage.list("3.7.0/../../").is_err());
        assert!(storage
            .write_text("3.7.0/Mount/MountResource.txt", "")
            .is_err());
        assert!(storage.remove("3.7.0/wuwaid/../game.pak").is_err());
        assert!(storage
            .write_text("3.7.0/Resource/unowned.pak", "unowned")
            .is_err());
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlinked_resource_components() {
        use std::os::unix::fs::symlink;

        let temporary = tempfile::tempdir().expect("temporary directory");
        let outside = tempfile::tempdir().expect("outside directory");
        fs::create_dir(temporary.path().join("3.7.0")).expect("version directory");
        symlink(outside.path(), temporary.path().join("3.7.0/wuwaid")).expect("symlink");
        let storage = LocalStorage::new(temporary.path()).expect("local storage");
        assert!(storage.stat("3.7.0/wuwaid/patch.pak").is_err());
        assert!(storage.mkdir("3.7.0/wuwaid").is_err());
    }

    #[test]
    fn existing_targets_are_never_replaced_by_write_copy_or_move() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir_all(root.path().join("3.7.0/wuwaid")).unwrap();
        let storage = LocalStorage::new(root.path()).unwrap();
        let source = "3.7.0/wuwaid/WuwaID_99_P.pak.new";
        let target = "3.7.0/wuwaid/WuwaID_99_P.pak";
        fs::write(root.path().join(source), b"new payload").unwrap();
        fs::write(root.path().join(target), b"unowned collision").unwrap();
        assert!(storage.write_text(target, "replacement").is_err());
        assert!(storage.copy(source, target).is_err());
        assert!(storage.move_file(source, target).is_err());
        assert!(storage
            .write_from_file(target, &root.path().join(source))
            .is_err());
        assert_eq!(
            fs::read(root.path().join(target)).unwrap(),
            b"unowned collision"
        );
        assert_eq!(fs::read(root.path().join(source)).unwrap(), b"new payload");
    }

    #[test]
    fn fresh_hashes_detect_same_size_changes_and_do_not_reuse_previous_sha1() {
        use sha1::Sha1;
        use sha2::{Digest, Sha256};
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("payload");
        fs::write(&path, b"first").unwrap();
        let storage = LocalStorage::new(root.path()).unwrap();
        let first = storage.hashes("payload", true).unwrap();
        fs::write(&path, b"other").unwrap();
        let changed = storage.hashes("payload", false).unwrap();
        assert_eq!(changed.size, 5);
        assert_eq!(changed.sha256, hex::encode(Sha256::digest(b"other")));
        assert_eq!(changed.sha1, None);
        assert_ne!(changed.sha256, first.sha256);
        let both = storage.hashes("payload", true).unwrap();
        assert_eq!(both.sha256, changed.sha256);
        assert_eq!(both.sha1, Some(hex::encode(Sha1::digest(b"other"))));
    }
}
