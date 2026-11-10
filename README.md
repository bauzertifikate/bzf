# BZF — the open building format

BZF (Bauzertifikate Format) describes buildings in JSON: their geometry, constructions, technical systems,
renovation plans and the documents issued about them. Initiated by Bauzertifikate, it is open to independent
implementations, contributions and forks. Using BZF does not require a Bauzertifikate app, account or calculation service.

This is the canonical repository for the specification, JSON Schemas, fictional examples and conformance tests.
The schemas are the source of truth; implementations derive their own calculation models from them.

**Specification edition: 0.1.0 (initial public release).** The document schemas describe BZF 1.x. Some schemas are
stable and others are drafts, as listed below. Drafts can still change incompatibly before specification 1.0;
stable schemas retain their compatibility promise. Pin a release tag or commit for reproducible integrations.
See [VERSIONING.md](VERSIONING.md) for versioning and compatibility, and [CONTRIBUTING.md](CONTRIBUTING.md) to contribute.

Start with the [example project](examples/haus-am-hang/) and the schema table below. [MAPPINGS.md](MAPPINGS.md)
maps the building data to EPBD and iSFP information requirements. The EPBD dataset is a BZF proposal, not an official EU format.

| Schema | What | Status |
|---|---|---|
| [`bzf.json`](schema/bzf/1/bzf.json) | The `<Name>.bzf` that opens a project | stable |
| [`revision.json`](schema/bzf/1/revision.json) | A revision in the store; under `$defs` a writer's head (with the results it wrote) and a device's record | stable |
| [`received.json`](schema/bzf/1/received.json) | The files received into `Zeichnungen/`, `Fotos/`, `Dokumente/`, by hash | stable |
| [`versand.json`](schema/bzf/1/versand.json) | What an issue in `Versendet/` records beside its files | stable |
| [`project.json`](schema/bzf/1/project.json) | The root document: the frame (north, origin, CRS, NHN datum), buildings, plan states | stable |
| [`common.json`](schema/bzf/1/common.json) | Ids, hashes, times, dates, versions, points, open values, sources, constructions, products, identifiers, addresses, carriers, services, money, document classes, conflicts | stable |
| [`building.json`](schema/bzf/1/building.json) | A building `buildings/<id>.json`: identifiers, address, year, use; parts, storeys, walls, spaces, slabs, openings (with Uw, g, glazing), roofs, elements, neighbours; its technical building systems | draft |
| [`plan-state.json`](schema/bzf/1/plan-state.json) | A plan state `plans/<id>.json`: its files by hash with their transforms | draft |
| [`plan-import.json`](schema/bzf/1/plan-import.json) | An import's result `imports/<id>.json`, with its lineage and the solids its elements came as | draft |
| [`roadmap.json`](schema/bzf/1/roadmap.json) | A renovation roadmap `roadmaps/<id>.json` (renovation passport, iSFP): steps, measures and their changes to the building, targets, assumptions | draft |
| [`costs.json`](schema/bzf/1/costs.json) | A roadmap's costs `costs/<id>.json`: investment, the share due anyway, funding per measure (commercial) | draft |
| [`consumption.json`](schema/bzf/1/consumption.json) | Measured consumption `consumption/<id>.json`: meters and amounts per period | draft |
| [`party.json`](schema/bzf/1/party.json) | A person or organisation `parties/<id>.json`: roles, contact, consents (party data) | draft |

Beside the schemas, [`ids/ifc-import.ids`](ids/ifc-import.ids) (IDS 1.0, buildingSMART's Information Delivery
Specification, CC0) says what an IFC file must contain for our apps to read a building from it: storeys with name and
elevation, walls with their material and whether they face outdoors, windows and doors filling openings in walls,
slabs, a roof's slabs in its IfcRoof, and rooms.

Each schema's `$id` is its place under `https://bauzertifikate.de/schema/bzf/1/` (this folder's `schema/` mirrors
that path), its `x-bzf-status` says whether it is stable or a draft, and a document's `x-bzf-class` says where it may
go. The schemas are JSON Schema 2020-12. A reader finds a document's schema by its path under `Daten/`: `project.json`,
`received.json`, `buildings/`, `plans/`, `imports/`, `roadmaps/`, `costs/`, `consumption/`, `parties/`; any other
path is an app's own document.

