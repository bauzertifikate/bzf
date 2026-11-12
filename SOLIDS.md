# The solids of a building

A draft part of the spec (CC BY 4.0). A building document stores walls as lines with a build-up, slabs as outlines
with a top, roof plates as outlines in a plane, and openings as a place in a wall. This text says which solid each of
them is, so that every program that draws, exports or measures a building from the same document works on the same
bodies. It belongs to [`building.json`](schema/bzf/1/building.json) and is a draft as that schema is.

Solids are derived. A project never stores them ("Facts, not results" in the [README](README.md)); a program makes
them from the building document when it needs them. The expected solids of the example buildings are published in
[`examples/solids/`](examples/solids/) as test data, and `./check.sh` checks them.

## Limits

The rules cover what the model holds today, and no more:

- **Straight walls.** A wall is a straight line in plan with vertical faces. There are no curved or leaning walls.
- **Planar roof plates.** A roof plate is plane, with one thickness. So is every element's plate.
- **Rectangular openings.** An opening is a rectangle in its wall's elevation, through the whole thickness.
- **Ends are decided in plan.** How a wall ends at another depends on their lines and build-ups, not on their
  heights: a low wall is mitred with a high one as two walls of one height are.
- **A wall stays in its storey.** It never reaches above its storey's top. A gable's triangle above the top storey
  is a wall only if a storey holds it.
