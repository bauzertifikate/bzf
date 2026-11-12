// SPDX-License-Identifier: Apache-2.0 OR MIT
//! What the solids are made from: the parts of a building document (`schema/bzf/1/building.json`) that SOLIDS.md reads,
//! as Rust types that deserialize from the document's JSON. Everything else in the document is skipped.
//!
//! Lengths are integer millimetres in the project's frame, heights relative to ±0,00; a layer's thickness is in metres,
//! as the document stores it. A value of a closed set this crate does not know (a wall kind or a roof shape of a later
//! edition) is read as `None`, never as an error, as the compatibility rule in the README asks of a reader.

use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer, Serialize};

/// A point in plan, mm: x and y in the project's frame.
pub type Point = [i64; 2];

/// An item's identity: a UUID in its standard text form.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Id(pub String);

impl Id {
    /// Whether it is in the standard text form of a UUID: 32 hex digits in groups of 8-4-4-4-12.
    pub fn well_formed(&self) -> bool {
        let groups: Vec<&str> = self.0.split('-').collect();
        groups.len() == 5
            && groups
                .iter()
                .zip([8, 4, 4, 4, 12])
                .all(|(g, n)| g.len() == n && g.chars().all(|c| c.is_ascii_hexdigit()))
    }
}

impl std::fmt::Display for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// A value of a closed set as the document holds it: none where it is a value this crate does not know.
fn lenient<'de, D: Deserializer<'de>, T: DeserializeOwned>(d: D) -> Result<Option<T>, D::Error> {
    let value = serde_json::Value::deserialize(d)?;
    Ok(serde_json::from_value(value).ok())
}

/// The value a document means where it leaves the field out.
fn by_default<T: Default>() -> Option<T> {
    Some(T::default())
}

/// A building document.
#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Building {
    /// Parts standing side by side.
    #[serde(default)]
    pub parts: Vec<Part>,
    /// The build-ups its walls and slabs refer to.
    #[serde(default)]
    pub constructions: Vec<Construction>,
}

/// A building part: its storeys and its roof.
#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Part {
    /// Its id.
    pub id: Id,
    /// Bottom to top.
    #[serde(default)]
    pub storeys: Vec<Storey>,
    /// The roof's plates.
    #[serde(default)]
    pub roofs: Vec<RoofPlate>,
    /// The roof as drawn: a shape, free planes, or faces each over a region.
    #[serde(default, deserialize_with = "lenient")]
    pub roof: Option<Roof>,
}

/// A storey.
#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Storey {
    /// Its id.
    pub id: Id,
    /// The top of its structural slab, mm.
    pub level: i64,
    /// Up to the next storey's level, mm; 0 where the document gives none.
    #[serde(default)]
    pub height: i64,
    /// Its walls.
    #[serde(default)]
    pub walls: Vec<Wall>,
    /// Its slabs.
    #[serde(default)]
    pub slabs: Vec<Slab>,
    /// The windows and doors in its walls.
    #[serde(default)]
    pub openings: Vec<Opening>,
    /// The built parts as a source gives them, each pointing at its wall, slab or roof plate.
    #[serde(default)]
    pub elements: Vec<Element>,
}

/// A wall: a reference line in plan and where its layers lie relative to it.
#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Wall {
    /// Its id.
    pub id: Id,
    /// The reference line, from its start to its end, mm.
    pub line: [Point; 2],
    /// Where the layers lie.
    #[serde(default)]
    pub placement: Placement,
    /// A drawn wall's build-up.
    #[serde(default)]
    pub construction: Option<Id>,
    /// A wall of elements: its elements, outside first.
    #[serde(default)]
    pub elements: Vec<Id>,
    /// Its own bottom, mm, where it is not its storey's level.
    #[serde(default)]
    pub bottom: Option<i64>,
    /// Its own top, mm, where it is not its storey's top.
    #[serde(default)]
    pub top: Option<i64>,
    /// A built wall or a separation line; none for a kind this crate does not know, which counts as built.
    #[serde(default = "by_default", deserialize_with = "lenient")]
    pub kind: Option<WallKind>,
    /// One side faces outdoors.
    #[serde(default)]
    pub exterior: bool,
}

/// Whether a wall is built or only divides rooms.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WallKind {
    /// A wall of layers.
    #[default]
    Wall,
    /// A separation line: only a boundary between rooms.
    Separation,
}

/// Where a wall's layers lie: its first layer's face `offset` mm from the reference line, to the line's left seen from
/// its start where positive, and the layers following one another to the `sense` side.
#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Placement {
    /// From the line to the first layer's face, mm, to the left where positive.
    #[serde(default)]
    pub offset: i64,
    /// The side the layers follow one another to; none for a value this crate does not know, which counts as left.
    #[serde(default = "by_default", deserialize_with = "lenient")]
    pub sense: Option<Side>,
}

