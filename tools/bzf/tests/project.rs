// SPDX-License-Identifier: Apache-2.0 OR MIT
//! A project checked as any reader can check it: the examples as folders and as one file, what a share holds, when a
//! result is current, and the command-line program from packing to verifying.

use bzf::archive::{self, LIMIT};
use bzf::project::{Tree, check, read_folder, share, snapshot};
use bzf::schemas::Schemas;
use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const BALANCE: &str = "Ergebnisse/Energieberatung/Energieausweis/ergebnis.json";
const ROADMAP: &str = "Ergebnisse/Energieberatung/Sanierungsfahrplan/ergebnis.json";
const BUILDING: &str = "Daten/buildings/019a5e00-0000-7000-8000-000000000002.json";

fn examples() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples")
}

fn house() -> Tree {
    read_folder(&examples().join("haus-am-hang")).unwrap()
}

fn json_of(tree: &Tree, path: &str) -> Value {
    serde_json::from_slice(&tree[path]).unwrap()
}

fn put(tree: &mut Tree, path: &str, doc: &Value) {
    tree.insert(path.to_string(), serde_json::to_vec_pretty(doc).unwrap());
}

fn has(notes: &[String], path: &str, what: &str) -> bool {
    notes.iter().any(|note| note.starts_with(path) && note.contains(what))
}

#[test]
fn the_example_projects_are_valid() {
    let schemas = Schemas::new();
    for name in ["haus-am-hang", "werkstatt-am-bach"] {
        let report = check(&read_folder(&examples().join(name)).unwrap(), &schemas);
        assert!(report.errors.is_empty(), "{name}: {:?}", report.errors);
        assert!(report.checked > 1, "{name}");
    }
    let report = check(&house(), &schemas);
    assert!(has(&report.notes, BALANCE, "current") && has(&report.notes, ROADMAP, "current"), "{:?}", report.notes);
    // The example's signature holds, and the note names the signer and that the certificate is the signer's own.
    assert!(has(&report.notes, BALANCE, "signed (energieberater) by "), "{:?}", report.notes);
    assert!(has(&report.notes, BALANCE, "a certificate of the signer's own"), "{:?}", report.notes);
    // The program's own document is named with the class the root gives it.
    assert!(has(&report.notes, "Daten/energieberatung/settings.json", "class building"), "{:?}", report.notes);
}

