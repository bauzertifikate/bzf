// SPDX-License-Identifier: Apache-2.0 OR MIT
//! The rules of SOLIDS.md on small buildings: every solid closed, with the volumes the rules give, at walls' ends,
//! under roofs and around openings.

use bzf_solids::{Building, Kind, Solid, Why, solids};
use serde_json::{Value, json};

/// A building of one part with these storeys and roof plates; the build-ups `w365` (two layers, 300 and 65 mm), `w115`
/// and `slab200`.
fn building(storeys: Value, part: Value) -> Building {
    let mut p = json!({ "id": "part", "storeys": storeys });
    for (k, v) in part.as_object().into_iter().flatten() {
        p[k] = v.clone();
    }
    serde_json::from_value(json!({
        "schema": "bzf/1",
        "id": "building",
        "parts": [p],
        "constructions": [
            { "id": "w365", "name": "Außenwand", "layers": [
                { "material": { "name": "Ziegel" }, "thickness": 0.3 },
                { "material": { "name": "Putz" }, "thickness": 0.065 }
            ] },
            { "id": "w115", "name": "Innenwand", "layers": [{ "material": { "name": "Ziegel" }, "thickness": 0.115 }] },
            { "id": "slab200", "name": "Decke", "layers": [{ "material": { "name": "Beton" }, "thickness": 0.2 }] }
        ]
    }))
    .expect("a building")
}

fn wall(id: &str, line: [[i64; 2]; 2], construction: &str, exterior: bool) -> Value {
    json!({ "id": id, "line": line, "construction": construction, "exterior": exterior })
}

fn storey(walls: Value, openings: Value) -> Value {
    json!([{ "id": "eg", "level": 0, "height": 2800, "walls": walls, "openings": openings }])
}

/// The solids, every one closed and the right way round.
fn all(b: &Building) -> Vec<Solid> {
    let (out, _) = solids(b);
    for s in out.iter().filter(|s| !s.sheet) {
        assert!(s.closed(), "{:?} {:?} layer {:?} is open: {:#?}", s.kind, s.id, s.layer, s.faces);
        assert!(s.volume() > 0.0, "{:?} {:?} is inside out", s.kind, s.id);
        for f in &s.faces {
            let n = bzf_solids::newell(&f.rings[0]);
            let dot: f64 = (0..3).map(|k| n[k] * f.normal[k]).sum();
            assert!(dot > 0.0, "{:?} {:?}: a face's ring runs against its normal", s.kind, s.id);
        }
    }
    out
}

/// The whole wall `id`.
fn whole<'a>(out: &'a [Solid], id: &str) -> &'a Solid {
    out.iter().find(|s| s.kind == Kind::Wall && s.id.0 == id && s.layer.is_none()).expect("the wall")
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1.0 + b.abs() * 1e-9
}

/// The lowest and highest z of a solid.
fn heights(s: &Solid) -> (f64, f64) {
    let zs = || s.faces.iter().flat_map(|f| &f.rings).flatten().map(|p| p[2]);
    (zs().fold(f64::MAX, f64::min), zs().fold(f64::MIN, f64::max))
}

#[test]
fn a_wall_is_a_box_and_its_layers_fill_it() {
    let b = building(storey(json!([wall("w", [[0, 0], [4000, 0]], "w365", true)]), json!([])), json!({}));
    let out = all(&b);
    let w = whole(&out, "w");
    assert_eq!(w.faces.len(), 6);
    assert!(close(w.volume(), 4000.0 * 365.0 * 2800.0));
    let layers: Vec<&Solid> = out.iter().filter(|s| s.layer.is_some()).collect();
    assert_eq!(layers.len(), 2);
    assert_eq!(layers[0].material.as_deref(), Some("Ziegel"));
    // The first layer lies at the line, the second to its left.
    let ys = |s: &Solid| {
        let y = || s.faces.iter().flat_map(|f| &f.rings).flatten().map(|p| p[1]);
        (y().fold(f64::MAX, f64::min), y().fold(f64::MIN, f64::max))
    };
    assert_eq!(ys(layers[0]), (0.0, 300.0));
    assert_eq!(ys(layers[1]), (300.0, 365.0));
    assert!(close(layers.iter().map(|s| s.volume()).sum::<f64>(), w.volume()));
}

