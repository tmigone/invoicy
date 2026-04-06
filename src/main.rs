use std::path::PathBuf;

use clap::Parser;
use serde::Deserialize;

mod world;

/// Generate PDF invoices from TOML config and Typst templates
#[derive(Parser, Debug)]
#[command(name = "invoicy", version, about)]
struct Args {
    /// Path to the invoice config file (TOML)
    #[arg(short, long)]
    config: PathBuf,

    /// Path to the Typst template file (optional, uses built-in template based on format)
    #[arg(short, long)]
    template: Option<PathBuf>,

    /// Output PDF path (defaults to invoice-{number}.pdf)
    #[arg(short, long)]
    output: Option<PathBuf>,
}

// ============================================================================
// Invoice Format Enum (tagged union)
// ============================================================================

#[derive(Debug, Deserialize)]
#[serde(tag = "format")]
enum InvoiceConfig {
    #[serde(rename = "generic")]
    Generic(GenericInvoice),
    #[serde(rename = "afip_c")]
    AfipC(AfipCInvoice),
    #[serde(rename = "afip_a")]
    AfipA(AfipAInvoice),
}

impl InvoiceConfig {
    fn to_typst_dict(&self) -> String {
        match self {
            InvoiceConfig::Generic(inv) => inv.to_typst_dict(),
            InvoiceConfig::AfipC(inv) => inv.to_typst_dict(),
            InvoiceConfig::AfipA(inv) => inv.to_typst_dict(),
        }
    }

    fn invoice_number(&self) -> String {
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

    fn default_template(&self) -> &'static str {
        match self {
            InvoiceConfig::Generic(_) => include_str!("../templates/generic.typ"),
            InvoiceConfig::AfipC(_) => include_str!("../templates/afip_c.typ"),
            InvoiceConfig::AfipA(_) => include_str!("../templates/afip_a.typ"),
        }
    }
}

// ============================================================================
// Generic Invoice Format
// ============================================================================

#[derive(Debug, Deserialize)]
struct GenericInvoice {
    company: GenericCompany,
    client: GenericClient,
    invoice: GenericInvoiceMeta,
    items: Vec<GenericLineItem>,
}

#[derive(Debug, Deserialize)]
struct GenericCompany {
    name: String,
    address: String,
    city_state_zip: String,
    country: String,
}

#[derive(Debug, Deserialize)]
struct GenericClient {
    name: String,
    address: String,
    city_state_zip: String,
    country: String,
    tax_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GenericInvoiceMeta {
    number: String,
    date: String,
    due_date: String,
    currency: String,
}

#[derive(Debug, Deserialize)]
struct GenericLineItem {
    description: String,
    rate: f64,
}

impl GenericInvoice {
    fn total(&self) -> f64 {
        self.items.iter().map(|item| item.rate).sum()
    }

    fn to_typst_dict(&self) -> String {
        let items_str: Vec<String> = self
            .items
            .iter()
            .map(|item| {
                format!(
                    "(description: \"{}\", rate: {})",
                    escape_typst_string(&item.description),
                    item.rate
                )
            })
            .collect();

        format!(
            r#"#let invoice-data = (
  company: (
    name: "{}",
    address: "{}",
    city_state_zip: "{}",
    country: "{}",
  ),
  client: (
    name: "{}",
    address: "{}",
    city_state_zip: "{}",
    country: "{}",
    tax_id: {},
  ),
  invoice: (
    number: "{}",
    date: "{}",
    due_date: "{}",
    currency: "{}",
  ),
  items: ({},),
  total: {},
)
"#,
            escape_typst_string(&self.company.name),
            escape_typst_string(&self.company.address),
            escape_typst_string(&self.company.city_state_zip),
            escape_typst_string(&self.company.country),
            escape_typst_string(&self.client.name),
            escape_typst_string(&self.client.address),
            escape_typst_string(&self.client.city_state_zip),
            escape_typst_string(&self.client.country),
            self.client
                .tax_id
                .as_ref()
                .map(|s| format!("\"{}\"", escape_typst_string(s)))
                .unwrap_or_else(|| "none".to_string()),
            escape_typst_string(&self.invoice.number),
            escape_typst_string(&self.invoice.date),
            escape_typst_string(&self.invoice.due_date),
            escape_typst_string(&self.invoice.currency),
            items_str.join(", "),
            self.total(),
        )
    }
}

