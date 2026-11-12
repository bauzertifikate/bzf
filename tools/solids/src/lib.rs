// SPDX-License-Identifier: Apache-2.0 OR MIT
//! The solids of a building document by the rules of SOLIDS.md: a reference implementation.
//!
//! [`solids`] takes a building document ([`Building`], read from `schema/bzf/1/building.json` with serde) and gives
//! every solid of it: a wall between its faces from its bottom to its top, cut at its ends as its footprint is
//! ([`footprint`]), ending under the roof and open where its openings are, and each of its layers likewise; a slab its
//! outline down from its top; a roof plate its outline against its plane's normal; an element its plate; an opening what
//! it takes out of its wall. [`without`] names every item the rules leave without a solid, with the rule, and [`file`]
//! writes both in the format of the expected solids (`examples/solids/`).
//!
//! Every solid is closed: each of its edges lies in exactly two of its faces, once each way, and its faces' outer rings
//! run counter-clockwise seen from outside ([`closed`], [`volume`]). A plate without thickness is a surface (`sheet`).
//!
//! Units: points are millimetres in the project's frame, volumes cubic millimetres. Plan coordinates come in as whole
//! millimetres; a footprint's corners are rounded to whole millimetres, everything after that is `f64` worked out with
//! addition, multiplication, division, square roots and `libm` only, so the results are the same on every platform.
//! SOLIDS.md states how close another implementation has to come: corners within 1 mm, volumes within 0.1 %.
//!
//! The text is the rule. Where this crate and SOLIDS.md differ, the crate is wrong: please report it.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod column;
pub mod document;
mod file;
pub mod footprint;
mod mesh;
mod over;
mod roof;

pub use document::{Building, Id, Part, Plate, Point, Roof, Storey, Wall};
pub use file::{Rule, Without, file, without};
pub use footprint::{Footprint, footprint};
pub use mesh::{closed, newell, volume};
pub use over::{Height, Surface};
pub use roof::Invalid;

use column::{Column, Void};
use document::{ElementKind, Opening, Side, WallKind};
use std::collections::BTreeSet;

/// A point in the project's frame, mm.
pub type P3 = [f64; 3];
pub(crate) type P2 = [f64; 2];

/// A planar face: its outer ring counter-clockwise seen from outside, then its holes the other way.
#[derive(Clone, Debug, PartialEq)]
pub struct Face {
    /// Its rings, each a closed run of points without the first repeated.
    pub rings: Vec<Vec<P3>>,
    /// The unit normal, pointing outside.
    pub normal: P3,
}

/// What a solid is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// A wall, a layer of one, or a wall element's plate.
    Wall,
    /// A slab or a slab element's plate.
    Slab,
    /// A roof plate or a roof element's plate.
    Roof,
    /// What an opening takes out of its wall.
    Opening,
}

/// An item's solid, or a part of it.
#[derive(Clone, Debug)]
pub struct Solid {
    /// What it is.
    pub kind: Kind,
    /// The item: a wall, a slab, a roof plate, an opening; for an element's plate the item it is part of, else the
    /// element.
    pub id: Id,
    /// The element whose plate this is.
    pub element: Option<Id>,
    /// For a layer of a wall with a build-up: which one, from 0 in the build-up's order. None for the whole wall.
    pub layer: Option<usize>,
    /// A layer's material, an element's label.
    pub material: Option<String>,
    /// The part, by its index in the building.
    pub part: usize,
    /// The storey it stands in, by its index in the part; none for a roof, which is the part's.
    pub storey: Option<usize>,
    /// Its faces.
    pub faces: Vec<Face>,
    /// A surface without thickness, which closes nothing.
    pub sheet: bool,
}

impl Solid {
    /// Whether its faces close it ([`closed`]).
    pub fn closed(&self) -> bool {
        closed(&self.faces)
    }

    /// Its volume, mm³ ([`volume`]).
    pub fn volume(&self) -> f64 {
        volume(&self.faces)
    }
}

