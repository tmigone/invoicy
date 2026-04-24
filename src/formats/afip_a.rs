use serde::Deserialize;

use super::{escape_typst_string, typst_option};

#[derive(Debug, Deserialize)]
pub struct AfipAInvoice {
    pub emisor: Emisor,
    pub receptor: Receptor,
    pub comprobante: Comprobante,
    pub items: Vec<LineItem>,
    pub otros_tributos: Option<Vec<Tributo>>,
    pub cae: Cae,
}

#[derive(Debug, Deserialize)]
pub struct Emisor {
    pub razon_social: String,
    pub domicilio_comercial: String,
    pub condicion_iva: String,
    pub cuit: String,
    pub ingresos_brutos: String,
    pub inicio_actividades: String,
}

#[derive(Debug, Deserialize)]
pub struct Receptor {
    pub nombre: Option<String>,
    pub domicilio: Option<String>,
    pub condicion_iva: String,
    pub condicion_venta: String,
    pub cuit: Option<String>,
}

#[derive(Debug, Deserialize)]
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

#[derive(Debug, Deserialize)]
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

#[derive(Debug, Deserialize)]
pub struct Tributo {
    pub descripcion: String,
    pub detalle: Option<String>,
    pub alicuota: Option<f64>,
    pub importe: f64,
}

#[derive(Debug, Deserialize)]
pub struct Cae {
    pub numero: String,
    pub vencimiento: String,
}

impl AfipAInvoice {
    fn neto_gravado(&self) -> f64 {
        self.items.iter().map(|item| item.subtotal).sum()
    }

    fn total_iva(&self) -> f64 {
        self.items
            .iter()
            .map(|item| item.subtotal_con_iva - item.subtotal)
            .sum()
    }

    fn otros_tributos_total(&self) -> f64 {
        self.otros_tributos
            .as_ref()
            .map(|t| t.iter().map(|x| x.importe).sum())
            .unwrap_or(0.0)
    }

    fn total(&self) -> f64 {
        self.items
            .iter()
            .map(|item| item.subtotal_con_iva)
            .sum::<f64>()
            + self.otros_tributos_total()
    }

    pub fn to_typst_dict(&self) -> String {
        let items_str: Vec<String> = self
            .items
            .iter()
            .map(|item| {
                format!(
                    r#"(codigo: "{}", descripcion: "{}", cantidad: {}, unidad: "{}", precio_unitario: {}, bonif_pct: {}, subtotal: {}, alicuota_iva: {}, subtotal_con_iva: {})"#,
                    escape_typst_string(&item.codigo),
                    escape_typst_string(&item.descripcion),
                    item.cantidad,
                    escape_typst_string(&item.unidad),
                    item.precio_unitario,
                    item.bonificacion_porcentaje,
                    item.subtotal,
                    item.alicuota_iva,
                    item.subtotal_con_iva
                )
            })
            .collect();

        let tributos_str: Vec<String> = self
            .otros_tributos
            .as_ref()
            .map(|tributos| {
                tributos
                    .iter()
                    .map(|t| {
                        format!(
                            r#"(descripcion: "{}", detalle: {}, alicuota: {}, importe: {})"#,
                            escape_typst_string(&t.descripcion),
                            typst_option(&t.detalle),
                            t.alicuota
                                .map(|a| a.to_string())
                                .unwrap_or_else(|| "none".to_string()),
                            t.importe
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();

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
    cuit: {},
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
  otros_tributos: ({},),
  cae: (
    numero: "{}",
    vencimiento: "{}",
  ),
  totales: (
    neto_gravado: {},
    iva_27: 0.0,
    iva_21: {},
    iva_10_5: 0.0,
    iva_5: 0.0,
    iva_2_5: 0.0,
    iva_0: 0.0,
    otros_tributos: {},
    total: {},
  ),
)
"#,
            escape_typst_string(&self.emisor.razon_social),
            escape_typst_string(&self.emisor.domicilio_comercial),
            escape_typst_string(&self.emisor.condicion_iva),
            escape_typst_string(&self.emisor.cuit),
            escape_typst_string(&self.emisor.ingresos_brutos),
            escape_typst_string(&self.emisor.inicio_actividades),
            typst_option(&self.receptor.nombre),
            typst_option(&self.receptor.domicilio),
            escape_typst_string(&self.receptor.condicion_iva),
            escape_typst_string(&self.receptor.condicion_venta),
            typst_option(&self.receptor.cuit),
            escape_typst_string(&self.comprobante.tipo),
            escape_typst_string(&self.comprobante.codigo),
            escape_typst_string(&self.comprobante.punto_de_venta),
            escape_typst_string(&self.comprobante.numero),
            escape_typst_string(&self.comprobante.fecha_emision),
            typst_option(&self.comprobante.periodo_desde),
            typst_option(&self.comprobante.periodo_hasta),
            escape_typst_string(&self.comprobante.fecha_vencimiento),
            items_str.join(", "),
            tributos_str.join(", "),
            escape_typst_string(&self.cae.numero),
            escape_typst_string(&self.cae.vencimiento),
            self.neto_gravado(),
            self.total_iva(),
            self.otros_tributos_total(),
            self.total(),
        )
    }
}
