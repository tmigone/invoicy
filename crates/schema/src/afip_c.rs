//! Factura C.
//!
//! [`AfipCInvoice`] is the complete invoice: the fields you write in the TOML
//! plus the ones invoicy fills in, which are output-only and tagged in the
//! schema with where they come from (`x-source`):
//!
//! - `AFIP`: the issuer (from the AFIP profile, `emisor.toml`) and what WSFE
//!   assigns when it authorizes the invoice (número, fecha, CAE);
//! - `computed`: derived from the other fields ([`AfipCInvoice::compute`]).
//!
//! Writing an output-only field in the TOML is an error.

use afip::qr::QrData;
use afip::{Concepto, CondicionIva, DocTipo, VoucherType};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Factura C. Unknown keys are rejected: a misspelled or outdated receptor
/// field must not silently fall back to "consumidor final" when authorizing.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AfipCInvoice {
    #[serde(skip_deserializing)]
    #[schemars(extend("x-source" = "AFIP"))]
    pub emisor: Emisor,
    pub receptor: Receptor,
    pub comprobante: Comprobante,
    pub items: Vec<LineItem>,
    #[serde(skip_deserializing)]
    #[schemars(extend("x-source" = "computed"))]
    pub totales: Totales,
    #[serde(skip_deserializing)]
    #[schemars(extend("x-source" = "AFIP"))]
    pub cae: Cae,
    /// Text of the QR printed on the invoice (RG 4892/2020): ARCA's
    /// verification URL for this voucher. Empty until it has a CAE.
    #[serde(skip_deserializing)]
    #[schemars(extend("x-source" = "computed"))]
    pub qr: String,
}

#[derive(Debug, Default, Deserialize, Serialize, JsonSchema)]
pub struct Emisor {
    pub razon_social: String,
    pub domicilio_comercial: String,
    pub condicion_iva: String,
    pub cuit: String,
    pub ingresos_brutos: String,
    pub inicio_actividades: String,
}

/// The buyer. `condicion_iva`, `doc_tipo` and `doc_nro` are sent to AFIP as
/// codes and printed as their labels, so the PDF always matches the CAE.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Receptor {
    pub nombre: Option<String>,
    pub domicilio: Option<String>,
    #[serde(default)]
    pub condicion_iva: CondicionIva,
    pub condicion_venta: String,
    #[serde(default)]
    pub doc_tipo: DocTipo,
    /// Document number; 0 (the default) for an unidentified consumidor final.
    #[serde(default)]
    pub doc_nro: u64,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Comprobante {
    /// What is billed: products, services, or both. Services require the
    /// billing period and payment due date.
    pub concepto: Concepto,
    /// Letter in the header box, from [`AfipCInvoice::voucher_type`].
    #[serde(skip_deserializing)]
    #[schemars(extend("x-source" = "computed"))]
    pub tipo: String,
    /// AFIP voucher code printed under the letter, e.g. `011`.
    #[serde(skip_deserializing)]
    #[schemars(extend("x-source" = "computed"))]
    pub codigo: String,
    #[serde(skip_deserializing)]
    #[schemars(extend("x-source" = "AFIP"))]
    pub punto_de_venta: String,
    #[serde(skip_deserializing)]
    #[schemars(extend("x-source" = "AFIP"))]
    pub numero: String,
    #[serde(skip_deserializing)]
    #[schemars(extend("x-source" = "AFIP"))]
    pub fecha_emision: String,
    pub periodo_desde: Option<String>,
    pub periodo_hasta: Option<String>,
    pub fecha_vencimiento: String,
}

/// A line of the invoice. Discounts (bonificaciones) are not supported.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LineItem {
    pub codigo: String,
    pub descripcion: String,
    pub cantidad: f64,
    pub unidad: String,
    pub precio_unitario: f64,
    /// `cantidad × precio_unitario`, rounded to cents.
    #[serde(skip_deserializing)]
    #[schemars(extend("x-source" = "computed"))]
    pub subtotal: f64,
}

#[derive(Debug, Default, Deserialize, Serialize, JsonSchema)]
pub struct Totales {
    /// Sum of the item subtotals.
    pub subtotal: f64,
    pub otros_tributos: f64,
    /// The amount authorized by AFIP.
    pub total: f64,
}

#[derive(Debug, Default, Deserialize, Serialize, JsonSchema)]
pub struct Cae {
    pub numero: String,
    /// CAE expiry, DD/MM/YYYY.
    pub vencimiento: String,
}

impl AfipCInvoice {
    /// The voucher this format issues and prints.
    pub fn voucher_type(&self) -> VoucherType {
        VoucherType::FacturaC
    }

