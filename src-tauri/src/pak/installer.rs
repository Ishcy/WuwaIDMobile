use std::collections::HashSet;
use std::fs::{self, File};
use std::io::Read;
use std::path::Path;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::model::{
    InstallStatus, PakResult, Progress, ProgressCallback, Release, ResourceLayout, MOUNT_FILENAME,
    PATCH_DIRECTORY, PATCH_FILENAME, PRIORITY, SIGNATURE_FILENAME,
};
use super::storage::Storage;
use super::uid;

const SCHEMA_VERSION: u8 = 1;
const JOURNAL_LIMIT: u64 = 64 * 1024;
const OWNER_LIMIT: u64 = 16 * 1024;
const SLOTS: [Slot; 4] = [Slot::Pak, Slot::Signature, Slot::Owner, Slot::Mount];

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Owner {
    schema_version: u8,
    resource_version: String,
    patch_directory: String,
    pak_filename: String,
    signature_filename: String,
    mount_filename: String,
    priority: u16,
    release_tag: String,
    game_version: String,
    asset_name: String,
    pak_sha256: String,
    pak_sha1: String,
    signature_sha256: String,
    signature_sha1: String,
    mount_sha256: String,
    #[serde(default)]
    source_sha256: Option<String>,
    #[serde(default)]
    custom_uid: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
enum Operation {
    Install,
    Uninstall,
}

#[derive(Clone, Copy, Debug)]
enum Slot {
    Pak,
    Signature,
    Owner,
    Mount,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct Snapshot {
    exists: bool,
    sha256: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct Journal {
    schema_version: u8,
    resource_version: String,
    operation: Operation,
    before: Vec<Snapshot>,
    after: Vec<Option<String>>,
}

pub fn discover(storage: &dyn Storage) -> PakResult<ResourceLayout> {
    let versions = resource_versions(storage)?;
    for (_, version) in versions.into_iter().rev() {
        if !is_materialized(storage, &version)? {
            continue;
        }
        if let Some(signature) = find_official_signature(storage, &version)? {
            return Ok(ResourceLayout { version, signature });
        }
    }
    Err("No materialized game resource version with an official PAK signature was found".to_owned())
}

pub fn inspect(storage: &dyn Storage, layout: &ResourceLayout) -> PakResult<InstallStatus> {
    validate_version(&layout.version)?;
    let paths = target_paths(&layout.version);
    let mut issues = Vec::new();
    let mut recovery_pending = false;
    for (_, version) in resource_versions(storage)? {
        if has_lifecycle_files(storage, &version)? {
            recovery_pending = true;
            break;
        }
    }
    let mut installed = false;
    for path in &paths {
        installed |= file_exists(storage, path)?;
    }
    for path in artifact_lifecycle_paths(&layout.version) {
        installed |= file_exists(storage, &path)?;
    }

    let owner_path = target_path(&layout.version, Slot::Owner);
    let owner = match load_owner(storage, &layout.version) {
        Ok(owner) => owner,
        Err(error) => {
            issues.push(error);
            None
        }
    };
    let version = owner
        .as_ref()
        .map(|owner| owner.release_tag.clone())
        .unwrap_or_else(|| layout.version.clone());
    let conflicts = find_conflicting_mounts(storage, &layout.version)?;
    if !conflicts.is_empty() {
        issues.push(format!(
            "Conflicting custom mounts: {}",
            conflicts.join(", ")
        ));
    }
    if recovery_pending {
        issues.push("An installer transaction or orphan lifecycle file needs recovery".to_owned());
    }

    let mut custom_uid = None;
    match owner {
        None if installed => issues.push(format!(
            "Installer artifacts exist without valid ownership metadata at {owner_path}"
        )),
        Some(owner) => {
            if !has_current_mount_format(&owner) {
                issues.push("Format mount lama memakai ekstensi .pak. Pilih Perbaiki pemasangan untuk memperbaruinya.".to_owned());
            }
            if let Err(error) = verify_owned_content(storage, &layout.version, &owner) {
                issues.push(error);
            }
            custom_uid = owner.custom_uid;
        }
        None => {}
    }

    let healthy = installed && issues.is_empty() && !recovery_pending;
    Ok(InstallStatus {
        installed,
        healthy,
        recovery_pending,
        version,
        message: if issues.is_empty() {
            None
        } else {
            Some(issues.join("; "))
        },
        custom_uid,
    })
}

pub fn install(
    storage: &dyn Storage,
    layout: &ResourceLayout,
    release: &Release,
    pak: &Path,
    custom_uid: Option<&str>,
    progress: &ProgressCallback,
) -> PakResult<InstallStatus> {
    let custom_uid = custom_uid.map(uid::validate_text).transpose()?;
    recover(storage, layout, progress)?;
    install_current(storage, layout, release, pak, custom_uid, progress)
}

pub fn repair(
    storage: &dyn Storage,
    layout: &ResourceLayout,
    release: &Release,
    pak: &Path,
    custom_uid: Option<&str>,
    progress: &ProgressCallback,
) -> PakResult<InstallStatus> {
    let custom_uid = custom_uid.map(uid::validate_text).transpose()?;
    recover(storage, layout, progress)?;
    install_current(storage, layout, release, pak, custom_uid, progress)
}

pub fn recover(
    storage: &dyn Storage,
    layout: &ResourceLayout,
    progress: &ProgressCallback,
) -> PakResult<InstallStatus> {
    validate_version(&layout.version)?;
    let mut pending = Vec::new();
    for (_, version) in resource_versions(storage)? {
        if let Some(journal) = read_pending_journal(storage, &version)? {
            pending.push((version, journal));
        } else if has_artifact_lifecycle_files(storage, &version)? {
            return Err(format!(
                "Orphan installer lifecycle files found for resource version {version}; refusing to guess ownership"
            ));
        }
    }

    if !pending.is_empty() {
        storage.force_stop()?;
        notify(progress, 68, "Memulihkan transaksi pemasangan");
        for (version, journal) in pending {
            if fully_applied(storage, &version, &journal)? {
                finalize_journal(storage, &version, &journal)?;
            } else {
                rollback_journal(storage, &version, &journal)?;
            }
        }
        notify(progress, 72, "Transaksi pemasangan dipulihkan");
    }
    inspect(storage, layout)
}

pub fn uninstall(
    storage: &dyn Storage,
    layout: &ResourceLayout,
    progress: &ProgressCallback,
) -> PakResult<InstallStatus> {
    recover(storage, layout, progress)?;
    validate_version(&layout.version)?;
    ensure_no_lifecycle_files(storage, &layout.version)?;

    let owner = load_owner(storage, &layout.version)?;
    let Some(owner) = owner else {
        if paths_have_artifacts(storage, &layout.version)? {
            return Err(
                "Refusing to uninstall unowned or unknown files at the canonical patch paths"
                    .to_owned(),
            );
        }
        return inspect(storage, layout);
    };
    verify_owned_content_for_uninstall(storage, &layout.version, &owner)?;
    let before = capture_snapshots(storage, &layout.version)?;
    let journal = Journal {
        schema_version: SCHEMA_VERSION,
        resource_version: layout.version.clone(),
        operation: Operation::Uninstall,
        before,
        after: vec![None; SLOTS.len()],
    };
    validate_journal(&journal, &layout.version)?;

    storage.force_stop()?;
    notify(progress, 78, "Menyiapkan penghapusan berkas milik aplikasi");
    write_initial_journal(storage, &journal)?;
    match apply_journal(storage, &layout.version, &journal) {
        Ok(()) => {
            notify(progress, 96, "Memverifikasi penghapusan");
            finalize_journal(storage, &layout.version, &journal)?;
        }
        Err(operation_error) => {
            return Err(recover_after_failure(
                storage,
                layout,
                progress,
                operation_error,
            ));
        }
    }
    notify(progress, 100, "Penghapusan selesai");
    inspect(storage, layout)
}

fn install_current(
    storage: &dyn Storage,
    layout: &ResourceLayout,
    release: &Release,
    pak: &Path,
    custom_uid: Option<&str>,
    progress: &ProgressCallback,
) -> PakResult<InstallStatus> {
    validate_version(&layout.version)?;
    validate_release(release, pak)?;
    ensure_no_lifecycle_files(storage, &layout.version)?;

    let official_signature =
        find_official_signature(storage, &layout.version)?.ok_or_else(|| {
            "No official sibling PAK/signature pair was found for this resource version".to_owned()
        })?;
    if official_signature != layout.signature {
        return Err(
            "The selected official signature no longer matches the discovered resource tree"
                .to_owned(),
        );
    }
    let signature_stats = storage.stat(&official_signature)?;
    if !signature_stats.exists || signature_stats.is_dir || signature_stats.size == 0 {
        return Err("The selected official signature is missing or empty".to_owned());
    }
    let official_signature_sha256 = checked_hash(storage, &official_signature, "sha256")?;
    let official_signature_sha1 = checked_hash(storage, &official_signature, "sha1")?;

    let owner = load_owner(storage, &layout.version)?;
    if owner.is_none() && paths_have_artifacts(storage, &layout.version)? {
        return Err(
            "Refusing to overwrite canonical patch files without valid ownership metadata"
                .to_owned(),
        );
    }
    for path in &target_paths(&layout.version) {
        let stat = storage.stat(path)?;
        if stat.exists && stat.is_dir {
            return Err(format!("Canonical installer target is a directory: {path}"));
        }
    }
    let conflicts = find_conflicting_mounts(storage, &layout.version)?;
    if !conflicts.is_empty() {
        return Err(format!(
            "Conflicting custom mounts: {}",
            conflicts.join(", ")
        ));
    }

    let source_sha256 = hash_local_file(pak)?;
    if !source_sha256.eq_ignore_ascii_case(&release.sha256) {
        return Err("Downloaded PAK SHA-256 does not match release metadata".to_owned());
    }
    let source_stats =
        fs::metadata(pak).map_err(|error| format!("Could not inspect downloaded PAK: {error}"))?;
    if source_stats.len() != release.size {
        return Err(format!(
            "Downloaded PAK size mismatch: expected {}, found {}",
            release.size,
            source_stats.len()
        ));
    }
    let derived = if let Some(text) = custom_uid {
        notify(progress, 70, "Menyiapkan UID kustom");
        Some(uid::prepare(pak, text)?)
    } else {
        None
    };
    let payload = derived.as_ref().map(|pak| pak.path()).unwrap_or(pak);
    let expected_sha256 = if derived.is_some() {
        hash_local_file(payload)?
    } else {
        source_sha256.clone()
    };
    let expected_size = derived.as_ref().map(|pak| pak.size).unwrap_or(release.size);

    storage.force_stop()?;
    notify(progress, 74, "Menyiapkan PAK dan signature terverifikasi");
    let patch_directory = format!("{}/{PATCH_DIRECTORY}", layout.version);
    storage.mkdir(&patch_directory)?;

    let stages = stage_paths(&layout.version);
    let mut staged_hashes: Vec<Option<String>> = vec![None; SLOTS.len()];
    let stage_result = (|| {
        storage.write_from_file(&stages[0], payload)?;
        staged_hashes[0] = Some(expected_sha256.clone());
        let staged_pak_sha256 = checked_hash(storage, &stages[0], "sha256")?;
        staged_hashes[0] = Some(staged_pak_sha256.clone());
        if staged_pak_sha256 != expected_sha256 {
            return Err("Staged PAK SHA-256 does not match the verified payload".to_owned());
        }
        if storage.stat(&stages[0])?.size != expected_size {
            return Err("Staged PAK size does not match the verified payload".to_owned());
        }

        storage.copy(&official_signature, &stages[1])?;
        staged_hashes[1] = Some(official_signature_sha256.clone());
        let staged_signature_sha256 = checked_hash(storage, &stages[1], "sha256")?;
        staged_hashes[1] = Some(staged_signature_sha256.clone());
        if staged_signature_sha256 != official_signature_sha256
            || storage.stat(&stages[1])?.size != signature_stats.size
        {
            return Err("Cloned signature did not verify against its official source".to_owned());
        }

        let pak_sha1 = checked_hash(storage, &stages[0], "sha1")?.to_ascii_uppercase();
        let signature_sha1 = checked_hash(storage, &stages[1], "sha1")?.to_ascii_uppercase();
        if signature_sha1.to_ascii_lowercase() != official_signature_sha1 {
            return Err("Cloned signature SHA-1 does not match its official source".to_owned());
        }
        let mount = mount_content(&pak_sha1, &signature_sha1);
        let expected_mount_sha256 = sha256_text(&mount);
        storage.write_text(&stages[3], &mount)?;
        staged_hashes[3] = Some(expected_mount_sha256.clone());
        let staged_mount_sha256 = checked_hash(storage, &stages[3], "sha256")?;
        staged_hashes[3] = Some(staged_mount_sha256.clone());
        if staged_mount_sha256 != expected_mount_sha256 || storage.read_text(&stages[3])? != mount {
            return Err("Staged mount descriptor failed exact-content verification".to_owned());
        }

        let owner = Owner {
            schema_version: SCHEMA_VERSION,
            resource_version: layout.version.clone(),
            patch_directory: PATCH_DIRECTORY.to_owned(),
            pak_filename: PATCH_FILENAME.to_owned(),
            signature_filename: SIGNATURE_FILENAME.to_owned(),
            mount_filename: MOUNT_FILENAME.to_owned(),
            priority: PRIORITY,
            release_tag: release.tag.clone(),
            game_version: release.game_version.clone(),
            asset_name: release.asset_name.clone(),
            pak_sha256: expected_sha256.clone(),
            pak_sha1,
            signature_sha256: staged_hashes[1].clone().expect("signature stage hash"),
            signature_sha1,
            mount_sha256: staged_hashes[3].clone().expect("mount stage hash"),
            source_sha256: Some(source_sha256.clone()),
            custom_uid: custom_uid.map(str::to_owned),
        };
        validate_owner(&owner, &layout.version)?;
        let owner_json = serde_json::to_string(&owner).map_err(|error| {
            format!("Could not serialize installer ownership metadata: {error}")
        })?;
        let expected_owner_sha256 = sha256_text(&owner_json);
        storage.write_text(&stages[2], &owner_json)?;
        staged_hashes[2] = Some(expected_owner_sha256.clone());
        let staged_owner_sha256 = checked_hash(storage, &stages[2], "sha256")?;
        staged_hashes[2] = Some(staged_owner_sha256.clone());
        if staged_owner_sha256 != expected_owner_sha256 {
            return Err("Staged ownership metadata failed verification".to_owned());
        }

        for index in 0..SLOTS.len() {
            let expected = staged_hashes[index]
                .as_deref()
                .ok_or_else(|| "A staged installer file has no verified hash".to_owned())?;
            if checked_hash(storage, &stages[index], "sha256")? != expected {
                return Err(format!(
                    "Staged installer file failed verification: {}",
                    stages[index]
                ));
            }
        }
        Ok(())
    })();
    if let Err(error) = stage_result {
        let cleanup = clean_stages(storage, &stages, &staged_hashes);
        return Err(append_cleanup_error(error, cleanup));
    }
    notify(progress, 80, "Berkas sementara terverifikasi");

    let before = match capture_snapshots(storage, &layout.version) {
        Ok(before) => before,
        Err(error) => {
            let cleanup = clean_stages(storage, &stages, &staged_hashes);
            return Err(append_cleanup_error(error, cleanup));
        }
    };
    let journal = Journal {
        schema_version: SCHEMA_VERSION,
        resource_version: layout.version.clone(),
        operation: Operation::Install,
        before,
        after: staged_hashes,
    };
    if let Err(error) = validate_journal(&journal, &layout.version)
        .and_then(|()| verify_journal_payload(storage, &layout.version, &journal))
    {
        let cleanup = clean_stages(storage, &stages, &journal.after);
        return Err(append_cleanup_error(error, cleanup));
    }
    notify(progress, 84, "Menyimpan jurnal transaksi");
    if let Err(error) = write_initial_journal(storage, &journal) {
        let mut journal_published = false;
        for path in journal_lifecycle_paths(&layout.version) {
            journal_published |= file_exists(storage, &path)?;
        }
        if journal_published {
            return Err(format!(
                "{error}; staged outputs were retained for transaction recovery"
            ));
        }
        let cleanup = clean_stages(storage, &stages, &journal.after);
        return Err(append_cleanup_error(error, cleanup));
    }

    match apply_journal(storage, &layout.version, &journal) {
        Ok(()) => {
            notify(progress, 96, "Memverifikasi hasil pemasangan");
            if let Err(error) = verify_committed_install(storage, &layout.version, &journal) {
                return Err(recover_after_failure(storage, layout, progress, error));
            }
            finalize_journal(storage, &layout.version, &journal)?;
            notify(progress, 100, "Pemasangan selesai");
            inspect(storage, layout)
        }
        Err(error) => Err(recover_after_failure(storage, layout, progress, error)),
    }
}

fn validate_release(release: &Release, pak: &Path) -> PakResult<()> {
    if !valid_hash(&release.sha256, 32) {
        return Err("Release SHA-256 metadata is malformed".to_owned());
    }
    if release.size == 0 {
        return Err("Release PAK size must be nonzero".to_owned());
    }
    if release.tag.trim().is_empty() || release.asset_name.trim().is_empty() {
        return Err("Release identity metadata is incomplete".to_owned());
    }
    let metadata = fs::symlink_metadata(pak)
        .map_err(|error| format!("Downloaded PAK is unavailable: {error}"))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err("Downloaded PAK must be a regular non-symlink file".to_owned());
    }
    if metadata.len() != release.size {
        return Err(format!(
            "Downloaded PAK size mismatch: expected {}, found {}",
            release.size,
            metadata.len()
        ));
    }
    Ok(())
}

fn resource_versions(storage: &dyn Storage) -> PakResult<Vec<((u64, u64, u64), String)>> {
    let mut versions = Vec::new();
    for entry in storage.list("")? {
        if !entry.is_dir {
            continue;
        }
        if let Some(parsed) = parse_version(&entry.name) {
            versions.push((parsed, entry.name));
        }
    }
    versions.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(versions)
}

fn is_materialized(storage: &dyn Storage, version: &str) -> PakResult<bool> {
    let manifest = storage.stat(&format!("{version}/ResManifest"))?;
    let mount_dir = storage.stat(&format!("{version}/Mount"))?;
    Ok(manifest.exists && mount_dir.exists && mount_dir.is_dir)
}

fn find_official_signature(storage: &dyn Storage, version: &str) -> PakResult<Option<String>> {
    let english = format!("{version}/Lang_en");
    if let Some(signature) = find_signature_pair(storage, &english)? {
        return Ok(Some(signature));
    }
    let resource = format!("{version}/Resource");
    find_signature_pair(storage, &resource)
}

fn find_signature_pair(storage: &dyn Storage, root: &str) -> PakResult<Option<String>> {
    let root_stat = storage.stat(root)?;
    if !root_stat.exists {
        return Ok(None);
    }
    if !root_stat.is_dir {
        return Err(format!(
            "Official resource directory is not a directory: {root}"
        ));
    }
    let mut pending = vec![root.to_owned()];
    let mut visited = HashSet::new();
    let mut matches = Vec::new();
    while let Some(directory) = pending.pop() {
        if !visited.insert(directory.clone()) {
            continue;
        }
        let mut entries = storage.list(&directory)?;
        entries.sort_by(|left, right| left.name.cmp(&right.name));
        let pak_stems: HashSet<String> = entries
            .iter()
            .filter(|entry| !entry.is_dir)
            .filter_map(|entry| strip_extension_ignore_case(&entry.name, ".pak"))
            .map(|stem| stem.to_ascii_lowercase())
            .collect();
        for entry in &entries {
            validate_child_name(&entry.name)?;
            if entry.is_dir {
                if !is_owned_or_mount_directory(&entry.name) {
                    pending.push(format!("{directory}/{}", entry.name));
                }
                continue;
            }
            let Some(stem) = strip_extension_ignore_case(&entry.name, ".sig") else {
                continue;
            };
            if pak_stems.contains(&stem.to_ascii_lowercase()) {
                matches.push(format!("{directory}/{}", entry.name));
            }
        }
    }
    matches.sort();
    Ok(matches.into_iter().next())
}

fn validate_child_name(name: &str) -> PakResult<()> {
    if name.is_empty() || name == "." || name == ".." || name.contains('/') || name.contains('\\') {
        return Err(format!(
            "Unsafe child name returned by resource storage: {name}"
        ));
    }
    Ok(())
}

fn is_owned_or_mount_directory(name: &str) -> bool {
    name.eq_ignore_ascii_case(PATCH_DIRECTORY) || name.eq_ignore_ascii_case("Mount")
}

fn strip_extension_ignore_case<'a>(name: &'a str, extension: &str) -> Option<&'a str> {
    if name.len() < extension.len() {
        return None;
    }
    let split_at = name.len() - extension.len();
    name[split_at..]
        .eq_ignore_ascii_case(extension)
        .then_some(&name[..split_at])
}

