# AFIP/ARCA setup

One-time steps before invoicy can issue `afip_c` invoices (see the [README](../README.md#afip-factura-c-argentina) for the invoice format).

**Requirements**

- A CUIT with **Clave Fiscal level 3** or higher.
- Registered in a regime that issues **Factura C** (e.g. **Monotributo**), with an activity on file.
- A **punto de venta enabled for Web Services** (step 1).

## The home directory

All AFIP commands, and `generate` for `afip_c`, work on a home directory that holds your issuer profile (`emisor.toml`), your key and certificate (`certs/invoicy.key`, `certs/invoicy.crt`) and the cached login credentials (`cache/`). It is `--home <dir>` if given, otherwise `$AFIP_HOME`, otherwise `~/invoicy`:

```bash
invoicy --home ~/invoicy/produccion generate -c factura.toml
```

**The home decides the environment.** Homologación (ARCA's testing environment) and producción need different certificates and different portal steps; a testing certificate doesn't work in production and vice versa. Keep one home per environment (e.g. `~/invoicy/homologacion` and `~/invoicy/produccion`) and double-check which one you use: every `generate` against production issues a real, fiscally valid invoice.

## 1. Create the punto de venta in ARCA

You need a punto de venta **enabled for Web Services**, which is **different** from the one usually used in «Comprobantes en Línea»:

1. In the ARCA portal, open **«Administración de puntos de venta y domicilios»**.
2. Choose **«A/B/M de puntos de venta»** and add a new one.
3. Under **Sistema**, choose **«Factura Electrónica – Monotributo – Web Service»**.
4. Confirm with **«Aceptar»**.

Write down the **punto de venta number**: it goes in `--punto-venta`. Detailed guide: [Crear punto de venta (Afip SDK)](https://docs.afipsdk.com/recursos/tutoriales-pagina-de-arca/crear-punto-de-venta).

## 2. Configure the issuer

```bash
invoicy --home ~/invoicy/homologacion afip configure \
  --cuit 20111111112 \
  --razon-social "Mi Nombre" \
  --punto-venta 1 \
  --domicilio "Av. Corrientes 1234 - CABA" \
  --ingresos-brutos "20111111112" \
  --inicio-actividades "01/01/2020"
# add --production for the real environment (default: homologación)
```

This writes `emisor.toml`. `--cuit`, `--razon-social` and `--punto-venta` are used to talk to AFIP; `--condicion-iva` (default `Responsable Monotributo`), `--domicilio`, `--ingresos-brutos` and `--inicio-actividades` are only printed on the invoice.

## 3. Generate the private key + CSR

```bash
invoicy --home ~/invoicy/homologacion afip generate-certificate
```

Creates two files **locally** in `<home>/certs/`:

- `invoicy.key`: your **private key**. It is never uploaded anywhere; it stays on your machine.
- `invoicy.csr`: the **certificate request** (PKCS#10) you upload to the ARCA portal in the next step.

`--alias <name>` (default `invoicy`) names the certificate: it becomes the CSR's CN, which is how you tell certificates apart in the ARCA portal (e.g. one per machine, or a renewal). The files are always `certs/invoicy.*`, the paths in `emisor.toml`, whatever the alias.

Once `invoicy.key` exists, generating a new one needs `--force`: the new key invalidates the certificate ARCA issued for the old one, so you'll need to upload the new CSR and replace `invoicy.crt`.

## 4. Enable the certificate in the ARCA portal

### Homologación (testing): via WSASS

The testing service is managed with **WSASS** and **can't be delegated**: log in with the **individual's** clave fiscal (not a company's).

1. In the ARCA portal, find and add the service **«WSASS – Autogestión Certificados Homologación»**.
2. Open **«Nuevo Certificado»**: enter a **DN symbolic name** and **paste the contents of `invoicy.csr`** into the PKCS#10 field. Save the issued certificate as `<home>/certs/invoicy.crt` (see [how to generate the certificate](https://docs.afipsdk.com/recursos/tutoriales-pagina-de-arca/habilitar-administrador-de-certificados-de-testing)).
3. Open **«Crear autorización a servicio»**: pick that DN, enter the **represented CUIT** and choose the **`wsfe`** service (see [how to authorize the testing web service](https://docs.afipsdk.com/recursos/tutoriales-pagina-de-arca/autorizar-web-service-de-testing)).

### Producción: via Administración de Certificados Digitales

1. Add and open **«Administración de Certificados Digitales»** (see [how to enable it](https://docs.afipsdk.com/recursos/tutoriales-pagina-de-arca/habilitar-administrador-de-certificados-de-produccion)). Create an **alias**, upload `invoicy.csr` and save the issued certificate as `<home>/certs/invoicy.crt` (see [how to generate the certificate](https://docs.afipsdk.com/recursos/tutoriales-pagina-de-arca/obtener-certificado-de-produccion#paso-4-generar-el-certificado-cert)).
2. Open **«Administrador de Relaciones de Clave Fiscal»** and create a relation that links that certificate (as a *computador fiscal*) to the **«wsfe – Factura Electrónica»** service (see [how to authorize the web service](https://docs.afipsdk.com/recursos/tutoriales-pagina-de-arca/autorizar-web-service-de-produccion)).

## 5. Check it works

```bash
# Service health (doesn't use the certificate)
invoicy --home ~/invoicy/homologacion afip status

# Last authorized Factura C: logs in with your certificate
invoicy --home ~/invoicy/homologacion afip last-voucher

# Issued vouchers (the last 10 by default)
invoicy --home ~/invoicy/homologacion afip list-vouchers
invoicy --home ~/invoicy/homologacion afip list-vouchers --last 25
invoicy --home ~/invoicy/homologacion afip list-vouchers --from 1 --to 50
```

If `last-voucher` works, `generate` can authorize `afip_c` invoices.
