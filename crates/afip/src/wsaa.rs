//! WSAA — Web Service de Autenticación y Autorización.
//!
//! Flow: build a Ticket de Requerimiento de Acceso (TRA), sign it as a PKCS#7 /
//! CMS message with the ARCA-issued certificate, send it to `loginCms`, and
//! receive a `token` + `sign` pair valid for ~12h. The pair is cached on disk
//! and reused until it is close to expiring.

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use chrono::{DateTime, Duration, FixedOffset, Utc};
use openssl::pkcs7::{Pkcs7, Pkcs7Flags};
use openssl::pkey::PKey;
use openssl::stack::Stack;
use openssl::x509::X509;
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::xml;

/// Access credentials returned by WSAA and cached on disk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Credentials {
    pub token: String,
    pub sign: String,
    /// RFC 3339 expiration timestamp as returned by ARCA.
    pub expiration: String,
    /// Service the credentials are scoped to (e.g. `wsfe`).
    pub service: String,
}

impl Credentials {
    /// Whether the credentials are still usable with a safety margin.
    fn valid_for(&self, service: &str) -> bool {
        if self.service != service {
            return false;
        }
        match DateTime::parse_from_rfc3339(&self.expiration) {
            // Renew if within 5 minutes of expiry.
            Ok(exp) => exp - Duration::minutes(5) > Utc::now(),
            Err(_) => false,
        }
    }
}

/// Argentina is UTC−03:00 year-round (no DST).
fn ar_now() -> DateTime<FixedOffset> {
    let offset = FixedOffset::west_opt(3 * 3600).expect("valid offset");
    Utc::now().with_timezone(&offset)
}

/// Build the TRA XML for `service`.
fn build_tra(service: &str) -> String {
    let now = ar_now();
    let gen_time = now - Duration::minutes(10);
    let exp = now + Duration::minutes(10);
    // uniqueId must be unique per request; seconds-since-epoch is sufficient.
    let unique_id = now.timestamp();
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<loginTicketRequest version="1.0">
  <header>
    <uniqueId>{unique_id}</uniqueId>
    <generationTime>{gen}</generationTime>
    <expirationTime>{exp}</expirationTime>
  </header>
  <service>{service}</service>
</loginTicketRequest>"#,
        // WSAA rejects fractional seconds, so format without them.
        gen = gen_time.format("%Y-%m-%dT%H:%M:%S%:z"),
        exp = exp.format("%Y-%m-%dT%H:%M:%S%:z"),
    )
}

/// Sign the TRA as a PKCS#7 / CMS message and return it base64-encoded.
fn sign_tra(cert_pem: &[u8], key_pem: &[u8], tra: &str) -> Result<String> {
    let cert = X509::from_pem(cert_pem)?;
    let pkey = PKey::private_key_from_pem(key_pem)?;
    let certs = Stack::new()?;
    // BINARY prevents CRLF canonicalization of the payload.
    let pkcs7 = Pkcs7::sign(&cert, &pkey, &certs, tra.as_bytes(), Pkcs7Flags::BINARY)?;
    let der = pkcs7.to_der()?;
    Ok(BASE64.encode(der))
}

/// SOAP envelope for `loginCms`.
fn login_cms_envelope(cms_base64: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<soapenv:Envelope xmlns:soapenv="http://schemas.xmlsoap.org/soap/envelope/" xmlns:wsaa="http://wsaa.view.sua.dvadac.desein.afip.gov">
  <soapenv:Header/>
  <soapenv:Body>
    <wsaa:loginCms>
      <wsaa:in0>{cms_base64}</wsaa:in0>
    </wsaa:loginCms>
  </soapenv:Body>
</soapenv:Envelope>"#
    )
}

/// Perform the full WSAA login: build → sign → `loginCms` → parse.
pub fn login(
    http: &reqwest::blocking::Client,
    url: &str,
    service: &str,
    cert_pem: &[u8],
    key_pem: &[u8],
) -> Result<Credentials> {
    let tra = build_tra(service);
    let cms = sign_tra(cert_pem, key_pem, &tra)?;
    let envelope = login_cms_envelope(&cms);

    let body = http
        .post(url)
        .header("Content-Type", "text/xml; charset=utf-8")
        .header("SOAPAction", "")
        .body(envelope)
        .send()?
        .text()?;

    xml::check_soap_fault(&body, "WSAA")?;

    // `loginCmsReturn` holds the (entity-escaped) loginTicketResponse XML.
    let inner = xml::first_text(&body, "loginCmsReturn")
        .ok_or(Error::MissingField("loginCmsReturn"))?;

    let token = xml::first_text(&inner, "token").ok_or(Error::MissingField("token"))?;
    let sign = xml::first_text(&inner, "sign").ok_or(Error::MissingField("sign"))?;
    let expiration =
        xml::first_text(&inner, "expirationTime").ok_or(Error::MissingField("expirationTime"))?;

    Ok(Credentials {
        token,
        sign,
        expiration,
        service: service.to_string(),
    })
}

/// Return cached credentials for `service` if still valid, else `None`.
pub fn load_cached(cache_path: &std::path::Path, service: &str) -> Option<Credentials> {
    let raw = std::fs::read_to_string(cache_path).ok()?;
    let creds: Credentials = serde_json::from_str(&raw).ok()?;
    creds.valid_for(service).then_some(creds)
}

/// Persist credentials to the cache file.
pub fn store_cache(cache_path: &std::path::Path, creds: &Credentials) -> Result<()> {
    if let Some(parent) = cache_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(cache_path, serde_json::to_string_pretty(creds)?)?;
    Ok(())
}
