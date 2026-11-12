// SPDX-License-Identifier: Apache-2.0 OR MIT
//! The `.bzf` file (README, "A document, a file, a folder"): what is written is read back, the same files give the
//! same archive, and an archive a reader must not unpack is refused whole.

use bzf::archive::{LIMIT, read, refused, write};
use bzf::project::Tree;
use std::io::{Cursor, Write};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

fn tree(files: &[(&str, &str)]) -> Tree {
    files.iter().map(|(name, contents)| (name.to_string(), contents.as_bytes().to_vec())).collect()
}

/// An archive as another program may write it: the names as given, unchecked.
fn foreign(build: impl FnOnce(&mut ZipWriter<Cursor<Vec<u8>>>)) -> Vec<u8> {
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    build(&mut writer);
    writer.finish().unwrap().into_inner()
}

fn file(writer: &mut ZipWriter<Cursor<Vec<u8>>>, name: &str, contents: &str) {
    let stored = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
    writer.start_file(name, stored).unwrap();
    writer.write_all(contents.as_bytes()).unwrap();
}

/// `bytes` with the two octets at `central` in every central directory header and at `local` in every local header
/// replaced: the general purpose flags lie at 8 and 6, the compression method at 10 and 8.
fn patched(mut bytes: Vec<u8>, central: usize, local: usize, value: u16) -> Vec<u8> {
    for start in 0..bytes.len() - 4 {
        let at = match &bytes[start..start + 4] {
            b"PK\x01\x02" => start + central,
            b"PK\x03\x04" => start + local,
            _ => continue,
        };
        bytes[at..at + 2].copy_from_slice(&value.to_le_bytes());
    }
    bytes
}

#[test]
fn what_is_written_is_read_back() {
    let project = tree(&[
        ("Haus.bzfproj", "{\"format\":\"bzf/1\"}"),
        ("Daten/project.json", "{}"),
        ("Daten/buildings/a.json", &"{\"walls\": []}".repeat(500)),
        ("Zeichnungen/Grundriss Erdgeschoß.pdf", "%PDF"),
        (".bauzertifikate/objects/ab/cd", "{}"),
    ]);
    let bytes = write(&project).unwrap();
    assert_eq!(&bytes[..2], b"PK");
    assert_eq!(read(&bytes, LIMIT).unwrap(), project);
    // The same files give the same archive, whenever it is written.
    assert_eq!(write(&project).unwrap(), bytes);
    // Deflated: the repeated document takes less room than it has octets.
    assert!(bytes.len() < 6500);
}

#[test]
fn an_archive_of_another_program_is_read() {
    // Stored and deflated entries, entries for the folders, a name in UTF-8.
    let bytes = foreign(|writer| {
        writer.add_directory("Daten/", SimpleFileOptions::default()).unwrap();
        file(writer, "Daten/project.json", "{}");
        writer.start_file("Dokumente/Übergabe.txt", SimpleFileOptions::default()).unwrap();
        writer.write_all(b"deflated").unwrap();
        file(writer, "Haus.bzfproj", "{}");
    });
    let expected =
        tree(&[("Daten/project.json", "{}"), ("Dokumente/Übergabe.txt", "deflated"), ("Haus.bzfproj", "{}")]);
    assert_eq!(read(&bytes, LIMIT).unwrap(), expected);
}

#[test]
fn a_name_that_leaves_the_folder_is_refused() {
    let leaving = [
        "../outside.json",
        "Daten/../../outside.json",
        "Daten/..",
        "/etc/passwd",
        "C:/Windows/system.ini",
        "c:outside.json",
        "Daten\\..\\..\\outside.json",
        "..\\outside.json",
        "Daten//project.json",
        "./Daten/project.json",
        "Daten/./project.json",
        "Daten/pro\u{0}ject.json",
        "Daten/pro\nject.json",
        "",
    ];
    for name in leaving {
        assert!(refused(name).is_some(), "{name:?}");
        // The whole archive is refused, the harmless entry beside it too.
        let bytes = foreign(|writer| {
            file(writer, "Haus.bzfproj", "{}");
            file(writer, name, "{}");
        });
        assert!(read(&bytes, LIMIT).is_err(), "{name:?}");
        assert!(write(&tree(&[(name, "{}")])).is_err(), "{name:?} written");
    }
    for name in ["Daten/project.json", "Daten/a..b.json", "Zeichnungen/Plan (2).pdf", "…/x", ".bauzertifikate/HEAD"] {
        assert_eq!(refused(name), None, "{name}");
    }
}

#[test]
fn what_iso_21320_does_not_allow_is_refused() {
    let plain = foreign(|writer| file(writer, "Daten/project.json", "{}"));
    assert!(read(&plain, LIMIT).is_ok());
    // Encrypted (bit 0 of the flags), and a compression other than stored or deflated (12 is bzip2, 93 zstd).
    assert!(read(&patched(plain.clone(), 8, 6, 1), LIMIT).unwrap_err().contains("encrypted"));
    assert!(read(&patched(plain.clone(), 10, 8, 12), LIMIT).unwrap_err().contains("neither stored nor deflated"));
    assert!(read(&patched(plain.clone(), 10, 8, 93), LIMIT).unwrap_err().contains("neither stored nor deflated"));
    // A link, which could point anywhere once unpacked.
    let link = foreign(|writer| {
        writer.add_symlink("Daten/project.json", "/etc/passwd", SimpleFileOptions::default()).unwrap();
    });
    assert!(read(&link, LIMIT).unwrap_err().contains("a link"));
    assert!(read(b"{\"format\":\"bzf/1\"}", LIMIT).unwrap_err().contains("not a ZIP archive"));
}

#[test]
fn names_that_meet_in_one_file_are_refused() {
    // On a file system that ignores case, the second would overwrite the first.
    let twice = foreign(|writer| {
        file(writer, "Daten/project.json", "{}");
        file(writer, "Daten/Project.JSON", "{\"other\": true}");
    });
    assert!(read(&twice, LIMIT).unwrap_err().contains("twice"));
    let both = foreign(|writer| {
        file(writer, "Daten", "a file");
        file(writer, "Daten/project.json", "{}");
    });
    assert!(read(&both, LIMIT).unwrap_err().contains("a file and a folder"));
}

#[test]
fn no_more_is_unpacked_than_the_limit() {
    // 2 MB of zeros in two entries deflate to a few kB.
    let bomb = foreign(|writer| {
        for name in ["a", "b"] {
            writer.start_file(name, SimpleFileOptions::default()).unwrap();
            writer.write_all(&vec![0; 1 << 20]).unwrap();
        }
    });
    assert!(bomb.len() < 10_000);
    assert!(read(&bomb, 2 << 20).is_ok());
    assert!(read(&bomb, (2 << 20) - 1).unwrap_err().contains("octets unpacked"));
    assert!(read(&bomb, 1 << 20).is_err() && read(&bomb, 10).is_err());
}
