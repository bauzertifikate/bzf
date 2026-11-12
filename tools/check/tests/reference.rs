// SPDX-License-Identifier: Apache-2.0 OR MIT
//! The expected solids (examples/solids/) made again by the reference implementation (tools/solids), within the
//! tolerance each file states (SOLIDS.md, "The expected solids"): the same solids for the same items in the same order,
//! the same items without one under the same rule, every volume within its share and every corner within its distance
//! of one in the file, both ways. This test holds the files to the crate; solids.rs checks them without it.

use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(path: &Path) -> Vec<u8> {
    fs::read(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn points(solid: &Value) -> Vec<[f64; 3]> {
    let rings = solid["faces"].as_array().into_iter().flatten().flat_map(|f| f.as_array().into_iter().flatten());
    let corners = rings.flat_map(|r| r.as_array().into_iter().flatten());
    corners.map(|p| [0, 1, 2].map(|k| p[k].as_f64().unwrap_or(f64::NAN))).collect()
}

/// What tells a solid from the others of its building.
fn key(solid: &Value) -> String {
    ["item", "kind", "element", "layer"].map(|k| solid[k].to_string()).join(" ")
}

/// Where the file `theirs` and the solids `ours` differ beyond the file's tolerance.
fn differences(name: &str, theirs: &Value, ours: &Value) -> Vec<String> {
    let mut out = Vec::new();
    if theirs["building"] != ours["building"] {
        out.push(format!("{name}: another building"));
    }
    let (point, share) = (theirs["tolerance"]["point"].as_f64(), theirs["tolerance"]["volume"].as_f64());
    let (Some(point), Some(share)) = (point, share) else { return vec![format!("{name}: no tolerance")] };
    if theirs["without"] != ours["without"] {
        out.push(format!("{name}: the items without a solid differ"));
    }
    let empty = Vec::new();
    let (theirs, ours) = (theirs["solids"].as_array().unwrap_or(&empty), ours["solids"].as_array().unwrap_or(&empty));
    if theirs.is_empty() {
        out.push(format!("{name}: no solids"));
    }
    if theirs.iter().map(key).ne(ours.iter().map(key)) {
        out.push(format!("{name}: other solids, or in another order"));
        return out;
    }
    for (t, o) in theirs.iter().zip(ours) {
        let what = key(t);
        for field in ["part", "storey", "material", "sheet"] {
            if t[field] != o[field] {
                out.push(format!("{name}: {what}: {field}"));
            }
        }
        if let Some(v) = t["volume"].as_f64() {
            let made = o["volume"].as_f64().unwrap_or(f64::NAN);
            if made.is_nan() || (made - v).abs() > share * v {
                out.push(format!("{name}: {what}: volume {made}, expected {v}"));
            }
        }
        let (a, b) = (points(t), points(o));
        let near = |p: &[f64; 3], set: &[[f64; 3]]| {
            set.iter().any(|q| (0..3).map(|k| (p[k] - q[k]) * (p[k] - q[k])).sum::<f64>().sqrt() <= point)
        };
        if !a.iter().all(|p| near(p, &b)) {
            out.push(format!("{name}: {what}: an expected corner the crate does not make"));
        }
        if !b.iter().all(|p| near(p, &a)) {
            out.push(format!("{name}: {what}: a corner that is not expected"));
        }
    }
    out
}

/// The files of expected solids, by name, each with the solids the crate makes of its building.
fn made() -> Vec<(String, Value, Value)> {
    let dir = root().join("examples/solids");
    let mut paths: Vec<PathBuf> = fs::read_dir(&dir).unwrap().map(|e| e.unwrap().path()).collect();
    paths.retain(|p| p.extension().is_some_and(|e| e == "json"));
    paths.sort();
    assert!(!paths.is_empty(), "no expected solids in {}", dir.display());
    paths
        .iter()
        .map(|p| {
            let name = p.file_stem().unwrap().to_string_lossy().into_owned();
            let theirs: Value = serde_json::from_slice(&read(p)).unwrap_or_else(|e| panic!("{name}: {e}"));
            let path = theirs["building"].as_str().unwrap_or_else(|| panic!("{name}: no building"));
            let building: bzf_solids::Building =
                serde_json::from_slice(&read(&root().join(path))).unwrap_or_else(|e| panic!("{path}: {e}"));
            let ours = serde_json::from_str(&bzf_solids::file(&building, path)).unwrap();
            (name, theirs, ours)
        })
        .collect()
}

#[test]
fn the_reference_implementation_makes_the_expected_solids() {
    for (name, theirs, ours) in made() {
        let found = differences(&name, &theirs, &ours);
        assert!(found.is_empty(), "{}", found.join("\n"));
    }
}

#[test]
fn a_difference_is_found() {
    let (name, theirs, ours) = made().into_iter().next().unwrap();
    let mut moved = ours.clone();
    let corner = &mut moved["solids"][0]["faces"][0][0][0][0];
    *corner = (corner.as_f64().unwrap() + 2.0).into();
    assert!(!differences(&name, &theirs, &moved).is_empty(), "a corner 2 mm off");
    let mut bigger = ours.clone();
    let volume = &mut bigger["solids"][0]["volume"];
    *volume = (volume.as_f64().unwrap() * 1.002).into();
    assert!(!differences(&name, &theirs, &bigger).is_empty(), "a volume 0.2 % off");
    let mut fewer = ours.clone();
    fewer["solids"].as_array_mut().unwrap().pop();
    assert!(!differences(&name, &theirs, &fewer).is_empty(), "a solid left out");
    let mut other = ours;
    other["without"] = Value::Array(Vec::new());
    assert!(!differences(&name, &theirs, &other).is_empty() || theirs["without"].as_array().is_some_and(Vec::is_empty));
}
