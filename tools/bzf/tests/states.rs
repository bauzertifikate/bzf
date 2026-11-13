// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Which states of a roadmap a reader can derive, against the public vectors in `vectors/states.json`, and what a
//! reader of edition 0.3 can tell of the example's roadmap, whose hydraulic balance is a change of the op `app`.

use bzf::project::{BUILDING_OPS, states};
use bzf::schemas::Schemas;
use serde_json::Value;
use std::path::Path;

fn vectors() -> Value {
    serde_json::from_str(include_str!("../../../vectors/states.json")).unwrap()
}

fn strings(value: &Value) -> Vec<&str> {
    value.as_array().unwrap().iter().map(|v| v.as_str().unwrap()).collect()
}

#[test]
fn states_are_known_as_the_vectors_say() {
    let schemas = Schemas::new();
    let cases = vectors()["cases"].as_array().unwrap().clone();
    assert!(cases.len() >= 7);
    for case in &cases {
        let about = case["about"].as_str().unwrap();
        // Each roadmap is one a reader may meet: valid as the schema has it.
        let errors = schemas.errors("roadmap", &case["roadmap"]);
        assert!(errors.is_empty(), "{about}: {errors:?}");
        let found = states(&case["roadmap"], &strings(&case["ops"]), &strings(&case["apps"]));
        let expected: Vec<(String, bool)> = case["states"]
            .as_array()
            .unwrap()
            .iter()
            .map(|pair| (pair[0].as_str().unwrap().to_string(), pair[1] == "known"))
            .collect();
        assert_eq!(found, expected, "{about}");
    }
}

/// The example's first planned step balances the heating hydraulically, a change only the Energieberatung's own
/// document model knows. A reader of edition 0.3 (or one that does not know that app) can tell the building as it
/// stands and nothing after it; a reader that knows the app, every state.
#[test]
fn a_reader_that_does_not_know_the_app_cannot_tell_the_example_s_later_states() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/haus-am-hang/Daten/roadmaps/019a5e00-0000-7000-8000-00000000002c.json");
    let roadmap: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let ids = [
        "019a5e00-0000-7000-8000-000000000021",
        "019a5e00-0000-7000-8000-000000000022",
        "019a5e00-0000-7000-8000-000000000023",
    ];
    let named = |known: [bool; 5]| -> Vec<(String, bool)> {
        let names = ["current", ids[0], ids[1], ids[2], "target"];
        names.iter().zip(known).map(|(name, known)| (name.to_string(), known)).collect()
    };
    assert_eq!(states(&roadmap, &BUILDING_OPS, &[]), named([true, false, false, false, false]));
    assert_eq!(states(&roadmap, &BUILDING_OPS, &["energieberatung"]), named([true; 5]));
}
