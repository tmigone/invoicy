//! The invoice data model shared across invoicy.
//!
//! One serde + `JsonSchema` struct tree per invoice format, as written in the
//! invoice TOML, plus [`introspect`] to list a format's fields and their types.
//! It has no rendering, network or file I/O, so anything that needs to read or
//! validate invoices can depend on it cheaply.

pub mod afip_c;
pub mod generic;
pub mod introspect;

use serde::{Deserialize, Serialize};

pub use afip_c::AfipCInvoice;
pub use generic::GenericInvoice;

/// Every supported format: `(name, description)`. `name` is the value of the
/// invoice's `format` key.
pub const FORMATS: &[(&str, &str)] = &[
    ("generic", "Simple international invoice"),
    ("afip_c", "Argentina AFIP Factura C (Monotributo)"),
];

/// An invoice of any format. Serializes back to the same TOML shape, with
/// the `format` tag first.
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "format")]
pub enum InvoiceConfig {
    #[serde(rename = "generic")]
    Generic(GenericInvoice),
    #[serde(rename = "afip_c")]
    AfipC(AfipCInvoice),
}

impl InvoiceConfig {
    pub fn invoice_number(&self) -> String {
        match self {
            InvoiceConfig::Generic(inv) => inv.invoice.number.clone(),
            InvoiceConfig::AfipC(inv) => format!(
                "{}-{}",
                inv.comprobante.punto_de_venta, inv.comprobante.numero
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invoice_config_parses_generic() {
        let toml = r#"
            format = "generic"
            [company]
            name = "Test"
            address = "123 St"
            address2 = ""
            city_state_zip = "City, ST 12345"
            country = "USA"
            [client]
            name = "Client"
            address = "456 Ave"
            address2 = ""
            city_state_zip = "Town, ST 67890"
            country = "USA"
            [invoice]
            number = "INV-001"
            date = "2025-01-01"
            due_date = "2025-01-15"
            currency = "USD"
            [[items]]
            description = "Service"
            rate = 100.0
        "#;
        let config: InvoiceConfig = toml::from_str(toml).unwrap();
        assert!(matches!(config, InvoiceConfig::Generic(_)));
        assert_eq!(config.invoice_number(), "INV-001");
    }

    #[test]
    fn invoice_config_parses_afip_c() {
        let toml = r#"
            format = "afip_c"
            [receptor]
            condicion_iva = "consumidor_final"
            condicion_venta = "Contado"
            [comprobante]
            concepto = "servicios"
            fecha_vencimiento = "15/01/2025"
            [[items]]
            codigo = "1"
            descripcion = "Service"
            cantidad = 1.0
            unidad = "u"
            precio_unitario = 100.0
        "#;
        let InvoiceConfig::AfipC(mut inv) = toml::from_str(toml).unwrap() else {
            panic!("expected afip_c");
        };
        inv.comprobante.punto_de_venta = "00001".into();
        inv.comprobante.numero = "00000001".into();
        assert_eq!(InvoiceConfig::AfipC(inv).invoice_number(), "00001-00000001");
    }

    #[test]
    fn every_format_is_introspectable() {
        for (name, _) in FORMATS {
            let fields = introspect::format_fields(name).unwrap();
            assert!(!fields.is_empty(), "{name} has no fields");
        }
        assert_eq!(
            introspect::field_type("afip_c", "receptor.doc_tipo").as_deref(),
            Some("string")
        );
        let fields = introspect::format_fields("afip_c").unwrap();
        let field = |path: &str| fields.iter().find(|f| f.path == path).unwrap();
        assert_eq!(
            field("comprobante.concepto").values,
            ["productos", "servicios", "productos_y_servicios"]
        );
        assert!(!field("comprobante.concepto").optional);
        assert_eq!(field("comprobante.concepto").default, None);
        assert_eq!(
            field("receptor.doc_tipo").values,
            ["cuit", "cuil", "dni", "consumidor_final"]
        );
        assert_eq!(
            field("receptor.doc_tipo").default.as_deref(),
            Some("consumidor_final")
        );
        assert_eq!(field("receptor.condicion_iva").values.len(), 11);
        assert_eq!(field("receptor.doc_nro").default.as_deref(), Some("0"));
        assert!(field("receptor.doc_nro").values.is_empty());
        assert_eq!(field("receptor.doc_nro").typ, "integer");

        // Every automatic field is listed, tagged with where it comes from.
        let source = |path: &str| field(path).source.as_deref();
        assert_eq!(source("emisor.cuit"), Some("AFIP"));
        assert_eq!(source("comprobante.punto_de_venta"), Some("AFIP"));
        assert_eq!(source("comprobante.numero"), Some("AFIP"));
        assert_eq!(source("comprobante.fecha_emision"), None);
        assert_eq!(
            field("comprobante.fecha_emision").default.as_deref(),
            Some("today")
        );
        assert_eq!(
            field("comprobante.fecha_vencimiento").default.as_deref(),
            Some("fecha_emision + 15 days")
        );
        assert_eq!(source("cae.numero"), Some("AFIP"));
        assert_eq!(source("comprobante.tipo"), Some("computed"));
        assert_eq!(source("items[].subtotal"), Some("computed"));
        assert_eq!(source("totales.total"), Some("computed"));
        assert_eq!(source("comprobante.concepto"), None);
        assert_eq!(source("items[].precio_unitario"), None);
        assert!(!field("cae.numero").optional);
        assert_eq!(field("items[].cantidad").typ, "number");

        assert_eq!(
            introspect::field_type("afip_c", "items[3].precio_unitario").as_deref(),
            Some("number")
        );
    }
}
