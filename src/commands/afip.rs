//! AFIP/ARCA web-service commands (authorization side).
//!
//! These drive the `afip` crate (WSAA + WSFEv1) and share an issuer config +
//! credential cache under a working directory (`--home`). They are exposed
//! under the `invoicy afip <command>` subcommand.

use std::path::{Path, PathBuf};

use afip::{Client, CondicionIva, EmisorConfig, Environment, VoucherType};
use clap::ValueEnum;

type BoxError = Box<dyn std::error::Error>;
type R = Result<(), BoxError>;

/// Issuer's condición frente al IVA (for `afip configure`).
#[derive(Copy, Clone, Debug, ValueEnum)]
pub enum CondicionIvaArg {
    Monotributo,
    ResponsableInscripto,
    Exento,
}

impl From<CondicionIvaArg> for CondicionIva {
    fn from(v: CondicionIvaArg) -> Self {
        match v {
            CondicionIvaArg::Monotributo => CondicionIva::Monotributo,
            CondicionIvaArg::ResponsableInscripto => CondicionIva::ResponsableInscripto,
            CondicionIvaArg::Exento => CondicionIva::Exento,
        }
    }
}

/// Resolve the working directory: `--home`, then `$AFIP_HOME`, then `~/invoicy`.
pub fn resolve_home(cli_home: Option<PathBuf>) -> PathBuf {
    if let Some(h) = cli_home {
        return h;
    }
    if let Ok(h) = std::env::var("AFIP_HOME") {
        return PathBuf::from(h);
    }
    let base = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    PathBuf::from(base).join("invoicy")
}

fn config_path(home: &Path) -> PathBuf {
    home.join("emisor_config.json")
}

pub fn load_client(home: &Path) -> Result<Client, BoxError> {
    let cfg = EmisorConfig::load(config_path(home)).map_err(|e| {
        format!(
            "no se pudo cargar emisor_config.json — ejecutá `invoicy afip configure` primero ({e})"
        )
    })?;
    Ok(Client::new(cfg, home.join("cache"))?)
}

pub fn configure(
    home: &Path,
    cuit: u64,
    punto_venta: u32,
    razon_social: &str,
    condicion_iva: CondicionIva,
    production: bool,
) -> R {
    std::fs::create_dir_all(home.join("certs"))?;
    let cfg = EmisorConfig {
        cuit,
        punto_venta,
        razon_social: razon_social.to_string(),
        condicion_iva,
        environment: if production {
            Environment::Produccion
        } else {
            Environment::Homologacion
        },
        cert_path: home.join("certs/invoicy.crt"),
        key_path: home.join("certs/invoicy.key"),
    };
    let path = config_path(home);
    cfg.save(&path)?;
    println!("✔ Configuración guardada en {}", path.display());
    println!(
        "  entorno: {}",
        if production { "producción" } else { "homologación" }
    );
    println!("  Siguiente: `invoicy afip generate-certificate`");
    Ok(())
}

pub fn generate_certificate(home: &Path, alias: &str, force: bool) -> R {
    let cfg = EmisorConfig::load(config_path(home))
        .map_err(|e| format!("ejecutá `invoicy afip configure` primero ({e})"))?;

    let certs_dir = home.join("certs");
    std::fs::create_dir_all(&certs_dir)?;
    let key_path = certs_dir.join(format!("{alias}.key"));
    let csr_path = certs_dir.join(format!("{alias}.csr"));

    if key_path.exists() && !force {
        return Err(format!(
            "{} ya existe — usá --force para sobrescribir (esto invalida el certificado emitido por ARCA)",
            key_path.display()
        )
        .into());
    }

    let out = afip::cert::generate_key_and_csr(cfg.cuit, &cfg.razon_social, alias)?;
    std::fs::write(&key_path, out.private_key_pem)?;
    std::fs::write(&csr_path, out.csr_pem)?;

    println!("✔ Clave privada: {}", key_path.display());
    println!("✔ CSR:           {}", csr_path.display());
    println!();
    println!("Pasos siguientes (manual, una sola vez):");
    println!("  1. Ingresá al portal de ARCA → «Administración de Certificados Digitales».");
    println!("  2. Subí {}.", csr_path.display());
    println!("  3. Descargá el certificado emitido a {}.", cfg.cert_path.display());
    println!("  4. Asociá el certificado a «Facturación Electrónica» (WSFE) en «Administrador de Relaciones».");
    Ok(())
}

pub fn status(home: &Path) -> R {
    let client = load_client(home)?;
    let (app, db, auth) = client.status()?;
    println!("Estado WSFE → AppServer: {app} | DbServer: {db} | AuthServer: {auth}");
    Ok(())
}

pub fn last_voucher(home: &Path) -> R {
    let client = load_client(home)?;
    let n = client.last_voucher(VoucherType::FacturaC)?;
    println!("Último comprobante autorizado (Factura C): {n} (siguiente: {})", n + 1);
    Ok(())
}

pub fn list_vouchers(home: &Path, last: u64, from: Option<u64>, to: Option<u64>) -> R {
    let client = load_client(home)?;

    let to = match to {
        Some(t) => t,
        None => client.last_voucher(VoucherType::FacturaC)?,
    };
    if to == 0 {
        println!("No hay comprobantes emitidos.");
        return Ok(());
    }
    let from = from
        .unwrap_or_else(|| to.saturating_sub(last.saturating_sub(1)))
        .max(1);
    if from > to {
        return Err(format!("--from ({from}) es mayor que --to ({to})").into());
    }

    let vouchers = client.list_vouchers(VoucherType::FacturaC, from, to)?;

    println!(
        "{:>8}  {:>8}  {:>14}  {:>11}  {:<14}  {:>3}",
        "NÚMERO", "FECHA", "IMPORTE", "DOC NRO", "CAE", "RES"
    );
    for v in &vouchers {
        let doc = if v.doc_nro == 0 {
            "CF".to_string()
        } else {
            v.doc_nro.to_string()
        };
        println!(
            "{:>8}  {:>8}  {:>14}  {:>11}  {:<14}  {:>3}",
            v.numero,
            v.fecha,
            format!("${:.2}", v.importe_total),
            doc,
            v.cae.as_deref().unwrap_or("-"),
            v.resultado,
        );
    }
    Ok(())
}
