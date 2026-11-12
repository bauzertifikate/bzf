//! The BZF schemas: valid JSON Schema 2020-12, open by design, and checked against positive and negative fixtures.
//! The example projects are always checked. Implementations can additionally provide generated documents through
//! BZF_DOCUMENTS: building/, project/, plan-state/ and plan-import/ contain JSON documents; folders/ contains
//! project folders with their project file (.bzfproj, or .bzf as edition 0.1 named it), a .bauzertifikate store,
//! Daten/received.json and Versendet/ issue records.
//!
//! In the examples, the result records under Ergebnisse/ and the issues are also checked against the documents they
//! name by hash, with canonical JSON (RFC 8785) written here from the RFC's text.

use jsonschema::{Draft, Registry, Validator};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

const BASE: &str = "https://bauzertifikate.de/schema/bzf/1/";

/// The schemas with their status and, for the documents under `Daten/` and the result records, their class; the
/// stable ones first.
const SCHEMAS: [(&str, &str, Option<&str>); 14] = [
    ("bzf", "stable", None),
    ("revision", "stable", None),
    ("received", "stable", Some("building")),
    ("versand", "stable", None),
    ("project", "stable", Some("building")),
    ("common", "stable", None),
    ("building", "draft", Some("building")),
    ("plan-state", "draft", Some("building")),
    ("plan-import", "draft", Some("building")),
    ("roadmap", "draft", Some("building")),
    ("consumption", "draft", Some("building")),
    ("costs", "draft", Some("commercial")),
    ("party", "draft", Some("party")),
    ("result", "draft", Some("building")),
];

/// The schema of a document under `Daten/` by its path, as a reader finds it; none for the apps' own documents.
fn schema_of(path: &str) -> Option<&'static str> {
    let folders =
        [("buildings/", "building"), ("plans/", "plan-state"), ("imports/", "plan-import"), ("roadmaps/", "roadmap")]
            .into_iter()
            .chain([("costs/", "costs"), ("consumption/", "consumption"), ("parties/", "party")]);
    match path {
        "project.json" => Some("project"),
        "received.json" => Some("received"),
        _ => folders.into_iter().find(|(folder, _)| path.starts_with(folder)).map(|(_, schema)| schema),
    }
}

fn dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../schema/bzf/1")
}

