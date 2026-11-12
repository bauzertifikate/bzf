# BZF — the open building format

BZF (Bauzertifikate Format) describes buildings in JSON: their geometry, constructions, technical systems,
renovation plans and the documents issued about them. Initiated by Bauzertifikate, it is open to independent
implementations, contributions and forks. Using BZF does not require a Bauzertifikate app, account or calculation service.

This is the canonical repository for the specification, JSON Schemas, fictional examples, conformance tests and
the `bzf` program that checks, packs and signs projects ("Checking").
The schemas are the source of truth; implementations derive their own calculation models from them.

**Specification edition: 0.3.0.** [CHANGELOG.md](CHANGELOG.md) lists what changed since 0.1.0. The document schemas
describe BZF 1.x. Some schemas are stable and others are drafts, as listed below. Drafts can still change
incompatibly before specification 1.0; stable schemas retain their compatibility promise. Pin a release tag or
commit for reproducible integrations. See [VERSIONING.md](VERSIONING.md) for versioning and compatibility, and [CONTRIBUTING.md](CONTRIBUTING.md) to contribute.

Start with the [example project](examples/haus-am-hang/) and the schema table below. [MAPPINGS.md](MAPPINGS.md)
maps the building data to EPBD and iSFP information requirements. The EPBD dataset is a BZF proposal, not an official EU format.
[SOLIDS.md](SOLIDS.md) says which solid a building document's walls, slabs, roof plates and openings are.

| Schema | What | Status |
|---|---|---|
| [`bzf.json`](schema/bzf/1/bzf.json) | The project file `<Name>.bzfproj` that marks a project folder | stable |
| [`revision.json`](schema/bzf/1/revision.json) | A revision in the store, with the roadmap steps it carried into the building; under `$defs` a writer's head (with the results it wrote) and a device's record | stable |
| [`received.json`](schema/bzf/1/received.json) | The files received into `Zeichnungen/`, `Fotos/`, `Dokumente/`, by hash | stable |
| [`versand.json`](schema/bzf/1/versand.json) | What an issue in `Versendet/` records beside its files | stable |
| [`result.json`](schema/bzf/1/result.json) | A result record `Ergebnisse/<app>/<title>/ergebnis.json`: the figures a method worked out, the documents they were made from, the program and kernel version, its settings and what the method assumed | draft |
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
path is an app's own document. Outside `Daten/`, a file named `versand.json` in a folder of `Versendet/` is an issue
record, and a file named `ergebnis.json` under `Ergebnisse/` is a result record.

## A document, a file, a folder

BZF data comes in three forms. The documents are the same in all three.

| Form | What it is | What it is for |
|---|---|---|
| A document | One JSON document that is valid against its schema: a building, a roadmap | A request to a service, a tool, a test |
| A `.bzf` file | A whole project in one file, `Haus am Hang.bzf`: a ZIP archive of the project folder | Handing a building over, uploading it, archiving it |
| A project folder | The documents under `Daten/`, the received files, the issues, the results and the store, marked by its project file `<Name>.bzfproj` | Working on a project, alone or with several apps and devices |

The project folder is what the schemas above describe and what the example shows. The `.bzf` file is written and
read by the `bzf` program in this repository ("Checking"), and [`examples/Haus am Hang.bzf`](examples/) is the
example project as a share. Its rules:

- **Container.** A ZIP archive within the limits of ISO/IEC 21320-1: each entry stored or deflated, no encryption,
  no split archives, names in UTF-8 with `/` between their parts.
- **Contents.** The project folder's contents lie at the archive's root, without a folder around them: exactly one
  project file `*.bzfproj`, `Daten/`, and the other folders as far as they are passed on. The project's name is the
  `.bzf` file's own name, and an app that unpacks the file names the project file after it.
- **With or without history.** With the store, the file is a full copy of the project and its history. Without it,
  the file is a snapshot: the documents as they are, and no earlier state.
