# invoicy

A CLI tool for generating PDF invoices from TOML configuration files using Typst templates.

## Features

- Generate PDF invoices from simple TOML configs
- Multiple invoice formats:
  - `generic` - Simple international invoice
  - `afip_c` - Argentina AFIP Factura C (Monotributo)
- Override config values via CLI (`--set key=value`)
- Customizable templates via Typst
- Single self-contained binary

## Installation

```bash
curl -fsSL https://raw.githubusercontent.com/tmigone/invoicy/main/install.sh | sh
```

Or download binaries directly from [GitHub Releases](https://github.com/tmigone/invoicy/releases).

### Build from source

```bash
cargo build --release
```

## Usage

```bash
# Generate invoice using built-in template
# (writes output/invoice-<number>.pdf and output/invoice-<number>.toml)
invoicy generate -c invoice.toml

# Write into another directory
invoicy generate -c invoice.toml -o invoices/2026-09

# Use a custom template
invoicy generate -c invoice.toml -t my-template.typ

# Override config values (useful for automation)
invoicy generate -c base.toml --set comprobante.periodo_desde=01/10/2026

# List available formats
invoicy schema list

# Show schema for a format (useful for discovering --set keys)
invoicy schema generic
invoicy schema afip_c
```


## Configuration

Create a TOML file with your invoice data. The `format` field determines which template to use.

`generate` writes two files into the output directory (`./output` by default,
`--output` to change it): the PDF, and a TOML with every field of the invoice,
including the ones filled in automatically. That TOML is the invoice's record;
`invoicy schema <format>` lists all fields and tags the automatic ones with
where they come from.

### Generic Invoice

```toml
format = "generic"

[company]
name = "Acme Corp"
address = "123 Business Ave"
address2 = ""
city_state_zip = "New York, NY, 10001"
country = "United States"

[client]
name = "Client Inc"
address = "456 Commerce St"
address2 = ""
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

You write the receptor, the comprobante's concepto and dates, and the items;
invoicy fills in the rest when AFIP authorizes the invoice. Every `generate`
issues a new comprobante.

```toml
format = "afip_c"

[receptor]
nombre = "Cliente SA"
domicilio = "Calle Falsa 123"
condicion_iva = "responsable_inscripto"
doc_tipo = "cuit"
doc_nro = 30123456789
condicion_venta = "Cuenta Corriente"

[comprobante]
concepto = "servicios"
periodo_desde = "01/01/2025"
periodo_hasta = "31/01/2025"
fecha_vencimiento = "15/02/2025"

[[items]]
codigo = "1"
descripcion = "Servicios profesionales"
cantidad = 1.0
unidad = "unidades"
precio_unitario = 50000.00
```

Filled in automatically (writing any of them is an error):

| Field | From |
|---|---|
| `[emisor]`, `comprobante.punto_de_venta` | AFIP: your profile, `emisor.toml` (`invoicy afip configure`) |
| `comprobante.numero`, `comprobante.fecha_emision`, `[cae]` | AFIP, when it authorizes the invoice |
| `comprobante.tipo` / `codigo` (`C` / `011`), `items[].subtotal`, `[totales]`, `qr` | computed |

The PDF footer carries the QR code ARCA requires on electronic invoices
(RG 4892/2020): it encodes ARCA's verification URL for the voucher
(`https://www.arca.gob.ar/fe/qr/?p=…`), which is also recorded as `qr` in the
output TOML.

The receptor's `condicion_iva`, `doc_tipo` and `doc_nro` are the codes sent to
AFIP when authorizing, and the PDF prints their labels ("IVA Responsable
Inscripto", "CUIT: 30123456789"), so the two always match. Omit all three for
an anonymous consumidor final. `condicion_iva` takes `responsable_inscripto`,
`exento`, `consumidor_final`, `monotributo`, `no_categorizado`,
`proveedor_del_exterior`, `cliente_del_exterior`, `liberado`,
`monotributista_social`, `no_alcanzado` or
`monotributo_trabajador_independiente_promovido`; `doc_tipo` takes `cuit`,
`cuil`, `dni` or `consumidor_final`.

`comprobante.concepto` is required: `productos`, `servicios` or
`productos_y_servicios`. For `servicios` (and mixed), `periodo_desde`,
`periodo_hasta` and `fecha_vencimiento` are sent to AFIP as the billing period
and payment due date.

Each item's subtotal is `cantidad × precio_unitario`, rounded to cents, and
the total sent to AFIP is their sum. Discounts (bonificaciones) are not
supported; the PDF prints them as 0.
