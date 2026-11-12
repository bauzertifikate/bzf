// SPDX-License-Identifier: Apache-2.0 OR MIT
//! The roof over a building part as planes over regions of the plan, from the roof plates or from the roof as drawn:
//! how high it stands over a point, or along a wall, above the wall's foot, so that walls stop under it (and
//! a wall standing on a plate or rising through one does not stop at it).

use super::{P2, P3};
use crate::document::{Part, Plane, Plate, Point, Roof};

type V3 = P3;

fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn cross(a: V3, b: V3) -> V3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

fn len(a: V3) -> f64 {
    (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt()
}

/// Whether `p` lies inside the ring (even-odd).
pub(crate) fn inside(p: P2, ring: &[P2]) -> bool {
    let mut odd = false;
    let n = ring.len();
    for i in 0..n {
        let (a, b) = (ring[i], ring[(i + 1) % n]);
        if (a[1] > p[1]) != (b[1] > p[1]) && p[0] < a[0] + (p[1] - a[1]) / (b[1] - a[1]) * (b[0] - a[0]) {
            odd = !odd;
        }
    }
    odd
}

/// The distance from `p` to the segment a–b.
pub(crate) fn segment_distance(p: P2, a: P2, b: P2) -> f64 {
    let d = [b[0] - a[0], b[1] - a[1]];
    let l = d[0] * d[0] + d[1] * d[1];
    let t = if l > 0.0 { (((p[0] - a[0]) * d[0] + (p[1] - a[1]) * d[1]) / l).clamp(0.0, 1.0) } else { 0.0 };
    let q = [a[0] + d[0] * t - p[0], a[1] + d[1] * t - p[1]];
    (q[0] * q[0] + q[1] * q[1]).sqrt()
}

/// A plane as its height over the plan, mm: z = a·x + b·y + c.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Height {
    /// Its rise along x.
    pub a: f64,
    /// Its rise along y.
    pub b: f64,
    /// Its height over the origin, mm.
    pub c: f64,
}

impl Height {
    /// The plane through three points; none for a vertical plane or points on one line.
    pub fn through(p: [V3; 3]) -> Option<Height> {
        let n = cross(sub(p[1], p[0]), sub(p[2], p[0]));
        if n[2].abs() < 1e-9 * len(n).max(1.0) {
            return None;
        }
        let (a, b) = (-n[0] / n[2], -n[1] / n[2]);
        Some(Height { a, b, c: p[0][2] - a * p[0][0] - b * p[0][1] })
    }

    /// A plane of the document; none for a vertical one.
    pub fn of(plane: &Plane) -> Option<Height> {
        Height::through(plane.points.map(|q| q.map(|c| c as f64)))
    }

    /// Its height over a point, mm.
    pub fn at(&self, p: P2) -> f64 {
        self.a * p[0] + self.b * p[1] + self.c
    }
}

/// The roof over a part: planes, each over its region in plan or over everything, the lowest one standing over each
/// point (a shape's planes over its rectangle, free planes), or each face over its own region only (faces, plates).
#[derive(Clone, Debug, Default)]
pub struct Surface {
    /// The planes and where each stands.
    pub faces: Vec<(Height, Option<Vec<P2>>)>,
    /// The rectangle a shape covers, mm.
    pub rect: Option<[P2; 2]>,
    /// The holes through each face's region, in the order of `faces`: a roof plate's openings, under which nothing of
    /// it stands. The plates' faces come first; a roof as drawn has none.
    pub holes: Vec<Vec<Vec<P2>>>,
}

/// A roof this little above a wall's foot, mm, does not stand over the wall: the wall stands on it or through it.
pub(crate) const OVER: f64 = 1.0;

impl Surface {
    /// The roof over the part: its roof plates' lower faces, each over its own outline in plan, and its roof as drawn;
    /// none where it has neither. Where a part has both, the lowest of them stands over each point: a wall under a
    /// plate ends at its underside, so that the two solids do not run into each other, and one under the drawn roof's
    /// surface where no plate's underside reaches (the strip under a plate's edge) ends there.
    pub fn of(part: &Part) -> Option<Surface> {
        let (mut faces, holes): (Vec<_>, Vec<_>) = part.roofs.iter().filter_map(|r| lower(&r.plate)).unzip();
        let drawn = part.roof.as_ref().and_then(Surface::drawn);
        let rect = drawn.as_ref().and_then(|d| d.rect);
        faces.extend(drawn.into_iter().flat_map(|d| d.faces));
        (!faces.is_empty()).then_some(Surface { faces, rect, holes })
    }

    /// A roof as drawn: for a shape or free planes the lowest of its planes over each point, for faces each over its own
    /// region; none for a roof without planes.
    pub fn drawn(roof: &Roof) -> Option<Surface> {
        use Roof::*;
        let rect = match roof {
            Mono { rect, .. }
            | Gable { rect, .. }
            | Hip { rect, .. }
            | HalfHip { rect, .. }
            | Mansard { rect, .. }
            | Pyramid { rect, .. } => {
                Some([[rect.x[0] as f64, rect.y[0] as f64], [rect.x[1] as f64, rect.y[1] as f64]])
            }
            Planes { .. } | Faces { .. } => None,
        };
        let faces: Vec<(Height, Option<Vec<P2>>)> = match roof {
            Faces { faces } => faces
                .iter()
                .filter_map(|f| {
                    let region = f.outline.iter().map(|p| [p[0] as f64, p[1] as f64]).collect();
                    Some((Height::of(&f.plane)?, Some(region)))
                })
                .collect(),
            _ => roof.planes().ok()?.iter().filter_map(|p| Some((Height::of(p)?, None))).collect(),
        };
        (!faces.is_empty()).then_some(Surface { faces, rect, holes: Vec::new() })
    }

