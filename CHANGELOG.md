# Changelog

## Unreleased (0.2.0)

Changes on `main` since 0.1.0. They are not released yet and can still change before the tag.

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

### Conformance tests

- The tests accept a project file under both names, cover `appliedSteps`, and check in the examples that no planned
  step adds an item the building document already holds.

## 0.1.0 — 2026-11-10

Initial standalone public edition of BZF (Bauzertifikate Format).

- Thirteen BZF JSON Schemas covering projects, buildings, revisions, imports, renovation roadmaps, consumption,
  costs, parties, received files and issued-document records.
- A standalone draft EPBD dataset schema and an informative EPBD/iSFP mapping.
- Fictional example projects, an IFC import IDS definition and independent conformance tests.
- Explicit compatibility, versioning and contribution policies.

The schema and example data are unchanged from the initial internal specification. The private schema generator
and application build dependencies are not part of this repository. Some document schemas remain drafts; see README.
