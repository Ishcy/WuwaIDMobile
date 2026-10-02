use std::path::Path;

use super::model::{Entry, FileStat, PakResult};

pub trait Storage: Send + Sync {
    fn list(&self, path: &str) -> PakResult<Vec<Entry>>;
    fn stat(&self, path: &str) -> PakResult<FileStat>;
    fn read_text(&self, path: &str) -> PakResult<String>;
    fn hash(&self, path: &str, algorithm: &str) -> PakResult<String>;
    fn mkdir(&self, path: &str) -> PakResult<()>;
    fn copy(&self, source: &str, target: &str) -> PakResult<()>;
    fn move_file(&self, source: &str, target: &str) -> PakResult<()>;
    fn remove(&self, path: &str) -> PakResult<()>;
    fn write_text(&self, path: &str, content: &str) -> PakResult<()>;
    fn write_from_file(&self, path: &str, source: &Path) -> PakResult<()>;
    fn force_stop(&self) -> PakResult<()>;
}
