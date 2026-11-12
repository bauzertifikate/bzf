//! The expected solids of the example buildings (examples/solids/, the rules in SOLIDS.md), checked without the
//! builder that made them. Each file is well-formed and refers to items of its building; every wall, slab, roof plate
//! and opening of the building has a solid or is listed without one under a rule that fits it; every solid is closed,
//! with plane faces whose normals point out of it, and holds the volume it states; every solid lies where the
//! building's facts put it (a wall between its faces, its bottom and its top, a slab under its top, a roof plate under
//! its plane); and each whole solid holds the volume worked out by hand from the building's figures.

use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

type P2 = [f64; 2];
type P3 = [f64; 3];
type Check = Result<(), String>;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(path: &Path) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap_or_else(|e| panic!("{}: {e}", path.display())))
        .unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// The files of expected solids, by name.
fn files() -> Vec<(String, Value)> {
    let dir = root().join("examples/solids");
    let mut paths: Vec<PathBuf> = fs::read_dir(&dir).unwrap().map(|e| e.unwrap().path()).collect();
    paths.retain(|p| p.extension().is_some_and(|e| e == "json"));
    paths.sort();
    assert!(!paths.is_empty(), "no expected solids in {}", dir.display());
    paths.iter().map(|p| (p.file_stem().unwrap().to_string_lossy().into_owned(), read(p))).collect()
}

fn sub(a: P3, b: P3) -> P3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn cross(a: P3, b: P3) -> P3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

