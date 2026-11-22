//! Reading Dolphin's configuration and turning ROM files into provider records.

use std::collections::BTreeSet;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use gamenode_plugin_sdk::proto::DiscoveryRecord;

/// Extensions Dolphin can load. `iso`/`gcm` carry a plain disc header; RVZ keeps that header at
/// an offset, and WBFS/WIA currently fall back to the file name for their id. `dol`/`elf`
/// are bare homebrew executables with neither a disc header nor a disc id.
pub const ROM_EXTENSIONS: &[&str] = &[
    "iso", "gcm", "rvz", "wbfs", "wia", "ciso", "gcz", "dol", "elf",
];

/// Homebrew executables. They are discovered and launched like discs but never borrow a disc
/// identity, because their headers carry section offsets or ELF metadata instead of a disc id.
pub const EXECUTABLE_EXTENSIONS: &[&str] = &["dol", "elf"];

/// Where Dolphin stores its configuration on each platform, plus the Flatpak location.
pub fn config_candidates() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Some(config) = std::env::var_os("XDG_CONFIG_HOME").filter(|value| !value.is_empty()) {
        paths.push(PathBuf::from(config).join("dolphin-emu/Dolphin.ini"));
    } else if let Some(home) = std::env::var_os("HOME") {
        paths.push(PathBuf::from(&home).join(".config/dolphin-emu/Dolphin.ini"));
        paths.push(
            PathBuf::from(&home)
                .join(".var/app/org.DolphinEmu.dolphin-emu/config/dolphin-emu/Dolphin.ini"),
        );
    }
    if let Some(appdata) = std::env::var_os("APPDATA") {
        paths.push(PathBuf::from(appdata).join("Dolphin Emulator/Config/Dolphin.ini"));
    }
    if let Some(home) = std::env::var_os("HOME") {
        paths.push(
            PathBuf::from(home).join("Library/Application Support/Dolphin/Config/Dolphin.ini"),
        );
    }
    paths
}

/// The `ISOPathN = ...` entries Dolphin keeps under `[General]`. Unknown lines are ignored.
pub fn configured_iso_paths(config: &str) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    for line in config.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim();
        let Some(index) = key.strip_prefix("ISOPath") else {
            continue;
        };
        if !index.bytes().all(|byte| byte.is_ascii_digit()) || value.is_empty() {
            continue;
        }
        paths.push(PathBuf::from(value));
    }
    paths
}

/// Every ROM root the plugin should scan: Dolphin's configured paths, then the explicit
/// `DOLPHIN_PLUGIN_ROOTS` override (path-list separated by the OS separator).
pub fn roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    for config in config_candidates() {
        let Ok(text) = fs::read_to_string(&config) else {
            continue;
        };
        roots.extend(configured_iso_paths(&text));
    }
    if let Some(extra) = std::env::var_os("DOLPHIN_PLUGIN_ROOTS") {
        roots.extend(std::env::split_paths(&extra));
    }
    let mut seen = BTreeSet::new();
    roots.retain(|root| !root.as_os_str().is_empty() && seen.insert(root.clone()));
    roots
}

fn has_extension(path: &Path, extensions: &[&str]) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            extensions
                .iter()
                .any(|candidate| candidate.eq_ignore_ascii_case(extension))
        })
}

pub fn is_rom(path: &Path) -> bool {
    has_extension(path, ROM_EXTENSIONS)
}

/// True for a homebrew executable, which has no disc header to read.
pub fn is_executable(path: &Path) -> bool {
    has_extension(path, EXECUTABLE_EXTENSIONS)
}

/// RVZ stores the first 0x100 bytes of the disc image (its header) at this offset.
const RVZ_DISC_HEADER_OFFSET: usize = 0x58;
const DISC_HEADER_LEN: usize = 0x100;
/// Inside the disc header: the six-character id and the console's own title field.
const DISC_ID_LEN: usize = 6;
const DISC_TITLE_OFFSET: usize = 0x20;
const DISC_TITLE_LEN: usize = 0x40;

/// The uncompressed disc header a GameCube/Wii image carries: its id and internal title.
struct DiscHeader {
    id: Option<String>,
    title: Option<String>,
}

fn is_disc_id(id: &[u8]) -> bool {
    id.iter()
        .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit())
}

/// The console title field, up to the first NUL and trimmed of padding.
fn disc_title(bytes: &[u8]) -> Option<String> {
    let end = bytes
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(bytes.len());
    let title = String::from_utf8_lossy(&bytes[..end]);
    let title = title.trim();
    (!title.is_empty() && !title.chars().any(char::is_control)).then(|| title.to_owned())
}

