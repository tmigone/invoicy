// AFIP Factura C Template
// Data is injected via #let invoice-data = (...) 

#set page(
  paper: "a4",
  margin: (x: 5mm, top: 7mm, bottom: 15mm),
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


  // Header: ORIGINAL
  #box(
    width: 100%,
    inset: 8pt,
    stroke: (
      left: 1pt,
      right: 1pt,
      top: 1pt,
      bottom: 0pt,
    ),
  )[
    #align(center)[
      #text(size: 14pt, weight: "bold")[ORIGINAL]
    ]
  ]

  // Header row: Left (emisor) | Right (FACTURA)
  #v(-11pt)
  #box(
    width: 100%,
    inset: 10pt,
    stroke: 1pt,
  )[
    // Vertical line
    #place(top + center)[
      #rect(width: 1pt, height: 113pt, fill: black)
    ]
        
    // Invoice type box -- overlapping top center
    #place(top + center)[
      #v(-10pt)
      #box(
        width: 50pt,
        height: 40pt,
        stroke: 1pt,
        fill: white,
        inset: 0pt,
      )[
        #v(-8pt)
        #align(center + horizon)[
          #text(size: 24pt, weight: "bold")[#data.comprobante.tipo]
        ]
        #place(bottom + center, dy: -4pt)[
          #text(size: 7pt, weight: "bold")[COD. #data.comprobante.codigo]
        ]
      ]
    ]

    #grid(
      columns: (1fr, 1fr),
      // Left section: Emisor info
      box(inset: (top: 8pt, left: 0pt), height: 103pt)[
        #align(center)[
          #text(size: 10pt, weight: "bold")[#data.emisor.razon_social]
        ]
        #v(1fr)
        #text(size: 9pt)[
          *Razón Social:* #data.emisor.razon_social \
        ]
        #v(1pt)
        #text(size: 9pt)[
          *Domicilio Comercial:* #data.emisor.domicilio_comercial \
        ]
        #v(1pt)
        #text(size: 9pt)[
          *Condición frente al IVA:* #data.emisor.condicion_iva
        ]
      ],
      
      // Right section: FACTURA info
      box(
        inset: (top: 8pt, left: 45pt),
        height: 90pt,
      )[
        #align(left)[
          #text(size: 16pt, weight: "bold")[FACTURA]
        ]
        #v(-3pt)
        #text(size: 9pt)[
          *Punto de Venta:* #data.comprobante.punto_de_venta #h(4pt) *Comp. Nro:* #data.comprobante.numero \
        ]
        #v(1pt)
        #text(size: 9pt)[
          *Fecha de Emisión:* #data.comprobante.fecha_emision \
        ]
        #v(8pt)
        #text(size: 9pt)[
          *CUIT:* #data.emisor.cuit \
          *Ingresos Brutos:* #data.emisor.ingresos_brutos \
          *Fecha de Inicio de Actividades:* #data.emisor.inicio_actividades
        ]
      ],
    )
  ]

  // Periodo facturado
  #v(-9pt)
  #box(width: 100%, inset: 8pt, stroke: 1pt)[
    #grid(
      columns: (1fr, 1fr, 1fr),
      gutter: 10pt,
      [*Período Facturado Desde:* #if data.comprobante.periodo_desde != none { data.comprobante.periodo_desde } else { "" }],
      align(center)[*Hasta:* #if data.comprobante.periodo_hasta != none { data.comprobante.periodo_hasta } else { "" }],
      align(right)[*Fecha de Vto. para el pago:* #data.comprobante.fecha_vencimiento],
    )
  ]


  // Receptor info
  #v(-9pt)
  #box(width: 100%, inset: 10pt, stroke: 1pt)[
    #grid(
      columns: (40%, 60%),
      row-gutter: 10pt,
      // Row 1
      [*CUIT:* #if data.receptor.documento != none { data.receptor.documento } else { "-" }],
      [*Apellido y Nombre / Razón Social:* #if data.receptor.nombre != none { data.receptor.nombre } else { "" }],
      // Row 2
      [*Condición frente al IVA:* #data.receptor.condicion_iva],
      [*Domicilio:* #if data.receptor.domicilio != none { data.receptor.domicilio } else { "" }],
      // Row 3
      [*Condición de venta:* #data.receptor.condicion_venta],
      [],
    )
  ]

  // Items table (simpler - no IVA columns)
  #v(-9pt)
  #box(width: 100%, inset: 0pt)[
    #table(
      columns: (40pt, 1fr, 55pt, 60pt, 85pt, 45pt, 75pt, 70pt),
      stroke: none,
      inset: 5pt,
      align: (left, left, right, center, right, center, right, right),

      // Header with background
      table.cell(fill: luma(200), stroke: 1pt)[*Código*],
      table.cell(fill: luma(200), stroke: 1pt)[*Producto / Servicio*],
      table.cell(fill: luma(200), stroke: 1pt)[*Cantidad*],
      table.cell(fill: luma(200), stroke: 1pt)[*U. Medida*],
      table.cell(fill: luma(200), stroke: 1pt)[*Precio Unit.*],
      table.cell(fill: luma(200), stroke: 1pt)[*% Bonif*],
      table.cell(fill: luma(200), stroke: 1pt)[*Imp. Bonif*],
      table.cell(fill: luma(200), stroke: 1pt)[*Subtotal*],

      // Items
      ..for item in data.items {
        (
          [#item.codigo],
          [#item.descripcion],
          [#format-number(item.cantidad)],
          [#item.unidad],
          [#format-number(item.precio_unitario)],
          [#format-number(item.bonif_pct)],
          [#format-number(item.bonif_imp)],
          [#format-number(item.subtotal)],
        )
      },
    )
  ]

  // Spacer to push footer down
  #v(1fr)

  // Totals (simple - right aligned)
  #box(width: 100%, stroke: 1pt, inset: (x: 10pt, y: 20pt))[
    #align(right)[
      #box(inset: 8pt)[
        #grid(
          columns: (auto, 100pt),
          row-gutter: 10pt,
          column-gutter: 12pt,
          align(right)[*Subtotal: \$*],
          align(right)[*#format-number(data.subtotal)*],
          align(right)[*Importe Otros Tributos: \$*],
          align(right)[*#format-number(data.otros_tributos)*],
          align(right)[*Importe Total: \$*],
          align(right)[*#format-number(data.total)*],
        )
      ]
    ]
  ]

  // Footer - QR + AFIP | Pág | CAE
  #v(25pt)
  #box(width: 100%, inset: 10pt)[
    
    #place(center + top)[
        *Pág 1/1*
    ]
    
    #grid(
      columns: (20%, 45%, 35%),
      
      // Left: QR placeholder
      box(width: 70pt, height: 70pt, stroke: 0.5pt)[
        #align(center + horizon)[
          #text(size: 6pt, fill: luma(150))[QR]
        ]
      ],
      
      // Center: ARCA info
      box(inset: (left: -20pt, top: -5pt))[
        #image("arca.jpeg", width: 55pt)
        #text(size: 9pt, style: "italic", weight: "bold")[Comprobante Autorizado]
        #v(0pt)
        #text(size: 7pt, style: "italic")[Esta Agencia no se responsabiliza por los datos ingresados en el detalle de la operación]
      ],
      
      // Right: CAE info
      box()[
        #grid(
          columns: (auto, auto),
          column-gutter: 4pt,
          row-gutter: 4pt,
          align(right)[*CAE N°:*],
          align(left)[#data.cae.numero],
          align(right)[*Fecha de Vto. de CAE:*],
          align(left)[#data.cae.vencimiento],
        )
      ]
    )
  ]
  #v(60pt)