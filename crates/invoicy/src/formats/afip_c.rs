use schemars::JsonSchema;
use serde::Deserialize;

use crate::typst::{escape_string, option_string};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AfipCInvoice {
    pub emisor: Emisor,
    pub receptor: Receptor,
    pub comprobante: Comprobante,
    pub items: Vec<LineItem>,
    #[serde(default = "default_cae")]
    pub cae: Cae,
    /// AFIP web-service codes used by `facturar` (ignored by `generate`).
    #[serde(default)]
    pub afip: Option<AfipParams>,
}

fn default_cae() -> Cae {
    Cae {
        numero: String::new(),
        vencimiento: String::new(),
    }
}

/// WSFE-specific codes for `facturar`. Defaults to a consumidor-final sale of
/// products.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct AfipParams {
    #[serde(default)]
    pub concepto: ConceptoParam,
    #[serde(default)]
    pub doc_tipo: DocTipoParam,
    #[serde(default)]
    pub doc_nro: u64,
    #[serde(default = "default_cond_iva")]
    pub cond_iva_receptor: u8,
}

fn default_cond_iva() -> u8 {
    5
}

impl Default for AfipParams {
    fn default() -> Self {
        Self {
            concepto: ConceptoParam::default(),
            doc_tipo: DocTipoParam::default(),
            doc_nro: 0,
            cond_iva_receptor: 5,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ConceptoParam {
    #[default]
    Productos,
    Servicios,
    ProductosYServicios,
}

#[derive(Debug, Clone, Copy, Default, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DocTipoParam {
    #[default]
    ConsumidorFinal,
    Cuit,
    Cuil,
    Dni,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct Emisor {
    pub razon_social: String,
    pub domicilio_comercial: String,
    pub condicion_iva: String,
    pub cuit: String,
    pub ingresos_brutos: String,
    pub inicio_actividades: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct Receptor {
    pub nombre: Option<String>,
    pub domicilio: Option<String>,
    pub condicion_iva: String,
    pub condicion_venta: String,
    pub documento: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct Comprobante {
    pub tipo: String,
    pub codigo: String,
    pub punto_de_venta: String,
    pub numero: String,
    pub fecha_emision: String,
    pub periodo_desde: Option<String>,
    pub periodo_hasta: Option<String>,
    pub fecha_vencimiento: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
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

#[derive(Debug, Deserialize, JsonSchema)]
pub struct Cae {
    pub numero: String,
    pub vencimiento: String,
}

impl AfipCInvoice {
    fn subtotal(&self) -> f64 {
        self.items.iter().map(|item| item.subtotal).sum()
    }

    fn otros_tributos(&self) -> f64 {
        0.0
    }

    pub fn total(&self) -> f64 {
        self.subtotal() + self.otros_tributos()
    }

    pub fn to_typst_dict(&self) -> String {
        let items_str: Vec<String> = self
            .items
            .iter()
            .map(|item| {
                format!(
                    r#"(codigo: "{}", descripcion: "{}", cantidad: {}, unidad: "{}", precio_unitario: {}, bonif_pct: {}, bonif_imp: {}, subtotal: {})"#,
                    escape_string(&item.codigo),
                    escape_string(&item.descripcion),
                    item.cantidad,
                    escape_string(&item.unidad),
                    item.precio_unitario,
                    item.bonificacion_porcentaje,
                    item.bonificacion_importe,
                    item.subtotal
                )
            })
            .collect();

        format!(
            r#"#let invoice-data = (
  emisor: (
    razon_social: "{}",
    domicilio_comercial: "{}",
    condicion_iva: "{}",
    cuit: "{}",
    ingresos_brutos: "{}",
    inicio_actividades: "{}",
  ),
  receptor: (
    nombre: {},
    domicilio: {},
    condicion_iva: "{}",
    condicion_venta: "{}",
    documento: {},
  ),
  comprobante: (
    tipo: "{}",
    codigo: "{}",
    punto_de_venta: "{}",
    numero: "{}",
    fecha_emision: "{}",
    periodo_desde: {},
    periodo_hasta: {},
    fecha_vencimiento: "{}",
  ),
  items: ({},),
  cae: (
    numero: "{}",
    vencimiento: "{}",
  ),
  subtotal: {},
  otros_tributos: {},
  total: {},
)
"#,
            escape_string(&self.emisor.razon_social),
            escape_string(&self.emisor.domicilio_comercial),
            escape_string(&self.emisor.condicion_iva),
            escape_string(&self.emisor.cuit),
            escape_string(&self.emisor.ingresos_brutos),
            escape_string(&self.emisor.inicio_actividades),
            option_string(&self.receptor.nombre),
            option_string(&self.receptor.domicilio),
            escape_string(&self.receptor.condicion_iva),
            escape_string(&self.receptor.condicion_venta),
            option_string(&self.receptor.documento),
            escape_string(&self.comprobante.tipo),
            escape_string(&self.comprobante.codigo),
            escape_string(&self.comprobante.punto_de_venta),
            escape_string(&self.comprobante.numero),
            escape_string(&self.comprobante.fecha_emision),
            option_string(&self.comprobante.periodo_desde),
            option_string(&self.comprobante.periodo_hasta),
            escape_string(&self.comprobante.fecha_vencimiento),
            items_str.join(", "),
            escape_string(&self.cae.numero),
            escape_string(&self.cae.vencimiento),
            self.subtotal(),
            self.otros_tributos(),
            self.total(),
        )
    }
}