- **Reading.** A reader takes the documents from `Daten/` in the archive, as it does in a folder. It MUST NOT write
  outside the folder it unpacks into, and MUST refuse the whole archive, unpacking nothing, for one entry
  - whose name is absolute (a leading `/`, a drive letter), has a backslash or a control character, or has a part
    that is `..`, `.` or empty;
  - that is encrypted, is a link, or is compressed by a method other than stored or deflated;
  - whose name another entry has too, compared without regard to case, or which lies under a name that is a file.

  A reader SHOULD limit how much it unpacks from one archive, whatever sizes the archive states: the `bzf` program
  stops at 1 GiB.
- **Writing.** A `.bzf` file is written whole and never changed in place. Work happens in a project folder: an app
  unpacks the file into one and packs a new file from it.
- **Passing on.** A `.bzf` file that leaves the organisation is a share ("Privacy"): it holds only documents and
  files of the class `building`, and no store, because the store's objects include every document the project ever
  held, those about people and money too. A share holds exactly: the project file, the documents under `Daten/` of
  the class `building`, the received files that `received.json` names, and the result records with the signatures
  beside them. It holds nothing from `Versendet/`, whose issues name recipients.

Project folders written under edition 0.1 have their project file as `<Name>.bzf`. A reader accepts both names and
tells such a file from an archive by its first bytes: the project file is JSON and starts with `{`, an archive
starts with `PK`. A writer names a new project's file `<Name>.bzfproj` and renames an older one with the next
revision it writes, never on opening.

## Scope

