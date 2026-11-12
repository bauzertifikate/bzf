// SPDX-License-Identifier: Apache-2.0 OR MIT
//! A roof as drawn, as its planes: generated from a shape, or given freely.

use crate::document::{Axis, Plane, Rect, RectSide, Roof};

/// Why a roof as drawn has no planes: the field at fault (`rect`, `top`, `ridgeAt`…) and the reason.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Invalid {
    /// The field of the roof at fault, by its name in the document.
    pub field: &'static str,
    /// What is wrong with it.
    pub reason: &'static str,
}

impl std::fmt::Display for Invalid {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}: {}", self.field, self.reason)
    }
}

impl std::error::Error for Invalid {}

fn invalid<T>(field: &'static str, reason: &'static str) -> Result<T, Invalid> {
    Err(Invalid { field, reason })
}

impl Roof {
    /// The roof's planes, from its shape. Every plane passes through three points of whole millimetres: where a plane
    /// reaches its height only halfway across, it is given by the point twice as far at twice the rise. For faces,
    /// their planes; whether the faces' regions are well formed is not checked here.
    pub fn planes(&self) -> Result<Vec<Plane>, Invalid> {
        let rising =
            |eaves: i64, top: i64| if eaves < top { Ok(()) } else { invalid("top", "must lie above the eaves") };
        match self {
            Roof::Planes { planes } => {
                if planes.is_empty() {
                    return invalid("planes", "a roof needs at least one plane");
                }
                Ok(planes.clone())
            }
            &Roof::Mono { rect, low, eaves, top } => {
                let [x0, x1, y0, y1] = corners(&rect)?;
                if eaves > top {
                    return invalid("top", "must not lie below the eaves");
                }
                let points = match low {
                    RectSide::YMin => [[x0, y0, eaves], [x1, y0, eaves], [x0, y1, top]],
                    RectSide::YMax => [[x1, y1, eaves], [x0, y1, eaves], [x1, y0, top]],
                    RectSide::XMin => [[x0, y1, eaves], [x0, y0, eaves], [x1, y1, top]],
                    RectSide::XMax => [[x1, y0, eaves], [x1, y1, eaves], [x0, y0, top]],
                };
                Ok(vec![Plane { points }])
            }
            &Roof::Gable { rect, ridge, eaves, top, ridge_at } => {
                let f = Frame::new(&rect, ridge)?;
                rising(eaves, top)?;
                let (u0, u1, v0, v1) = (f.u[0], f.u[1], f.v[0], f.v[1]);
                match ridge_at {
                    None => Ok(f.gable(eaves, top)),
                    Some(r) if v0 < r && r < v1 => Ok(vec![
                        f.plane([u0, v0, eaves], [u1, v0, eaves], [u0, r, top]),
                        f.plane([u1, v1, eaves], [u0, v1, eaves], [u1, r, top]),
                    ]),
                    Some(_) => invalid("ridgeAt", "must lie inside the rectangle, across the ridge"),
                }
            }
            &Roof::Hip { rect, ridge, eaves, top, hip_run } => {
                let f = Frame::new(&rect, ridge)?;
                rising(eaves, top)?;
                let (u0, u1, v0, v1) = (f.u[0], f.u[1], f.v[0], f.v[1]);
                let mut planes = f.gable(eaves, top);
                match hip_run {
                    // As steep as the long sides: the ridge's height half the depth in, so twice the rise a depth in.
                    None => {
                        let depth = v1 - v0;
                        if depth > u1 - u0 {
                            return invalid("ridge", "with equal pitches the ridge must run along the longer side");
                        }
                        let far = 2 * top - eaves;
                        planes.push(f.plane([u0, v1, eaves], [u0, v0, eaves], [u0 + depth, v1, far]));
                        planes.push(f.plane([u1, v0, eaves], [u1, v1, eaves], [u1 - depth, v0, far]));
                    }
                    Some(h) => {
                        if h <= 0 || 2 * h > u1 - u0 {
                            return invalid("hipRun", "must be greater than 0 and at most half the ridge's side");
                        }
                        planes.push(f.plane([u0, v1, eaves], [u0, v0, eaves], [u0 + h, v1, top]));
                        planes.push(f.plane([u1, v0, eaves], [u1, v1, eaves], [u1 - h, v0, top]));
                    }
                }
                Ok(planes)
            }
            &Roof::HalfHip { rect, ridge, eaves, top, hip_base, hip_run } => {
                let f = Frame::new(&rect, ridge)?;
                rising(eaves, top)?;
                if hip_base <= eaves || hip_base >= top {
                    return invalid("hipBase", "must lie between the eaves and the ridge");
                }
                let (u0, u1, v0, v1) = (f.u[0], f.u[1], f.v[0], f.v[1]);
                if hip_run <= 0 || 2 * hip_run > u1 - u0 {
                    return invalid("hipRun", "must be greater than 0 and at most half the ridge's side");
                }
                let mut planes = f.gable(eaves, top);
                planes.push(f.plane([u0, v1, hip_base], [u0, v0, hip_base], [u0 + hip_run, v1, top]));
                planes.push(f.plane([u1, v0, hip_base], [u1, v1, hip_base], [u1 - hip_run, v0, top]));
                Ok(planes)
            }
            &Roof::Mansard { rect, ridge, eaves, top, break_height: b, break_run: r } => {
                let f = Frame::new(&rect, ridge)?;
                rising(eaves, top)?;
                if b <= eaves || b >= top {
                    return invalid("breakHeight", "must lie between the eaves and the ridge");
                }
                let (u0, u1, v0, v1) = (f.u[0], f.u[1], f.v[0], f.v[1]);
                let depth = v1 - v0;
                if r <= 0 || 2 * r >= depth {
                    return invalid("breakRun", "must be greater than 0 and less than half the depth");
                }
                // The lower part rises (b − eaves) over r, the upper (top − b) over depth/2 − r; a roof whose upper part
                // is the steeper one is not convex.
                if ((b - eaves) as i128) * ((depth - 2 * r) as i128) < 2 * ((top - b) as i128) * (r as i128) {
                    return invalid("breakHeight", "the lower part must be at least as steep as the upper one");
                }
                // The upper planes reach the ridge halfway across, so twice their rise at the opposite break line.
                let far = 2 * top - b;
                Ok(vec![
                    f.plane([u0, v0, eaves], [u1, v0, eaves], [u0, v0 + r, b]),
                    f.plane([u0, v0 + r, b], [u1, v0 + r, b], [u0, v1 - r, far]),
                    f.plane([u1, v1, eaves], [u0, v1, eaves], [u1, v1 - r, b]),
                    f.plane([u1, v1 - r, b], [u0, v1 - r, b], [u1, v0 + r, far]),
                ])
            }
            Roof::Faces { faces } => {
                if faces.is_empty() {
                    return invalid("faces", "a roof needs at least one face");
                }
                Ok(faces.iter().map(|f| f.plane.clone()).collect())
            }
            &Roof::Pyramid { rect, eaves, top } => {
                let f = Frame::new(&rect, Axis::X)?;
                rising(eaves, top)?;
                let (u0, u1, v0, v1) = (f.u[0], f.u[1], f.v[0], f.v[1]);
                let far = 2 * top - eaves;
                let mut planes = f.gable(eaves, top);
                planes.push(f.plane([u0, v1, eaves], [u0, v0, eaves], [u1, v1, far]));
                planes.push(f.plane([u1, v0, eaves], [u1, v1, eaves], [u0, v0, far]));
                Ok(planes)
            }
        }
    }
}

