use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::LazyLock;
use std::time::Duration;

use reqwest::blocking::{Client, Response};
use reqwest::redirect::Policy;
use reqwest::StatusCode;
use serde::Deserialize;
use sha2::{Digest, Sha256};

use super::model::{PakResult, Progress, ProgressCallback, Release};

const LATEST_RELEASE_URL: &str = "https://api.github.com/repos/TitoTFP/WuwaID/releases/latest";
const GITHUB_HOST: &str = "github.com";
const RELEASE_ASSET_HOST: &str = "release-assets.githubusercontent.com";
const USER_AGENT: &str = "WuwaIDMobile/0.1 (GitHub release downloader)";
const MAX_METADATA_BYTES: usize = 4 * 1024 * 1024;
const MAX_CHECKSUM_BYTES: usize = 1024 * 1024;
const STREAM_BUFFER_BYTES: usize = 64 * 1024;
const MAX_REDIRECTS: usize = 5;
static STAGE_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Deserialize)]
struct GithubRelease {
    tag_name: String,
    html_url: String,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
    #[serde(default)]
    assets: Vec<GithubAsset>,
}

#[derive(Debug, Deserialize)]
struct GithubAsset {
    name: String,
    browser_download_url: String,
    size: u64,
    state: String,
    #[serde(default)]
    digest: Option<String>,
}

/// Find the latest stable GitHub release and its unique uploaded PAK asset.
pub fn latest() -> PakResult<Release> {
    let _phase = crate::trace::Span::new("latest");
    let client = http_client()?;
    let response = client
        .get(LATEST_RELEASE_URL)
        .header("Accept", "application/vnd.github+json")
        .timeout(Duration::from_secs(30))
        .send()
        .map_err(|_| "Gagal menghubungi GitHub untuk memeriksa rilis terbaru.".to_owned())?;
    let response = ensure_success(response, "memeriksa rilis terbaru")?;
    let body = read_bounded(response, MAX_METADATA_BYTES, "metadata rilis")?;
    let metadata: GithubRelease = serde_json::from_slice(&body)
        .map_err(|_| "Metadata rilis GitHub tidak valid.".to_owned())?;

    if metadata.draft || metadata.prerelease {
        return Err("Rilis terbaru GitHub bukan rilis stabil.".to_owned());
    }
    let game_version = parse_game_version(&metadata.tag_name)?;
    validate_github_url(
        &metadata.html_url,
        &["TitoTFP", "WuwaID", "releases", "tag", &metadata.tag_name],
    )?;

    let mut pak_assets = metadata
        .assets
        .iter()
        .filter(|asset| asset.state == "uploaded" && is_pak_name(&asset.name));
    let asset = pak_assets
        .next()
        .ok_or_else(|| "Rilis GitHub tidak memiliki file PAK yang diunggah.".to_owned())?;
    if pak_assets.next().is_some() {
        return Err("Rilis GitHub memiliki lebih dari satu file PAK.".to_owned());
    }
    validate_asset_name(&asset.name)?;
    if asset.size == 0 {
        return Err("Ukuran file PAK pada metadata GitHub tidak valid.".to_owned());
    }
    validate_asset_url(&asset.browser_download_url, &metadata.tag_name, &asset.name)?;

    let sha256 = match asset.digest.as_deref() {
        Some(digest) => normalize_sha256(digest)?,
        None => checksum_digest(client, &metadata, asset)?,
    };

    Ok(Release {
        tag: metadata.tag_name,
        game_version,
        asset_name: asset.name.clone(),
        download_url: asset.browser_download_url.clone(),
        html_url: metadata.html_url,
        size: asset.size,
        sha256,
    })
}

