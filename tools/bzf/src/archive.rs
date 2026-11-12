// SPDX-License-Identifier: Apache-2.0 OR MIT
//! The `.bzf` file: a project folder's contents as a ZIP archive (README, "A document, a file, a folder").

use crate::project::Tree;
use std::collections::BTreeSet;
use std::io::{Cursor, Read, Write};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, DateTime, ZipArchive, ZipWriter};

/// What this program reads from one archive at most, unpacked: 1 GiB. An archive may claim any size for its
/// entries, so the limit is on what is read, not on what is claimed.
pub const LIMIT: u64 = 1 << 30;

/// Why a reader refuses an entry by its name; none for a name that stays inside the folder it is unpacked into.
pub fn refused(name: &str) -> Option<&'static str> {
    let drive = name.as_bytes().get(1) == Some(&b':') && name.as_bytes()[0].is_ascii_alphabetic();
    if name.is_empty() {
        Some("an empty name")
    } else if name.starts_with('/') || drive {
        Some("an absolute name")
    } else if name.contains('\\') {
        Some("a backslash in the name")
    } else if name.chars().any(char::is_control) {
        Some("a control character in the name")
    } else if name.trim_end_matches('/').split('/').any(|part| part == "..") {
        Some("a `..` part")
    } else if name.trim_end_matches('/').split('/').any(|part| part.is_empty() || part == ".") {
        Some("an empty or `.` part")
    } else {
        None
    }
}

/// The files of an archive by their names. Refuses the whole archive for one entry a reader must not unpack: a name
/// that leaves the folder, a link, an encrypted entry, a compression other than stored or deflated, a name twice
/// (in any case), or more than `limit` octets unpacked.
pub fn read(bytes: &[u8], limit: u64) -> Result<Tree, String> {
    let mut archive = ZipArchive::new(Cursor::new(bytes)).map_err(|e| format!("not a ZIP archive: {e}"))?;
    let mut tree = Tree::new();
    let mut seen = BTreeSet::new();
    let mut left = limit;
    for index in 0..archive.len() {
        let (name, directory) = {
            let entry = archive.by_index_raw(index).map_err(|e| format!("entry {index}: {e}"))?;
            let name = std::str::from_utf8(entry.name_raw())
                .map_err(|_| format!("entry {index}: its name is not UTF-8"))?
                .to_string();
            if let Some(why) = refused(&name) {
                return Err(format!("{name:?}: {why}"));
            }
            if entry.encrypted() {
                return Err(format!("{name}: encrypted"));
            }
            if entry.is_symlink() {
                return Err(format!("{name}: a link"));
            }
            if !matches!(entry.compression(), CompressionMethod::Stored | CompressionMethod::Deflated) {
                return Err(format!("{name}: neither stored nor deflated"));
            }
            (name, entry.is_dir())
        };
        if !seen.insert(name.trim_end_matches('/').to_lowercase()) {
            return Err(format!("{name}: a name twice"));
        }
        if directory {
            continue;
        }
        let mut entry = archive.by_index(index).map_err(|e| format!("{name}: {e}"))?;
        let mut contents = Vec::new();
        (&mut entry).take(left + 1).read_to_end(&mut contents).map_err(|e| format!("{name}: {e}"))?;
        left = left.checked_sub(contents.len() as u64).ok_or_else(|| format!("more than {limit} octets unpacked"))?;
        tree.insert(name, contents);
    }
    // A file may not lie where another entry needs a folder.
    for name in tree.keys() {
        let mut folder = name.as_str();
        while let Some((parent, _)) = folder.rsplit_once('/') {
            if tree.contains_key(parent) {
                return Err(format!("{parent}: a file and a folder"));
            }
            folder = parent;
        }
    }
    Ok(tree)
}

/// `tree` as an archive: its files in the order of their names, deflated, each with the same time (1980-01-01, the
/// format's earliest), so that the same files give the same archive with the same version of this program.
pub fn write(tree: &Tree) -> Result<Vec<u8>, String> {
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .last_modified_time(DateTime::DEFAULT)
        .unix_permissions(0o644);
    for (name, contents) in tree {
        if let Some(why) = refused(name) {
            return Err(format!("{name:?}: {why}"));
        }
        writer.start_file(name.as_str(), options).map_err(|e| format!("{name}: {e}"))?;
        writer.write_all(contents).map_err(|e| format!("{name}: {e}"))?;
    }
    Ok(writer.finish().map_err(|e| e.to_string())?.into_inner())
}
