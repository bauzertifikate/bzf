# Hashes and canonical JSON

BZF names things by hash: a received file, an object in the store, a document a result was made from, the result
an issue issued. This text says which hash is meant where, and how a document's hash is formed, so that two
programs in two languages get the same one. [`vectors/canonical.json`](vectors/canonical.json) has test vectors.

Status: the rules for a document's hash are part of the stable store format (`revision.json` names documents this
way since edition 0.1). This text and the vectors are new in edition 0.3.0 and change nothing in them.

## Two hashes

Every hash in BZF is a SHA-256 (FIPS 180-4), written as 64 lower-case hexadecimal digits
(`common.json` `$defs/hash`).

| Hash of | Taken of | Where it is used |
|---|---|---|
| A file | The file's octets as they are | `received.json` `sha256`; an issue's `files[].sha256`; a head's `results.<path>.sha256` |
| A document | The UTF-8 octets of the document's canonical JSON | A revision's `tree`; `inputs` in a result record, an issue and a head's result entry; an issue's `result`; a revision's own name |

A document is any JSON document of a project: one under `Daten/`, a result record, a revision. Its hash does not
depend on how the file is written (indentation, line ends, the order of the keys, `6.0` or `6`), only on what it
says.

A result record therefore has two hashes, and they differ. A writer's head names the file `ergebnis.json` by its
octets, as it names every result file. An issue names the record by the hash of its canonical JSON
(`versand.json` `result`), as documents are named.

## Canonical JSON

A document's canonical JSON is the JSON Canonicalization Scheme of [RFC 8785](https://www.rfc-editor.org/rfc/rfc8785),
without additions. In short:

- No whitespace between tokens.
- An object's members are sorted by their names, compared as arrays of UTF-16 code units (so `"ö"` comes before
  `"€"`, and a character beyond U+FFFF is compared by its two surrogates). Arrays keep their order.
- A number is written as ECMAScript's `Number.prototype.toString` writes the IEEE 754 double nearest to it: `6` for
  `6.0`, `0.035`, `1e+21`, `1e-7`, `0` for `-0`. A number is read as a double before it is written.
- A string is written in UTF-8 with only these escapes: `\"`, `\\`, `\b`, `\t`, `\n`, `\f`, `\r`, and `\u00xx` in
  lower case for the other characters below U+0020. Every other character is written as it is, `/` and characters
  beyond ASCII too.
- `true`, `false` and `null` as they are.

The hash is taken of the canonical form's UTF-8 octets, without a byte order mark and without a line end after it.

## What a document must be

RFC 8785 works on I-JSON ([RFC 7493](https://www.rfc-editor.org/rfc/rfc7493)). For a document's hash to be the
same everywhere:

- **No name twice in one object.** Readers differ in which of two members they keep.
- **Numbers are doubles.** An integer beyond ±2⁵³ (9007199254740992) is not kept exactly: it is hashed as the
  double nearest to it, and so is its neighbour. BZF's lengths in millimetres and its amounts are far below that.
  An identifier that is longer is written as a string.
- **Strings are Unicode.** No lone surrogate (`"\ud800"` alone).

A writer of BZF writes its documents within these limits. `bzf check` reports a name that is twice in one object
and a string that is not Unicode as errors, and `bzf hash` gives no hash for such a document.

## What is not done

- **Text is not normalised.** `é` as one character (U+00E9) and as `e` with a combining accent (U+0065 U+0301)
  are different strings and give different hashes. A writer keeps a text as it received it.
- **Defaults are not filled in, and nothing is left out.** A field that is absent and the same field with its
  default value are different documents. Every member counts, the ones a reader does not know too: a reader keeps
  them (README, "Compatibility rule"), so a document read and written back has the hash it had.
- **Conflicts are not settled.** A document holding `{"$conflict": …}` is hashed as it lies. Only validation
  settles conflicts by their first value.

## Stale or current

A result record, an issue and a head's result entry name the documents they were made from as `inputs`: the
document's path under `Daten/` → its hash. For each named document a reader finds one of three things:

| The document | Means |
|---|---|
| is there and has the named hash | unchanged since |
| is there and has another hash | changed since: the result is stale |
| is not there | cannot be told here; in a project with its store, the document was removed and the result is stale |

The third case is its own because a share ("Privacy" in the README) passes on documents of the class `building`
only. A result that names a document of another class is neither current nor stale for the receiver, and a reader
says which document is missing.

## Test vectors

[`vectors/canonical.json`](vectors/canonical.json) (CC0, as the examples) has:

- `numbers`: pairs of a double's bits (IEEE 754, 16 hexadecimal digits) and the number as canonical JSON writes it.
  They are the samples of RFC 8785, Appendix B, among them the smallest and largest doubles, the limits of the
  plain form at 1e-6 and 1e21, and a value where two shortest digit strings are equally near.
- `documents`: a JSON text, its canonical form, and the SHA-256 of that form. They cover the RFC's own example
  (section 3.2.2), the order of names, number forms as documents write them, whitespace and nesting, and the
  escapes.

The vectors were written with a second implementation (an ECMAScript engine's `JSON.stringify` over sorted names,
which RFC 8785 is defined by) and are checked by `./check.sh` against the implementation in `tools/bzf`. The
example project gives whole documents: every hash its results and issues name is one `bzf hash` gives for the
named document.

## Doing it

```sh
bzf hash "examples/haus-am-hang/Daten/buildings/019a5e00-0000-7000-8000-000000000002.json"
```

In a language with an ECMAScript engine, the whole rule is a few lines, because `JSON.stringify` writes numbers and
strings as RFC 8785 asks (for documents within the limits above):

```js
const canonical = (v) =>
  Array.isArray(v) ? "[" + v.map(canonical).join(",") + "]"
  : v && typeof v === "object"
    ? "{" + Object.keys(v).sort().map((k) => JSON.stringify(k) + ":" + canonical(v[k])).join(",") + "}"
  : JSON.stringify(v);
// hash = SHA-256 of the UTF-8 octets of canonical(JSON.parse(text)), in lower-case hexadecimal
```

In other languages, the number form is the part to take from a library that implements ECMAScript's rule (the
shortest digits that read back as the same double, with ties to even). RFC 8785 names open-source implementations in an appendix.
