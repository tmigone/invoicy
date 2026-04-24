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
            condicion_iva = "Consumidor Final"
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
}
