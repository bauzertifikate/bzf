# Changelog

## Unreleased

### A reference implementation of the solids

- New library `bzf-solids` in [`tools/solids`](tools/solids/): the solids of a building document by the rules of
  [SOLIDS.md](SOLIDS.md), the items without a solid with their rule, and the expected-solids file written from both.
  Apache-2.0 OR MIT, as the other code in `tools/`. It is the code the expected solids in `examples/solids/` were
  made with; it writes them again byte for byte. The text stays the rule: where the two differ, the library is wrong.
- New command `bzf solids <folder | file.bzf | building.json>`: prints a building's solids in the format of
  `examples/solids/`.
- `./check.sh` makes the expected solids again with the library and compares them within the files' tolerance
  (`tools/check/tests/reference.rs`), besides checking them without it as before. The library's own tests hold it
  to the rules on small buildings, including cases the examples do not cover yet.
- No schema, rule or example changes.

### Facts an energy model needs (drafts)

Four additions, all optional: a document valid under 0.3.0 stays valid, and a reader of 0.3.0 reads a document
that uses them (it keeps what it does not know, and takes an `app` change for the newer kind of change it already
keeps). Each is a draft until specification 1.0.

- `roadmap.json`: the change op `app`, `{"op": "app", "app": <folder>, "change": {…}}`, a change that only the named
  app's own document model knows. What it changes lies in that app's documents under `Daten/<app>/`, never in the
  building document. A reader that knows the app and has its documents applies it; any other reader keeps it as it
  came and treats the state after its step, every later state and the target as unknown, deriving and showing no
  figures for them. A writer that does not know the app does not mark such a step done. README, "Changes over
  time", has the rule; [`vectors/states.json`](vectors/states.json) has cases of which states a reader can derive.
- `common.json` `money`: `source`, where an amount comes from (an offer among the received files, a cost database).
  `costs.json`: a funding entry's `source`, where its rate and amount come from. The measure's `source` applies to
  the figures that have none of their own.
- `consumption.json`, a period: `calorificBasis` (`gross` or `net`: the basis of the period's kWh and of its
  calorific value), `calorificValue` (kWh per unit of the amount, as the bill states it), `woodType` (`hardwood`,
  `softwood`, `mixed`, for log wood billed by volume), `hotWater` (the share of the amount that heated water, as a
  bill states it) and `hotWaterUnit` (its unit where it is not the amount's).
- `common.json` `layer`: `parts`, an inhomogeneous layer (rafters and insulation) as its materials side by side by
  area fraction; `air`, an air layer's ventilation (`unventilated`, `slightlyVentilated`, `wellVentilated`, EN ISO
  6946); `airOpenings`, its measured openings. A part can be an air cavity (`air` on the part). `material`: `mu`,
  the water vapour diffusion resistance factor, one value or the pair a catalogue states.
- How hot water shows in the consumption needed no new field: a meter's `services` say whether its amounts hold
  the energy that heated water. Their description now says so, and what it means when every meter lists its
  services and none lists `hotWater` (water heated apart from this consumption, decentrally).
- `bzf check` reports what a document says against itself that its schema cannot catch: parts of a layer that do not
  add up to 1, a μ pair with the higher value first, a hot-water share larger than its amount in the same unit. The
  library has the rule for which states of a roadmap a reader can derive (`bzf::project::states`).
- The examples use the additions: in Haus am Hang the hydraulic balance is an `app` change of the Energieberatung's
  model, the wall's insulation build-up has μ, the plasterer's offer is a received file that the wall's investment
  names, the anyway costs and the funding say where they come from, and the gas bills their calorific basis, one of
  them in m³ with its calorific value. The workshop's roof insulation lies between battens, as an inhomogeneous
  layer, and its wall has μ. The Sanierungsfahrplan's result and the two issues made from the roadmap name its new
  hash, and `examples/Haus am Hang.bzf` is packed again.
- `./check.sh` validates the examples' roadmap, costs, consumption and workshop against the schemas of 0.3.0, kept
  unchanged in `tools/check/tests/edition-0.3.0/`, and checks every addition with valid and invalid fixtures.

Migration:

- A change of the op `app` without `app` (one path segment) or without an object `change` was valid under 0.3.0 as
  a newer kind of change and is not from 0.4. No writer is known to have written one.
