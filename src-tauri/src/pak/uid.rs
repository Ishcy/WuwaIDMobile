use std::fs::{self, File};
use std::io::{self, BufReader, BufWriter, Write};
use std::path::Path;

use rusqlite::Connection;
use sha2::{Digest, Sha256};
use tempfile::NamedTempFile;

use super::model::PakResult;

const DATABASE: &str = "Client/Content/Aki/ConfigDB/en/lang_multi_text.db";
const TARGETS: [&str; 3] = [
    "Text_FriendMyUid_Text",
    "Text_UserId_Text",
    "PrefabTextItem_1341587207_Text",
];
const MAX_UTF16_LENGTH: usize = 32;

pub struct DerivedPak {
    file: NamedTempFile,
    pub size: u64,
}

impl DerivedPak {
    pub fn path(&self) -> &Path {
        self.file.path()
    }
}

pub fn validate_text(value: &str) -> PakResult<&str> {
    if value
        .chars()
        .any(|c| c.is_control() || matches!(c, '\u{2028}' | '\u{2029}'))
    {
        return Err("Teks UID tidak boleh memuat karakter kontrol atau pemisah baris.".into());
    }
    let value = value.trim();
    if value.is_empty() {
        return Err("Masukkan teks UID kustom.".into());
    }
    if value.encode_utf16().count() > MAX_UTF16_LENGTH {
        return Err(format!("Teks UID maksimal {MAX_UTF16_LENGTH} karakter."));
    }
    Ok(value)
}

// The caller verifies the release's original size and SHA-256 before deriving.
// Only a private temporary database is edited; all other PAK entries are preserved.
pub fn prepare(source: &Path, text: &str) -> PakResult<DerivedPak> {
    let _phase = crate::trace::Span::new("uid.prepare");
    let text = validate_text(text)?;
    let parent = source
        .parent()
        .ok_or("Direktori cache PAK tidak tersedia.")?;
    let work = tempfile::Builder::new()
        .prefix("uid-work-")
        .tempdir_in(parent)
        .map_err(|e| format!("Tidak dapat menyiapkan database UID: {e}"))?;
    let mut source_reader = BufReader::new(File::open(source).map_err(|e| e.to_string())?);
    let source_pak = repak::PakBuilder::new()
        .reader(&mut source_reader)
        .map_err(|e| format!("Format PAK UID tidak dapat dibaca: {e}"))?;
    if source_pak.version() != repak::Version::V12 {
        return Err("Format PAK UID tidak sesuai dengan PAK v12 launcher.".into());
    }
    let entries = source_pak.files();
    if !entries.iter().any(|entry| entry == DATABASE) {
        return Err("Database UID tidak ditemukan dalam PAK terjemahan.".into());
    }
    let db_path = work.path().join("uid.db");
    {
        let mut db_file = File::create(&db_path).map_err(|e| e.to_string())?;
        source_pak
            .read_file(DATABASE, &mut source_reader, &mut db_file)
            .map_err(|e| format!("Database UID tidak dapat diekstrak: {e}"))?;
    }
    patch_database(&db_path, text)?;

    let mut output = tempfile::Builder::new()
        .prefix("uid-pak-")
        .suffix(".pak")
        .tempfile_in(parent)
        .map_err(|e| e.to_string())?;
    let mut expected_hashes = Vec::with_capacity(entries.len());
    {
        let mut output_writer = BufWriter::new(output.as_file_mut());
        let mut writer = repak::PakBuilder::new().writer(
            &mut output_writer,
            repak::Version::V12,
            source_pak.mount_point().to_owned(),
            None,
        );
        for entry in &entries {
            let data = if entry == DATABASE {
                fs::read(&db_path).map_err(|e| e.to_string())?
            } else {
                source_pak
                    .get(entry, &mut source_reader)
                    .map_err(|e| e.to_string())?
            };
            expected_hashes.push(<[u8; 32]>::from(Sha256::digest(&data)));
            writer
                .write_file(entry, false, &data)
                .map_err(|e| e.to_string())?;
        }
        writer.write_index().map_err(|e| e.to_string())?;
        output_writer.flush().map_err(|e| e.to_string())?;
    }
    output.as_file().sync_all().map_err(|e| e.to_string())?;
    let mut verify_reader = BufReader::new(File::open(output.path()).map_err(|e| e.to_string())?);
    let verified = repak::PakBuilder::new()
        .reader(&mut verify_reader)
        .map_err(|e| e.to_string())?;
    if verified.version() != repak::Version::V12
        || verified.mount_point() != source_pak.mount_point()
        || verified.files() != entries
    {
        return Err("Struktur PAK hasil UID tidak sesuai dengan sumber.".into());
    }
    for (entry, expected) in entries.iter().zip(expected_hashes) {
        let mut hash = HashSink(Sha256::new());
        verified
            .read_file(entry, &mut verify_reader, &mut hash)
            .map_err(|e| e.to_string())?;
        if <[u8; 32]>::from(hash.0.finalize()) != expected {
            return Err(format!("Isi PAK hasil UID gagal diverifikasi: {entry}"));
        }
    }
    let size = output
        .as_file()
        .metadata()
        .map_err(|e| e.to_string())?
        .len();
    Ok(DerivedPak { file: output, size })
}

