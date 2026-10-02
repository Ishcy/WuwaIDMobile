use serde::{Deserialize, Serialize};

pub const GAME_PACKAGE: &str = "com.kurogame.wutheringwaves.global";
pub const PATCH_DIRECTORY: &str = "wuwaid";
pub const PATCH_FILENAME: &str = "WuwaID_99_P.pak";
pub const SIGNATURE_FILENAME: &str = "WuwaID_99_P.sig";
pub const MOUNT_FILENAME: &str = "wuwaid.txt";
pub const PRIORITY: u16 = 99;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Release {
    pub tag: String,
    pub game_version: String,
    pub asset_name: String,
    pub download_url: String,
    pub html_url: String,
    pub size: u64,
    pub sha256: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub percent: u8,
    pub label: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub name: String,
    pub is_dir: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileStat {
    pub exists: bool,
    pub is_dir: bool,
    pub size: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceProbe {
    pub game_installed: bool,
    pub game_version: String,
    pub shizuku_running: bool,
    pub shizuku_granted: bool,
    pub root_readable: bool,
    pub root_writable: bool,
    pub storage_error: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallStatus {
    pub installed: bool,
    pub healthy: bool,
    pub recovery_pending: bool,
    pub version: String,
    pub message: Option<String>,
}

#[derive(Clone, Debug)]
pub struct ResourceLayout {
    pub version: String,
    pub signature: String,
}

pub type PakResult<T> = Result<T, String>;
pub type ProgressCallback<'a> = dyn Fn(Progress) + Send + Sync + 'a;