#[test]
fn what_breaks_a_rule_is_an_error() {
    let schemas = Schemas::new();
    let errors = |tree: &Tree| check(tree, &schemas).errors;

    let mut tree = house();
    tree.remove("Haus am Hang.bzfproj");
    assert!(errors(&tree).iter().any(|e| e.contains("no project file")));
    tree.insert("A.bzfproj".into(), b"{\"format\":\"bzf/1\"}".to_vec());
    assert!(errors(&tree).iter().any(|e| e.starts_with("A.bzfproj as bzf")), "{:?}", errors(&tree));
    tree.insert("B.bzfproj".into(), house()["Haus am Hang.bzfproj"].clone());
    assert!(errors(&tree).iter().any(|e| e.contains("more than one project file")));

    // The project file under its name of edition 0.1 is read, and noted.
    let mut tree = house();
    let marker = tree.remove("Haus am Hang.bzfproj").unwrap();
    tree.insert("Haus am Hang.bzf".into(), marker);
    let report = check(&tree, &schemas);
    assert!(report.errors.is_empty() && has(&report.notes, "Haus am Hang.bzf", "edition 0.1"), "{:?}", report.notes);

    let mut tree = house();
    let mut building = json_of(&tree, BUILDING);
    building["parts"] = json!("none");
    put(&mut tree, BUILDING, &building);
    assert!(errors(&tree).iter().any(|e| e.starts_with(BUILDING) && e.contains("as building")));

    let mut tree = house();
    tree.insert(BUILDING.into(), b"{ not JSON".to_vec());
    assert!(errors(&tree).iter().any(|e| e.starts_with(BUILDING) && e.contains("not JSON")));

    // A name twice in one object: no two readers need agree on the document's hash. In a program's own document too.
    let mut tree = house();
    let settings = r#"{"schema": "1.0", "balance": {"partialHeating": false, "partialHeating": true}}"#;
    tree.insert("Daten/energieberatung/settings.json".into(), settings.into());
    assert!(errors(&tree).iter().any(|e| e.contains("settings.json: the name \"partialHeating\" twice")));
    tree.insert("Daten/energieberatung/settings.json".into(), b"{ not JSON".to_vec());
    assert!(errors(&tree).iter().any(|e| e.contains("settings.json: not JSON")));

    // An issue that states another figure than the result it names.
    let mut tree = house();
    let issue = "Versendet/2026-10-20 Energieausweis/versand.json";
    let mut sent = json_of(&tree, issue);
    sent["figures"][0]["primaryEnergy"] = json!(150);
    put(&mut tree, issue, &sent);
    assert!(
        errors(&tree).iter().any(|e| e.starts_with(issue) && e.contains("primaryEnergy: issued 150")),
        "{:?}",
        errors(&tree)
    );

    // A result without its required fields, and one whose signature is for the record before a change.
    let mut tree = house();
    let mut balance = json_of(&tree, BALANCE);
    balance["figures"][0]["primaryEnergy"] = json!(120);
    put(&mut tree, BALANCE, &balance);
    let found = errors(&tree);
    assert!(
        found.iter().any(|e| e.contains("ergebnis.energieberater.jws") && e.contains("does not hold")),
        "{found:?}"
    );
    balance.as_object_mut().unwrap().remove("figures");
    put(&mut tree, BALANCE, &balance);
    assert!(errors(&tree).iter().any(|e| e.starts_with(BALANCE) && e.contains("as result")));
}

#[test]
fn a_result_is_current_until_a_document_it_names_changes() {
    let schemas = Schemas::new();
    let mut tree = house();
    // A document neither result names changes: both stay current.
    let consumption = tree.keys().find(|path| path.starts_with("Daten/consumption/")).unwrap().clone();
    let mut doc = json_of(&tree, &consumption);
    doc["note"] = json!("Zähler getauscht");
    put(&mut tree, &consumption, &doc);
    let report = check(&tree, &schemas);
    assert!(has(&report.notes, BALANCE, "current") && has(&report.notes, ROADMAP, "current"), "{:?}", report.notes);

    // The program's settings change: the result that names them is stale, the other is not. Neither is an error.
    let mut settings = json_of(&tree, "Daten/energieberatung/settings.json");
    settings["balance"]["partialHeating"] = json!(true);
    put(&mut tree, "Daten/energieberatung/settings.json", &settings);
    let report = check(&tree, &schemas);
    assert!(report.errors.is_empty(), "{:?}", report.errors);
    assert!(has(&report.notes, BALANCE, "stale, changed since: energieberatung/settings.json"), "{:?}", report.notes);
    assert!(has(&report.notes, ROADMAP, "current"), "{:?}", report.notes);

    // The building changes: both are stale. Written another way (6 for 6.0), it is the same document.
    let mut tree = house();
    let mut building = json_of(&tree, BUILDING);
    let n50 = building["airtightness"]["n50"].as_f64().unwrap();
    building["airtightness"]["n50"] = json!(n50 + 0.5);
    put(&mut tree, BUILDING, &building);
    let report = check(&tree, &schemas);
    assert!(has(&report.notes, BALANCE, "stale") && has(&report.notes, ROADMAP, "stale"), "{:?}", report.notes);
    building["airtightness"]["n50"] = if n50.fract() == 0.0 { json!(n50 as i64) } else { json!(n50) };
    put(&mut tree, BUILDING, &building);
    let report = check(&tree, &schemas);
    assert!(has(&report.notes, BALANCE, "current") && has(&report.notes, ROADMAP, "current"), "{:?}", report.notes);

    // A named document that is not there: the reader cannot tell, and says which is missing.
    tree.remove("Daten/energieberatung/settings.json");
    let report = check(&tree, &schemas);
    assert!(
        has(&report.notes, BALANCE, "cannot be told, not here: energieberatung/settings.json"),
        "{:?}",
        report.notes
    );
}