#[test]
fn openings_are_cut_out_and_their_solids_fill_the_wall_again() {
    let openings = json!([
        { "id": "window", "host": "w", "offset": 500, "sill": 900, "width": 1200, "height": 1300 },
        { "id": "door", "host": "w", "kind": "door", "offset": 2500, "sill": 0, "width": 1000, "height": 2100 },
        { "id": "top", "host": "w", "offset": 3600, "sill": 2000, "width": 300, "height": 800 },
        { "id": "none", "host": "w", "offset": 100, "sill": 0, "width": 0, "height": 2000 }
    ]);
    let b = building(storey(json!([wall("w", [[0, 0], [4000, 0]], "w365", true)]), openings), json!({}));
    let out = all(&b);
    let w = whole(&out, "w");
    let holes = 1200.0 * 1300.0 + 1000.0 * 2100.0 + 300.0 * 800.0;
    assert!(close(w.volume(), (4000.0 * 2800.0 - holes) * 365.0), "{}", w.volume());
    // Two long faces, two ends, the bottom in two pieces, the top in two, and ten reveals.
    assert_eq!(w.faces.len(), 2 + 2 + 2 + 2 + 4 + 3 + 3);
    let voids: Vec<&Solid> = out.iter().filter(|s| s.kind == Kind::Opening).collect();
    assert_eq!(voids.iter().map(|s| s.id.0.as_str()).collect::<Vec<_>>(), ["window", "door", "top"]);
    assert!(close(voids.iter().map(|s| s.volume()).sum::<f64>(), holes * 365.0));
    // Each layer has the same openings.
    let layers: f64 = out.iter().filter(|s| s.layer.is_some()).map(|s| s.volume()).sum();
    assert!(close(layers, w.volume()));
}

#[test]
fn walls_meeting_at_a_corner_are_mitred_and_fill_it_once() {
    // An L of two exterior walls, the layers to the left of each line.
    let walls =
        json!([wall("a", [[0, 0], [5000, 0]], "w365", true), wall("b", [[5000, 0], [5000, 4000]], "w365", true)]);
    let out = all(&building(storey(walls, json!([])), json!({})));
    let (a, b) = (whole(&out, "a"), whole(&out, "b"));
    // Each a trapezoid in plan: the outer face as long as the line, the inner shorter by the other's thickness.
    assert!(close(a.volume(), (5000.0 + 4635.0) / 2.0 * 365.0 * 2800.0), "{}", a.volume());
    assert!(close(b.volume(), (4000.0 + 3635.0) / 2.0 * 365.0 * 2800.0), "{}", b.volume());
    assert_eq!(a.faces.len(), 6);
}

#[test]
fn a_separation_line_at_a_corner_changes_no_wall() {
    let corner = |more: Value| {
        let mut walls =
            vec![wall("a", [[0, 0], [4000, 0]], "w365", true), wall("b", [[4000, 0], [4000, 3000]], "w365", true)];
        walls.extend(more.as_array().unwrap().iter().cloned());
        all(&building(storey(json!(walls), json!([])), json!({})))
    };
    let line = |id: &str, line: [[i64; 2]; 2]| json!({ "id": id, "line": line, "kind": "separation" });
    let plain = corner(json!([]));
    // A separation line ending in the corner: the two walls are mitred as without it, and it has no solid.
    let with = corner(json!([line("s", [[4000, 0], [2000, 2000]])]));
    assert_eq!(with.len(), plain.len());
    for id in ["a", "b"] {
        assert!(close(whole(&with, id).volume(), whole(&plain, id).volume()), "{id}");
    }
    // One ending at a wall's free end leaves that end square.
    let free = corner(json!([line("s", [[0, 0], [0, 2000]])]));
    assert!(close(whole(&free, "a").volume(), whole(&plain, "a").volume()));
}

#[test]
fn an_inner_wall_ends_at_the_face_of_the_wall_it_runs_into() {
    // The inner wall's line runs on to the exterior wall's line, its outer face.
    let walls = json!([
        wall("outer", [[0, 0], [6000, 0]], "w365", true),
        wall("inner", [[3000, 3000], [3000, 0]], "w115", false)
    ]);
    let out = all(&building(storey(walls, json!([])), json!({})));
    let inner = whole(&out, "inner");
    assert!(close(inner.volume(), (3000.0 - 365.0) * 115.0 * 2800.0), "{}", inner.volume());
    assert!(close(whole(&out, "outer").volume(), 6000.0 * 365.0 * 2800.0));
}