fn dot(a: P3, b: P3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn unit(a: P3) -> P3 {
    let l = dot(a, a).sqrt();
    [a[0] / l, a[1] / l, a[2] / l]
}

/// A ring's area as a vector along its right-handed normal, seen from `about`.
fn area(ring: &[P3], about: P3) -> P3 {
    let mut sum = [0.0; 3];
    for (i, p) in ring.iter().enumerate() {
        let c = cross(sub(*p, about), sub(ring[(i + 1) % ring.len()], about));
        sum = [sum[0] + c[0] / 2.0, sum[1] + c[1] / 2.0, sum[2] + c[2] / 2.0];
    }
    sum
}

/// An outline's area in plan, positive where it runs counter-clockwise.
fn plan_area(ring: &[P2]) -> f64 {
    (0..ring.len()).map(|i| (ring[i], ring[(i + 1) % ring.len()])).map(|(a, b)| a[0] * b[1] - b[0] * a[1]).sum::<f64>()
        / 2.0
}

/// What a face's rings enclose, as seen from `about`: a third of its area times its distance.
fn share_of(face: &[Vec<P3>], about: P3) -> f64 {
    face.iter().map(|ring| dot(area(ring, about), sub(ring[0], about)) / 3.0).sum()
}

/// A corner as the thousandths of a millimetre the files are written to.
fn key(p: P3) -> [i64; 3] {
    p.map(|c| (c * 1000.0).round() as i64)
}

/// A solid's faces, each its rings of corners; the shape is part of the check.
fn faces(solid: &Value) -> Result<Vec<Vec<Vec<P3>>>, String> {
    let list = |v: &Value, what: &str| v.as_array().cloned().ok_or(format!("{what} is no list"));
    let mut out = Vec::new();
    for face in list(&solid["faces"], "faces")? {
        let mut rings = Vec::new();
        for ring in list(&face, "a face")? {
            let mut points = Vec::new();
            for p in list(&ring, "a ring")? {
                let c: Vec<f64> = list(&p, "a corner")?.iter().filter_map(Value::as_f64).collect();
                points.push(<P3>::try_from(c).map_err(|_| "a corner is not three numbers".to_string())?);
            }
            if points.len() < 3 {
                return Err("a ring of fewer than three corners".into());
            }
            rings.push(points);
        }
        if rings.is_empty() {
            return Err("a face without a ring".into());
        }
        out.push(rings);
    }
    if out.is_empty() {
        return Err("no faces".into());
    }
    Ok(out)
}

/// Every face plane, its first ring running counter-clockwise seen along its normal from outside and its holes the
/// other way.
fn plane(faces: &[Vec<Vec<P3>>]) -> Check {
    for (k, face) in faces.iter().enumerate() {
        let about = face[0][0];
        let outer = area(&face[0], about);
        if dot(outer, outer).sqrt() < 1.0 {
            return Err(format!("face {k} has no area"));
        }
        let n = unit(outer);
        if face.iter().flatten().any(|p| dot(sub(*p, about), n).abs() > 0.01) {
            return Err(format!("face {k} is not plane"));
        }
        if face[1..].iter().any(|hole| dot(area(hole, about), n) >= 0.0) {
            return Err(format!("face {k} has a hole that runs as its outline does"));
        }
    }
    Ok(())
}

/// Closed: every edge is walked once by one face and once, the other way, by another, so two faces meet at every edge
/// and all of them face the same side; and that side is the outside, each connected piece enclosing a volume above
/// zero. Gives the volume.
fn closed(faces: &[Vec<Vec<P3>>]) -> Result<f64, String> {
    let mut edges: BTreeMap<([i64; 3], [i64; 3]), Vec<usize>> = BTreeMap::new();
    for (k, face) in faces.iter().enumerate() {
        for ring in face {
            for (i, p) in ring.iter().enumerate() {
                let (a, b) = (key(*p), key(ring[(i + 1) % ring.len()]));
                if a == b {
                    return Err(format!("face {k} has an edge without length"));
                }
                edges.entry((a, b)).or_default().push(k);
            }
        }
    }
    // Faces that share an edge are one piece.
    let mut piece: Vec<usize> = (0..faces.len()).collect();
    fn find(piece: &mut [usize], k: usize) -> usize {
        if piece[k] != k {
            piece[k] = find(piece, piece[k]);
        }
        piece[k]
    }
    for ((a, b), at) in &edges {
        let back = edges.get(&(*b, *a)).map_or(0, Vec::len);
        if at.len() != 1 || back != 1 {
            return Err(format!("the edge {a:?} to {b:?} is walked {} times and {back} times back", at.len()));
        }
        let (x, y) = (find(&mut piece, at[0]), find(&mut piece, edges[&(*b, *a)][0]));
        piece[x] = y;
    }
    let about = faces[0][0][0];
    let mut volumes: BTreeMap<usize, f64> = BTreeMap::new();
    for (k, face) in faces.iter().enumerate() {
        *volumes.entry(find(&mut piece, k)).or_default() += share_of(face, about);
    }
    if volumes.values().any(|v| *v <= 0.0) {
        return Err("a piece's faces look inwards".into());
    }
    Ok(volumes.values().sum())
}

/// A building's facts as far as the solids' rules read them.
struct Building<'a> {
    doc: &'a Value,
}

/// A wall, slab, roof plate or opening where it stands.
#[derive(Clone, Copy)]
struct Item<'a> {
    kind: &'static str,
    part: &'a Value,
    /// The storey, its level and its top, mm; none for a roof plate.
    storey: Option<(&'a Value, f64, f64)>,
    item: &'a Value,
}

fn list<'a>(v: &'a Value, key: &str) -> impl Iterator<Item = &'a Value> {
    v[key].as_array().into_iter().flatten()
}

fn mm(v: &Value) -> f64 {
    v.as_f64().unwrap_or(0.0)
}

fn point(v: &Value) -> P2 {
    [mm(&v[0]), mm(&v[1])]
}

