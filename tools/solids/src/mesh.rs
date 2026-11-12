// SPDX-License-Identifier: Apache-2.0 OR MIT
//! A solid's faces from the pieces it is built of: the pieces' polygons with their points made one where they meet,
//! polygons in one plane joined into faces with their holes, and points no corner needs left out, so that every edge
//! of the solid lies in exactly two faces, once each way.

use super::{Face, P3};
use std::collections::{BTreeMap, BTreeSet};

/// Points this close, mm, are one point.
const WELD: f64 = 1e-6;
/// A polygon whose points lie this close to a face's plane, mm, lies in it.
const PLANE: f64 = 1e-5;

/// Polygons over shared points.
#[derive(Default)]
pub(crate) struct Mesh {
    points: Vec<P3>,
    rings: Vec<Vec<usize>>,
}

fn sub(a: P3, b: P3) -> P3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn dot(a: P3, b: P3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: P3, b: P3) -> P3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

fn len(a: P3) -> f64 {
    dot(a, a).sqrt()
}

/// Twice the vector area of a ring (Newell): its normal, as long as twice its area.
pub fn newell(ring: &[P3]) -> P3 {
    let mut n = [0.0; 3];
    for i in 0..ring.len() {
        let (a, b) = (ring[i], ring[(i + 1) % ring.len()]);
        n[0] += (a[1] - b[1]) * (a[2] + b[2]);
        n[1] += (a[2] - b[2]) * (a[0] + b[0]);
        n[2] += (a[0] - b[0]) * (a[1] + b[1]);
    }
    n
}

/// How far `p` lies from the segment a–b, and where along it from 0 to 1.
fn off(p: P3, a: P3, b: P3) -> (f64, f64) {
    let d = sub(b, a);
    let l = dot(d, d);
    let t = if l > 0.0 { dot(sub(p, a), d) / l } else { 0.0 };
    let q = [0, 1, 2].map(|k| a[k] + d[k] * t.clamp(0.0, 1.0));
    (len(sub(p, q)), t)
}

/// A plane with the polygons in it.
struct Sheet {
    normal: P3,
    at: f64,
    rings: Vec<usize>,
}

impl Mesh {
    /// Adds a polygon, counter-clockwise seen from outside the solid.
    pub(crate) fn add(&mut self, ring: &[P3]) {
        let mut ids: Vec<usize> = Vec::with_capacity(ring.len());
        for p in ring {
            let found = self.points.iter().position(|q| (0..3).all(|k| (p[k] - q[k]).abs() <= WELD));
            let id = found.unwrap_or_else(|| {
                self.points.push(*p);
                self.points.len() - 1
            });
            if ids.last() != Some(&id) {
                ids.push(id);
            }
        }
        while ids.len() > 1 && ids.first() == ids.last() {
            ids.pop();
        }
        if ids.len() >= 3 {
            self.rings.push(ids);
        }
    }

    /// The solid's faces.
    pub(crate) fn faces(mut self) -> Vec<Face> {
        self.meet();
        let sheets = self.sheets();
        let mut faces: Vec<(P3, Vec<Vec<usize>>)> = Vec::new();
        for sheet in &sheets {
            faces.extend(self.joined(sheet).into_iter().map(|rings| (sheet.normal, rings)));
        }
        // A point between two others on one line, at which nothing else meets, is no corner.
        let mut next: BTreeMap<usize, BTreeSet<usize>> = BTreeMap::new();
        for ring in faces.iter().flat_map(|(_, rings)| rings) {
            for i in 0..ring.len() {
                let (a, b) = (ring[i], ring[(i + 1) % ring.len()]);
                next.entry(a).or_default().insert(b);
                next.entry(b).or_default().insert(a);
            }
        }
        let spare: BTreeSet<usize> = next
            .iter()
            .filter(|(v, to)| {
                let mut to = to.iter();
                match (to.next(), to.next(), to.next()) {
                    (Some(&a), Some(&b), None) => {
                        let (d, t) = off(self.points[**v], self.points[a], self.points[b]);
                        d <= WELD && t > 0.0 && t < 1.0
                    }
                    _ => false,
                }
            })
            .map(|(v, _)| *v)
            .collect();
        faces
            .into_iter()
            .filter_map(|(normal, rings)| {
                let rings: Vec<Vec<P3>> = rings
                    .into_iter()
                    .map(|r| r.into_iter().filter(|v| !spare.contains(v)).map(|v| self.points[v]).collect::<Vec<P3>>())
                    .filter(|r| r.len() >= 3)
                    .collect();
                (!rings.is_empty()).then_some(Face { rings, normal })
            })
            .collect()
    }