fn find_conflicting_mounts(storage: &dyn Storage, version: &str) -> PakResult<Vec<String>> {
    let mount_dir = format!("{version}/Mount");
    let stat = storage.stat(&mount_dir)?;
    if !stat.exists {
        return Ok(Vec::new());
    }
    if !stat.is_dir {
        return Err(format!("Game mount path is not a directory: {mount_dir}"));
    }
    let mut conflicts = Vec::new();
    for entry in storage.list(&mount_dir)? {
        validate_child_name(&entry.name)?;
        if entry.is_dir
            || entry.name.eq_ignore_ascii_case(MOUNT_FILENAME)
            || entry.name.to_ascii_lowercase().starts_with("mountresource")
            || entry.name.to_ascii_lowercase().starts_with("mountlang_")
            || !entry.name.to_ascii_lowercase().ends_with(".txt")
        {
            continue;
        }
        let path = format!("{mount_dir}/{}", entry.name);
        let descriptor = storage.read_text(&path)?;
        if descriptor.lines().any(is_high_priority_mount_line) {
            conflicts.push(format!("Mount/{}", entry.name));
        }
    }
    conflicts.sort();
    Ok(conflicts)
}

fn is_high_priority_mount_line(line: &str) -> bool {
    let mut fields = line.split(',');
    let _ = fields.next();
    fields
        .next()
        .and_then(|priority| priority.trim().parse::<u64>().ok())
        .is_some_and(|priority| priority >= u64::from(PRIORITY))
}

