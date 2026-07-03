//! WSFEv1 — Web Service de Facturación Electrónica.
//!
//! Two calls are needed to issue a voucher: `FECompUltimoAutorizado` to learn
//! the last authorized number, and `FECAESolicitar` to request the CAE for the
//! next one.

use chrono::{FixedOffset, Utc};

use crate::error::{Error, Result};
use crate::types::{CaeResult, FacturaC, VoucherInfo, VoucherType};
use crate::wsaa::Credentials;
use crate::xml;

const NS: &str = "http://ar.gov.afip.dif.FEV1/";

/// SOAP `Auth` block inputs.
struct Auth<'a> {
    token: &'a str,
    sign: &'a str,
    cuit: u64,
}

impl Auth<'_> {
    fn xml(&self) -> String {
        format!(
            "<ar:Auth><ar:Token>{}</ar:Token><ar:Sign>{}</ar:Sign><ar:Cuit>{}</ar:Cuit></ar:Auth>",
            self.token, self.sign, self.cuit
        )
    }
}

/// Today's date in Argentina (UTC−03:00) as `YYYYMMDD`.
fn today_yyyymmdd() -> u32 {
    let offset = FixedOffset::west_opt(3 * 3600).expect("valid offset");
    Utc::now()
        .with_timezone(&offset)
        .format("%Y%m%d")
        .to_string()
        .parse()
        .expect("date is numeric")
}

fn post(http: &reqwest::blocking::Client, url: &str, action: &str, body: String) -> Result<String> {
    let resp = http
        .post(url)
        .header("Content-Type", "text/xml; charset=utf-8")
        .header("SOAPAction", format!("{NS}{action}"))
        .body(body)
        .send()?
        .text()?;
    xml::check_soap_fault(&resp, "WSFEv1")?;
    Ok(resp)
}

/// Collect `<Errors><Err><Code/><Msg/>` records into a readable string.
fn collect_errors(xml_str: &str) -> Option<String> {
    let errs = xml::records(xml_str, "Err");
    if errs.is_empty() {
        return None;
    }
    let joined = errs
        .iter()
        .map(|fields| {
            let get = |k: &str| {
                fields
                    .iter()
                    .find(|(n, _)| n == k)
                    .map(|(_, v)| v.as_str())
                    .unwrap_or("")
            };
            format!("[{}] {}", get("Code"), get("Msg"))
        })
        .collect::<Vec<_>>()
        .join("; ");
    Some(joined)
}

/// `FECompUltimoAutorizado` — last authorized voucher number for the given
/// punto de venta / voucher type. Returns 0 when none exist yet.
pub fn last_voucher(
    http: &reqwest::blocking::Client,
    url: &str,
    creds: &Credentials,
    cuit: u64,
    punto_venta: u32,
    tipo: VoucherType,
) -> Result<u64> {
    let auth = Auth {
        token: &creds.token,
        sign: &creds.sign,
        cuit,
    };
    let body = format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<soapenv:Envelope xmlns:soapenv="http://schemas.xmlsoap.org/soap/envelope/" xmlns:ar="{NS}">
  <soapenv:Header/>
  <soapenv:Body>
    <ar:FECompUltimoAutorizado>
      {auth}
      <ar:PtoVta>{pv}</ar:PtoVta>
      <ar:CbteTipo>{tipo}</ar:CbteTipo>
    </ar:FECompUltimoAutorizado>
  </soapenv:Body>
</soapenv:Envelope>"#,
        auth = auth.xml(),
        pv = punto_venta,
        tipo = tipo.code(),
    );

    let resp = post(http, url, "FECompUltimoAutorizado", body)?;

    if let Some(errs) = collect_errors(&resp) {
        return Err(Error::Wsfe(format!("FECompUltimoAutorizado: {errs}")));
    }

    let nro = xml::first_text(&resp, "CbteNro").ok_or(Error::MissingField("CbteNro"))?;
    nro.parse::<u64>()
        .map_err(|_| Error::MissingField("CbteNro"))
}