fn patch_database(path: &Path, text: &str) -> PakResult<()> {
    let mut db = Connection::open(path).map_err(|e| e.to_string())?;
    let tx = db.transaction().map_err(|e| e.to_string())?;
    for id in TARGETS {
        let changed = tx
            .execute(
                "UPDATE MultiText SET Content = ?1 WHERE Id = ?2",
                (text, id),
            )
            .map_err(|e| format!("Database UID tidak kompatibel: {e}"))?;
        if changed != 1 {
            return Err(format!(
                "Target UID {id} harus ditemukan tepat sekali; ditemukan {changed}."
            ));
        }
        let actual: String = tx
            .query_row("SELECT Content FROM MultiText WHERE Id = ?1", [id], |row| {
                row.get(0)
            })
            .map_err(|e| e.to_string())?;
        if actual != text {
            return Err(format!("Teks UID gagal diverifikasi: {id}"));
        }
    }
    tx.commit().map_err(|e| e.to_string())?;
    let integrity: String = db
        .query_row("PRAGMA integrity_check", [], |row| row.get(0))
        .map_err(|e| e.to_string())?;
    if integrity != "ok" {
        return Err(format!("Integritas database UID gagal: {integrity}"));
    }
    Ok(())
}

struct HashSink(Sha256);
impl Write for HashSink {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.update(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;

    pub(crate) fn write_fixture(path: &Path, targets: &[&str]) {
        let db_file = NamedTempFile::new_in(path.parent().unwrap()).unwrap();
        {
            let db = Connection::open(db_file.path()).unwrap();
            db.execute_batch("CREATE TABLE MultiText (Id TEXT, Content TEXT);")
                .unwrap();
            for id in targets {
                db.execute("INSERT INTO MultiText VALUES (?1, ?2)", (*id, "UID: {0}"))
                    .unwrap();
            }
            db.execute(
                "INSERT INTO MultiText VALUES (?1, ?2)",
                ("Quest_Text", "Terjemahan misi"),
            )
            .unwrap();
        }
        let mut file = BufWriter::new(File::create(path).unwrap());
        let mut pak = repak::PakBuilder::new().writer(
            &mut file,
            repak::Version::V12,
            "../../../".into(),
            None,
        );
        pak.write_file(DATABASE, false, &fs::read(db_file.path()).unwrap())
            .unwrap();
        pak.write_file(
            "Client/Content/other.db",
            false,
            b"preserved translation asset",
        )
        .unwrap();
        pak.write_index().unwrap();
        file.flush().unwrap();
    }

    pub(crate) fn write_valid_fixture(path: &Path) {
        write_fixture(path, &TARGETS);
    }

    #[test]
    fn uid_limits_match_html_utf16_and_reject_control_characters() {
        assert_eq!(
            validate_text("  Rover Indonesia  ").unwrap(),
            "Rover Indonesia"
        );
        assert_eq!(validate_text(&"🦋".repeat(16)).unwrap(), "🦋".repeat(16));
        assert!(validate_text(&"🦋".repeat(17)).is_err());
        for value in [
            "",
            "   ",
            "Rover\nID",
            "\tRover",
            "Rover\u{2028}ID",
            "Rover\u{007f}ID",
        ] {
            assert!(validate_text(value).is_err());
        }
    }

    #[test]
    fn derived_pak_replaces_all_uid_text_without_changing_other_translations_or_source() {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("source.pak");
        write_valid_fixture(&source);
        let original = fs::read(&source).unwrap();
        let text = "Rover 'Nusantara' <ID>";
        let derived = prepare(&source, text).unwrap();
        let mut file = BufReader::new(File::open(derived.path()).unwrap());
        let pak = repak::PakBuilder::new().reader(&mut file).unwrap();
        let db_path = root.path().join("result.db");
        fs::write(&db_path, pak.get(DATABASE, &mut file).unwrap()).unwrap();
        let db = Connection::open(db_path).unwrap();
        for id in TARGETS {
            let actual: String = db
                .query_row("SELECT Content FROM MultiText WHERE Id = ?1", [id], |row| {
                    row.get(0)
                })
                .unwrap();
            assert_eq!(actual, text);
        }
        let quest: String = db
            .query_row(
                "SELECT Content FROM MultiText WHERE Id = 'Quest_Text'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(quest, "Terjemahan misi");
        assert_eq!(
            pak.get("Client/Content/other.db", &mut file).unwrap(),
            b"preserved translation asset"
        );
        assert_eq!(fs::read(&source).unwrap(), original);
    }

    #[test]
    fn missing_or_duplicate_uid_targets_leave_the_original_pak_untouched() {
        for targets in [
            vec![TARGETS[0], TARGETS[1]],
            vec![TARGETS[0], TARGETS[1], TARGETS[2], TARGETS[2]],
        ] {
            let root = tempfile::tempdir().unwrap();
            let source = root.path().join("source.pak");
            write_fixture(&source, &targets);
            let original = fs::read(&source).unwrap();
            assert!(prepare(&source, "Rover Indonesia").is_err());
            assert_eq!(fs::read(&source).unwrap(), original);
        }
    }
}