/// Download the release into the cache, verifying size and SHA-256 before publishing it.
pub fn download(
    release: &Release,
    cache_dir: &Path,
    progress: &ProgressCallback<'_>,
) -> PakResult<PathBuf> {
    let _download = crate::trace::Span::new("download");
    let sha256 = validate_release(release)?;
    let cache_name = format!("{}-{}", release.tag, release.asset_name);
    let cache_path = cache_dir.join(cache_name);

    fs::create_dir_all(cache_dir)
        .map_err(|_| "Tidak dapat menyiapkan direktori cache patch.".to_owned())?;
    emit_progress(progress, 5, "Memeriksa cache patch...");

    let cache_check = crate::trace::Span::new("cache.verify");
    match cache_is_valid(&cache_path, release.size, &sha256) {
        Ok(true) => {
            drop(cache_check);
            crate::trace::mark("cache", "hit", release.size);
            emit_progress(progress, 65, "Patch rilis sudah tersedia di cache.");
            return Ok(cache_path);
        }
        Ok(false) => remove_cache_file(&cache_path)?,
        Err(_) => {
            remove_cache_file(&cache_path)?;
        }
    }
    drop(cache_check);
    crate::trace::mark("cache", "miss", 0);
    let _transfer = crate::trace::Span::new("download.transfer");

    let client = http_client()?;
    let response = client
        .get(&release.download_url)
        .header("Accept", "application/octet-stream")
        .timeout(Duration::from_secs(30 * 60))
        .send()
        .map_err(|_| "Gagal mengunduh file PAK dari GitHub.".to_owned())?;
    let response = ensure_success(response, "mengunduh file PAK")?;
    if response
        .content_length()
        .is_some_and(|content_length| content_length > release.size)
    {
        return Err("Unduhan PAK melebihi ukuran yang dinyatakan GitHub.".to_owned());
    }

    let mut staged = StagedFile::create(cache_dir, &cache_path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; STREAM_BUFFER_BYTES];
    let mut downloaded = 0_u64;
    let mut last_percent = 5_u8;
    let mut response = response;

    loop {
        let count = response
            .read(&mut buffer)
            .map_err(|_| "Koneksi terputus saat mengunduh file PAK.".to_owned())?;
        if count == 0 {
            break;
        }
        let count = u64::try_from(count)
            .map_err(|_| "Ukuran potongan unduhan PAK tidak valid.".to_owned())?;
        downloaded = downloaded
            .checked_add(count)
            .ok_or_else(|| "Ukuran unduhan PAK melampaui batas.".to_owned())?;
        if downloaded > release.size {
            return Err("Unduhan PAK melebihi ukuran yang dinyatakan GitHub.".to_owned());
        }

        let bytes = &buffer[..count as usize];
        staged
            .file_mut()
            .write_all(bytes)
            .map_err(|_| "Tidak dapat menulis file PAK ke cache.".to_owned())?;
        hasher.update(bytes);

        let percent = 5 + ((u128::from(downloaded) * 60) / u128::from(release.size)) as u8;
        if percent > last_percent {
            last_percent = percent;
            emit_progress(
                progress,
                percent.min(65),
                &format!("Mengunduh patch {}...", release.game_version),
            );
        }
    }

    if downloaded != release.size {
        return Err("Ukuran file PAK yang diunduh tidak sesuai metadata GitHub.".to_owned());
    }
    let actual_sha256 = hex::encode(hasher.finalize());
    if actual_sha256 != sha256 {
        return Err("SHA-256 file PAK tidak sesuai metadata GitHub.".to_owned());
    }
    staged
        .file_mut()
        .sync_all()
        .map_err(|_| "Tidak dapat menyimpan file PAK ke cache.".to_owned())?;
    staged.publish(&cache_path)?;
    crate::trace::mark("download", "bytes", downloaded);

    emit_progress(progress, 65, "Unduhan dan pemeriksaan integritas selesai.");
    Ok(cache_path)
}

