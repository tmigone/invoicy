use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct AfipAInvoice {
    pub emisor: Emisor,
    pub receptor: Receptor,
    pub comprobante: Comprobante,
    pub items: Vec<LineItem>,
    #[serde(default)]
    pub otros_tributos: Vec<Tributo>,
    pub cae: Cae,
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

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct Receptor {
    pub nombre: Option<String>,
    pub domicilio: Option<String>,
    pub condicion_iva: String,
    pub condicion_venta: String,
    pub cuit: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
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

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct LineItem {
    pub codigo: String,
    pub descripcion: String,
    pub cantidad: f64,
    pub unidad: String,
    pub precio_unitario: f64,
    pub bonificacion_porcentaje: f64,
    pub subtotal: f64,
    pub alicuota_iva: f64,
    pub subtotal_con_iva: f64,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct Tributo {
    pub descripcion: String,
    pub detalle: Option<String>,
    pub alicuota: Option<f64>,
    pub importe: f64,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct Cae {
    pub numero: String,
    pub vencimiento: String,
}

impl AfipAInvoice {
    pub fn neto_gravado(&self) -> f64 {
        self.items.iter().map(|item| item.subtotal).sum()
    }

    pub fn total_iva(&self) -> f64 {
        self.items
            .iter()
            .map(|item| item.subtotal_con_iva - item.subtotal)
            .sum()
    }

    pub fn otros_tributos_total(&self) -> f64 {
        self.otros_tributos.iter().map(|t| t.importe).sum()
    }

    pub fn total(&self) -> f64 {
        self.items
            .iter()
            .map(|item| item.subtotal_con_iva)
            .sum::<f64>()
            + self.otros_tributos_total()
    }
}