/// A wall or slab that has no solid, as [`solids`] finds it on the way. [`without`] lists every item without a solid.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Left {
    /// The part, by its index in the building.
    pub part: usize,
    /// The storey, by its index in the part.
    pub storey: usize,
    /// The wall or slab.
    pub id: Id,
    /// Why it has none.
    pub why: Why,
}

/// Why a wall or slab has no solid.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Why {
    /// A wall without a build-up, a thickness, a length or a height, or one cut back to nothing at its ends: it lies
    /// in a wall that runs on past it.
    NoBody,
    /// A slab without a build-up or with fewer than three corners.
    NoBuildUp,
}

/// Every solid of `b`, and the walls and slabs that have none. Part by part and storey by storey: each built wall
/// whole, then its layers, then what its openings take out of it; the storey's slabs; then the part's roof plates and
/// its elements' plates. A separation line has no solid; an item made of elements with plates is those plates.
pub fn solids(b: &Building) -> (Vec<Solid>, Vec<Left>) {
    let mut out = Vec::new();
    let mut left = Vec::new();
    for (pi, part) in b.parts.iter().enumerate() {
        let over = Surface::of(part);
        let plates: BTreeSet<&Id> =
            part.storeys.iter().flat_map(|s| &s.elements).filter(|e| e.plate.is_some()).map(|e| &e.id).collect();
        let plated = |ids: &[Id]| ids.iter().any(|id| plates.contains(id));
        for (k, st) in part.storeys.iter().enumerate() {
            let here = Place { part: pi, storey: Some(k) };
            walls(b, part, k, over.as_ref(), &plated, here, &mut out, &mut left);
            for slab in st.slabs.iter().filter(|s| !plated(&s.elements)) {
                let thickness = thickness(b, slab.construction.as_ref());
                if thickness <= 0.0 || slab.outline.len() < 3 {
                    left.push(Left { part: pi, storey: k, id: slab.id.clone(), why: Why::NoBuildUp });
                    continue;
                }
                let mut rings = vec![floats(&slab.outline)];
                rings.extend(slab.holes.iter().map(|h| floats(h)));
                let faces = prism(&rings, slab.top as f64 - thickness, slab.top as f64);
                out.push(here.solid(Kind::Slab, &slab.id, faces));
            }
        }
        for roof in part.roofs.iter().filter(|r| !plated(&r.elements)) {
            if let Some((faces, sheet)) = plate(&roof.plate) {
                let mut solid = Place { part: pi, storey: None }.solid(Kind::Roof, &roof.id, faces);
                solid.sheet = sheet;
                out.push(solid);
            }
        }
        // Every element's plate, once.
        let mut used: BTreeSet<&Id> = BTreeSet::new();
        for (k, st) in part.storeys.iter().enumerate() {
            for e in &st.elements {
                let Some(p) = &e.plate else { continue };
                if !used.insert(&e.id) {
                    continue;
                }
                let Some((faces, sheet)) = plate(p) else { continue };
                let kind = match e.kind {
                    Some(ElementKind::Slab) => Kind::Slab,
                    Some(ElementKind::Roof) => Kind::Roof,
                    _ => Kind::Wall,
                };
                let here = Place { part: pi, storey: (kind != Kind::Roof).then_some(k) };
                let mut solid = here.solid(kind, e.of.as_ref().unwrap_or(&e.id), faces);
                solid.element = Some(e.id.clone());
                solid.layer = e.layer.map(|l| l as usize);
                solid.material = (!e.label.trim().is_empty()).then(|| e.label.trim().to_string());
                solid.sheet = sheet;
                out.push(solid);
            }
        }
    }
    (out, left)
}

/// Where solids belong.
#[derive(Clone, Copy)]
struct Place {
    part: usize,
    storey: Option<usize>,
}

impl Place {
    fn solid(self, kind: Kind, id: &Id, faces: Vec<Face>) -> Solid {
        let Place { part, storey } = self;
        Solid { kind, id: id.clone(), element: None, layer: None, material: None, part, storey, faces, sheet: false }
    }
}