#[test]
fn a_wall_ends_under_the_drawn_roof_across_its_thickness() {
    // A gable roof over 10 by 8 m, its eaves at 2,8 m, its ridge along x at 5,8 m; a storey 6 m high under it.
    let roof = json!({ "roof": { "kind": "gable", "rect": { "x": [0, 10000], "y": [0, 8000] }, "ridge": "x",
        "eaves": 2800, "top": 5800 } });
    let walls = json!([
        wall("eaves", [[0, 0], [10000, 0]], "w365", true),
        wall("gable", [[10000, 0], [10000, 8000]], "w365", true)
    ]);
    let st = json!([{ "id": "eg", "level": 0, "height": 6000, "walls": walls }]);
    let out = all(&building(st, roof));
    // The roof rises 0,75 mm per mm from the eaves.
    let eaves = whole(&out, "eaves");
    assert_eq!(heights(eaves), (0.0, 2800.0 + 0.75 * 365.0));
    // The wall under the eaves is a trapezoid across its thickness, mitred at the gable.
    let across = (2800.0 + 2800.0 + 0.75 * 365.0) / 2.0 * 365.0;
    let mitre = {
        // The wedge the mitre takes: over the triangle 365 by 365 in plan, the height at each point.
        let h = |y: f64| 2800.0 + 0.75 * y;
        // ∫ over y from 0 to 365 of (365 − y) · h(y): the triangle's width at y is what lies beyond the mitre.
        let n = 10_000;
        (0..n).map(|i| (i as f64 + 0.5) * 365.0 / n as f64).map(|y| y * h(y) * 365.0 / n as f64).sum::<f64>()
    };
    assert!(
        (eaves.volume() - (across * 10000.0 - mitre)).abs() < 1e4,
        "{} {}",
        eaves.volume(),
        across * 10000.0 - mitre
    );
    // The gable wall follows both slopes up to the ridge.
    let gable = whole(&out, "gable");
    assert_eq!(heights(gable), (0.0, 5800.0));
    let n = 80_000;
    let area: f64 = (0..n)
        .map(|i| (i as f64 + 0.5) * 8000.0 / n as f64)
        .map(|y| {
            let h = 2800.0 + 0.75 * y.min(8000.0 - y);
            // The wall's width at y: less at its mitred end, whole at its free one.
            let width = y.min(365.0);
            h * width * 8000.0 / n as f64
        })
        .sum();
    assert!((gable.volume() - area).abs() < area * 1e-6, "{} {}", gable.volume(), area);
}

/// A level roof plate 200 mm thick with its upper face at `top`, from x0 to x1 over y from 0 to 8 m.
fn level(id: &str, top: i64, x0: i64, x1: i64, holes: Value) -> Value {
    json!({ "id": id, "plate": {
        "plane": { "points": [[x0, 0, top], [x1, 0, top], [x0, 8000, top]] },
        "outline": [[0, 0], [x1 - x0, 0], [x1 - x0, 8000], [0, 8000]],
        "holes": holes,
        "thickness": 200
    } })
}

#[test]
fn a_wall_ends_under_a_roof_plate_and_one_standing_on_it_does_not() {
    // A flat roof with its top at 3,0 m; under it a wall with a top of its own at 3,6 m, on it an Attika.
    let mut under = wall("under", [[0, 4000], [10000, 4000]], "w365", false);
    under["top"] = json!(3600);
    let mut attika = wall("attika", [[0, 2000], [10000, 2000]], "w365", false);
    (attika["bottom"], attika["top"]) = (json!(3000), json!(3600));
    let b = building(
        storey(json!([under, attika]), json!([])),
        json!({ "roofs": [level("roof", 3000, 0, 10000, json!([]))] }),
    );
    let out = all(&b);
    assert_eq!(heights(whole(&out, "under")), (0.0, 2800.0));
    assert_eq!(heights(whole(&out, "attika")), (3000.0, 3600.0));
    let roof = out.iter().find(|s| s.kind == Kind::Roof).expect("the roof plate");
    assert!(close(roof.volume(), 10000.0 * 8000.0 * 200.0));
    assert_eq!(roof.storey, None);
}

#[test]
fn a_wall_under_a_hole_in_the_roof_goes_through_it() {
    // A dormer's opening in the plate from x = 2 to 4 m: the wall, 4 m high of its own, ends at the plate's underside
    // (2,8 m) but for those 2 m.
    let hole = json!([[[2000, 3000], [2000, 5000], [4000, 5000], [4000, 3000]]]);
    let mut w = wall("w", [[0, 4000], [10000, 4000]], "w115", false);
    w["top"] = json!(4000);
    let b = building(storey(json!([w]), json!([])), json!({ "roofs": [level("roof", 3000, 0, 10000, hole)] }));
    let out = all(&b);
    assert!(close(whole(&out, "w").volume(), (10000.0 * 2800.0 + 2000.0 * 1200.0) * 115.0));
    let roof = out.iter().find(|s| s.kind == Kind::Roof).expect("the roof plate");
    assert!(close(roof.volume(), (10000.0 * 8000.0 - 2000.0 * 2000.0) * 200.0));
}