- **No trimming beyond these rules.** Solids of different items may overlap: a slab drawn out to the walls' lines
  shares its thickness with the walls under it, and a roof plate's edge can lie in a wall's top (see "The roof over
  a wall"). Quantities taken from the solids count such a volume twice unless the program taking them decides
  otherwise.

## A solid

A solid is a list of plane faces in the project's frame, in millimetres. A face is a list of rings of corners: its
outline first, then its holes. The first point of a ring is not repeated at its end.

- A solid is **closed**: every edge is walked once by one face and once, the other way, by another. So exactly two
  faces meet at every edge.
- Its faces look **outwards**: seen from outside, a face's outline runs counter-clockwise and its holes run clockwise.
- A plate without thickness is no solid. It is a surface, marked `sheet`, with one face.

## Walls

A wall with `kind` `separation` is a boundary between rooms and has no solid. Every other wall is built. A built
wall's solid comes from its line, its placement and its construction; the storey gives its bottom and top.

**Faces.** The thickness is the sum of the construction's layers (stored in metres, taken in millimetres). The first
face lies `offset` mm from the line, to the left seen from the line's start where positive. The second lies the
thickness further, to the `sense` side. Where the middle between the two faces lies within 1 mm of the line, the wall
is centred on the line exactly: an inner wall of 145 mm drawn on its axis with an offset of −73 has its faces at
−72.5 and +72.5.

**Ends.** Each end of a wall is cut square to its line or along a mitre.

- It is mitred where exactly one other wall of the storey has an end within 5 mm of this end, and the two do not
  run exactly in line. Separation lines and walls made of elements do not count as that other wall. The cut runs
  through the two points where the walls' faces meet: the first wall's face on one side of the corner with the
  other's face on that side, and likewise on the other side. Both walls are cut along the same line, so their
  footprints close the corner without overlapping.
- It is square where no wall or several walls end there, where the two walls run in line, and where a mitre point
  would lie farther from the end than three times the thicker wall's thickness (a very sharp corner).

The corners of the footprint that results are rounded to whole millimetres. A face at 4072.5 therefore stands at
4073, which is the reason for the corner tolerance below.

**Square ends that run into another wall.** A wall drawn up to another wall's line would stand inside that wall. A
square end is therefore cut back to the point where the wall enters the other wall's footprint:

- an inner wall, where it runs into an exterior wall, or into an inner wall that runs on past the end (both ends of
  that wall's line lie farther from this end than its thickness plus 5 mm);
- an exterior wall, where it runs into an exterior wall that runs on past the end.

A wall that is cut back to nothing, because it lies within a wall running past it, has no solid.

**Bottom and top.** The bottom is the wall's `bottom`, else its storey's `level`. The top is the wall's `top`, else
the storey's level plus its height. The storey's height is its `height` where that is above zero, else the distance
up to the next storey's level, else 2500 mm.

**The roof over a wall.** The roof only ever cuts a wall lower than its top. Over each point of the wall's
footprint, the roof stands at the lowest of:

- the lower faces of the part's roof plates (`roofs`). A plate's lower face is its outline moved `thickness` mm
  against the plane's normal, and it counts over its own outline in plan, less the plate's holes moved the same way.
  A point within 0.5 mm of the outline's edge is over it.
- the part's roof as drawn (`roof`): for a shape, the lowest of its planes over the point; for faces, each over its
  region.

A roof face counts only where it stands more than 1 mm above the wall's bottom. A wall that stands on a plate (a
parapet on a flat roof) or rises through one is not cut by that plate. The cut is exact across the wall's thickness:
under a sloping plate the wall's top slopes with it.

Only a plate's lower face cuts. Where a sloping plate's edge lies over a wall, the strip between the plate's
upper outline and its lower one is not under the lower face, so the wall stands to its top there and shares a wedge
with the plate. In the first example this strip is 89.5 mm wide along both eaves walls of the upper storey.

**Openings.** An opening with a width and a height above zero takes a box out of its host wall: across the whole
thickness, from `offset` to `offset + width` along the line from its start, and from the storey's level plus `sill`
up by `height`. The opening's own solid is the part of the wall's solid the box takes out. Where the roof cuts the
wall lower than the opening's top, the opening's solid ends under the roof too.

**Layers.** Each layer of the construction has a solid of its own, made as the whole wall is: the layers follow one
another from the first face to the `sense` side in the construction's order, each with the wall's ends, bottom, top,
roof cut and openings. A layer thinner than 0.5 mm, or with a face shorter than 1 mm after the ends are cut, has no
solid. A layer's material is its material's name, else the construction's name.

## Slabs

A slab's solid is its outline less its holes, from `top` down by its construction's thickness. A slab without a
construction, with a thickness of zero or with an outline of fewer than three corners has no solid.

## Roof plates

A roof plate's solid is its plate: the outline and holes in the plane's own coordinates, reaching `thickness` mm
against the plane's normal. The side faces stand at right angles to the plane. The plate's own `thickness` counts,
not its construction's. A plate with a thickness of zero is a surface (`sheet`).

## Items made of elements

A wall, slab or roof plate whose elements carry plates is those plates. Each element's plate is one solid, made as a
roof plate's is; it names the item it is part of (`of`), the element, the element's layer and the element's label as
its material. Such a wall has no solid of its own and its openings have none: the plates came with their holes.

## Items without a solid

| Rule | Which item | Why |
|---|---|---|
| `separation` | A wall with `kind` `separation` | It bounds rooms and is no building element |
| `noBody` | A built wall without a construction, thickness, length or height, or cut back to nothing; a roof plate without a plane; an opening without a width or height, or outside its wall's solid | The rules leave nothing of it |
| `noBuildUp` | A slab without a construction or with fewer than three corners | There is no thickness to give it |
| `host` | An opening whose wall has no solid of its own | Its wall is made of elements, or has none |

## The expected solids

[`examples/solids/`](examples/solids/) holds one file per example building. The files are example data (CC0).

```json
{
  "building": "examples/haus-am-hang/Daten/buildings/019a5e00-0000-7000-8000-000000000002.json",
  "tolerance": { "point": 1, "volume": 0.001 },
  "solids": [
    { "item": "…", "kind": "wall", "part": "…", "storey": "…", "volume": 7971291500, "faces": [[[[0, 0, 2750], …]]] },
    { "item": "…", "kind": "wall", "part": "…", "storey": "…", "layer": 0, "material": "Kalkzementputz", "volume": 493223000, "faces": [ … ] }
  ],
  "without": [{ "item": "…", "kind": "wall", "part": "…", "storey": "…", "rule": "separation" }]
}
```

- `building`: the building document, by its path in this repository.
- `tolerance`: how close an implementation has to come. `point` is a distance in millimetres, `volume` a share of
  the volume.
- `solids`: every solid, in the building's order: each wall whole, then its layers, then its openings; the storey's
  slabs; then the part's roof plates.
  - `item`, `kind` (`wall`, `slab`, `roof`, `opening`), `part` and, except for a roof plate, `storey`: the item by id
    and where it stands.
  - `layer` and `material` on a layer's solid; `element` on an element's plate.
  - `volume` in mm³, or `sheet` as `true` for a surface.
  - `faces`: the faces as lists of rings of `[x, y, z]`, in millimetres to the thousandth.
- `without`: every wall, slab, roof plate and opening that has no solid, with the `rule` from the table above.

**Tolerance.** An implementation reproduces a file when it makes the same solids for the same items and leaves the
same items without one, each solid's volume is within 0.1 % of the stated one, and every corner it makes lies within
1 mm of a stated corner and every stated corner within 1 mm of one it makes. The stated volumes are those of the
stated faces. They differ from the figures worked out by hand from the document by the rounding of footprints to the
millimetre, which is well inside the tolerance.

**What the examples cover.**

| Example | Covers |
|---|---|
| [Haus am Hang](examples/haus-am-hang/) | Exterior walls mitred at four corners, on two storeys; layers; windows and a door; a slab; two roof plates and a drawn gable roof cutting the upper storey's walls across their thickness |
| [Werkstatt am Bach](examples/werkstatt-am-bach/) | A wall placed with `sense` `right`; an inner wall centred on its line and cut back at two exterior walls; an inner wall with a top of its own, cut back at an inner wall running past and at an exterior wall; a separation line; a door in an inner wall; a slab with a hole; a slab without a construction; a monopitch roof plate with an overhang and a hole, cutting every wall it covers |

Not covered by an example yet: an exterior wall running into an exterior wall that passes, a wall standing on a
plate or rising through one, a square end at a very sharp corner, a window cut by the roof, items made of elements,
surfaces, and the rules `noBody` and `host`. The rules above are the specification for them; expected solids will
follow with examples that hold such items.

## Checking

`./check.sh` checks the files in `examples/solids/` without the program that made them
([`tools/check/tests/solids.rs`](tools/check/tests/solids.rs)):

- each file is well-formed and names items of its building, with their kind, part and storey;
- every wall, slab, roof plate and opening of the building has a solid or is listed without one under a rule that
  fits it, and never both;
- every solid is closed, its faces are plane and look outwards, and it holds the volume it states;
- every solid lies where the document puts its item: a wall and its layers between their faces, the bottom and the
  top; an opening within its box; a slab under its top within its outline; a roof plate under its plane;
- a wall's layers together hold what the wall holds;
- each whole solid holds the volume worked out by hand from the document's figures, within the tolerance.

The check also shows that it finds a faulty file: a face missing or turned, another volume, an item left out, a rule
that does not fit.
