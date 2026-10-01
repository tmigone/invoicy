use std::fmt::Write;

use schema::introspect::Field;
use schema::{FORMATS, introspect};

/// Wrap the type column so enum value lists stay within this many columns.
const MAX_WIDTH: usize = 100;

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
    if fields.iter().any(|f| f.source.is_some()) {
        println!(
            "Fields tagged [..] are filled in automatically and can't be written in the TOML.\n"
        );
    }
    print!("{}", describe(&fields));
    Ok(())
}

/// One line per field: its path, then its type — or, for enums, the accepted
/// values — wrapped and aligned under the type column.
fn describe(fields: &[Field]) -> String {
    let width = fields.iter().map(|f| f.path.len()).max().unwrap_or(0);
    let indent = 2 + width + 2;
    let available = MAX_WIDTH.saturating_sub(indent).max(30);

    let mut out = String::new();
    for field in fields {
        for (i, line) in type_lines(field, available).iter().enumerate() {
            if i == 0 {
                let _ = writeln!(out, "  {:<width$}  {line}", field.path);
            } else {
                let _ = writeln!(out, "{:indent$}{line}", "");
            }
        }
    }
    out
}

fn type_lines(field: &Field, available: usize) -> Vec<String> {
    let mut words: Vec<String> = if field.values.is_empty() {
        vec![field.typ.clone()]
    } else {
        let last = field.values.len() - 1;
        field
            .values
            .iter()
            .enumerate()
            .map(|(i, v)| {
                if i < last {
                    format!("{v} |")
                } else {
                    v.clone()
                }
            })
            .collect()
    };
    match &field.default {
        Some(default) => words.push(format!("(optional, default: {default})")),
        None if field.optional => words.push("(optional)".to_string()),
        None => {}
    }
    if let Some(source) = &field.source {
        words.push(format!("[{source}]"));
    }

    let mut lines = Vec::new();
    let mut line = String::new();
    for word in words {
        let len = line.chars().count();
        if len > 0 && len + 1 + word.chars().count() > available {
            lines.push(std::mem::take(&mut line));
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(&word);
    }
    lines.push(line);
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    fn field(path: &str, values: &[&str], optional: bool, default: Option<&str>) -> Field {
        Field {
            path: path.into(),
            typ: "string".into(),
            optional,
            values: values.iter().map(|v| v.to_string()).collect(),
            default: default.map(str::to_string),
            source: None,
        }
    }

    #[test]
    fn plain_fields_keep_their_type() {
        let out = describe(&[field("a", &[], false, None), field("bb", &[], true, None)]);
        assert_eq!(out, "  a   string\n  bb  string (optional)\n");
    }

    #[test]
    fn enums_list_their_values_and_default() {
        let out = describe(&[field("doc_tipo", &["cuit", "dni"], true, Some("dni"))]);
        assert_eq!(out, "  doc_tipo  cuit | dni (optional, default: dni)\n");
    }

    #[test]
    fn automatic_fields_show_their_source() {
        let mut numero = field("comprobante.numero", &[], false, None);
        numero.source = Some("AFIP".into());
        assert_eq!(describe(&[numero]), "  comprobante.numero  string [AFIP]\n");
    }

    #[test]
    fn long_value_lists_wrap_under_the_type_column() {
        let values: Vec<String> = (0..30).map(|i| format!("value_{i}")).collect();
        let values: Vec<&str> = values.iter().map(String::as_str).collect();
        let out = describe(&[field("x", &values, false, None)]);
        let lines: Vec<&str> = out.lines().collect();
        assert!(lines.len() > 1);
        assert!(
            lines.iter().all(|l| l.chars().count() <= MAX_WIDTH),
            "{out}"
        );
        for continuation in &lines[1..] {
            assert!(continuation.starts_with("     ") && !continuation.starts_with("      "));
        }
        let rejoined: String = out.split_whitespace().collect::<Vec<_>>().join(" ");
        assert!(rejoined.ends_with("value_28 | value_29"));
    }
}
