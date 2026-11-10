//! The EPBD proposal's schema (schema/bzf-epbd/0.1/dataset.json): valid JSON Schema 2020-12, at its place, open
//! by design; its example files (examples/bzf-epbd/) valid against it, and
//! every field they hold declared in it, so the schema says all the crate writes; and a few files it must refuse.

use jsonschema::{Draft, Validator};
use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};

const ID: &str = "https://bauzertifikate.de/schema/bzf-epbd/0.1/dataset.json";

fn spec() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(path: &Path) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))).unwrap()
}

fn schema() -> Value {
    read(&spec().join("schema/bzf-epbd/0.1/dataset.json"))
}

fn validator() -> Validator {
    jsonschema::options().with_draft(Draft::Draft202012).should_validate_formats(true).build(&schema()).unwrap()
}

fn errors(v: &Validator, doc: &Value) -> Vec<String> {
    v.iter_errors(doc).map(|e| format!("{}: {e}", e.instance_path())).collect()
}

fn examples() -> Vec<(String, Value)> {
    let dir = spec().join("examples/bzf-epbd");
    let mut out: Vec<(String, Value)> = (fs::read_dir(&dir).unwrap().map(|e| e.unwrap().path()))
        .filter(|p| p.extension().is_some_and(|e| e == "json"))
        .map(|p| (p.file_name().unwrap().to_string_lossy().into_owned(), read(&p)))
        .collect();
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

#[test]
fn the_schema_is_valid_and_open() {
    let s = schema();
    jsonschema::meta::validate(&s).unwrap();
    assert_eq!(s["$schema"], "https://json-schema.org/draft/2020-12/schema");
    assert_eq!(s["$id"], ID, "named by its place");
    let mut closed = Vec::new();
    walk(&s, &mut String::new(), &mut |pointer, v| {
        if v.get("type") == Some(&json!("object"))
            && v.get("properties").is_some()
            && v.get("additionalProperties") != Some(&json!(true))
        {
            closed.push(pointer.to_string());
        }
    });
    assert!(closed.is_empty(), "objects without additionalProperties: true at {closed:?}");
}

#[test]
fn the_examples_are_valid() {
    let v = validator();
    let examples = examples();
    assert_eq!(examples.iter().map(|e| e.0.as_str()).collect::<Vec<_>>(), ["every-field.json", "haus-am-hang.json"]);
    for (name, doc) in &examples {
        let e = errors(&v, doc);
        assert!(e.is_empty(), "{name}:\n{}", e.join("\n"));
    }
}

/// Every field an example holds is one the schema lists (open objects would take any): the schema says everything
/// the crate writes, and a field added to the crate without the schema fails here.
#[test]
fn every_field_of_the_examples_is_declared() {
    let s = schema();
    let mut undeclared = Vec::new();
    for (name, doc) in examples() {
        declared(&s, &s, doc.clone(), &format!("{name}:"), &mut undeclared);
    }
    assert!(undeclared.is_empty(), "fields the schema doesn't list:\n{}", undeclared.join("\n"));

    // The walk finds a field the schema lacks, in a listed object, in an array's item, in a map's value and in an
    // object made of `allOf` branches (a certificate is an issue and more).
    let probe = json!({
        "format": "bzf-epbd/0.1",
        "building": { "roofColour": "red" },
        "systems": [{ "kind": "boiler", "noise": 40 }],
        "certificate": { "issued": "x", "stamp": "y", "figures": { "uses": { "heating": { "hours": 1 } } } }
    });
    let mut found = Vec::new();
    declared(&s, &s, probe, "probe:", &mut found);
    assert_eq!(
        found,
        [
            "probe:/building/roofColour",
            "probe:/certificate/figures/uses/heating/hours",
            "probe:/certificate/stamp",
            "probe:/systems/0/noise"
        ]
    );
}

/// Walks `doc` with the schema `at` (refs resolved in `root`, `allOf` branches joined) and collects the paths of
/// fields no `properties` lists.
fn declared(root: &Value, at: &Value, doc: Value, path: &str, out: &mut Vec<String>) {
    let parts = joined(root, at);
    match doc {
        Value::Object(map) => {
            for (k, v) in map {
                let p = format!("{path}/{k}");
                let listed = parts.iter().find_map(|s| s.get("properties").and_then(|ps| ps.get(&k)));
                let map_value = parts.iter().find_map(|s| s.get("additionalProperties").filter(|a| a.is_object()));
                match (listed, map_value) {
                    (Some(sub), _) => declared(root, sub, v, &p, out),
                    (None, Some(map_value)) => declared(root, map_value, v, &p, out),
                    (None, None) => out.push(p),
                }
            }
        }
        Value::Array(items) => {
            if let Some(item) = parts.iter().find_map(|s| s.get("items")) {
                for (i, v) in items.into_iter().enumerate() {
                    declared(root, item, v, &format!("{path}/{i}"), out);
                }
            }
        }
        _ => {}
    }
}

/// A subschema and every `allOf` branch of it, refs followed: together they say which fields an object has.
fn joined<'a>(root: &'a Value, at: &'a Value) -> Vec<&'a Value> {
    let at = resolve(root, at);
    let mut out = vec![at];
    for branch in at.get("allOf").and_then(Value::as_array).into_iter().flatten() {
        out.extend(joined(root, branch));
    }
    out
}

/// A subschema with its `$ref` into the same file followed.
fn resolve<'a>(root: &'a Value, at: &'a Value) -> &'a Value {
    match at.get("$ref").and_then(Value::as_str).and_then(|r| r.strip_prefix('#')) {
        Some(pointer) => resolve(root, root.pointer(pointer).unwrap_or_else(|| panic!("dangling $ref #{pointer}"))),
        None => at,
    }
}

#[test]
fn what_it_refuses() {
    let v = validator();
    let base = json!({ "format": "bzf-epbd/0.1", "language": "de", "currency": "EUR", "building": { "address": { "city": "Musterstadt" } } });
    assert!(errors(&v, &base).is_empty());
    for (why, change) in [
        ("another format", json!({ "format": "bzf-epbd/9" })),
        ("another major version", json!({ "format": "bzf-epbd/1.0" })),
        ("a writing time without its offset", json!({ "written": { "time": "2026-11-08T18:00:00" } })),
        ("no building", json!({ "building": null })),
        ("no language", json!({ "language": null })),
        ("no currency", json!({ "currency": null })),
        ("a currency in words", json!({ "currency": "Euro" })),
        ("a building a register can't find", json!({ "building": { "year": 1968 } })),
        ("a building with an empty list of ids", json!({ "building": { "identifiers": [] } })),
        ("a country in lower case", json!({ "building": { "address": { "country": "de" } } })),
        ("smart readiness in per cent", json!({ "smartReadiness": { "issued": "2026-11-07", "value": 42 } })),
        (
            "a lifetime in fractions of a year",
            json!({ "passport": { "steps": [{ "label": "x", "measures": [{ "label": "y", "lifetime": 12.5 }] }] } }),
        ),
        (
            "a share over 1",
            json!({ "certificate": { "issued": "2026-10-20T15:00:00+02:00", "figures": { "selfConsumption": 1.5 } } }),
        ),
        ("a certificate without figures", json!({ "certificate": { "issued": "2026-10-20T15:00:00+02:00" } })),
        ("an envelope without its volume", json!({ "envelope": { "netFloorArea": 1, "area": 1 } })),
        (
            "a period without a date",
            json!({ "consumption": [{ "carrier": "naturalGas", "from": "Januar", "to": "2025-12-31", "amount": 1, "unit": "kWh" }] }),
        ),
        ("an identifier without a value", json!({ "building": { "identifiers": [{ "scheme": "de:alkis" }] } })),
        (
            "a registration without its number",
            json!({ "certificate": { "issued": "2026-10-20T15:00:00+02:00", "figures": {}, "registrations": [{ "scheme": "de:dibt" }] } }),
        ),
    ] {
        let mut doc = base.clone();
        for (k, val) in change.as_object().unwrap() {
            if val.is_null() {
                doc.as_object_mut().unwrap().remove(k);
            } else {
                doc[k] = val.clone();
            }
        }
        assert!(!errors(&v, &doc).is_empty(), "took {why}: {doc}");
    }
    // A newer value of a listed one is kept, as in bzf; an assessment known only by its day is issued on a date; a
    // newer minor version is valid here, its additions in open objects.
    let mut newer = base.clone();
    newer["format"] = json!("bzf-epbd/0.7");
    newer["building"]["roofColour"] = json!("red");
    newer["building"]["category"] = json!("warehouse");
    newer["smartReadiness"] = json!({ "issued": "2026-11-07", "value": 0.42 });
    assert!(errors(&v, &newer).is_empty(), "{:?}", errors(&v, &newer));
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
