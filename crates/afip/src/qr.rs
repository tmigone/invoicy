//! The QR code ARCA requires on every electronic invoice (RG 4892/2020).
//!
//! The QR encodes `{QR_URL}?p={base64(json)}`, where the JSON describes the
//! authorized voucher (version 1 of the spec). Scanning it opens ARCA's
//! verification page. Spec:
//! <https://www.afip.gob.ar/fe/qr/documentos/QRespecificaciones.pdf>

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;

/// Base URL of the QR text, as given by the spec.
pub const QR_URL: &str = "https://www.arca.gob.ar/fe/qr/";

/// The voucher data a QR carries.
#[derive(Debug, Clone, PartialEq)]
pub struct QrData<'a> {
    /// Issue date, `YYYY-MM-DD`.
    pub fecha: &'a str,
    /// Issuer CUIT.
    pub cuit: u64,
    pub punto_venta: u32,
    /// Voucher type code (`CbteTipo`, e.g. 11 for Factura C).
    pub tipo: u16,
    pub numero: u64,
    /// Total, in `moneda`.
    pub importe: f64,
    /// Currency code (`PES` for pesos).
    pub moneda: &'a str,
    /// Exchange rate to pesos (1 for pesos).
    pub cotizacion: f64,
    /// Receptor document as (type code, number), when the receptor is
    /// identified; omitted for an anonymous consumidor final.
    pub receptor: Option<(u8, u64)>,
    /// The CAE.
    pub cae: u64,
}

impl QrData<'_> {
    /// The JSON payload, with the fields in the spec's order. Numbers use
    /// their shortest form (`12100`, `12100.5`), as in the spec's example.
    pub fn json(&self) -> String {
        let receptor = self
            .receptor
            .map(|(tipo, nro)| format!(r#","tipoDocRec":{tipo},"nroDocRec":{nro}"#))
            .unwrap_or_default();
        format!(
            r#"{{"ver":1,"fecha":"{fecha}","cuit":{cuit},"ptoVta":{pv},"tipoCmp":{tipo},"nroCmp":{nro},"importe":{importe},"moneda":"{moneda}","ctz":{ctz}{receptor},"tipoCodAut":"E","codAut":{cae}}}"#,
            fecha = self.fecha,
            cuit = self.cuit,
            pv = self.punto_venta,
            tipo = self.tipo,
            nro = self.numero,
            importe = self.importe,
            moneda = self.moneda,
            ctz = self.cotizacion,
            // "E": authorized by CAE (vs. "A" for CAEA, which this crate doesn't use).
            cae = self.cae,
        )
    }

    /// The text to encode in the QR.
    pub fn url(&self) -> String {
        format!("{QR_URL}?p={}", BASE64.encode(self.json()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The example from the spec, which prints both the JSON and its encoding.
    fn spec_example() -> QrData<'static> {
        QrData {
            fecha: "2020-10-13",
            cuit: 30000000007,
            punto_venta: 10,
            tipo: 1,
            numero: 94,
            importe: 12100.0,
            moneda: "DOL",
            cotizacion: 65.0,
            receptor: Some((80, 20000000001)),
            cae: 70417054367476,
        }
    }

    #[test]
    fn matches_the_spec_example() {
        let qr = spec_example();
        assert_eq!(
            qr.json(),
            r#"{"ver":1,"fecha":"2020-10-13","cuit":30000000007,"ptoVta":10,"tipoCmp":1,"nroCmp":94,"importe":12100,"moneda":"DOL","ctz":65,"tipoDocRec":80,"nroDocRec":20000000001,"tipoCodAut":"E","codAut":70417054367476}"#
        );
        assert_eq!(
            qr.url(),
            "https://www.arca.gob.ar/fe/qr/?p=eyJ2ZXIiOjEsImZlY2hhIjoiMjAyMC0xMC0xMyIsImN1aXQiOjMwMDAwMDAwMDA3LCJwdG9WdGEiOjEwLCJ0aXBvQ21wIjoxLCJucm9DbXAiOjk0LCJpbXBvcnRlIjoxMjEwMCwibW9uZWRhIjoiRE9MIiwiY3R6Ijo2NSwidGlwb0RvY1JlYyI6ODAsIm5yb0RvY1JlYyI6MjAwMDAwMDAwMDEsInRpcG9Db2RBdXQiOiJFIiwiY29kQXV0Ijo3MDQxNzA1NDM2NzQ3Nn0="
        );
    }

    #[test]
    fn omits_the_receptor_when_anonymous_and_keeps_cents() {
        let qr = QrData {
            receptor: None,
            importe: 1500.5,
            ..spec_example()
        };
        let json = qr.json();
        assert!(!json.contains("DocRec"), "{json}");
        assert!(json.contains(r#""importe":1500.5,"#), "{json}");
    }
}
