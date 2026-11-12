// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Signatures over a result record (SIGNATURES.md): made and verified here, and refused where the profile says so.
//! The keys are made in the test from fixed octets; they sign nothing outside it.

use base64ct::{Base64UrlUnpadded, Encoding};
use bzf::signature::{self, Failure, sign, verify};
use p256::ecdsa::{DerSignature, SigningKey};
use p256::pkcs8::{EncodePrivateKey, LineEnding};
use serde_json::{Value, json};
use std::str::FromStr;
use std::time::Duration;
use x509_cert::builder::profile::BuilderProfile;
use x509_cert::builder::{Builder, CertificateBuilder};
use x509_cert::der::EncodePem;
use x509_cert::der::asn1::GeneralizedTime;
use x509_cert::ext::Extension;
use x509_cert::name::Name;
use x509_cert::serial_number::SerialNumber;
use x509_cert::spki::{SubjectPublicKeyInfoOwned, SubjectPublicKeyInfoRef};
use x509_cert::time::{Time, Validity};
use x509_cert::{TbsCertificate, certificate::Rfc5280};

/// A certificate its holder made for itself, with no extensions: the least a signature of the profile needs.
struct Own(Name);

impl BuilderProfile for Own {
    fn get_issuer(&self, subject: &Name) -> Name {
        subject.clone()
    }
    fn get_subject(&self) -> Name {
        self.0.clone()
    }
    fn build_extensions(
        &self,
        _: SubjectPublicKeyInfoRef<'_>,
        _: SubjectPublicKeyInfoRef<'_>,
        _: &TbsCertificate,
    ) -> x509_cert::builder::Result<Vec<Extension>> {
        Ok(Vec::new())
    }
}

const FROM: u64 = 1_790_000_000;
const TO: u64 = 1_860_000_000;

/// A key and its own certificate for `subject`, valid from FROM to TO, both as PEM.
fn signer(seed: u8, subject: &str) -> (String, String) {
    let key = SigningKey::from_slice(&[seed; 32]).unwrap();
    let time = |seconds| Time::from(GeneralizedTime::from_unix_duration(Duration::from_secs(seconds)).unwrap());
    let validity: Validity<Rfc5280> = Validity::new(time(FROM), time(TO));
    let spki = SubjectPublicKeyInfoOwned::from_key(key.verifying_key()).unwrap();
    let profile = Own(Name::from_str(subject).unwrap());
    let builder = CertificateBuilder::new(profile, SerialNumber::from(seed), validity, spki).unwrap();
    let certificate = builder.build::<_, DerSignature>(&key).unwrap();
    (key.to_pkcs8_pem(LineEnding::LF).unwrap().to_string(), certificate.to_pem(LineEnding::LF).unwrap())
}

fn record() -> Value {
    serde_json::from_str(include_str!(
        "../../../examples/haus-am-hang/Ergebnisse/Energieberatung/Energieausweis/ergebnis.json"
    ))
    .unwrap()
}

const AT: i64 = 1_792_000_000;

#[test]
fn a_signature_shows_who_signed_the_record_as_it_stands() {
    let (key, certificate) = signer(7, "CN=Beispiel Energieberatung,O=Beispiel Energieberatung GmbH,C=DE");
    let jws = sign(&record(), &key, &certificate, AT).unwrap();
    let signer = verify(&record(), &jws).unwrap();
    assert_eq!(signer.subject, "CN=Beispiel Energieberatung,O=Beispiel Energieberatung GmbH,C=DE");
    assert_eq!(signer.issuer, signer.subject);
    assert_eq!((signer.time, signer.in_validity), (AT, true));

    // The file: a compact JWS with nothing between its two full stops, and a header of alg, iat and x5c only.
    let parts: Vec<&str> = jws.split('.').collect();
    assert_eq!((parts.len(), parts[1]), (3, ""));
    let header: Value = serde_json::from_slice(&Base64UrlUnpadded::decode_vec(parts[0]).unwrap()).unwrap();
    let names: Vec<&String> = header.as_object().unwrap().keys().collect();
    assert_eq!(names, ["alg", "iat", "x5c"]);
    assert_eq!((&header["alg"], &header["iat"]), (&json!("ES256"), &json!(AT)));
    assert_eq!(Base64UrlUnpadded::decode_vec(parts[2]).unwrap().len(), 64);
    // A line end after it, as an editor leaves one, is no part of it.
    assert!(verify(&record(), &format!("{jws}\n")).is_ok());

    // The record written another way (key order, whitespace, 192.0 for 192) is the same record.
    let mut rewritten = record();
    let mut fields: Vec<_> = rewritten.as_object().unwrap().clone().into_iter().collect();
    fields.reverse();
    rewritten = Value::Object(fields.into_iter().collect());
    rewritten["figures"][0]["area"] = json!(192.0);
    assert!(verify(&rewritten, &jws).is_ok());
}

