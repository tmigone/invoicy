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
mod world;

use schema::InvoiceConfig;

/// Virtual file the invoice JSON is served under inside the Typst world.
const DATA_FILE: &str = "data.json";

/// Render `config` to PDF bytes, using `template` (Typst source) if given or
/// the format's built-in template otherwise.
pub fn render(config: &InvoiceConfig, template: Option<&str>) -> Result<Vec<u8>, String> {
    let template = template.unwrap_or_else(|| default_template(config));
    let data = data::to_json(config)?;
    let source = format!("#let invoice-data = json(\"{DATA_FILE}\")\n{template}");
    world::compile_to_pdf(&source, data)
}

fn default_template(config: &InvoiceConfig) -> &'static str {
    match config {
        InvoiceConfig::Generic(_) => include_str!("../templates/generic.typ"),
        InvoiceConfig::AfipC(_) => include_str!("../templates/afip_c.typ"),
        InvoiceConfig::AfipA(_) => include_str!("../templates/afip_a.typ"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn example(name: &str) -> InvoiceConfig {
        let path = format!("{}/../../examples/{name}.toml", env!("CARGO_MANIFEST_DIR"));
        toml::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
    }

    fn is_pdf(bytes: &[u8]) -> bool {
        bytes.starts_with(b"%PDF-")
    }

    #[test]
    fn renders_every_example() {
        for name in ["generic", "afip_c", "afip_a"] {
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
            serde_json::from_slice(&data::to_json(&example("afip_c")).unwrap()).unwrap();
        assert_eq!(json["emisor"]["razon_social"], "Juan Pérez");
        assert_eq!(json["items"][0]["bonificacion_porcentaje"], 0.0);
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

        let config = example("afip_c");
        let r = receptor(&config);
        assert_eq!(r["condicion_iva"], "Consumidor Final");
        assert_eq!(r["doc_tipo"], "CUIT");
        assert_eq!(r["doc_nro"], serde_json::Value::Null);

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
    fn json_afip_a_defaults_otros_tributos_to_empty() {
        let mut value: toml::Value =
            toml::from_str(include_str!("../../../examples/afip_a.toml")).unwrap();
        value.as_table_mut().unwrap().remove("otros_tributos");
        let config: InvoiceConfig = value.try_into().unwrap();
        let json: serde_json::Value =
            serde_json::from_slice(&data::to_json(&config).unwrap()).unwrap();
        assert_eq!(json["otros_tributos"], serde_json::json!([]));
        assert!(is_pdf(&render(&config, None).unwrap()));
    }
}
