use std::path::{Path, PathBuf};

use schema::InvoiceConfig;
use toml::Value;

use crate::afip_invoice;
use crate::emisor::EmisorProfile;
use crate::overrides;

/// Generate an invoice into `output_dir`: `<name>.toml` with every field of
/// the invoice (including the ones filled in automatically) and `<name>.pdf`.
pub fn generate(
    home: &Path,
    config_path: PathBuf,
    template: Option<PathBuf>,
    output_dir: PathBuf,
    override_args: Vec<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Read and parse config as TOML Value first
    let config_content = std::fs::read_to_string(&config_path)?;
    let mut config_value: Value = toml::from_str(&config_content)?;

    // Extract format for schema-aware overrides
    let format = config_value
        .get("format")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    // Apply overrides
    for override_str in &override_args {
        overrides::apply(&mut config_value, override_str, format.as_deref())?;
    }

    let mut config: InvoiceConfig = config_value.try_into()?;

    // Everything that can fail locally happens before AFIP issues anything.
    let template_content = template.map(std::fs::read_to_string).transpose()?;
    std::fs::create_dir_all(&output_dir)?;

    // afip_c invoices are drafts: authorizing fills in the emisor, número,
    // fecha and CAE. Each run issues a new comprobante.
    if let InvoiceConfig::AfipC(inv) = &mut config {
        let profile = EmisorProfile::load(home)?;
        afip_invoice::authorize(&profile, home, inv)?;
    }

    let stem = unique_stem(&output_dir, &format!("invoice-{}", config.invoice_number()));

    // The TOML is the invoice's record: write it before rendering so a
    // failed render never loses an issued CAE.
    let toml_path = output_dir.join(format!("{stem}.toml"));
    std::fs::write(&toml_path, toml::to_string_pretty(&config)?)?;
    println!("Generated: {}", toml_path.display());

    let pdf_bytes = renderer::render(&config, template_content.as_deref())?;
    let pdf_path = output_dir.join(format!("{stem}.pdf"));
    std::fs::write(&pdf_path, pdf_bytes)?;
    println!("Generated: {}", pdf_path.display());

    Ok(())
}

/// `name`, or `name_2`, `name_3`, … — the first for which neither the `.toml`
/// nor the `.pdf` exists in `dir`.
fn unique_stem(dir: &Path, name: &str) -> String {
    let taken = |stem: &str| {
        dir.join(format!("{stem}.toml")).exists() || dir.join(format!("{stem}.pdf")).exists()
    };
    if !taken(name) {
        return name.to_string();
    }
    (2..)
        .map(|n| format!("{name}_{n}"))
        .find(|stem| !taken(stem))
        .expect("an unused name exists")
}
