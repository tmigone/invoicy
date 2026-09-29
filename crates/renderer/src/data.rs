//! The JSON document templates receive as `invoice-data`: the invoice with the
//! same field names as the TOML, with AFIP codes (e.g.
//! `receptor.condicion_iva`) replaced by their printed labels. An `afip_c`
//! invoice must be complete (see `AfipCInvoice::compute`): its subtotals and
//! totals are printed as stored, so no arithmetic lives in Typst.

use afip::DocTipo;
use serde::Serialize;

use schema::afip_c::{self, Receptor};
use schema::{GenericInvoice, InvoiceConfig};

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
    items: Vec<AfipCItem<'a>>,
    cae: &'a afip_c::Cae,
    /// QR text; the template shows `qr.svg` when this isn't empty.
    qr: &'a str,
    subtotal: f64,
    otros_tributos: f64,
    total: f64,
}

/// A line of the invoice. The layout keeps AFIP's bonificación columns;
/// invoicy doesn't support discounts, so they are always zero.
#[derive(Serialize)]
struct AfipCItem<'a> {
    #[serde(flatten)]
    item: &'a afip_c::LineItem,
    bonificacion_porcentaje: f64,
    bonificacion_importe: f64,
}

impl<'a> From<&'a afip_c::LineItem> for AfipCItem<'a> {
    fn from(item: &'a afip_c::LineItem) -> Self {
        Self {
            item,
            bonificacion_porcentaje: 0.0,
            bonificacion_importe: 0.0,
        }
    }
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
            items: inv.items.iter().map(Into::into).collect(),
            cae: &inv.cae,
            qr: &inv.qr,
            subtotal: inv.totales.subtotal,
            otros_tributos: inv.totales.otros_tributos,
            total: inv.totales.total,
        }),
    };
    json.map_err(|e| format!("cannot serialize invoice data: {e}"))
}
