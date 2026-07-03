use std::path::PathBuf;

use clap::{Parser, Subcommand};

use commands::afip::{self, CondicionIvaArg};

mod afip_invoice;
mod commands;
mod formats;
mod overrides;
mod schema;
mod typst;
mod world;

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
    /// Generate an invoice PDF from a TOML config file.
    ///
    /// For `afip_c` invoices without a CAE, this authorizes against AFIP
    /// (WSFE → CAE) before rendering; if a CAE is already present it just
    /// re-renders, so it never issues a duplicate comprobante.
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

    /// Show the schema for an invoice format.
    Schema {
        /// Format name (generic, afip_c, afip_a) or "list" to show all
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
    /// Create the AFIP issuer config (emisor_config.json).
    Configure {
        #[arg(long)]
        cuit: u64,
        #[arg(long)]
        punto_venta: u32,
        #[arg(long)]
        razon_social: String,
        #[arg(long, value_enum, default_value_t = CondicionIvaArg::Monotributo)]
        condicion_iva: CondicionIvaArg,
        /// Target the real production environment (default: homologación/testing).
        #[arg(long)]
        production: bool,
    },
    /// Generate the private key + CSR to upload to the ARCA portal.
    GenerateCertificate {
        /// Certificate alias / common name.
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
    let cli = Cli::parse();
    let home = afip::resolve_home(cli.home);

    match cli.command {
        Commands::Generate {
            config,
            template,
            output,
            overrides,
        } => commands::generate(&home, config, template, output, overrides),

        Commands::Schema { format } => commands::schema(&format),

        Commands::Afip { command } => match command {
            AfipCommand::Configure {
                cuit,
                punto_venta,
                razon_social,
                condicion_iva,
                production,
            } => afip::configure(
                &home,
                cuit,
                punto_venta,
                &razon_social,
                condicion_iva.into(),
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
        },
    }
}