fn target_paths(version: &str) -> Vec<String> {
    SLOTS
        .iter()
        .map(|slot| target_path(version, *slot))
        .collect()
}

fn stage_paths(version: &str) -> Vec<String> {
    target_paths(version)
        .into_iter()
        .map(|path| format!("{path}.new"))
        .collect()
}

fn backup_paths(version: &str) -> Vec<String> {
    target_paths(version)
        .into_iter()
        .map(|path| format!("{path}.bak"))
        .collect()
}

fn target_path(version: &str, slot: Slot) -> String {
    match slot {
        Slot::Pak => format!("{version}/{PATCH_DIRECTORY}/{PATCH_FILENAME}"),
        Slot::Signature => format!("{version}/{PATCH_DIRECTORY}/{SIGNATURE_FILENAME}"),
        Slot::Owner => format!("{version}/{PATCH_DIRECTORY}/.owner.json"),
        Slot::Mount => format!("{version}/Mount/{MOUNT_FILENAME}"),
    }
}

fn journal_path(version: &str) -> String {
    format!("{version}/{PATCH_DIRECTORY}/.transaction.json")
}

fn journal_lifecycle_paths(version: &str) -> [String; 3] {
    let path = journal_path(version);
    [path.clone(), format!("{path}.new"), format!("{path}.bak")]
}

fn artifact_lifecycle_paths(version: &str) -> Vec<String> {
    let mut paths = Vec::with_capacity(SLOTS.len() * 2);
    for path in target_paths(version) {
        paths.push(format!("{path}.new"));
        paths.push(format!("{path}.bak"));
    }
    paths
}

fn all_lifecycle_paths(version: &str) -> Vec<String> {
    let mut paths = artifact_lifecycle_paths(version);
    paths.extend(journal_lifecycle_paths(version));
    paths
}

fn has_lifecycle_files(storage: &dyn Storage, version: &str) -> PakResult<bool> {
    for path in all_lifecycle_paths(version) {
        if file_exists(storage, &path)? {
            return Ok(true);
        }
    }
    Ok(false)
}

fn has_artifact_lifecycle_files(storage: &dyn Storage, version: &str) -> PakResult<bool> {
    for path in artifact_lifecycle_paths(version) {
        if file_exists(storage, &path)? {
            return Ok(true);
        }
    }
    Ok(false)
}

fn ensure_no_lifecycle_files(storage: &dyn Storage, version: &str) -> PakResult<()> {
    for path in all_lifecycle_paths(version) {
        if file_exists(storage, &path)? {
            return Err(format!(
                "Unexpected installer lifecycle path {path}; recover or remove it only after verifying ownership"
            ));
        }
    }
    Ok(())
}

fn paths_have_artifacts(storage: &dyn Storage, version: &str) -> PakResult<bool> {
    for path in target_paths(version) {
        if file_exists(storage, &path)? {
            return Ok(true);
        }
    }
    Ok(false)
}

fn file_exists(storage: &dyn Storage, path: &str) -> PakResult<bool> {
    Ok(storage.stat(path)?.exists)
}

fn load_owner(storage: &dyn Storage, version: &str) -> PakResult<Option<Owner>> {
    let path = target_path(version, Slot::Owner);
    let stat = storage.stat(&path)?;
    if !stat.exists {
        return Ok(None);
    }
    if stat.is_dir || stat.size > OWNER_LIMIT {
        return Err(format!(
            "Ownership metadata is not a bounded regular file: {path}"
        ));
    }
    let content = storage.read_text(&path)?;
    let owner: Owner = serde_json::from_str(&content)
        .map_err(|error| format!("Ownership metadata is malformed: {error}"))?;
    validate_owner(&owner, version)?;
    Ok(Some(owner))
}

fn validate_owner(owner: &Owner, version: &str) -> PakResult<()> {
    if owner.schema_version != SCHEMA_VERSION
        || owner.resource_version != version
        || owner.patch_directory != PATCH_DIRECTORY
        || owner.pak_filename != PATCH_FILENAME
        || owner.signature_filename != SIGNATURE_FILENAME
        || owner.mount_filename != MOUNT_FILENAME
        || owner.priority != PRIORITY
    {
        return Err("Ownership metadata does not describe the canonical WuwaID install".to_owned());
    }
    if owner.release_tag.trim().is_empty()
        || owner.asset_name.trim().is_empty()
        || !valid_hash(&owner.pak_sha256, 32)
        || !valid_hash(&owner.pak_sha1, 20)
        || !valid_hash(&owner.signature_sha256, 32)
        || !valid_hash(&owner.signature_sha1, 20)
        || !valid_hash(&owner.mount_sha256, 32)
    {
        return Err("Ownership metadata contains invalid identity or content hashes".to_owned());
    }
    if let Some(source_hash) = &owner.source_sha256 {
        if !valid_hash(source_hash, 32)
            || (owner.custom_uid.is_none() && !source_hash.eq_ignore_ascii_case(&owner.pak_sha256))
        {
            return Err("Ownership metadata contains an invalid source PAK hash".into());
        }
    }
    if let Some(text) = &owner.custom_uid {
        if owner.source_sha256.is_none() || uid::validate_text(text)? != text {
            return Err("Ownership metadata contains an invalid custom UID".into());
        }
    }
    if !has_current_mount_format(owner) {
        // Previous releases persisted a filename instead of a mount base. Decode
        // that exact owned format only for repair, removal and lossless rollback.
        let previous_descriptor = format!(
            "::Mount::\n{PATCH_DIRECTORY}/{PATCH_FILENAME},{PRIORITY},{},{},,\n::Del::\n",
            owner.pak_sha1.to_ascii_uppercase(),
            owner.signature_sha1.to_ascii_uppercase()
        );
        if !sha256_text(&previous_descriptor).eq_ignore_ascii_case(&owner.mount_sha256) {
            return Err(
                "Ownership metadata does not match a supported mount descriptor".to_owned(),
            );
        }
    }
    Ok(())
}
fn has_current_mount_format(owner: &Owner) -> bool {
    sha256_text(&mount_content(
        &owner.pak_sha1.to_ascii_uppercase(),
        &owner.signature_sha1.to_ascii_uppercase(),
    ))
    .eq_ignore_ascii_case(&owner.mount_sha256)
}

