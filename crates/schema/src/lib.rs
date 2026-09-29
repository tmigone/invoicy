//! The invoice data model shared across invoicy.
//!
//! One serde + `JsonSchema` struct tree per invoice format, as written in the
//! invoice TOML, plus [`introspect`] to list a format's fields and their types.
//! It has no rendering, network or file I/O, so anything that needs to read or
//! validate invoices can depend on it cheaply.

pub mod afip_a;
pub mod afip_c;
pub mod generic;
pub mod introspect;

use serde::Deserialize;

pub use afip_a::AfipAInvoice;
pub use afip_c::AfipCInvoice;
pub use generic::GenericInvoice;

/// Every supported format: `(name, description)`. `name` is the value of the
/// invoice's `format` key.
pub const FORMATS: &[(&str, &str)] = &[
    ("generic", "Simple international invoice"),
    ("afip_c", "Argentina AFIP Factura C (Monotributo)"),
    ("afip_a", "Argentina AFIP Factura A (Responsable Inscripto)"),
];

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
            [emisor]
            razon_social = "Test"
            domicilio_comercial = "Address"
            condicion_iva = "Monotributo"
            cuit = "20123456789"
            ingresos_brutos = "12345"
            inicio_actividades = "01/01/2020"
            [receptor]
            condicion_iva = "consumidor_final"
            condicion_venta = "Contado"
            [comprobante]
            tipo = "C"
            codigo = "011"
            punto_de_venta = "00001"
            numero = "00000001"
            fecha_emision = "01/01/2025"
            fecha_vencimiento = "15/01/2025"
            [[items]]
            codigo = "1"
            descripcion = "Service"
            cantidad = 1.0
            unidad = "u"
            precio_unitario = 100.0
            bonificacion_porcentaje = 0.0
            bonificacion_importe = 0.0
            subtotal = 100.0
            [cae]
            numero = "12345678901234"
            vencimiento = "25/01/2025"
        "#;
        let config: InvoiceConfig = toml::from_str(toml).unwrap();
        assert!(matches!(config, InvoiceConfig::AfipC(_)));
        assert_eq!(config.invoice_number(), "00001-00000001");
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
        assert_eq!(
            introspect::field_type("afip_c", "items[3].subtotal").as_deref(),
            Some("number")
        );
    }
}
