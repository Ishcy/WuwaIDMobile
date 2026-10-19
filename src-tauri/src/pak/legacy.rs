#[cfg(any(target_os = "android", test))]
use serde::{Deserialize, Serialize};
#[cfg(any(target_os = "android", test))]
use std::sync::LazyLock;

#[cfg(any(target_os = "android", test))]
use super::model::PakResult;

pub(super) const DIRECTORY: &str = "wuwaindonesia";
pub(super) const MOUNT: &str = "wuwaindonesia.txt";

#[cfg(any(target_os = "android", test))]
#[derive(Deserialize)]
pub(super) struct PublishedPak {
    pub tag: String,
    pub game_version: String,
    pub asset_name: String,
    pub size: u64,
    pub sha256: String,
}

#[cfg(any(target_os = "android", test))]
// Published TitoTFP/WuwaID asset digests captured on 2026-10-04. Never infer
// ownership from a filename, cached preferences, or a self-consistent mount alone.
pub(super) fn published(hash: &str, size: u64) -> PakResult<&'static PublishedPak> {
    static RELEASES: LazyLock<Result<Vec<PublishedPak>, String>> = LazyLock::new(|| {
        serde_json::from_str(include_str!("legacy-releases.json"))
            .map_err(|error| format!("Legacy release catalog is malformed: {error}"))
    });
    let releases = RELEASES.as_ref().map_err(Clone::clone)?;
    releases.iter().find(|release| release.sha256 == hash && release.size == size)
        .ok_or_else(|| "PAK versi lama tidak cocok dengan hash/ukuran rilis resmi yang dikenal. Berkas dipertahankan; jangan hapus atau timpa secara otomatis.".into())
}

#[cfg(any(target_os = "android", test))]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(super) struct Proof {
    pub size: u64,
    pub pak_sha256: String,
    pub pak_sha1: String,
    pub signature_sha256: String,
    pub signature_sha1: String,
    pub mount_sha256: String,
}

pub(super) fn paths(version: &str) -> [String; 3] {
    [
        format!("{version}/{DIRECTORY}/WuWaID_99_P.pak"),
        format!("{version}/{DIRECTORY}/WuWaID_99_P.sig"),
        format!("{version}/Mount/{MOUNT}"),
    ]
}

pub(super) fn lifecycle_paths(version: &str) -> Vec<String> {
    paths(version)
        .into_iter()
        .flat_map(|path| {
            [
                format!("{path}.new"),
                format!("{path}.new.tmp"),
                format!("{path}.bak"),
            ]
        })
        .collect()
}

#[cfg(any(target_os = "android", test))]
pub(super) fn mount_content(pak_sha1: &str, signature_sha1: &str) -> String {
    format!(
        "::Mount::\n{DIRECTORY}/WuWaID_99_P,99,{},{},,\n::Del::\n",
        pak_sha1.to_ascii_uppercase(),
        signature_sha1.to_ascii_uppercase()
    )
}
