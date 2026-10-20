#[cfg(any(target_os = "android", test))]
use std::path::Path;

use super::model::{Entry, FileHashes, FileStat, PakResult};

pub trait Storage: Send + Sync {
    fn list(&self, path: &str) -> PakResult<Vec<Entry>>;
    fn stat(&self, path: &str) -> PakResult<FileStat>;
    /// Ordered fresh results; adapters must reject partial or oversized responses.
    fn stat_many(&self, paths: &[String]) -> PakResult<Vec<FileStat>>;
    fn read_text(&self, path: &str) -> PakResult<String>;
    /// Reject missing/non-regular/changed files; hashes and size share one stream.
    fn hashes(&self, path: &str, with_sha1: bool) -> PakResult<FileHashes>;
    #[cfg(any(target_os = "android", test))]
    fn mkdir(&self, path: &str) -> PakResult<()>;
    #[cfg(any(target_os = "android", test))]
    fn copy(&self, source: &str, target: &str) -> PakResult<()>;
    #[cfg(any(target_os = "android", test))]
    fn move_file(&self, source: &str, target: &str) -> PakResult<()>;
    #[cfg(any(target_os = "android", test))]
    fn remove(&self, path: &str) -> PakResult<()>;
    #[cfg(any(target_os = "android", test))]
    fn write_text(&self, path: &str, content: &str) -> PakResult<()>;
    #[cfg(any(target_os = "android", test))]
    fn write_from_file(&self, path: &str, source: &Path) -> PakResult<()>;
    #[cfg(any(target_os = "android", test))]
    fn force_stop(&self) -> PakResult<()>;
}