    /// Fill in the `computed` fields: header letter and code, item subtotals,
    /// totals and — once the invoice has a CAE — the QR. Amounts are rounded
    /// to cents because AFIP receives the total with two decimals, so the
    /// printed lines must add up to exactly that.
    pub fn compute(&mut self) {
        let voucher = self.voucher_type();
        self.comprobante.tipo = voucher.letra().to_string();
        self.comprobante.codigo = format!("{:03}", voucher.code());
        for item in &mut self.items {
            item.subtotal = round_cents(item.cantidad * item.precio_unitario);
        }
        let subtotal = round_cents(self.items.iter().map(|i| i.subtotal).sum());
        let otros_tributos = 0.0;
        self.totales = Totales {
            subtotal,
            otros_tributos,
            total: round_cents(subtotal + otros_tributos),
        };
        self.qr = self.qr_url().unwrap_or_default();
    }

    /// The QR text, or `None` until the invoice is authorized (no CAE yet).
    fn qr_url(&self) -> Option<String> {
        let comprobante = &self.comprobante;
        let receptor = &self.receptor;
        let fecha = ddmmyyyy_to_iso(&comprobante.fecha_emision)?;
        let qr = QrData {
            fecha: &fecha,
            cuit: self.emisor.cuit.parse().ok()?,
            punto_venta: comprobante.punto_de_venta.parse().ok()?,
            tipo: self.voucher_type().code(),
            numero: comprobante.numero.parse().ok()?,
            importe: self.totales.total,
            // invoicy only issues in pesos.
            moneda: "PES",
            cotizacion: 1.0,
            receptor: (receptor.doc_tipo != DocTipo::ConsumidorFinal)
                .then_some((receptor.doc_tipo.code(), receptor.doc_nro)),
            cae: self.cae.numero.parse().ok()?,
        };
        Some(qr.url())
    }
}

/// `DD/MM/YYYY` → `YYYY-MM-DD`.
fn ddmmyyyy_to_iso(date: &str) -> Option<String> {
    let mut parts = date.trim().split('/');
    let (d, m, y) = (parts.next()?, parts.next()?, parts.next()?);
    if parts.next().is_some() || d.len() != 2 || m.len() != 2 || y.len() != 4 {
        return None;
    }
    Some(format!("{y}-{m}-{d}"))
}

