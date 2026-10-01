//! Native Rust SDK for Argentina's ARCA/AFIP web services.
//!
//! Covers the pieces needed to issue an electronic **Factura C**:
//! - [`cert`] — generate the private key + CSR to enroll at the ARCA portal.
//! - [`wsaa`] — authenticate (CMS-signed login ticket) and cache credentials.
//! - [`wsfe`] — query the last voucher and request a CAE.
//!
//! The [`Client`] type wires these together against a [`ClientConfig`].
//!
//! # Features
//!
//! - `client` (default): everything that talks to ARCA — [`Client`], [`cert`],
//!   [`wsaa`], [`wsfe`] — and the HTTP/OpenSSL stack behind it. Without it the
//!   crate is just the data types ([`types`], [`config`]), cheap to depend on.
//! - `schemars`: derive `JsonSchema` for the request enums.
//!
//! [`qr`] builds the QR code ARCA requires on printed invoices; it needs no
//! network, so it's available either way.

mod error;

pub mod config;
pub mod qr;
pub mod types;

#[cfg(feature = "client")]
mod client;
#[cfg(feature = "client")]
mod xml;

#[cfg(feature = "client")]
pub mod cert;
#[cfg(feature = "client")]
pub mod wsaa;
#[cfg(feature = "client")]
pub mod wsfe;

#[cfg(feature = "client")]
pub use client::Client;
pub use config::{ClientConfig, Environment};
pub use error::{Error, Result};
pub use types::{CaeResult, Concepto, CondicionIva, DocTipo, FacturaC, VoucherInfo, VoucherType};
#[cfg(feature = "client")]
pub use wsaa::Credentials;