In the spec: the building model's documents, the project folder with its project file and its store (objects,
canonical JSON, revisions, heads, merge, received files), how a document is named by its hash
([HASHES.md](HASHES.md)), the project as one `.bzf` file, the envelope of results and issued documents (which
revision, which kernel version, to whom), signatures over a result ([SIGNATURES.md](SIGNATURES.md), a draft), and
the rules that turn a building document's
walls, slabs, roof plates and openings into solids ([SOLIDS.md](SOLIDS.md), a draft; the solids themselves are
derived and never stored). Not in it: our kernel and the builders that
derive figures from the model (envelope, areas, quantities, U-values, the DIN/TS 18599 balance are never stored in
the model), the
apps' own documents (`energieberatung/…`, `aufmass/…`: reserved, kept unchanged by others, not described), the
contents of issued files (their formats are GAEB's, DIBt's, BAFA's), sync, server, accounts and AI. Overrides are
described as opaque data.

Conformance comes in three levels: a **reader** reads `Daten/` and the project file, from a folder or a `.bzf` file,
validates against these schemas and ignores what it doesn't know; a **store reader** also reads revisions and objects
and checks each object against its hash; a **writer** writes on a head of its own, keeps unknown fields and values,
and merges by the spec's rules. The rules of levels 2 and 3 are to be written down in the specification text; the
schemas here describe the files.

A writer need not read every document. A program for one task, time sheets for instance, may have no use for the
building document, and it can still contribute to the project: it adds its own documents and the files it received
in a revision on its head, and carries every document it does not read into that revision unchanged, by its hash.
So every program that works on a project can add to it, whatever it reads of it.

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
  The store's directory is `.bauzertifikate/`, and that name stays.
- **Defaults are left out**, so a document read and written back keeps its bytes.
- **Conflicts.** The store's merge keeps a value two writers changed differently as `{"$conflict": {"base", "values"}}`
  in place of the value. A document holding conflicts is validated as the apps read it: each conflict settled by its
  first value.
- **Facts, not results.** The model stores facts about the building, as entered or received. What a method works out
  from them is never stored in it, by any app: areas, quantities, the U-value of a layered construction, an energy
  balance, a life-cycle assessment, a roadmap's figures, a rating. A value that a product's data sheet states (a
  window's Uw, a heat pump's SCOP) is a fact. What was issued is kept as issued: a certificate or a renovation
  passport lies in `Versendet/` with its files, its registration and the figures it states, tied to the revision and
  the inputs it was made from, and a certificate received from someone else is a received file of type `certificate`.
  An issue's figures record that document. They are not inputs to another calculation.
- **Results beside the model** (a draft). A program that keeps what a method worked out writes a result record,
  `Ergebnisse/<app>/<title>/ergebnis.json` ([`result.json`](schema/bzf/1/result.json)): the figures per state, the
  documents they were made from (path → the hash of the document's canonical JSON, [HASHES.md](HASHES.md)), the
  method, the program and
  kernel version, how the program was set (`choices`) and what the method assumed where the model was silent
  (`assumptions`). Its program replaces it whenever it calculates again; nothing else writes it, and no document
  under `Daten/` refers to it.
  - **Current or stale.** A result is current while each document it names still has the hash it names, whatever
    else in the project changed since, and stale once one of them has another. Any reader can tell this without
    the program that wrote the result. Where a document it names is not there, as in a share that leaves a
    program's settings behind, the reader cannot tell and says so; it does not call the result stale. A result
    that names no documents is current only at its revision.
  - **Three places for a value.** A fact about the building is in the building document. A setting of the program
    is in the program's own document, which the result then names among its inputs, or in the result's `choices`.
    A value the method took because the model had none is in the result's `assumptions`, in words, for people to
    read and not for another program to calculate with.
  - **Issued.** An issue names the result it issued by the hash of its canonical JSON (`versand.json` `result`), and
    that record is then kept as an object with the issue, as the issue's inputs are. The issue's figures are that
    result's figures.
  - **What a result shows.** That nothing changed since it was written (the hashes), and what it was made from and
    with. A program of the same version gives the same figures from the same inputs; another version or another
    program may differ within the method's tolerance. A result record does not show that its figures are right.
  - **Signatures** (a draft, [SIGNATURES.md](SIGNATURES.md)). A file `ergebnis.<name>.jws` beside a result record
    is a detached signature over that record's canonical JSON: a JSON Web Signature (ES256) that carries the
    signer's certificate. Several may lie beside one record. A signature states who signed and that the record is
    unchanged since, and nothing about that person's or organisation's entitlement to issue; whether the
    certificate is to be believed is the reader's decision. A writer that replaces a result record removes the
    signatures beside it, and a reader that does not verify signatures ignores them.
- **One state.** A building document describes the building as it stands, in every revision. Work under way belongs
  to that state: an item's `phase` says whether it stands and stays (`existing`), stands and is being removed
  (`demolished`) or is being built (`new`: a building in planning, an extension under construction). What is planned
  for later is not in the building document. It lies in a roadmap.
- **Changes over time.** A renovation roadmap doesn't copy the building per step: each measure lists its changes to
  the building document (a field set, an item added or removed, by id). Three rules keep a building and its roadmaps
  in step:
  1. The state after a planned step is the building document with the changes of every planned step up to it applied
     in order. A reader that shows a later state derives it this way and never writes it into the building document.
  2. A writer that marks a step `done` applies the step's changes to the building document in the same revision,
     after correcting them to what was built. The building is then again as it stands: what the step built is
     `existing`, and what it removed is no longer in the document. The revision names the step (`appliedSteps`), so
     its parent holds the building before the step. Where documents are exchanged without a store, the roadmap and
     the building are written together.
  3. A reader never applies a done step, whose changes are the record of what was done, nor a declined one.
     A renovation from before the building was recorded is a done step without changes.
- **Other major versions.** Each schema describes major version 1 of its document (`"schema": "1.x"`, store format
  `bzf/1.x`). A document or revision of another major version is only required to say which, and a reader opens the
  project read-only.

## Privacy

Every document under `Daten/` has a class (`x-bzf-class` in its schema; the apps' own documents by the root's
`classes`):

- **building**: the building, its plan states and imports, roadmaps, consumption, received files, result records.
  Shared with another
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

Versions: the store format (`bzf/1` in the project file and every revision) changes with the folder or the store; each
document's `schema` (`"1.0"`) changes with the document, a minor version only adding; the spec edition (`0.3.0`
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

### The `bzf` program

`tools/bzf` is a program for a project of your own, and a library (Rust) with the same rules. It needs no
Bauzertifikate app, account or service and fetches nothing.

```sh
cargo install --locked --path tools/bzf     # or: cd tools && cargo run -p bzf -- <command>

bzf check "examples/Haus am Hang.bzf"       # a folder or a .bzf file
bzf pack --share examples/haus-am-hang "Haus am Hang.bzf"
bzf unpack "Haus am Hang.bzf" haus
bzf hash examples/haus-am-hang/Daten/received.json
bzf verify examples/haus-am-hang/Ergebnisse/Energieberatung/Energieausweis/ergebnis.json
```

| Command | What it does |
|---|---|
| `bzf check <folder \| file.bzf>` | Checks the project file, every document under `Daten/` against its schema, the references between documents, each result (current, stale or not to be told; its signatures) and each issue (its result and inputs). Ends with 1 if a rule is broken |
| `bzf pack [--history \| --share] <folder> <file.bzf>` | Writes the folder as one `.bzf` file: a snapshot without the store, `--history` with it, `--share` only what may leave the organisation |
| `bzf unpack <file.bzf> <folder>` | Unpacks into a folder that is new or empty, refusing what the rules for reading refuse |
| `bzf hash <document.json>` | The hash that results and issues name the document by ([HASHES.md](HASHES.md)) |
| `bzf sign <ergebnis.json> --name … --key … --cert …` | Signs a result record ([SIGNATURES.md](SIGNATURES.md)) |
| `bzf verify <ergebnis.json>` | Verifies the signatures beside a result record and names who signed. Ends with 1 if one does not hold |
| `bzf version` | The specification edition the program checks by |

It ends with 0 when all is well, 1 when a rule is broken or a signature does not hold, and 2 when it could not
do what was asked (a file missing, a wrong argument). What `bzf check` does not check yet: the store under
`.bauzertifikate/` (objects against their hashes, revisions, heads) and the contents of the apps' own documents,
which it lists as notes.

The check also covers the expected solids in [`examples/solids/`](examples/solids/): each solid closed and looking
outwards, lying where the building document puts its item, every item covered or its absence explained by a rule,
and the volumes compared with figures worked out by hand. It does this without the program that made the files;
[SOLIDS.md](SOLIDS.md) has the rules and the tolerance.

[`examples/haus-am-hang/`](examples/haus-am-hang/) is a fictional house of 1968, with a gas boiler, consumption,
an issued certificate, renovation steps, costs and consent records, two result records, one of them made with a
program's settings document and signed by a made-up Energieberater. `examples/Haus am Hang.bzf` is that project as
`bzf pack --share` writes it: the building, without the people, the costs and the issues.
[`examples/werkstatt-am-bach/`](examples/werkstatt-am-bach/) is a fictional workshop of 1995, a building document
only, drawn for the solids' rules: inner walls, a separation line, a monopitch roof plate with an overhang. People,
projects and records in the examples are fictional. Referenced photographs, plans, invoices and issued PDFs are not
included.

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

BZF is the format's public name. `.bzf` is the extension of a project in one file, and `.bzfproj` that of a project
folder's project file. Independent implementations may describe their BZF support. Forks should identify their changes and must not imply that they are official Bauzertifikate releases.

## Kurzfassung

BZF (Bauzertifikate Format) ist ein offenes, versioniertes Gebäudedatenformat. Es beschreibt Gebäude, Bauteile,
Anlagentechnik, Sanierungsschritte und ausgestellte Dokumente als JSON. Andere Programme können es unabhängig von
Bauzertifikate lesen und schreiben. Ein Projekt liegt als Ordner vor oder als einzelne Datei `<Name>.bzf`. Das
Gebäudedokument beschreibt das Gebäude immer so, wie es steht; geplante Sanierungsschritte stehen im Fahrplan.
Die Schemas und Beispiele sind frei verfügbar; Beiträge und Forks sind willkommen.
Mit dem Programm `bzf` in diesem Repository lässt sich ein Projekt prüfen, packen, entpacken und ein Ergebnis signieren.
Die Spezifikation ist in Version 0.3.0, mit ausdrücklich gekennzeichneten stabilen und vorläufigen Teilen.