// ============================================================================
// AFIP Invoice Format (Argentina)
// ============================================================================

#[derive(Debug, Deserialize)]
struct AfipCInvoice {
    emisor: AfipCEmisor,
    receptor: AfipCReceptor,
    comprobante: AfipCComprobante,
    items: Vec<AfipCLineItem>,
    cae: AfipCae,
}

#[derive(Debug, Deserialize)]
struct AfipCEmisor {
    razon_social: String,
    domicilio_comercial: String,
    condicion_iva: String,
    cuit: String,
    ingresos_brutos: String,
    inicio_actividades: String,
}

#[derive(Debug, Deserialize)]
struct AfipCReceptor {
    nombre: Option<String>,
    domicilio: Option<String>,
    condicion_iva: String,
    condicion_venta: String,
    documento: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AfipCComprobante {
    tipo: String,           // "C", "A", "B"
    codigo: String,         // "011"
    punto_de_venta: String, // "00001"
    numero: String,         // "00000083"
    fecha_emision: String,
    periodo_desde: Option<String>,
    periodo_hasta: Option<String>,
    fecha_vencimiento: String,
}

#[derive(Debug, Deserialize)]
struct AfipCLineItem {
    codigo: String,
    descripcion: String,
    cantidad: f64,
    unidad: String,
    precio_unitario: f64,
    bonificacion_porcentaje: f64,
    bonificacion_importe: f64,
    subtotal: f64,
}

#[derive(Debug, Deserialize)]
struct AfipCae {
    numero: String,
    vencimiento: String,
}

impl AfipCInvoice {
    fn subtotal(&self) -> f64 {
        self.items.iter().map(|item| item.subtotal).sum()
    }

    fn otros_tributos(&self) -> f64 {
        0.0 // Can be extended if needed
    }

    fn total(&self) -> f64 {
        self.subtotal() + self.otros_tributos()
    }