/// Parse a GitHub release tag such as `v3.7.1-id.1` into its game version.
pub fn parse_game_version(tag: &str) -> PakResult<String> {
    let version = tag
        .strip_prefix('v')
        .ok_or_else(|| "Tag rilis GitHub tidak diawali huruf v.".to_owned())?;
    let (numeric, suffix) = match version.split_once('-') {
        Some((numeric, suffix)) => (numeric, Some(suffix)),
        None => (version, None),
    };

    let mut components = numeric.split('.');
    let valid_component = |component: &str| {
        !component.is_empty()
            && component.bytes().all(|byte| byte.is_ascii_digit())
            && (component == "0" || !component.starts_with('0'))
    };
    let major = components.next().unwrap_or_default();
    let minor = components.next().unwrap_or_default();
    let patch = components.next().unwrap_or_default();
    if !valid_component(major)
        || !valid_component(minor)
        || !valid_component(patch)
        || components.next().is_some()
        || suffix.is_some_and(|value| {
            value.is_empty()
                || !value
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'.' || byte == b'-')
                || value.starts_with('.')
                || value.ends_with('.')
                || value.contains("..")
        })
    {
        return Err("Format versi pada tag rilis GitHub tidak valid.".to_owned());
    }
    Ok(format!("{major}.{minor}.{patch}"))
}

fn http_client() -> PakResult<&'static Client> {
    static CLIENT: LazyLock<PakResult<Client>> = LazyLock::new(|| {
        let redirect_policy = Policy::custom(|attempt| {
            if attempt.previous().len() >= MAX_REDIRECTS {
                return attempt.error("too many redirects");
            }
            let url = attempt.url();
            if url.scheme() == "https"
                && url.username().is_empty()
                && url.password().is_none()
                && url.port().is_none()
                && matches!(url.host_str(), Some(GITHUB_HOST | RELEASE_ASSET_HOST))
            {
                attempt.follow()
            } else {
                attempt.stop()
            }
        });
        Client::builder()
            .user_agent(USER_AGENT)
            .connect_timeout(Duration::from_secs(20))
            .redirect(redirect_policy)
            .build()
            .map_err(|_| "Tidak dapat menyiapkan koneksi HTTPS GitHub.".to_owned())
    });
    CLIENT.as_ref().map_err(Clone::clone)
}

fn ensure_success(response: Response, action: &str) -> PakResult<Response> {
    let status = response.status();
    if status.is_success() {
        Ok(response)
    } else {
        Err(status_error(status, action))
    }
}

fn status_error(status: StatusCode, action: &str) -> String {
    match status {
        StatusCode::NOT_FOUND => {
            format!("Rilis atau file GitHub tidak ditemukan saat {action} (HTTP 404).")
        }
        StatusCode::FORBIDDEN | StatusCode::TOO_MANY_REQUESTS => {
            format!(
                "Batas akses GitHub tercapai saat {action} (HTTP {}).",
                status.as_u16()
            )
        }
        _ => format!("GitHub gagal {action} (HTTP {}).", status.as_u16()),
    }
}

fn read_bounded(response: Response, limit: usize, label: &str) -> PakResult<Vec<u8>> {
    if response
        .content_length()
        .is_some_and(|content_length| content_length > limit as u64)
    {
        return Err(format!("Ukuran {label} GitHub melampaui batas."));
    }
    let mut bytes = Vec::new();
    response
        .take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| format!("Gagal membaca {label} dari GitHub."))?;
    if bytes.len() > limit {
        return Err(format!("Ukuran {label} GitHub melampaui batas."));
    }
    Ok(bytes)
}