fn verify_owned_content(storage: &dyn Storage, version: &str, owner: &Owner) -> PakResult<()> {
    validate_owner(owner, version)?;
    let expected = [
        Some(owner.pak_sha256.to_ascii_lowercase()),
        Some(owner.signature_sha256.to_ascii_lowercase()),
        None,
        Some(owner.mount_sha256.to_ascii_lowercase()),
    ];
    let expected_sha1 = [
        Some(owner.pak_sha1.to_ascii_lowercase()),
        Some(owner.signature_sha1.to_ascii_lowercase()),
        None,
        None,
    ];
    let paths = target_paths(version);
    for index in 0..SLOTS.len() {
        let stat = storage.stat(&paths[index])?;
        if !stat.exists {
            return Err(format!("Owned installer file is missing: {}", paths[index]));
        }
        if stat.is_dir {
            return Err(format!(
                "Owned installer path is not a file: {}",
                paths[index]
            ));
        }
        if let Some(expected_hash) = &expected[index] {
            if checked_hash(storage, &paths[index], "sha256")? != *expected_hash {
                return Err(format!(
                    "Owned installer file failed SHA-256 verification: {}",
                    paths[index]
                ));
            }
        }
        if let Some(expected_hash) = &expected_sha1[index] {
            if checked_hash(storage, &paths[index], "sha1")? != *expected_hash {
                return Err(format!(
                    "Owned installer file failed SHA-1 verification: {}",
                    paths[index]
                ));
            }
        }
    }
    Ok(())
}
fn verify_owned_content_for_uninstall(
    storage: &dyn Storage,
    version: &str,
    owner: &Owner,
) -> PakResult<()> {
    validate_owner(owner, version)?;
    let paths = target_paths(version);
    let expected = [
        Some(owner.pak_sha256.to_ascii_lowercase()),
        Some(owner.signature_sha256.to_ascii_lowercase()),
        None,
        Some(owner.mount_sha256.to_ascii_lowercase()),
    ];
    let expected_sha1 = [
        Some(owner.pak_sha1.to_ascii_lowercase()),
        Some(owner.signature_sha1.to_ascii_lowercase()),
        None,
        None,
    ];
    for index in 0..SLOTS.len() {
        let stat = storage.stat(&paths[index])?;
        if !stat.exists {
            if index == 2 {
                return Err("Ownership metadata disappeared before uninstall".to_owned());
            }
            continue;
        }
        if stat.is_dir {
            return Err(format!(
                "Owned installer path is not a file: {}",
                paths[index]
            ));
        }
        if let Some(hash) = &expected[index] {
            if checked_hash(storage, &paths[index], "sha256")? != *hash {
                return Err(format!(
                    "Modified file blocks safe uninstall: {}",
                    paths[index]
                ));
            }
        }
        if let Some(hash) = &expected_sha1[index] {
            if checked_hash(storage, &paths[index], "sha1")? != *hash {
                return Err(format!(
                    "Modified file blocks safe uninstall: {}",
                    paths[index]
                ));
            }
        }
    }
    Ok(())
}

fn capture_snapshots(storage: &dyn Storage, version: &str) -> PakResult<Vec<Snapshot>> {
    let mut snapshots = Vec::with_capacity(SLOTS.len());
    for path in target_paths(version) {
        let stat = storage.stat(&path)?;
        if !stat.exists {
            snapshots.push(Snapshot {
                exists: false,
                sha256: None,
            });
        } else {
            if stat.is_dir {
                return Err(format!(
                    "Cannot journal a non-file installer target: {path}"
                ));
            }
            snapshots.push(Snapshot {
                exists: true,
                sha256: Some(checked_hash(storage, &path, "sha256")?),
            });
        }
    }
    Ok(snapshots)
}

fn validate_journal(journal: &Journal, version: &str) -> PakResult<()> {
    if journal.schema_version != SCHEMA_VERSION || journal.resource_version != version {
        return Err("Transaction journal has an unsupported schema or resource version".to_owned());
    }
    if journal.before.len() != SLOTS.len() || journal.after.len() != SLOTS.len() {
        return Err(
            "Transaction journal does not contain the fixed installer snapshot set".to_owned(),
        );
    }
    for snapshot in &journal.before {
        match (snapshot.exists, snapshot.sha256.as_deref()) {
            (true, Some(hash)) if valid_hash(hash, 32) => {}
            (false, None) => {}
            _ => {
                return Err(
                    "Transaction journal contains an invalid pre-install snapshot".to_owned(),
                )
            }
        }
    }
    for hash in journal.after.iter().flatten() {
        if !valid_hash(hash, 32) {
            return Err("Transaction journal contains an invalid staged hash".to_owned());
        }
    }
    match journal.operation {
        Operation::Install => {
            if journal.after.iter().any(Option::is_none) {
                return Err("Install journal is missing an expected output hash".to_owned());
            }
            if !journal.before[2].exists
                && journal
                    .before
                    .iter()
                    .enumerate()
                    .any(|(index, item)| index != 2 && item.exists)
            {
                return Err(
                    "Install journal claims unowned files existed before ownership metadata"
                        .to_owned(),
                );
            }
        }
        Operation::Uninstall => {
            if journal.after.iter().any(Option::is_some) || !journal.before[2].exists {
                return Err("Uninstall journal does not describe a verified owned set".to_owned());
            }
        }
    }
    Ok(())
}

fn verify_journal_payload(
    storage: &dyn Storage,
    version: &str,
    journal: &Journal,
) -> PakResult<()> {
    if journal.operation != Operation::Install {
        return Ok(());
    }
    let owner_target = target_path(version, Slot::Owner);
    let owner_stage = format!("{owner_target}.new");
    let expected_hash = journal.after[2]
        .as_deref()
        .expect("validated install owner hash");
    let owner_path = if checked_hash(storage, &owner_stage, "sha256")
        .ok()
        .as_deref()
        == Some(expected_hash)
    {
        owner_stage
    } else if checked_hash(storage, &owner_target, "sha256")
        .ok()
        .as_deref()
        == Some(expected_hash)
    {
        owner_target
    } else {
        return Err("Transaction journal has no matching staged ownership metadata".to_owned());
    };
    let owner: Owner = serde_json::from_str(&storage.read_text(&owner_path)?)
        .map_err(|error| format!("Transaction ownership metadata is malformed: {error}"))?;
    validate_owner(&owner, version)?;
    if owner.pak_sha256.to_ascii_lowercase() != journal.after[0].as_deref().unwrap_or_default()
        || owner.signature_sha256.to_ascii_lowercase()
            != journal.after[1].as_deref().unwrap_or_default()
        || owner.mount_sha256.to_ascii_lowercase()
            != journal.after[3].as_deref().unwrap_or_default()
    {
        return Err(
            "Transaction journal output hashes disagree with ownership metadata".to_owned(),
        );
    }
    Ok(())
}

fn write_initial_journal(storage: &dyn Storage, journal: &Journal) -> PakResult<()> {
    let version = &journal.resource_version;
    let canonical = journal_path(version);
    let staged = format!("{canonical}.new");
    let backup = format!("{canonical}.bak");
    if file_exists(storage, &canonical)?
        || file_exists(storage, &staged)?
        || file_exists(storage, &backup)?
    {
        return Err("An installer transaction journal lifecycle path already exists".to_owned());
    }
    let text = serde_json::to_string(journal)
        .map_err(|error| format!("Could not serialize transaction journal: {error}"))?;
    storage.write_text(&staged, &text)?;
    let staged_hash = checked_hash(storage, &staged, "sha256")?;
    if staged_hash != sha256_text(&text) {
        let _ = storage.remove(&staged);
        return Err("Staged transaction journal failed content verification".to_owned());
    }
    storage
        .move_file(&staged, &canonical)
        .map_err(|error| format!("Could not publish transaction journal: {error}"))
}

fn fully_applied(storage: &dyn Storage, version: &str, journal: &Journal) -> PakResult<bool> {
    for (index, path) in target_paths(version).iter().enumerate() {
        let stat = storage.stat(path)?;
        match (&journal.after[index], stat.exists, stat.is_dir) {
            (None, false, _) => {}
            (Some(expected), true, false) => {
                if checked_hash(storage, path, "sha256")? != *expected {
                    return Ok(false);
                }
            }
            _ => return Ok(false),
        }
    }
    Ok(true)
}

fn apply_journal(storage: &dyn Storage, version: &str, journal: &Journal) -> PakResult<()> {
    backup_current_targets(storage, version, journal)?;
    if journal.operation == Operation::Install {
        let targets = target_paths(version);
        let stages = stage_paths(version);
        for index in 0..SLOTS.len() {
            let expected = journal.after[index]
                .as_deref()
                .expect("install has all output hashes");
            if checked_hash(storage, &stages[index], "sha256")? != expected {
                return Err(format!(
                    "Staged file changed before commit: {}",
                    stages[index]
                ));
            }
            storage.move_file(&stages[index], &targets[index])?;
            if checked_hash(storage, &targets[index], "sha256")? != expected {
                return Err(format!(
                    "Committed file failed verification: {}",
                    targets[index]
                ));
            }
        }
        verify_committed_install(storage, version, journal)?;
    }
    if !fully_applied(storage, version, journal)? {
        return Err("Transaction did not reach its exact expected target state".to_owned());
    }
    Ok(())
}

