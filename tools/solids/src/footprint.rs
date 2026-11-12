// SPDX-License-Identifier: Apache-2.0 OR MIT
//! A wall's footprint in plan (SOLIDS.md, "Walls": faces and ends): the band between its faces, cut square or along
//! the mitre with the one other wall that ends where it ends; for a wall made of elements the rectangle around its
//! elements' plates.
//!
//! [`footprint`] takes a building document. The functions under it ([`faces`], [`drawn`], [`of_elements`],
//! [`plate_corners`]) take plain values, so that a program with a model of its own can draw the same footprints.

use crate::document::{Building, Construction, ElementKind, Id, PlaneFrame, Point, Side, Storey, Wall, WallKind};

/// How far apart, mm, a wall's end and another wall's end may lie and still meet.
pub const MEET: f64 = 5.0;

/// A wall's footprint in plan and where it lies along and across its reference line.
#[derive(Clone, Debug, PartialEq)]
pub struct Footprint {
    /// Its corners, mm: for a drawn wall the band between its faces cut square or along the mitre at each end, for a
    /// wall of elements the rectangle around its elements' plates.
    pub corners: Vec<Point>,
    /// The unit direction of its reference line, from its start.
    pub along: [f64; 2],
    /// The unit normal to the line's left.
    pub across: [f64; 2],
    /// How far along the line, from its start, the footprint reaches either way, mm.
    pub span: (f64, f64),
    /// Where its faces lie across the line, mm, to the left where positive, the smaller first.
    pub faces: (f64, f64),
}

/// A wall of a storey as the rules for its ends see it.
#[derive(Clone, Copy, Debug)]
pub struct Line<'a> {
    /// The wall's id: a wall never meets itself.
    pub id: &'a Id,
    /// Its reference line, mm.
    pub line: [Point; 2],
    /// Where its faces lie across the line ([`faces`]); none for a wall without a thickness.
    pub faces: Option<(f64, f64)>,
    /// A separation line, which bounds rooms only.
    pub separation: bool,
    /// Made of elements: its footprint is theirs.
    pub elements: bool,
}