#[test]
fn a_share_holds_the_building_and_nothing_about_people_or_money() {
    let schemas = Schemas::new();
    let mut whole = house();
    // A store and a received file, which the example leaves out.
    whole.insert(".bauzertifikate/objects/ab/cdef".into(), b"{}".to_vec());
    whole.insert("Fotos/Kessel Typenschild.jpg".into(), b"JFIF".to_vec());
    whole.insert("Fotos/nicht verzeichnet.jpg".into(), b"JFIF".to_vec());
    whole.insert("Daten/aufmass/angebot.json".into(), b"{}".to_vec());

    let shared = share(&whole, &schemas);
    let names: Vec<&str> = shared.keys().map(String::as_str).collect();
    for kept in [
        "Haus am Hang.bzfproj",
        "Daten/project.json",
        "Daten/received.json",
        BUILDING,
        "Daten/energieberatung/settings.json",
        BALANCE,
        ROADMAP,
        "Ergebnisse/Energieberatung/Energieausweis/ergebnis.energieberater.jws",
        "Fotos/Kessel Typenschild.jpg",
    ] {
        assert!(names.contains(&kept), "{kept} is not in the share: {names:?}");
    }
    assert!(names.iter().any(|name| name.starts_with("Daten/roadmaps/")));
    assert!(names.iter().any(|name| name.starts_with("Daten/consumption/")));
    // People, money, the issues (they name whom a document went to), the store, a file no record lists, and an
    // app's document whose class the root does not give.
    for left in ["Daten/parties/", "Daten/costs/", "Versendet/", ".bauzertifikate/", "Fotos/nicht", "Daten/aufmass/"] {
        assert!(!names.iter().any(|name| name.starts_with(left)), "{left} is in the share");
    }
    assert!(whole.keys().any(|name| name.starts_with("Daten/parties/")));
    assert!(whole.keys().any(|name| name.starts_with("Daten/costs/")));

    // The share is a project: valid, its results current and its signature holding.
    let report = check(&shared, &schemas);
    assert!(report.errors.is_empty(), "{:?}", report.errors);
    assert!(has(&report.notes, BALANCE, "current") && has(&report.notes, BALANCE, "signed"), "{:?}", report.notes);

    // A snapshot is everything but the store.
    let snap = snapshot(&whole);
    assert_eq!(snap.len(), whole.len() - 1);
    assert!(snap.contains_key("Versendet/2026-10-20 Energieausweis/versand.json"));
}

/// The example project as one file, as `bzf pack --share` writes it. The file in the repository is compared by what
/// it holds, not by its octets, which another version of the compression may write differently.
#[test]
fn the_example_file_is_the_example_folders_share() {
    let schemas = Schemas::new();
    let file = examples().join("Haus am Hang.bzf");
    let packed = archive::read(&fs::read(&file).unwrap(), LIMIT).unwrap();
    assert_eq!(
        packed,
        share(&house(), &schemas),
        "run: bzf pack --share examples/haus-am-hang 'examples/Haus am Hang.bzf'"
    );
    let report = check(&packed, &schemas);
    assert!(report.errors.is_empty(), "{:?}", report.errors);
}

