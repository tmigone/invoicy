// AFIP Factura Template with IVA breakdown (Argentina - Responsable Inscripto)
// Data is injected via #let invoice-data = (...) before this file

#set page(
  paper: "a4",
  margin: (x: 15mm, top: 10mm, bottom: 15mm),
)

#set text(
  font: "Helvetica Neue",
  size: 9pt,
)

#let data = invoice-data

// Helper: format number with comma as decimal separator (AFIP style)
#let format-number(n) = {
  let formatted = str(calc.round(n, digits: 2))
  if not formatted.contains(".") { formatted += ".00" }
  else if formatted.split(".").at(1).len() == 1 { formatted += "0" }
  formatted.replace(".", ",")
}

// ============================================================================
// MAIN CONTENT BOX
// ============================================================================

#box(
  width: 100%,
  stroke: 1pt,
  inset: 0pt,
)[
  // Header: ORIGINAL
  #box(
    width: 100%,
    fill: white,
    inset: 6pt,
  )[
    #align(center)[
      #text(size: 14pt, weight: "bold")[ORIGINAL]
    ]
  ]

  #line(length: 100%, stroke: 1pt)

  // Header row: Left (emisor) | C box (center divider) | Right (FACTURA)
  #grid(
    columns: (1fr, auto, 1fr),
    // Left section: Emisor info
    box(inset: 8pt, height: 90pt)[
      #align(center)[
        #text(size: 16pt, weight: "bold")[#data.emisor.razon_social]
      ]
      #v(1fr)
      #text(size: 9pt)[
        *Razón Social:* #data.emisor.razon_social \
        *Domicilio Comercial:* #data.emisor.domicilio_comercial \
        *Condición frente al IVA:* #data.emisor.condicion_iva
      ]
    ],
    // Center: C box as divider
    box(
      width: 50pt,
      height: 90pt,
      stroke: 1pt,
      inset: 0pt,
    )[
      #align(center + horizon)[
        #text(size: 36pt, weight: "bold")[#data.comprobante.tipo]
      ]
      #place(bottom + center, dy: -4pt)[
        #text(size: 7pt, weight: "bold")[COD. #data.comprobante.codigo]
      ]
    ],
    // Right section: FACTURA info
    box(inset: 8pt, height: 90pt)[
      #align(center)[
        #text(size: 16pt, weight: "bold")[FACTURA]
      ]
      #v(4pt)
      #text(size: 9pt)[
        *Punto de Venta:* #data.comprobante.punto_de_venta #h(4pt) *Comp. Nro:* #data.comprobante.numero \
        *Fecha de Emisión:* #data.comprobante.fecha_emision \
        *CUIT:* #data.emisor.cuit \
        *Ingresos Brutos:* #data.emisor.ingresos_brutos \
        *Fecha de Inicio de Actividades:* #data.emisor.inicio_actividades
      ]
    ],
  )

  #line(length: 100%, stroke: 1pt)

  // Periodo facturado
  #box(width: 100%, inset: 8pt)[
    #grid(
      columns: (1fr, 1fr, 1fr),
      gutter: 10pt,
      [*Período Facturado Desde:* #if data.comprobante.periodo_desde != none { data.comprobante.periodo_desde } else { "" }],
      align(center)[*Hasta:* #if data.comprobante.periodo_hasta != none { data.comprobante.periodo_hasta } else { "" }],
      align(right)[*Fecha de Vto. para el pago:* #data.comprobante.fecha_vencimiento],
    )
  ]

  #line(length: 100%, stroke: 1pt)

  // Receptor info
  #box(width: 100%, inset: 10pt)[
    #grid(
      columns: (30%, 70%),
      row-gutter: 8pt,
      [*CUIT:* #if data.receptor.cuit != none { data.receptor.cuit } else { "-" }],
      [*Apellido y Nombre / Razón Social:* #if data.receptor.nombre != none { data.receptor.nombre } else { "" }],
      [*Condición frente al IVA:* #data.receptor.condicion_iva],
      [*Domicilio:* #if data.receptor.domicilio != none { data.receptor.domicilio } else { "" }],
    )
    #v(4pt)
    *Condición de venta:* #data.receptor.condicion_venta
  ]

  #line(length: 100%, stroke: 1pt)

  // Items table
  #box(width: 100%, inset: 6pt)[
    #table(
      columns: (35pt, 1fr, 50pt, 55pt, 60pt, 40pt, 55pt, 50pt, 60pt),
      stroke: none,
      inset: 5pt,
      align: (left, left, right, center, right, center, right, right, right),

      // Header with background
      table.cell(fill: luma(200), stroke: 1pt)[*Código*],
      table.cell(fill: luma(200), stroke: 1pt)[*Producto / Servicio*],
      table.cell(fill: luma(200), stroke: 1pt)[*Cantidad*],
      table.cell(fill: luma(200), stroke: 1pt)[*U. Medida*],
      table.cell(fill: luma(200), stroke: 1pt)[*Precio Unit.*],
      table.cell(fill: luma(200), stroke: 1pt)[*% Bonif*],
      table.cell(fill: luma(200), stroke: 1pt)[*Subtotal*],
      table.cell(fill: luma(200), stroke: 1pt)[*Alic. IVA*],
      table.cell(fill: luma(200), stroke: 1pt)[*Subt. c/IVA*],

      // Items
      ..for item in data.items {
        (
          [#item.codigo],
          [#item.descripcion],
          [#format-number(item.cantidad)],
          [#item.unidad],
          [#format-number(item.precio_unitario)],
          [#format-number(item.bonif_pct)],
          [#format-number(item.subtotal)],
          [#format-number(item.alicuota_iva)],
          [#format-number(item.subtotal_con_iva)],
        )
      },
    )
  ]

  // Spacer to push footer down
  #v(1fr)

  // Footer: Otros tributos + Totals
  #box(width: 100%, stroke: (top: 1pt), inset: 8pt)[
    #grid(
      columns: (55%, 45%),
      gutter: 10pt,
      // Left: Otros tributos table
      [
        #text(weight: "bold")[Otros tributos]
        #v(4pt)
        #table(
          columns: (1fr, auto, auto, 60pt),
          stroke: none,
          inset: 4pt,
          align: (left, left, right, right),

          table.cell(fill: luma(200), stroke: 1pt)[*Descripción*],
          table.cell(fill: luma(200), stroke: 1pt)[*Detalle*],
          table.cell(fill: luma(200), stroke: 1pt)[*Alíc. %*],
          table.cell(fill: luma(200), stroke: 1pt)[*Importe*],

          ..for tributo in data.otros_tributos {
            (
              [#tributo.descripcion],
              [#if tributo.detalle != none { tributo.detalle } else { "" }],
              [#if tributo.alicuota != none { format-number(tributo.alicuota) } else { "" }],
              [#format-number(tributo.importe)],
            )
          },
        )
      ],
      // Right: Totals box
      align(right)[
        #box(stroke: 1pt, inset: 8pt)[
          #set text(size: 8pt)
          #grid(
            columns: (auto, 70pt),
            row-gutter: 4pt,
            column-gutter: 8pt,
            align(right)[*Importe Neto Gravado: \$*],
            align(right)[#format-number(data.totales.neto_gravado)],
            align(right)[*IVA 27%: \$*],
            align(right)[#format-number(data.totales.iva_27)],
            align(right)[*IVA 21%: \$*],
            align(right)[#format-number(data.totales.iva_21)],
            align(right)[*IVA 10.5%: \$*],
            align(right)[#format-number(data.totales.iva_10_5)],
            align(right)[*IVA 5%: \$*],
            align(right)[#format-number(data.totales.iva_5)],
            align(right)[*IVA 2.5%: \$*],
            align(right)[#format-number(data.totales.iva_2_5)],
            align(right)[*IVA 0%: \$*],
            align(right)[#format-number(data.totales.iva_0)],
            align(right)[*Importe Otros Tributos: \$*],
            align(right)[#format-number(data.totales.otros_tributos)],
            align(right)[*Importe Total: \$*],
            align(right)[*#format-number(data.totales.total)*],
          )
        ]
      ],
    )
  ]

  #line(length: 100%, stroke: 1pt)

  // Footer - QR + AFIP | Pág | CAE
  #box(width: 100%, inset: 10pt)[
    #grid(
      columns: (20%, 45%, 35%),
      gutter: 8pt,
      // Left: QR placeholder
      box(width: 60pt, height: 60pt, stroke: 0.5pt)[
        #align(center + horizon)[
          #text(size: 6pt, fill: luma(150))[QR]
        ]
      ],
      // Center: AFIP info + page
      [
        #text(size: 16pt, weight: "bold")[AFIP]
        #v(2pt)
        #text(size: 7pt)[ADMINISTRACIÓN FEDERAL DE\ INGRESOS PÚBLICOS]
        #v(6pt)
        #text(size: 9pt, style: "italic", weight: "bold")[Comprobante Autorizado]
        #v(4pt)
        #text(size: 7pt, style: "italic")[Esta Administración Federal no se responsabiliza por los datos ingresados en el detalle de la operación]
        #place(right + horizon)[
          *Pág 1/1*
        ]
      ],
      // Right: CAE info
      align(right + horizon)[
        #grid(
          columns: (auto, auto),
          column-gutter: 8pt,
          row-gutter: 4pt,
          align(right)[*CAE N°:*],
          align(left)[#data.cae.numero],
          align(right)[*Fecha de Vto. de CAE:*],
          align(left)[#data.cae.vencimiento],
        )
      ],
    )
  ]
]