impl<'a> Building<'a> {
    /// Every wall, slab, opening and roof plate, in the document's order.
    fn items(&self) -> Vec<Item<'a>> {
        let mut out = Vec::new();
        for part in list(self.doc, "parts") {
            let storeys: Vec<&Value> = list(part, "storeys").collect();
            for (k, st) in storeys.iter().enumerate() {
                let level = mm(&st["level"]);
                // A storey's height, else up to the next storey's level, else 2,5 m.
                let height = match (mm(&st["height"]), storeys.get(k + 1)) {
                    (h, _) if h > 0.0 => h,
                    (_, Some(next)) => mm(&next["level"]) - level,
                    _ => 2500.0,
                };
                let storey = Some((*st, level, level + height));
                for (kind, key) in [("wall", "walls"), ("slab", "slabs"), ("opening", "openings")] {
                    out.extend(list(st, key).map(|item| Item { kind, part, storey, item }));
                }
            }
            out.extend(list(part, "roofs").map(|item| Item { kind: "roof", part, storey: None, item }));
        }
        out
    }

    /// A construction's name and its layers, each its thickness in mm and its material's name.
    fn construction(&self, id: &Value) -> Option<(String, Vec<(f64, Option<String>)>)> {
        let c = list(self.doc, "constructions").find(|c| c["id"] == *id && id.is_string())?;
        let layers = list(c, "layers")
            .map(|l| (mm(&l["thickness"]) * 1000.0, l["material"]["name"].as_str().map(str::to_string)))
            .collect();
        Some((c["name"].as_str().unwrap_or_default().to_string(), layers))
    }

    fn thickness(&self, id: &Value) -> f64 {
        self.construction(id).map_or(0.0, |(_, layers)| layers.iter().map(|l| l.0).sum())
    }
}

/// A wall's line as its start, its direction, its left and its length.
fn line(wall: &Value) -> (P2, P2, P2, f64) {
    let (a, b) = (point(&wall["line"][0]), point(&wall["line"][1]));
    let l = ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2)).sqrt();
    let u = [(b[0] - a[0]) / l, (b[1] - a[1]) / l];
    (a, u, [-u[1], u[0]], l)
}

/// Where a wall's layers lie across its line, mm, to its left where positive: its first face and the side the layers
/// follow to. Layers centred on the line to the millimetre are on it.
fn placement(wall: &Value, thickness: f64) -> (f64, f64) {
    let sense = if wall["placement"]["sense"] == "right" { -1.0 } else { 1.0 };
    let first = mm(&wall["placement"]["offset"]);
    let mid = first + sense * thickness / 2.0;
    (if mid.abs() <= 1.0 { first - mid } else { first }, sense)
}

fn within(c: f64, lo: f64, hi: f64, tol: f64) -> bool {
    c >= lo.min(hi) - tol && c <= lo.max(hi) + tol
}