/// A side of a line, seen from its start.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Side {
    /// To the left.
    #[default]
    Left,
    /// To the right.
    Right,
}

/// A slab: an outline in plan, its top and its build-up.
#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Slab {
    /// Its id.
    pub id: Id,
    /// mm, counter-clockwise.
    pub outline: Vec<Point>,
    /// Openings through it, clockwise.
    #[serde(default)]
    pub holes: Vec<Vec<Point>>,
    /// Its top, mm.
    pub top: i64,
    /// Its build-up.
    #[serde(default)]
    pub construction: Option<Id>,
    /// Its elements, outside first.
    #[serde(default)]
    pub elements: Vec<Id>,
}

/// A window, a door, an opening: one item hosted by a wall.
#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Opening {
    /// Its id.
    pub id: Id,
    /// The wall it is in.
    pub host: Id,
    /// From the wall's reference line's start to the opening's near side, mm.
    pub offset: i64,
    /// Its bottom above the storey's level, mm.
    #[serde(default)]
    pub sill: i64,
    /// Width, mm.
    pub width: i64,
    /// Height, mm.
    pub height: i64,
}

/// A roof plate.
#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoofPlate {
    /// Its id.
    pub id: Id,
    /// Its plane, outline and thickness.
    pub plate: Plate,
    /// Its elements, outside first.
    #[serde(default)]
    pub elements: Vec<Id>,
}

/// A plate: a plane through three integer points and its outline in the plane's own integer coordinates (from the
/// first point, u towards the second, v at right angles to it in the plane, mm), the plate reaching `thickness` mm from
/// it against the plane's normal (the right-handed normal of its three points).
#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Plate {
    /// The plane, through three integer points, mm.
    pub plane: Plane,
    /// The outline in the plane's coordinates, mm.
    pub outline: Vec<Point>,
    /// Holes through it, in the same coordinates.
    #[serde(default)]
    pub holes: Vec<Vec<Point>>,
    /// mm.
    #[serde(default)]
    pub thickness: i64,
}

impl Plate {
    /// The plane's frame; none for three points on one line.
    pub fn frame(&self) -> Option<PlaneFrame> {
        PlaneFrame::through(self.plane.points)
    }
}

/// A plate's plane as a frame: its first point, the unit directions u and v in it, and its unit normal.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlaneFrame {
    /// The plane's first point, mm.
    pub origin: [f64; 3],
    /// Towards its second point.
    pub u: [f64; 3],
    /// At right angles to u in the plane.
    pub v: [f64; 3],
    /// The right-handed normal of its three points.
    pub normal: [f64; 3],
}

impl PlaneFrame {
    /// The frame of the plane through three points, mm; none for three points on one line.
    pub fn through(points: [[i64; 3]; 3]) -> Option<PlaneFrame> {
        let p = points.map(|q| q.map(|c| c as f64));
        let sub = |a: [f64; 3], b: [f64; 3]| [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
        let cross = |a: [f64; 3], b: [f64; 3]| {
            [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
        };
        let unit = |a: [f64; 3]| {
            let l = (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt();
            (l > 0.0).then(|| a.map(|c| c / l))
        };
        let u = unit(sub(p[1], p[0]))?;
        let normal = unit(cross(sub(p[1], p[0]), sub(p[2], p[0])))?;
        Some(PlaneFrame { origin: p[0], u, v: cross(normal, u), normal })
    }

    /// A point given in the plane's coordinates, in the project's frame, mm.
    pub fn at(&self, q: Point) -> [f64; 3] {
        let (s, t) = (q[0] as f64, q[1] as f64);
        [0, 1, 2].map(|k| self.origin[k] + self.u[k] * s + self.v[k] * t)
    }
}

/// A built part as a source gives it: a layer of a wall, a slab's or a roof's plate.
#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Element {
    /// Its id.
    pub id: Id,
    /// The source's name for it.
    #[serde(default)]
    pub label: String,
    /// Wall, slab, roof or other; none for a kind this crate does not know.
    #[serde(default = "by_default", deserialize_with = "lenient")]
    pub kind: Option<ElementKind>,
    /// The wall, slab or roof plate it is part of.
    #[serde(default)]
    pub of: Option<Id>,
    /// Which of their layers, outside first.
    #[serde(default)]
    pub layer: Option<u32>,
    /// Its plate.
    #[serde(default)]
    pub plate: Option<Plate>,
}

/// What an element is.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ElementKind {
    /// A wall's layer.
    #[default]
    Wall,
    /// A slab's plate.
    Slab,
    /// A roof's plate.
    Roof,
    /// Anything else.
    Other,
}

/// A build-up: its layers, outside to inside.
#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Construction {
    /// Its id.
    pub id: Id,
    /// Its name.
    #[serde(default)]
    pub name: String,
    /// Outside to inside.
    #[serde(default)]
    pub layers: Vec<Layer>,
}