fn checksum_digest(
    client: &Client,
    release: &GithubRelease,
    pak_asset: &GithubAsset,
) -> PakResult<String> {
    let mut checksum_assets = release.assets.iter().filter(|asset| {
        asset.state == "uploaded" && asset.name.eq_ignore_ascii_case("SHA256sums.txt")
    });
    let asset = checksum_assets.next().ok_or_else(|| {
        "Metadata SHA-256 PAK tidak tersedia dan berkas SHA256sums.txt tidak ditemukan.".to_owned()
    })?;
    if checksum_assets.next().is_some() {
        return Err("Rilis GitHub memiliki lebih dari satu berkas SHA256sums.txt.".to_owned());
    }
    if asset.size > MAX_CHECKSUM_BYTES as u64 {
        return Err("Berkas SHA256sums.txt GitHub melampaui batas ukuran.".to_owned());
    }
    validate_asset_name(&asset.name)?;
    validate_asset_url(&asset.browser_download_url, &release.tag_name, &asset.name)?;

    let response = client
        .get(&asset.browser_download_url)
        .header("Accept", "text/plain")
        .timeout(Duration::from_secs(30))
        .send()
        .map_err(|_| "Gagal mengunduh berkas checksum GitHub.".to_owned())?;
    let response = ensure_success(response, "mengunduh berkas checksum")?;
    let body = read_bounded(response, MAX_CHECKSUM_BYTES, "berkas checksum")?;
    let text = std::str::from_utf8(&body)
        .map_err(|_| "Berkas checksum GitHub bukan teks UTF-8 yang valid.".to_owned())?;
    checksum_for_asset(text, &pak_asset.name)
}

fn checksum_for_asset(contents: &str, asset_name: &str) -> PakResult<String> {
    let mut digest = None;
    for line in contents.lines() {
        let line = line.trim_start();
        let Some(hash) = line.get(..64) else {
            continue;
        };
        let Some(separator) = line.get(64..65) else {
            continue;
        };
        if !separator.chars().all(char::is_whitespace) {
            continue;
        }
        let filename = line[65..].trim();
        let filename = filename.strip_prefix('*').unwrap_or(filename);
        if filename == asset_name {
            if digest.is_some() {
                return Err("Berkas checksum GitHub memiliki entri PAK duplikat.".to_owned());
            }
            digest = Some(normalize_sha256(hash)?);
        }
    }
    digest.ok_or_else(|| "Checksum SHA-256 untuk file PAK tidak ditemukan.".to_owned())
}

fn validate_release(release: &Release) -> PakResult<String> {
    let game_version = parse_game_version(&release.tag)?;
    if game_version != release.game_version {
        return Err("Versi game pada metadata rilis tidak sesuai tag.".to_owned());
    }
    validate_asset_name(&release.asset_name)?;
    if release.size == 0 {
        return Err("Ukuran file PAK pada metadata GitHub tidak valid.".to_owned());
    }
    validate_asset_url(&release.download_url, &release.tag, &release.asset_name)?;
    validate_github_url(
        &release.html_url,
        &["TitoTFP", "WuwaID", "releases", "tag", &release.tag],
    )?;
    normalize_sha256(&release.sha256)
}

fn validate_asset_name(name: &str) -> PakResult<()> {
    if name.is_empty()
        || name.len() > 255
        || name == "."
        || name == ".."
        || name.contains('/')
        || name.contains('\\')
        || name.chars().any(char::is_control)
        || !is_pak_name(name) && !name.eq_ignore_ascii_case("SHA256sums.txt")
    {
        return Err("Nama aset rilis GitHub tidak valid.".to_owned());
    }
    Ok(())
}

fn is_pak_name(name: &str) -> bool {
    name.to_ascii_lowercase().ends_with(".pak")
}

fn validate_asset_url(url: &str, tag: &str, asset_name: &str) -> PakResult<()> {
    validate_github_url(
        url,
        &["TitoTFP", "WuwaID", "releases", "download", tag, asset_name],
    )
}

fn validate_github_url(url: &str, expected_segments: &[&str]) -> PakResult<()> {
    let parsed =
        reqwest::Url::parse(url).map_err(|_| "URL rilis GitHub tidak valid.".to_owned())?;
    if parsed.scheme() != "https"
        || parsed.host_str() != Some(GITHUB_HOST)
        || !parsed.username().is_empty()
        || parsed.password().is_some()
        || parsed.port().is_some()
        || parsed.query().is_some()
        || parsed.fragment().is_some()
    {
        return Err(
            "URL rilis harus berasal dari repositori HTTPS GitHub yang tepercaya.".to_owned(),
        );
    }
    let expected = github_url(expected_segments)?;
    if parsed.path() != expected.path() {
        return Err("URL aset tidak cocok dengan repositori atau tag rilis GitHub.".to_owned());
    }
    Ok(())
}