/// A solid lies where the building's facts put its item.
fn placed(b: &Building, at: Item, solid: &Value, corners: &[P3], volume: Option<f64>, tol: (f64, f64)) -> Check {
    let (point_tol, share) = tol;
    let near = |v: f64, expected: f64| (v - expected).abs() <= share * expected;
    match at.kind {
        "wall" | "opening" => {
            let (st, level, top) = at.storey.unwrap();
            let wall = if at.kind == "wall" {
                at.item
            } else {
                list(st, "walls").find(|w| w["id"] == at.item["host"]).ok_or("its host is no wall of its storey")?
            };
            let (name, layers) = b.construction(&wall["construction"]).ok_or("its wall has no build-up")?;
            let thickness: f64 = layers.iter().map(|l| l.0).sum();
            let (first, sense) = placement(wall, thickness);
            // Across the line: the whole thickness, or the one layer's share of it.
            let (mut lo, mut hi) = (first, first + sense * thickness);
            if let Some(k) = solid["layer"].as_u64() {
                let k = k as usize;
                let (thick, material) = layers.get(k).ok_or("its wall has no such layer")?;
                let before: f64 = layers[..k].iter().map(|l| l.0).sum();
                (lo, hi) = (first + sense * before, first + sense * (before + thick));
                if solid["material"].as_str() != Some(material.as_deref().unwrap_or(&name)) {
                    return Err("its material is not its layer's".into());
                }
            }
            let (a, u, left, l) = line(wall);
            // Along the line: a mitre reaches no farther past an end than three times the thicker wall's thickness.
            let reach = 3.0 * list(st, "walls").map(|w| b.thickness(&w["construction"])).fold(0.0, f64::max);
            let (mut from, mut to) = (-reach, l + reach);
            let (mut bottom, mut top) = (wall["bottom"].as_f64().unwrap_or(level), wall["top"].as_f64().unwrap_or(top));
            if at.kind == "opening" {
                (from, to) = (mm(&at.item["offset"]), mm(&at.item["offset"]) + mm(&at.item["width"]));
                (bottom, top) = (
                    (level + mm(&at.item["sill"])).max(bottom),
                    (level + mm(&at.item["sill"]) + mm(&at.item["height"])).min(top),
                );
                let most = mm(&at.item["width"]) * mm(&at.item["height"]) * thickness;
                if volume.is_some_and(|v| v > most * (1.0 + share)) {
                    return Err("it holds more than its width, height and its wall's thickness".into());
                }
            }
            for p in corners {
                let d = [p[0] - a[0], p[1] - a[1]];
                let (along, across) = (d[0] * u[0] + d[1] * u[1], d[0] * left[0] + d[1] * left[1]);
                if !within(across, lo, hi, point_tol) {
                    return Err(format!("a corner {across} mm off the line, outside {lo} to {hi}"));
                }
                if !within(along, from, to, point_tol) {
                    return Err(format!("a corner {along} mm along the line, outside {from} to {to}"));
                }
                if !within(p[2], bottom, top, point_tol) {
                    return Err(format!("a corner at {} mm, outside {bottom} to {top}", p[2]));
                }
            }
        }
        "slab" => {
            let thickness = b.thickness(&at.item["construction"]);
            let top = mm(&at.item["top"]);
            let outline: Vec<P2> = list(at.item, "outline").map(point).collect();
            let (xs, ys) = (outline.iter().map(|p| p[0]), outline.iter().map(|p| p[1]));
            let (x0, x1) = (xs.clone().fold(f64::MAX, f64::min), xs.fold(f64::MIN, f64::max));
            let (y0, y1) = (ys.clone().fold(f64::MAX, f64::min), ys.fold(f64::MIN, f64::max));
            let inside = |p: &P3| within(p[0], x0, x1, point_tol) && within(p[1], y0, y1, point_tol);
            if !corners.iter().all(|p| inside(p) && within(p[2], top - thickness, top, point_tol)) {
                return Err("a corner outside its outline, above its top or under its build-up".into());
            }
            let holes: f64 = list(at.item, "holes")
                .map(|h| plan_area(&h.as_array().unwrap().iter().map(point).collect::<Vec<_>>()).abs())
                .sum();
            if !volume.is_some_and(|v| near(v, (plan_area(&outline) - holes) * thickness)) {
                return Err("it does not hold its outline less its holes times its build-up's thickness".into());
            }
        }
        _ => {
            let plate = &at.item["plate"];
            let p: Vec<P3> = (0..3).map(|k| [0, 1, 2].map(|c| mm(&plate["plane"]["points"][k][c]))).collect();
            let normal = unit(cross(sub(p[1], p[0]), sub(p[2], p[0])));
            let thickness = mm(&plate["thickness"]);
            if !corners.iter().all(|c| within(dot(sub(*c, p[0]), normal), -thickness, 0.0, point_tol)) {
                return Err("a corner above its plane or farther under it than its thickness".into());
            }
            let ring = |r: &Value| r.as_array().unwrap().iter().map(point).collect::<Vec<_>>();
            let holes: f64 = list(plate, "holes").map(|h| plan_area(&ring(h)).abs()).sum();
            let expected = (plan_area(&ring(&plate["outline"])).abs() - holes) * thickness;
            match volume {
                Some(v) if thickness > 0.0 && near(v, expected) => {}
                None if thickness == 0.0 => {}
                _ => return Err("it does not hold its outline less its holes times its thickness".into()),
            }
        }
    }
    Ok(())
}