fn backup_current_targets(
    storage: &dyn Storage,
    version: &str,
    journal: &Journal,
) -> PakResult<()> {
    let targets = target_paths(version);
    let backups = backup_paths(version);
    let stages = stage_paths(version);
    for index in 0..SLOTS.len() {
        let target_stat = storage.stat(&targets[index])?;
        let backup_stat = storage.stat(&backups[index])?;
        if backup_stat.exists {
            let expected = journal.before[index]
                .sha256
                .as_deref()
                .filter(|_| journal.before[index].exists)
                .ok_or_else(|| {
                    format!("Unexpected transaction backup exists: {}", backups[index])
                })?;
            if backup_stat.is_dir || checked_hash(storage, &backups[index], "sha256")? != expected {
                return Err(format!(
                    "Transaction backup failed verification: {}",
                    backups[index]
                ));
            }
            if target_stat.exists {
                let actual = checked_hash(storage, &targets[index], "sha256")?;
                if journal.after[index].as_deref() != Some(actual.as_str()) {
                    return Err(format!(
                        "Unexpected target beside verified backup: {}",
                        targets[index]
                    ));
                }
            }
            continue;
        }
        if journal.before[index].exists {
            let expected = journal.before[index]
                .sha256
                .as_deref()
                .expect("validated snapshot hash");
            if !target_stat.exists || target_stat.is_dir {
                return Err(format!(
                    "Original target and its backup are both missing: {}",
                    targets[index]
                ));
            }
            if checked_hash(storage, &targets[index], "sha256")? != expected {
                return Err(format!(
                    "Original target changed before backup: {}",
                    targets[index]
                ));
            }
            storage.move_file(&targets[index], &backups[index])?;
            if file_exists(storage, &targets[index])?
                || checked_hash(storage, &backups[index], "sha256")? != expected
            {
                return Err(format!(
                    "Could not verify transaction backup: {}",
                    backups[index]
                ));
            }
        } else if target_stat.exists {
            return Err(format!(
                "Unexpected target appeared during transaction: {}",
                targets[index]
            ));
        }
        let stage_stat = storage.stat(&stages[index])?;
        if journal.operation == Operation::Install && !stage_stat.exists {
            return Err(format!(
                "Staged output disappeared before commit: {}",
                stages[index]
            ));
        }
    }
    Ok(())
}

fn verify_committed_install(
    storage: &dyn Storage,
    version: &str,
    journal: &Journal,
) -> PakResult<()> {
    if journal.operation != Operation::Install || !fully_applied(storage, version, journal)? {
        return Err("Installed files do not match the transaction output hashes".to_owned());
    }
    verify_journal_payload(storage, version, journal)?;
    let owner = load_owner(storage, version)?
        .ok_or_else(|| "Installed ownership metadata is missing".to_owned())?;
    verify_owned_content(storage, version, &owner)
}

fn rollback_journal(storage: &dyn Storage, version: &str, journal: &Journal) -> PakResult<()> {
    validate_journal(journal, version)?;
    if journal.before[2].exists {
        verify_previous_owner(storage, version, journal)?;
    }
    let targets = target_paths(version);
    let stages = stage_paths(version);
    let backups = backup_paths(version);
    for index in (0..SLOTS.len()).rev() {
        let target_stat = storage.stat(&targets[index])?;
        let backup_stat = storage.stat(&backups[index])?;
        if backup_stat.exists {
            let before_hash = journal.before[index]
                .sha256
                .as_deref()
                .filter(|_| journal.before[index].exists)
                .ok_or_else(|| {
                    format!(
                        "Unexpected backup cannot be safely restored: {}",
                        backups[index]
                    )
                })?;
            if backup_stat.is_dir
                || checked_hash(storage, &backups[index], "sha256")? != before_hash
            {
                return Err(format!(
                    "Transaction backup is corrupt; preserving it: {}",
                    backups[index]
                ));
            }
            if target_stat.exists {
                if target_stat.is_dir {
                    return Err(format!(
                        "Cannot roll back over a directory: {}",
                        targets[index]
                    ));
                }
                let target_hash = checked_hash(storage, &targets[index], "sha256")?;
                let is_before = target_hash == before_hash;
                let is_after = journal.after[index].as_deref() == Some(target_hash.as_str());
                if !is_before && !is_after {
                    return Err(format!(
                        "Unknown file blocks transaction rollback: {}",
                        targets[index]
                    ));
                }
                storage.remove(&targets[index])?;
            }
            storage.move_file(&backups[index], &targets[index])?;
            if checked_hash(storage, &targets[index], "sha256")? != before_hash {
                return Err(format!(
                    "Restored target failed verification: {}",
                    targets[index]
                ));
            }
        } else if journal.before[index].exists {
            let before_hash = journal.before[index]
                .sha256
                .as_deref()
                .expect("validated snapshot hash");
            if !target_stat.exists
                || target_stat.is_dir
                || checked_hash(storage, &targets[index], "sha256")? != before_hash
            {
                return Err(format!(
                    "Exact original is unavailable for rollback: {}",
                    targets[index]
                ));
            }
        } else if target_stat.exists {
            let target_hash = checked_hash(storage, &targets[index], "sha256")?;
            let expected = journal.after[index].as_deref();
            if target_stat.is_dir || expected != Some(target_hash.as_str()) {
                return Err(format!(
                    "Unknown file blocks rollback at {}",
                    targets[index]
                ));
            }
            storage.remove(&targets[index])?;
        }

        let stage_stat = storage.stat(&stages[index])?;
        if stage_stat.exists {
            let expected = journal.after[index].as_deref().ok_or_else(|| {
                format!("Unexpected staged file blocks rollback: {}", stages[index])
            })?;
            if stage_stat.is_dir || checked_hash(storage, &stages[index], "sha256")? != expected {
                return Err(format!(
                    "Staged file is not transaction-owned: {}",
                    stages[index]
                ));
            }
            storage.remove(&stages[index])?;
        }
    }
    verify_before_snapshot(storage, version, journal)?;
    remove_journal_files(storage, version, journal)?;
    Ok(())
}

fn finalize_journal(storage: &dyn Storage, version: &str, journal: &Journal) -> PakResult<()> {
    validate_journal(journal, version)?;
    if !fully_applied(storage, version, journal)? {
        return Err("Cannot finalize an incomplete installer transaction".to_owned());
    }
    if journal.operation == Operation::Install {
        verify_committed_install(storage, version, journal)?;
    }
    let stages = stage_paths(version);
    let backups = backup_paths(version);
    for index in 0..SLOTS.len() {
        if file_exists(storage, &stages[index])? {
            let expected = journal.after[index].as_deref().ok_or_else(|| {
                format!(
                    "Unexpected stage blocks transaction cleanup: {}",
                    stages[index]
                )
            })?;
            if checked_hash(storage, &stages[index], "sha256")? != expected {
                return Err(format!(
                    "Unrecognized stage blocks transaction cleanup: {}",
                    stages[index]
                ));
            }
            storage.remove(&stages[index])?;
        }
        if file_exists(storage, &backups[index])? {
            let expected = journal.before[index]
                .sha256
                .as_deref()
                .filter(|_| journal.before[index].exists)
                .ok_or_else(|| {
                    format!(
                        "Unrecognized backup blocks transaction cleanup: {}",
                        backups[index]
                    )
                })?;
            if checked_hash(storage, &backups[index], "sha256")? != expected {
                return Err(format!(
                    "Unrecognized backup blocks transaction cleanup: {}",
                    backups[index]
                ));
            }
            storage.remove(&backups[index])?;
        }
    }
    remove_journal_files(storage, version, journal)
}

fn verify_before_snapshot(
    storage: &dyn Storage,
    version: &str,
    journal: &Journal,
) -> PakResult<()> {
    for (index, path) in target_paths(version).iter().enumerate() {
        let stat = storage.stat(path)?;
        let snapshot = &journal.before[index];
        match (snapshot.exists, stat.exists, stat.is_dir) {
            (false, false, _) => {}
            (true, true, false)
                if checked_hash(storage, path, "sha256")?
                    == snapshot.sha256.as_deref().unwrap_or_default() => {}
            _ => {
                return Err(format!(
                    "Rollback did not restore the exact snapshot for {path}"
                ))
            }
        }
    }
    Ok(())
}

fn verify_previous_owner(storage: &dyn Storage, version: &str, journal: &Journal) -> PakResult<()> {
    let path = target_path(version, Slot::Owner);
    let backup = format!("{path}.bak");
    let expected = journal.before[2]
        .sha256
        .as_deref()
        .expect("owner snapshot hash");
    for candidate in [&path, &backup] {
        let stat = storage.stat(candidate)?;
        if !stat.exists || stat.is_dir || checked_hash(storage, candidate, "sha256")? != expected {
            continue;
        }
        if stat.size > OWNER_LIMIT {
            return Err("Previous ownership metadata exceeds its size limit".to_owned());
        }
        let content = storage.read_text(candidate)?;
        let owner: Owner = serde_json::from_str(&content)
            .map_err(|error| format!("Previous ownership metadata is malformed: {error}"))?;
        return validate_owner(&owner, version);
    }
    Err("Previous owner metadata cannot be verified from the transaction snapshot".to_owned())
}

fn remove_journal_files(storage: &dyn Storage, version: &str, journal: &Journal) -> PakResult<()> {
    for path in journal_lifecycle_paths(version) {
        if !file_exists(storage, &path)? {
            continue;
        }
        let stat = storage.stat(&path)?;
        if stat.is_dir || stat.size > JOURNAL_LIMIT {
            return Err(format!(
                "Cannot safely remove transaction journal path {path}"
            ));
        }
        let found: Journal = serde_json::from_str(&storage.read_text(&path)?).map_err(|error| {
            format!("Cannot safely remove malformed transaction journal {path}: {error}")
        })?;
        if &found != journal {
            return Err(format!(
                "A different transaction journal blocks cleanup: {path}"
            ));
        }
        storage.remove(&path)?;
    }
    Ok(())
}