fn github_url(segments: &[&str]) -> PakResult<reqwest::Url> {
    let mut url = reqwest::Url::parse("https://github.com/")
        .map_err(|_| "Tidak dapat membentuk URL GitHub.".to_owned())?;
    url.path_segments_mut()
        .map_err(|_| "Tidak dapat membentuk URL GitHub.".to_owned())?
        .pop_if_empty()
        .extend(segments.iter().copied());
    Ok(url)
}

fn normalize_sha256(digest: &str) -> PakResult<String> {
    let value = digest.strip_prefix("sha256:").unwrap_or(digest);
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("Digest SHA-256 pada metadata GitHub tidak valid.".to_owned());
    }
    Ok(value.to_ascii_lowercase())
}

fn cache_is_valid(path: &Path, expected_size: u64, expected_sha256: &str) -> io::Result<bool> {
    match fs::symlink_metadata(path) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error),
        Ok(metadata) if metadata.file_type().is_symlink() => Ok(false),
        Ok(metadata) if !metadata.is_file() => Ok(false),
        Ok(_) => match verify_file(path, expected_size, expected_sha256) {
            Ok(valid) => Ok(valid),
            Err(_) => Ok(false),
        },
    }
}

fn verify_file(path: &Path, expected_size: u64, expected_sha256: &str) -> io::Result<bool> {
    let mut file = File::open(path)?;
    if file.metadata()?.len() != expected_size {
        return Ok(false);
    }
    let mut hasher = Sha256::new();
    let mut total = 0_u64;
    let mut buffer = [0_u8; STREAM_BUFFER_BYTES];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        total = match total.checked_add(count as u64) {
            Some(total) if total <= expected_size => total,
            _ => return Ok(false),
        };
        hasher.update(&buffer[..count]);
    }
    Ok(total == expected_size && hex::encode(hasher.finalize()) == expected_sha256)
}

fn remove_cache_file(path: &Path) -> PakResult<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(_) => Err("Tidak dapat menghapus cache PAK yang tidak valid.".to_owned()),
    }
}

fn emit_progress(callback: &ProgressCallback<'_>, percent: u8, label: &str) {
    callback(Progress {
        percent,
        label: label.to_owned(),
    });
}

struct StagedFile {
    path: PathBuf,
    file: Option<File>,
    published: bool,
}

impl StagedFile {
    fn create(directory: &Path, cache_path: &Path) -> PakResult<Self> {
        let filename = cache_path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| "Nama file cache PAK tidak valid.".to_owned())?;
        for _ in 0..32 {
            let counter = STAGE_COUNTER.fetch_add(1, Ordering::Relaxed);
            let path = directory.join(format!(".{filename}.part-{}-{counter}", std::process::id()));
            match OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(file) => {
                    return Ok(Self {
                        path,
                        file: Some(file),
                        published: false,
                    })
                }
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(_) => {
                    return Err("Tidak dapat membuat file sementara untuk unduhan PAK.".to_owned())
                }
            }
        }
        Err("Tidak dapat membuat nama file sementara yang unik untuk PAK.".to_owned())
    }

    fn file_mut(&mut self) -> &mut File {
        self.file
            .as_mut()
            .expect("staged file remains open until publish")
    }

    fn publish(&mut self, cache_path: &Path) -> PakResult<()> {
        self.file.take();
        fs::rename(&self.path, cache_path).map_err(|_| {
            "Tidak dapat mempublikasikan file PAK terverifikasi ke cache.".to_owned()
        })?;
        self.published = true;
        Ok(())
    }
}

