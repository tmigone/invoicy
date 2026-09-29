//! The JSON document templates receive as `invoice-data`: the invoice as
//! parsed (same field names as the TOML), plus the totals the templates print.
//! Totals are computed here so no arithmetic lives in Typst, and AFIP codes
//! (e.g. `receptor.condicion_iva`) are replaced by their printed labels.

use afip::DocTipo;
use serde::Serialize;

use schema::afip_c::{self, Receptor};
use schema::{AfipAInvoice, GenericInvoice, InvoiceConfig};

#[derive(Serialize)]
struct GenericData<'a> {
    #[serde(flatten)]
    invoice: &'a GenericInvoice,
    total: f64,
}

#[derive(Serialize)]
struct AfipCData<'a> {
    emisor: &'a afip_c::Emisor,
    receptor: AfipCReceptor<'a>,
    comprobante: &'a afip_c::Comprobante,
    items: &'a [afip_c::LineItem],
    cae: &'a afip_c::Cae,
    subtotal: f64,
    otros_tributos: f64,
    total: f64,
}

/// The receptor with its AFIP codes turned into the text printed on the PDF.
#[derive(Serialize)]
struct AfipCReceptor<'a> {
    nombre: Option<&'a str>,
    domicilio: Option<&'a str>,
    condicion_iva: &'static str,
    condicion_venta: &'a str,
    /// Label of the document row, e.g. "CUIT" or "DNI".
    doc_tipo: &'static str,
    /// `None` for an unidentified consumidor final.
    doc_nro: Option<String>,
}

impl<'a> From<&'a Receptor> for AfipCReceptor<'a> {
    fn from(r: &'a Receptor) -> Self {
        let (doc_tipo, doc_nro) = match r.doc_tipo {
            // No document to show: keep the row labelled CUIT and leave it blank.
            DocTipo::ConsumidorFinal => ("CUIT", None),
            tipo => (tipo.label(), Some(r.doc_nro.to_string())),
        };
        Self {
            nombre: r.nombre.as_deref(),
            domicilio: r.domicilio.as_deref(),
            condicion_iva: r.condicion_iva.label(),
            condicion_venta: &r.condicion_venta,
            doc_tipo,
            doc_nro,
        }
    }
}

#[derive(Serialize)]
struct AfipAData<'a> {
    #[serde(flatten)]
    invoice: &'a AfipAInvoice,
    totales: AfipATotales,
}

/// Per-rate IVA totals. Only 21% is broken out; every other rate prints 0.
#[derive(Serialize)]
struct AfipATotales {
    neto_gravado: f64,
    iva_27: f64,
    iva_21: f64,
    iva_10_5: f64,
    iva_5: f64,
    iva_2_5: f64,
    iva_0: f64,
    otros_tributos: f64,
    total: f64,
}

/// Serialize `config` into the JSON document the template reads.
pub(crate) fn to_json(config: &InvoiceConfig) -> Result<Vec<u8>, String> {
    let json = match config {
        InvoiceConfig::Generic(inv) => serde_json::to_vec(&GenericData {
            invoice: inv,
            total: inv.total(),
        }),
        InvoiceConfig::AfipC(inv) => serde_json::to_vec(&AfipCData {
            emisor: &inv.emisor,
            receptor: (&inv.receptor).into(),
            comprobante: &inv.comprobante,
            items: &inv.items,
            cae: &inv.cae,
            subtotal: inv.subtotal(),
            otros_tributos: inv.otros_tributos(),
            total: inv.total(),
        }),
        InvoiceConfig::AfipA(inv) => serde_json::to_vec(&AfipAData {
            invoice: inv,
            totales: AfipATotales {
                neto_gravado: inv.neto_gravado(),
                iva_27: 0.0,
                iva_21: inv.total_iva(),
                iva_10_5: 0.0,
                iva_5: 0.0,
                iva_2_5: 0.0,
                iva_0: 0.0,
                otros_tributos: inv.otros_tributos_total(),
                total: inv.total(),
            },
        }),
    };
    json.map_err(|e| format!("cannot serialize invoice data: {e}"))
}
