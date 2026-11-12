// SPDX-License-Identifier: Apache-2.0 OR MIT
//! The solid over a convex piece of the plan (a wall's footprint, a layer's band, an opening's stretch of it): from its
//! bottom up to its top, or to the roof where the roof stands lower over it, less the boxes cut out of it. The piece is
//! divided into cells over each of which one plane is the top and the same boxes are cut out; the solid's faces are the
//! cells' tops and bottoms and, along every edge of a cell, what only one of the two cells beside it fills.

use super::mesh::Mesh;
use super::over::{Height, OVER, Surface, inside};
use super::{Face, P2};
use std::collections::BTreeMap;

/// Points this close in plan, mm, or heights this close, mm, are the same.
const ON: f64 = 1e-6;

/// A box cut out of a column: between two lines across the plan and between two heights.
pub(crate) struct Void {
    /// How far along a point of the plan lies, mm; the box reaches from `from` to `to` of it.
    pub along: Height,
    pub from: f64,
    pub to: f64,
    /// Its bottom and top, mm.
    pub z0: f64,
    pub z1: f64,
}

/// A convex piece of the plan, counter-clockwise, from `bottom` up to `top`, mm, under the roof `over` as it stands
/// over a wall whose foot is at `foot`, less `voids`.
pub(crate) struct Column<'a> {
    pub ring: Vec<P2>,
    pub bottom: f64,
    pub top: f64,
    pub over: Option<&'a Surface>,
    pub foot: f64,
    pub voids: &'a [Void],
}

/// A cell and what stands over it: each piece from one height to another.
type Cell = (Vec<P2>, Vec<(Height, Height)>);

fn flat(z: f64) -> Height {
    Height { a: 0.0, b: 0.0, c: z }
}

/// `f` less `g`.
fn less(f: &Height, g: &Height) -> Height {
    Height { a: f.a - g.a, b: f.b - g.b, c: f.c - g.c }
}

fn twice_area(ring: &[P2]) -> f64 {
    (0..ring.len())
        .map(|i| {
            let (a, b) = (ring[i], ring[(i + 1) % ring.len()]);
            a[0] * b[1] - b[0] * a[1]
        })
        .sum()
}

fn centre(ring: &[P2]) -> P2 {
    let n = ring.len().max(1) as f64;
    [ring.iter().map(|p| p[0]).sum::<f64>() / n, ring.iter().map(|p| p[1]).sum::<f64>() / n]
}

/// The parts of a convex ring where `f` is below zero and where it is not; a ring on which it is nowhere below zero
/// stays whole in the second, also where it is zero all over.
fn split(ring: &[P2], f: &Height) -> (Option<Vec<P2>>, Option<Vec<P2>>) {
    let on = ON * (f.a * f.a + f.b * f.b).sqrt().max(1.0);
    let v: Vec<f64> = ring.iter().map(|p| f.at(*p)).collect();
    if !v.iter().any(|x| *x < -on) {
        return (None, Some(ring.to_vec()));
    }
    if !v.iter().any(|x| *x > on) {
        return (Some(ring.to_vec()), None);
    }
    let (mut below, mut rest) = (Vec::new(), Vec::new());
    for i in 0..ring.len() {
        let j = (i + 1) % ring.len();
        if v[i] <= on {
            below.push(ring[i]);
        }
        if v[i] >= -on {
            rest.push(ring[i]);
        }
        if (v[i] < -on && v[j] > on) || (v[i] > on && v[j] < -on) {
            let t = v[i] / (v[i] - v[j]);
            let p = [ring[i][0] + (ring[j][0] - ring[i][0]) * t, ring[i][1] + (ring[j][1] - ring[i][1]) * t];
            below.push(p);
            rest.push(p);
        }
    }
    let whole = |r: Vec<P2>| (r.len() >= 3 && twice_area(&r) > ON).then_some(r);
    (whole(below), whole(rest))
}

/// Both parts of a ring split by `f`.
fn both(ring: &[P2], f: &Height) -> impl Iterator<Item = Vec<P2>> + use<> {
    let (below, rest) = split(ring, f);
    [below, rest].into_iter().flatten()
}

