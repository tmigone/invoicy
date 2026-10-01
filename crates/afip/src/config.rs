use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// ARCA/AFIP environment. Certificates are issued per-environment and are not
/// interchangeable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Environment {
    /// Testing environment (homologación).
    #[default]
    Homologacion,
    /// Real environment; vouchers issued here are fiscally valid.
    Produccion,
}

impl Environment {
    /// WSAA `loginCms` SOAP endpoint.
    pub fn wsaa_url(self) -> &'static str {
        match self {
            Environment::Homologacion => "https://wsaahomo.afip.gov.ar/ws/services/LoginCms",
            Environment::Produccion => "https://wsaa.afip.gov.ar/ws/services/LoginCms",
        }
    }

    /// WSFEv1 SOAP endpoint.
    pub fn wsfe_url(self) -> &'static str {
        match self {
            Environment::Homologacion => "https://wswhomo.afip.gov.ar/wsfev1/service.asmx",
            Environment::Produccion => "https://servicios1.afip.gov.ar/wsfev1/service.asmx",
        }
    }
}

/// Everything the [`crate::Client`] needs to authenticate with AFIP and issue
/// vouchers. Presentation-only issuer data (address, ingresos brutos, …) is not
/// part of the SDK — the caller keeps that alongside its own config.
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// Issuer CUIT (11 digits, no dashes).
    pub cuit: u64,
    /// Punto de venta registered in "Web Services" mode.
    pub punto_venta: u32,
    /// Target environment.
    pub environment: Environment,
    /// Path to the PEM certificate issued by ARCA.
    pub cert_path: PathBuf,
    /// Path to the PEM private key that produced the CSR.
    pub key_path: PathBuf,
}