#[test]
fn a_changed_record_or_signature_does_not_hold() {
    let (key, certificate) = signer(7, "CN=Beispiel Energieberatung,C=DE");
    let jws = sign(&record(), &key, &certificate, AT).unwrap();

    let mut changed = record();
    changed["figures"][0]["primaryEnergy"] = json!(181);
    assert_eq!(verify(&changed, &jws), Err(Failure::Invalid));

    // Another time of signing than the one signed.
    let parts: Vec<&str> = jws.split('.').collect();
    let mut header: Value = serde_json::from_slice(&Base64UrlUnpadded::decode_vec(parts[0]).unwrap()).unwrap();
    header["iat"] = json!(AT + 1);
    let forged = Base64UrlUnpadded::encode_string(header.to_string().as_bytes());
    assert_eq!(verify(&record(), &format!("{forged}..{}", parts[2])), Err(Failure::Invalid));

    // Another signer's certificate in place of the signer's.
    let (_, other) = signer(9, "CN=Jemand anderes,C=DE");
    let theirs = sign(&record(), &signer(9, "CN=Jemand anderes,C=DE").0, &other, AT).unwrap();
    let their_header = theirs.split('.').next().unwrap();
    assert_eq!(verify(&record(), &format!("{their_header}..{}", parts[2])), Err(Failure::Invalid));

    // One octet of the signature.
    let mut octets = Base64UrlUnpadded::decode_vec(parts[2]).unwrap();
    octets[10] ^= 1;
    let broken = format!("{}..{}", parts[0], Base64UrlUnpadded::encode_string(&octets));
    assert_eq!(verify(&record(), &broken), Err(Failure::Invalid));
}

#[test]
fn a_key_signs_only_with_its_own_certificate() {
    let (key, _) = signer(7, "CN=Beispiel Energieberatung,C=DE");
    let (_, other) = signer(9, "CN=Jemand anderes,C=DE");
    assert!(sign(&record(), &key, &other, AT).unwrap_err().contains("not this key's"));
}

#[test]
fn the_time_of_signing_is_held_against_the_certificates_validity() {
    let (key, certificate) = signer(7, "CN=Beispiel Energieberatung,C=DE");
    for (time, within) in [(FROM as i64 - 1, false), (FROM as i64, true), (TO as i64, true), (TO as i64 + 1, false)] {
        let signer = verify(&record(), &sign(&record(), &key, &certificate, time).unwrap()).unwrap();
        assert_eq!(signer.in_validity, within, "{time}");
    }
}

/// A file with `header` as its protected header and any signature.
fn with_header(header: Value) -> String {
    format!("{}..{}", Base64UrlUnpadded::encode_string(header.to_string().as_bytes()), "A".repeat(86))
}

#[test]
fn what_is_not_of_the_profile_is_refused() {
    let (key, certificate) = signer(7, "CN=Beispiel Energieberatung,C=DE");
    let jws = sign(&record(), &key, &certificate, AT).unwrap();
    let parts: Vec<&str> = jws.split('.').collect();
    let header: Value = serde_json::from_slice(&Base64UrlUnpadded::decode_vec(parts[0]).unwrap()).unwrap();
    let malformed = |jws: &str| matches!(verify(&record(), jws), Err(Failure::Malformed(_)));
    let changed = |name: &str, value: Value| {
        let mut header = header.clone();
        header[name] = value;
        with_header(header)
    };
    let without = |name: &str| {
        let mut header = header.clone();
        header.as_object_mut().unwrap().remove(name);
        with_header(header)
    };

    // No signature at all, a secret shared between signer and reader, or a header that changes what is signed.
    assert!(malformed(&changed("alg", json!("none"))));
    assert!(malformed(&changed("alg", json!("HS256"))));
    assert!(malformed(&changed("crit", json!(["b64"]))));
    assert!(malformed(&changed("b64", json!(false))));
    assert!(malformed(&changed("sigD", json!({}))));
    assert!(malformed(&without("alg")) && malformed(&without("iat")) && malformed(&without("x5c")));
    assert!(malformed(&changed("iat", json!("2026-10-20T15:00:00Z"))));
    assert!(malformed(&changed("x5c", json!([]))) && malformed(&changed("x5c", json!(["AAAA"]))));
    // A payload inside the file, the JSON form of a JWS, or no JWS.
    assert!(malformed(&format!("{}.e30.{}", parts[0], parts[2])));
    assert!(malformed("{\"payload\":\"\",\"signatures\":[]}") && malformed("") && malformed("a.b"));
    assert!(malformed(&format!("{}..{}", parts[0], &parts[2][..40])));

    // Another algorithm by a certificate's key is a signature this program cannot verify, and says so.
    assert_eq!(verify(&record(), &changed("alg", json!("ES384"))), Err(Failure::Unsupported("ES384".into())));
    assert_eq!(verify(&record(), &changed("alg", json!("PS256"))), Err(Failure::Unsupported("PS256".into())));
}

#[test]
fn a_signature_file_is_told_by_its_name() {
    assert_eq!(signature::name_of("ergebnis.energieberater.jws"), Some("energieberater"));
    assert_eq!(signature::name_of("ergebnis.Buero-2_b.jws"), Some("Buero-2_b"));
    for other in ["ergebnis.json", "ergebnis..jws", "ergebnis.a.b.jws", "ergebnis.ä.jws", "ergebnis.a b.jws", "x.a.jws"]
    {
        assert_eq!(signature::name_of(other), None, "{other}");
    }
}
