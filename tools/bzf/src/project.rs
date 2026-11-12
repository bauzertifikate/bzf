// SPDX-License-Identifier: Apache-2.0 OR MIT
//! A project as its files, from a folder or a `.bzf` file, and what can be told of it without the program that
//! wrote it: whether its documents are valid, whether its results are current, who signed them.

use crate::canonical::{hash, names_twice};
use crate::schemas::{Schemas, schema_of};
use crate::signature;
use serde_json::Value;
use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::Path;

/// A project's files by their paths in the folder, with `/` between the parts.
pub type Tree = BTreeMap<String, Vec<u8>>;

/// The store's directory.
pub const STORE: &str = ".bauzertifikate/";

/// The files of the folder `dir`. Links are not followed, and hidden files other than the store are left out (a
/// system's `.DS_Store` is no part of a project).
pub fn read_folder(dir: &Path) -> Result<Tree, String> {
    fn walk(dir: &Path, prefix: &str, tree: &mut Tree) -> Result<(), String> {
        let mut entries: Vec<_> = fs::read_dir(dir)
            .and_then(|entries| entries.collect::<Result<Vec<_>, _>>())
            .map_err(|e| format!("{}: {e}", dir.display()))?;
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            let name = entry.file_name().to_string_lossy().into_owned();
            let path = format!("{prefix}{name}");
            let kind = entry.file_type().map_err(|e| format!("{path}: {e}"))?;
            if name.starts_with('.') && format!("{path}/") != STORE {
                continue;
            }
            if kind.is_dir() {
                walk(&entry.path(), &format!("{path}/"), tree)?;
            } else if kind.is_file() {
                tree.insert(path.clone(), fs::read(entry.path()).map_err(|e| format!("{path}: {e}"))?);
            }
        }
        Ok(())
    }
    let mut tree = Tree::new();
    walk(dir, "", &mut tree)?;
    Ok(tree)
}

/// The project files at the folder's root: `<Name>.bzfproj`, and `<Name>.bzf` as edition 0.1 named it (JSON, where
/// an archive starts with `PK`).
pub fn project_files(tree: &Tree) -> Vec<&str> {
    let is_project_file = |(name, bytes): &(&String, &Vec<u8>)| {
        !name.contains('/') && (name.ends_with(".bzfproj") || (name.ends_with(".bzf") && bytes.first() == Some(&b'{')))
    };
    tree.iter().filter(is_project_file).map(|(name, _)| name.as_str()).collect()
}

/// The documents under `Daten/` that are JSON, by their paths there.
pub fn documents(tree: &Tree) -> BTreeMap<String, Value> {
    let under = tree.iter().filter_map(|(path, bytes)| Some((path.strip_prefix("Daten/")?, bytes)));
    under.filter_map(|(path, bytes)| Some((path.to_string(), serde_json::from_slice(bytes).ok()?))).collect()
}

/// The inputs of a result or an issue that are no longer the project's documents: changed since, or not there. A
/// result is current while there are none, whatever else in the project changed.
pub fn not_current(record: &Value, documents: &BTreeMap<String, Value>) -> Vec<String> {
    let inputs = record["inputs"].as_object().into_iter().flatten();
    let changed = inputs.filter(|(path, named)| documents.get(*path).map(hash).as_deref() != named.as_str());
    changed.map(|(path, _)| path.clone()).collect()
}

/// Where an issue does not state the result it names: a figure the issue states for a state is the result's figure
/// for it, the result's inputs are among the issue's, and both name the same method, basis and kernel version.
pub fn not_as_calculated(issue: &Value, result: &Value) -> Vec<String> {
    let mut errors = Vec::new();
    let calculated = |state: &Value| result["figures"].as_array().into_iter().flatten().find(|f| &f["state"] == state);
    for issued in issue["figures"].as_array().into_iter().flatten() {
        let Some(figures) = calculated(&issued["state"]) else {
            errors.push(format!("the result has no figures for {}", issued["state"]));
            continue;
        };
        for (key, value) in issued.as_object().into_iter().flatten() {
            if figures.get(key) != Some(value) {
                errors.push(format!("{} {key}: issued {value}, calculated {:?}", issued["state"], figures.get(key)));
            }
        }
    }
    for (path, named) in result["inputs"].as_object().into_iter().flatten() {
        if issue["inputs"].get(path) != Some(named) {
            errors.push(format!("the result was made from another {path}"));
        }
    }
    for key in ["method", "basis", "kernelVersion"] {
        if issue.get(key).is_some() && result.get(key).is_some() && issue[key] != result[key] {
            errors.push(format!("{key}: issued {}, calculated {}", issue[key], result[key]));
        }
    }
    errors
}

