use thiserror::Error;

/// Errors returned by the `afip` crate.
#[derive(Debug, Error)]
pub enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("OpenSSL error: {0}")]
    OpenSsl(#[from] openssl::error::ErrorStack),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("XML parse error: {0}")]
    Xml(#[from] roxmltree::Error),

    /// A SOAP `<Fault>` was returned by the web service.
    #[error("SOAP fault from {service}: {message}")]
    SoapFault { service: String, message: String },

    /// WSAA rejected the login (bad cert, clock skew, not authorized, ...).
    #[error("WSAA login failed: {0}")]
    Wsaa(String),

    /// WSFE returned an error record (`<Errors><Err>`) on a query call.
    #[error("WSFE error: {0}")]
    Wsfe(String),

    /// WSFE returned `Resultado = R` (rejected) with observations/errors.
    #[error("WSFE rejected the voucher: {0}")]
    WsfeRejected(String),

    /// A field expected in a web-service response was missing.
    #[error("unexpected response: missing `{0}`")]
    MissingField(&'static str),

    #[error("configuration error: {0}")]
    Config(String),
}

pub type Result<T> = std::result::Result<T, Error>;