/// Reads the disc header of a plain image or of an RVZ container. WBFS/WIA are recognised but
/// their header layout is not read yet, so their id still falls back to the file name.
fn disc_header(path: &Path) -> Option<DiscHeader> {
    let file = fs::File::open(path).ok()?;
    let mut buffer = Vec::with_capacity(RVZ_DISC_HEADER_OFFSET + DISC_HEADER_LEN);
    file.take((RVZ_DISC_HEADER_OFFSET + DISC_HEADER_LEN) as u64)
        .read_to_end(&mut buffer)
        .ok()?;
    let header = if buffer.starts_with(b"RVZ\x01") {
        buffer.get(RVZ_DISC_HEADER_OFFSET..)?
    } else if buffer.starts_with(b"WBFS") || buffer.starts_with(b"WIA\x01") {
        return None;
    } else {
        buffer.as_slice()
    };
    let id = header
        .get(..DISC_ID_LEN)
        .filter(|id| is_disc_id(id))
        .map(|id| String::from_utf8_lossy(id).into_owned());
    let title = header
        .get(DISC_TITLE_OFFSET..DISC_TITLE_OFFSET + DISC_TITLE_LEN)
        .and_then(disc_title);
    Some(DiscHeader { id, title })
}

/// A plain GameCube/Wii disc starts with a six-character id such as `GZLE01` or `RMCE01`.
pub fn read_game_id(path: &Path) -> Option<String> {
    disc_header(path)?.id
}

/// Dolphin users often name compressed files `GZLE01 - The Legend of Zelda.iso`.
pub fn id_from_filename(stem: &str) -> Option<String> {
    let prefix = stem.get(..6)?;
    let rest = &stem[6..];
    let candidate = prefix
        .chars()
        .all(|character| character.is_ascii_uppercase() || character.is_ascii_digit());
    let separated = rest.is_empty()
        || rest
            .chars()
            .next()
            .is_some_and(|character| matches!(character, ' ' | '-' | '_' | '.'));
    (candidate && separated).then(|| prefix.to_owned())
}

pub fn clean_title(file_name: &str) -> String {
    clean_title_with_id(file_name, true)
}

/// Executables keep a leading six-character token: `GZLE01 demo.dol` is not a disc, so stripping
/// the prefix would imply an identity the file does not have.
fn clean_title_with_id(file_name: &str, strip_disc_id: bool) -> String {
    let stem = file_name
        .rsplit_once('.')
        .map_or(file_name, |(stem, _)| stem);
    let stem = match strip_disc_id.then(|| id_from_filename(stem)).flatten() {
        Some(id) => stem[id.len()..].trim_start_matches([' ', '-', '_', '.']),
        None => stem,
    };
    let cleaned = stem.replace(['_', '.'], " ");
    cleaned.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn platform_of(game_id: Option<&str>) -> &'static str {
    match game_id.and_then(|id| id.chars().next()) {
        Some('G') | Some('D') => "gamecube",
        Some('R') | Some('S') | Some('W') => "wii",
        _ => "unknown",
    }
}

/// A stable provider-local id for files without a readable disc header.
fn path_id(path: &Path) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in path.to_string_lossy().as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("dolphin-emu-{hash:016x}")
}

/// One discovery record for one ROM. The folder is the install directory so the library groups
/// games by collection; the file name and console travel as template fields.
pub fn record_for(path: &Path) -> Option<DiscoveryRecord> {
    let file_name = path.file_name()?.to_str()?;
    let executable = is_executable(path);
    let name = clean_title_with_id(file_name, !executable);
    if name.is_empty() {
        return None;
    }
    let install_dir = path.parent()?.to_string_lossy().replace('\\', "/");
    let header = (!executable).then(|| disc_header(path)).flatten();
    let game_id = if executable {
        None
    } else {
        header
            .as_ref()
            .and_then(|header| header.id.clone())
            .or_else(|| {
                id_from_filename(
                    file_name
                        .rsplit_once('.')
                        .map_or(file_name, |(stem, _)| stem),
                )
            })
    };
    let id = game_id.clone().unwrap_or_else(|| path_id(path));
    let platform = platform_of(game_id.as_deref());
    let mut fields = std::collections::HashMap::from([
        ("rom".to_owned(), file_name.to_owned()),
        ("platform".to_owned(), platform.to_owned()),
        ("game_id".to_owned(), game_id.clone().unwrap_or_default()),
    ]);
    if let Some(title) = header.and_then(|header| header.title) {
        fields.insert("title".to_owned(), title);
    }
    Some(DiscoveryRecord {
        id,
        name,
        install_dir,
        fields,
        target: None,
    })
}