/// Storey `k`'s walls with a build-up: each whole, its layers, and what its openings take out of it.
#[allow(clippy::too_many_arguments)]
fn walls(
    b: &Building,
    part: &Part,
    k: usize,
    over: Option<&Surface>,
    plated: &dyn Fn(&[Id]) -> bool,
    here: Place,
    out: &mut Vec<Solid>,
    left: &mut Vec<Left>,
) {
    let st = &part.storeys[k];
    let height = storey_height(part, k);
    let prints: Vec<(&Wall, Option<Footprint>)> =
        st.walls.iter().filter(|w| built(w)).map(|w| (w, footprint(b, st, w))).collect();
    let others: Vec<Other> = prints
        .iter()
        .filter_map(|(w, fp)| {
            let fp = fp.as_ref()?;
            Some(Other {
                id: &w.id,
                ring: fp.corners.iter().map(|c| [c[0] as f64, c[1] as f64]).collect(),
                line: w.line.map(|p| [p[0] as f64, p[1] as f64]),
                thickness: fp.faces.1 - fp.faces.0,
                exterior: w.exterior,
            })
        })
        .collect();
    for (wall, fp) in &prints {
        if plated(&wall.elements) {
            continue;
        }
        let before = out.len();
        if let (Some(fp), Some(construction)) =
            (fp, wall.construction.as_ref().and_then(|c| b.constructions.iter().find(|x| x.id == *c)))
        {
            let (bottom, top) = (wall.bottom.unwrap_or(st.level) as f64, wall.top.unwrap_or(st.level + height) as f64);
            let base = [wall.line[0][0] as f64, wall.line[0][1] as f64];
            let band =
                Band { base, along: fp.along, across: fp.across, faces: fp.faces, ends: ends(wall, fp, &others) };
            let along = Height { a: fp.along[0], b: fp.along[1], c: -(fp.along[0] * base[0] + fp.along[1] * base[1]) };
            let openings: Vec<&Opening> =
                st.openings.iter().filter(|o| o.host == wall.id && o.width > 0 && o.height > 0).collect();
            let voids: Vec<Void> = openings
                .iter()
                .map(|o| {
                    let z0 = (st.level + o.sill) as f64;
                    Void { along, from: o.offset as f64, to: (o.offset + o.width) as f64, z0, z1: z0 + o.height as f64 }
                })
                .collect();
            let column = |ring: Vec<P2>| Column { ring, bottom, top, over, foot: bottom, voids: &voids }.faces();
            let whole = band.quad(fp.faces.0, fp.faces.1).map(column).unwrap_or_default();
            if !whole.is_empty() {
                out.push(here.solid(Kind::Wall, &wall.id, whole));
                // The layers from the first's face to the `sense` side.
                let sense = if wall.placement.sense == Some(Side::Right) { -1.0 } else { 1.0 };
                let mut at = if sense > 0.0 { fp.faces.0 } else { fp.faces.1 };
                for (i, layer) in construction.layers.iter().enumerate() {
                    let next = at + sense * layer.thickness * 1000.0;
                    let (ta, tb) = (at.min(next), at.max(next));
                    at = next;
                    let faces = band.quad(ta, tb).map(column).unwrap_or_default();
                    if faces.is_empty() {
                        continue;
                    }
                    let name = layer.material.name.trim();
                    let name = if name.is_empty() { construction.name.trim() } else { name };
                    let mut solid = here.solid(Kind::Wall, &wall.id, faces);
                    solid.layer = Some(i);
                    solid.material = (!name.is_empty()).then(|| name.to_string());
                    out.push(solid);
                }
                for (o, v) in openings.iter().zip(&voids) {
                    let Some(ring) = band.quad(fp.faces.0, fp.faces.1) else { continue };
                    let Some(ring) = column::between(&ring, &along, v.from, v.to) else { continue };
                    let (z0, z1) = (v.z0.max(bottom), v.z1.min(top));
                    let faces = Column { ring, bottom: z0, top: z1, over, foot: bottom, voids: &[] }.faces();
                    if !faces.is_empty() {
                        out.push(here.solid(Kind::Opening, &o.id, faces));
                    }
                }
            }
        }
        if out.len() == before {
            left.push(Left { part: here.part, storey: k, id: wall.id.clone(), why: Why::NoBody });
        }
    }
}

