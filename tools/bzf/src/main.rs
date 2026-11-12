// SPDX-License-Identifier: Apache-2.0 OR MIT
//! `bzf`: checks, packs, unpacks and signs BZF projects.

use bzf::canonical::{hash, names_twice};
use bzf::project::{self, Tree};
use bzf::schemas::Schemas;
use bzf::{archive, signature};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

const USAGE: &str = "\
bzf: checks, packs, unpacks and signs BZF projects (format bzf/1)

  bzf check <folder | file.bzf>
      Checks a project against the format: its project file, the documents under Daten/, the references between
      them, the results (current, stale, signed by whom) and the issues. Ends with 1 if a rule is broken.

  bzf pack [--history | --share] <folder> <file.bzf>
      Writes the project folder as one .bzf file. Without an option a snapshot: everything but the store.
      --history  with the store, so a full copy of the project and its history
      --share    only what may leave the organisation: the class `building`, the received files, the results

  bzf unpack <file.bzf> <folder>
      Unpacks a .bzf file into a folder that is not there yet or is empty, and names the project file after it.

  bzf hash <document.json>
      The SHA-256 of the document's canonical JSON, as results and issues name their inputs.

  bzf sign <ergebnis.json> --name <name> --key <key.pem> --cert <certificate.pem> [--time <unix seconds>]
      Signs a result record and writes ergebnis.<name>.jws beside it. The key is a P-256 key in PKCS#8 PEM.

  bzf verify <ergebnis.json>
      Verifies the signatures beside a result record and names who signed. Ends with 1 if one does not hold.

  bzf version
      The specification edition this program checks by.
";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let rest: Vec<&str> = args.iter().skip(1).map(String::as_str).collect();
    let done = match args.first().map(String::as_str) {
        Some("check") => check(&rest),
        Some("pack") => pack(&rest),
        Some("unpack") => unpack(&rest),
        Some("hash") => hash_of(&rest),
        Some("sign") => sign(&rest),
        Some("verify") => verify(&rest),
        Some("version" | "--version" | "-V") => {
            println!("bzf {} (specification edition {0}, format bzf/1)", env!("CARGO_PKG_VERSION"));
            Ok(true)
        }
        Some("help" | "--help" | "-h") | None => {
            print!("{USAGE}");
            Ok(true)
        }
        Some(other) => Err(format!("no command `{other}`\n\n{USAGE}")),
    };
    match done {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(message) => {
            eprintln!("bzf: {message}");
            ExitCode::from(2)
        }
    }
}

/// The options given, each with its value (empty for a flag).
type Given<'a> = Vec<(&'a str, &'a str)>;

/// The arguments without the options, and the value of each option in `valued`; an option in neither is an error.
fn options<'a>(args: &[&'a str], flags: &[&str], valued: &[&str]) -> Result<(Vec<&'a str>, Given<'a>), String> {
    let (mut plain, mut given) = (Vec::new(), Vec::new());
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        if valued.contains(arg) {
            given.push((*arg, *args.next().ok_or_else(|| format!("{arg} needs a value"))?));
        } else if flags.contains(arg) {
            given.push((*arg, ""));
        } else if arg.starts_with("--") {
            return Err(format!("no option {arg}"));
        } else {
            plain.push(*arg);
        }
    }
    Ok((plain, given))
}

fn read(path: &str) -> Result<Vec<u8>, String> {
    fs::read(path).map_err(|e| format!("{path}: {e}"))
}

/// A JSON document a hash can be taken of: JSON, and no name twice in one object (HASHES.md).
fn document(path: &str) -> Result<Value, String> {
    let bytes = read(path)?;
    let doc = serde_json::from_slice(&bytes).map_err(|e| format!("{path}: not JSON: {e}"))?;
    match names_twice(&bytes).unwrap_or_default().first() {
        Some(name) => {
            Err(format!("{path}: the name \"{name}\" twice in one object, so it has no hash readers agree on"))
        }
        None => Ok(doc),
    }
}

/// A project's files, from a folder or a `.bzf` file.
fn tree(path: &str) -> Result<Tree, String> {
    if Path::new(path).is_dir() {
        project::read_folder(Path::new(path))
    } else {
        archive::read(&read(path)?, archive::LIMIT).map_err(|e| format!("{path}: {e}"))
    }
}

