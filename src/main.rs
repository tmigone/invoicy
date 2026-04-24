use std::path::PathBuf;

use clap::{Parser, Subcommand};

mod commands;
mod formats;
mod overrides;
mod schema;
mod typst;
mod world;

/// Generate PDF invoices from TOML config and Typst templates
#[derive(Parser, Debug)]
#[command(name = "invoicy", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Generate a PDF invoice from a TOML config file
    #[command(alias = "gen")]
    Generate {
        /// Path to the invoice config file (TOML)
        #[arg(short, long)]
        config: PathBuf,

        /// Path to a custom Typst template file
        #[arg(short, long)]
        template: Option<PathBuf>,

        /// Output PDF path (defaults to invoice-{number}.pdf)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Override config values (e.g., --set comprobante.numero=00000153)
        #[arg(short = 's', long = "set", value_name = "KEY=VALUE")]
        overrides: Vec<String>,
    },

    /// Show the schema for an invoice format
    Schema {
        /// Format name (generic, afip_c, afip_a) or "list" to show all
        format: String,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Generate {
            config,
            template,
            output,
            overrides,
        } => commands::generate(config, template, output, overrides),

        Commands::Schema { format } => commands::schema(&format),
    }
}