impl<'a> Line<'a> {
    /// A wall of a building document, with the build-ups it may refer to.
    pub fn of(wall: &'a Wall, constructions: &[Construction]) -> Line<'a> {
        let separation = wall.kind == Some(WallKind::Separation);
        let thickness = wall
            .construction
            .as_ref()
            .and_then(|c| constructions.iter().find(|k| k.id == *c))
            .map(|c| c.layers.iter().map(|l| l.thickness).sum::<f64>() * 1000.0);
        let right = wall.placement.sense == Some(Side::Right);
        Line {
            id: &wall.id,
            line: wall.line,
            faces: faces(wall.placement.offset, right, thickness, separation),
            separation,
            elements: !wall.elements.is_empty(),
        }
    }
}

/// The footprint of `wall` in `storey` of the building `b`; none for a wall of no length or no thickness. A separation
/// line has one too, a band of 2 mm, so that it bounds rooms.
pub fn footprint(b: &Building, storey: &Storey, wall: &Wall) -> Option<Footprint> {
    let line = Line::of(wall, &b.constructions);
    if !line.elements {
        let others: Vec<Line> = storey.walls.iter().map(|w| Line::of(w, &b.constructions)).collect();
        return drawn(&line, &others, [0, 0]);
    }
    let mut corners = Vec::new();
    for e in storey.elements.iter().filter(|e| wall.elements.contains(&e.id) && e.kind == Some(ElementKind::Wall)) {
        let Some(plate) = &e.plate else { continue };
        let Some(f) = plate.frame() else { continue };
        corners.extend(plate_corners(&f, &plate.outline, plate.thickness, [0, 0]));
    }
    of_elements(wall.line, &corners, [0, 0])
}

/// Where a drawn wall's faces lie across its line, mm, to its left where positive: the first `offset` mm from the
/// line, the second `thickness` mm (its build-up's) further to the right where `right`, else to the left. Faces
/// centred on the line to the millimetre are on it: an inner wall of an odd number of millimetres drawn on its axis
/// has its offset rounded. None for a wall without a build-up or thinner than a millimetre; a separation line's faces
/// lie 1 mm either side of it.
pub fn faces(offset: i64, right: bool, thickness: Option<f64>, separation: bool) -> Option<(f64, f64)> {
    if separation {
        return Some((-1.0, 1.0));
    }
    let thickness = thickness?;
    let sense = if right { -1.0 } else { 1.0 };
    let first = offset as f64;
    let (f0, f1) = (first, first + sense * thickness);
    let (f0, f1) = (f0.min(f1), f0.max(f1));
    let mid = (f0 + f1) / 2.0;
    let (f0, f1) = if mid.abs() <= 1.0 { (f0 - mid, f1 - mid) } else { (f0, f1) };
    (thickness >= 1.0).then_some((f0, f1))
}

/// The footprint of the drawn wall `wall` among the storey's walls `walls` (which may hold it too), its corners
/// rounded to whole millimetres less `origin`: the band between its faces, cut at each end square or along the mitre
/// with the wall meeting it there, so that a wall shorter than its mitres is a triangle, never folded over itself.
pub fn drawn(wall: &Line, walls: &[Line], origin: Point) -> Option<Footprint> {
    let (a, b) = (wall.line[0], wall.line[1]);
    let len = libm::hypot((b[0] - a[0]) as f64, (b[1] - a[1]) as f64);
    if len < 1.0 {
        return None;
    }
    let along = [(b[0] - a[0]) as f64 / len, (b[1] - a[1]) as f64 / len];
    let across = [-along[1], along[0]];
    let base = [(a[0] - origin[0]) as f64, (a[1] - origin[1]) as f64];
    let faces = wall.faces?;
    let local = |p: [f64; 2]| [p[0] - origin[0] as f64, p[1] - origin[1] as f64];
    let point = |s: f64, t: f64| [base[0] + along[0] * s + across[0] * t, base[1] + along[1] * s + across[1] * t];
    let reach = len + 10.0 * (faces.1 - faces.0);
    let band =
        vec![point(-reach, faces.0), point(len + reach, faces.0), point(len + reach, faces.1), point(-reach, faces.1)];
    let cut = |end: bool| -> Half {
        // Its own side: back along it from the end, far beyond any mitre.
        let e = if end { point(len, 0.0) } else { base };
        let inward = if end { [-along[0], -along[1]] } else { along };
        let keep = [e[0] + inward[0] * 10_000.0, e[1] + inward[1] * 10_000.0];
        let (r, l) = match mitre(wall, walls, end, faces) {
            Some((r, l)) => (local(r), local(l)),
            None => (point(if end { len } else { 0.0 }, faces.0), point(if end { len } else { 0.0 }, faces.1)),
        };
        let n = [-(l[1] - r[1]), l[0] - r[0]];
        let side = n[0] * (keep[0] - r[0]) + n[1] * (keep[1] - r[1]);
        let n = if side < 0.0 { [-n[0], -n[1]] } else { n };
        Half { a: n[0], b: n[1], c: -(n[0] * r[0] + n[1] * r[1]) }
    };
    let ring = clip(band, &[cut(false), cut(true)])?;
    let on = |p: &[f64; 2]| (p[0] - base[0]) * along[0] + (p[1] - base[1]) * along[1];
    let span = ring.iter().fold((f64::MAX, f64::MIN), |(lo, hi), p| (lo.min(on(p)), hi.max(on(p))));
    let mut corners: Vec<Point> = ring.iter().map(|p| [p[0].round() as i64, p[1].round() as i64]).collect();
    corners.dedup();
    while corners.len() > 1 && corners.first() == corners.last() {
        corners.pop();
    }
    (corners.len() >= 3).then_some(Footprint { corners, along, across, span, faces })
}

/// The footprint of a wall of elements along the line `line`: the rectangle along and across the line around
/// `corners`, its elements' plates' corners in plan less `origin` ([`plate_corners`]), rounded to whole millimetres;
/// none where they span less than a millimetre either way.
pub fn of_elements(line: [Point; 2], corners: &[[f64; 2]], origin: Point) -> Option<Footprint> {
    let (a, b) = (line[0], line[1]);
    let len = libm::hypot((b[0] - a[0]) as f64, (b[1] - a[1]) as f64);
    if len < 1.0 {
        return None;
    }
    let along = [(b[0] - a[0]) as f64 / len, (b[1] - a[1]) as f64 / len];
    let across = [-along[1], along[0]];
    let base = [(a[0] - origin[0]) as f64, (a[1] - origin[1]) as f64];
    let at = |s: f64, t: f64| {
        [
            (base[0] + along[0] * s + across[0] * t).round() as i64,
            (base[1] + along[1] * s + across[1] * t).round() as i64,
        ]
    };
    let on = |p: &[f64; 2], d: [f64; 2]| (p[0] - base[0]) * d[0] + (p[1] - base[1]) * d[1];
    let (lo, hi) =
        corners.iter().fold((f64::MAX, f64::MIN), |(lo, hi), p| (lo.min(on(p, along)), hi.max(on(p, along))));
    let faces = corners.iter().fold((f64::MAX, f64::MIN), |(lo, hi), p| (lo.min(on(p, across)), hi.max(on(p, across))));
    if hi - lo < 1.0 || faces.1 - faces.0 < 1.0 {
        return None;
    }
    let corners = vec![at(lo, faces.0), at(hi, faces.0), at(hi, faces.1), at(lo, faces.1)];
    Some(Footprint { corners, along, across, span: (lo, hi), faces })
}

/// A plate's corners in plan, less `origin`, mm: each corner of its outline on its plane's face and `thickness` mm
/// against the normal.
pub fn plate_corners(frame: &PlaneFrame, outline: &[Point], thickness: i64, origin: Point) -> Vec<[f64; 2]> {
    let mut out = Vec::with_capacity(2 * outline.len());
    for q in outline {
        let p = frame.at(*q);
        let p = [p[0] - origin[0] as f64, p[1] - origin[1] as f64];
        out.push(p);
        out.push([p[0] - frame.normal[0] * thickness as f64, p[1] - frame.normal[1] * thickness as f64]);
    }
    out
}

/// The mitre at a drawn wall's start or end: where its faces (`faces`) meet the faces of the one other drawn wall
/// whose end lies there at an angle, its right face's point and its left face's, so that the two footprints close the
/// corner between them without overlapping, inside and outside it. None where no such wall ends there, or several do,
/// or the two run exactly in line, or the corner is too sharp to mitre.
fn mitre(wall: &Line, walls: &[Line], end: bool, faces: (f64, f64)) -> Option<([f64; 2], [f64; 2])> {
    let float = |p: Point| [p[0] as f64, p[1] as f64];
    let unit = |a: [f64; 2], b: [f64; 2]| {
        let len = libm::hypot(b[0] - a[0], b[1] - a[1]);
        [(b[0] - a[0]) / len, (b[1] - a[1]) / len]
    };
    let (a, b) = (float(wall.line[0]), float(wall.line[1]));
    let e = if end { b } else { a };
    let near = |p: [f64; 2]| libm::hypot(p[0] - e[0], p[1] - e[1]) <= MEET;
    let built = !wall.separation;
    let others: Vec<(&Line, bool)> = walls
        .iter()
        .filter(|w| w.id != wall.id && !w.elements)
        // A separation line is no wall to mitre with: a built wall's end is cut as if it were not there.
        .filter(|w| !built || !w.separation)
        .filter_map(|w| {
            if near(float(w.line[0])) {
                Some((w, false))
            } else if near(float(w.line[1])) {
                Some((w, true))
            } else {
                None
            }
        })
        .collect();
    let [(other, at_its_end)] = others[..] else { return None };
    let theirs = other.faces?;
    // The corner as a path in along this wall and out along the other, the faces across it to its left.
    let along = unit(a, b);
    let u = if end { along } else { [-along[0], -along[1]] };
    let (c0, c1) = if end { faces } else { (-faces.1, -faces.0) };
    let v = unit(float(other.line[0]), float(other.line[1]));
    let v = if at_its_end { [-v[0], -v[1]] } else { v };
    let (d0, d1) = if at_its_end { (-theirs.1, -theirs.0) } else { theirs };
    // Walls in line meet square; a bend of a fraction of a degree is mitred too, or their square ends would step.
    let turn = u[0] * v[1] - u[1] * v[0];
    if turn.abs() < 1e-9 {
        return None;
    }
    // Where this wall's face at `c` across the path meets the other's at `d`.
    let meet = |c: f64, d: f64| {
        let p = [e[0] - u[1] * c, e[1] + u[0] * c];
        let q = [e[0] - v[1] * d, e[1] + v[0] * d];
        let r = [q[0] - p[0], q[1] - p[1]];
        let s = (r[0] * v[1] - r[1] * v[0]) / turn;
        [p[0] + u[0] * s, p[1] + u[1] * s]
    };
    let (right, left) = (meet(c0, d0), meet(c1, d1));
    // Square where the corner is so sharp that a mitre would reach more than three walls' thickness from it: the two
    // footprints then overlap at the corner instead.
    let limit = 3.0 * (faces.1 - faces.0).max(theirs.1 - theirs.0);
    let far = |x: [f64; 2]| libm::hypot(x[0] - e[0], x[1] - e[1]) > limit;
    if far(right) || far(left) {
        return None;
    }
    // The path's right face is this wall's first where the path runs along it, its second where against it.
    Some(if end { (right, left) } else { (left, right) })
}

/// A half-plane a·x + b·y + c ≥ 0.
struct Half {
    a: f64,
    b: f64,
    c: f64,
}

impl Half {
    fn at(&self, p: [f64; 2]) -> f64 {
        self.a * p[0] + self.b * p[1] + self.c
    }
}

/// A ring cut by half-planes; none where fewer than three points are left after a cut.
fn clip(mut ring: Vec<[f64; 2]>, halves: &[Half]) -> Option<Vec<[f64; 2]>> {
    for half in halves {
        let n = ring.len();
        let mut out = Vec::with_capacity(n + 2);
        for i in 0..n {
            let (p, q) = (ring[i], ring[(i + 1) % n]);
            let (dp, dq) = (half.at(p), half.at(q));
            if dp >= 0.0 {
                out.push(p);
            }
            if (dp >= 0.0) != (dq >= 0.0) {
                let t = dp / (dp - dq);
                out.push([p[0] + t * (q[0] - p[0]), p[1] + t * (q[1] - p[1])]);
            }
        }
        ring = out;
        if ring.len() < 3 {
            return None;
        }
    }
    Some(ring)
}