fn check(args: &[&str]) -> Result<bool, String> {
    let (plain, _) = options(args, &[], &[])?;
    let [path] = plain[..] else { return Err("check takes one folder or .bzf file".into()) };
    let report = project::check(&tree(path)?, &Schemas::new());
    for note in &report.notes {
        println!("note   {note}");
    }
    for error in &report.errors {
        println!("error  {error}");
    }
    let verdict = if report.errors.is_empty() { "valid" } else { "not valid" };
    println!("{path}: {verdict} ({} documents checked, {} errors)", report.checked, report.errors.len());
    Ok(report.errors.is_empty())
}

fn pack(args: &[&str]) -> Result<bool, String> {
    let (plain, given) = options(args, &["--history", "--share"], &[])?;
    let [folder, file] = plain[..] else { return Err("pack takes a folder and the .bzf file to write".into()) };
    let (history, share) = (given.contains(&("--history", "")), given.contains(&("--share", "")));
    if history && share {
        return Err("a share has no history: the store holds every document the project ever held".into());
    }
    if !file.ends_with(".bzf") {
        return Err(format!("{file}: a project in one file is named <Name>.bzf"));
    }
    if Path::new(file).exists() {
        return Err(format!("{file} is there already; a .bzf file is written whole and never changed in place"));
    }
    let whole = project::read_folder(Path::new(folder))?;
    let [_] = project::project_files(&whole)[..] else {
        return Err(format!("{folder}: no project folder (exactly one <Name>.bzfproj at its root)"));
    };
    let packed = match (history, share) {
        (true, _) => whole.clone(),
        (_, true) => project::share(&whole, &Schemas::new()),
        _ => project::snapshot(&whole),
    };
    fs::write(file, archive::write(&packed)?).map_err(|e| format!("{file}: {e}"))?;
    println!("{file}: {} of {} files", packed.len(), whole.len());
    Ok(true)
}

fn unpack(args: &[&str]) -> Result<bool, String> {
    let (plain, _) = options(args, &[], &[])?;
    let [file, folder] = plain[..] else { return Err("unpack takes a .bzf file and the folder to write".into()) };
    let mut tree = archive::read(&read(file)?, archive::LIMIT).map_err(|e| format!("{file}: {e}"))?;
    let [project_file] = project::project_files(&tree)[..] else {
        return Err(format!("{file}: not a project (exactly one <Name>.bzfproj at the archive's root)"));
    };
    // The project's name is the .bzf file's own name.
    let name = Path::new(file).file_stem().map(|stem| stem.to_string_lossy().into_owned()).unwrap_or_default();
    let project_file = project_file.to_string();
    let contents = tree.remove(&project_file).unwrap_or_default();
    tree.insert(format!("{name}.bzfproj"), contents);

    let root = PathBuf::from(folder);
    if root.exists() && fs::read_dir(&root).map_err(|e| format!("{folder}: {e}"))?.next().is_some() {
        return Err(format!("{folder} is not empty"));
    }
    for (path, bytes) in &tree {
        // Every name was checked on reading, so each part is a plain name below the folder.
        let target = path.split('/').fold(root.clone(), |target, part| target.join(part));
        let parent = target.parent().unwrap_or(&root);
        fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
        fs::write(&target, bytes).map_err(|e| format!("{}: {e}", target.display()))?;
    }
    println!("{folder}: {} files", tree.len());
    Ok(true)
}

fn hash_of(args: &[&str]) -> Result<bool, String> {
    let (plain, _) = options(args, &[], &[])?;
    let [path] = plain[..] else { return Err("hash takes one document".into()) };
    println!("{}", hash(&document(path)?));
    Ok(true)
}

/// The signature files beside a result record, by their names.
fn beside(record: &str) -> Result<Vec<(String, PathBuf)>, String> {
    let folder = Path::new(record).parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."));
    let entries = fs::read_dir(folder).map_err(|e| format!("{}: {e}", folder.display()))?;
    let mut found = Vec::new();
    for entry in entries.flatten() {
        let file = entry.file_name().to_string_lossy().into_owned();
        if let Some(name) = signature::name_of(&file) {
            found.push((name.to_string(), entry.path()));
        }
    }
    found.sort();
    Ok(found)
}

