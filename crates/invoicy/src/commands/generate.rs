use std::path::{Path, PathBuf};

use afip::Environment;
use schema::InvoiceConfig;
use toml::Value;

use super::afip::resolve_home;
use crate::afip_invoice;
use crate::emisor::EmisorProfile;
use crate::overrides;

type BoxError = Box<dyn std::error::Error>;

/// Generate an invoice into the output directory: `<name>.toml` with every
/// field of the invoice (including the ones filled in automatically) and
/// `<name>.pdf`.
///
/// The home and output directory come from `--home` / `--output` or from the
/// draft's `home` / `output` keys (they must agree if both are given), then
/// `$AFIP_HOME` / `~/invoicy` and [`default_output_dir`].
///
/// With `dry_run`, every check runs (for `afip_c`: the profile, the AFIP login
/// and the next voucher number, all read-only) and the PDF is rendered in
/// memory, but no CAE is requested and nothing is written.
pub fn generate(
    cli_home: Option<PathBuf>,
    config_path: PathBuf,
    template: Option<PathBuf>,
    cli_output: Option<PathBuf>,
    override_args: Vec<String>,
    dry_run: bool,
) -> Result<(), BoxError> {
    // Read and parse config as TOML Value first
    let config_content = std::fs::read_to_string(&config_path)?;
    let mut config_value: Value = toml::from_str(&config_content)?;

    // `home` and `output` say who issues the invoice and where it goes, not
    // what it contains: take them out before the invoice is parsed, so they
    // never reach the model or the output record.
    let base = config_path.parent().unwrap_or(Path::new(""));
    let from_file = take_run_settings(&mut config_value, base)?;
    let home = pick_path("home", "--home", cli_home, from_file.home)?
        .unwrap_or_else(|| resolve_home(None));
    let output_dir = pick_path("output", "--output", cli_output, from_file.output)?
        .unwrap_or_else(|| default_output_dir(&home));
    let home = home.as_path();

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
    if !dry_run {
        std::fs::create_dir_all(&output_dir)?;
    }

    // afip_c invoices are drafts: authorizing fills in the emisor, número,
    // fecha and CAE. Each run issues a new comprobante.
    let mut summary = Vec::new();
    if let InvoiceConfig::AfipC(inv) = &mut config {
        let profile = EmisorProfile::load(home)?;
        if dry_run {
            afip_invoice::check(&profile, home, inv)?;
            let environment = match profile.environment {
                Environment::Homologacion => "homologación (testing)",
                Environment::Produccion => "PRODUCCIÓN (would be a real invoice)",
            };
            summary.push(format!(
                "Issuer:      {} (CUIT {}), {environment}",
                profile.razon_social, profile.cuit
            ));
            summary.push(format!(
                "Number:      {}-{} (next free one; AFIP assigns it when issuing)",
                inv.comprobante.punto_de_venta, inv.comprobante.numero
            ));
            summary.push(format!(
                "Dates:       issued {}, payment due {}",
                inv.comprobante.fecha_emision, inv.comprobante.fecha_vencimiento
            ));
            summary.push(format!("Total:       ${:.2}", inv.totales.total));
        } else {
            afip_invoice::authorize(&profile, home, inv)?;
        }
    }

    let stem = unique_stem(&output_dir, &format!("invoice-{}", config.invoice_number()));

    if dry_run {
        renderer::render(&config, template_content.as_deref())?;
        println!("✔ Dry run: the invoice is valid and renders. Nothing was issued or written.");
        for line in &summary {
            println!("  {line}");
        }
        println!(
            "  Would write: {}",
            output_dir.join(format!("{stem}.{{toml,pdf}}")).display()
        );
        return Ok(());
    }

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

/// The draft's `home` and `output` keys, resolved to paths.
#[derive(Debug, Default)]
struct RunSettings {
    home: Option<PathBuf>,
    output: Option<PathBuf>,
}

/// Remove `home` and `output` from the draft's top level. Relative paths are
/// relative to the draft's directory (`base`), so a draft works from anywhere.
fn take_run_settings(value: &mut Value, base: &Path) -> Result<RunSettings, BoxError> {
    let Some(root) = value.as_table_mut() else {
        return Ok(RunSettings::default());
    };
    let mut take = |key: &str| -> Result<Option<PathBuf>, BoxError> {
        match root.remove(key) {
            None => Ok(None),
            Some(Value::String(raw)) => Ok(Some(resolve_path(&raw, base))),
            Some(_) => Err(format!("`{key}` debe ser una ruta (texto)").into()),
        }
    };
    Ok(RunSettings {
        home: take("home")?,
        output: take("output")?,
    })
}

/// Expand a leading `~` and make relative paths relative to `base`.
fn resolve_path(raw: &str, base: &Path) -> PathBuf {
    let user_home = || PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".into()));
    let path = match raw.strip_prefix("~/") {
        Some(rest) => user_home().join(rest),
        None if raw == "~" => user_home(),
        None => PathBuf::from(raw),
    };
    if path.is_absolute() {
        path
    } else {
        base.join(path)
    }
}

/// A path the command line (`flag`) and the draft (`key`) can both set. When
/// both do, they must name the same place: disagreeing about who issues the
/// invoice, or where it goes, is almost certainly a mistake.
fn pick_path(
    key: &str,
    flag: &str,
    cli: Option<PathBuf>,
    file: Option<PathBuf>,
) -> Result<Option<PathBuf>, BoxError> {
    match (cli, file) {
        (Some(cli), Some(file)) if !same_path(&cli, &file) => Err(format!(
            "{flag} {} no coincide con `{key}` del TOML ({}); dejá solo uno",
            cli.display(),
            file.display()
        )
        .into()),
        (Some(cli), _) => Ok(Some(cli)),
        (None, file) => Ok(file),
    }
}

/// Whether two paths name the same place (resolving `..` and symlinks when
/// they exist).
fn same_path(a: &Path, b: &Path) -> bool {
    match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
        (Ok(a), Ok(b)) => a == b,
        _ => a.components().eq(b.components()),
    }
}