    fn to_typst_dict(&self) -> String {
        let items_str: Vec<String> = self
            .items
            .iter()
            .map(|item| {
                format!(
                    r#"(codigo: "{}", descripcion: "{}", cantidad: {}, unidad: "{}", precio_unitario: {}, bonif_pct: {}, bonif_imp: {}, subtotal: {})"#,
                    escape_typst_string(&item.codigo),
                    escape_typst_string(&item.descripcion),
                    item.cantidad,
                    escape_typst_string(&item.unidad),
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
            escape_typst_string(&self.emisor.razon_social),
            escape_typst_string(&self.emisor.domicilio_comercial),
            escape_typst_string(&self.emisor.condicion_iva),
            escape_typst_string(&self.emisor.cuit),
            escape_typst_string(&self.emisor.ingresos_brutos),
            escape_typst_string(&self.emisor.inicio_actividades),
            self.receptor
                .nombre
                .as_ref()
                .map(|s| format!("\"{}\"", escape_typst_string(s)))
                .unwrap_or_else(|| "none".to_string()),
            self.receptor
                .domicilio
                .as_ref()
                .map(|s| format!("\"{}\"", escape_typst_string(s)))
                .unwrap_or_else(|| "none".to_string()),
            escape_typst_string(&self.receptor.condicion_iva),
            escape_typst_string(&self.receptor.condicion_venta),
            self.receptor
                .documento
                .as_ref()
                .map(|s| format!("\"{}\"", escape_typst_string(s)))
                .unwrap_or_else(|| "none".to_string()),
            escape_typst_string(&self.comprobante.tipo),
            escape_typst_string(&self.comprobante.codigo),
            escape_typst_string(&self.comprobante.punto_de_venta),
            escape_typst_string(&self.comprobante.numero),
            escape_typst_string(&self.comprobante.fecha_emision),
            self.comprobante
                .periodo_desde
                .as_ref()
                .map(|s| format!("\"{}\"", escape_typst_string(s)))
                .unwrap_or_else(|| "none".to_string()),
            self.comprobante
                .periodo_hasta
                .as_ref()
                .map(|s| format!("\"{}\"", escape_typst_string(s)))
                .unwrap_or_else(|| "none".to_string()),
            escape_typst_string(&self.comprobante.fecha_vencimiento),
            items_str.join(", "),
            escape_typst_string(&self.cae.numero),
            escape_typst_string(&self.cae.vencimiento),
            self.subtotal(),
            self.otros_tributos(),
            self.total(),
        )
    }
}

// ============================================================================
// AFIP IVA Invoice Format (Argentina - Responsable Inscripto)
// ============================================================================

#[derive(Debug, Deserialize)]
struct AfipAInvoice {
    emisor: AfipAEmisor,
    receptor: AfipAReceptor,
    comprobante: AfipAComprobante,
    items: Vec<AfipALineItem>,
    otros_tributos: Option<Vec<AfipATributo>>,
    cae: AfipCae,
}

#[derive(Debug, Deserialize)]
struct AfipAEmisor {
    razon_social: String,
    domicilio_comercial: String,
    condicion_iva: String,
    cuit: String,
    ingresos_brutos: String,
    inicio_actividades: String,
}

#[derive(Debug, Deserialize)]
struct AfipAReceptor {
    nombre: Option<String>,
    domicilio: Option<String>,
    condicion_iva: String,
    condicion_venta: String,
    cuit: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AfipAComprobante {
    tipo: String,
    codigo: String,
    punto_de_venta: String,
    numero: String,
    fecha_emision: String,
    periodo_desde: Option<String>,
    periodo_hasta: Option<String>,
    fecha_vencimiento: String,
}

#[derive(Debug, Deserialize)]
struct AfipALineItem {
    codigo: String,
    descripcion: String,
    cantidad: f64,
    unidad: String,
    precio_unitario: f64,
    bonificacion_porcentaje: f64,
    subtotal: f64,
    alicuota_iva: f64,
    subtotal_con_iva: f64,
}

#[derive(Debug, Deserialize)]
struct AfipATributo {
    descripcion: String,
    detalle: Option<String>,
    alicuota: Option<f64>,
    importe: f64,
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
        self.items.iter().map(|item| item.subtotal_con_iva).sum::<f64>() + self.otros_tributos_total()
    }

    fn to_typst_dict(&self) -> String {
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
                            t.detalle
                                .as_ref()
                                .map(|s| format!("\"{}\"", escape_typst_string(s)))
                                .unwrap_or_else(|| "none".to_string()),
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
            self.receptor
                .nombre
                .as_ref()
                .map(|s| format!("\"{}\"", escape_typst_string(s)))
                .unwrap_or_else(|| "none".to_string()),
            self.receptor
                .domicilio
                .as_ref()
                .map(|s| format!("\"{}\"", escape_typst_string(s)))
                .unwrap_or_else(|| "none".to_string()),
            escape_typst_string(&self.receptor.condicion_iva),
            escape_typst_string(&self.receptor.condicion_venta),
            self.receptor
                .cuit
                .as_ref()
                .map(|s| format!("\"{}\"", escape_typst_string(s)))
                .unwrap_or_else(|| "none".to_string()),
            escape_typst_string(&self.comprobante.tipo),
            escape_typst_string(&self.comprobante.codigo),
            escape_typst_string(&self.comprobante.punto_de_venta),
            escape_typst_string(&self.comprobante.numero),
            escape_typst_string(&self.comprobante.fecha_emision),
            self.comprobante
                .periodo_desde
                .as_ref()
                .map(|s| format!("\"{}\"", escape_typst_string(s)))
                .unwrap_or_else(|| "none".to_string()),
            self.comprobante
                .periodo_hasta
                .as_ref()
                .map(|s| format!("\"{}\"", escape_typst_string(s)))
                .unwrap_or_else(|| "none".to_string()),
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

// ============================================================================
// Utilities
// ============================================================================

fn escape_typst_string(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

// ============================================================================
// Main
// ============================================================================

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Read and parse config
    let config_content = std::fs::read_to_string(&args.config)?;
    let config: InvoiceConfig = toml::from_str(&config_content)?;

    // Get template (from file or built-in)
    let template_content = match &args.template {
        Some(path) => std::fs::read_to_string(path)?,
        None => config.default_template().to_string(),
    };

    // Generate data definition + template
    let full_source = format!("{}\n{}", config.to_typst_dict(), template_content);

    // Determine output path
    let output_path = args
        .output
        .unwrap_or_else(|| PathBuf::from(format!("invoice-{}.pdf", config.invoice_number())));

    // Compile to PDF (assets loaded from ./assets relative to cwd)
    let pdf_bytes = world::compile_to_pdf(&full_source, PathBuf::from("assets"))?;

    // Write output
    std::fs::write(&output_path, pdf_bytes)?;
    println!("Generated: {}", output_path.display());

    Ok(())
}