fn sign(args: &[&str]) -> Result<bool, String> {
    let (plain, given) = options(args, &[], &["--name", "--key", "--cert", "--time"])?;
    let [record] = plain[..] else { return Err("sign takes one result record".into()) };
    let option = |name: &str| given.iter().find(|(option, _)| *option == name).map(|(_, value)| *value);
    let needed = |name: &str| option(name).ok_or_else(|| format!("sign needs {name}"));
    let (name, key, certificate) = (needed("--name")?, needed("--key")?, needed("--cert")?);
    let file = format!("ergebnis.{name}.jws");
    if signature::name_of(&file).is_none() {
        return Err(format!("{name}: a signature's name is made of letters, digits, - and _"));
    }
    if Path::new(record).file_name().and_then(|f| f.to_str()) != Some("ergebnis.json") {
        return Err(format!("{record}: a result record is named ergebnis.json"));
    }
    let doc = document(record)?;
    let errors = Schemas::new().errors("result", &doc);
    if !errors.is_empty() {
        return Err(format!("{record} is no valid result record: {}", errors.join("; ")));
    }
    let time = match option("--time") {
        Some(time) => time.parse().map_err(|_| format!("{time}: not a time in seconds since 1970"))?,
        None => SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e| e.to_string())?.as_secs() as i64,
    };
    let text = |path: &str| String::from_utf8(read(path)?).map_err(|_| format!("{path}: not PEM"));
    let jws = signature::sign(&doc, &text(key)?, &text(certificate)?, time)?;
    let target = Path::new(record).with_file_name(&file);
    fs::write(&target, format!("{jws}\n")).map_err(|e| format!("{}: {e}", target.display()))?;
    println!("{}", target.display());
    Ok(true)
}

fn verify(args: &[&str]) -> Result<bool, String> {
    let (plain, _) = options(args, &[], &[])?;
    let [record] = plain[..] else { return Err("verify takes one result record".into()) };
    let doc = document(record)?;
    let signatures = beside(record)?;
    if signatures.is_empty() {
        println!("{record}: not signed");
        return Ok(true);
    }
    let mut holds = true;
    for (name, path) in signatures {
        let text =
            String::from_utf8_lossy(&fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?).into_owned();
        match signature::verify(&doc, &text) {
            Ok(signer) => {
                println!("{name}: signed by {} at {}", signer.subject, utc(signer.time));
                if signer.issuer == signer.subject {
                    println!("  by a certificate of the signer's own, which no one else vouches for");
                } else {
                    println!("  certificate issued by {}", signer.issuer);
                }
                if !signer.in_validity {
                    println!("  the time lies outside the certificate's validity");
                }
            }
            Err(failure) => {
                println!("{name}: {failure}");
                holds = false;
            }
        }
    }
    println!("A signature says who signed these figures and that they are unchanged since. Whether to trust the");
    println!("certificate is the reader's decision; nothing here says the figures are right.");
    Ok(holds)
}

/// Seconds since 1970 as a date and time in UTC, by the days-to-civil reckoning of the proleptic Gregorian calendar.
fn utc(seconds: i64) -> String {
    let (days, of_day) = (seconds.div_euclid(86_400), seconds.rem_euclid(86_400));
    let shifted = days + 719_468;
    let (era, day_of_era) = (shifted.div_euclid(146_097), shifted.rem_euclid(146_097));
    let year_of_era = (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_from_march = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_from_march + 2) / 5 + 1;
    let month = if month_from_march < 10 { month_from_march + 3 } else { month_from_march - 9 };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z", of_day / 3_600, of_day % 3_600 / 60, of_day % 60)
}

#[cfg(test)]
mod tests {
    #[test]
    fn seconds_are_shown_as_a_date() {
        assert_eq!(super::utc(0), "1970-01-01T00:00:00Z");
        assert_eq!(super::utc(951_782_400), "2000-02-29T00:00:00Z");
        assert_eq!(super::utc(1_792_500_300), "2026-10-20T12:45:00Z");
        assert_eq!(super::utc(-1), "1969-12-31T23:59:59Z");
        assert_eq!(super::utc(4_102_444_799), "2099-12-31T23:59:59Z");
    }
}
