// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Canonical JSON (RFC 8785) and the hash a document is named by (HASHES.md).

use serde::de::{DeserializeSeed, Deserializer, MapAccess, SeqAccess, Visitor};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::HashSet;

/// `value` as canonical JSON: object keys sorted by their UTF-16 code units, no whitespace, numbers as ECMAScript
/// writes them, strings with JSON's shortest escapes.
pub fn canonical(value: &Value) -> String {
    let mut out = String::new();
    write(value, &mut out);
    out
}

fn write(value: &Value, out: &mut String) {
    match value {
        // serde_json reads every number as a u64, an i64 or a double; as a double it is what RFC 8785 serialises.
        Value::Number(n) => out.push_str(&number(n.as_f64().expect("a JSON number is finite"))),
        Value::Array(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write(item, out);
            }
            out.push(']');
        }
        Value::Object(map) => {
            let mut entries: Vec<_> = map.iter().collect();
            entries.sort_by(|a, b| a.0.encode_utf16().cmp(b.0.encode_utf16()));
            out.push('{');
            for (i, (key, item)) in entries.into_iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                string(key, out);
                out.push(':');
                write(item, out);
            }
            out.push('}');
        }
        Value::String(text) => string(text, out),
        Value::Bool(true) => out.push_str("true"),
        Value::Bool(false) => out.push_str("false"),
        Value::Null => out.push_str("null"),
    }
}

/// A string as RFC 8785, 3.2.2.2 writes it: `"` and `\` escaped, the control characters as `\b`, `\t`, `\n`, `\f`,
/// `\r` or `\u00xx` in lower case, and every other character as it is.
fn string(text: &str, out: &mut String) {
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{8}' => out.push_str("\\b"),
            '\t' => out.push_str("\\t"),
            '\n' => out.push_str("\\n"),
            '\u{c}' => out.push_str("\\f"),
            '\r' => out.push_str("\\r"),
            c if c < '\u{20}' => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

/// A finite double as ECMAScript's `Number.prototype.toString` writes it (RFC 8785, 3.2.2.3).
pub fn number(x: f64) -> String {
    ryu_js::Buffer::new().format_finite(x).to_string()
}

/// The lower-case hex SHA-256 of `bytes`.
pub fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().map(|byte| format!("{byte:02x}")).collect()
}

/// The hash a document is named by: the SHA-256 of its canonical JSON.
pub fn hash(doc: &Value) -> String {
    sha256(canonical(doc).as_bytes())
}

/// The names a JSON text has twice in one object, each once and in the order met; an error where the text is not
/// JSON. A document with such a name has no hash every reader agrees on (HASHES.md, "What a document must be"):
/// one reader keeps the first member and another the last.
pub fn names_twice(text: &[u8]) -> Result<Vec<String>, serde_json::Error> {
    let mut found = Vec::new();
    let mut deserializer = serde_json::Deserializer::from_slice(text);
    Twice(&mut found).deserialize(&mut deserializer)?;
    deserializer.end()?;
    Ok(found)
}

/// Walks a JSON value and collects the names met twice in one object.
struct Twice<'a>(&'a mut Vec<String>);

impl<'de> DeserializeSeed<'de> for Twice<'_> {
    type Value = ();

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<(), D::Error> {
        deserializer.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for Twice<'_> {
    type Value = ();

    fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("a JSON value")
    }

    fn visit_bool<E>(self, _: bool) -> Result<(), E> {
        Ok(())
    }

    fn visit_i64<E>(self, _: i64) -> Result<(), E> {
        Ok(())
    }

    fn visit_u64<E>(self, _: u64) -> Result<(), E> {
        Ok(())
    }

    fn visit_f64<E>(self, _: f64) -> Result<(), E> {
        Ok(())
    }

    fn visit_str<E>(self, _: &str) -> Result<(), E> {
        Ok(())
    }

    fn visit_unit<E>(self) -> Result<(), E> {
        Ok(())
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut items: A) -> Result<(), A::Error> {
        while items.next_element_seed(Twice(&mut *self.0))?.is_some() {}
        Ok(())
    }

    fn visit_map<A: MapAccess<'de>>(self, mut members: A) -> Result<(), A::Error> {
        let mut seen = HashSet::new();
        while let Some(name) = members.next_key::<String>()? {
            if !seen.insert(name.clone()) && !self.0.contains(&name) {
                self.0.push(name);
            }
            members.next_value_seed(Twice(&mut *self.0))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::names_twice;

    #[test]
    fn a_name_twice_in_one_object_is_found() {
        let twice = |text: &str| names_twice(text.as_bytes()).unwrap();
        assert!(twice(r#"{"a": 1, "b": {"a": 2}, "c": [{"a": 3}, {"a": 4}], "d": "a"}"#).is_empty());
        assert_eq!(twice(r#"{"a": 1, "b": 2, "a": 3}"#), ["a"]);
        assert_eq!(
            twice(r#"{"x": [{"n": 1, "\u006e": 2}], "y": {"m": null, "m": {"k": 1, "k": 1, "k": 2}}}"#),
            ["n", "m", "k"]
        );
        assert!(twice(r#"[1, 2.5, -3, 1e30, true, null, "\u00e9", {}, []]"#).is_empty());
        // Not JSON: a lone surrogate, something after the value, nothing at all.
        for text in [r#"{"a": "\ud800"}"#, r#"{"a": 1} {"a": 1}"#, "", "{'a': 1}"] {
            assert!(names_twice(text.as_bytes()).is_err(), "{text}");
        }
    }
}
