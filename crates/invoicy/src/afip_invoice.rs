//! AFIP authorization step for `afip_c` invoices.
//!
//! Completes a draft [`AfipCInvoice`] (only user-written fields) into the full
//! invoice: the emisor and punto de venta come from the profile, the computed
//! fields are filled in, and the número, fecha and CAE are the ones WSFE
//! assigns when it authorizes the invoice.

use std::path::Path;

use afip::{DocTipo, FacturaC};
use chrono::{Datelike, Days, FixedOffset, NaiveDate, Utc};
use schema::AfipCInvoice;
use schema::afip_c::{Cae, Receptor};

use crate::emisor::EmisorProfile;

type BoxError = Box<dyn std::error::Error>;

/// Default payment term: `fecha_vencimiento` is this many days after
/// `fecha_emision` when the draft doesn't set it.
const DIAS_VENCIMIENTO: u64 = 15;

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

/// Everything [`authorize`] does short of requesting the CAE: fill in and
/// validate the invoice, then log in to AFIP (proving the certificate works
/// and is authorized for WSFE) and read the last voucher number. Nothing is
/// issued. Sets `comprobante.numero` to the number the invoice would get.
pub fn check(profile: &EmisorProfile, home: &Path, inv: &mut AfipCInvoice) -> Result<(), BoxError> {
    prepare(profile, inv)?;

    let client = profile.client(home)?;
    let last = client.last_voucher(inv.voucher_type())?;
    inv.comprobante.numero = format!("{:08}", last + 1);
    Ok(())
}

/// Fill in what's known before calling AFIP (emisor, punto de venta, computed
/// fields), validate, and build the WSFE request.
fn prepare(profile: &EmisorProfile, inv: &mut AfipCInvoice) -> Result<FacturaC, BoxError> {
    inv.emisor = profile.emisor();
    inv.comprobante.punto_de_venta = format!("{:05}", profile.punto_venta);

    // Dates: validate what the draft wrote, fill in the defaults, and store
    // the values used so the record and the PDF show exactly what AFIP gets.
    let comprobante = &mut inv.comprobante;
    let emision =
        parse_fecha("fecha_emision", &comprobante.fecha_emision)?.unwrap_or_else(today_ar);
    let vencimiento = parse_fecha("fecha_vencimiento", &comprobante.fecha_vencimiento)?
        .unwrap_or(emision + Days::new(DIAS_VENCIMIENTO));
    let desde = parse_periodo("periodo_desde", comprobante.periodo_desde.as_deref())?;
    let hasta = parse_periodo("periodo_hasta", comprobante.periodo_hasta.as_deref())?;
    comprobante.fecha_emision = ddmmyyyy(emision);
    comprobante.fecha_vencimiento = ddmmyyyy(vencimiento);

    inv.compute();

    let total = inv.totales.total;
    if total <= 0.0 {
        return Err("el total (suma de los items) debe ser positivo".into());
    }
    check_documento(&inv.receptor)?;

    let concepto = inv.comprobante.concepto;
    let (desde, hasta, vto) = if concepto.requires_service_dates() {
        // AFIP needs the billing period for services; there's no sensible
        // default, and the PDF must show the same period AFIP authorizes.
        let (Some(desde), Some(hasta)) = (desde, hasta) else {
            return Err("para servicios hacen falta comprobante.periodo_desde y \
                        comprobante.periodo_hasta"
                .into());
        };
        (
            Some(yyyymmdd(desde)),
            Some(yyyymmdd(hasta)),
            Some(yyyymmdd(vencimiento)),
        )
    } else {
        (None, None, None)
    };

    Ok(FacturaC {
        concepto,
        doc_tipo: inv.receptor.doc_tipo,
        doc_nro: inv.receptor.doc_nro,
        importe_total: total,
        fecha: Some(yyyymmdd(emision)),
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

/// Today in Argentina (UTC−03:00, no DST).
fn today_ar() -> NaiveDate {
    let offset = FixedOffset::west_opt(3 * 3600).expect("valid offset");
    Utc::now().with_timezone(&offset).date_naive()
}

/// `comprobante.<field>` as a `DD/MM/YYYY` date; `None` when empty.
fn parse_fecha(field: &str, value: &str) -> Result<Option<NaiveDate>, BoxError> {
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    NaiveDate::parse_from_str(value, "%d/%m/%Y")
        .map(Some)
        .map_err(|_| {
            format!("comprobante.{field} no es una fecha válida (DD/MM/YYYY): \"{value}\"").into()
        })
}

fn parse_periodo(field: &str, value: Option<&str>) -> Result<Option<NaiveDate>, BoxError> {
    Ok(match value {
        Some(value) => parse_fecha(field, value)?,
        None => None,
    })
}

fn ddmmyyyy(date: NaiveDate) -> String {
    date.format("%d/%m/%Y").to_string()
}

fn yyyymmdd(date: NaiveDate) -> u32 {
    date.year() as u32 * 10000 + date.month() * 100 + date.day()
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

    #[test]
    fn dates_default_to_today_and_fifteen_days_later() {
        let mut inv = draft("");
        inv.comprobante.fecha_vencimiento.clear();
        let factura = prepare(&profile(3), &mut inv).unwrap();

        let today = today_ar();
        let due = today + Days::new(15);
        assert_eq!(inv.comprobante.fecha_emision, ddmmyyyy(today));
        assert_eq!(inv.comprobante.fecha_vencimiento, ddmmyyyy(due));
        assert_eq!(factura.fecha, Some(yyyymmdd(today)));
        assert_eq!(factura.fecha_vto_pago, Some(yyyymmdd(due)));
    }

    #[test]
    fn due_date_follows_a_written_issue_date() {
        let mut inv = draft("");
        inv.comprobante.fecha_emision = "25/12/2026".into();
        inv.comprobante.fecha_vencimiento.clear();
        let factura = prepare(&profile(3), &mut inv).unwrap();

        assert_eq!(factura.fecha, Some(20261225));
        assert_eq!(inv.comprobante.fecha_vencimiento, "09/01/2027");
        assert_eq!(factura.fecha_vto_pago, Some(20270109));
    }

    #[test]
    fn invalid_dates_are_rejected_instead_of_replaced() {
        for (field, set) in [
            ("fecha_emision", "31/02/2026"),
            ("fecha_vencimiento", "2026-10-10"),
            ("periodo_desde", "1/9"),
        ] {
            let mut inv = draft("");
            match field {
                "fecha_emision" => inv.comprobante.fecha_emision = set.into(),
                "fecha_vencimiento" => inv.comprobante.fecha_vencimiento = set.into(),
                _ => inv.comprobante.periodo_desde = Some(set.into()),
            }
            let err = prepare(&profile(3), &mut inv).unwrap_err().to_string();
            assert!(err.contains(field), "{field}: {err}");
        }
    }

    #[test]
    fn services_need_a_billing_period() {
        let mut inv = draft("");
        inv.comprobante.periodo_hasta = None;
        let err = prepare(&profile(3), &mut inv).unwrap_err().to_string();
        assert!(err.contains("periodo_hasta"), "{err}");

        // Products don't.
        let mut inv = draft("");
        inv.comprobante.concepto = afip::Concepto::Productos;
        inv.comprobante.periodo_desde = None;
        inv.comprobante.periodo_hasta = None;
        let factura = prepare(&profile(3), &mut inv).unwrap();
        assert_eq!(factura.fecha_servicio_desde, None);
    }
}
