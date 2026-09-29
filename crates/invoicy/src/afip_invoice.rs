//! AFIP authorization step for `afip_c` invoices.
//!
//! Completes a draft [`AfipCInvoice`] (only user-written fields) into the full
//! invoice: the emisor and punto de venta come from the profile, the computed
//! fields are filled in, and the número, fecha and CAE are the ones WSFE
//! assigns when it authorizes the invoice.

use std::path::Path;

use afip::{DocTipo, FacturaC};
use schema::AfipCInvoice;
use schema::afip_c::{Cae, Receptor};

use crate::emisor::EmisorProfile;

type BoxError = Box<dyn std::error::Error>;

/// Authorize `inv` against WSFE, filling in every automatic field.
pub fn authorize(
    profile: &EmisorProfile,
    home: &Path,
    inv: &mut AfipCInvoice,
) -> Result<(), BoxError> {
    let factura = prepare(profile, inv)?;

    let client = profile.client(home)?;
    println!("Solicitando CAE a WSFE (total ${:.2})…", inv.totales.total);
    let res = client.create_factura_c(&factura)?;

    inv.comprobante.punto_de_venta = format!("{:05}", res.punto_venta);
    inv.comprobante.numero = format!("{:08}", res.numero);
    inv.comprobante.fecha_emision = yyyymmdd_to_ddmmyyyy(res.fecha);
    inv.cae = Cae {
        numero: res.cae,
        vencimiento: yyyymmdd_str_to_ddmmyyyy(&res.cae_vencimiento),
    };
    // Now that AFIP assigned número, fecha and CAE, the QR can be built.
    inv.compute();
    if inv.qr.is_empty() {
        // Don't fail: the CAE is issued and the invoice must still be written.
        eprintln!("⚠ no se pudo generar el código QR con los datos devueltos por AFIP");
    }

    println!(
        "✔ CAE {} (vto {}) — comprobante {}-{}",
        inv.cae.numero, inv.cae.vencimiento, inv.comprobante.punto_de_venta, inv.comprobante.numero
    );
    Ok(())
}

/// Fill in what's known before calling AFIP (emisor, punto de venta, computed
/// fields), validate, and build the WSFE request.
fn prepare(profile: &EmisorProfile, inv: &mut AfipCInvoice) -> Result<FacturaC, BoxError> {
    inv.emisor = profile.emisor();
    inv.comprobante.punto_de_venta = format!("{:05}", profile.punto_venta);
    inv.compute();

    let total = inv.totales.total;
    if total <= 0.0 {
        return Err("el total (suma de los items) debe ser positivo".into());
    }
    check_documento(&inv.receptor)?;

    let concepto = inv.comprobante.concepto;
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

    Ok(FacturaC {
        concepto,
        doc_tipo: inv.receptor.doc_tipo,
        doc_nro: inv.receptor.doc_nro,
        importe_total: total,
        fecha: None,
        fecha_servicio_desde: desde,
        fecha_servicio_hasta: hasta,
        fecha_vto_pago: vto,
        condicion_iva_receptor: inv.receptor.condicion_iva,
    })
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

#[cfg(test)]
mod tests {
    use super::*;
    use schema::InvoiceConfig;

    fn profile(punto_venta: u32) -> EmisorProfile {
        EmisorProfile {
            cuit: 20111111112,
            razon_social: "Test".into(),
            domicilio_comercial: String::new(),
            condicion_iva: "Responsable Monotributo".into(),
            ingresos_brutos: String::new(),
            inicio_actividades: String::new(),
            punto_venta,
            environment: afip::Environment::Homologacion,
            cert_path: "cert.crt".into(),
            key_path: "key.key".into(),
        }
    }

    fn draft(receptor: &str) -> AfipCInvoice {
        let src = format!(
            r#"
            format = "afip_c"
            [receptor]
            condicion_venta = "Contado"
            {receptor}
            [comprobante]
            concepto = "servicios"
            periodo_desde = "01/09/2026"
            periodo_hasta = "30/09/2026"
            fecha_vencimiento = "10/10/2026"
            [[items]]
            codigo = "1"
            descripcion = "x"
            cantidad = 2.0
            unidad = "u"
            precio_unitario = 1500.5
            "#
        );
        match toml::from_str(&src).unwrap() {
            InvoiceConfig::AfipC(inv) => inv,
            _ => unreachable!(),
        }
    }

    #[test]
    fn prepare_fills_profile_and_computed_fields() {
        let mut inv = draft("");
        let factura = prepare(&profile(3), &mut inv).unwrap();

        assert_eq!(inv.emisor.cuit, "20111111112");
        assert_eq!(inv.comprobante.punto_de_venta, "00003");
        assert_eq!(inv.comprobante.tipo, "C");
        assert_eq!(inv.totales.total, 3001.0);

        assert_eq!(factura.importe_total, 3001.0);
        assert_eq!(factura.fecha_servicio_desde, Some(20260901));
        assert_eq!(factura.fecha_servicio_hasta, Some(20260930));
        assert_eq!(factura.fecha_vto_pago, Some(20261010));
    }

    #[test]
    fn prepare_rejects_document_mismatch() {
        assert!(prepare(&profile(3), &mut draft("doc_tipo = \"cuit\"")).is_err());
        assert!(prepare(&profile(3), &mut draft("doc_nro = 123")).is_err());
        assert!(prepare(&profile(3), &mut draft("doc_tipo = \"dni\"\ndoc_nro = 123")).is_ok());
    }
}
