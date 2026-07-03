use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

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

/// Condición frente al IVA of the invoice issuer. Monotributistas issue
/// Factura C and carry no discriminated IVA.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CondicionIva {
    Monotributo,
    ResponsableInscripto,
    Exento,
}

/// Issuer ("emisor") configuration, persisted as `emisor_config.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmisorConfig {
    /// Issuer CUIT (11 digits, no dashes).
    pub cuit: u64,
    /// Punto de venta registered in "Web Services" mode.
    pub punto_venta: u32,
    /// Legal name.
    pub razon_social: String,
    /// Tax condition of the issuer.
    pub condicion_iva: CondicionIva,
    /// Target environment.
    #[serde(default)]
    pub environment: Environment,
    /// Path to the PEM certificate issued by ARCA.
    pub cert_path: PathBuf,
    /// Path to the PEM private key that produced the CSR.
    pub key_path: PathBuf,
}

impl EmisorConfig {
    /// Load configuration from a JSON file.
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let raw = std::fs::read_to_string(path)
            .map_err(|e| Error::Config(format!("cannot read {}: {e}", path.display())))?;
        let cfg: EmisorConfig = serde_json::from_str(&raw)?;
        Ok(cfg)
    }

    /// Persist configuration to a JSON file (pretty-printed).
    pub fn save(&self, path: impl AsRef<Path>) -> Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }
}