fn bzf(args: &[&str]) -> (Option<i32>, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_bzf")).args(args).output().unwrap();
    let text = format!("{}{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
    (output.status.code(), text)
}

#[test]
fn the_program_packs_unpacks_checks_and_verifies() {
    let scratch = Path::new(env!("CARGO_TARGET_TMPDIR")).join("program");
    let _ = fs::remove_dir_all(&scratch);
    fs::create_dir_all(&scratch).unwrap();
    let path = |name: &str| scratch.join(name).to_string_lossy().into_owned();
    let folder = examples().join("haus-am-hang").to_string_lossy().into_owned();

    let (code, text) = bzf(&["check", &folder]);
    assert_eq!(code, Some(0), "{text}");
    assert!(text.contains("valid (") && text.contains("0 errors"), "{text}");

    // A snapshot under another name, unpacked: the project file takes the file's name, and the folder checks.
    let (code, text) = bzf(&["pack", &folder, &path("Musterhaus.bzf")]);
    assert_eq!(code, Some(0), "{text}");
    let (code, text) = bzf(&["check", &path("Musterhaus.bzf")]);
    assert_eq!(code, Some(0), "{text}");
    let (code, text) = bzf(&["unpack", &path("Musterhaus.bzf"), &path("unpacked")]);
    assert_eq!(code, Some(0), "{text}");
    assert!(scratch.join("unpacked/Musterhaus.bzfproj").is_file());
    assert!(!scratch.join("unpacked/Haus am Hang.bzfproj").exists());
    let mut expected = snapshot(&house());
    let marker = expected.remove("Haus am Hang.bzfproj").unwrap();
    expected.insert("Musterhaus.bzfproj".into(), marker);
    assert_eq!(read_folder(&scratch.join("unpacked")).unwrap(), expected);

    // A file is never changed in place, and a folder with something in it is not unpacked into.
    assert_eq!(bzf(&["pack", &folder, &path("Musterhaus.bzf")]).0, Some(2));
    assert_eq!(bzf(&["unpack", &path("Musterhaus.bzf"), &path("unpacked")]).0, Some(2));
    assert_eq!(bzf(&["pack", "--share", "--history", &folder, &path("x.bzf")]).0, Some(2));
    assert_eq!(bzf(&["pack", &path("unpacked/Daten"), &path("y.bzf")]).0, Some(2));

    // The hash a result names a document by.
    let (code, text) = bzf(&["hash", &format!("{folder}/{BUILDING}")]);
    let named = json_of(&house(), BALANCE)["inputs"][BUILDING.strip_prefix("Daten/").unwrap()].clone();
    assert_eq!((code, json!(text.trim())), (Some(0), named));

    // The example's signature holds; once the record changes, verify and check end with 1.
    let record = path(&format!("unpacked/{BALANCE}"));
    let (code, text) = bzf(&["verify", &record]);
    assert_eq!(code, Some(0), "{text}");
    assert!(text.contains("energieberater: signed by "), "{text}");
    let changed = fs::read_to_string(&record).unwrap().replace("\"primaryEnergy\": 182", "\"primaryEnergy\": 150");
    fs::write(&record, changed).unwrap();
    let (code, text) = bzf(&["verify", &record]);
    assert_eq!(code, Some(1), "{text}");
    let (code, text) = bzf(&["check", &path("unpacked")]);
    assert_eq!(code, Some(1), "{text}");
    assert!(text.contains("error  ") && text.contains("not valid"), "{text}");

    // An archive whose entry leaves the folder is not unpacked at all.
    let bad = {
        use std::io::Write;
        let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        let options = zip::write::SimpleFileOptions::default();
        writer.start_file("Haus.bzfproj", options).unwrap();
        writer.write_all(b"{\"format\":\"bzf/1\"}").unwrap();
        writer.start_file("../escaped.json", options).unwrap();
        writer.write_all(b"{}").unwrap();
        writer.finish().unwrap().into_inner()
    };
    fs::write(scratch.join("Slip.bzf"), bad).unwrap();
    let (code, text) = bzf(&["unpack", &path("Slip.bzf"), &path("slipped/into")]);
    assert_eq!(code, Some(2), "{text}");
    assert!(!scratch.join("slipped").exists() && !scratch.join("escaped.json").exists());
    fs::remove_dir_all(&scratch).unwrap();
}