fn round_cents(amount: f64) -> f64 {
    (amount * 100.0).round() / 100.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::InvoiceConfig;

    /// A minimal afip_c draft (only user-written fields), splicing extra
    /// lines into `[receptor]` and `[comprobante]`.
    fn source(receptor: &str, comprobante: &str) -> String {
        format!(
            r#"
format = "afip_c"
[receptor]
condicion_venta = "Contado"
{receptor}
[comprobante]
concepto = "productos"
fecha_vencimiento = ""
{comprobante}
[[items]]
codigo = "1"
descripcion = "x"
cantidad = 1.0
unidad = "u"
precio_unitario = 1.0
"#
        )
    }

    fn parse(receptor: &str, comprobante: &str) -> Result<AfipCInvoice, toml::de::Error> {
        match toml::from_str::<InvoiceConfig>(&source(receptor, comprobante))? {
            InvoiceConfig::AfipC(inv) => Ok(inv),
            _ => unreachable!(),
        }
    }

    fn item(cantidad: f64, precio_unitario: f64) -> LineItem {
        LineItem {
            codigo: "1".into(),
            descripcion: "x".into(),
            cantidad,
            unidad: "u".into(),
            precio_unitario,
            subtotal: 0.0,
        }
    }

    #[test]
    fn receptor_defaults_to_anonymous_consumidor_final() {
        let r = parse("", "").unwrap().receptor;
        assert_eq!(r.condicion_iva, CondicionIva::ConsumidorFinal);
        assert_eq!(r.doc_tipo, DocTipo::ConsumidorFinal);
        assert_eq!(r.doc_nro, 0);
    }

    #[test]
    fn receptor_takes_afip_codes() {
        let r = parse(
            "condicion_iva = \"responsable_inscripto\"\ndoc_tipo = \"cuit\"\ndoc_nro = 30712345678",
            "",
        )
        .unwrap()
        .receptor;
        assert_eq!(r.condicion_iva, CondicionIva::ResponsableInscripto);
        assert_eq!(r.doc_tipo, DocTipo::Cuit);
        assert_eq!(r.doc_nro, 30712345678);
    }

    #[test]
    fn concepto_is_required() {
        let without = source("", "").replace("concepto = \"productos\"\n", "");
        let err = toml::from_str::<InvoiceConfig>(&without).unwrap_err();
        assert!(err.to_string().contains("concepto"), "{err}");
    }

    #[test]
    fn unknown_fields_are_rejected() {
        // Pre-typed receptor and the old [afip] table must not be ignored.
        assert!(parse("documento = \"30712345678\"", "").is_err());
        assert!(parse("", "[afip]\ndoc_tipo = \"cuit\"").is_err());
        assert!(parse("doc_numero = 1", "").is_err());
    }

    #[test]
    fn automatic_fields_cannot_be_written() {
        for comprobante in [
            "tipo = \"C\"",
            "codigo = \"011\"",
            "punto_de_venta = \"00001\"",
            "numero = \"00000001\"",
            "fecha_emision = \"01/01/2025\"",
            "[emisor]\ncuit = \"20123456789\"",
            "[cae]\nnumero = \"1\"",
            "[totales]\ntotal = 1.0",
        ] {
            assert!(parse("", comprobante).is_err(), "{comprobante}");
        }
        let item = "codigo = \"1\"\ndescripcion = \"x\"\ncantidad = 1.0\nunidad = \"u\"\nprecio_unitario = 1.0";
        assert!(toml::from_str::<LineItem>(item).is_ok());
        for field in [
            "subtotal = 1.0",
            "bonificacion_porcentaje = 0.0",
            "bonificacion_importe = 0.0",
        ] {
            let src = format!("{item}\n{field}");
            assert!(toml::from_str::<LineItem>(&src).is_err(), "{field}");
        }
    }

    #[test]
    fn compute_fills_header_subtotals_and_totals() {
        let mut inv = parse("", "").unwrap();
        inv.items = vec![item(10.0, 5000.0), item(3.0, 0.1), item(1.5, 33.333)];
        inv.compute();
        assert_eq!(inv.comprobante.tipo, "C");
        assert_eq!(inv.comprobante.codigo, "011");
        let subtotals: Vec<f64> = inv.items.iter().map(|i| i.subtotal).collect();
        assert_eq!(subtotals, [50000.0, 0.3, 50.0]);
        assert_eq!(inv.totales.subtotal, 50050.3);
        assert_eq!(inv.totales.otros_tributos, 0.0);
        assert_eq!(inv.totales.total, 50050.3);
    }

    /// A draft with what authorization fills in.
    fn authorized(receptor: &str) -> AfipCInvoice {
        let mut inv = parse(receptor, "").unwrap();
        inv.emisor.cuit = "20123456789".into();
        inv.comprobante.punto_de_venta = "00003".into();
        inv.comprobante.numero = "00000042".into();
        inv.comprobante.fecha_emision = "29/09/2026".into();
        inv.cae.numero = "86139680699705".into();
        inv.items = vec![item(2.0, 750.25)];
        inv.compute();
        inv
    }

    fn qr_json(inv: &AfipCInvoice) -> String {
        use base64::Engine;
        let payload = inv
            .qr
            .strip_prefix("https://www.arca.gob.ar/fe/qr/?p=")
            .unwrap();
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(payload)
            .unwrap();
        String::from_utf8(bytes).unwrap()
    }

    #[test]
    fn qr_encodes_the_authorized_voucher() {
        let inv = authorized("doc_tipo = \"cuit\"\ndoc_nro = 30712345678");
        assert_eq!(
            qr_json(&inv),
            r#"{"ver":1,"fecha":"2026-09-29","cuit":20123456789,"ptoVta":3,"tipoCmp":11,"nroCmp":42,"importe":1500.5,"moneda":"PES","ctz":1,"tipoDocRec":80,"nroDocRec":30712345678,"tipoCodAut":"E","codAut":86139680699705}"#
        );
    }

    #[test]
    fn qr_omits_an_anonymous_receptor() {
        let json = qr_json(&authorized(""));
        assert!(!json.contains("DocRec"), "{json}");
    }

    #[test]
    fn qr_is_empty_before_authorization() {
        let mut inv = parse("", "").unwrap();
        inv.compute();
        assert_eq!(inv.qr, "");
    }

    #[test]
    fn output_includes_every_field() {
        let mut inv = parse("", "").unwrap();
        inv.emisor.cuit = "20123456789".into();
        inv.comprobante.numero = "00000042".into();
        inv.cae.numero = "86139680699705".into();
        inv.compute();
        let out = toml::to_string_pretty(&InvoiceConfig::AfipC(inv)).unwrap();
        for key in [
            "format = \"afip_c\"",
            "[emisor]",
            "cuit = \"20123456789\"",
            "concepto = \"productos\"",
            "tipo = \"C\"",
            "codigo = \"011\"",
            "numero = \"00000042\"",
            "subtotal = 1.0",
            "[totales]",
            "[cae]",
            "doc_tipo = \"consumidor_final\"",
            "doc_nro = 0",
            "qr = ",
        ] {
            assert!(out.contains(key), "missing {key} in:\n{out}");
        }
    }
}
