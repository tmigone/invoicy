//! Local key + CSR generation.
//!
//! ARCA does not issue certificates programmatically for the native web
//! services: you generate a private key and a Certificate Signing Request
//! locally, upload the CSR to the ARCA portal ("Administración de Certificados
//! Digitales"), and download the signed X.509 certificate. This module
//! produces the key + CSR pair.

use openssl::hash::MessageDigest;
use openssl::pkey::{PKey, Private};
use openssl::rsa::Rsa;
use openssl::x509::{X509NameBuilder, X509ReqBuilder};

use crate::error::Result;

/// A freshly generated private key and matching CSR, both PEM-encoded.
pub struct KeyAndCsr {
    /// PKCS#8 PEM private key. Keep this secret — it is not sent to ARCA.
    pub private_key_pem: String,
    /// PEM `CERTIFICATE REQUEST` to upload to the ARCA portal.
    pub csr_pem: String,
}

/// Generate a 2048-bit RSA key and a CSR whose subject encodes the issuer's
/// CUIT and legal name, as ARCA expects:
///
/// `/C=AR/O=<razon_social>/CN=<alias>/serialNumber=CUIT <cuit>`
pub fn generate_key_and_csr(cuit: u64, razon_social: &str, alias: &str) -> Result<KeyAndCsr> {
    // 1. RSA private key.
    let rsa = Rsa::generate(2048)?;
    let pkey: PKey<Private> = PKey::from_rsa(rsa)?;

    // 2. Subject name.
    let mut name = X509NameBuilder::new()?;
    name.append_entry_by_text("C", "AR")?;
    name.append_entry_by_text("O", razon_social)?;
    name.append_entry_by_text("CN", alias)?;
    name.append_entry_by_text("serialNumber", &format!("CUIT {cuit}"))?;
    let name = name.build();

    // 3. CSR signed with the private key over SHA-256.
    let mut req = X509ReqBuilder::new()?;
    req.set_subject_name(&name)?;
    req.set_pubkey(&pkey)?;
    req.sign(&pkey, MessageDigest::sha256())?;
    let req = req.build();

    Ok(KeyAndCsr {
        private_key_pem: String::from_utf8(pkey.private_key_to_pem_pkcs8()?)
            .expect("PEM is valid UTF-8"),
        csr_pem: String::from_utf8(req.to_pem()?).expect("PEM is valid UTF-8"),
    })
}