    /// Puts every point that lies on a polygon's edge into that polygon, so that polygons meeting along a line share
    /// the same edges there.
    fn meet(&mut self) {
        for r in 0..self.rings.len() {
            let ring = std::mem::take(&mut self.rings[r]);
            let mut out = Vec::with_capacity(ring.len());
            for i in 0..ring.len() {
                let (a, b) = (ring[i], ring[(i + 1) % ring.len()]);
                out.push(a);
                let (pa, pb) = (self.points[a], self.points[b]);
                let (lo, hi) = ([0, 1, 2].map(|k| pa[k].min(pb[k]) - WELD), [0, 1, 2].map(|k| pa[k].max(pb[k]) + WELD));
                let mut on: Vec<(f64, usize)> = (0..self.points.len())
                    .filter(|&v| v != a && v != b)
                    .filter(|&v| (0..3).all(|k| self.points[v][k] >= lo[k] && self.points[v][k] <= hi[k]))
                    .filter_map(|v| {
                        let (d, t) = off(self.points[v], pa, pb);
                        (d <= WELD && t > 0.0 && t < 1.0).then_some((t, v))
                    })
                    .collect();
                on.sort_by(|x, y| x.0.total_cmp(&y.0));
                out.extend(on.into_iter().map(|(_, v)| v));
            }
            self.rings[r] = out;
        }
    }

    /// The polygons by the plane they lie in, the largest first; one without area is left out.
    fn sheets(&self) -> Vec<Sheet> {
        let mut order: Vec<(f64, usize, P3)> = (self.rings.iter().enumerate())
            .map(|(k, ring)| {
                let n = newell(&ring.iter().map(|&v| self.points[v]).collect::<Vec<P3>>());
                (len(n), k, n)
            })
            .filter(|(twice, _, _)| *twice > 1e-9)
            .collect();
        order.sort_by(|x, y| y.0.total_cmp(&x.0).then(x.1.cmp(&y.1)));
        let mut sheets: Vec<Sheet> = Vec::new();
        for (twice, k, n) in order {
            let normal = n.map(|c| c / twice);
            let ring = &self.rings[k];
            let found = sheets.iter_mut().find(|s| {
                dot(s.normal, normal) > 0.5
                    && ring.iter().all(|&v| (dot(s.normal, self.points[v]) - s.at).abs() <= PLANE)
            });
            match found {
                Some(s) => s.rings.push(k),
                None => sheets.push(Sheet { normal, at: dot(normal, self.points[ring[0]]), rings: vec![k] }),
            }
        }
        sheets
    }

