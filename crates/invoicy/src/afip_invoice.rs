//! AFIP authorization step for `afip_c` invoices.
//!
//! Given a parsed `afip_c` TOML value, this computes the total from `items`,
//! requests a CAE from WSFE, and writes `numero` / `fecha_emision` /
//! `punto_de_venta` / `[cae]` back into the value so the normal render path can
//! produce the PDF.

use std::path::Path;

use afip::{Concepto, DocTipo, FacturaC};
use toml::Value;

use crate::commands::afip::load_client;
use crate::formats::{AfipCInvoice, ConceptoParam, DocTipoParam};

type BoxError = Box<dyn std::error::Error>;

/// Whether the TOML already carries a non-empty CAE (already authorized).
pub fn has_cae(value: &Value) -> bool {
    value
        .get("cae")
        .and_then(|c| c.get("numero"))
        .and_then(|n| n.as_str())
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false)
}

/// Authorize the invoice against WSFE and fold the result into `value`.
pub fn authorize(home: &Path, value: &mut Value) -> Result<(), BoxError> {
    ensure_placeholders(value);

    // Deserialize a copy to compute the total and read AFIP params / dates.
    let inv: AfipCInvoice = value.clone().try_into()?;
    let params = inv.afip.clone().unwrap_or_default();
    let total = inv.total();
    if total <= 0.0 {
        return Err("el total (suma de los items) debe ser positivo".into());
    }

    let concepto = concepto_to_afip(params.concepto);
    let needs_dates = matches!(
        concepto,
        Concepto::Servicios | Concepto::ProductosYServicios
    );
    let (desde, hasta, vto) = if needs_dates {
        (
            inv.comprobante
                .periodo_desde
                .as_deref()
                .and_then(ddmmyyyy_to_yyyymmdd),
            inv.comprobante
                .periodo_hasta
                .as_deref()
                .and_then(ddmmyyyy_to_yyyymmdd),
            ddmmyyyy_to_yyyymmdd(&inv.comprobante.fecha_vencimiento),
        )
    } else {
        (None, None, None)
    };

    let factura = FacturaC {
        concepto,
        doc_tipo: doc_tipo_to_afip(params.doc_tipo),
        doc_nro: params.doc_nro,
        importe_total: total,
        fecha: None,
        fecha_servicio_desde: desde,
        fecha_servicio_hasta: hasta,
        fecha_vto_pago: vto,
        condicion_iva_receptor: params.cond_iva_receptor,
    };

    let client = load_client(home)?;
    println!("Solicitando CAE a WSFE (total ${total:.2})…");
    let res = client.create_factura_c(&factura)?;

    let vto_cae = yyyymmdd_str_to_ddmmyyyy(&res.cae_vencimiento);
    set_str(
        value,
        "comprobante",
        "punto_de_venta",
        &format!("{:05}", res.punto_venta),
    );
    set_str(
        value,
        "comprobante",
        "numero",
        &format!("{:08}", res.numero),
    );
    set_str(
        value,
        "comprobante",
        "fecha_emision",
        &yyyymmdd_to_ddmmyyyy(res.fecha),
    );
    set_str(value, "cae", "numero", &res.cae);
    set_str(value, "cae", "vencimiento", &vto_cae);

    println!(
        "✔ CAE {} (vto {}) — comprobante {:05}-{:08}",
        res.cae, vto_cae, res.punto_venta, res.numero
    );
    Ok(())
}

fn set_str(value: &mut Value, table: &str, key: &str, v: &str) {
    if let Some(t) = value.get_mut(table).and_then(Value::as_table_mut) {
        t.insert(key.to_string(), Value::String(v.to_string()));
    }
}

/// Ensure `comprobante.{numero,fecha_emision,fecha_vencimiento}` and `[cae]`
/// exist so the struct deserializes; they get overwritten after authorization.
fn ensure_placeholders(value: &mut Value) {
    let Some(root) = value.as_table_mut() else {
        return;
    };
    let comp = root
        .entry("comprobante".to_string())
        .or_insert_with(|| Value::Table(Default::default()));
    if let Some(t) = comp.as_table_mut() {
        for key in ["numero", "fecha_emision", "fecha_vencimiento"] {
            t.entry(key.to_string())
                .or_insert_with(|| Value::String(String::new()));
        }
    }
    let cae = root
        .entry("cae".to_string())
        .or_insert_with(|| Value::Table(Default::default()));
    if let Some(t) = cae.as_table_mut() {
        for key in ["numero", "vencimiento"] {
            t.entry(key.to_string())
                .or_insert_with(|| Value::String(String::new()));
        }
    }
}

fn concepto_to_afip(c: ConceptoParam) -> Concepto {
    match c {
        ConceptoParam::Productos => Concepto::Productos,
        ConceptoParam::Servicios => Concepto::Servicios,
        ConceptoParam::ProductosYServicios => Concepto::ProductosYServicios,
    }
}

fn doc_tipo_to_afip(d: DocTipoParam) -> DocTipo {
    match d {
        DocTipoParam::ConsumidorFinal => DocTipo::ConsumidorFinal,
        DocTipoParam::Cuit => DocTipo::Cuit,
        DocTipoParam::Cuil => DocTipo::Cuil,
        DocTipoParam::Dni => DocTipo::Dni,
    }
}

fn ddmmyyyy_to_yyyymmdd(s: &str) -> Option<u32> {
    let p: Vec<&str> = s.split('/').collect();
    if p.len() != 3 {
        return None;
    }
    let d: u32 = p[0].trim().parse().ok()?;
    let m: u32 = p[1].trim().parse().ok()?;
    let y: u32 = p[2].trim().parse().ok()?;
    Some(y * 10000 + m * 100 + d)
}

fn yyyymmdd_to_ddmmyyyy(n: u32) -> String {
    let (y, m, d) = (n / 10000, (n / 100) % 100, n % 100);
    format!("{d:02}/{m:02}/{y:04}")
}

fn yyyymmdd_str_to_ddmmyyyy(s: &str) -> String {
    s.parse::<u32>()
        .map(yyyymmdd_to_ddmmyyyy)
        .unwrap_or_else(|_| s.to_string())
}
