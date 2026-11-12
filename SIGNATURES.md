# Signatures over a result record

A result record ([`result.json`](schema/bzf/1/result.json); README, "Results beside the model") can be signed.
A signature shows two things: who signed these figures, and that the record is unchanged since. This text says how
a signature is written and verified.

Status: **draft**, as the result record is. It is implemented in `tools/bzf` (`bzf sign`, `bzf verify`,
`bzf check`), and the example project carries a signature made with a certificate created for the example.

## What a signature says, and what it does not

- **It says who signed.** The signature is made with a key, and the certificate that travels with it names the
  key's holder. Whether that name is to be believed is the reader's decision: by whom the certificate was issued,
  whether the reader knows the certificate, whether it has been withdrawn.
- **It says the record is unchanged.** A signature holds for exactly one result record. Any change to the record's
  content breaks it; writing the same record another way (indentation, the order of the keys) does not.
- **It says nothing about entitlement.** Not that the signer may issue energy certificates, not that the signer is
  listed anywhere. The format has no such check and does not plan one.
- **It does not say the figures are right.** It ties them to the documents they were made from (the record's
  `inputs`), the program and the kernel version, so that someone else can calculate again and compare.
- **It does not sign the building.** The record names its inputs by hash, so the signature covers which documents
  the figures were made from. Whether the project still holds those documents is the question of "current or
  stale" ([HASHES.md](HASHES.md)), and a reader answers both.

## Where it lies

A signature is a file of its own beside the record, never inside it:

```
Ergebnisse/Energieberatung/Energieausweis/ergebnis.json
Ergebnisse/Energieberatung/Energieausweis/ergebnis.energieberater.jws
```

- The file's name is `ergebnis.<name>.jws`. `<name>` tells one signature from another and consists of ASCII
  letters, digits, `-` and `_`. It has no meaning to a reader: who signed is in the certificate.
- Several signatures may lie beside one record, each over the same record: the Energieberater's, the program's,
  a calculation service's.
- A writer that replaces a result record removes the signature files beside it. They would no longer hold.
- A signature belongs to its record: a share passes both on ("Privacy" in the README), and an issue that names a
  result by hash names the record a signature holds for.

## The file

The file holds a JSON Web Signature ([RFC 7515](https://www.rfc-editor.org/rfc/rfc7515)) in the compact
serialization with the payload detached (RFC 7515, Appendix F): two base64url parts with nothing between the two
full stops, in ASCII, optionally followed by one line end.

```
BASE64URL(protected header) .. BASE64URL(signature)
```

**The payload**, which is not in the file, is the record's canonical JSON as UTF-8 octets
([HASHES.md](HASHES.md)). The signature is made over the ASCII octets of

```
BASE64URL(protected header) . BASE64URL(canonical JSON of ergebnis.json)
```

**The protected header** is a JSON object with exactly these members:

| Member | Value |
|---|---|
| `alg` | The signature algorithm (RFC 7518). `ES256`: ECDSA on the curve P-256 with SHA-256, the signature being the 64 octets of `r` and `s` |
| `iat` | The time the signer claims to have signed at: an integer, seconds since 1970-01-01T00:00:00Z, without fractions |
| `x5c` | The signer's certificate first (X.509, DER, in standard base64 with padding, as RFC 7515, 4.1.6 says), then optionally the certificates of its issuers |

A header for the example, before it is encoded:

```json
{"alg":"ES256","iat":1792500300,"x5c":["MIIBxT…"]}
```

A writer writes no other member. In particular no `crit`, `b64` or `sigD` (they change what is signed or how a
reader must treat the header), no `kid` or `x5u` (the certificate travels with the signature, so nothing is
fetched), and never `x5t` (SHA-1).

## Verifying

A reader verifies a signature file against the record beside it in these steps:

1. Split the file at its full stops. There are three parts and the middle one is empty; otherwise the file is not a
   signature of this profile.
2. Decode the first part (base64url without padding) to the header, a JSON object. Refuse a header with `crit`,
   `b64` or `sigD`, one without `alg`, `iat` or `x5c`, and one whose `alg` is `none` or starts with `HS`. Any
   other member the reader does not know is ignored (RFC 7515, 4).
3. Take the key from the first certificate in `x5c`.
4. If `alg` is not one the reader verifies, say so and stop: the signature is neither shown to hold nor shown to
   be broken. Every reader that verifies signatures verifies `ES256`.
5. Write the record as canonical JSON, form the octets signed (above) from the first part **as it stands in the
   file** and the record, and verify the signature with the key.
6. Report who signed (the certificate's subject), who issued the certificate, and the claimed time; and whether
   that time lies within the certificate's validity.

A signature that does not hold in step 5 is an error: the record or the file was changed. `bzf check` ends with 1
on it. An algorithm the reader does not verify (step 4) is a note, not an error.

## Trust is the reader's

This profile shows which key signed and what the certificate says about its holder. It deliberately stops there:

- No certificate chain is built and no trust list is read. `bzf verify` tells a certificate the signer made
  themselves ("of the signer's own") from one issued by someone else, and names the issuer.
- No revocation is asked for (CRL, OCSP).
- The time is the signer's claim (`iat`), not a time stamp by a third party.

A reader for whom this matters applies its own policy to the certificate in `x5c`: a bank may accept only
certificates it issued or knows, an office may keep a list of its Energieberater's certificates. A qualified
electronic signature under eIDAS is possible with a qualified certificate and a reader that validates it; this
profile neither requires nor checks one.

## Relation to JAdES

The header members and the serialization follow the requirements for a JAdES baseline signature of level B-B in
ETSI TS 119 182-1 V1.2.1 (2024-07): a compact JWS, the signing certificate in the protected header by `x5c`
(clause 5.1.7 asks for one of `x5t#S256`, `x5c`, `sigX5ts`, `x5t#o`), the claimed signing time as `iat`
(clause 5.1.11; `sigT` may no longer be written since 2025-07-15), and a detached payload without `sigD`, as
RFC 7515, Appendix F describes it, so that a plain JWS library can verify the signature.

This is a statement about how the profile was written. The signatures `bzf sign` makes have **not been run
through a JAdES conformance checker**, and this text makes no claim that a given validation service accepts them.
What was checked: a signature made by `bzf sign` verifies with an independent implementation (Node.js'
`crypto.verify` over the octets described above), and a signature made by that implementation verifies with
`bzf verify`.

## The example

[`examples/haus-am-hang/`](examples/haus-am-hang/) has one signature,
`Ergebnisse/Energieberatung/Energieausweis/ergebnis.energieberater.jws`, over the result behind the issued
certificate. Its certificate names a made-up office ("Ingenieurbüro Beispiel (made up)") and was created by the
signer themselves for this example; the key was deleted after signing and is in no repository.

```sh
bzf verify "examples/haus-am-hang/Ergebnisse/Energieberatung/Energieausweis/ergebnis.json"
```

Signing needs a P-256 key in PKCS #8 (PEM) and its certificate (PEM). For a trial, OpenSSL makes both:

```sh
openssl ecparam -name prime256v1 -genkey -noout | openssl pkcs8 -topk8 -nocrypt -out key.pem
openssl req -new -x509 -utf8 -key key.pem -out certificate.pem -days 365 -subj "/O=My office/CN=My name"
bzf sign path/to/ergebnis.json --name energieberater --key key.pem --cert certificate.pem
```

A key that signs real results is kept where the office keeps its other keys, and never in a project folder.