/// Where a result of a roadmap is not that roadmap's: the roadmap is one of its inputs, a state it has figures for is
/// the building as it stands, the target or one of the roadmap's steps, and it was calculated by the method the
/// roadmap rests on.
pub fn not_the_roadmaps(result: &Value, documents: &BTreeMap<String, Value>) -> Vec<String> {
    let Some(id) = result.get("roadmap").and_then(Value::as_str) else { return Vec::new() };
    let path = format!("roadmaps/{id}.json");
    if result["inputs"].get(&path).is_none() {
        return vec![format!("{path} is not among its inputs")];
    }
    // A roadmap that is not here (a share without it) cannot be compared; the result is then not current anyway.
    let Some(roadmap) = documents.get(&path) else { return Vec::new() };
    let mut errors = Vec::new();
    let steps: Vec<&Value> = roadmap["steps"].as_array().into_iter().flatten().map(|step| &step["id"]).collect();
    for figures in result["figures"].as_array().into_iter().flatten() {
        let state = &figures["state"];
        if state != "current" && state != "target" && !steps.contains(&state) {
            errors.push(format!("figures for {state}, which is no step of the roadmap"));
        }
    }
    let planned = &roadmap["assumptions"]["method"];
    if planned.is_string() && result.get("method").is_some() && planned != &result["method"] {
        errors.push(format!("calculated by {}, the roadmap rests on {planned}", result["method"]));
    }
    errors
}

/// Calls `f` with every value in `value`, at any depth.
fn each(value: &Value, f: &mut impl FnMut(&Value)) {
    f(value);
    match value {
        Value::Object(map) => map.values().for_each(|item| each(item, f)),
        Value::Array(items) => items.iter().for_each(|item| each(item, f)),
        _ => {}
    }
}

/// The ids of every item in `documents`.
fn ids<'a>(documents: impl Iterator<Item = &'a Value>) -> HashSet<String> {
    let mut ids = HashSet::new();
    for doc in documents {
        each(doc, &mut |value| {
            if let Some(id) = value.get("id").and_then(Value::as_str) {
                ids.insert(id.to_string());
            }
        });
    }
    ids
}

/// The items a planned step adds that a building document already holds. The building is the building as it stands,
/// so what a planned step is still to add is not in it.
pub fn later_state(documents: &BTreeMap<String, Value>) -> Vec<String> {
    let held = ids(documents.iter().filter(|(path, _)| path.starts_with("buildings/")).map(|(_, doc)| doc));
    let mut found = Vec::new();
    for (path, doc) in documents.iter().filter(|(p, _)| p.starts_with("roadmaps/")) {
        // A step is planned where its status is left out.
        let planned = |step: &&Value| step.get("status").is_none_or(|s| s.as_str() == Some("planned"));
        for step in doc["steps"].as_array().into_iter().flatten().filter(planned) {
            let measures = step["measures"].as_array().into_iter().flatten();
            for change in measures.flat_map(|m| m["changes"].as_array().into_iter().flatten()) {
                let added = change["item"]["id"].as_str().filter(|_| change["op"] == "add");
                if let Some(id) = added.filter(|id| held.contains(*id)) {
                    found.push(format!("{path}: a planned step adds {id}, which a building already holds"));
                }
            }
        }
    }
    found
}

/// The ids roadmaps and costs refer to that no document of the project has.
pub fn dangling(documents: &BTreeMap<String, Value>) -> Vec<String> {
    let ids = ids(documents.values());
    let mut found = Vec::new();
    for (path, doc) in documents.iter().filter(|(p, _)| p.starts_with("roadmaps/") || p.starts_with("costs/")) {
        let mut refs = Vec::new();
        for step in doc["steps"].as_array().into_iter().flatten() {
            for measure in step["measures"].as_array().into_iter().flatten() {
                refs.extend(measure["affects"].as_array().into_iter().flatten().cloned());
                for change in measure["changes"].as_array().into_iter().flatten() {
                    refs.extend([&change["target"], &change["into"]].into_iter().filter(|v| v.is_string()).cloned());
                }
            }
        }
        refs.extend([&doc["building"], &doc["roadmap"]].into_iter().filter(|v| v.is_string()).cloned());
        for measure in doc["measures"].as_array().into_iter().flatten() {
            refs.push(measure["measure"].clone());
        }
        for id in refs.iter().filter_map(Value::as_str) {
            if !ids.contains(id) {
                found.push(format!("{path} refers to {id}, which no document has"));
            }
        }
    }
    found
}