fn read_pending_journal(storage: &dyn Storage, version: &str) -> PakResult<Option<Journal>> {
    let paths = journal_lifecycle_paths(version);
    let mut selected: Option<Journal> = None;
    for path in paths {
        let stat = storage.stat(&path)?;
        if !stat.exists {
            continue;
        }
        if stat.is_dir || stat.size > JOURNAL_LIMIT {
            return Err(format!(
                "Transaction journal path is not a bounded regular file: {path}"
            ));
        }
        let text = storage.read_text(&path)?;
        let journal: Journal = serde_json::from_str(&text)
            .map_err(|error| format!("Transaction journal is malformed at {path}: {error}"))?;
        validate_journal(&journal, version)?;
        if let Some(existing) = &selected {
            if existing != &journal {
                return Err(format!(
                    "Conflicting transaction journal lifecycle copies exist for {version}"
                ));
            }
        } else {
            selected = Some(journal);
        }
    }
    if let Some(journal) = &selected {
        verify_journal_payload(storage, version, journal)?;
    }
    Ok(selected)
}

fn recover_after_failure(
    storage: &dyn Storage,
    layout: &ResourceLayout,
    progress: &ProgressCallback,
    operation_error: String,
) -> String {
    match recover(storage, layout, progress) {
        Ok(status) if !status.recovery_pending => operation_error,
        Ok(status) => format!(
            "{operation_error}; recovery remains pending: {}",
            status.message.unwrap_or_default()
        ),
        Err(recovery_error) => {
            format!("{operation_error}; exact rollback/recovery failed: {recovery_error}")
        }
    }
}

