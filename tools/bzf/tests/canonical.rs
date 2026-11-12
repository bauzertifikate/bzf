// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Canonical JSON and the hash a document is named by (HASHES.md), against the public vectors in
//! `vectors/canonical.json`. The vectors were not made by this program.

use bzf::canonical::{canonical, hash, number, sha256};
use serde_json::{Value, json};

fn vectors() -> Value {
    serde_json::from_str(include_str!("../../../vectors/canonical.json")).unwrap()
}

#[test]
fn numbers_are_written_as_ecmascript_writes_them() {
    let vectors = vectors();
    let numbers = vectors["numbers"].as_array().unwrap();
    assert!(numbers.len() >= 24);
    for pair in numbers {
        let (bits, expected) = (pair[0].as_str().unwrap(), pair[1].as_str().unwrap());
        let double = f64::from_bits(u64::from_str_radix(bits, 16).unwrap());
        assert_eq!(number(double), expected, "{bits}");
        // And as a document's number: read from its canonical text, it is written the same.
        let read: Value = serde_json::from_str(expected).unwrap();
        assert_eq!(canonical(&read), expected, "{bits} read back");
    }
}

#[test]
fn documents_are_written_and_hashed_as_the_vectors_say() {
    let vectors = vectors();
    let documents = vectors["documents"].as_array().unwrap();
    assert!(documents.len() >= 5);
    for vector in documents {
        let about = vector["about"].as_str().unwrap();
        let doc: Value = serde_json::from_str(vector["json"].as_str().unwrap()).unwrap();
        assert_eq!(canonical(&doc), vector["canonical"].as_str().unwrap(), "{about}");
        assert_eq!(hash(&doc), vector["sha256"].as_str().unwrap(), "{about}");
        // The canonical form is its own canonical form.
        let again: Value = serde_json::from_str(&canonical(&doc)).unwrap();
        assert_eq!(canonical(&again), canonical(&doc), "{about}");
    }
}

#[test]
fn a_hash_is_sha_256_in_lower_case_hexadecimal() {
    // FIPS 180-4's own example.
    assert_eq!(sha256(b"abc"), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    assert_eq!(hash(&json!({})), sha256(b"{}"));
}