/// Recursively collects records below the roots. Hidden directories, symlinks and very deep
/// trees are skipped so an accidental root such as `$HOME` cannot walk the whole disk.
pub fn scan(roots: &[PathBuf]) -> Vec<DiscoveryRecord> {
    const MAX_DEPTH: usize = 8;
    const MAX_ENTRIES: usize = 50_000;

    fn walk(
        directory: &Path,
        depth: usize,
        entries: &mut usize,
        records: &mut Vec<DiscoveryRecord>,
    ) {
        if depth > MAX_DEPTH || *entries >= MAX_ENTRIES {
            return;
        }
        let Ok(read) = fs::read_dir(directory) else {
            return;
        };
        for entry in read.flatten() {
            *entries += 1;
            if *entries >= MAX_ENTRIES {
                return;
            }
            let path = entry.path();
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if file_type.is_symlink() {
                continue;
            }
            if file_type.is_dir() {
                let hidden = entry
                    .file_name()
                    .to_str()
                    .is_some_and(|name| name.starts_with('.'));
                if !hidden {
                    walk(&path, depth + 1, entries, records);
                }
            } else if file_type.is_file() && is_rom(&path) {
                if let Some(record) = record_for(&path) {
                    records.push(record);
                }
            }
        }
    }

    let mut records = Vec::new();
    let mut entries = 0;
    for root in roots {
        if root.is_dir() {
            walk(root, 0, &mut entries, &mut records);
        }
    }
    records.sort_by(|left, right| {
        left.name
            .to_lowercase()
            .cmp(&right.name.to_lowercase())
            .then_with(|| left.id.cmp(&right.id))
    });
    records.dedup_by(|left, right| left.id == right.id);
    records
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_rom(directory: &Path, name: &str, header: &[u8]) -> PathBuf {
        let path = directory.join(name);
        fs::write(&path, header).unwrap();
        path
    }

    #[test]
    fn plain_discs_expose_their_six_character_id() {
        let directory = tempfile::tempdir().unwrap();
        let path = write_rom(directory.path(), "game.iso", b"GZLE01\x00\x00");
        assert_eq!(read_game_id(&path).as_deref(), Some("GZLE01"));
        let name = record_for(&path).unwrap();
        assert_eq!(name.id, "GZLE01");
        assert_eq!(name.fields["platform"], "gamecube");
        assert_eq!(name.fields["rom"], "game.iso");
    }

    #[test]
    fn compressed_discs_fall_back_to_the_file_name() {
        let directory = tempfile::tempdir().unwrap();
        let path = write_rom(
            directory.path(),
            "RMCE01 - Mario Kart Wii.rvz",
            b"RVZ\x01\x00\x00\x00",
        );
        assert_eq!(read_game_id(&path), None);
        let record = record_for(&path).unwrap();
        assert_eq!(record.id, "RMCE01");
        assert_eq!(record.name, "Mario Kart Wii");
        assert_eq!(record.fields["platform"], "wii");
    }

    #[test]
    fn rvz_headers_expose_the_disc_id_and_internal_title() {
        let directory = tempfile::tempdir().unwrap();
        let mut header = vec![0u8; RVZ_DISC_HEADER_OFFSET + DISC_HEADER_LEN];
        header[..4].copy_from_slice(b"RVZ\x01");
        header[RVZ_DISC_HEADER_OFFSET..RVZ_DISC_HEADER_OFFSET + DISC_ID_LEN]
            .copy_from_slice(b"GPIE01");
        let title = RVZ_DISC_HEADER_OFFSET + DISC_TITLE_OFFSET;
        header[title..title + 6].copy_from_slice(b"Pikmin");
        let path = write_rom(directory.path(), "Pikmin (USA) (Rev 1).rvz", &header);

        assert_eq!(read_game_id(&path).as_deref(), Some("GPIE01"));
        let record = record_for(&path).unwrap();
        assert_eq!(record.id, "GPIE01");
        assert_eq!(record.name, "Pikmin (USA) (Rev 1)");
        assert_eq!(record.fields["platform"], "gamecube");
        assert_eq!(record.fields["title"], "Pikmin");
    }

    #[test]
    fn a_real_rvz_exposes_its_identity_when_the_acceptance_path_is_set() {
        let Some(path) = std::env::var_os("DOLPHIN_RVZ_PATH") else {
            return;
        };
        let record = record_for(Path::new(&path)).expect("a discovery record");
        let id = &record.fields["game_id"];
        assert_eq!(id.len(), 6, "{id}");
        assert!(
            id.chars()
                .all(|character| character.is_ascii_uppercase() || character.is_ascii_digit())
        );
        assert_ne!(record.fields["platform"], "unknown");
        assert!(record.fields.contains_key("title"));
    }

    #[test]
    fn plain_discs_expose_the_header_title_when_present() {
        let directory = tempfile::tempdir().unwrap();
        let mut header = vec![0u8; DISC_HEADER_LEN];
        header[..6].copy_from_slice(b"GZLE01");
        let title = b"The Legend of Zelda";
        header[DISC_TITLE_OFFSET..DISC_TITLE_OFFSET + title.len()].copy_from_slice(title);
        let path = write_rom(directory.path(), "zelda.iso", &header);

        let record = record_for(&path).unwrap();
        assert_eq!(record.fields["title"], "The Legend of Zelda");
    }

    #[test]
    fn files_without_a_disc_id_get_a_stable_path_id() {
        let directory = tempfile::tempdir().unwrap();
        let path = write_rom(
            directory.path(),
            "Homebrew Demo.iso",
            b"\x00\x00\x00\x00\x00\x00\x00\x00",
        );
        let first = record_for(&path).unwrap();
        let second = record_for(&path).unwrap();
        assert_eq!(first.id, second.id);
        assert!(first.id.starts_with("dolphin-emu-"));
        assert_eq!(first.name, "Homebrew Demo");
        assert_eq!(first.fields["platform"], "unknown");
    }

    #[test]
    fn only_dolphin_extensions_count_case_insensitively() {
        for name in [
            "a.iso", "b.GCM", "c.rvz", "d.wbfs", "e.wia", "f.ciso", "g.gcz", "h.dol", "i.ELF",
        ] {
            assert!(is_rom(Path::new(name)), "{name}");
        }
        for name in ["a.txt", "b.iso.part", "c", "d.dol.txt"] {
            assert!(!is_rom(Path::new(name)), "{name}");
        }
    }

    #[test]
    fn homebrew_executables_are_discovered_without_a_disc_id() {
        let directory = tempfile::tempdir().unwrap();
        let dol = write_rom(directory.path(), "240pSuite.dol", &[0, 0, 1, 0, 0, 0, 0, 0]);
        let record = record_for(&dol).unwrap();
        assert_eq!(record.name, "240pSuite");
        assert_eq!(record.fields["rom"], "240pSuite.dol");
        assert_eq!(record.fields["game_id"], "");
        assert_eq!(record.fields["platform"], "unknown");
        assert!(record.id.starts_with("dolphin-emu-"));

        let elf = write_rom(directory.path(), "homebrew.ELF", b"\x7fELF\x01\x02\x01\x00");
        let record = record_for(&elf).unwrap();
        assert_eq!(record.name, "homebrew");
        assert_eq!(record.fields["game_id"], "");
        assert_eq!(record.fields["platform"], "unknown");
        assert!(record.id.starts_with("dolphin-emu-"));
    }

    #[test]
    fn executables_never_borrow_a_disc_id_or_strip_the_name() {
        let directory = tempfile::tempdir().unwrap();
        let named = write_rom(directory.path(), "GZLE01 demo.dol", &[0; 8]);
        let record = record_for(&named).unwrap();
        assert_eq!(record.fields["game_id"], "");
        assert_eq!(record.name, "GZLE01 demo");
        assert!(record.id.starts_with("dolphin-emu-"));

        let header = write_rom(directory.path(), "plain.dol", b"GZLE01\x00\x00");
        let record = record_for(&header).unwrap();
        assert_eq!(record.fields["game_id"], "");
        assert!(record.id.starts_with("dolphin-emu-"));
    }

    #[test]
    fn moving_a_homebrew_file_changes_its_identity() {
        let directory = tempfile::tempdir().unwrap();
        let first = record_for(&write_rom(directory.path(), "homebrew.dol", &[0; 8])).unwrap();
        let second = record_for(&write_rom(directory.path(), "renamed.dol", &[0; 8])).unwrap();
        assert_ne!(first.id, second.id);
        assert_eq!(first.fields["game_id"], second.fields["game_id"]);
    }

    #[test]
    fn dolphin_iso_paths_are_read_and_other_keys_ignored() {
        let config = "\
[General]
ISOPath0 = /games/gamecube
ISOPath1 = D:\\\\Games\\\\Wii
ISOPathTwo = ignored
ConfigPath = /tmp
";
        assert_eq!(
            configured_iso_paths(config),
            [
                PathBuf::from("/games/gamecube"),
                PathBuf::from("D:\\\\Games\\\\Wii")
            ]
        );
    }

    #[test]
    fn scanning_is_recursive_sorted_and_ignores_hidden_directories() {
        let directory = tempfile::tempdir().unwrap();
        let nested = directory.path().join("wii");
        fs::create_dir_all(&nested).unwrap();
        let hidden = directory.path().join(".git");
        fs::create_dir_all(&hidden).unwrap();
        write_rom(directory.path(), "b.iso", b"GZLE01\x00\x00");
        write_rom(&nested, "a.iso", b"RMCE01\x00\x00");
        write_rom(&hidden, "ignored.iso", b"GZLE01\x00\x00");
        fs::write(directory.path().join("notes.txt"), b"x").unwrap();
        let records = scan(&[directory.path().to_path_buf()]);
        assert_eq!(
            records
                .iter()
                .map(|record| record.name.as_str())
                .collect::<Vec<_>>(),
            ["a", "b"]
        );
    }
}
