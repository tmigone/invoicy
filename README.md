# invoicy

A CLI tool for generating PDF invoices from TOML configuration files using Typst templates.

## Features

- Generate professional PDF invoices from simple TOML configs
- Multiple invoice formats:
  - `generic` - Simple international invoice
  - `afip_c` - Argentina AFIP Factura C (Monotributo)
  - `afip_a` - Argentina AFIP Factura A (Responsable Inscripto)
- Customizable templates via Typst
- System font support

## Installation

```bash
cargo build --release
```

The binary will be at `target/release/invoicy`.

## Usage

```bash
# Generate invoice using built-in template
invoicy -c examples/afip_c.toml

# Generate with custom output path
invoicy -c examples/afip_c.toml -o my-invoice.pdf

# Use a custom template
invoicy -c examples/generic.toml -t my-template.typ
```

## Configuration

Create a TOML file with your invoice data. The `format` field determines which template to use.

### Generic Invoice

```toml
format = "generic"

[company]
name = "Acme Corp"
address = "123 Business Ave"
city_state_zip = "New York, NY, 10001"
country = "United States"

[client]
name = "Client Inc"
address = "456 Commerce St"
city_state_zip = "Los Angeles, CA, 90001"
country = "United States"
tax_id = "12-3456789"

[invoice]
number = "INV-2025.001"
date = "Mar 30 2025"
due_date = "Apr 14 2025"
currency = "USD"

[[items]]
description = "Consulting Services"
rate = 5000.00
```

### AFIP Factura C (Argentina)

```toml
format = "afip_c"

[emisor]
razon_social = "Juan Pérez"
domicilio_comercial = "Av. Corrientes 1234 - CABA"
condicion_iva = "Responsable Monotributo"
cuit = "20123456789"
ingresos_brutos = "12345"
inicio_actividades = "01/01/2020"

[receptor]
nombre = "Cliente SA"
domicilio = "Calle Falsa 123"
documento = "30123456789"
condicion_iva = "Consumidor Final"
condicion_venta = "Cuenta Corriente"

[comprobante]
tipo = "C"
codigo = "011"
punto_de_venta = "00001"
numero = "00000001"
fecha_emision = "01/02/2025"
periodo_desde = "01/01/2025"
periodo_hasta = "31/01/2025"
fecha_vencimiento = "15/02/2025"

[[items]]
codigo = "1"
descripcion = "Servicios profesionales"
cantidad = 1.0
unidad = "unidades"
precio_unitario = 50000.00
bonificacion_porcentaje = 0.0
bonificacion_importe = 0.0
subtotal = 50000.00

[cae]
numero = "12345678901234"
vencimiento = "11/02/2025"
```

## Assets

Place images and other assets in the `assets/` directory. Templates can reference them by filename (e.g., `#image("arca.jpeg")`).

## License

MIT