/// A layer: a material and its thickness.
#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Layer {
    /// Its material.
    pub material: Material,
    /// m.
    pub thickness: f64,
}

/// A layer's material, of which the solids need only the name.
#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Material {
    /// Its name.
    #[serde(default)]
    pub name: String,
}

/// A roof as drawn over a building part: a shape over a rectangle of the project's frame, free planes, or faces each
/// over its own region. The shapes and free planes are convex, the roof over each point their lowest plane; faces
/// carry valleys and steps. Heights are mm, like the storey levels.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Roof {
    /// Free planes; over each point the lowest one is the roof.
    Planes {
        /// The roof planes.
        planes: Vec<Plane>,
    },
    /// A monopitch roof: one plane, at `eaves` along the rectangle's `low` side and at `top` along the opposite one.
    Mono {
        /// The rectangle the roof covers.
        rect: Rect,
        /// The side with the eaves.
        low: RectSide,
        /// The eaves' height.
        eaves: i64,
        /// The height along the opposite side.
        top: i64,
    },
    /// A gable roof: eaves at `eaves` on both long sides, the ridge at `top` along `ridge`, halfway across or at
    /// `ridge_at`.
    #[serde(rename_all = "camelCase")]
    Gable {
        /// The rectangle the roof covers.
        rect: Rect,
        /// The axis the ridge runs along.
        ridge: Axis,
        /// The eaves' height.
        eaves: i64,
        /// The ridge's height.
        top: i64,
        /// For an asymmetric roof, the ridge's coordinate across the rectangle (y for a ridge along x).
        #[serde(default)]
        ridge_at: Option<i64>,
    },
    /// A hip roof: a gable whose ends slope too. Each hip plane reaches the ridge's height `hip_run` in from its end, by
    /// default as steep as the long sides.
    #[serde(rename_all = "camelCase")]
    Hip {
        /// The rectangle the roof covers.
        rect: Rect,
        /// The axis the ridge runs along.
        ridge: Axis,
        /// The eaves' height, all round.
        eaves: i64,
        /// The ridge's height.
        top: i64,
        /// How far in from each end the hip plane reaches the ridge's height, mm.
        #[serde(default)]
        hip_run: Option<i64>,
    },
    /// A half-hip roof: a gable whose end walls stop at `hip_base`; above it a small hip plane reaches the ridge's
    /// height `hip_run` in from the end.
    #[serde(rename_all = "camelCase")]
    HalfHip {
        /// The rectangle the roof covers.
        rect: Rect,
        /// The axis the ridge runs along.
        ridge: Axis,
        /// The eaves' height on the long sides.
        eaves: i64,
        /// The ridge's height.
        top: i64,
        /// Where the end walls stop and the hip planes begin.
        hip_base: i64,
        /// How far in from each end the hip plane reaches the ridge's height, mm.
        hip_run: i64,
    },
    /// A mansard roof: each long side rises steeply from `eaves` to `break_height`, `break_run` in from its edge, then
    /// less steeply to the ridge at `top` halfway across; the ends are walls.
    #[serde(rename_all = "camelCase")]
    Mansard {
        /// The rectangle the roof covers.
        rect: Rect,
        /// The axis the ridge runs along.
        ridge: Axis,
        /// The eaves' height.
        eaves: i64,
        /// The ridge's height.
        top: i64,
        /// The height of the break between the steep and the flatter part.
        break_height: i64,
        /// How far in from the edge the break lies, mm.
        break_run: i64,
    },
    /// A pyramid roof: four planes rising from eaves at `eaves` on every side to a peak at `top` in the rectangle's
    /// middle.
    Pyramid {
        /// The rectangle the roof covers.
        rect: Rect,
        /// The eaves' height, all round.
        eaves: i64,
        /// The peak's height.
        top: i64,
    },
    /// Faces, each a plane over its own region of the plan: any roof, valleys and steps included.
    Faces {
        /// The faces.
        faces: Vec<RoofFace>,
    },
}

/// A roof face: a plane over a region of the plan.
#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoofFace {
    /// The region in plan, mm, counter-clockwise seen from above.
    pub outline: Vec<Point>,
    /// The face's plane.
    pub plane: Plane,
}

/// A rectangle along the axes of the project's frame, mm.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
pub struct Rect {
    /// From and to in x.
    pub x: [i64; 2],
    /// From and to in y.
    pub y: [i64; 2],
}

/// An axis of the project's frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Axis {
    /// x.
    X,
    /// y.
    Y,
}

/// A side of a rectangle.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RectSide {
    /// The side at the smaller x.
    XMin,
    /// The side at the larger x.
    XMax,
    /// The side at the smaller y.
    YMin,
    /// The side at the larger y.
    YMax,
}

/// A plane through three points, mm: x, y and height.
#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Plane {
    /// Three points on the plane.
    pub points: [[i64; 3]; 3],
}
