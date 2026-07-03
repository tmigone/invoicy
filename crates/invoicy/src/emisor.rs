//! The emisor profile — a single source of truth for the issuer.
//!
//! Stored as `emisor.toml` under the working directory (`--home`). It holds
//! both the AFIP authorization settings (used to build an [`afip::Client`]) and
//! the presentation fields shown on the PDF. Invoice TOMLs no longer repeat
//! this; `generate` merges it in.

use std::path::{Path, PathBuf};

use afip::{Client, ClientConfig, Environment};
use serde::{Deserialize, Serialize};

type BoxError = Box<dyn std::error::Error>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmisorProfile {
    /// Issuer CUIT (11 digits, no dashes).
    pub cuit: u64,
    /// Legal name.
    pub razon_social: String,
    #[serde(default)]
    pub domicilio_comercial: String,
    #[serde(default = "default_condicion_iva")]
    pub condicion_iva: String,
    #[serde(default)]
    pub ingresos_brutos: String,
    #[serde(default)]
    pub inicio_actividades: String,
    /// Punto de venta registered in "Web Services" mode.
    pub punto_venta: u32,
    #[serde(default)]
    pub environment: Environment,
    /// Path to the ARCA-issued PEM certificate.
    pub cert_path: PathBuf,
    /// Path to the PEM private key that produced the CSR.
    pub key_path: PathBuf,
}

fn default_condicion_iva() -> String {
    "Responsable Monotributo".to_string()
}

impl EmisorProfile {
    pub fn path(home: &Path) -> PathBuf {
        home.join("emisor.toml")
    }

    pub fn load(home: &Path) -> Result<Self, BoxError> {
        let path = Self::path(home);
        let raw = std::fs::read_to_string(&path).map_err(|e| {
            format!(
                "no se pudo leer {} — ejecutá `invoicy afip configure` primero ({e})",
                path.display()
            )
        })?;
        Ok(toml::from_str(&raw)?)
    }

    pub fn save(&self, home: &Path) -> Result<(), BoxError> {
        std::fs::create_dir_all(home)?;
        std::fs::write(Self::path(home), toml::to_string_pretty(self)?)?;
        Ok(())
    }

    fn client_config(&self) -> ClientConfig {
        ClientConfig {
            cuit: self.cuit,
            punto_venta: self.punto_venta,
            environment: self.environment,
            cert_path: self.cert_path.clone(),
            key_path: self.key_path.clone(),
        }
    }

    pub fn client(&self, home: &Path) -> Result<Client, BoxError> {
        Ok(Client::new(self.client_config(), home.join("cache"))?)
    }

    /// Build the `[emisor]` table for rendering (matches the afip_c `Emisor`).
    pub fn emisor_table(&self) -> toml::value::Table {
        let mut t = toml::value::Table::new();
        t.insert("razon_social".into(), self.razon_social.clone().into());
        t.insert(
            "domicilio_comercial".into(),
            self.domicilio_comercial.clone().into(),
        );
        t.insert("condicion_iva".into(), self.condicion_iva.clone().into());
        t.insert("cuit".into(), self.cuit.to_string().into());
        t.insert(
            "ingresos_brutos".into(),
            self.ingresos_brutos.clone().into(),
        );
        t.insert(
            "inicio_actividades".into(),
            self.inicio_actividades.clone().into(),
        );
        t
    }
}