/// `FECompConsultar` — fetch the details of a single already-issued voucher.
pub fn get_voucher(
    http: &reqwest::blocking::Client,
    url: &str,
    creds: &Credentials,
    cuit: u64,
    punto_venta: u32,
    tipo: VoucherType,
    numero: u64,
) -> Result<VoucherInfo> {
    let auth = Auth {
        token: &creds.token,
        sign: &creds.sign,
        cuit,
    };
    let body = format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<soapenv:Envelope xmlns:soapenv="http://schemas.xmlsoap.org/soap/envelope/" xmlns:ar="{NS}">
  <soapenv:Header/>
  <soapenv:Body>
    <ar:FECompConsultar>
      {auth}
      <ar:FeCompConsReq>
        <ar:CbteTipo>{tipo}</ar:CbteTipo>
        <ar:CbteNro>{numero}</ar:CbteNro>
        <ar:PtoVta>{pv}</ar:PtoVta>
      </ar:FeCompConsReq>
    </ar:FECompConsultar>
  </soapenv:Body>
</soapenv:Envelope>"#,
        auth = auth.xml(),
        pv = punto_venta,
        tipo = tipo.code(),
    );

    let resp = post(http, url, "FECompConsultar", body)?;

    if let Some(errs) = collect_errors(&resp) {
        return Err(Error::Wsfe(format!("FECompConsultar #{numero}: {errs}")));
    }

    let field = |k: &str| xml::first_text(&resp, k);
    Ok(VoucherInfo {
        numero,
        tipo: tipo.code(),
        punto_venta,
        fecha: field("CbteFch").and_then(|s| s.parse().ok()).unwrap_or(0),
        importe_total: field("ImpTotal").and_then(|s| s.parse().ok()).unwrap_or(0.0),
        doc_tipo: field("DocTipo").and_then(|s| s.parse().ok()).unwrap_or(0),
        doc_nro: field("DocNro").and_then(|s| s.parse().ok()).unwrap_or(0),
        cae: field("CodAutorizacion").filter(|s| !s.is_empty()),
        cae_vencimiento: field("FchVto").filter(|s| !s.is_empty()),
        resultado: field("Resultado").unwrap_or_default(),
    })
}