- A writer that uses one of the additions writes the next minor version of the document's schema: the examples'
  roadmap, costs and consumption are `1.1`, the workshop's building `1.2`.
- A layer with `parts` or `air` SHOULD give its own material no `lambda`, so that a reader that does not know the
  addition finds the layer's conductivity not given rather than a wrong one.

## 0.3.0 — 2026-11-12

The first edition with a program to use it by: `bzf` checks a project, packs and unpacks the `.bzf` file, names a
document by its hash and signs a result. Documents that were valid under 0.2.0 stay valid; "Migration" below has
the two things a writer does differently.

### Results beside the model (draft)

- New schema `result.json` for a result record, `Ergebnisse/<app>/<title>/ergebnis.json`: the figures a method
  worked out per state, the documents they were made from by the hash of their canonical JSON, the method, the
  program and kernel version, the program's settings (`choices`) and what the method assumed (`assumptions`). Class
  `building`. README, "Results beside the model", has the rules: who writes it, when it is stale, what it shows.
- A result is current, stale, or not to be told: where a document it names is not in the project (a share leaves a
  program's settings behind), a reader says that it cannot tell, and does not call the result stale.
- `versand.json`: an issue can name the result it issued (`result`, a hash). The figures' definition moved unchanged
  to `common.json` `$defs/figures`, which `result.json` shares.
- `revision.json`: a head's result entry can name the documents the file was made from (`inputs`), so a result is
  stale only when one of them changed, and not with every later revision.

### How a document is named by its hash

- New text [HASHES.md](HASHES.md): the two hashes (a file's octets, a document's canonical JSON), canonical JSON
  as RFC 8785 has it, what a document must be for readers to agree on its hash, and when a result is stale.
- New: no name twice in one object, in any document. JSON allows it and parsers disagree on it, so such a document
  has no hash readers agree on. `bzf check` reports it as an error.
- New test vectors [`vectors/canonical.json`](vectors/canonical.json): the number samples of RFC 8785, Appendix B,
  and documents with their canonical form and hash, for an implementation in any language.

### Signatures over a result (draft)

- New text [SIGNATURES.md](SIGNATURES.md). 0.2.0's draft only reserved the file name. A signature is a file
  `ergebnis.<name>.jws` beside the record: a JSON Web Signature in the compact serialization with the payload
  detached, over the record's canonical JSON, by ES256, with the signer's certificate in the header (`x5c`) and the
  claimed time (`iat`). The header follows the requirements of a JAdES baseline signature (ETSI TS 119 182-1); the
  signatures were not run through a JAdES conformance checker.
- A signature states who signed and that the record is unchanged. It states nothing about entitlement, and trust
  in the certificate is the reader's decision: no chain is built and no revocation asked for.

### A project in one file

- The `.bzf` file is no longer "a draft until an implementation writes and reads it": `bzf pack` and `bzf unpack`
  do, and `examples/Haus am Hang.bzf` is the example project as a share.
- The rules for reading say in full what a reader refuses: a name that leaves the folder (absolute, a drive
  letter, a backslash, a control character, a `..`, `.` or empty part), an encrypted entry, a link, a compression
  other than stored or deflated, a name twice without regard to case. 0.2.0 named only absolute names and `..`.
- The rules for passing on say what a share holds: the project file, the documents of the class `building`, the
  received files that `received.json` names, the result records with their signatures; nothing from `Versendet/`
  and no store.

### The `bzf` program

- New `tools/bzf`, a program and a Rust library: `bzf check`, `pack`, `unpack`, `hash`, `sign`, `verify`,
  `version`. README, "Checking". It checks a folder or a `.bzf` file against the schemas and the rules between
  documents: references, a result's inputs, an issue's result and figures, signatures. It does not check the store
  yet.
- `./check.sh` runs the program's tests with the conformance tests, which now use the library's rules instead of
  a copy of them.

### The example

- Haus am Hang: two result records, one behind the certificate and one behind the roadmap; a settings document of
  the program that the certificate's result names among its inputs; a signature by a made-up Energieberater, whose
  key is in no repository. The three issues name their inputs by the documents' real hashes; in 0.2.0 these were
  made-up values.

### Migration

- A writer that keeps a result writes it as a result record and names its inputs. A head's result entry without
  `inputs` stays valid and is current only at its revision, as before.
- A writer that replaces a result record removes the `ergebnis.*.jws` files beside it.
- A document with a name twice in one object was never one that two readers read alike; a writer that produced
  one writes it again with each name once.

## 0.2.0 — 2026-11-12

### A project in one file

- A project can be passed on as one file, `<Name>.bzf`: a ZIP archive of the project folder, with or without its
  store. README, "A document, a file, a folder", has the rules. This form is a draft until an implementation writes
  and reads it.
- The project file that marks a project folder is now `<Name>.bzfproj`. It was `<Name>.bzf`. Its content, its schema
  (`bzf.json`, under the same `$id`) and the store format `bzf/1` are unchanged.

The second point changes the folder layout that 0.1.0 described. Migration:

- A reader accepts both names for the project file. It tells an older project file from a `.bzf` archive by the
  first bytes: the project file is JSON and starts with `{`, an archive starts with `PK`.
- A writer names a new project's file `<Name>.bzfproj`. It renames an older `<Name>.bzf` with the next revision it
  writes, never on opening.

### The store's directory

- The store's directory keeps its name, `.bauzertifikate/`. 0.1.0 announced a rename to `.bzf/` before spec 1.0;
  that rename is dropped, so no reader has to look for two names.

### The building as it stands

- A building document describes the building as it stands, in every revision, and holds no later state
  (`building.json`; README, "One state"). An item's `phase` covers work under way. What is planned for later lies in
  a roadmap.
- `roadmap.json` now says what a step's status means for its changes. A planned step's changes are still to be made,
  and a reader applies them to show the building after it. A done step's changes are in the building document and are
  not applied again. A declined step's changes are never applied.
- A writer that marks a step done applies its changes to the building document in the same revision, corrected to
  what was built. The revision names the step in the new optional field `appliedSteps` (`revision.json`, draft until
  spec 1.0), so the building before the step is the one in the revision's parent.

Documents that were valid under 0.1.0 stay valid. Migration: 0.1.0 left open whether a done step's changes were
already in the building document. If a roadmap has a done step whose changes the building document does not show,
its writer applies them in a revision that names the step in `appliedSteps`. A renovation from before the building
was recorded stays a done step without changes.

### Facts, not results

- README, "Facts, not results", states the rule the schemas already followed: the model stores facts as entered or
  received, and nothing a method works out from them. What was issued is kept as issued in `Versendet/`. No schema
  changed for this.

### Contributing without reading

- README, conformance: a writer need not read every document. It can add its own documents and received files to a
  project and carries the documents it does not read into its revision unchanged. No schema changed for this.

### Conformance tests

- The tests accept a project file under both names, cover `appliedSteps`, and check in the examples that no planned
  step adds an item the building document already holds.

### The solids of a building

- [SOLIDS.md](SOLIDS.md), a draft: the rules that turn a building document's walls, slabs, roof plates and openings
  into solids. Wall faces from the placement, mitred and square ends, ends cut back at a wall running past, bottom
  and top, the roof's cut, openings, layers, slabs, roof plates, items made of elements, and the items that have no
  solid. Its limits are stated: straight walls, planar roof plates, rectangular openings.
- `examples/solids/`: the expected solids of the example buildings, one file per building, with the items that have
  no solid and the rule that leaves each without. Tolerance: corners within 1 mm, volumes within 0.1 %.
- `examples/werkstatt-am-bach/`: a second fictional example, a workshop of 1995, for the rules the house does not
  exercise.
- `./check.sh` checks the expected solids without the program that made them (`tools/check/tests/solids.rs`): closed,
  looking outwards, where the document puts the item, every item covered or explained, volumes against figures
  worked out by hand.

### Editorial

- The description of `bzf-epbd/0.1` no longer names the paths of one implementation; it points to MAPPINGS.md.

## 0.1.0 — 2026-11-10

Initial standalone public edition of BZF (Bauzertifikate Format).

- Thirteen BZF JSON Schemas covering projects, buildings, revisions, imports, renovation roadmaps, consumption,
  costs, parties, received files and issued-document records.
- A standalone draft EPBD dataset schema and an informative EPBD/iSFP mapping.
- Fictional example projects, an IFC import IDS definition and independent conformance tests.
- Explicit compatibility, versioning and contribution policies.

The schema and example data are unchanged from the initial internal specification. The private schema generator
and application build dependencies are not part of this repository. Some document schemas remain drafts; see README.