/// Whether the segment a–b runs through the inside of a convex ring.
fn crosses(ring: &[P2], a: P2, b: P2) -> bool {
    let (mut t0, mut t1) = (0.0_f64, 1.0_f64);
    for i in 0..ring.len() {
        let (p, q) = (ring[i], ring[(i + 1) % ring.len()]);
        let e = [q[0] - p[0], q[1] - p[1]];
        let l = (e[0] * e[0] + e[1] * e[1]).sqrt();
        if l < 1e-12 {
            continue;
        }
        let inside = |x: P2| (e[0] * (x[1] - p[1]) - e[1] * (x[0] - p[0])) / l - ON;
        let (fa, fb) = (inside(a), inside(b));
        if fa <= 0.0 && fb <= 0.0 {
            return false;
        }
        if fa < 0.0 {
            t0 = t0.max(fa / (fa - fb));
        } else if fb < 0.0 {
            t1 = t1.min(fa / (fa - fb));
        }
    }
    (t1 - t0) * ((b[0] - a[0]) * (b[0] - a[0]) + (b[1] - a[1]) * (b[1] - a[1])).sqrt() > ON
}

/// The pieces of a convex ring over each of which one of `planes` is the lowest, with that plane.
fn lowest(ring: Vec<P2>, planes: &[Height]) -> Vec<(Vec<P2>, Height)> {
    let Some(first) = planes.first() else { return Vec::new() };
    let mut out = vec![(ring, *first)];
    for g in &planes[1..] {
        out = out
            .into_iter()
            .flat_map(|(r, h)| {
                let (under, rest) = split(&r, &less(g, &h));
                [under.map(|r| (r, *g)), rest.map(|r| (r, h))].into_iter().flatten()
            })
            .collect();
    }
    out
}

impl Column<'_> {
    /// The solid's faces; none where nothing of it stands.
    pub(crate) fn faces(&self) -> Vec<Face> {
        if self.ring.len() < 3 || self.top <= self.bottom {
            return Vec::new();
        }
        let mut ring = self.ring.clone();
        if twice_area(&ring) < 0.0 {
            ring.reverse();
        }
        // Cells no edge of a roof face runs through.
        let mut cells = vec![ring];
        if let Some(s) = self.over {
            let regions = s.faces.iter().filter_map(|(_, region)| region.as_ref());
            for r in regions.chain(s.holes.iter().flatten()) {
                for i in 0..r.len() {
                    let (a, b) = (r[i], r[(i + 1) % r.len()]);
                    let l = ((b[0] - a[0]) * (b[0] - a[0]) + (b[1] - a[1]) * (b[1] - a[1])).sqrt();
                    if l < ON {
                        continue;
                    }
                    let (x, y) = (-(b[1] - a[1]) / l, (b[0] - a[0]) / l);
                    let line = Height { a: x, b: y, c: -(x * a[0] + y * a[1]) };
                    cells = cells
                        .into_iter()
                        .flat_map(|c| if crosses(&c, a, b) { both(&c, &line).collect() } else { vec![c] })
                        .collect();
                }
            }
        }
        // Each with the one plane on top of it.
        let mut topped: Vec<(Vec<P2>, Height)> = cells.into_iter().flat_map(|c| self.tops(c)).collect();
        // And with the same boxes cut out all over.
        for v in self.voids {
            for bound in [v.from, v.to] {
                let f = Height { a: v.along.a, b: v.along.b, c: v.along.c - bound };
                topped = topped.into_iter().flat_map(|(r, h)| both(&r, &f).map(move |r| (r, h))).collect();
            }
        }
        let cells: Vec<Cell> = topped.into_iter().flat_map(|(r, h)| self.pieces(r, h)).collect();
        faces(&cells)
    }

    /// A cell's pieces by the plane on top: the column's own top, or the roof where it stands lower over a wall with
    /// this foot ([`Surface::above`]).
    fn tops(&self, cell: Vec<P2>) -> Vec<(Vec<P2>, Height)> {
        let own = flat(self.top);
        let Some(s) = self.over else { return vec![(cell, own)] };
        let c = centre(&cell);
        // A plane counts where it is higher than this.
        let limit = flat(self.foot + OVER);
        let mut pieces: Vec<(Vec<P2>, Vec<Height>)> = Vec::new();
        // A shape's planes are one roof: the lowest of them, where that one counts.
        let free: Vec<Height> = s.faces.iter().filter(|(_, region)| region.is_none()).map(|(h, _)| *h).collect();
        if free.is_empty() {
            pieces.push((cell, vec![own]));
        } else {
            for (ring, low) in lowest(cell, &free) {
                let (counts, not) = split(&ring, &less(&limit, &low));
                pieces.extend(counts.map(|r| (r, vec![own, low])));
                pieces.extend(not.map(|r| (r, vec![own])));
            }
        }
        // A face over its region counts for itself.
        for (k, (h, region)) in s.faces.iter().enumerate() {
            let Some(region) = region else { continue };
            if !inside(c, region) || s.holes.get(k).is_some_and(|holes| holes.iter().any(|x| inside(c, x))) {
                continue;
            }
            pieces = pieces
                .into_iter()
                .flat_map(|(ring, planes)| {
                    let (counts, not) = split(&ring, &less(&limit, h));
                    let mut with = planes.clone();
                    with.push(*h);
                    [counts.map(|r| (r, with)), not.map(|r| (r, planes))].into_iter().flatten()
                })
                .collect();
        }
        pieces.into_iter().flat_map(|(ring, planes)| lowest(ring, &planes)).collect()
    }

    /// What stands over a cell under the plane `top`: from the bottom up to it, less the boxes over the cell; the cell
    /// divided where the plane passes a box's bottom or top, or the column's bottom.
    fn pieces(&self, ring: Vec<P2>, top: Height) -> Vec<Cell> {
        let c = centre(&ring);
        let mut gaps: Vec<(f64, f64)> = (self.voids.iter())
            .filter(|v| {
                let s = v.along.at(c);
                s > v.from && s < v.to
            })
            .map(|v| (v.z0.max(self.bottom), v.z1))
            .filter(|(z0, z1)| z1 > z0)
            .collect();
        gaps.sort_by(|x, y| x.0.total_cmp(&y.0));
        let mut joined: Vec<(f64, f64)> = Vec::new();
        for g in gaps {
            match joined.last_mut() {
                Some(last) if g.0 <= last.1 => last.1 = last.1.max(g.1),
                _ => joined.push(g),
            }
        }
        let mut parts = vec![ring];
        for z in [self.bottom].into_iter().chain(joined.iter().flat_map(|g| [g.0, g.1])) {
            parts = parts.iter().flat_map(|r| both(r, &less(&top, &flat(z)))).collect();
        }
        parts
            .into_iter()
            .map(|part| {
                let t = top.at(centre(&part));
                let mut pieces = Vec::new();
                let mut from = self.bottom;
                for &(z0, z1) in &joined {
                    if z0 >= t {
                        break;
                    }
                    if z0 > from + 1e-9 {
                        pieces.push((flat(from), flat(z0)));
                    }
                    from = z1;
                }
                if t > from + 1e-9 {
                    pieces.push((flat(from), top));
                }
                (part, pieces)
            })
            .collect()
    }
}