/// `FECAESolicitar` — request a CAE for a single Factura C voucher numbered
/// `numero`.
pub fn create_factura_c(
    http: &reqwest::blocking::Client,
    url: &str,
    creds: &Credentials,
    cuit: u64,
    punto_venta: u32,
    numero: u64,
    factura: &FacturaC,
) -> Result<CaeResult> {
    let auth = Auth {
        token: &creds.token,
        sign: &creds.sign,
        cuit,
    };
    let fecha = factura.fecha.unwrap_or_else(today_yyyymmdd);

    // Monotributo / Factura C: no discriminated IVA, net equals total.
    let total = format!("{:.2}", factura.importe_total);

    // Service concepts require the period + payment-due dates.
    let service_dates = if factura.concepto.requires_service_dates() {
        let desde = factura.fecha_servicio_desde.unwrap_or(fecha);
        let hasta = factura.fecha_servicio_hasta.unwrap_or(fecha);
        let vto = factura.fecha_vto_pago.unwrap_or(fecha);
        format!(
            "<ar:FchServDesde>{desde}</ar:FchServDesde><ar:FchServHasta>{hasta}</ar:FchServHasta><ar:FchVtoPago>{vto}</ar:FchVtoPago>"
        )
    } else {
        String::new()
    };

    let body = format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<soapenv:Envelope xmlns:soapenv="http://schemas.xmlsoap.org/soap/envelope/" xmlns:ar="{NS}">
  <soapenv:Header/>
  <soapenv:Body>
    <ar:FECAESolicitar>
      {auth}
      <ar:FeCAEReq>
        <ar:FeCabReq>
          <ar:CantReg>1</ar:CantReg>
          <ar:PtoVta>{pv}</ar:PtoVta>
          <ar:CbteTipo>{tipo}</ar:CbteTipo>
        </ar:FeCabReq>
        <ar:FeDetReq>
          <ar:FECAEDetRequest>
            <ar:Concepto>{concepto}</ar:Concepto>
            <ar:DocTipo>{doc_tipo}</ar:DocTipo>
            <ar:DocNro>{doc_nro}</ar:DocNro>
            <ar:CbteDesde>{numero}</ar:CbteDesde>
            <ar:CbteHasta>{numero}</ar:CbteHasta>
            <ar:CbteFch>{fecha}</ar:CbteFch>
            <ar:ImpTotal>{total}</ar:ImpTotal>
            <ar:ImpTotConc>0.00</ar:ImpTotConc>
            <ar:ImpNeto>{total}</ar:ImpNeto>
            <ar:ImpOpEx>0.00</ar:ImpOpEx>
            <ar:ImpIVA>0.00</ar:ImpIVA>
            <ar:ImpTrib>0.00</ar:ImpTrib>
            {service_dates}
            <ar:MonId>PES</ar:MonId>
            <ar:MonCotiz>1</ar:MonCotiz>
            <ar:CondicionIVAReceptorId>{cond_iva}</ar:CondicionIVAReceptorId>
          </ar:FECAEDetRequest>
        </ar:FeDetReq>
      </ar:FeCAEReq>
    </ar:FECAESolicitar>
  </soapenv:Body>
</soapenv:Envelope>"#,
        auth = auth.xml(),
        pv = punto_venta,
        tipo = VoucherType::FacturaC.code(),
        concepto = factura.concepto.code(),
        doc_tipo = factura.doc_tipo.code(),
        doc_nro = factura.doc_nro,
        cond_iva = factura.condicion_iva_receptor,
    );

    let resp = post(http, url, "FECAESolicitar", body)?;

    // Global request errors take precedence.
    if let Some(errs) = collect_errors(&resp) {
        return Err(Error::WsfeRejected(errs));
    }

    // Per-voucher observations (non-fatal, but surfaced).
    let observaciones: Vec<String> = xml::records(&resp, "Obs")
        .iter()
        .map(|f| {
            let get = |k: &str| {
                f.iter()
                    .find(|(n, _)| n == k)
                    .map(|(_, v)| v.as_str())
                    .unwrap_or("")
            };
            format!("[{}] {}", get("Code"), get("Msg"))
        })
        .collect();

    let resultado = xml::first_text(&resp, "Resultado").unwrap_or_default();
    if resultado != "A" {
        let detail = if observaciones.is_empty() {
            format!("Resultado={resultado}")
        } else {
            observaciones.join("; ")
        };
        return Err(Error::WsfeRejected(detail));
    }

    let cae = xml::first_text(&resp, "CAE").ok_or(Error::MissingField("CAE"))?;
    let cae_vencimiento =
        xml::first_text(&resp, "CAEFchVto").ok_or(Error::MissingField("CAEFchVto"))?;

    Ok(CaeResult {
        cae,
        cae_vencimiento,
        numero,
        punto_venta,
        tipo: VoucherType::FacturaC.code(),
        fecha,
        importe_total: factura.importe_total,
        observaciones,
    })
}

/// `FEDummy` — service health check (no auth required).
pub fn dummy(http: &reqwest::blocking::Client, url: &str) -> Result<(String, String, String)> {
    let body = format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<soapenv:Envelope xmlns:soapenv="http://schemas.xmlsoap.org/soap/envelope/" xmlns:ar="{NS}">
  <soapenv:Header/>
  <soapenv:Body><ar:FEDummy/></soapenv:Body>
</soapenv:Envelope>"#
    );
    let resp = post(http, url, "FEDummy", body)?;
    let app = xml::first_text(&resp, "AppServer").unwrap_or_else(|| "?".into());
    let db = xml::first_text(&resp, "DbServer").unwrap_or_else(|| "?".into());
    let auth = xml::first_text(&resp, "AuthServer").unwrap_or_else(|| "?".into());
    Ok((app, db, auth))
}