fn clean_stages(
    storage: &dyn Storage,
    stages: &[String],
    hashes: &[Option<String>],
) -> PakResult<()> {
    let mut errors = Vec::new();
    for (index, path) in stages.iter().enumerate() {
        match storage.stat(path) {
            Ok(stat) if !stat.exists => {}
            Ok(stat) if stat.is_dir => {
                errors.push(format!("Refusing to remove staged directory {path}"))
            }
            Ok(_) => match hashes.get(index).and_then(Option::as_deref) {
                Some(expected) => match checked_hash(storage, path, "sha256") {
                    Ok(actual) if actual == expected => {
                        if let Err(error) = storage.remove(path) {
                            errors.push(error);
                        }
                    }
                    Ok(_) => errors.push(format!("Refusing to remove unverified stage {path}")),
                    Err(error) => errors.push(error),
                },
                None => errors.push(format!("Refusing to remove unverified stage {path}")),
            },
            Err(error) => errors.push(error),
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}

fn append_cleanup_error(error: String, cleanup: PakResult<()>) -> String {
    match cleanup {
        Ok(()) => error,
        Err(cleanup_error) => format!("{error}; staged-file cleanup failed: {cleanup_error}"),
    }
}

fn checked_hash(storage: &dyn Storage, path: &str, algorithm: &str) -> PakResult<String> {
    let stat = storage.stat(path)?;
    if !stat.exists || stat.is_dir {
        return Err(format!(
            "Cannot hash a missing or non-file resource path: {path}"
        ));
    }
    let hash = storage.hash(path, algorithm)?.to_ascii_lowercase();
    let length = if algorithm == "sha1" { 20 } else { 32 };
    if !valid_hash(&hash, length) {
        return Err(format!(
            "Storage returned an invalid {algorithm} digest for {path}"
        ));
    }
    Ok(hash)
}

fn hash_local_file(path: &Path) -> PakResult<String> {
    let mut file =
        File::open(path).map_err(|error| format!("Could not open downloaded PAK: {error}"))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 128 * 1024];
    loop {
        let count = file
            .read(&mut buffer)
            .map_err(|error| format!("Could not hash downloaded PAK: {error}"))?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(hex::encode(hasher.finalize()))
}

fn valid_hash(value: &str, bytes: usize) -> bool {
    value.len() == bytes * 2 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn sha256_text(value: &str) -> String {
    hex::encode(Sha256::digest(value.as_bytes()))
}

fn mount_content(pak_sha1: &str, signature_sha1: &str) -> String {
    let pak_base = PATCH_FILENAME
        .strip_suffix(".pak")
        .expect("canonical PAK extension");
    format!(
        "::Mount::\n{PATCH_DIRECTORY}/{pak_base},{PRIORITY},{pak_sha1},{signature_sha1},,\n::Del::\n"
    )
}

fn parse_version(value: &str) -> Option<(u64, u64, u64)> {
    if value
        .bytes()
        .any(|byte| !(byte.is_ascii_digit() || byte == b'.'))
    {
        return None;
    }
    let mut parts = value.split('.');
    let parse_component = |part: &str| {
        if part.len() > 1 && part.starts_with('0') {
            None
        } else {
            part.parse::<u64>().ok()
        }
    };
    let major = parse_component(parts.next()?)?;
    let minor = parse_component(parts.next()?)?;
    let patch = parse_component(parts.next()?)?;
    if parts.next().is_some() {
        return None;
    }
    Some((major, minor, patch))
}

fn validate_version(value: &str) -> PakResult<()> {
    if parse_version(value).is_none() {
        return Err(format!("Unsafe or malformed resource version: {value}"));
    }
    Ok(())
}

fn notify(progress: &ProgressCallback, percent: u8, label: &str) {
    progress(Progress {
        percent,
        label: label.to_owned(),
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pak::local_storage::LocalStorage;
    use crate::pak::storage::Storage;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::{Arc, Mutex};

    fn setup_fixture() -> (tempfile::TempDir, LocalStorage, PathBuf, ResourceLayout) {
        let root = tempfile::tempdir().expect("resource root");
        let version_root = root.path().join("3.7.0");
        fs::create_dir_all(version_root.join("Mount")).expect("mount directory");
        fs::create_dir_all(version_root.join("Resource")).expect("resource directory");
        fs::create_dir_all(version_root.join("Lang_en")).expect("language directory");
        fs::write(version_root.join("ResManifest"), b"manifest").expect("manifest");
        fs::write(
            version_root.join("Mount/MountResource.txt"),
            b"::Mount::\n::Del::\n",
        )
        .expect("mount manifest");
        fs::write(
            version_root.join("Lang_en/pakchunk0-Lang_en_1000_P.pak"),
            b"official pak",
        )
        .expect("official pak");
        fs::write(
            version_root.join("Lang_en/pakchunk0-Lang_en_1000_P.sig"),
            b"official signature bytes",
        )
        .expect("official signature");
        let storage = LocalStorage::new(root.path()).expect("local storage");
        let layout = discover(&storage).expect("resource layout");
        let pak = root.path().join("download.pak");
        fs::write(&pak, b"verified opaque patch bytes").expect("download fixture");
        (root, storage, pak, layout)
    }

    fn release(pak: &Path, tag: &str) -> Release {
        let bytes = fs::read(pak).expect("read fixture PAK");
        Release {
            tag: tag.to_owned(),
            game_version: "3.7.1".to_owned(),
            asset_name: "pakchunk0-ID-WindowsNoEditor_1000_P.pak".to_owned(),
            download_url: "https://example.invalid/patch.pak".to_owned(),
            html_url: "https://example.invalid/release".to_owned(),
            size: bytes.len() as u64,
            sha256: hex::encode(Sha256::digest(&bytes)),
        }
    }

    fn no_progress(_: Progress) {}

    fn assert_mount_resolves(
        root: &Path,
        storage: &LocalStorage,
        layout: &ResourceLayout,
        pak: &Path,
    ) {
        let descriptor = storage
            .read_text(&target_path(&layout.version, Slot::Mount))
            .expect("installed descriptor");
        let fields: Vec<_> = descriptor
            .lines()
            .nth(1)
            .expect("mount entry")
            .split(',')
            .collect();
        assert_eq!(fields.len(), 6);
        assert_eq!(fields[1], "99");
        assert!(fields[4].is_empty() && fields[5].is_empty());
        // Resource Mount resolves a base name into separate PAK and signature files.
        let resolved_pak = format!("{}/{}.pak", layout.version, fields[0]);
        let resolved_signature = format!("{}/{}.sig", layout.version, fields[0]);
        assert_eq!(
            fs::read(root.join(&resolved_pak)).expect("game resolves mounted PAK"),
            fs::read(pak).expect("downloaded payload")
        );
        assert_eq!(
            storage
                .hash(&resolved_pak, "sha1")
                .expect("mounted PAK hash")
                .to_ascii_uppercase(),
            fields[2]
        );
        assert_eq!(
            storage
                .hash(&resolved_signature, "sha1")
                .expect("mounted signature hash")
                .to_ascii_uppercase(),
            fields[3]
        );
        assert_eq!(
            fs::read(root.join(&resolved_signature)).expect("mounted signature"),
            fs::read(root.join(&layout.signature)).expect("official signature")
        );
    }

    fn persist_previous_mount(root: &Path, layout: &ResourceLayout) {
        let owner_path = root.join(target_path(&layout.version, Slot::Owner));
        let mut owner: Owner =
            serde_json::from_slice(&fs::read(&owner_path).expect("ownership metadata"))
                .expect("old owner");
        let previous_descriptor = format!(
            "::Mount::\nwuwaid/WuwaID_99_P.pak,99,{},{},,\n::Del::\n",
            owner.pak_sha1, owner.signature_sha1
        );
        owner.mount_sha256 = sha256_text(&previous_descriptor);
        fs::write(
            root.join(target_path(&layout.version, Slot::Mount)),
            previous_descriptor,
        )
        .expect("previous descriptor");
        fs::write(
            owner_path,
            serde_json::to_vec(&owner).expect("previous ownership metadata"),
        )
        .expect("persist previous owner");
    }

    #[test]
    fn repairs_the_previous_owned_mount_format() {
        let (root, storage, pak, layout) = setup_fixture();
        let release = release(&pak, "v3.7.1-id.1");
        install(&storage, &layout, &release, &pak, None, &no_progress).expect("fixture install");
        persist_previous_mount(root.path(), &layout);
        let previous = inspect(&storage, &layout).expect("previous installation");
        assert!(previous.installed && !previous.healthy && !previous.recovery_pending);
        let original_mount =
            fs::read(root.path().join("3.7.0/Mount/MountResource.txt")).expect("game mount");
        let repaired = repair(&storage, &layout, &release, &pak, None, &no_progress)
            .expect("format migration");
        assert!(repaired.installed && repaired.healthy && !repaired.recovery_pending);
        assert_mount_resolves(root.path(), &storage, &layout, &pak);
        assert_eq!(
            fs::read(root.path().join("3.7.0/Mount/MountResource.txt"))
                .expect("preserved game mount"),
            original_mount
        );
    }

    #[test]
    fn uninstalls_the_previous_owned_mount_without_repair() {
        let (root, storage, pak, layout) = setup_fixture();
        let release = release(&pak, "v3.7.1-id.1");
        install(&storage, &layout, &release, &pak, None, &no_progress).expect("fixture install");
        persist_previous_mount(root.path(), &layout);
        let removed = uninstall(&storage, &layout, &no_progress).expect("remove previous format");
        assert!(!removed.installed && !removed.recovery_pending);
        for path in target_paths(&layout.version) {
            assert!(!root.path().join(path).exists());
        }
        assert_eq!(
            fs::read(root.path().join(&layout.signature)).expect("preserved official signature"),
            b"official signature bytes"
        );
    }

    #[test]
    fn failed_format_migration_restores_the_previous_install_exactly() {
        let (root, storage, pak, layout) = setup_fixture();
        let storage = Arc::new(storage);
        let release = release(&pak, "v3.7.1-id.1");
        install(
            storage.as_ref(),
            &layout,
            &release,
            &pak,
            None,
            &no_progress,
        )
        .expect("fixture install");
        persist_previous_mount(root.path(), &layout);
        let paths = target_paths(&layout.version);
        let previous: Vec<_> = paths
            .iter()
            .map(|path| fs::read(root.path().join(path)).expect("snapshot"))
            .collect();
        let failing = FailMoveStorage {
            inner: Arc::clone(&storage),
            fail_target: target_path(&layout.version, Slot::Mount),
            failures_remaining: Arc::new(Mutex::new(1)),
        };
        assert!(repair(&failing, &layout, &release, &pak, None, &no_progress).is_err());
        for (path, bytes) in paths.iter().zip(previous) {
            assert_eq!(
                fs::read(root.path().join(path)).expect("restored snapshot"),
                bytes
            );
        }
        let restored = inspect(storage.as_ref(), &layout).expect("restored previous format");
        assert!(restored.installed && !restored.healthy && !restored.recovery_pending);
        uninstall(storage.as_ref(), &layout, &no_progress)
            .expect("previous format still removable");
    }

    #[test]
    fn refuses_an_unrecognized_mount_even_with_a_matching_recorded_hash() {
        let (root, storage, pak, layout) = setup_fixture();
        let release = release(&pak, "v3.7.1-id.1");
        install(&storage, &layout, &release, &pak, None, &no_progress).expect("fixture install");
        let owner_path = root.path().join(target_path(&layout.version, Slot::Owner));
        let mount_path = root.path().join(target_path(&layout.version, Slot::Mount));
        let mut owner: Owner =
            serde_json::from_slice(&fs::read(&owner_path).expect("owner")).expect("parsed owner");
        let foreign = format!(
            "::Mount::\nforeign/mod,99,{},{},,\n::Del::\n",
            owner.pak_sha1, owner.signature_sha1
        );
        owner.mount_sha256 = sha256_text(&foreign);
        fs::write(&mount_path, &foreign).expect("unrecognized mount");
        fs::write(
            &owner_path,
            serde_json::to_vec(&owner).expect("changed owner"),
        )
        .expect("changed metadata");
        let before_owner = fs::read(&owner_path).expect("owner snapshot");
        assert!(repair(&storage, &layout, &release, &pak, None, &no_progress).is_err());
        assert!(uninstall(&storage, &layout, &no_progress).is_err());
        assert_eq!(
            fs::read(&owner_path).expect("preserved metadata"),
            before_owner
        );
        assert_eq!(
            fs::read_to_string(&mount_path).expect("preserved unknown mount"),
            foreign
        );
    }

    #[test]
    fn discover_uses_numeric_versions_and_recursive_english_signature_pairs() {
        let root = tempfile::tempdir().expect("resource root");
        for version in ["3.9.0", "3.10.0"] {
            let version_root = root.path().join(version);
            fs::create_dir_all(version_root.join("Mount")).expect("mount directory");
            fs::create_dir_all(version_root.join("Resource")).expect("resource directory");
            let signature_directory = version_root.join("Lang_en/nested");
            fs::create_dir_all(&signature_directory).expect("nested language directory");
            fs::write(version_root.join("ResManifest"), b"manifest").expect("manifest");
            fs::write(version_root.join("Mount/MountResource.txt"), b"mount")
                .expect("mount manifest");
            fs::write(
                signature_directory.join("pakchunk-Lang_en_1000_P.PAK"),
                b"official pak",
            )
            .expect("official pak");
            fs::write(
                signature_directory.join("pakchunk-Lang_en_1000_P.SIG"),
                b"official signature",
            )
            .expect("official signature");
        }
        fs::create_dir_all(root.path().join("3.11.0")).expect("new incomplete resource version");
        fs::write(root.path().join("3.11.0/ResManifest"), b"pre-download")
            .expect("incomplete manifest");

        let storage = LocalStorage::new(root.path()).expect("local storage");
        let layout = discover(&storage).expect("newest materialized layout");
        assert_eq!(layout.version, "3.10.0");
        assert_eq!(
            layout.signature,
            "3.10.0/Lang_en/nested/pakchunk-Lang_en_1000_P.SIG"
        );
    }

    #[test]
    fn installs_repairs_and_uninstalls_only_the_owned_patch_set() {
        let (root, storage, pak, layout) = setup_fixture();
        let release = release(&pak, "v3.7.1-id.1");
        let installed =
            install(&storage, &layout, &release, &pak, None, &no_progress).expect("install");
        assert!(installed.installed && installed.healthy && !installed.recovery_pending);
        let mount_path = format!("3.7.0/Mount/{MOUNT_FILENAME}");
        assert_mount_resolves(root.path(), &storage, &layout, &pak);

        let pak_path = format!("3.7.0/{PATCH_DIRECTORY}/{PATCH_FILENAME}");
        storage
            .remove(&pak_path)
            .expect("simulate damaged owned PAK");
        assert!(
            !inspect(&storage, &layout)
                .expect("inspect damaged install")
                .healthy
        );
        let repaired =
            repair(&storage, &layout, &release, &pak, None, &no_progress).expect("repair");
        assert!(repaired.healthy);

        let removed = uninstall(&storage, &layout, &no_progress).expect("uninstall");
        assert!(!removed.installed && !removed.healthy);
        assert!(!storage.stat(&pak_path).expect("removed PAK stat").exists);
        assert!(
            !storage
                .stat(&mount_path)
                .expect("removed mount stat")
                .exists
        );
        assert!(
            storage
                .stat("3.7.0/Mount/MountResource.txt")
                .expect("game mount stat")
                .exists
        );
    }

    #[test]
    fn refuses_unowned_target_collision_and_bad_download_without_mutation() {
        let (_root, storage, pak, layout) = setup_fixture();
        let unowned = format!("3.7.0/{PATCH_DIRECTORY}/{PATCH_FILENAME}");
        storage
            .mkdir(&format!("3.7.0/{PATCH_DIRECTORY}"))
            .expect("patch directory");
        storage
            .write_text(&unowned, "another mod")
            .expect("unowned collision");
        let valid_release = release(&pak, "v3.7.1-id.1");
        assert!(install(&storage, &layout, &valid_release, &pak, None, &no_progress).is_err());
        assert_eq!(
            storage.read_text(&unowned).expect("preserved collision"),
            "another mod"
        );

        storage.remove(&unowned).expect("clear fixture collision");
        let mut bad_release = valid_release;
        bad_release.sha256 = "00".repeat(32);
        assert!(install(&storage, &layout, &bad_release, &pak, None, &no_progress).is_err());
        assert!(
            !storage
                .stat(&unowned)
                .expect("target remains absent")
                .exists
        );
    }

    #[test]
    fn refuses_a_competing_custom_mount_at_priority_ninety_nine() {
        let (root, storage, pak, layout) = setup_fixture();
        fs::write(
            root.path().join("3.7.0/Mount/other-mod.txt"),
            "::Mount::\nother/patch.pak,99,ABC,DEF,,\n::Del::\n",
        )
        .expect("competing mount");
        let release = release(&pak, "v3.7.1-id.1");
        let status = inspect(&storage, &layout).expect("conflict inspection");
        assert!(!status.healthy);
        assert!(status
            .message
            .as_deref()
            .unwrap_or_default()
            .contains("other-mod.txt"));
        let error = install(&storage, &layout, &release, &pak, None, &no_progress)
            .expect_err("mount conflict");
        assert!(error.contains("other-mod.txt"));
        assert!(
            !storage
                .stat(&format!("3.7.0/{PATCH_DIRECTORY}/{PATCH_FILENAME}"))
                .expect("no install")
                .exists
        );
    }

    #[test]
    fn ignores_the_games_resource_and_language_mount_files_for_conflicts() {
        let (root, storage, pak, layout) = setup_fixture();
        fs::write(
            root.path().join("3.7.0/Mount/MountResource.txt"),
            "::Mount::\ngame/resource.pak,99,A,B,,\n::Del::\n",
        )
        .expect("game resource mount");
        fs::write(
            root.path().join("3.7.0/Mount/MountLang_en.txt"),
            "::Mount::\ngame/language.pak,99,A,B,,\n::Del::\n",
        )
        .expect("game language mount");
        let release = release(&pak, "v3.7.1-id.1");
        let status = install(&storage, &layout, &release, &pak, None, &no_progress)
            .expect("install beside game mounts");
        assert!(status.healthy);
    }

    #[derive(Clone)]
    struct FailMoveStorage {
        inner: Arc<LocalStorage>,
        fail_target: String,
        failures_remaining: Arc<Mutex<usize>>,
    }

    impl Storage for FailMoveStorage {
        fn list(&self, path: &str) -> PakResult<Vec<super::super::model::Entry>> {
            self.inner.list(path)
        }
        fn stat(&self, path: &str) -> PakResult<super::super::model::FileStat> {
            self.inner.stat(path)
        }
        fn read_text(&self, path: &str) -> PakResult<String> {
            self.inner.read_text(path)
        }
        fn hash(&self, path: &str, algorithm: &str) -> PakResult<String> {
            self.inner.hash(path, algorithm)
        }
        fn mkdir(&self, path: &str) -> PakResult<()> {
            self.inner.mkdir(path)
        }
        fn copy(&self, source: &str, target: &str) -> PakResult<()> {
            self.inner.copy(source, target)
        }
        fn move_file(&self, source: &str, target: &str) -> PakResult<()> {
            let mut remaining = self.failures_remaining.lock().expect("failure counter");
            if target == self.fail_target && *remaining > 0 {
                *remaining -= 1;
                return Err("injected mount commit failure".to_owned());
            }
            self.inner.move_file(source, target)
        }
        fn remove(&self, path: &str) -> PakResult<()> {
            self.inner.remove(path)
        }
        fn write_text(&self, path: &str, content: &str) -> PakResult<()> {
            self.inner.write_text(path, content)
        }
        fn write_from_file(&self, path: &str, source: &Path) -> PakResult<()> {
            self.inner.write_from_file(path, source)
        }
        fn force_stop(&self) -> PakResult<()> {
            self.inner.force_stop()
        }
    }

    #[test]
    fn interrupted_commit_survives_resource_update_and_recovers_exact_originals() {
        let (root, storage, pak, layout) = setup_fixture();
        let storage = Arc::new(storage);
        let first_release = release(&pak, "v3.7.1-id.1");
        install(
            storage.as_ref(),
            &layout,
            &first_release,
            &pak,
            None,
            &no_progress,
        )
        .expect("initial install");
        let snapshot_paths = target_paths(&layout.version);
        let before_hashes: Vec<_> = snapshot_paths
            .iter()
            .map(|path| storage.hash(path, "sha256").expect("pre-transaction hash"))
            .collect();

        let updated_pak = pak.with_file_name("updated.pak");
        fs::write(&updated_pak, b"second opaque patch payload").expect("updated PAK");
        let updated_release = release(&updated_pak, "v3.7.1-id.2");
        let mount = format!("3.7.0/Mount/{MOUNT_FILENAME}");
        let failing = FailMoveStorage {
            inner: Arc::clone(&storage),
            fail_target: mount,
            failures_remaining: Arc::new(Mutex::new(2)),
        };
        assert!(install(
            &failing,
            &layout,
            &updated_release,
            &updated_pak,
            None,
            &no_progress
        )
        .is_err());
        assert!(
            storage
                .stat(&format!("3.7.0/{PATCH_DIRECTORY}/.transaction.json"))
                .expect("journal")
                .exists
        );

        let updated_root = root.path().join("3.8.0");
        fs::create_dir_all(updated_root.join("Mount")).expect("updated mount");
        fs::create_dir_all(updated_root.join("Lang_en")).expect("updated language");
        fs::write(updated_root.join("ResManifest"), b"updated manifest").expect("updated manifest");
        fs::write(
            updated_root.join("Mount/MountResource.txt"),
            b"::Mount::\n::Del::\n",
        )
        .expect("updated game mount");
        fs::write(
            updated_root.join("Lang_en/base.pak"),
            b"updated official PAK",
        )
        .expect("updated PAK");
        fs::write(
            updated_root.join("Lang_en/base.sig"),
            b"updated official signature",
        )
        .expect("updated signature");
        let newest = discover(storage.as_ref()).expect("updated resource layout");
        assert_eq!(newest.version, "3.8.0");
        let pending = inspect(storage.as_ref(), &newest).expect("pending older transaction");
        assert!(pending.recovery_pending && !pending.installed);
        let recovered = recover(storage.as_ref(), &newest, &no_progress)
            .expect("exact recovery without release metadata");
        assert!(!recovered.recovery_pending && !recovered.installed);
        let restored = inspect(storage.as_ref(), &layout).expect("restored older patch");
        assert!(restored.healthy);
        assert_eq!(restored.version, first_release.tag);
        for (path, expected_hash) in snapshot_paths.iter().zip(before_hashes) {
            assert_eq!(
                storage.hash(path, "sha256").expect("restored file hash"),
                expected_hash
            );
        }
        assert!(
            !storage
                .stat(&format!("3.7.0/{PATCH_DIRECTORY}/.transaction.json"))
                .expect("journal cleared")
                .exists
        );
    }

    #[test]
    fn custom_uid_mount_repair_and_reset_preserve_original_release_and_game_files() {
        let (root, storage, pak, layout) = setup_fixture();
        uid::tests::write_valid_fixture(&pak);
        let release = release(&pak, "v3.7.1-id.1");
        let original = fs::read(&pak).unwrap();
        let official = fs::read(root.path().join(&layout.signature)).unwrap();
        install(&storage, &layout, &release, &pak, None, &no_progress).unwrap();
        let applied = install(
            &storage,
            &layout,
            &release,
            &pak,
            Some("Rover Indonesia"),
            &no_progress,
        )
        .unwrap();
        assert!(applied.healthy);
        assert_eq!(applied.custom_uid.as_deref(), Some("Rover Indonesia"));
        let owner = load_owner(&storage, &layout.version).unwrap().unwrap();
        assert_eq!(
            owner.source_sha256.as_deref(),
            Some(release.sha256.as_str())
        );
        assert_ne!(owner.pak_sha256, release.sha256);
        let target = target_path(&layout.version, Slot::Pak);
        assert_eq!(owner.pak_sha256, storage.hash(&target, "sha256").unwrap());
        let mount = storage
            .read_text(&target_path(&layout.version, Slot::Mount))
            .unwrap();
        assert_eq!(
            mount,
            mount_content(
                &storage.hash(&target, "sha1").unwrap().to_ascii_uppercase(),
                &storage
                    .hash(&layout.signature, "sha1")
                    .unwrap()
                    .to_ascii_uppercase(),
            )
        );
        storage.remove(&target).unwrap();
        let repaired = repair(
            &storage,
            &layout,
            &release,
            &pak,
            applied.custom_uid.as_deref(),
            &no_progress,
        )
        .unwrap();
        assert!(repaired.healthy);
        assert_eq!(repaired.custom_uid.as_deref(), Some("Rover Indonesia"));
        let reset = install(&storage, &layout, &release, &pak, None, &no_progress).unwrap();
        assert!(reset.healthy);
        assert_eq!(reset.custom_uid, None);
        assert_eq!(storage.hash(&target, "sha256").unwrap(), release.sha256);
        assert_eq!(fs::read(&pak).unwrap(), original);
        assert_eq!(
            fs::read(root.path().join(&layout.signature)).unwrap(),
            official
        );
    }

    #[test]
    fn failed_uid_change_rolls_back_the_previous_uid_and_all_owned_artifacts() {
        let (root, storage, pak, layout) = setup_fixture();
        uid::tests::write_valid_fixture(&pak);
        let storage = Arc::new(storage);
        let release = release(&pak, "v3.7.1-id.1");
        install(
            storage.as_ref(),
            &layout,
            &release,
            &pak,
            Some("Rover Indonesia"),
            &no_progress,
        )
        .unwrap();
        let paths = target_paths(&layout.version);
        let previous: Vec<_> = paths
            .iter()
            .map(|path| fs::read(root.path().join(path)).unwrap())
            .collect();
        let failing = FailMoveStorage {
            inner: Arc::clone(&storage),
            fail_target: target_path(&layout.version, Slot::Mount),
            failures_remaining: Arc::new(Mutex::new(1)),
        };
        assert!(install(
            &failing,
            &layout,
            &release,
            &pak,
            Some("Rover Nusantara"),
            &no_progress
        )
        .is_err());
        let restored = inspect(storage.as_ref(), &layout).unwrap();
        assert!(restored.healthy && !restored.recovery_pending);
        assert_eq!(restored.custom_uid.as_deref(), Some("Rover Indonesia"));
        for (path, bytes) in paths.iter().zip(previous) {
            assert_eq!(fs::read(root.path().join(path)).unwrap(), bytes);
        }
    }
}