/// A rectangle's corners x0, x1, y0, y1, checked.
fn corners(rect: &Rect) -> Result<[i64; 4], Invalid> {
    let ([x0, x1], [y0, y1]) = (rect.x, rect.y);
    if x0 >= x1 || y0 >= y1 {
        return invalid("rect", "must run from the smaller to the larger coordinate");
    }
    Ok([x0, x1, y0, y1])
}

/// A roof's rectangle seen with its ridge along the first axis: u along the ridge, v across.
struct Frame {
    u: [i64; 2],
    v: [i64; 2],
    swap: bool,
}

impl Frame {
    fn new(rect: &Rect, ridge: Axis) -> Result<Frame, Invalid> {
        let [x0, x1, y0, y1] = corners(rect)?;
        Ok(match ridge {
            Axis::X => Frame { u: [x0, x1], v: [y0, y1], swap: false },
            Axis::Y => Frame { u: [y0, y1], v: [x0, x1], swap: true },
        })
    }

    /// The plane through three points given as (u, v, height).
    fn plane(&self, a: [i64; 3], b: [i64; 3], c: [i64; 3]) -> Plane {
        let back = |[u, v, z]: [i64; 3]| if self.swap { [v, u, z] } else { [u, v, z] };
        Plane { points: [back(a), back(b), back(c)] }
    }

    /// The two planes of a symmetric gable: each continues past the ridge to the opposite eaves line, where it stands
    /// twice the rise above the eaves.
    fn gable(&self, eaves: i64, top: i64) -> Vec<Plane> {
        let (u0, u1, v0, v1) = (self.u[0], self.u[1], self.v[0], self.v[1]);
        let far = 2 * top - eaves;
        vec![
            self.plane([u0, v0, eaves], [u1, v0, eaves], [u0, v1, far]),
            self.plane([u1, v1, eaves], [u0, v1, eaves], [u1, v0, far]),
        ]
    }
}