/// One file of expected solids against its building: the faults found, none where it holds.
fn faults(file: &Value) -> Vec<String> {
    let mut faults = Vec::new();
    let known = ["building", "tolerance", "solids", "without"];
    faults.extend(file.as_object().unwrap().keys().filter(|k| !known.contains(&k.as_str())).map(|k| format!("{k}?")));
    let Some(path) = file["building"].as_str().map(|p| root().join(p)).filter(|p| p.is_file()) else {
        return vec!["`building` names no document of this repository".into()];
    };
    let doc = read(&path);
    let b = Building { doc: &doc };
    let items = b.items();
    let tol = (mm(&file["tolerance"]["point"]), mm(&file["tolerance"]["volume"]));
    if !(tol.0 > 0.0 && tol.0 <= 1.0 && tol.1 > 0.0 && tol.1 <= 0.001) {
        faults.push("the tolerance is a point distance up to 1 mm and a share of the volume up to 0.001".into());
    }
    let find = |entry: &Value| {
        let at = items.iter().find(|i| i.item["id"] == entry["item"] && entry["item"].is_string())?;
        let storey = at.storey.map_or(Value::Null, |s| s.0["id"].clone());
        (at.kind == entry["kind"] && at.part["id"] == entry["part"] && storey == entry["storey"]).then_some(*at)
    };

    // The solids, each by itself.
    let mut whole: BTreeMap<&str, f64> = BTreeMap::new();
    let mut layers: BTreeMap<&str, Vec<(u64, f64)>> = BTreeMap::new();
    let mut seen = BTreeSet::new();
    for (n, solid) in list(file, "solids").enumerate() {
        let what = format!("solid {n} ({})", solid["item"]);
        let mut fault = |why: String| faults.push(format!("{what}: {why}"));
        let known = ["item", "kind", "part", "storey", "element", "layer", "material", "volume", "sheet", "faces"];
        if let Some(k) = solid.as_object().and_then(|o| o.keys().find(|k| !known.contains(&k.as_str()))) {
            fault(format!("{k}?"));
        }
        let Some(at) = find(solid) else {
            fault("no such item of that kind in that part and storey".into());
            continue;
        };
        let id = at.item["id"].as_str().unwrap();
        if !seen.insert((id, solid["element"].to_string(), solid["layer"].to_string())) {
            fault("given twice".into());
        }
        let element = &solid["element"];
        if !element.is_null() && !list(at.item, "elements").any(|e| e == element) {
            fault("its element is none of the item's".into());
        }
        if !(solid["layer"].is_null() || solid["layer"].is_u64() && at.kind == "wall") {
            fault("`layer` is a wall's layer, counted from 0".into());
        }
        let faces = match faces(solid) {
            Ok(faces) => faces,
            Err(why) => {
                fault(why);
                continue;
            }
        };
        if let Err(why) = plane(&faces) {
            fault(why);
        }
        // A sheet is a surface and states no volume; anything else is closed and states the volume its faces enclose.
        let volume = match (solid["sheet"] == true, solid["volume"].as_f64(), solid.get("sheet").is_some()) {
            (true, None, _) => None,
            (false, Some(stated), false) => match closed(&faces) {
                Ok(v) if (v - stated).abs() <= 2.0 && stated > 0.0 => Some(stated),
                Ok(v) => {
                    fault(format!("it states {stated} mm³ and its faces enclose {v}"));
                    continue;
                }
                Err(why) => {
                    fault(why);
                    continue;
                }
            },
            _ => {
                fault("either `volume`, or `sheet` as true".into());
                continue;
            }
        };
        // Where the item is made of elements its solids are their plates, which the item's own figures do not place.
        if element.is_null() {
            let corners: Vec<P3> = faces.iter().flatten().flatten().copied().collect();
            if let Err(why) = placed(&b, at, solid, &corners, volume, tol) {
                fault(why);
            }
            match (solid["layer"].as_u64(), volume) {
                (Some(k), Some(v)) => layers.entry(id).or_default().push((k, v)),
                (None, Some(v)) => _ = whole.insert(id, v),
                _ => {}
            }
        }
    }
    // A wall's layers together are the wall.
    for (id, parts) in &layers {
        let sum: f64 = parts.iter().map(|p| p.1).sum();
        match whole.get(id) {
            Some(v) if sum <= v * (1.0 + tol.1) => {}
            _ => faults.push(format!("{id}: its layers hold more than the wall, or there is no whole wall")),
        }
        let wall = items.iter().find(|i| i.item["id"] == **id).unwrap().item;
        let all = b.construction(&wall["construction"]).is_some_and(|(_, l)| l.len() == parts.len());
        if all && (sum - whole.get(id).copied().unwrap_or(0.0)).abs() > tol.1 * sum {
            faults.push(format!("{id}: all its layers together do not hold what the wall holds"));
        }
    }

    // Every item has a solid, or is listed without one under a rule that fits.
    let covered: BTreeSet<&str> = list(file, "solids").filter_map(|s| s["item"].as_str()).collect();
    let mut without = BTreeMap::new();
    for entry in list(file, "without") {
        let known = ["item", "kind", "part", "storey", "rule"];
        let extra = entry.as_object().is_some_and(|o| o.keys().any(|k| !known.contains(&k.as_str())));
        match (find(entry), entry["rule"].as_str()) {
            (Some(at), Some(rule)) if !extra => {
                if without.insert(at.item["id"].as_str().unwrap(), (at, rule)).is_some() {
                    faults.push(format!("{} is listed twice without a solid", entry["item"]));
                }
            }
            _ => faults.push(format!("without {}: no such item of that kind there, or no rule", entry["item"])),
        }
    }
    for at in &items {
        let id = at.item["id"].as_str().unwrap();
        let separation = at.kind == "wall" && at.item["kind"] == "separation";
        let fits = match without.get(id) {
            None => covered.contains(id) && !separation,
            Some(_) if covered.contains(id) => false,
            // A separation line is no building element.
            Some((_, "separation")) => separation,
            // A slab without a build-up, or without an outline, has no thickness to be a solid of.
            Some((_, "noBuildUp")) => {
                at.kind == "slab"
                    && (b.thickness(&at.item["construction"]) <= 0.0 || list(at.item, "outline").count() < 3)
            }
            // An opening has its solid from its wall's: where the wall has none of its own, neither has the opening.
            Some((_, "host")) => {
                at.kind == "opening" && !whole.contains_key(at.item["host"].as_str().unwrap_or_default())
            }
            // A built item the rules leave nothing of.
            Some((_, "noBody")) => !separation,
            Some(_) => false,
        };
        if !fits {
            faults.push(format!(
                "the {} {id} has neither a solid nor a rule that leaves it without one, or both",
                at.kind
            ));
        }
    }
    faults
}

