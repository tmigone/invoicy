use std::path::PathBuf;

use toml::Value;

use crate::formats::InvoiceConfig;
use crate::overrides;
use crate::world;

pub fn generate(
    config_path: PathBuf,
    template: Option<PathBuf>,
    output: Option<PathBuf>,
    override_args: Vec<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Read and parse config as TOML Value first
    let config_content = std::fs::read_to_string(&config_path)?;
    let mut config_value: Value = toml::from_str(&config_content)?;

    // Apply overrides
    for override_str in &override_args {
        overrides::apply(&mut config_value, override_str)?;
    }

    // Deserialize to InvoiceConfig
    let config: InvoiceConfig = config_value.try_into()?;

    // Get template (from file or built-in)
    let template_content = match &template {
        Some(path) => std::fs::read_to_string(path)?,
        None => config.default_template().to_string(),
    };

    // Generate data definition + template
    let full_source = format!("{}\n{}", config.to_typst_dict(), template_content);

    // Determine output path (auto-increment if exists)
    let base_path =
        output.unwrap_or_else(|| PathBuf::from(format!("invoice-{}.pdf", config.invoice_number())));
    let output_path = unique_path(base_path);

    // Compile to PDF
    let pdf_bytes = world::compile_to_pdf(&full_source)?;

    // Write output
    std::fs::write(&output_path, pdf_bytes)?;
    println!("Generated: {}", output_path.display());

    Ok(())
}

fn unique_path(path: PathBuf) -> PathBuf {
    if !path.exists() {
        return path;
    }

    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("invoice");
    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("pdf");
    let parent = path.parent().unwrap_or(std::path::Path::new("."));

    let mut n = 2;
    loop {
        let new_path = parent.join(format!("{}_{}.{}", stem, n, ext));
        if !new_path.exists() {
            return new_path;
        }
        n += 1;
    }
}
