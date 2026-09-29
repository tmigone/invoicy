//! AFIP authorization step for `afip_c` invoices.
//!
//! Given a parsed `afip_c` TOML value, this computes the total from `items`,
//! requests a CAE from WSFE, and writes `numero` / `fecha_emision` /
//! `punto_de_venta` / `[cae]` back into the value so the normal render path can
//! produce the PDF.

use std::path::Path;

use afip::{DocTipo, FacturaC};
use schema::InvoiceConfig;
use schema::afip_c::Receptor;
use toml::Value;

use crate::emisor::EmisorProfile;

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

/// Whether the TOML carries its own `[emisor]` block (an override).
pub fn has_emisor(value: &Value) -> bool {
    value.get("emisor").is_some()
}

/// Inject the emisor profile as the `[emisor]` table (invoice omitted it).
pub fn inject_emisor(value: &mut Value, profile: &EmisorProfile) {
    if let Some(root) = value.as_table_mut() {
        root.insert("emisor".to_string(), Value::Table(profile.emisor_table()));
    }
}

/// Authorize the invoice against WSFE and fold the result into `value`.
pub fn authorize(profile: &EmisorProfile, home: &Path, value: &mut Value) -> Result<(), BoxError> {
    ensure_placeholders(value);

    // Deserialize a copy to compute the total and read the receptor / dates.
    let InvoiceConfig::AfipC(inv) = value.clone().try_into()? else {
        return Err("se esperaba un comprobante afip_c".into());
    };
    let total = inv.total();
    if total <= 0.0 {
        return Err("el total (suma de los items) debe ser positivo".into());
    }
    check_documento(&inv.receptor)?;

    let concepto = inv.concepto();
    let (desde, hasta, vto) = if concepto.requires_service_dates() {
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
        doc_tipo: inv.receptor.doc_tipo,
        doc_nro: inv.receptor.doc_nro,
        importe_total: total,
        fecha: None,
        fecha_servicio_desde: desde,
        fecha_servicio_hasta: hasta,
        fecha_vto_pago: vto,
        condicion_iva_receptor: inv.receptor.condicion_iva,
    };

    let client = profile.client(home)?;
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

/// Catch a document type/number mismatch before it reaches AFIP.
fn check_documento(receptor: &Receptor) -> Result<(), BoxError> {
    match (receptor.doc_tipo, receptor.doc_nro) {
        (DocTipo::ConsumidorFinal, 0) => Ok(()),
        (DocTipo::ConsumidorFinal, _) => Err(
            "receptor.doc_nro requiere receptor.doc_tipo (cuit, cuil o dni); \
             consumidor_final no lleva número"
                .into(),
        ),
        (tipo, 0) => Err(format!(
            "receptor.doc_nro es obligatorio cuando receptor.doc_tipo es {}",
            tipo.label()
        )
        .into()),
        _ => Ok(()),
    }
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