#[test]
fn the_expected_solids_hold() {
    for (name, file) in files() {
        let faults = faults(&file);
        assert!(faults.is_empty(), "{name}:\n{}", faults.join("\n"));
        assert!(list(&file, "solids").count() > 0, "{name}: no solids");
    }
}

/// The check finds what it is there to find: a solid with a face missing, with a face turned inside out, with a
/// corner moved, with another volume; an item left out; a rule that does not fit.
#[test]
fn a_faulty_file_is_found() {
    let (_, good) = files().into_iter().find(|(name, _)| name == "werkstatt-am-bach").unwrap();
    let broken = |change: &dyn Fn(&mut Value)| {
        let mut file = good.clone();
        change(&mut file);
        !faults(&file).is_empty()
    };
    assert!(!broken(&|_| {}));
    assert!(broken(&|f| _ = f["solids"][0]["faces"].as_array_mut().unwrap().pop()), "a face missing");
    assert!(broken(&|f| f["solids"][0]["faces"][2][0].as_array_mut().unwrap().reverse()), "a face turned");
    assert!(
        broken(&|f| f["solids"][0]["faces"].as_array_mut().unwrap().iter_mut().for_each(|face| {
            face.as_array_mut().unwrap().iter_mut().for_each(|ring| ring.as_array_mut().unwrap().reverse())
        })),
        "inside out"
    );
    assert!(broken(&|f| f["solids"][0]["volume"] = 8_915_232_800_u64.into()), "another volume");
    assert!(broken(&|f| _ = f["solids"].as_array_mut().unwrap().remove(4)), "an opening left out");
    assert!(broken(&|f| f["without"][0]["rule"] = "noBuildUp".into()), "a rule that does not fit");
    assert!(broken(&|f| f["without"][1]["rule"] = "separation".into()), "a rule that does not fit");
    assert!(broken(&|f| f["solids"][1]["layer"] = 2.into()), "a layer in another layer's place");
    // Every corner of the first wall moved 20 mm down: still closed, and no longer on its storey's level.
    assert!(
        broken(&|f| {
            let faces = f["solids"][0]["faces"].as_array_mut().unwrap();
            let rings = faces.iter_mut().flat_map(|face| face.as_array_mut().unwrap());
            for p in rings.flat_map(|ring| ring.as_array_mut().unwrap()) {
                p[2] = (p[2].as_f64().unwrap() - 20.0).into();
            }
        }),
        "a wall moved"
    );
}

