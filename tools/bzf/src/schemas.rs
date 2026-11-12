// SPDX-License-Identifier: Apache-2.0 OR MIT
//! The schemas of bzf/1, built into the program, so that checking needs neither this repository nor the network.

use jsonschema::{Draft, Registry, Validator};
use serde_json::{Value, json};
use std::collections::BTreeMap;

const BASE: &str = "https://bauzertifikate.de/schema/bzf/1/";

macro_rules! schemas {
    ($($name:literal),* $(,)?) => {
        /// Every schema of bzf/1 by its name, as this edition publishes it.
        pub const SOURCES: &[(&str, &str)] = &[$(($name, include_str!(concat!("../../../schema/bzf/1/", $name, ".json")))),*];
    };
}

schemas!(
    "bzf",
    "revision",
    "received",
    "versand",
    "project",
    "common",
    "building",
    "plan-state",
    "plan-import",
    "roadmap",
    "consumption",
    "costs",
    "party",
    "result",
);

/// The schema of a document under `Daten/` by its path there, as a reader finds it; none for the apps' own
/// documents.
pub fn schema_of(path: &str) -> Option<&'static str> {
    let folders = [
        ("buildings/", "building"),
        ("plans/", "plan-state"),
        ("imports/", "plan-import"),
        ("roadmaps/", "roadmap"),
        ("costs/", "costs"),
        ("consumption/", "consumption"),
        ("parties/", "party"),
    ];
    match path {
        "project.json" => Some("project"),
        "received.json" => Some("received"),
        _ => folders.into_iter().find(|(folder, _)| path.starts_with(folder)).map(|(_, schema)| schema),
    }
}

/// `value` with every `{"$conflict": {"values": [a, …]}}` replaced by `a`: a document as the apps read it.
pub fn settled(value: Value) -> Value {
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

/// The validators, by a schema's name.
pub struct Schemas {
    validators: BTreeMap<&'static str, Validator>,
    classes: BTreeMap<&'static str, String>,
}

impl Default for Schemas {
    fn default() -> Self {
        Self::new()
    }
}

impl Schemas {
    pub fn new() -> Schemas {
        let parsed: Vec<(&str, Value)> = SOURCES
            .iter()
            .map(|(name, text)| (*name, serde_json::from_str(text).expect("a built-in schema is JSON")))
            .collect();
        let mut registry = Registry::new();
        for (name, schema) in &parsed {
            registry = registry.add(format!("{BASE}{name}.json"), schema.clone()).expect("a built-in schema");
        }
        let registry = registry.prepare().expect("the built-in schemas refer to each other only");
        let mut validators = BTreeMap::new();
        let mut classes = BTreeMap::new();
        for (name, schema) in &parsed {
            let validator = jsonschema::options()
                .with_draft(Draft::Draft202012)
                .with_registry(&registry)
                .should_validate_formats(true)
                .build(&json!({ "$ref": format!("{BASE}{name}.json") }))
                .expect("a built-in schema is valid");
            validators.insert(*name, validator);
            if let Some(class) = schema.get("x-bzf-class").and_then(Value::as_str) {
                classes.insert(*name, class.to_string());
            }
        }
        Schemas { validators, classes }
    }

    /// What is wrong with `doc` as a `schema`, once each conflict two writers left in it is settled by its first
    /// value.
    pub fn errors(&self, schema: &str, doc: &Value) -> Vec<String> {
        let doc = settled(doc.clone());
        self.validators[schema].iter_errors(&doc).map(|e| format!("{}: {e}", e.instance_path())).collect()
    }

    /// The class of the document at `path` under `Daten/` ("Privacy"): its schema's, or for an app's own document
    /// the one the root's `classes` gives its folder. None where neither says.
    pub fn class(&self, path: &str, root: Option<&Value>) -> Option<String> {
        if let Some(schema) = schema_of(path) {
            return self.classes.get(schema).cloned();
        }
        let classes = root?.get("classes")?.as_object()?;
        let longest =
            classes.iter().filter(|(prefix, _)| path.starts_with(prefix.as_str())).max_by_key(|(p, _)| p.len());
        longest.and_then(|(_, class)| class.as_str()).map(String::from)
    }
}
