use std::path::PathBuf;

use clap::Parser;

mod formats;
mod world;

use formats::InvoiceConfig;

/// Generate PDF invoices from TOML config and Typst templates
#[derive(Parser, Debug)]
#[command(name = "invoicy", version, about)]
struct Args {
    /// Path to the invoice config file (TOML)
    #[arg(short, long)]
    config: PathBuf,

    /// Path to the Typst template file (optional, uses built-in template based on format)
    #[arg(short, long)]
    template: Option<PathBuf>,

    /// Output PDF path (defaults to invoice-{number}.pdf)
    #[arg(short, long)]
    output: Option<PathBuf>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Read and parse config
    let config_content = std::fs::read_to_string(&args.config)?;
    let config: InvoiceConfig = toml::from_str(&config_content)?;

    // Get template (from file or built-in)
    let template_content = match &args.template {
        Some(path) => std::fs::read_to_string(path)?,
        None => config.default_template().to_string(),
    };

    // Generate data definition + template
    let full_source = format!("{}\n{}", config.to_typst_dict(), template_content);

    // Determine output path (auto-increment if exists)
    let base_path = args
        .output
        .unwrap_or_else(|| PathBuf::from(format!("invoice-{}.pdf", config.invoice_number())));
    let output_path = unique_path(base_path);

    // Compile to PDF (assets loaded from ./assets relative to cwd)
    let pdf_bytes = world::compile_to_pdf(&full_source, PathBuf::from("assets"))?;

    // Write output
    std::fs::write(&output_path, pdf_bytes)?;
    println!("Generated: {}", output_path.display());

    Ok(())
}

fn unique_path(path: PathBuf) -> PathBuf {
    if !path.exists() {
        return path;
    }

    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("invoice");
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