    /// Its height over a point for a wall whose foot is at `foot`, mm: the lowest of its faces that stand over the
    /// point higher than the foot. A plate at or under the foot is none of the wall's roof: the wall stands on it (an
    /// Attika on a flat roof, a storey set back on a roof terrace) or rises through it (a dormer's cheeks), and the
    /// next one up, or nothing, stands over it. The planes of a shape are one roof, the lowest of them over each point;
    /// that one counts or none. None where nothing stands over the point above the foot.
    pub fn above(&self, p: P2, foot: f64) -> Option<f64> {
        let over = |k: usize, region: &Vec<P2>| {
            (inside(p, region) || on_edge(p, region))
                && !self.holes.get(k).is_some_and(|holes| holes.iter().any(|h| inside(p, h)))
        };
        let own = (self.faces.iter().enumerate())
            .filter(|(k, (_, region))| region.as_ref().is_some_and(|r| over(*k, r)))
            .map(|(_, (h, _))| h.at(p))
            .filter(|h| *h > foot + OVER)
            .reduce(f64::min);
        let shape = (self.faces.iter().filter(|(_, region)| region.is_none()).map(|(h, _)| h.at(p)))
            .reduce(f64::min)
            .filter(|h| *h > foot + OVER);
        [own, shape].into_iter().flatten().reduce(f64::min)
    }

    /// Its profile along the segment a–b for a wall whose foot is at `foot`, mm ([`Surface::above`]): the distances
    /// along it, mm, where the profile bends or steps, each with the height just before and just after; none where the
    /// roof stands nowhere over it.
    pub fn along(&self, a: P2, b: P2, foot: f64) -> Vec<(f64, Option<f64>, Option<f64>)> {
        let d = [b[0] - a[0], b[1] - a[1]];
        let l = (d[0] * d[0] + d[1] * d[1]).sqrt();
        if l < 1.0 {
            return Vec::new();
        }
        let at = |t: f64| [a[0] + d[0] * t / l, a[1] + d[1] * t / l];
        let mut ts = vec![0.0, l];
        // Where two planes cross over the line.
        for (i, (h, _)) in self.faces.iter().enumerate() {
            for (g, _) in &self.faces[i + 1..] {
                let (h0, h1) = (h.at(a) - g.at(a), h.at(b) - g.at(b));
                if (h0 < 0.0) != (h1 < 0.0) && (h1 - h0).abs() > 1e-9 {
                    ts.push(l * h0 / (h0 - h1));
                }
            }
        }
        // Where a plane passes the foot, and counts from there on or no longer.
        for (h, _) in &self.faces {
            let (h0, h1) = (h.at(a) - foot - OVER, h.at(b) - foot - OVER);
            if (h0 < 0.0) != (h1 < 0.0) && (h1 - h0).abs() > 1e-9 {
                ts.push(l * h0 / (h0 - h1));
            }
        }
        // Where the line crosses a region's edge, or a hole's.
        let holes = self.holes.iter().flatten();
        for r in self.faces.iter().filter_map(|(_, region)| region.as_ref()).chain(holes) {
            for i in 0..r.len() {
                let (p, q) = (r[i], r[(i + 1) % r.len()]);
                let e = [q[0] - p[0], q[1] - p[1]];
                let den = d[0] * e[1] - d[1] * e[0];
                if den.abs() < 1e-9 {
                    continue;
                }
                let s = ((p[0] - a[0]) * e[1] - (p[1] - a[1]) * e[0]) / den;
                let u = ((p[0] - a[0]) * d[1] - (p[1] - a[1]) * d[0]) / den;
                if (0.0..=1.0).contains(&s) && (-1e-9..=1.0 + 1e-9).contains(&u) {
                    ts.push(s * l);
                }
            }
        }
        ts.sort_by(f64::total_cmp);
        ts.dedup_by(|x, y| (*x - *y).abs() < 0.5);
        let eps = 0.25;
        ts.iter()
            .map(|&t| {
                // Just before and after; at the wall's ends what stands over the end itself. Where the roof ends
                // along the wall (its edge, a hole, a plate dipping under the foot) nothing stands beyond, and the
                // profile steps there.
                let here = self.above(at(t), foot);
                let before = if t > 0.0 { self.above(at((t - eps).max(0.0)), foot) } else { here };
                let after = if t < l { self.above(at((t + eps).min(l)), foot) } else { here };
                (t, before, after)
            })
            .collect()
    }
}

fn on_edge(p: P2, ring: &[P2]) -> bool {
    (0..ring.len()).any(|i| segment_distance(p, ring[i], ring[(i + 1) % ring.len()]) < 0.5)
}

/// A plate's lower face as a height over its outline in plan, and its holes in plan; none for a vertical plate.
#[allow(clippy::type_complexity)]
fn lower(plate: &Plate) -> Option<((Height, Option<Vec<P2>>), Vec<Vec<P2>>)> {
    let f = plate.frame()?;
    let down = f.normal.map(|c| -c * plate.thickness as f64);
    let corner = |s: f64, t: f64| [0, 1, 2].map(|k| f.origin[k] + f.u[k] * s + f.v[k] * t + down[k]);
    let h = Height::through([corner(0.0, 0.0), corner(1000.0, 0.0), corner(0.0, 1000.0)])?;
    let plan = |ring: &Vec<Point>| -> Vec<P2> {
        ring.iter().map(|q| corner(q[0] as f64, q[1] as f64)).map(|p| [p[0], p[1]]).collect()
    };
    Some(((h, Some(plan(&plate.outline))), plate.holes.iter().map(plan).collect()))
}