/// How high storey `k` of `part` is, mm: its own height, else up to the next storey, else 2,50 m.
pub fn storey_height(part: &Part, k: usize) -> i64 {
    let st = &part.storeys[k];
    if st.height > 0 {
        return st.height;
    }
    match part.storeys.get(k + 1) {
        Some(next) if next.level > st.level => next.level - st.level,
        _ => 2500,
    }
}

/// A build-up's thickness, mm; 0 without one.
pub fn thickness(b: &Building, construction: Option<&Id>) -> f64 {
    construction
        .and_then(|c| b.constructions.iter().find(|k| k.id == *c))
        .map_or(0.0, |c| c.layers.iter().map(|l| l.thickness).sum::<f64>() * 1000.0)
}

/// Whether a wall is built: not a separation line, which only bounds rooms.
pub fn built(w: &Wall) -> bool {
    w.kind != Some(WallKind::Separation)
}

/// Another wall of the storey, which a wall's end may run into.
struct Other<'a> {
    id: &'a Id,
    /// Its footprint.
    ring: Vec<P2>,
    line: [P2; 2],
    thickness: f64,
    exterior: bool,
}

impl Other<'_> {
    /// Whether it runs on past `p` both ways, rather than ending there.
    fn passes(&self, p: P2) -> bool {
        let far = |q: P2| ((q[0] - p[0]) * (q[0] - p[0]) + (q[1] - p[1]) * (q[1] - p[1])).sqrt() > self.thickness + 5.0;
        far(self.line[0]) && far(self.line[1])
    }
}

/// Where a wall starts and ends on either face (start on the first face, on the second, end on the first, on the
/// second), along its line, mm: as its footprint `fp` is cut, but a square end that runs into another wall cut back to
/// that wall's face, as a wall drawn up to another wall's line would otherwise stand inside it: an inner wall into an
/// exterior wall or into an inner wall running on past it, an exterior wall into an exterior wall running on past it.
fn ends(wall: &Wall, fp: &Footprint, others: &[Other]) -> [f64; 4] {
    let base = [wall.line[0][0] as f64, wall.line[0][1] as f64];
    let (along, across) = (fp.along, fp.across);
    // The footprint's corners along and across the line, those on a face put onto it exactly.
    let st_of = |p: &Point| {
        let d = [p[0] as f64 - base[0], p[1] as f64 - base[1]];
        let mut t = d[0] * across[0] + d[1] * across[1];
        for f in [fp.faces.0, fp.faces.1] {
            if (t - f).abs() <= 1.5 {
                t = f;
            }
        }
        [d[0] * along[0] + d[1] * along[1], t]
    };
    let corners: Vec<P2> = fp.corners.iter().map(st_of).collect();
    let on = |t: f64| corners.iter().filter(move |c| c[1] == t).map(|c| c[0]);
    let ends = |t: f64| -> (f64, f64) {
        let lo = on(t).fold(f64::MAX, f64::min);
        let hi = on(t).fold(f64::MIN, f64::max);
        if lo <= hi { (lo, hi) } else { fp.span }
    };
    let (e0, e1) = (ends(fp.faces.0), ends(fp.faces.1));
    let (mut s0, mut s1, mut f0, mut f1) = (e0.0, e1.0, e0.1, e1.1);
    let to_wall = |p: P2| {
        let d = [p[0] - base[0], p[1] - base[1]];
        [d[0] * along[0] + d[1] * along[1], d[0] * across[0] + d[1] * across[1]]
    };
    for end in [false, true] {
        let square = if end { (f0 - f1).abs() < 0.5 } else { (s0 - s1).abs() < 0.5 };
        if !square {
            continue;
        }
        let at = if end { f0.max(f1) } else { s0.min(s1) };
        let tip = if end { wall.line[1] } else { wall.line[0] };
        let tip = [tip[0] as f64, tip[1] as f64];
        let mut cut: Option<f64> = None;
        for o in others.iter().filter(|o| o.id != &wall.id) {
            let runs_into = if wall.exterior { o.exterior && o.passes(tip) } else { o.exterior || o.passes(tip) };
            if !runs_into {
                continue;
            }
            let ring: Vec<P2> = o.ring.iter().map(|&p| to_wall(p)).collect();
            for t in [fp.faces.0 + 1.0, fp.faces.1 - 1.0] {
                let probe = if end { at - 2.0 } else { at + 2.0 };
                if !over::inside([probe, t], &ring) {
                    continue;
                }
                let crossings = (0..ring.len()).filter_map(|k| {
                    let (p, q) = (ring[k], ring[(k + 1) % ring.len()]);
                    ((p[1] - t) * (q[1] - t) < 0.0).then(|| p[0] + (t - p[1]) / (q[1] - p[1]) * (q[0] - p[0]))
                });
                let entry = if end {
                    crossings.filter(|&s| s < probe).fold(None, |m: Option<f64>, s| Some(m.map_or(s, |m| m.max(s))))
                } else {
                    crossings.filter(|&s| s > probe).fold(None, |m: Option<f64>, s| Some(m.map_or(s, |m| m.min(s))))
                };
                if let Some(e) = entry {
                    cut = Some(cut.map_or(e, |c| if end { c.min(e) } else { c.max(e) }));
                }
            }
        }
        match (cut, end) {
            (Some(c), true) => (f0, f1) = (c, c),
            (Some(c), false) => (s0, s1) = (c, c),
            _ => {}
        }
    }
    [s0, s1, f0, f1]
}

