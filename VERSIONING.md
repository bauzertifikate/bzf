# Versioning and compatibility

BZF has three independent version identifiers:

| Identifier | Meaning | Current value |
|---|---|---|
| Specification edition / Git release tag | A complete, reproducible snapshot of schemas, documentation and tests | `0.3.0` / `v0.3.0` |
| Document `schema` | The version of that document's data contract | `1.0` |
| Store `format` | The project folder and revision format | `bzf/1` |

The separate EPBD proposal uses `bzf-epbd/0.1`. Its version is independent of the main BZF specification.

## Stable contracts

Within a stable document major version:

- Existing valid documents remain valid.
- Existing fields retain their meaning, units and interpretation.
- Additions are optional. A new required field, a narrower accepted range, a field removal or rename, or a changed
  unit is a breaking change and requires a new major version.
- New values must follow the existing open-value convention. Readers preserve values they do not understand.
- Changes to defaults that alter existing documents' interpretation are breaking changes too.

Schema validity alone does not prove compatibility. Changes must also be reviewed for meaning, defaults, round-trip
preservation and their effect on existing readers. Add regression fixtures for changed behavior.

Readers and writers preserve unknown fields and values. They do not migrate documents merely by opening them.
Newer major versions are opened read-only where supported by the project's compatibility rules. Preserving an unknown
feature does not mean understanding it: a calculator or other processor must report unsupported features that affect
its output, rather than silently ignoring them. Unresolved conflicts likewise need an explicit handling policy.

## Drafts before specification 1.0

The initial public release includes schemas marked `x-bzf-status: draft`. These are not yet stable contracts and may
change incompatibly before specification 1.0. Every such change must be described in the changelog with migration
guidance, and requires a new specification minor edition. Stable schemas keep their compatibility promise even
while the overall specification is 0.x.

Once specification 1.0 is released, compatible additions use minor specification releases and editorial corrections
use patch releases. Breaking format changes require a major specification release and the relevant document/store
major-version changes. During 0.x, patch releases must not break either draft or stable contracts.

## Reproducible releases

Release tags are immutable: do not move, replace or reuse them. Corrections get a new release.
Consumers should pin a release tag or, for exact dependency pinning, its commit SHA. `main` is development work.

The canonical `$id` URLs identify schema families. The repository retains the existing IDs under
`https://bauzertifikate.de/schema/`; deployment of those URLs is separate from this repository's release.
For a reproducible validator, load the schema files from a pinned release and register them locally by `$id`.
Do not depend on a mutable website response to reproduce an old validation result.

Before releasing, run `./check.sh`, update `CHANGELOG.md`, and state any draft changes, affected document versions,
migration steps and newly supported features. Keep old tags and their schema files available.