/// `output/<home name>`, so each issuer's invoices (one home each) land in
/// their own folder: `--home ~/invoicy/ana` writes to `output/ana`.
fn default_output_dir(home: &Path) -> PathBuf {
    let output = PathBuf::from("output");
    match home.file_name() {
        Some(name) => output.join(name),
        None => output,
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_output_dir_follows_the_home_name() {
        let out = |home: &str| default_output_dir(Path::new(home));
        assert_eq!(out("/Users/x/invoicy/ana"), Path::new("output/ana"));
        assert_eq!(out("home/homologacion"), Path::new("output/homologacion"));
        assert_eq!(out("home/produccion/"), Path::new("output/produccion"));
        assert_eq!(out("/"), Path::new("output"));
    }

    fn draft(extra: &str) -> Value {
        toml::from_str(&format!(
            r#"
            format = "afip_c"
            {extra}
            [receptor]
            condicion_venta = "Contado"
            [comprobante]
            concepto = "productos"
            fecha_vencimiento = "15/10/2026"
            [[items]]
            codigo = "1"
            descripcion = "x"
            cantidad = 1.0
            unidad = "u"
            precio_unitario = 1.0
            "#
        ))
        .unwrap()
    }

    #[test]
    fn run_settings_resolve_against_the_draft_directory() {
        let mut value = draft("home = \"homes/ana\"\noutput = \"/tmp/facturas\"");
        let settings = take_run_settings(&mut value, Path::new("/work/drafts")).unwrap();
        assert_eq!(settings.home.unwrap(), Path::new("/work/drafts/homes/ana"));
        assert_eq!(settings.output.unwrap(), Path::new("/tmp/facturas"));

        // They aren't invoice data: the draft still parses as afip_c.
        assert!(value.get("home").is_none() && value.get("output").is_none());
        assert!(value.try_into::<InvoiceConfig>().is_ok());
    }

    #[test]
    fn run_settings_expand_tilde_and_reject_non_paths() {
        let user_home = PathBuf::from(std::env::var("HOME").unwrap());
        let mut value = draft("home = \"~/invoicy/ana\"");
        let settings = take_run_settings(&mut value, Path::new("/work")).unwrap();
        assert_eq!(settings.home.unwrap(), user_home.join("invoicy/ana"));
        assert!(settings.output.is_none());

        let mut value = draft("home = 3");
        assert!(take_run_settings(&mut value, Path::new("/work")).is_err());
    }

    #[test]
    fn flag_and_draft_must_agree() {
        let path = |p: &str| Some(PathBuf::from(p));
        assert_eq!(
            pick_path("home", "--home", path("/a"), None).unwrap(),
            path("/a")
        );
        assert_eq!(
            pick_path("home", "--home", None, path("/b")).unwrap(),
            path("/b")
        );
        assert_eq!(pick_path("home", "--home", None, None).unwrap(), None);
        assert!(pick_path("home", "--home", path("/a"), path("/b")).is_err());
        assert!(pick_path("output", "--output", path("/a"), path("/b")).is_err());

        // Same place, spelled differently.
        let dir = std::env::temp_dir();
        let dotted = dir.join("..").join(dir.file_name().unwrap());
        assert!(pick_path("home", "--home", Some(dir), Some(dotted)).is_ok());
    }

    fn example(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../../examples/{name}.toml"))
    }

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("invoicy-dry-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn dry_run_checks_and_renders_but_writes_nothing() {
        let out = scratch("generic");
        let generic = example("generic");
        generate(None, generic.clone(), None, Some(out.clone()), vec![], true).unwrap();
        assert!(!out.exists(), "dry run created {}", out.display());

        // Still a real check: a broken draft fails.
        let broken = vec!["items[0].rate=not-a-number".to_string()];
        assert!(generate(None, generic, None, Some(out.clone()), broken, true).is_err());
        assert!(!out.exists());
    }

    #[test]
    fn afip_dry_run_needs_a_working_certificate() {
        // A configured home without a certificate: the dry run gets through
        // the local checks and stops at the AFIP login, before any network.
        let home = scratch("afip-home");
        super::super::afip::configure(
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
        let out = scratch("afip-out");
        let draft_path = home.join("draft.toml");
        std::fs::write(&draft_path, toml::to_string(&draft("")).unwrap()).unwrap();
        let err = generate(
            Some(home.clone()),
            draft_path,
            None,
            Some(out.clone()),
            vec![],
            true,
        )
        .unwrap_err();
        assert!(err.to_string().contains("certificate"), "{err}");
        assert!(!out.exists());
        std::fs::remove_dir_all(&home).unwrap();
    }
}