impl Drop for StagedFile {
    fn drop(&mut self) {
        self.file.take();
        if !self.published {
            let _ = fs::remove_file(&self.path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{checksum_for_asset, parse_game_version, verify_file};
    use sha2::{Digest, Sha256};
    use std::fs;

    fn sha256(bytes: &[u8]) -> String {
        hex::encode(Sha256::digest(bytes))
    }

    #[test]
    fn parses_tag_version_and_rejects_invalid_semver_boundaries() {
        assert_eq!(parse_game_version("v3.7.1-id.1").unwrap(), "3.7.1");
        assert_eq!(parse_game_version("v0.10.2").unwrap(), "0.10.2");
        for invalid in ["3.7.1", "v03.7.1", "v3.7", "v3.7.1.2", "v3.7.1-"] {
            assert!(parse_game_version(invalid).is_err(), "accepted {invalid}");
        }
    }

    #[test]
    fn checksum_lookup_matches_exact_asset_and_rejects_ambiguous_entries() {
        let hash = "a".repeat(64);
        let contents = format!("{hash}  another.pak\n{hash} *target.pak\n");
        assert_eq!(checksum_for_asset(&contents, "target.pak").unwrap(), hash);
        let duplicate = format!("{hash}  target.pak\n{hash}  target.pak\n");
        assert!(checksum_for_asset(&duplicate, "target.pak").is_err());
        assert!(checksum_for_asset(&contents, "missing.pak").is_err());
    }

    #[test]
    fn cached_file_requires_exact_size_and_sha256() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("patch.pak");
        let bytes = b"opaque PAK bytes";
        fs::write(&path, bytes).unwrap();
        let digest = sha256(bytes);

        assert!(verify_file(&path, bytes.len() as u64, &digest).unwrap());
        assert!(!verify_file(&path, bytes.len() as u64 + 1, &digest).unwrap());
        assert!(!verify_file(&path, bytes.len() as u64, &"0".repeat(64)).unwrap());
    }

    fn release_fixture(bytes: &[u8]) -> super::Release {
        super::Release {
            tag: "v3.7.1-id.1".into(),
            game_version: "3.7.1".into(),
            asset_name: "translation.pak".into(),
            download_url: "https://github.com/TitoTFP/WuwaID/releases/download/v3.7.1-id.1/translation.pak".into(),
            html_url: "https://github.com/TitoTFP/WuwaID/releases/tag/v3.7.1-id.1".into(),
            size: bytes.len() as u64,
            sha256: sha256(bytes),
        }
    }

    #[test]
    fn public_download_reuses_verified_cache_without_network() {
        let root = tempfile::tempdir().unwrap();
        let bytes = b"verified offline translation";
        let mut release = release_fixture(bytes);
        release.sha256 = format!("sha256:{}", release.sha256.to_ascii_uppercase());
        let cache = root.path().join(format!("{}-{}", release.tag, release.asset_name));
        fs::write(&cache, bytes).unwrap();
        let path = super::download(&release, root.path(), &|_| {}).unwrap();
        assert_eq!(path, cache);
        assert_eq!(fs::read(&path).unwrap(), bytes);
    }

    #[test]
    fn invalid_release_metadata_is_rejected_before_creating_or_mutating_cache() {
        let root = tempfile::tempdir().unwrap();
        let untouched = root.path().join("unrelated");
        fs::write(&untouched, b"keep").unwrap();
        let valid = release_fixture(b"payload");
        let mut invalid = Vec::new();
        for url in [
            "http://github.com/TitoTFP/WuwaID/releases/download/v3.7.1-id.1/translation.pak",
            "https://evil.example/translation.pak",
            "https://github.com/TitoTFP/WuwaIDMobile/releases/download/v3.7.1-id.1/translation.pak",
            "https://github.com/TitoTFP/WuwaID/releases/download/v3.7.2-id.1/translation.pak",
            "https://user@github.com/TitoTFP/WuwaID/releases/download/v3.7.1-id.1/translation.pak",
            "https://github.com:444/TitoTFP/WuwaID/releases/download/v3.7.1-id.1/translation.pak",
            "https://github.com/TitoTFP/WuwaID/releases/download/v3.7.1-id.1/translation.pak?token=x",
            "https://github.com/TitoTFP/WuwaID/releases/download/v3.7.1-id.1/translation.pak#fragment",
        ] {
            let mut release = valid.clone();
            release.download_url = url.into();
            invalid.push(release);
        }
        for name in ["../translation.pak", "other\\translation.pak", "translation.apk", "bad\n.pak"] {
            let mut release = valid.clone();
            release.asset_name = name.into();
            invalid.push(release);
        }
        let mut wrong_version = valid.clone();
        wrong_version.game_version = "3.8.0".into();
        invalid.push(wrong_version);
        let mut empty = valid.clone();
        empty.size = 0;
        invalid.push(empty);
        let mut bad_hash = valid.clone();
        bad_hash.sha256 = "g".repeat(64);
        invalid.push(bad_hash);
        let mut wrong_page = valid;
        wrong_page.html_url = "https://github.com/TitoTFP/WuwaID/releases/tag/v3.7.2-id.1".into();
        invalid.push(wrong_page);
        let cache = root.path().join("not-yet-created");
        for release in invalid {
            assert!(super::download(&release, &cache, &|_| {}).is_err(), "{release:?}");
            assert!(!cache.exists());
            assert_eq!(fs::read(&untouched).unwrap(), b"keep");
        }
    }

    #[test]
    fn abandoned_or_failed_publication_removes_only_its_own_partial() {
        use std::io::Write;
        let root = tempfile::tempdir().unwrap();
        let cache = root.path().join("translation.pak");
        fs::write(&cache, b"previous cache").unwrap();
        let partial;
        {
            let mut stage = super::StagedFile::create(root.path(), &cache).unwrap();
            partial = stage.path.clone();
            stage.file_mut().write_all(b"unfinished").unwrap();
        }
        assert!(!partial.exists());
        assert_eq!(fs::read(&cache).unwrap(), b"previous cache");
        let directory = root.path().join("directory-collision");
        fs::create_dir(&directory).unwrap();
        fs::write(directory.join("keep"), b"unrelated").unwrap();
        let failed;
        {
            let mut stage = super::StagedFile::create(root.path(), &cache).unwrap();
            failed = stage.path.clone();
            stage.file_mut().write_all(b"cannot publish").unwrap();
            assert!(stage.publish(&directory).is_err());
        }
        assert!(!failed.exists());
        assert_eq!(fs::read(directory.join("keep")).unwrap(), b"unrelated");
        assert_eq!(fs::read(&cache).unwrap(), b"previous cache");
    }

    #[test]
    fn successful_stage_publication_replaces_cache_with_exact_new_bytes() {
        use std::io::Write;
        let root = tempfile::tempdir().unwrap();
        let cache = root.path().join("translation.pak");
        fs::write(&cache, b"old").unwrap();
        let partial;
        {
            let mut stage = super::StagedFile::create(root.path(), &cache).unwrap();
            partial = stage.path.clone();
            stage.file_mut().write_all(b"verified replacement").unwrap();
            stage.file_mut().sync_all().unwrap();
            stage.publish(&cache).unwrap();
        }
        assert!(!partial.exists());
        assert_eq!(fs::read(&cache).unwrap(), b"verified replacement");
    }

    #[cfg(unix)]
    #[test]
    fn cache_verification_rejects_symlinks_even_when_target_digest_matches() {
        use std::os::unix::fs::symlink;
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("outside-cache");
        fs::write(&source, b"valid bytes").unwrap();
        let link = root.path().join("cache.pak");
        symlink(&source, &link).unwrap();
        assert!(!super::cache_is_valid(&link, 11, &sha256(b"valid bytes")).unwrap());
        assert_eq!(fs::read(&source).unwrap(), b"valid bytes");
    }
}