## Scope

In the spec: the building model's documents, the project folder with its `.bzf` and its store (objects, canonical
JSON, revisions, heads, merge, received files), and the envelope of results and issued documents (which revision,
which kernel version, to whom). Not in it: our kernel and the builders that derive figures from the model (envelope,
areas, quantities, U-values, the DIN/TS 18599 balance are never stored), the apps' own documents (`energieberatung/…`,
`aufmass/…`: reserved, kept unchanged by others, not described), the contents of issued files (their formats are
GAEB's, DIBt's, BAFA's), sync, server, accounts and AI. Overrides are described as opaque data.

Conformance comes in three levels: a **reader** reads `Daten/` and the `.bzf`, validates against
these schemas and ignores what it doesn't know; a **store reader** also reads revisions and objects and checks each
object against its hash; a **writer** writes on a head of its own, keeps unknown fields and values, and merges by the
spec's rules. The rules of levels 2 and 3 are to be written down in the specification text; the schemas here describe
the files.

## How the documents are written

- **Identity.** Every item has an id for life, a UUID in its standard text form (UUIDv7 when we make one), and a label
  people read beside it. References go by id only. Ids from a source (a LoD2 gml:id, an IFC GlobalId, a DWG handle)
  go into the item's `source`.
- **Units.** Plan coordinates and lengths are integer millimetres in the project's frame, heights millimetres relative
  to its ±0,00, layer thicknesses metres; the frame's origin and datum are metres. Each field's description says its
  unit.
- **Open by design.** Every object schema has `additionalProperties: true`. Every closed set of values is "one of the
  known values, or any other string or object": the known values are listed for tools, a newer one is valid. Where a
  value is an object tagged by `kind` (a space's condition, a roof), a known `kind` must have its known shape and an
  unknown `kind` may have any.
- **Language.** Keys and listed values are English, in camelCase, in every language. What people write (labels,
  names, notes, reasons) is in the project's `language` (`project.json`, a BCP 47 tag, `de` where absent). A value
  national to one country carries its code, `<country>:<value>` (`de:isfp`, `de:AN`, `de:effizienzhaus-70`); a value
  the EPBD or an EU or international standard defines stays bare (`nzeb`, `scop`). The folder names (`Daten/`,
  `Versendet/`, `Ergebnisse/`, `Zeichnungen/`, `Fotos/`, `Dokumente/`) are fixed identifiers of the format and are not
  translated; a received file's `role` (`plans`, `photos`, `documents`) says what its folder is for in any language.
  The store's directory `.bauzertifikate/` becomes `.bzf/` before spec 1.0, and readers then read both.
- **Defaults are left out**, so a document read and written back keeps its bytes.
- **Conflicts.** The store's merge keeps a value two writers changed differently as `{"$conflict": {"base", "values"}}`
  in place of the value. A document holding conflicts is validated as the apps read it: each conflict settled by its
  first value.
- **Changes over time.** A renovation roadmap doesn't copy the building per step: each measure lists its changes to
  the building document (a field set, an item added or removed, by id), and the state after a step is the building
  with every change up to it applied. Past renovations are steps marked done.
- **Other major versions.** Each schema describes major version 1 of its document (`"schema": "1.x"`, store format
  `bzf/1.x`). A document or revision of another major version is only required to say which, and a reader opens the
  project read-only.

## Privacy

Every document under `Daten/` has a class (`x-bzf-class` in its schema; the apps' own documents by the root's
`classes`):

- **building**: the building, its plan states and imports, roadmaps, consumption, received files. Shared with another
  organisation only with the owner's consent, which the issue that passes them on names (`versand.json` `consent`).
  Even these can be personal data: a home's address, plans, photos and consumption relate to the people living in it.
- **party**: people and organisations, their contacts and consents. Never leaves the organisation that holds it.
  Building documents name no person; revisions name writers by opaque ids only.
- **commercial**: costs, prices, funding amounts, fees. Never leaves the organisation that holds it, so a roadmap
  shared with a trade goes without its costs.

An object can be purged from the store (a GDPR erasure, a wrong building's plans): its file is deleted and a tombstone
records that (`revision.json` `$defs/purged`), so readers show it as removed rather than missing.

## Compatibility rule

These rules apply from specification edition 0.x onward. A reader or writer of bzf/1:

1. MUST keep every field and every value it doesn't know, at every level of every document and revision, and MUST
   write them back unchanged. A known value with fields it doesn't know is kept with them.
2. MUST open a project, document or revision of a newer major version read-only, and MUST NOT refuse it.
3. MUST NOT migrate a document on opening; a document is written in a newer version only when it is changed.
4. SHOULD record in the root's `needs` the lowest app version that reads everything in the project, once something
   needs a newer one than the schema versions say.

Versions: the store format (`bzf/1` in the `.bzf` and every revision) changes with the folder or the store; each
document's `schema` (`"1.0"`) changes with the document, a minor version only adding; the spec edition (`0.1.0`
now) changes with the text. A stable schema changes only by additions within its major version.

## Checking

Install the Rust toolchain specified in `tools/rust-toolchain.toml`, then run from this repository's root:

```sh
./check.sh
```

This checks schema validity, reference resolution, extensibility, positive and negative document fixtures, and the
fictional examples, including references between renovation measures and costs. Schema references are registered
locally; validation does not fetch schemas from the internet. Rust dependencies may need downloading on the first run.
The check does not require any Bauzertifikate application or private repository.

The optional `BZF_DOCUMENTS` environment variable points to implementation-generated fixtures. The fixture layout
is described in `tools/check/tests/documents.rs`; without it, the independent schema and example tests still run.
This is a conformance test suite, not yet a general-purpose `bzf check` command-line validator.

[`examples/haus-am-hang/`](examples/haus-am-hang/) is a fictional house of 1968, with a gas boiler, consumption,
an issued certificate, renovation steps, costs and consent records. People, projects and records in the examples
are fictional. Referenced photographs, plans, invoices and issued PDFs are not included.

## The EPBD proposal: bzf-epbd

[`schema/bzf-epbd/0.1/dataset.json`](schema/bzf-epbd/0.1/dataset.json) describes a standalone exchange dataset for
building, certificate and renovation-passport information. It is an experimental BZF proposal inspired by the EPBD,
not a claim of official standardization or certification. Its mapping to BZF is documented in [MAPPINGS.md](MAPPINGS.md).

The format identifier is `bzf-epbd/0.1`. Text has a `language`, monetary values have a `currency`, and national
identifiers carry a country prefix. Version 0.1 carries the current certificate, passport and assessments, not their
history. The examples in [`examples/bzf-epbd/`](examples/bzf-epbd/) contain a typical and a fully populated dataset.
The tests validate both and check that every example field is declared in the schema.

## Licences

| Part | Licence |
|---|---|
| Specification and documentation | [CC BY 4.0](LICENSES/CC-BY-4.0.txt) |
| JSON Schemas, IDS definition and example data | [CC0 1.0](LICENSES/CC0-1.0.txt) |
| Code in `tools/`, scripts and CI configuration | [Apache-2.0](LICENSES/Apache-2.0.txt) OR [MIT](LICENSES/MIT.txt) |

Attribution for the documentation: Bauzertifikate contributors. The same terms apply to contributions to each part.
The repository's licences do not change the ownership or licences of real building data stored in BZF files.

BZF is the format's public name; `.bzf` is its file extension. Independent implementations may describe their BZF
support. Forks should identify their changes and must not imply that they are official Bauzertifikate releases.

## Kurzfassung

BZF (Bauzertifikate Format) ist ein offenes, versioniertes Gebäudedatenformat. Es beschreibt Gebäude, Bauteile,
Anlagentechnik, Sanierungsschritte und ausgestellte Dokumente als JSON. Andere Programme können es unabhängig von
Bauzertifikate lesen und schreiben. Die Schemas und Beispiele sind frei verfügbar; Beiträge und Forks sind willkommen.
Die Spezifikation ist noch in Version 0.1.0, mit ausdrücklich gekennzeichneten stabilen und vorläufigen Teilen.