/// A wall in plan in its own frame: along its line from its start and across it, its faces and its ends there.
struct Band {
    base: P2,
    along: P2,
    across: P2,
    faces: (f64, f64),
    ends: [f64; 4],
}

impl Band {
    fn point(&self, s: f64, t: f64) -> P2 {
        let (a, c) = (self.along, self.across);
        [self.base[0] + a[0] * s + c[0] * t, self.base[1] + a[1] * s + c[1] * t]
    }

    /// Where the start cuts the band at `t` across it.
    fn start(&self, t: f64) -> f64 {
        let [s0, s1, ..] = self.ends;
        s0 + (s1 - s0) * (t - self.faces.0) / (self.faces.1 - self.faces.0)
    }

    /// Where the end cuts the band at `t` across it.
    fn end(&self, t: f64) -> f64 {
        let [.., f0, f1] = self.ends;
        f0 + (f1 - f0) * (t - self.faces.0) / (self.faces.1 - self.faces.0)
    }

    /// The band from `ta` to `tb` across the wall, counter-clockwise; none where it is thinner than half a millimetre
    /// or one of its faces shorter than a millimetre.
    fn quad(&self, ta: f64, tb: f64) -> Option<Vec<P2>> {
        if tb - ta < 0.5 {
            return None;
        }
        let (sa, ea, sb, eb) = (self.start(ta), self.end(ta), self.start(tb), self.end(tb));
        if ea - sa < 1.0 || eb - sb < 1.0 {
            return None;
        }
        Some(vec![self.point(sa, ta), self.point(ea, ta), self.point(eb, tb), self.point(sb, tb)])
    }
}