/// The result records of a project by their paths, with the signature files beside each by their names.
pub fn results(tree: &Tree) -> Vec<(&str, Vec<(&str, &str)>)> {
    let records = tree.keys().filter(|path| path.starts_with("Ergebnisse/") && path.ends_with("/ergebnis.json"));
    let beside = |record: &str| {
        let folder = record.strip_suffix("ergebnis.json").unwrap_or(record);
        let files = tree.keys().filter_map(move |path| Some((path.as_str(), path.strip_prefix(folder)?)));
        files.filter_map(|(path, file)| Some((signature::name_of(file)?, path))).collect()
    };
    records.map(|record| (record.as_str(), beside(record))).collect()
}

/// What checking a project found: what is wrong with it, and what a reader should know of it.
#[derive(Debug, Default)]
pub struct Report {
    /// What breaks a rule of the format.
    pub errors: Vec<String>,
    /// What is as the format allows and still worth knowing: a result that is no longer current, who signed.
    pub notes: Vec<String>,
    /// How many documents and records were checked against a schema.
    pub checked: usize,
}

/// Checks a project's files against the format: its project file, the documents under `Daten/`, the references
/// between them, the result records with their inputs and signatures, and the issues.
pub fn check(tree: &Tree, schemas: &Schemas) -> Report {
    let mut report = Report::default();
    let parsed =
        |path: &str| serde_json::from_slice::<Value>(&tree[path]).map_err(|e| format!("{path}: not JSON: {e}"));
    // A name twice in one object: the document has no hash every reader agrees on (HASHES.md).
    let twice = |path: &str| -> Vec<String> {
        let names = names_twice(&tree[path]).unwrap_or_default();
        names.iter().map(|name| format!("{path}: the name \"{name}\" twice in one object")).collect()
    };
    let validate = |report: &mut Report, schema: &str, path: &str, doc: &Value| {
        report.checked += 1;
        report.errors.extend(schemas.errors(schema, doc).into_iter().map(|e| format!("{path} as {schema}: {e}")));
    };

    match project_files(tree)[..] {
        [file] => {
            match parsed(file) {
                Ok(doc) => validate(&mut report, "bzf", file, &doc),
                Err(e) => report.errors.push(e),
            }
            if !file.ends_with(".bzfproj") {
                report.notes.push(format!("{file}: the project file has its name of edition 0.1 (now <Name>.bzfproj)"));
            }
        }
        [] => report.errors.push("no project file <Name>.bzfproj at the root".into()),
        _ => report.errors.push("more than one project file at the root".into()),
    }

    let root = tree.contains_key("Daten/project.json").then(|| parsed("Daten/project.json").ok()).flatten();
    for path in tree.keys().filter(|path| path.starts_with("Daten/")) {
        let name = &path["Daten/".len()..];
        match (schema_of(name), parsed(path)) {
            (Some(schema), Ok(doc)) => validate(&mut report, schema, path, &doc),
            (Some(_), Err(e)) => report.errors.push(e),
            // A program's own document is not described here. As a document of the project it is still JSON.
            (None, Err(e)) if path.ends_with(".json") => report.errors.push(e),
            (None, _) => {
                let class = schemas.class(name, root.as_ref());
                let class =
                    class.map_or("no class, so it stays with its organisation".into(), |c| format!("class {c}"));
                report.notes.push(format!("{path}: an app's own document ({class}), not checked against a schema"));
            }
        }
        report.errors.extend(twice(path));
    }
    let documents = documents(tree);
    report.errors.extend(dangling(&documents));
    report.errors.extend(later_state(&documents));

    let mut held = BTreeMap::new();
    for (path, signatures) in results(tree) {
        let doc = match parsed(path) {
            Ok(doc) => doc,
            Err(e) => {
                report.errors.push(e);
                continue;
            }
        };
        validate(&mut report, "result", path, &doc);
        report.errors.extend(twice(path));
        report.errors.extend(not_the_roadmaps(&doc, &documents).into_iter().map(|e| format!("{path}: {e}")));
        held.insert(hash(&doc), doc.clone());
        if doc["inputs"].as_object().is_none_or(|inputs| inputs.is_empty()) {
            report.notes.push(format!("{path}: names no documents, so it is current only at its revision"));
        } else {
            let changed = not_current(&doc, &documents);
            let (gone, changed): (Vec<_>, Vec<_>) = changed.into_iter().partition(|p| !documents.contains_key(p));
            match (changed.is_empty(), gone.is_empty()) {
                (true, true) => report.notes.push(format!("{path}: current")),
                (false, _) => report.notes.push(format!("{path}: stale, changed since: {}", changed.join(", "))),
                (true, false) => report.notes.push(format!("{path}: cannot be told, not here: {}", gone.join(", "))),
            }
        }
        for (name, file) in signatures {
            let text = String::from_utf8_lossy(&tree[file]);
            match signature::verify(&doc, &text) {
                Ok(signer) => {
                    let own =
                        if signer.issuer == signer.subject { ", by a certificate of the signer's own" } else { "" };
                    let late = if signer.in_validity { "" } else { ", at a time outside the certificate's validity" };
                    report.notes.push(format!("{path}: signed ({name}) by {}{own}{late}", signer.subject));
                }
                Err(signature::Failure::Unsupported(algorithm)) => {
                    report.notes.push(format!("{file}: signed with {algorithm}, which this program does not verify"))
                }
                Err(failure) => report.errors.push(format!("{file}: {failure}")),
            }
        }
    }

    let issues = tree.keys().filter(|path| path.starts_with("Versendet/") && path.ends_with("/versand.json"));
    for path in issues.filter(|path| path.matches('/').count() == 2) {
        let doc = match parsed(path) {
            Ok(doc) => doc,
            Err(e) => {
                report.errors.push(e);
                continue;
            }
        };
        validate(&mut report, "versand", path, &doc);
        // An issue states the figures of the result it names. One whose result was replaced since is as it was
        // issued: the store keeps that result, and this program does not read the store.
        match doc.get("result").and_then(Value::as_str).map(|named| held.get(named)) {
            Some(Some(result)) => {
                let differs = not_as_calculated(&doc, result);
                report.errors.extend(differs.into_iter().map(|e| format!("{path}: {e}")));
            }
            Some(None) => {
                let note = "the result it issued has been replaced since (the store keeps it)";
                report.notes.push(format!("{path}: {note}"));
            }
            None => {}
        }
    }
    if tree.keys().any(|path| path.starts_with(STORE)) {
        report.notes.push("the store (.bauzertifikate/) is not checked by this program".into());
    }
    report
}

