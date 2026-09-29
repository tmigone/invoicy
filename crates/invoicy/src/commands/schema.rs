use schema::{FORMATS, introspect};

pub fn schema(format: &str) -> Result<(), Box<dyn std::error::Error>> {
    if format == "list" {
        println!("Available formats:\n");
        for (name, description) in FORMATS {
            println!("  {name:<8} - {description}");
        }
        return Ok(());
    }

    let Some(fields) = introspect::format_fields(format) else {
        eprintln!("Unknown format: {}", format);
        eprintln!("Run 'invoicy schema list' to see available formats");
        std::process::exit(1);
    };

    println!("Format: {}\n", format);
    let width = fields.iter().map(|f| f.path.len()).max().unwrap_or(0);
    for field in fields {
        let optional = if field.optional { " (optional)" } else { "" };
        println!("  {:<width$}  {}{}", field.path, field.typ, optional);
    }
    Ok(())
}