fn read(path: &Path) -> Value {
    let bytes = fs::read(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    serde_json::from_slice(&bytes).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// A head or an object of the store, if it is all there: one a sync is still writing is not JSON yet, and the store
/// waits for it as for one that is missing.
fn arrived(path: &Path) -> Option<Value> {
    serde_json::from_slice(&fs::read(path).ok()?).ok()
}

/// Every schema, registered under its `$id`, so references resolve without the network.
fn registry() -> Registry<'static> {
    let mut registry = Registry::new();
    for (name, _, _) in SCHEMAS {
        registry = registry.add(format!("{BASE}{name}.json"), read(&dir().join(format!("{name}.json")))).unwrap();
    }
    registry.prepare().unwrap()
}

/// Validators by what they validate: a schema's name, or `revision#/$defs/head`.
struct Schemas {
    validators: BTreeMap<String, Validator>,
}

impl Schemas {
    fn new() -> Schemas {
        let registry = registry();
        let mut validators = BTreeMap::new();
        let targets = SCHEMAS
            .iter()
            .map(|(name, _, _)| name.to_string())
            .chain(["revision#/$defs/head", "revision#/$defs/device"].map(String::from));
        for target in targets {
            let url = match target.split_once('#') {
                Some((name, pointer)) => format!("{BASE}{name}.json#{pointer}"),
                None => format!("{BASE}{target}.json"),
            };
            let validator = jsonschema::options()
                .with_draft(Draft::Draft202012)
                .with_registry(&registry)
                .should_validate_formats(true)
                .build(&json!({ "$ref": url }))
                .unwrap_or_else(|e| panic!("{target}: {e}"));
            validators.insert(target, validator);
        }
        Schemas { validators }
    }

    /// What is wrong with `doc` as a `target`, once each conflict two writers left in it is settled by its first value
    /// (as the apps read it).
    fn errors(&self, target: &str, doc: &Value) -> Vec<String> {
        let doc = settled(doc.clone());
        self.validators[target].iter_errors(&doc).map(|e| format!("{}: {e}", e.instance_path())).collect()
    }
}

/// `value` with every `{"$conflict": {"values": [a, …]}}` replaced by `a`.
fn settled(value: Value) -> Value {
    match value {
        Value::Object(mut map) => {
            if let Some(Value::Object(conflict)) = map.remove("$conflict") {
                let first = conflict.get("values").and_then(Value::as_array).and_then(|v| v.first().cloned());
                return settled(first.unwrap_or(Value::Null));
            }
            Value::Object(map.into_iter().map(|(k, v)| (k, settled(v))).collect())
        }
        Value::Array(items) => Value::Array(items.into_iter().map(settled).collect()),
        other => other,
    }
}

#[test]
fn the_schemas_are_valid_and_open() {
    for (name, status, class) in SCHEMAS {
        let schema = read(&dir().join(format!("{name}.json")));
        jsonschema::meta::validate(&schema).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(schema["$schema"], "https://json-schema.org/draft/2020-12/schema", "{name}");
        assert_eq!(schema["$id"], format!("{BASE}{name}.json"), "{name}: named by its place");
        assert_eq!(schema["x-bzf-status"], status, "{name}");
        assert_eq!(schema.get("x-bzf-class").and_then(Value::as_str), class, "{name}: where it may go");
        // Open by design: every object that lists properties keeps the ones it doesn't list.
        let mut closed = Vec::new();
        walk(&schema, &mut String::new(), &mut |pointer, s| {
            if s.get("type") == Some(&json!("object"))
                && s.get("properties").is_some()
                && s.get("additionalProperties") != Some(&json!(true))
            {
                closed.push(pointer.to_string());
            }
        });
        assert!(closed.is_empty(), "{name}: objects without additionalProperties: true at {closed:?}");
    }
    let names: Vec<String> =
        fs::read_dir(dir()).unwrap().map(|e| e.unwrap().file_name().into_string().unwrap()).collect();
    assert_eq!(names.len(), SCHEMAS.len(), "no schema left unchecked: {names:?}");
    Schemas::new();
}

/// Calls `f` on every subschema of `schema` with its JSON pointer.
fn walk(schema: &Value, pointer: &mut String, f: &mut impl FnMut(&str, &Value)) {
    match schema {
        Value::Object(map) => {
            f(pointer, schema);
            for (key, value) in map {
                let len = pointer.len();
                pointer.push('/');
                pointer.push_str(key);
                walk(value, pointer, f);
                pointer.truncate(len);
            }
        }
        Value::Array(items) => {
            for (i, item) in items.iter().enumerate() {
                let len = pointer.len();
                pointer.push_str(&format!("/{i}"));
                walk(item, pointer, f);
                pointer.truncate(len);
            }
        }
        _ => {}
    }
}

const ID: &str = "0193a1b2-7c4d-7e5f-8a9b-0c1d2e3f4a5b";
const HASH: &str = "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08";

fn building(storey: Value) -> Value {
    json!({"schema": "1.0", "id": ID, "parts": [{"id": ID, "storeys": [storey]}]})
}

/// A roadmap of one step with one measure making `change`.
fn roadmap(change: Value) -> Value {
    json!({"schema": "1.0", "id": ID, "building": ID, "steps": [{"id": ID, "measures": [{"id": ID, "changes": [change]}]}]})
}

#[test]
fn known_values_are_checked_and_newer_ones_kept() {
    let s = Schemas::new();
    let wall = |extra: Value| {
        let mut wall = json!({"id": ID, "line": [[0, 0], [10000, 0]]});
        wall.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
        building(json!({"id": ID, "level": 0, "walls": [wall]}))
    };
    let space =
        |condition: Value| building(json!({"id": ID, "level": 0, "spaces": [{"id": ID, "condition": condition}]}));
    let valid = [
        ("building", wall(json!({}))),
        // Values and fields a newer writer added.
        (
            "building",
            wall(json!({"phase": "planned", "placement": {"sense": "centre", "reference": "core"}, "fire": "F90"})),
        ),
        ("building", space(json!({"kind": "heated", "zone": "Wohnen", "profile": "Wohnen 2030"}))),
        ("building", space(json!({"kind": "semiHeated", "temperature": 15}))),
        ("building", space(json!({"kind": "unheated", "space": "wintergarten"}))),
        // A conflict is checked once settled.
        ("building", wall(json!({"exterior": {"$conflict": {"base": false, "values": [true, false]}}}))),
        // A document of a newer major version only says which.
        ("building", json!({"schema": "2.0", "walls": "elsewhere"})),
        (
            "project",
            json!({"schema": "1.0", "id": ID, "frame": {"north": 74.29, "crs": "EPSG:25832", "epoch": 2016.0}}),
        ),
        ("bzf", json!({"format": "bzf/1", "project": ID, "created": "2026-11-06T10:12:00+01:00"})),
        ("bzf", json!({"format": "bzf/2", "project": "anything a newer store calls an id"})),
        ("revision", json!({"format": "bzf/1.4", "tree": {"buildings/b.json": HASH}, "parents": [], "signature": {}})),
        ("revision", json!({"format": "bzf/2.0", "objects": {}})),
        // A revision that carried a roadmap step into the building names it.
        ("revision", json!({"format": "bzf/1", "tree": {"buildings/b.json": HASH}, "appliedSteps": [ID]})),
        // The parts a renovation roadmap and an energy performance certificate take, a newer change and system kept.
        (
            "building",
            json!({"schema": "1.1", "id": ID, "systems": [{"id": ID, "kind": "fuelCell", "carrier": "hydrogen"}]}),
        ),
        ("roadmap", roadmap(json!({"op": "split", "target": ID}))),
        ("roadmap", roadmap(json!({"op": "set", "target": ID, "field": "phase", "value": "demolished"}))),
        // A result: figures per state, the documents it was made from, what was assumed; a newer kind of result kept.
        (
            "result",
            json!({"schema": "1.0", "time": "2026-11-06T10:00:00+01:00", "type": "daylight", "figures": [{"state": "current", "energyNeed": 131}],
                "inputs": {"buildings/b.json": HASH}, "choices": [{"id": "partial-heating", "value": false}],
                "assumptions": [{"quantity": "Verschattungsfaktor der Fenster", "value": "0,9", "item": ID}]}),
        ),
        ("result", json!({"schema": "2.0", "figures": "elsewhere"})),
        // An issue names the result it issued, and a head the documents a result file was made from.
        (
            "versand",
            json!({"revision": HASH, "to": "x", "time": "2026-11-06T10:00:00+01:00", "files": [], "result": HASH}),
        ),
        (
            "revision#/$defs/head",
            json!({"device": "d", "app": "aufmass", "results": {"Aufmass/mengen.xlsx": {"sha256": HASH, "revision": HASH, "inputs": {"buildings/b.json": HASH}}}}),
        ),
    ];
    for (target, doc) in valid {
        let errors = s.errors(target, &doc);
        assert!(errors.is_empty(), "{target} {doc}: {errors:?}");
    }
    let invalid = [
        ("building", wall(json!({"line": [[0, 0]]}))),
        ("building", wall(json!({"line": [[0, 0], [1000.5, 0]]}))),
        ("building", wall(json!({"id": "EG-W07"}))),
        ("building", wall(json!({"exterior": "ja"}))),
        ("building", wall(json!({"phase": 3}))),
        ("building", space(json!({"kind": "heated", "zone": 3}))),
        ("building", space(json!({"kind": "unheated"}))),
        ("building", json!({"schema": "1.0"})),
        ("building", json!({"schema": "eins", "id": ID})),
        ("project", json!({"schema": "1.0", "id": ID})),
        ("plan-state", json!({"schema": "1.0", "id": ID, "files": [{"hash": "eg.dwg", "name": "eg.dwg"}]})),
        ("bzf", json!({"format": "bzf/1", "project": ID})),
        ("bzf", json!({"format": "zip", "project": ID})),
        ("revision", json!({"format": "bzf/1", "parents": []})),
        ("revision", json!({"format": "bzf/1", "tree": {"a.json": "x"}})),
        ("revision", json!({"format": "bzf/1", "tree": {"buildings/b.json": HASH}, "appliedSteps": ["Schritt 1"]})),
        ("received", json!({"files": [{"id": ID, "name": "eg.dwg", "folder": "Zeichnungen"}]})),
        ("versand", json!({"revision": HASH, "to": "Familie Müller", "time": "gestern", "files": []})),
        (
            "versand",
            json!({"revision": HASH, "to": "x", "time": "2026-11-06T10:00:00+01:00", "files": [], "figures": [{"class": "D"}]}),
        ),
        ("building", json!({"schema": "1.1", "id": ID, "systems": [{"id": ID, "carrier": "naturalGas"}]})),
        ("building", json!({"schema": "1.1", "id": ID, "systems": [{"id": ID, "kind": "boiler", "coverage": 1.5}]})),
        ("result", json!({"schema": "1.0", "time": "2026-11-06T10:00:00+01:00", "figures": [{"state": "current"}]})),
        (
            "result",
            json!({"schema": "1.0", "time": "2026-11-06T10:00:00+01:00", "figures": [{"energyNeed": 131}], "inputs": {}}),
        ),
        (
            "result",
            json!({"schema": "1.0", "time": "2026-11-06T10:00:00+01:00", "figures": [], "inputs": {"buildings/b.json": "b"}}),
        ),
        (
            "result",
            json!({"schema": "1.0", "time": "2026-11-06T10:00:00+01:00", "figures": [], "inputs": {}, "assumptions": [{"value": "0,9"}]}),
        ),
        (
            "versand",
            json!({"revision": HASH, "to": "x", "time": "2026-11-06T10:00:00+01:00", "files": [], "result": "Bilanz"}),
        ),
        ("roadmap", roadmap(json!({"op": "set", "target": ID, "value": 1}))),
        ("roadmap", roadmap(json!({"op": "add", "into": ID, "list": "systems", "item": {"kind": "heatPump"}}))),
        ("costs", json!({"schema": "1.0", "id": ID, "measures": []})),
        ("party", json!({"schema": "1.0", "id": ID, "consents": [{"id": ID, "to": "Zimmerei"}]})),
        (
            "consumption",
            json!({"schema": "1.0", "id": ID, "building": ID, "periods": [{"from": "2025-01-01", "to": "2025-12-31", "amount": 1}]}),
        ),
    ];
    for (target, doc) in invalid {
        assert!(!s.errors(target, &doc).is_empty(), "{target} {doc} passed");
    }
}

/// The example projects in examples, each a project folder as a reader sees it without the store: its project file,
/// the documents under `Daten/` by their path, and its issues. Every id a roadmap's measures and costs refer to is one
/// the project has, or one an earlier change added, and no building holds what a planned step is still to add.
/// Nothing in an example changed after its results and issues were made, so each names its documents as they lie,
/// by the hash of their canonical JSON; and an issue that names a result states that result's figures.
#[test]
fn the_examples() {
    let s = Schemas::new();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
    let mut errors = Vec::new();
    let mut checked = 0;
    // The projects: the folders with a project file (bzf-epbd/ holds the EPBD proposal's files, tests/epbd.rs; solids/ the expected solids, tests/solids.rs).
    let projects = subfolders(&root).into_iter().filter(|p| !project_files(p).is_empty());
    for project in projects {
        let mut check = |target: &str, path: &Path, doc: &Value| {
            checked += 1;
            errors.extend(s.errors(target, doc).into_iter().map(|e| format!("{} as {target}: {e}", path.display())));
        };
        let mut unmatched = Vec::new();
        let markers = project_files(&project);
        for file in &markers {
            check("bzf", file, &read(file));
        }
        let daten = project.join("Daten");
        let mut documents = BTreeMap::new();
        for file in all_files(&daten) {
            let path = file.strip_prefix(&daten).unwrap().to_string_lossy().replace('\\', "/");
            let schema = schema_of(&path).unwrap_or_else(|| panic!("{}: no schema for {path}", project.display()));
            let doc = read(&file);
            check(schema, &file, &doc);
            documents.insert(path, doc);
        }
        // The result records, by the hash an issue names them by.
        let mut results = BTreeMap::new();
        for file in all_files(&project.join("Ergebnisse")) {
            if file.file_name().is_some_and(|name| name == "ergebnis.json") {
                let doc = read(&file);
                check("result", &file, &doc);
                unmatched
                    .extend(stale(&doc, &documents).into_iter().map(|p| format!("{}: {p} changed", file.display())));
                unmatched
                    .extend(not_the_roadmaps(&doc, &documents).into_iter().map(|e| format!("{}: {e}", file.display())));
                results.insert(hash(&doc), doc);
            }
        }
        for issue in subfolders(&project.join("Versendet")) {
            let file = issue.join("versand.json");
            let doc = read(&file);
            check("versand", &file, &doc);
            unmatched.extend(stale(&doc, &documents).into_iter().map(|p| format!("{}: {p} changed", file.display())));
            if let Some(named) = doc.get("result").and_then(Value::as_str) {
                match results.get(named) {
                    Some(result) => unmatched.extend(
                        not_as_calculated(&doc, result).into_iter().map(|e| format!("{}: {e}", file.display())),
                    ),
                    None => unmatched.push(format!("{}: names a result the project does not hold", file.display())),
                }
            }
        }
        errors.extend(unmatched);
        // The examples are written as this edition says: one project file, under its current name.
        if markers.len() != 1 || markers[0].extension().is_none_or(|e| e != "bzfproj") {
            errors.push(format!("{}: not exactly one project file <Name>.bzfproj", project.display()));
        }
        errors.extend(dangling(&documents).into_iter().map(|e| format!("{}: {e}", project.display())));
        errors.extend(later_state(&documents).into_iter().map(|e| format!("{}: {e}", project.display())));
    }
    assert!(errors.is_empty(), "{} invalid:\n{}", errors.len(), errors.join("\n"));
    assert!(checked > 0, "no examples");
}

/// `value` as canonical JSON (RFC 8785), the form a document's hash is taken of: object keys sorted by their UTF-16
/// code units, no whitespace, numbers as ECMAScript writes them, strings with JSON's shortest escapes.
fn canonical(value: &Value) -> String {
    match value {
        Value::Number(n) => n.as_f64().map_or_else(|| n.to_string(), number),
        Value::Array(items) => format!("[{}]", items.iter().map(canonical).collect::<Vec<_>>().join(",")),
        Value::Object(map) => {
            let mut entries: Vec<_> = map.iter().collect();
            entries.sort_by(|a, b| a.0.encode_utf16().cmp(b.0.encode_utf16()));
            let entries = entries.iter().map(|(k, v)| format!("{}:{}", canonical(&json!(k)), canonical(v)));
            format!("{{{}}}", entries.collect::<Vec<_>>().join(","))
        }
        // serde_json writes null, true, false and strings as RFC 8785 asks: only `"`, `\` and the control characters
        // escaped, the latter as \b, \t, \n, \f, \r or \u00xx in lower case.
        other => other.to_string(),
    }
}

/// A finite double as ECMAScript's `Number.prototype.toString` writes it: the shortest digits that read back as the
/// same double, plain from 1e-6 up to 1e21 and in exponent form outside. (Where two shortest digit strings are equally
/// near, ECMAScript takes the even one and Rust the upper; no number in a building document comes near that.)
fn number(x: f64) -> String {
    if x == 0.0 {
        return "0".into();
    }
    let text = format!("{:e}", x.abs());
    let (mantissa, exponent) = text.split_once('e').unwrap();
    let digits: String = mantissa.chars().filter(|c| *c != '.').collect();
    // The value is 0.digits × 10^n, digits being k long.
    let (k, n) = (digits.len() as i32, exponent.parse::<i32>().unwrap() + 1);
    let body = if k <= n && n <= 21 {
        format!("{digits}{}", "0".repeat((n - k) as usize))
    } else if 0 < n && n <= 21 {
        format!("{}.{}", &digits[..n as usize], &digits[n as usize..])
    } else if -6 < n && n <= 0 {
        format!("0.{}{digits}", "0".repeat(-n as usize))
    } else {
        let point = if k > 1 { "." } else { "" };
        format!("{}{point}{}e{}{}", &digits[..1], &digits[1..], if n > 0 { "+" } else { "" }, n - 1)
    };
    format!("{}{body}", if x < 0.0 { "-" } else { "" })
}

/// The hash a document is named by: the SHA-256 of its canonical JSON.
fn hash(doc: &Value) -> String {
    Sha256::digest(canonical(doc).as_bytes()).iter().map(|byte| format!("{byte:02x}")).collect()
}

/// The inputs of a result or an issue that are no longer the project's documents: changed since, or gone. A result is
/// current while there are none, whatever else in the project changed.
fn stale(record: &Value, documents: &BTreeMap<String, Value>) -> Vec<String> {
    let inputs = record["inputs"].as_object().into_iter().flatten();
    let changed = inputs.filter(|(path, named)| documents.get(*path).map(hash).as_deref() != named.as_str());
    changed.map(|(path, _)| path.clone()).collect()
}

/// Where an issue does not state the result it names: a figure the issue states for a state is the result's figure
/// for it, the result's inputs are among the issue's, and both name the same method, basis and kernel version.
fn not_as_calculated(issue: &Value, result: &Value) -> Vec<String> {
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
fn not_the_roadmaps(result: &Value, documents: &BTreeMap<String, Value>) -> Vec<String> {
    let Some(id) = result.get("roadmap").and_then(Value::as_str) else { return Vec::new() };
    let path = format!("roadmaps/{id}.json");
    let (Some(roadmap), true) = (documents.get(&path), result["inputs"].get(&path).is_some()) else {
        return vec![format!("{path} is not among its inputs")];
    };
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

/// Canonical JSON against the example of RFC 8785 (section 3.2.2 and 3.2.3) and the number forms a document meets.
#[test]
fn canonical_json_is_the_rfcs() {
    let source = r#"{
        "numbers": [333333333.33333329, 1E30, 4.50, 2e-3, 0.000000000000000000000000001],
        "string": "\u20ac$\u000F\u000aA'\u0042\u0022\u005c\\\"\/",
        "literals": [null, true, false]
    }"#;
    let expected = r#"{"literals":[null,true,false],"numbers":[333333333.3333333,1e+30,4.5,0.002,1e-27],"string":"€$\u000f\nA'B\"\\\\\"/"}"#;
    assert_eq!(canonical(&serde_json::from_str(source).unwrap()), expected);
    // Keys in UTF-16 order (RFC 8785, 3.2.3), not in UTF-8 order: U+10000 sorts before U+FB33.
    let keys = json!({"\u{20ac}": 1, "\r": 2, "\u{fb33}": 3, "1": 4, "\u{1f600}": 5, "\u{80}": 6, "\u{f6}": 7});
    assert_eq!(canonical(&keys), "{\"\\r\":2,\"1\":4,\"\u{80}\":6,\"ö\":7,\"€\":1,\"😀\":5,\"\u{fb33}\":3}");
    // A whole number is written without a fraction, however the document wrote it.
    let numbers = json!([6.0, 6, -0.0, 0.035, 1e21, 1e-7, 123456789012345680000.0, -2.5e-6, 5291300.0]);
    assert_eq!(canonical(&numbers), "[6,6,0,0.035,1e+21,1e-7,123456789012345680000,-0.0000025,5291300]");
}

/// A result is current while the documents it was made from are unchanged, whatever else in the project changes.
#[test]
fn a_result_is_stale_once_one_of_its_inputs_changes() {
    let project = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/haus-am-hang");
    let daten = project.join("Daten");
    let path = |file: &Path| file.strip_prefix(&daten).unwrap().to_string_lossy().replace('\\', "/");
    let documents: BTreeMap<String, Value> = all_files(&daten).iter().map(|file| (path(file), read(file))).collect();
    let balance = read(&project.join("Ergebnisse/Energieberatung/Energieausweis/ergebnis.json"));
    let roadmap = read(&project.join("Ergebnisse/Energieberatung/Sanierungsfahrplan/ergebnis.json"));
    let key = |folder: &str| documents.keys().find(|p| p.starts_with(folder)).unwrap().clone();
    assert!(stale(&balance, &documents).is_empty() && stale(&roadmap, &documents).is_empty());

    // A document neither was made from changes: both stay current.
    let mut later = documents.clone();
    later.get_mut(&key("consumption/")).unwrap()["note"] = json!("Zähler getauscht");
    assert!(stale(&balance, &later).is_empty() && stale(&roadmap, &later).is_empty());

    // The roadmap changes: its result is stale, the balance of the building as it stands is not.
    later.get_mut(&key("roadmaps/")).unwrap()["label"] = json!("Fahrplan 2027");
    assert!(stale(&balance, &later).is_empty());
    assert_eq!(stale(&roadmap, &later), [key("roadmaps/")]);

    // The building changes: both are stale and name the building.
    let mut later = documents.clone();
    later.get_mut(&key("buildings/")).unwrap()["airtightness"]["n50"] = json!(4.5);
    assert_eq!(stale(&balance, &later), [key("buildings/")]);
    assert_eq!(stale(&roadmap, &later), [key("buildings/")]);

    // The same document written another way (key order, 6.0 for 6) is the same input.
    let mut later = documents.clone();
    let building = later.get_mut(&key("buildings/")).unwrap();
    let mut keys: Vec<_> = building.as_object().unwrap().clone().into_iter().collect();
    keys.reverse();
    *building = Value::Object(keys.into_iter().collect());
    building["airtightness"]["n50"] = json!(6);
    assert!(stale(&balance, &later).is_empty());

    // A document that is gone makes its results stale.
    later.remove(&key("roadmaps/"));
    assert_eq!(stale(&roadmap, &later), [key("roadmaps/")]);
}

/// The project files directly in `dir`: `<Name>.bzfproj`, and `<Name>.bzf` as edition 0.1 named it. A `.bzf` that does
/// not start as a JSON object is a project in one file (an archive) and none of these.
fn project_files(dir: &Path) -> Vec<PathBuf> {
    let is_project_file = |file: &PathBuf| match file.extension().and_then(|e| e.to_str()) {
        Some("bzfproj") => true,
        Some("bzf") => fs::read(file).is_ok_and(|bytes| bytes.first() == Some(&b'{')),
        _ => false,
    };
    files(dir).into_iter().filter(is_project_file).collect()
}

/// The items a planned step adds that a building document already holds. The building is the building as it stands,
/// so what a planned step is still to add is not in it.
fn later_state(documents: &BTreeMap<String, Value>) -> Vec<String> {
    let mut held = std::collections::HashSet::new();
    for (_, doc) in documents.iter().filter(|(p, _)| p.starts_with("buildings/")) {
        walk(doc, &mut String::new(), &mut |_, v| {
            if let Some(id) = v.get("id").and_then(Value::as_str) {
                held.insert(id.to_string());
            }
        });
    }
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
fn dangling(documents: &BTreeMap<String, Value>) -> Vec<String> {
    let mut ids = std::collections::HashSet::new();
    for doc in documents.values() {
        walk(doc, &mut String::new(), &mut |_, v| {
            if let Some(id) = v.get("id").and_then(Value::as_str) {
                ids.insert(id.to_string());
            }
        });
    }
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

/// Every file under `dir`, at any depth, sorted.
fn all_files(dir: &Path) -> Vec<PathBuf> {
    let mut found = files(dir);
    for sub in subfolders(dir) {
        found.extend(all_files(&sub));
    }
    found.sort();
    found
}

/// Optional implementation-generated documents under `$BZF_DOCUMENTS`: the building model's documents by their schema's name, and the store tests' project folders,
/// whose own files are checked here (the project file, the revisions reachable from the heads, the heads, the devices'
/// records, `received.json`, `versand.json`). The store's tests stand any JSON in for a building, so the building
/// model's documents are checked from the separate document fixture directories.
#[test]
fn the_documents_our_tests_write() {
    let Some(root) = std::env::var_os("BZF_DOCUMENTS").map(PathBuf::from) else {
        eprintln!("BZF_DOCUMENTS is not set: skipping optional implementation-generated fixtures.");
        return;
    };
    let s = Schemas::new();
    let mut checked: BTreeMap<String, usize> = BTreeMap::new();
    let mut errors = Vec::new();
    let mut check = |target: &str, path: &Path, doc: &Value| {
        *checked.entry(target.to_string()).or_default() += 1;
        errors.extend(s.errors(target, doc).into_iter().map(|e| format!("{} as {target}: {e}", path.display())));
    };
    for target in ["building", "project", "plan-state", "plan-import"] {
        for file in files(&root.join(target)) {
            check(target, &file, &read(&file));
        }
    }
    for project in projects(&root.join("folders")) {
        let store = project.join(".bauzertifikate");
        for file in project_files(&project) {
            check("bzf", &file, &read(&file));
        }
        let mut revisions = Vec::new();
        for file in files(&store.join("heads")) {
            let Some(head) = arrived(&file) else { continue };
            revisions.extend(head["revision"].as_str().map(String::from));
            check("revision#/$defs/head", &file, &head);
        }
        for file in files(&store.join("devices")) {
            check("revision#/$defs/device", &file, &read(&file));
        }
        // Where the object `hash` lies; nowhere for a name that isn't a hash.
        let object =
            |hash: &str| store.join("objects").join(hash.get(..2).unwrap_or("")).join(hash.get(2..).unwrap_or(""));
        let mut seen = std::collections::HashSet::new();
        while let Some(hash) = revisions.pop() {
            // A head may name objects a sync has not brought yet.
            let Some(revision) = arrived(&object(&hash)).filter(|_| seen.insert(hash.clone())) else { continue };
            check("revision", &object(&hash), &revision);
            let parents = revision["parents"].as_array().into_iter().flatten().filter_map(Value::as_str);
            revisions.extend(parents.map(String::from));
            if let Some(received) = revision["tree"]["received.json"].as_str().and_then(|h| arrived(&object(h))) {
                check("received", &object(revision["tree"]["received.json"].as_str().unwrap()), &received);
            }
        }
        let daten = project.join("Daten/received.json");
        if daten.is_file() {
            check("received", &daten, &read(&daten));
        }
        for issue in subfolders(&project.join("Versendet")) {
            check("versand", &issue.join("versand.json"), &read(&issue.join("versand.json")));
        }
    }
    eprintln!("checked: {checked:?}");
    assert!(errors.is_empty(), "{} invalid:\n{}", errors.len(), errors.join("\n"));
    for target in ["building", "project", "plan-state", "plan-import", "bzf", "revision", "received", "versand"] {
        assert!(checked.contains_key(target), "no {target} documents in the BZF_DOCUMENTS fixture directory");
    }
}

/// The files directly in `dir`, sorted; none where it doesn't exist.
fn files(dir: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = match fs::read_dir(dir) {
        Ok(entries) => entries.map(|e| e.unwrap().path()).filter(|p| p.is_file()).collect(),
        Err(_) => Vec::new(),
    };
    files.sort();
    files
}

fn subfolders(dir: &Path) -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = match fs::read_dir(dir) {
        Ok(entries) => entries.map(|e| e.unwrap().path()).filter(|p| p.is_dir()).collect(),
        Err(_) => Vec::new(),
    };
    dirs.sort();
    dirs
}

/// The project folders under `dir`, at any depth: those with a store. (A test also puts a file named like a project
/// file beside them that is none, which the store refuses as foreign.)
fn projects(dir: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for sub in subfolders(dir) {
        if sub.join(".bauzertifikate").is_dir() {
            found.push(sub);
        } else {
            found.extend(projects(&sub));
        }
    }
    found
}