    /// A plane's polygons joined: the edges two of them share fall away, and what is left runs around each face, its
    /// outline first and then its holes.
    fn joined(&self, sheet: &Sheet) -> Vec<Vec<Vec<usize>>> {
        let mut edges: BTreeSet<(usize, usize)> = BTreeSet::new();
        for &k in &sheet.rings {
            let ring = &self.rings[k];
            for i in 0..ring.len() {
                let (a, b) = (ring[i], ring[(i + 1) % ring.len()]);
                if a != b && !edges.remove(&(b, a)) {
                    edges.insert((a, b));
                }
            }
        }
        // The plane's own coordinates, counter-clockwise seen against its normal.
        let n = sheet.normal;
        let least = (0..3).min_by(|&i, &j| n[i].abs().total_cmp(&n[j].abs())).unwrap_or(0);
        let mut axis = [0.0; 3];
        axis[least] = 1.0;
        let u = cross(axis, n);
        let u = u.map(|c| c / len(u));
        let v = cross(n, u);
        let flat = |p: usize| [dot(self.points[p], u), dot(self.points[p], v)];
        // How far the way from `from` turns clockwise to the way `to`, both from one point: 0 to 4 for a full turn.
        let turn = |from: [f64; 2], to: [f64; 2]| {
            let l = ((from[0] * from[0] + from[1] * from[1]) * (to[0] * to[0] + to[1] * to[1])).sqrt().max(1e-300);
            let (c, s) = ((from[0] * to[0] + from[1] * to[1]) / l, (from[0] * to[1] - from[1] * to[0]) / l);
            let ccw = if s >= 0.0 { 1.0 - c } else { 3.0 + c };
            if ccw <= 1e-12 { 4.0 } else { 4.0 - ccw }
        };
        let mut loops: Vec<Vec<usize>> = Vec::new();
        while let Some(&(start, first)) = edges.iter().next() {
            edges.remove(&(start, first));
            let mut ring = vec![start];
            let (mut before, mut here) = (start, first);
            while here != start {
                ring.push(here);
                let (p, back) = (flat(here), flat(before));
                let back = [back[0] - p[0], back[1] - p[1]];
                let onward = edges
                    .range((here, 0)..=(here, usize::MAX))
                    .map(|&(_, to)| {
                        let q = flat(to);
                        (turn(back, [q[0] - p[0], q[1] - p[1]]), to)
                    })
                    .min_by(|x, y| x.0.total_cmp(&y.0));
                let Some((_, to)) = onward else { break };
                edges.remove(&(here, to));
                (before, here) = (here, to);
            }
            if ring.len() >= 3 {
                loops.push(ring);
            }
        }
        let twice = |ring: &[usize]| -> f64 {
            (0..ring.len())
                .map(|i| {
                    let (a, b) = (flat(ring[i]), flat(ring[(i + 1) % ring.len()]));
                    a[0] * b[1] - b[0] * a[1]
                })
                .sum()
        };
        let (outer, holes): (Vec<_>, Vec<_>) = loops.into_iter().partition(|r| twice(r) > 0.0);
        let mut faces: Vec<Vec<Vec<usize>>> = outer.into_iter().map(|r| vec![r]).collect();
        for hole in holes {
            let (a, b) = (flat(hole[0]), flat(hole[1]));
            let mid = [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0];
            let around = (faces.iter().enumerate())
                .filter(|(_, f)| super::over::inside(mid, &f[0].iter().map(|&p| flat(p)).collect::<Vec<_>>()))
                .min_by(|x, y| twice(&x.1[0]).total_cmp(&twice(&y.1[0])))
                .map(|(k, _)| k);
            match around {
                Some(k) => faces[k].push(hole),
                None => faces.push(vec![hole]),
            }
        }
        faces
    }
}

/// Whether the faces close a solid: every edge runs in exactly two faces, once each way.
pub fn closed(faces: &[Face]) -> bool {
    let key = |p: P3| p.map(|c| (c + 0.0).to_bits());
    let mut edges: BTreeMap<([u64; 3], [u64; 3]), usize> = BTreeMap::new();
    for ring in faces.iter().flat_map(|f| &f.rings) {
        for i in 0..ring.len() {
            *edges.entry((key(ring[i]), key(ring[(i + 1) % ring.len()]))).or_default() += 1;
        }
    }
    !edges.is_empty() && edges.iter().all(|((a, b), n)| *n == 1 && edges.get(&(*b, *a)) == Some(&1))
}

/// The volume the faces enclose, mm³: positive where their rings run counter-clockwise seen from outside.
pub fn volume(faces: &[Face]) -> f64 {
    let Some(origin) = faces.iter().flat_map(|f| &f.rings).flat_map(|r| r.first()).next().copied() else { return 0.0 };
    let mut six = 0.0;
    for ring in faces.iter().flat_map(|f| &f.rings) {
        let p0 = sub(ring[0], origin);
        for i in 1..ring.len().saturating_sub(1) {
            six += dot(p0, cross(sub(ring[i], origin), sub(ring[i + 1], origin)));
        }
    }
    six / 6.0
}
