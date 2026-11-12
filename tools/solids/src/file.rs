// SPDX-License-Identifier: Apache-2.0 OR MIT
//! The items without a solid, each with the rule of SOLIDS.md's table, and the solids written in the format of the
//! expected solids (SOLIDS.md, "The expected solids").

use crate::{Building, Id, Kind, Solid, Why, built, solids, volume};
use std::collections::BTreeSet;

/// How close the files promise to come: corners within a millimetre, volumes within a thousandth.
pub const POINT: f64 = 1.0;
/// See [`POINT`].
pub const VOLUME: f64 = 0.001;

/// The rule that leaves an item without a solid (SOLIDS.md, "Items without a solid").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rule {
    /// A separation line: it bounds rooms and is no building element.
    Separation,
    /// The rules leave nothing of it: a wall without a build-up, thickness, length or height, or cut back to nothing;
    /// a roof plate without a plane; an opening without a width or height, or outside its wall's solid.
    NoBody,
    /// A slab without a build-up or with fewer than three corners.
    NoBuildUp,
    /// An opening whose wall has no solid of its own.
    Host,
}

impl Rule {
    /// Its name in the files.
    pub fn name(self) -> &'static str {
        match self {
            Rule::Separation => "separation",
            Rule::NoBody => "noBody",
            Rule::NoBuildUp => "noBuildUp",
            Rule::Host => "host",
        }
    }
}

/// An item without a solid.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Without {
    /// The item.
    pub id: Id,
    /// A wall, a slab, a roof plate or an opening.
    pub kind: Kind,
    /// The part, by its index in the building.
    pub part: usize,
    /// The storey, by its index in the part; none for a roof plate.
    pub storey: Option<usize>,
    /// Why it has none.
    pub rule: Rule,
}

/// Every wall, slab, roof plate and opening of `b` that has none of `made` ([`solids`]), in the building's order, with
/// the rule that leaves it without.
pub fn without(b: &Building, made: &[Solid]) -> Vec<Without> {
    let covered: BTreeSet<&Id> = made.iter().flat_map(|s| [Some(&s.id), s.element.as_ref()]).flatten().collect();
    let has = |id: &Id, elements: &[Id]| covered.contains(id) || elements.iter().any(|e| covered.contains(e));
    let mut out = Vec::new();
    for (pi, part) in b.parts.iter().enumerate() {
        for (k, st) in part.storeys.iter().enumerate() {
            let mut line = |id: &Id, kind: Kind, rule: Rule| {
                out.push(Without { id: id.clone(), kind, part: pi, storey: Some(k), rule });
            };
            for w in st.walls.iter().filter(|w| !has(&w.id, &w.elements)) {
                line(&w.id, Kind::Wall, if built(w) { Rule::NoBody } else { Rule::Separation });
            }
            for s in st.slabs.iter().filter(|s| !has(&s.id, &s.elements)) {
                line(&s.id, Kind::Slab, Rule::NoBuildUp);
            }
            for o in st.openings.iter().filter(|o| !covered.contains(&o.id)) {
                let whole = made.iter().any(|s| s.kind == Kind::Wall && s.id == o.host && s.element.is_none());
                line(&o.id, Kind::Opening, if whole { Rule::NoBody } else { Rule::Host });
            }
        }
        for r in part.roofs.iter().filter(|r| !has(&r.id, &r.elements)) {
            out.push(Without { id: r.id.clone(), kind: Kind::Roof, part: pi, storey: None, rule: Rule::NoBody });
        }
    }
    out
}

/// The solids of `b` as an expected-solids file: `building` is the document's path as the file names it, then the
/// tolerance, every solid with its item, where it stands, its volume and its faces (corners to the thousandth of a
/// millimetre, the volume from those to the cubic millimetre), and every item without a solid with its rule.
pub fn file(b: &Building, building: &str) -> String {
    let (all, left) = solids(b);
    let mut out = format!("{{\n  \"building\": {},\n", text(building));
    out += &format!("  \"tolerance\": {{ \"point\": {POINT}, \"volume\": {VOLUME} }},\n  \"solids\": [\n");
    let records: Vec<String> = all.iter().map(|s| record(b, s)).collect();
    out += &records.join(",\n");
    out += "\n  ],\n  \"without\": [";
    let lines: Vec<String> = without(b, &all)
        .iter()
        .map(|w| {
            let part = &b.parts[w.part];
            debug_assert!(
                w.kind != Kind::Wall
                    || w.rule != Rule::NoBody
                    || left.iter().any(|l| l.id == w.id && l.why == Why::NoBody),
                "a wall without a solid is left out by a rule"
            );
            let mut head =
                format!("\"item\": {}, \"kind\": \"{}\", \"part\": {}", text(&w.id.0), kind(w.kind), text(&part.id.0));
            if let Some(k) = w.storey {
                head += &format!(", \"storey\": {}", text(&part.storeys[k].id.0));
            }
            format!("    {{ {head}, \"rule\": \"{}\" }}", w.rule.name())
        })
        .collect();
    if !lines.is_empty() {
        out += &format!("\n{}\n  ", lines.join(",\n"));
    }
    out + "]\n}\n"
}

fn kind(k: Kind) -> &'static str {
    match k {
        Kind::Wall => "wall",
        Kind::Slab => "slab",
        Kind::Roof => "roof",
        Kind::Opening => "opening",
    }
}

/// A length in mm to the thousandth, whole where it is whole.
fn number(c: f64) -> String {
    let r = (c * 1000.0).round() / 1000.0;
    if r == r.trunc() { format!("{}", r as i64) } else { format!("{r}") }
}

fn text(s: &str) -> String {
    serde_json::to_string(s).unwrap_or_default()
}

/// One solid: its corners to the thousandth of a millimetre, and its volume from those, to the cubic millimetre.
fn record(b: &Building, s: &Solid) -> String {
    let part = &b.parts[s.part];
    let mut head =
        format!("\"item\": {}, \"kind\": \"{}\", \"part\": {}", text(&s.id.0), kind(s.kind), text(&part.id.0));
    if let Some(k) = s.storey {
        head += &format!(", \"storey\": {}", text(&part.storeys[k].id.0));
    }
    if let Some(e) = &s.element {
        head += &format!(", \"element\": {}", text(&e.0));
    }
    if let Some(l) = s.layer {
        head += &format!(", \"layer\": {l}");
    }
    if let Some(m) = &s.material {
        head += &format!(", \"material\": {}", text(m));
    }
    let round = |c: f64| number(c).parse::<f64>().unwrap_or(c);
    let mut faces = s.faces.clone();
    faces.iter_mut().flat_map(|f| &mut f.rings).flatten().for_each(|p| *p = p.map(round));
    let lines: Vec<String> = faces
        .iter()
        .map(|f| {
            let rings: Vec<String> = f
                .rings
                .iter()
                .map(|r| {
                    let points: Vec<String> =
                        r.iter().map(|p| format!("[{}, {}, {}]", number(p[0]), number(p[1]), number(p[2]))).collect();
                    format!("[{}]", points.join(", "))
                })
                .collect();
            format!("        [{}]", rings.join(", "))
        })
        .collect();
    let size =
        if s.sheet { "\"sheet\": true".to_string() } else { format!("\"volume\": {}", number(volume(&faces).round())) };
    format!("    {{\n      {head},\n      {size},\n      \"faces\": [\n{}\n      ]\n    }}", lines.join(",\n"))
}