/// The integral of a width times a height along a stretch, both linear on it: (from, to, width at each, height at
/// each). Simpson's rule, exact for their product.
fn under(pieces: &[(f64, f64, [f64; 2], [f64; 2])]) -> f64 {
    pieces.iter().map(|(a, b, w, h)| (b - a) / 6.0 * (w[0] * h[0] + (w[0] + w[1]) * (h[0] + h[1]) + w[1] * h[1])).sum()
}

/// The whole solids' volumes worked out by hand from the examples' figures, mm³, by file and the item's last digits.
fn by_hand() -> Vec<(&'static str, &'static str, f64)> {
    let mut out = Vec::new();

    // Haus am Hang: 10 m × 8 m, walls of 335 mm inside their lines and mitred at the four corners, so a wall's mean
    // length is its line's less 335; two storeys of 2750.
    let (t, h) = (335.0, 2750.0);
    let window = |w: f64, h: f64| w * h * t;
    out.push(("haus-am-hang", "0b", (10000.0 - t) * t * h - window(2010.0, 1385.0)));
    out.push(("haus-am-hang", "0c", (8000.0 - t) * t * h - window(1135.0, 1260.0)));
    out.push(("haus-am-hang", "0d", (10000.0 - t) * t * h - window(1010.0, 2135.0)));
    out.push(("haus-am-hang", "0e", (8000.0 - t) * t * h));
    for (id, w) in [("16", window(2010.0, 1385.0)), ("17", window(1135.0, 1260.0)), ("18", window(1010.0, 2135.0))] {
        out.push(("haus-am-hang", id, w));
    }
    out.push(("haus-am-hang", "19", window(1510.0, 1385.0)));
    // Upper storey, 2750 to 5500. The roof plates rise 2700 on 4000 from 5500 at the eaves lines and are 160 thick.
    // A plate's lower face is its outline moved 160 against its normal: it lies 160 · √(1 + 0,675²) under the upper
    // one and begins `e` inside the eaves line. From there to `d`, where it passes 5500, it cuts the walls; the strip
    // of `e` outside it stands to the storey's top.
    let slope: f64 = 2700.0 / 4000.0;
    let pitch = (1.0 + slope * slope).sqrt();
    let lower = |y: f64| 5500.0 - 160.0 * pitch + slope * y - 2750.0;
    let e = 160.0 * slope / pitch;
    let d = (h - lower(0.0)) / slope;
    // An eaves wall: its length shrinks by 2 per mm inwards.
    let long = |y: f64| 10000.0 - 2.0 * y;
    let eaves_wall = under(&[
        (0.0, e, [long(0.0), long(e)], [h, h]),
        (e, d, [long(e), long(d)], [lower(e), h]),
        (d, t, [long(d), long(t)], [h, h]),
    ]);
    out.push(("haus-am-hang", "0f", eaves_wall - window(1510.0, 1385.0)));
    out.push(("haus-am-hang", "11", eaves_wall));
    // A gable wall: the slope cuts only its two mitred ends, where it is as wide as it is far from the eaves line.
    let end = under(&[(0.0, e, [0.0, e], [h, h]), (e, d, [e, d], [lower(e), h]), (d, t, [d, t], [h, h])]);
    for id in ["10", "12"] {
        out.push(("haus-am-hang", id, 2.0 * end + (8000.0 - 2.0 * t) * t * h));
    }
    out.push(("haus-am-hang", "13", 10000.0 * 8000.0 * 160.0));
    for id in ["14", "15"] {
        out.push(("haus-am-hang", id, 10000.0 * 4826.0 * 160.0));
    }

    // Werkstatt am Bach: 8 m × 5 m, outer walls of 400 mm inside their lines, mitred. The roof plate falls 500 on
    // 5000 from 3400 over the south line and is 200 thick; its underside cuts every outer wall and the wall between.
    let t = 400.0;
    let roof = |y: f64| 3400.0 - 200.0 * 1.01_f64.sqrt() - 0.1 * y;
    let south = |y: f64| 8000.0 - 2.0 * y;
    out.push((
        "werkstatt-am-bach",
        "0a",
        under(&[(0.0, t, [south(0.0), south(t)], [roof(0.0), roof(t)])]) - 1500.0 * 1250.0 * t,
    ));
    let north = |y: f64| 8000.0 - 2.0 * (5000.0 - y);
    out.push((
        "werkstatt-am-bach",
        "0c",
        under(&[(4600.0, 5000.0, [north(4600.0), north(5000.0)], [roof(4600.0), roof(5000.0)])]),
    ));
    // East and west: full width between the mitres, which narrow them to nothing at the two corners.
    let side = under(&[
        (0.0, t, [0.0, t], [roof(0.0), roof(t)]),
        (t, 5000.0 - t, [t, t], [roof(t), roof(5000.0 - t)]),
        (5000.0 - t, 5000.0, [t, 0.0], [roof(5000.0 - t), roof(5000.0)]),
    ]);
    out.push(("werkstatt-am-bach", "0b", side));
    out.push(("werkstatt-am-bach", "0d", side - 1000.0 * 2100.0 * t));
    // The wall between, 145 mm on its line, from the south wall's inner face to the north wall's.
    out.push(("werkstatt-am-bach", "0e", under(&[(t, 5000.0 - t, [145.0, 145.0], [roof(t), roof(5000.0 - t)])])));
    // The store's wall, 145 mm and 2200 high, from the face of the wall between (72,5 mm past its line at 4000) to the
    // east wall's inner face at 7600, less its door.
    out.push(("werkstatt-am-bach", "0f", (7600.0 - 4072.5) * 145.0 * 2200.0 - 1000.0 * 2000.0 * 145.0));
    for (id, v) in [("11", 1500.0 * 1250.0 * t), ("13", 1000.0 * 2100.0 * t), ("12", 1000.0 * 2000.0 * 145.0)] {
        out.push(("werkstatt-am-bach", id, v));
    }
    // The store's floor less its hatch, 200 mm; the roof plate less its roof light, 200 mm.
    out.push(("werkstatt-am-bach", "15", (4000.0 * 2500.0 - 800.0 * 800.0) * 200.0));
    out.push(("werkstatt-am-bach", "16", (8800.0 * 5830.0 - 1000.0 * 1000.0) * 200.0));
    out
}

#[test]
fn the_volumes_are_those_worked_out_by_hand() {
    let by_hand = by_hand();
    for (name, file) in files() {
        let share = mm(&file["tolerance"]["volume"]);
        let whole: Vec<&Value> =
            list(&file, "solids").filter(|s| s["layer"].is_null() && s["element"].is_null()).collect();
        for solid in &whole {
            let id = solid["item"].as_str().unwrap();
            let hand = by_hand.iter().find(|(file, item, _)| *file == name && id.ends_with(item));
            let (_, _, expected) = hand.unwrap_or_else(|| panic!("{name}: {id} has no volume worked out by hand"));
            let stated = mm(&solid["volume"]);
            assert!((stated - expected).abs() <= share * expected, "{name}: {id} states {stated}, by hand {expected}");
        }
        assert_eq!(whole.len(), by_hand.iter().filter(|(file, _, _)| *file == name).count(), "{name}");
    }
}