/// The faces of the solid the cells' pieces make together.
fn faces(cells: &[Cell]) -> Vec<Face> {
    // The cells over shared points, each with the points of the others that lie on its edges.
    let mut points: Vec<P2> = Vec::new();
    let mut rings: Vec<Vec<usize>> = Vec::new();
    for (ring, _) in cells {
        let mut ids: Vec<usize> = Vec::new();
        for p in ring {
            let found = points.iter().position(|q| (p[0] - q[0]).abs() <= ON && (p[1] - q[1]).abs() <= ON);
            let id = found.unwrap_or_else(|| {
                points.push(*p);
                points.len() - 1
            });
            if ids.last() != Some(&id) {
                ids.push(id);
            }
        }
        while ids.len() > 1 && ids.first() == ids.last() {
            ids.pop();
        }
        rings.push(if ids.len() >= 3 { ids } else { Vec::new() });
    }
    for ring in rings.iter_mut() {
        let mut out = Vec::with_capacity(ring.len());
        for i in 0..ring.len() {
            let (a, b) = (ring[i], ring[(i + 1) % ring.len()]);
            out.push(a);
            let (pa, pb) = (points[a], points[b]);
            let d = [pb[0] - pa[0], pb[1] - pa[1]];
            let l2 = d[0] * d[0] + d[1] * d[1];
            let mut on: Vec<(f64, usize)> = (0..points.len())
                .filter(|&v| v != a && v != b)
                .filter_map(|v| {
                    let w = [points[v][0] - pa[0], points[v][1] - pa[1]];
                    let t = (w[0] * d[0] + w[1] * d[1]) / l2;
                    let off = [w[0] - d[0] * t, w[1] - d[1] * t];
                    (t > 0.0 && t < 1.0 && off[0].abs() <= ON && off[1].abs() <= ON).then_some((t, v))
                })
                .collect();
            on.sort_by(|x, y| x.0.total_cmp(&y.0));
            out.extend(on.into_iter().map(|(_, v)| v));
        }
        *ring = out;
    }
    let mut mesh = Mesh::default();
    // Tops and bottoms.
    for (ring, (_, pieces)) in rings.iter().zip(cells) {
        for (lo, hi) in pieces {
            let lift = |h: &Height, v: &usize| [points[*v][0], points[*v][1], h.at(points[*v])];
            mesh.add(&ring.iter().map(|v| lift(hi, v)).collect::<Vec<_>>());
            mesh.add(&ring.iter().rev().map(|v| lift(lo, v)).collect::<Vec<_>>());
        }
    }
    // Along each edge, what only the cell on one side of it fills. The cell to the left of an edge from its lower
    // point to its higher one comes first.
    let mut edges: BTreeMap<(usize, usize), [Option<usize>; 2]> = BTreeMap::new();
    for (k, ring) in rings.iter().enumerate() {
        for i in 0..ring.len() {
            let (a, b) = (ring[i], ring[(i + 1) % ring.len()]);
            if a != b {
                edges.entry((a.min(b), a.max(b))).or_default()[usize::from(a > b)] = Some(k);
            }
        }
    }
    for ((i, j), sides) in edges {
        let (p, q) = (points[i], points[j]);
        // Each side's pieces as the heights of their bottoms and tops at the edge's two ends.
        let spans = |side: Option<usize>| -> Vec<[[f64; 2]; 2]> {
            let pieces = side.map_or(&[][..], |k| &cells[k].1[..]);
            pieces.iter().map(|(lo, hi)| [[lo.at(p), lo.at(q)], [hi.at(p), hi.at(q)]]).collect()
        };
        let (left, right) = (spans(sides[0]), spans(sides[1]));
        let lines: Vec<[f64; 2]> = left.iter().chain(&right).flatten().copied().collect();
        // Where two of the lines cross between the ends.
        let mut ts = vec![0.0, 1.0];
        for (n, f) in lines.iter().enumerate() {
            for g in &lines[n + 1..] {
                let (d0, d1) = (f[0] - g[0], f[1] - g[1]);
                if (d0 < -ON && d1 > ON) || (d0 > ON && d1 < -ON) {
                    ts.push(d0 / (d0 - d1));
                }
            }
        }
        ts.sort_by(f64::total_cmp);
        ts.dedup_by(|x, y| (*x - *y).abs() < 1e-12);
        let at = |t: f64, line: &[f64; 2]| -> [f64; 3] {
            if t == 0.0 {
                [p[0], p[1], line[0]]
            } else if t == 1.0 {
                [q[0], q[1], line[1]]
            } else {
                [p[0] + (q[0] - p[0]) * t, p[1] + (q[1] - p[1]) * t, line[0] + (line[1] - line[0]) * t]
            }
        };
        for w in ts.windows(2) {
            let (ta, tb) = (w[0], w[1]);
            let mid = (ta + tb) / 2.0;
            let z = |line: &[f64; 2]| line[0] + (line[1] - line[0]) * mid;
            let mut levels: Vec<&[f64; 2]> = lines.iter().collect();
            levels.sort_by(|x, y| z(x).total_cmp(&z(y)));
            levels.dedup_by(|x, y| (z(x) - z(y)).abs() <= ON * 0.1);
            let fills = |spans: &[[[f64; 2]; 2]], h: f64| spans.iter().any(|s| z(&s[0]) < h && h < z(&s[1]));
            for pair in levels.windows(2) {
                let (lo, hi) = (pair[0], pair[1]);
                let h = (z(lo) + z(hi)) / 2.0;
                let (l, r) = (fills(&left, h), fills(&right, h));
                if l == r {
                    continue;
                }
                let quad = [at(ta, lo), at(tb, lo), at(tb, hi), at(ta, hi)];
                if l {
                    mesh.add(&quad);
                } else {
                    mesh.add(&[quad[1], quad[0], quad[3], quad[2]]);
                }
            }
        }
    }
    mesh.faces()
}

/// The part of a convex ring where `f` lies between `from` and `to`; none where nothing of it does.
pub(crate) fn between(ring: &[P2], f: &Height, from: f64, to: f64) -> Option<Vec<P2>> {
    let (_, rest) = split(ring, &Height { a: f.a, b: f.b, c: f.c - from });
    let (below, _) = split(&rest?, &Height { a: f.a, b: f.b, c: f.c - to });
    below
}
