//! Native Rust SDK for Argentina's ARCA/AFIP web services.
//!
//! Covers the pieces needed to issue an electronic **Factura C**:
//! - [`cert`] — generate the private key + CSR to enroll at the ARCA portal.
//! - [`wsaa`] — authenticate (CMS-signed login ticket) and cache credentials.
//! - [`wsfe`] — query the last voucher and request a CAE.
//!
//! The [`Client`] type wires these together against an [`EmisorConfig`].

mod error;
mod xml;

pub mod cert;
pub mod config;
pub mod types;
pub mod wsaa;
pub mod wsfe;

pub use config::{CondicionIva, EmisorConfig, Environment};
pub use error::{Error, Result};
pub use types::{CaeResult, Concepto, DocTipo, FacturaC, VoucherInfo, VoucherType};
pub use wsaa::Credentials;

use std::path::{Path, PathBuf};

/// WSFE service identifier used for WSAA scoping.
const SERVICE_WSFE: &str = "wsfe";

/// High-level client bound to a single issuer configuration.
pub struct Client {
    config: EmisorConfig,
    http: reqwest::blocking::Client,
    cache_dir: PathBuf,
}

impl Client {
    /// Build a client. `cache_dir` is where the WSAA credential cache lives.
    pub fn new(config: EmisorConfig, cache_dir: impl Into<PathBuf>) -> Result<Self> {
        let http = reqwest::blocking::Client::builder()
            .user_agent(concat!("afip-rs/", env!("CARGO_PKG_VERSION")))
            .build()?;
        Ok(Self {
            config,
            http,
            cache_dir: cache_dir.into(),
        })
    }

    pub fn config(&self) -> &EmisorConfig {
        &self.config
    }

    fn cache_path(&self, service: &str) -> PathBuf {
        let env = match self.config.environment {
            Environment::Homologacion => "homo",
            Environment::Produccion => "prod",
        };
        self.cache_dir.join(format!("wsaa-{env}-{service}.json"))
    }

    /// Obtain WSFE credentials, reusing the on-disk cache when still valid.
    pub fn authenticate(&self) -> Result<Credentials> {
        let cache = self.cache_path(SERVICE_WSFE);
        if let Some(creds) = wsaa::load_cached(&cache, SERVICE_WSFE) {
            return Ok(creds);
        }

        let cert = read_pem(&self.config.cert_path, "certificate")?;
        let key = read_pem(&self.config.key_path, "private key")?;

        let creds = wsaa::login(
            &self.http,
            self.config.environment.wsaa_url(),
            SERVICE_WSFE,
            &cert,
            &key,
        )?;
        wsaa::store_cache(&cache, &creds)?;
        Ok(creds)
    }

    /// Last authorized voucher number for `tipo` at the configured punto de venta.
    pub fn last_voucher(&self, tipo: VoucherType) -> Result<u64> {
        let creds = self.authenticate()?;
        wsfe::last_voucher(
            &self.http,
            self.config.environment.wsfe_url(),
            &creds,
            self.config.cuit,
            self.config.punto_venta,
            tipo,
        )
    }

    /// Issue a Factura C, automatically assigning the next voucher number.
    pub fn create_factura_c(&self, factura: &FacturaC) -> Result<CaeResult> {
        let creds = self.authenticate()?;
        let url = self.config.environment.wsfe_url();
        let last = wsfe::last_voucher(
            &self.http,
            url,
            &creds,
            self.config.cuit,
            self.config.punto_venta,
            VoucherType::FacturaC,
        )?;
        wsfe::create_factura_c(
            &self.http,
            url,
            &creds,
            self.config.cuit,
            self.config.punto_venta,
            last + 1,
            factura,
        )
    }

    /// Fetch a single already-issued voucher by number.
    pub fn get_voucher(&self, tipo: VoucherType, numero: u64) -> Result<VoucherInfo> {
        let creds = self.authenticate()?;
        wsfe::get_voucher(
            &self.http,
            self.config.environment.wsfe_url(),
            &creds,
            self.config.cuit,
            self.config.punto_venta,
            tipo,
            numero,
        )
    }

    /// Fetch vouchers `from..=to` (inclusive). One WSFE call per number,
    /// authenticating only once.
    pub fn list_vouchers(
        &self,
        tipo: VoucherType,
        from: u64,
        to: u64,
    ) -> Result<Vec<VoucherInfo>> {
        let creds = self.authenticate()?;
        let url = self.config.environment.wsfe_url();
        let mut out = Vec::new();
        for numero in from..=to {
            out.push(wsfe::get_voucher(
                &self.http,
                url,
                &creds,
                self.config.cuit,
                self.config.punto_venta,
                tipo,
                numero,
            )?);
        }
        Ok(out)
    }

    /// WSFE service health (`FEDummy`).
    pub fn status(&self) -> Result<(String, String, String)> {
        wsfe::dummy(&self.http, self.config.environment.wsfe_url())
    }
}

fn read_pem(path: &Path, what: &str) -> Result<Vec<u8>> {
    std::fs::read(path).map_err(|e| {
        Error::Config(format!("cannot read {what} at {}: {e}", path.display()))
    })
}
