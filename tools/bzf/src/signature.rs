// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Signatures over a result record (SIGNATURES.md): a JWS in compact form with the payload detached, the payload
//! being the record's canonical JSON, the signer's certificate in the protected header.
//!
//! A signature says who signed. It says nothing about that person's or organisation's entitlement to issue, and this
//! program neither builds a certificate chain nor asks whether a certificate was revoked: whom to trust is the
//! reader's decision.

use crate::canonical::canonical;
use base64ct::{Base64, Base64UrlUnpadded, Encoding};
use p256::ecdsa::signature::{Signer as _, Verifier as _};
use p256::ecdsa::{Signature, SigningKey, VerifyingKey};
use p256::pkcs8::{DecodePrivateKey, DecodePublicKey};
use serde_json::{Map, Value, json};
use x509_cert::Certificate;
use x509_cert::der::{Decode, DecodePem, Encode};

/// The one algorithm every reader verifies: ECDSA on P-256 with SHA-256 (RFC 7518, 3.4).
pub const ALGORITHM: &str = "ES256";

/// Who signed a result record, as the signature's certificate and header state it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signer {
    /// The certificate's subject, as RFC 4514 writes a name.
    pub subject: String,
    /// The certificate's issuer; the subject itself where the signer made the certificate.
    pub issuer: String,
    /// The time the signer claims to have signed at, in seconds since 1970-01-01T00:00:00Z.
    pub time: i64,
    /// Whether that time lies within the certificate's validity.
    pub in_validity: bool,
}

/// Why a signature file does not show who signed the record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Failure {
    /// The file is not a signature of this profile.
    Malformed(String),
    /// The file is one, by an algorithm this program does not verify.
    Unsupported(String),
    /// The signature is not the certificate's key's over this record: the record or the file changed since.
    Invalid,
}

impl std::fmt::Display for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Failure::Malformed(why) => write!(f, "not a signature of this profile: {why}"),
            Failure::Unsupported(alg) => write!(f, "signed with {alg}, which this program does not verify"),
            Failure::Invalid => write!(f, "does not hold for the record as it stands"),
        }
    }
}

fn malformed(why: impl Into<String>) -> Failure {
    Failure::Malformed(why.into())
}

/// The octets a signature is made over (RFC 7515, 5.1 and Appendix F): the protected header as it stands in the
/// file, a full stop, and the record's canonical JSON, both base64url without padding.
fn signing_input(header: &str, record: &Value) -> String {
    format!("{header}.{}", Base64UrlUnpadded::encode_string(canonical(record).as_bytes()))
}

/// A signature over `record` by the key in `key` (PKCS #8, PEM) for the certificate in `certificate` (PEM), claiming
/// `time` (seconds since 1970) as the time of signing: the contents of `ergebnis.<name>.jws`.
pub fn sign(record: &Value, key: &str, certificate: &str, time: i64) -> Result<String, String> {
    let key = SigningKey::from_pkcs8_pem(key).map_err(|e| format!("the key is not a P-256 key in PKCS #8: {e}"))?;
    let certificate = Certificate::from_pem(certificate).map_err(|e| format!("the certificate: {e}"))?;
    if verifying_key(&certificate).ok().as_ref() != Some(key.verifying_key()) {
        return Err("the certificate is not this key's".into());
    }
    let der = certificate.to_der().map_err(|e| format!("the certificate: {e}"))?;
    let header = json!({ "alg": ALGORITHM, "iat": time, "x5c": [Base64::encode_string(&der)] });
    let header = Base64UrlUnpadded::encode_string(canonical(&header).as_bytes());
    let signature: Signature = key.sign(signing_input(&header, record).as_bytes());
    Ok(format!("{header}..{}", Base64UrlUnpadded::encode_string(&signature.to_bytes())))
}

/// Who signed `record`, by the signature file's contents `jws`.
pub fn verify(record: &Value, jws: &str) -> Result<Signer, Failure> {
    let parts: Vec<&str> = jws.trim_end_matches(['\r', '\n']).split('.').collect();
    let [header, "", signature] = parts[..] else {
        return Err(malformed("not a compact JWS with its payload detached (header..signature)"));
    };
    let decoded = Base64UrlUnpadded::decode_vec(header).map_err(|_| malformed("the header is not base64url"))?;
    let fields: Map<String, Value> =
        serde_json::from_slice(&decoded).map_err(|_| malformed("the header is not a JSON object"))?;
    // A reader that does not understand a critical header parameter must refuse (RFC 7515, 4.1.11). The profile uses
    // none, and none of the parameters that change what is signed.
    for parameter in ["crit", "b64", "sigD"] {
        if fields.contains_key(parameter) {
            return Err(malformed(format!("the header has `{parameter}`, which the profile does not use")));
        }
    }
    let algorithm = fields.get("alg").and_then(Value::as_str).ok_or_else(|| malformed("the header has no `alg`"))?;
    if algorithm == "none" || algorithm.starts_with("HS") {
        return Err(malformed(format!("`alg` is {algorithm}, which is no signature by a certificate's key")));
    }
    let time = fields.get("iat").and_then(Value::as_i64).ok_or_else(|| malformed("the header has no `iat`"))?;
    let chain = fields.get("x5c").and_then(Value::as_array).filter(|chain| !chain.is_empty());
    let first = chain.and_then(|chain| chain[0].as_str()).ok_or_else(|| malformed("the header has no `x5c`"))?;
    let der = Base64::decode_vec(first).map_err(|_| malformed("`x5c` is not base64"))?;
    let certificate = Certificate::from_der(&der).map_err(|_| malformed("`x5c` holds no certificate"))?;
    if algorithm != ALGORITHM {
        return Err(Failure::Unsupported(algorithm.to_string()));
    }
    let key = verifying_key(&certificate).map_err(malformed)?;
    let bytes = Base64UrlUnpadded::decode_vec(signature).map_err(|_| malformed("the signature is not base64url"))?;
    let signature = Signature::from_slice(&bytes).map_err(|_| malformed("the signature is not 64 octets"))?;
    key.verify(signing_input(header, record).as_bytes(), &signature).map_err(|_| Failure::Invalid)?;
    let tbs = certificate.tbs_certificate();
    let (from, to) = (tbs.validity().not_before.to_unix_duration(), tbs.validity().not_after.to_unix_duration());
    let in_validity = u64::try_from(time).is_ok_and(|time| from.as_secs() <= time && time <= to.as_secs());
    Ok(Signer { subject: tbs.subject().to_string(), issuer: tbs.issuer().to_string(), time, in_validity })
}

/// The P-256 key a certificate is for.
fn verifying_key(certificate: &Certificate) -> Result<VerifyingKey, String> {
    let spki = certificate.tbs_certificate().subject_public_key_info().to_der().map_err(|e| e.to_string())?;
    VerifyingKey::from_public_key_der(&spki).map_err(|_| "the certificate's key is not a P-256 key".to_string())
}

/// The name in a signature file's name, `ergebnis.<name>.jws`; none for any other file.
pub fn name_of(file: &str) -> Option<&str> {
    let name = file.strip_prefix("ergebnis.")?.strip_suffix(".jws")?;
    let plain = !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    plain.then_some(name)
}
