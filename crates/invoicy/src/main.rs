use std::path::PathBuf;

use clap::{Parser, Subcommand};

use commands::afip;

mod afip_invoice;
mod commands;
mod emisor;
mod overrides;

/// Generate PDF invoices from TOML config, and issue Argentine electronic
/// invoices (Factura C) against ARCA/AFIP.
#[derive(Parser, Debug)]
#[command(name = "invoicy", version, about)]
struct Cli {
    /// Working directory for AFIP config, certs and the credential cache.
    /// Defaults to $AFIP_HOME, then ~/invoicy.
    #[arg(long, global = true)]
    home: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Generate an invoice from a TOML config file.
    ///
    /// Writes `<output>/invoice-<number>.pdf` and `.toml`; the TOML holds every
    /// field of the invoice, including the ones filled in automatically. An
    /// `afip_c` invoice is authorized against AFIP (WSFE → CAE) first, so each
    /// run issues a new comprobante.
    #[command(alias = "gen")]
    Generate {
        /// Path to the invoice config file (TOML)
        #[arg(short, long)]
        config: PathBuf,

        /// Path to a custom Typst template file
        #[arg(short, long)]
        template: Option<PathBuf>,

        /// Directory for the generated PDF and TOML
        /// [default: output/<name of the home directory>, e.g. output/invoicy]
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Override config values (e.g., --set comprobante.periodo_desde=01/10/2026)
        #[arg(short = 's', long = "set", value_name = "KEY=VALUE")]
        overrides: Vec<String>,
    },

    /// Show the schema for an invoice format.
    Schema {
        /// Format name (generic, afip_c) or "list" to show all
        format: String,
    },

    /// AFIP/ARCA authorization utilities (certificate, status, vouchers).
    Afip {
        #[command(subcommand)]
        command: AfipCommand,
    },
}

#[derive(Subcommand, Debug)]
enum AfipCommand {
    /// Create the emisor profile (emisor.toml).
    Configure {
        #[arg(long)]
        cuit: u64,
        #[arg(long)]
        razon_social: String,
        #[arg(long)]
        punto_venta: u32,
        /// Issuer's condición frente al IVA (shown on the PDF).
        #[arg(long, default_value = "Responsable Monotributo")]
        condicion_iva: String,
        /// Commercial address (shown on the PDF).
        #[arg(long, default_value = "")]
        domicilio: String,
        /// Ingresos brutos (shown on the PDF).
        #[arg(long, default_value = "")]
        ingresos_brutos: String,
        /// Business start date, DD/MM/YYYY (shown on the PDF).
        #[arg(long, default_value = "")]
        inicio_actividades: String,
        /// Target the real production environment (default: homologación/testing).
        #[arg(long)]
        production: bool,
    },
    /// Generate the private key + CSR to upload to the ARCA portal.
    GenerateCertificate {
        /// Name of the certificate (the CSR's CN), shown in the ARCA portal.
        /// The key and CSR are always written to the profile's paths.
        #[arg(long, default_value = "invoicy")]
        alias: String,
        /// Overwrite an existing key/CSR.
        #[arg(long)]
        force: bool,
    },
    /// Check WSFE service health (FEDummy).
    Status,
    /// Print the last authorized Factura C number.
    LastVoucher,
    /// List issued Factura C vouchers (one WSFE query per voucher).
    ListVouchers {
        /// How many of the most recent to show (ignored if --from is set).
        #[arg(long, default_value_t = 10)]
        last: u64,
        /// First voucher number, inclusive.
        #[arg(long)]
        from: Option<u64>,
        /// Last voucher number, inclusive (default: last authorized).
        #[arg(long)]
        to: Option<u64>,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let Cli {
        home: cli_home,
        command,
    } = Cli::parse();

    match command {
        // The draft can name its home too, so `generate` resolves it itself.
        Commands::Generate {
            config,
            template,
            output,
            overrides,
        } => commands::generate(cli_home, config, template, output, overrides),

        Commands::Schema { format } => commands::schema(&format),

        Commands::Afip { command } => {
            let home = afip::resolve_home(cli_home);
            match command {
                AfipCommand::Configure {
                    cuit,
                    razon_social,
                    punto_venta,
                    condicion_iva,
                    domicilio,
                    ingresos_brutos,
                    inicio_actividades,
                    production,
                } => afip::configure(
                    &home,
                    cuit,
                    razon_social,
                    punto_venta,
                    condicion_iva,
                    domicilio,
                    ingresos_brutos,
                    inicio_actividades,
                    production,
                ),
                AfipCommand::GenerateCertificate { alias, force } => {
                    afip::generate_certificate(&home, &alias, force)
                }
                AfipCommand::Status => afip::status(&home),
                AfipCommand::LastVoucher => afip::last_voucher(&home),
                AfipCommand::ListVouchers { last, from, to } => {
                    afip::list_vouchers(&home, last, from, to)
                }
            }
        }
    }
}
