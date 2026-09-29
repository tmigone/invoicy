# invoicy

A CLI tool for generating PDF invoices from TOML configuration files using Typst templates, and for issuing Argentine electronic invoices (Factura C) directly against ARCA/AFIP's web services.

## Features

- Generate PDF invoices from simple TOML configs
- Multiple invoice formats:
  - `generic` - Simple international invoice
  - `afip_c` - Argentina Factura C (Monotributo), authorized against ARCA/AFIP
- Argentine electronic invoicing without third-party services: talks to AFIP's WSAA/WSFE web services directly to get the CAE, and prints the QR code ARCA requires
- Each invoice is written as a PDF plus a TOML record with every field, including the ones filled in automatically
- Override config values via CLI (`--set key=value`)
- Customizable templates via Typst
- Single binary: templates and assets are built in; fonts come from the system (see [Fonts](#fonts))

## Installation

```bash
curl -fsSL https://raw.githubusercontent.com/tmigone/invoicy/main/install.sh | sh
```

Or download binaries directly from [GitHub Releases](https://github.com/tmigone/invoicy/releases).

### Build from source

Requires Rust 1.85+ (edition 2024) and a C compiler: the first build compiles OpenSSL from source (used to sign AFIP login tickets), which takes a while.

```bash
cargo build --release
# binary at target/release/invoicy
```

## Usage

```bash
# Generate invoice using built-in template
# (writes output/<home>/invoice-<number>.pdf and .toml; see below)
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

`examples/` has ready-to-edit invoices: `generic.toml`, and for `afip_c` `consumidor_final.toml` and `responsable_inscripto.toml`.

## Configuration

Create a TOML file with your invoice data. The `format` field determines which template to use.

`generate` writes two files into the output directory: the PDF, and a TOML with every field of the invoice, including the ones filled in automatically. That TOML is the invoice's record; `invoicy schema <format>` lists all fields and tags the automatic ones with where they come from.

The output directory defaults to `output/<name of the home directory>` under the current directory, so each issuer's invoices land in their own folder: with the default home `~/invoicy` that's `output/invoicy`, with `--home ~/invoicy/ana` it's `output/ana`. `--output <dir>` overrides it.

A draft can also say which home and output directory it uses, so you don't have to pass them every time:

```toml
format = "afip_c"
home = "~/invoicy/ana"   # the issuer (see docs/afip-setup.md)
output = "facturas/ana"  # where the PDF and TOML go
```

Relative paths are relative to the draft's own folder, and `~` is your home folder. These two keys aren't part of the invoice: they don't appear in `invoicy schema` or in the output TOML. If you also pass `--home` or `--output` and it points somewhere else, `generate` stops with an error instead of guessing which one you meant. Otherwise the draft's values win over `$AFIP_HOME` and the defaults.

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

Requires the one-time [AFIP setup](docs/afip-setup.md).

You write the receptor, the comprobante's concepto and dates, and the items; invoicy fills in the rest when AFIP authorizes the invoice. Every `generate` issues a new comprobante.

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

The PDF footer carries the QR code ARCA requires on electronic invoices (RG 4892/2020): it encodes ARCA's verification URL for the voucher (`https://www.arca.gob.ar/fe/qr/?p=…`), which is also recorded as `qr` in the output TOML.

The receptor's `condicion_iva`, `doc_tipo` and `doc_nro` are the codes sent to AFIP when authorizing, and the PDF prints their labels ("IVA Responsable Inscripto", "CUIT: 30123456789"), so the two always match. Omit all three for an anonymous consumidor final. `condicion_iva` takes `responsable_inscripto`, `exento`, `consumidor_final`, `monotributo`, `no_categorizado`, `proveedor_del_exterior`, `cliente_del_exterior`, `liberado`, `monotributista_social`, `no_alcanzado` or `monotributo_trabajador_independiente_promovido`; `doc_tipo` takes `cuit`, `cuil`, `dni` or `consumidor_final`.

`comprobante.concepto` is required: `productos`, `servicios` or `productos_y_servicios`. `comprobante.fecha_vencimiento` (the payment due date) is required too and always printed. For `servicios` (and mixed), it is sent to AFIP together with `periodo_desde` and `periodo_hasta` as the billing period; for `productos` it is only printed.

Each item's subtotal is `cantidad × precio_unitario`, rounded to cents, and the total sent to AFIP is their sum. Discounts (bonificaciones) are not supported; the PDF prints them as 0.

## AFIP setup

Issuing `afip_c` invoices needs a one-time setup: a Web Services punto de venta in ARCA, your issuer profile, and a certificate authorized for AFIP's `wsfe` service. The full walkthrough is in [docs/afip-setup.md](docs/afip-setup.md).

In short, all AFIP commands (and `generate` for `afip_c`) use a home directory (`--home`, else `$AFIP_HOME`, else `~/invoicy`) that holds `emisor.toml`, the certificate and the login cache. **The home decides the environment** (homologación or producción): keep one per environment.

## Custom templates

`-t my-template.typ` replaces the format's built-in template (see `crates/renderer/templates/`). The invoice reaches the template as the `invoice-data` variable, loaded from JSON before your template runs:

- The fields have the same names as in the TOML record (`invoicy schema <format>`).
- `generic` adds `total`.
- `afip_c` adds `subtotal`, `otros_tributos` and `total` at the top level, and zero `bonificacion_porcentaje` / `bonificacion_importe` on each item. The receptor's codes arrive as their printed labels: `condicion_iva` (e.g. "Consumidor Final"), `doc_tipo` (e.g. "CUIT") and `doc_nro` (`none` for an anonymous consumidor final).

Files available to `image()`: `arca.jpeg` (the ARCA logo) and, for an authorized `afip_c` invoice, `qr.svg` (the QR code; `invoice-data.qr` is empty when there is none).

## Fonts

Fonts are not built in: invoicy loads the fonts installed on the system. The built-in templates use `Helvetica Neue` (`afip_c`) and `Helvetica` (`generic`), which ship with macOS. On Linux or Windows, Typst falls back to another installed font, so the PDF looks different.