/// What of a project is a snapshot: everything but the store.
pub fn snapshot(tree: &Tree) -> Tree {
    tree.iter()
        .filter(|(path, _)| !path.starts_with(STORE))
        .map(|(path, bytes)| (path.clone(), bytes.clone()))
        .collect()
}

/// What of a project may leave its organisation (README, "Privacy"): the project file, the documents of the class
/// `building`, the received files `received.json` lists, and the result records with their signatures. No store, no
/// issues, no document about people or money, and no app's document whose class the root does not give.
pub fn share(tree: &Tree, schemas: &Schemas) -> Tree {
    let documents = documents(tree);
    let root = documents.get("project.json");
    let mut names: HashSet<String> = project_files(tree).into_iter().map(String::from).collect();
    for path in tree.keys().filter_map(|path| path.strip_prefix("Daten/")) {
        if schemas.class(path, root).as_deref() == Some("building") {
            names.insert(format!("Daten/{path}"));
        }
    }
    for file in documents.get("received.json").into_iter().flat_map(|doc| doc["files"].as_array()).flatten() {
        if let (Some(folder), Some(name)) = (file["folder"].as_str(), file["name"].as_str()) {
            names.insert(format!("{folder}/{name}"));
        }
    }
    for (record, signatures) in results(tree) {
        names.insert(record.to_string());
        names.extend(signatures.into_iter().map(|(_, file)| file.to_string()));
    }
    tree.iter().filter(|(path, _)| names.contains(*path)).map(|(path, bytes)| (path.clone(), bytes.clone())).collect()
}