/// A plate's solid: its outline and holes in its plane, reaching its thickness against the normal; a surface (true)
/// where it has none. None for a plate without a plane or with fewer than three corners.
pub fn plate(p: &Plate) -> Option<(Vec<Face>, bool)> {
    let f = p.frame()?;
    if p.outline.len() < 3 {
        return None;
    }
    let mut rings: Vec<Vec<P2>> = vec![oriented(floats(&p.outline), true)];
    rings.extend(p.holes.iter().filter(|h| h.len() >= 3).map(|h| oriented(floats(h), false)));
    let at = |q: P2, depth: f64| -> P3 {
        [0, 1, 2].map(|k| f.origin[k] + f.u[k] * q[0] + f.v[k] * q[1] - f.normal[k] * depth)
    };
    let t = p.thickness as f64;
    let front: Vec<Vec<P3>> = rings.iter().map(|r| r.iter().map(|&q| at(q, 0.0)).collect()).collect();
    if t <= 0.0 {
        return Some((face(front, f.normal).into_iter().collect(), true));
    }
    let back: Vec<Vec<P3>> = rings.iter().map(|r| r.iter().map(|&q| at(q, t)).collect()).collect();
    let mut faces: Vec<Face> = face(front, f.normal).into_iter().collect();
    faces.extend(face(back, f.normal.map(|c| -c)));
    for r in &rings {
        for i in 0..r.len() {
            let (a, b) = (r[i], r[(i + 1) % r.len()]);
            let (du, dv) = (b[0] - a[0], b[1] - a[1]);
            let out = [0, 1, 2].map(|k| f.u[k] * dv - f.v[k] * du);
            faces.extend(face(vec![vec![at(a, 0.0), at(a, t), at(b, t), at(b, 0.0)]], out));
        }
    }
    Some((faces, false))
}

/// The prism over the plan rings `rings` (the outline, then holes), mm, from `z0` to `z1`.
pub fn prism(rings: &[Vec<[f64; 2]>], z0: f64, z1: f64) -> Vec<Face> {
    let rings: Vec<Vec<P2>> =
        rings.iter().enumerate().filter(|(_, r)| r.len() >= 3).map(|(i, r)| oriented(r.clone(), i == 0)).collect();
    if rings.is_empty() || z1 <= z0 {
        return Vec::new();
    }
    let lift = |z: f64| rings.iter().map(|r| r.iter().map(|q| [q[0], q[1], z]).collect()).collect::<Vec<Vec<P3>>>();
    let mut faces: Vec<Face> = face(lift(z1), [0.0, 0.0, 1.0]).into_iter().collect();
    faces.extend(face(lift(z0), [0.0, 0.0, -1.0]));
    for r in &rings {
        for i in 0..r.len() {
            let (a, b) = (r[i], r[(i + 1) % r.len()]);
            let out = [b[1] - a[1], a[0] - b[0], 0.0];
            faces.extend(face(vec![vec![[a[0], a[1], z0], [b[0], b[1], z0], [b[0], b[1], z1], [a[0], a[1], z1]]], out));
        }
    }
    faces
}

/// A ring counter-clockwise (`outer`) or clockwise, without repeated points.
fn oriented(mut ring: Vec<P2>, outer: bool) -> Vec<P2> {
    ring.dedup();
    while ring.len() > 1 && ring.first() == ring.last() {
        ring.pop();
    }
    let twice: f64 = (0..ring.len())
        .map(|i| {
            let (a, b) = (ring[i], ring[(i + 1) % ring.len()]);
            a[0] * b[1] - b[0] * a[1]
        })
        .sum();
    if (twice > 0.0) != outer {
        ring.reverse();
    }
    ring
}

/// A face of `rings` whose normal points to the side of `outward`; none where the outer ring has no area.
pub fn face(mut rings: Vec<Vec<P3>>, outward: P3) -> Option<Face> {
    for r in rings.iter_mut() {
        r.dedup();
        while r.len() > 1 && r.first() == r.last() {
            r.pop();
        }
    }
    rings.retain(|r| r.len() >= 3);
    let n = newell(rings.first()?);
    let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
    if len < 1e-6 {
        return None;
    }
    let mut normal = n.map(|c| c / len);
    if normal[0] * outward[0] + normal[1] * outward[1] + normal[2] * outward[2] < 0.0 {
        normal = normal.map(|c| -c);
        rings[0].reverse();
    }
    for hole in rings.iter_mut().skip(1) {
        let h = newell(hole);
        if h[0] * normal[0] + h[1] * normal[1] + h[2] * normal[2] > 0.0 {
            hole.reverse();
        }
    }
    Some(Face { rings, normal })
}

fn floats(ring: &[Point]) -> Vec<P2> {
    ring.iter().map(|p| [p[0] as f64, p[1] as f64]).collect()
}
