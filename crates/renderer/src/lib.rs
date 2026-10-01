//! Invoice rendering for invoicy.
//!
//! Owns the built-in Typst templates and the in-memory Typst world that
//! compiles them to PDF. The invoice model itself lives in the `schema` crate.
//! It does no network or config-file I/O: callers hand it a parsed invoice and
//! get PDF bytes back.
//!
//! The invoice reaches the template as JSON (see [`data`]), loaded into the
//! `invoice-data` variable ahead of the template source.

mod data;
mod qr;
mod world;

use schema::InvoiceConfig;

/// Virtual file the invoice JSON is served under inside the Typst world.
const DATA_FILE: &str = "data.json";
/// Virtual file for the invoice's QR code, when it has one.
const QR_FILE: &str = "qr.svg";

/// Render `config` to PDF bytes, using `template` (Typst source) if given or
/// the format's built-in template otherwise.
pub fn render(config: &InvoiceConfig, template: Option<&str>) -> Result<Vec<u8>, String> {
    let template = template.unwrap_or_else(|| default_template(config));
    let mut files = vec![(DATA_FILE, data::to_json(config)?)];
    if let InvoiceConfig::AfipC(inv) = config
        && !inv.qr.is_empty()
    {
        files.push((QR_FILE, qr::svg(&inv.qr)?));
    }
    let source = format!("#let invoice-data = json(\"{DATA_FILE}\")\n{template}");
    world::compile_to_pdf(&source, files)
}

fn default_template(config: &InvoiceConfig) -> &'static str {
    match config {
        InvoiceConfig::Generic(_) => include_str!("../templates/generic.typ"),
        InvoiceConfig::AfipC(_) => include_str!("../templates/afip_c.typ"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn example(name: &str) -> InvoiceConfig {
        let path = format!("{}/../../examples/{name}.toml", env!("CARGO_MANIFEST_DIR"));
        let mut value: toml::Value =
            toml::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        // Drafts may name their `home` / `output`; like `invoicy generate`,
        // drop them before parsing the invoice.
        let root = value.as_table_mut().unwrap();
        root.remove("home");
        root.remove("output");
        let mut config: InvoiceConfig = value.try_into().unwrap();
        if let InvoiceConfig::AfipC(inv) = &mut config {
            fake_authorization(inv);
        }
        config
    }

    /// The afip_c examples are drafts. Fill in what invoicy adds when
    /// authorizing: the emisor profile, punto de venta, number, date, CAE and
    /// the computed fields.
    fn fake_authorization(inv: &mut schema::AfipCInvoice) {
        inv.emisor = schema::afip_c::Emisor {
            razon_social: "Juan Pérez".into(),
            domicilio_comercial: "Av. Corrientes 1234 - CABA".into(),
            condicion_iva: "Responsable Monotributo".into(),
            cuit: "20123456789".into(),
            ingresos_brutos: "12345".into(),
            inicio_actividades: "01/01/2020".into(),
        };
        inv.comprobante.punto_de_venta = "00001".into();
        inv.comprobante.numero = "00000001".into();
        inv.comprobante.fecha_emision = "29/09/2026".into();
        inv.cae = schema::afip_c::Cae {
            numero: "12345678901234".into(),
            vencimiento: "09/10/2026".into(),
        };
        inv.compute();
    }

    fn is_pdf(bytes: &[u8]) -> bool {
        bytes.starts_with(b"%PDF-")
    }

    #[test]
    fn renders_every_example() {
        for name in ["generic", "consumidor_final", "responsable_inscripto"] {
            let pdf = render(&example(name), None).unwrap_or_else(|e| panic!("{name}: {e}"));
            assert!(is_pdf(&pdf), "{name} did not produce a PDF");
        }
    }

    #[test]
    fn renders_strings_that_need_escaping() {
        let InvoiceConfig::Generic(mut inv) = example("generic") else {
            unreachable!()
        };
        inv.company.name = "Acme \"Best\" \\ Corp\nsecond line #[not markup]".into();
        inv.items[0].description = "$math$ *bold* <label> @ref".into();
        let pdf = render(&InvoiceConfig::Generic(inv), None).unwrap();
        assert!(is_pdf(&pdf));
    }

    #[test]
    fn json_carries_toml_field_names_and_totals() {
        let json: serde_json::Value =
            serde_json::from_slice(&data::to_json(&example("consumidor_final")).unwrap()).unwrap();
        assert_eq!(json["emisor"]["razon_social"], "Juan Pérez");
        assert_eq!(json["items"][0]["bonificacion_porcentaje"], 0.0);
        assert_eq!(json["items"][1]["subtotal"], 50000.0);
        assert_eq!(json["subtotal"], 100000.0);
        assert_eq!(json["total"], 100000.0);
    }

    #[test]
    fn json_prints_receptor_labels_not_codes() {
        let receptor = |config: &InvoiceConfig| {
            let json: serde_json::Value =
                serde_json::from_slice(&data::to_json(config).unwrap()).unwrap();
            json["receptor"].clone()
        };

        let config = example("consumidor_final");
        let r = receptor(&config);
        assert_eq!(r["condicion_iva"], "Consumidor Final");
        assert_eq!(r["doc_tipo"], "CUIT");
        assert_eq!(r["doc_nro"], serde_json::Value::Null);
        let json: serde_json::Value =
            serde_json::from_slice(&data::to_json(&config).unwrap()).unwrap();
        assert_eq!(json["comprobante"]["tipo"], "C");
        assert_eq!(json["comprobante"]["codigo"], "011");

        let InvoiceConfig::AfipC(mut inv) = config else {
            unreachable!()
        };
        inv.receptor.condicion_iva = afip::CondicionIva::ResponsableInscripto;
        inv.receptor.doc_tipo = afip::DocTipo::Dni;
        inv.receptor.doc_nro = 12345678;
        let config = InvoiceConfig::AfipC(inv);
        let r = receptor(&config);
        assert_eq!(r["condicion_iva"], "IVA Responsable Inscripto");
        assert_eq!(r["doc_tipo"], "DNI");
        assert_eq!(r["doc_nro"], "12345678");
        assert!(is_pdf(&render(&config, None).unwrap()));
    }

    #[test]
    fn renders_the_qr_once_authorized_and_a_blank_box_before() {
        let config = example("consumidor_final");
        let InvoiceConfig::AfipC(inv) = &config else {
            unreachable!()
        };
        assert!(inv.qr.starts_with("https://www.arca.gob.ar/fe/qr/?p="));
        let svg = qr::svg(&inv.qr).unwrap();
        assert!(String::from_utf8(svg).unwrap().starts_with("<?xml"));
        assert!(is_pdf(&render(&config, None).unwrap()));

        let InvoiceConfig::AfipC(mut inv) = config else {
            unreachable!()
        };
        inv.qr.clear();
        assert!(is_pdf(&render(&InvoiceConfig::AfipC(inv), None).unwrap()));
    }
}