#[test]
fn a_storey_set_back_on_a_roof_terrace_ends_under_the_roof_above_it() {
    // The terrace's plate up to 3,0 m over x from 0 to 4 m, the main roof's up to 6,0 m over everything. A wall of the
    // upper storey (floor at 3,0 m) over both: under the main roof's underside all along.
    let roofs = json!([level("terrace", 3000, 0, 4000, json!([])), level("main", 6000, 0, 10000, json!([]))]);
    let st = json!([{ "id": "og", "level": 3000, "height": 3000,
        "walls": [wall("w", [[0, 4000], [10000, 4000]], "w115", false)] }]);
    let out = all(&building(st, json!({ "roofs": roofs })));
    assert_eq!(heights(whole(&out, "w")), (3000.0, 5800.0));
    assert!(close(whole(&out, "w").volume(), 10000.0 * 2800.0 * 115.0));
}

#[test]
fn a_window_under_a_roof_slope_is_open_to_the_roof() {
    // A mono-pitch roof from 2,0 m at x = 0 up to 4,0 m at x = 10 m; a window whose top the roof cuts.
    let roof = json!({ "roofs": [{ "id": "roof", "plate": {
        "plane": { "points": [[0, 0, 2000], [0, 8000, 2000], [10000, 0, 4000]] },
        "outline": [[0, 0], [8000, 0], [8000, 10200], [0, 10200]],
        "thickness": 0
    } }] });
    let openings = json!([{ "id": "window", "host": "w", "offset": 500, "sill": 1000, "width": 2000, "height": 1300 }]);
    let st = json!([{ "id": "eg", "level": 0, "height": 5000,
        "walls": [wall("w", [[0, 4000], [10000, 4000]], "w115", false)], "openings": openings }]);
    let out = all(&building(st, roof));
    // The wall without the window, under the roof: a trapezoid from 2,0 to 4,0 m.
    let full = 10000.0 * 3000.0 * 115.0;
    let window = out.iter().find(|s| s.kind == Kind::Opening).expect("the window");
    assert!(
        close(whole(&out, "w").volume() + window.volume(), full),
        "{} {}",
        whole(&out, "w").volume(),
        window.volume()
    );
    // The window from 1,0 m up to 2,3 m, less what the roof cuts off between x = 0,5 and 1,5 m (2,1 to 2,3 m).
    assert!(close(window.volume(), (2000.0 * 1300.0 - 1000.0 * 200.0 / 2.0) * 115.0), "{}", window.volume());
    // The plate without thickness is a surface.
    assert!(out.iter().any(|s| s.kind == Kind::Roof && s.sheet));
}

#[test]
fn slabs_elements_and_what_has_no_solid() {
    let element = json!({ "id": "e1", "kind": "wall", "of": "plated", "layer": 0, "label": "BSP 100", "plate": {
        "plane": { "points": [[0, 6000, 0], [4000, 6000, 0], [0, 6000, 2800]] },
        "outline": [[0, 0], [4000, 0], [4000, 2800], [0, 2800]],
        "thickness": 100
    } });
    let st = json!([{ "id": "eg", "level": 0, "height": 2800,
        "walls": [
            { "id": "plated", "line": [[0, 6000], [4000, 6000]], "elements": ["e1"] },
            { "id": "bare", "line": [[0, 7000], [4000, 7000]] },
            { "id": "line", "line": [[0, 3000], [4000, 3000]], "kind": "separation" }
        ],
        "slabs": [
            { "id": "slab", "outline": [[0, 0], [6000, 0], [6000, 5000], [0, 5000]],
              "holes": [[[1000, 1000], [1000, 2000], [3000, 2000], [3000, 1000]]], "top": 0, "construction": "slab200" },
            { "id": "unbuilt", "outline": [[0, 0], [6000, 0], [6000, 5000]], "top": 0 }
        ],
        "elements": [element] }]);
    let b = building(st, json!({}));
    let (_, left) = solids(&b);
    assert_eq!(
        left.iter().map(|l| (l.id.0.as_str(), l.why)).collect::<Vec<_>>(),
        [("bare", Why::NoBody), ("unbuilt", Why::NoBuildUp)]
    );
    let out = all(&b);
    assert_eq!(out.len(), 2);
    let slab = out.iter().find(|s| s.kind == Kind::Slab).expect("the slab");
    assert!(close(slab.volume(), (6000.0 * 5000.0 - 2000.0 * 1000.0) * 200.0));
    assert_eq!(heights(slab), (-200.0, 0.0));
    let plate = out.iter().find(|s| s.element.is_some()).expect("the element's plate");
    assert_eq!(
        (plate.kind, plate.id.0.as_str(), plate.layer, plate.material.as_deref()),
        (Kind::Wall, "plated", Some(0), Some("BSP 100"))
    );
    assert!(close(plate.volume(), 4000.0 * 2800.0 * 100.0));
}
