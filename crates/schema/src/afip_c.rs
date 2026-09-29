use afip::{Concepto, CondicionIva, DocTipo};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Factura C. Unknown keys are rejected: a misspelled or outdated receptor
/// field must not silently fall back to "consumidor final" when authorizing.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AfipCInvoice {
    pub emisor: Emisor,
    pub receptor: Receptor,
    pub comprobante: Comprobante,
    pub items: Vec<LineItem>,
    #[serde(default = "default_cae")]
    pub cae: Cae,
}

fn default_cae() -> Cae {
    Cae {
        numero: String::new(),
        vencimiento: String::new(),
    }
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct Emisor {
    pub razon_social: String,
    pub domicilio_comercial: String,
    pub condicion_iva: String,
    pub cuit: String,
    pub ingresos_brutos: String,
    pub inicio_actividades: String,
}

/// The buyer. `condicion_iva`, `doc_tipo` and `doc_nro` are sent to AFIP as
/// codes and printed as their labels, so the PDF always matches the CAE.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Receptor {
    pub nombre: Option<String>,
    pub domicilio: Option<String>,
    #[serde(default)]
    pub condicion_iva: CondicionIva,
    pub condicion_venta: String,
    #[serde(default)]
    pub doc_tipo: DocTipo,
    /// Document number; 0 (the default) for an unidentified consumidor final.
    #[serde(default)]
    pub doc_nro: u64,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct Comprobante {
    /// What is billed. Defaults to [`AfipCInvoice::concepto`]'s inference.
    #[serde(default)]
    pub concepto: Option<Concepto>,
    pub tipo: String,
    pub codigo: String,
    pub punto_de_venta: String,
    pub numero: String,
    pub fecha_emision: String,
    pub periodo_desde: Option<String>,
    pub periodo_hasta: Option<String>,
    pub fecha_vencimiento: String,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct LineItem {
    pub codigo: String,
    pub descripcion: String,
    pub cantidad: f64,
    pub unidad: String,
    pub precio_unitario: f64,
    pub bonificacion_porcentaje: f64,
    pub bonificacion_importe: f64,
    pub subtotal: f64,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct Cae {
    pub numero: String,
    pub vencimiento: String,
}

impl AfipCInvoice {
    pub fn subtotal(&self) -> f64 {
        self.items.iter().map(|item| item.subtotal).sum()
    }

    pub fn otros_tributos(&self) -> f64 {
        0.0
    }

    pub fn total(&self) -> f64 {
        self.subtotal() + self.otros_tributos()
    }

    /// `comprobante.concepto` if set; otherwise services when a billing
    /// period (`periodo_desde` / `periodo_hasta`) is given, products if not.
    pub fn concepto(&self) -> Concepto {
        let has_period = [
            &self.comprobante.periodo_desde,
            &self.comprobante.periodo_hasta,
        ]
        .into_iter()
        .any(|p| p.as_deref().is_some_and(|s| !s.trim().is_empty()));
        self.comprobante.concepto.unwrap_or(if has_period {
            Concepto::Servicios
        } else {
            Concepto::Productos
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::InvoiceConfig;

    /// Parse a minimal afip_c invoice, splicing extra lines into `[receptor]`
    /// and `[comprobante]`.
    fn parse(receptor: &str, comprobante: &str) -> Result<AfipCInvoice, toml::de::Error> {
        let src = format!(
            r#"
format = "afip_c"
[emisor]
razon_social = "Test"
domicilio_comercial = ""
condicion_iva = "Responsable Monotributo"
cuit = "20123456789"
ingresos_brutos = ""
inicio_actividades = ""
[receptor]
condicion_venta = "Contado"
{receptor}
[comprobante]
tipo = "C"
codigo = "011"
punto_de_venta = "00001"
numero = ""
fecha_emision = ""
fecha_vencimiento = ""
{comprobante}
[[items]]
codigo = "1"
descripcion = "x"
cantidad = 1.0
unidad = "u"
precio_unitario = 1.0
bonificacion_porcentaje = 0.0
bonificacion_importe = 0.0
subtotal = 1.0
"#
        );
        match toml::from_str::<InvoiceConfig>(&src)? {
            InvoiceConfig::AfipC(inv) => Ok(inv),
            _ => unreachable!(),
        }
    }

    #[test]
    fn receptor_defaults_to_anonymous_consumidor_final() {
        let r = parse("", "").unwrap().receptor;
        assert_eq!(r.condicion_iva, CondicionIva::ConsumidorFinal);
        assert_eq!(r.doc_tipo, DocTipo::ConsumidorFinal);
        assert_eq!(r.doc_nro, 0);
    }

    #[test]
    fn receptor_takes_afip_codes() {
        let r = parse(
            "condicion_iva = \"responsable_inscripto\"\ndoc_tipo = \"cuit\"\ndoc_nro = 30712345678",
            "",
        )
        .unwrap()
        .receptor;
        assert_eq!(r.condicion_iva, CondicionIva::ResponsableInscripto);
        assert_eq!(r.doc_tipo, DocTipo::Cuit);
        assert_eq!(r.doc_nro, 30712345678);
    }

    #[test]
    fn concepto_is_inferred_from_billing_period() {
        let concepto = |comprobante| parse("", comprobante).unwrap().concepto();
        assert_eq!(concepto(""), Concepto::Productos);
        assert_eq!(concepto("periodo_desde = \"  \""), Concepto::Productos);
        assert_eq!(
            concepto("periodo_hasta = \"31/01/2025\""),
            Concepto::Servicios
        );
        assert_eq!(
            concepto("periodo_desde = \"01/01/2025\"\nconcepto = \"productos_y_servicios\""),
            Concepto::ProductosYServicios
        );
        assert_eq!(
            concepto("periodo_desde = \"01/01/2025\"\nconcepto = \"productos\""),
            Concepto::Productos
        );
    }

    #[test]
    fn unknown_fields_are_rejected() {
        // Pre-typed receptor and the old [afip] table must not be ignored.
        assert!(parse("documento = \"30712345678\"", "").is_err());
        assert!(parse("", "[afip]\ndoc_tipo = \"cuit\"").is_err());
        assert!(parse("doc_numero = 1", "").is_err());
    }
}
