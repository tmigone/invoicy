use crate::formats::{AfipAInvoice, AfipCInvoice, GenericInvoice};
use crate::schema;

pub fn schema(format: &str) -> Result<(), Box<dyn std::error::Error>> {
    match format {
        "list" => {
            println!("Available formats:\n");
            println!("  generic  - Simple international invoice");
            println!("  afip_c   - Argentina AFIP Factura C (Monotributo)");
            println!("  afip_a   - Argentina AFIP Factura A (Responsable Inscripto)");
        }
        "generic" => schema::print_schema::<GenericInvoice>("generic"),
        "afip_c" => schema::print_schema::<AfipCInvoice>("afip_c"),
        "afip_a" => schema::print_schema::<AfipAInvoice>("afip_a"),
        _ => {
            eprintln!("Unknown format: {}", format);
            eprintln!("Run 'invoicy schema list' to see available formats");
            std::process::exit(1);
        }
    }
    Ok(())
}
