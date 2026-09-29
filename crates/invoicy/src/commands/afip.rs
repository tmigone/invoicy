//! AFIP/ARCA web-service commands (authorization side).
//!
//! These drive the `afip` crate (WSAA + WSFEv1) via the shared emisor profile
//! (`emisor.toml` under `--home`). Exposed under `invoicy afip <command>`.

use std::path::{Path, PathBuf};

use afip::{Client, Environment, VoucherType};

use crate::emisor::EmisorProfile;

type BoxError = Box<dyn std::error::Error>;
type R = Result<(), BoxError>;

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

pub fn load_client(home: &Path) -> Result<Client, BoxError> {
    EmisorProfile::load(home)?.client(home)
}

#[allow(clippy::too_many_arguments)]
pub fn configure(
    home: &Path,
    cuit: u64,
    razon_social: String,
    punto_venta: u32,
    condicion_iva: String,
    domicilio_comercial: String,
    ingresos_brutos: String,
    inicio_actividades: String,
    production: bool,
) -> R {
    std::fs::create_dir_all(home.join("certs"))?;
    let profile = EmisorProfile {
        cuit,
        razon_social,
        domicilio_comercial,
        condicion_iva,
        ingresos_brutos,
        inicio_actividades,
        punto_venta,
        environment: if production {
            Environment::Produccion
        } else {
            Environment::Homologacion
        },
        cert_path: home.join("certs/invoicy.crt"),
        key_path: home.join("certs/invoicy.key"),
    };
    profile.save(home)?;
    println!(
        "✔ Perfil de emisor guardado en {}",
        EmisorProfile::path(home).display()
    );
    println!(
        "  entorno: {}",
        if production {
            "producción"
        } else {
            "homologación"
        }
    );
    println!("  Siguiente: `invoicy afip generate-certificate`");
    Ok(())
}

/// Generate the key + CSR at the profile's key path (the CSR next to it), so
/// the certificate ARCA issues pairs with the key invoicy logs in with.
/// `alias` only names the certificate: it becomes the CSR's CN.
pub fn generate_certificate(home: &Path, alias: &str, force: bool) -> R {
    let profile = EmisorProfile::load(home)?;

    let key_path = profile.key_path.clone();
    let csr_path = key_path.with_extension("csr");
    if let Some(dir) = key_path.parent() {
        std::fs::create_dir_all(dir)?;
    }

    if key_path.exists() && !force {
        return Err(format!(
            "{} ya existe — usá --force para sobrescribir (esto invalida el certificado emitido por ARCA)",
            key_path.display()
        )
        .into());
    }

    let out = afip::cert::generate_key_and_csr(profile.cuit, &profile.razon_social, alias)?;
    std::fs::write(&key_path, out.private_key_pem)?;
    std::fs::write(&csr_path, out.csr_pem)?;

    println!("✔ Clave privada: {}", key_path.display());
    println!("✔ CSR:           {}", csr_path.display());
    println!();
    println!("Pasos siguientes (manual, una sola vez):");
    println!("  1. Ingresá al portal de ARCA → «Administración de Certificados Digitales».");
    println!("  2. Subí {}.", csr_path.display());
    println!(
        "  3. Descargá el certificado emitido a {}.",
        profile.cert_path.display()
    );
    println!(
        "  4. Asociá el certificado a «Facturación Electrónica» (WSFE) en «Administrador de Relaciones»."
    );
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
    println!(
        "Último comprobante autorizado (Factura C): {n} (siguiente: {})",
        n + 1
    );
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A fresh home with a profile, under the system temp dir.
    fn home(name: &str) -> PathBuf {
        let home = std::env::temp_dir().join(format!("invoicy-test-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        configure(
            &home,
            20111111112,
            "Test".into(),
            1,
            "Responsable Monotributo".into(),
            String::new(),
            String::new(),
            String::new(),
            false,
        )
        .unwrap();
        home
    }

    #[test]
    fn certificate_files_follow_the_profile_whatever_the_alias() {
        let home = home("alias");
        generate_certificate(&home, "renovado", false).unwrap();

        let profile = EmisorProfile::load(&home).unwrap();
        assert!(profile.key_path.exists());
        assert!(profile.key_path.with_extension("csr").exists());
        assert!(!home.join("certs/renovado.key").exists());

        // A second key must be asked for explicitly: it invalidates the
        // certificate ARCA issued for the first one.
        assert!(generate_certificate(&home, "otro", false).is_err());
        let before = std::fs::read(&profile.key_path).unwrap();
        generate_certificate(&home, "otro", true).unwrap();
        assert_ne!(std::fs::read(&profile.key_path).unwrap(), before);

        std::fs::remove_dir_all(&home).unwrap();
    }
}
