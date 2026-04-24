mod afip_a;
mod afip_c;
mod generic;

use serde::Deserialize;

pub use afip_a::AfipAInvoice;
pub use afip_c::AfipCInvoice;
pub use generic::GenericInvoice;

#[derive(Debug, Deserialize)]
#[serde(tag = "format")]
pub enum InvoiceConfig {
    #[serde(rename = "generic")]
    Generic(GenericInvoice),
    #[serde(rename = "afip_c")]
    AfipC(AfipCInvoice),
    #[serde(rename = "afip_a")]
    AfipA(AfipAInvoice),
}

impl InvoiceConfig {
    pub fn to_typst_dict(&self) -> String {
        match self {
            InvoiceConfig::Generic(inv) => inv.to_typst_dict(),
            InvoiceConfig::AfipC(inv) => inv.to_typst_dict(),
            InvoiceConfig::AfipA(inv) => inv.to_typst_dict(),
        }
    }

    pub fn invoice_number(&self) -> String {
        match self {
            InvoiceConfig::Generic(inv) => inv.invoice.number.clone(),
            InvoiceConfig::AfipC(inv) => format!(
                "{}-{}",
                inv.comprobante.punto_de_venta, inv.comprobante.numero
            ),
            InvoiceConfig::AfipA(inv) => format!(
                "{}-{}",
                inv.comprobante.punto_de_venta, inv.comprobante.numero
            ),
        }
    }

    pub fn default_template(&self) -> &'static str {
        match self {
            InvoiceConfig::Generic(_) => include_str!("../../templates/generic.typ"),
            InvoiceConfig::AfipC(_) => include_str!("../../templates/afip_c.typ"),
            InvoiceConfig::AfipA(_) => include_str!("../../templates/afip_a.typ"),
        }
    }
}

pub fn escape_typst_string(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

pub fn typst_option(opt: &Option<String>) -> String {
    opt.as_ref()
        .map(|s| format!("\"{}\"", escape_typst_string(s)))
        .unwrap_or_else(|| "none".to_string())
}
