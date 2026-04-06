// Invoice Template
// Data is injected via #let invoice-data = (...) before this file

#set page(
  paper: "a4",
  margin: 32mm,
)

#set text(
  font: "Helvetica",
  size: 10pt,
)

#let data = invoice-data

// Title
#text(size: 18pt, weight: "bold")[Invoice]

#v(16pt)

// From/To section
#grid(
  columns: (1fr, 1fr),
  gutter: 24pt,
  [
    #text(weight: "bold")[From]
    #v(4pt)
    #data.company.name \
    #data.company.address \
    #data.company.city_state_zip \
    #data.company.country
  ],
  [
    #text(weight: "bold")[To]
    #v(4pt)
    #data.client.name \
    #data.client.address \
    #data.client.city_state_zip \
    #data.client.country \
    #if data.client.tax_id != none [
      Tax ID: #data.client.tax_id
    ]
  ],
)

#v(24pt)

// Invoice metadata
#let meta-row(label, value) = {
  grid(
    columns: (100pt, auto),
    text(weight: "bold")[#label],
    value,
  )
}

#meta-row("Invoice No.", data.invoice.number)
#v(4pt)
#meta-row("Date", data.invoice.date)
#v(4pt)
#meta-row("Invoice Due", data.invoice.due_date)

#v(16pt)

// Line items table
#table(
  columns: (1fr, auto),
  stroke: none,
  inset: (x: 0pt, y: 6pt),

  // Header
  table.cell(stroke: (bottom: 1pt))[#text(weight: "bold")[Description]],
  table.cell(stroke: (bottom: 1pt), align: right)[#text(weight: "bold")[Rate]],

  // Items
  ..for item in data.items {
    (
      table.cell(stroke: (bottom: 0.5pt + luma(200)))[#item.description],
      table.cell(stroke: (bottom: 0.5pt + luma(200)), align: right)[
        #{
          let formatted = str(calc.round(item.rate, digits: 2))
          if not formatted.contains(".") { formatted += ".00" }
          else if formatted.split(".").at(1).len() == 1 { formatted += "0" }
          formatted
        } #data.invoice.currency
      ],
    )
  },
)

// Total
#v(4pt)
#line(length: 100%, stroke: 1pt)
#v(4pt)
#grid(
  columns: (1fr, auto),
  text(weight: "bold")[Total #data.invoice.currency],
  text(weight: "bold")[#{
    let formatted = str(calc.round(data.total, digits: 2))
    if not formatted.contains(".") { formatted += ".00" }
    else if formatted.split(".").at(1).len() == 1 { formatted += "0" }
    // Add thousand separators
    let parts = formatted.split(".")
    let int-part = parts.at(0)
    let dec-part = parts.at(1)
    let chars = int-part.clusters().rev()
    let grouped = ()
    for (i, c) in chars.enumerate() {
      if i > 0 and calc.rem(i, 3) == 0 { grouped.push(",") }
      grouped.push(c)
    }
    grouped.rev().join() + "." + dec-part
  }],
)
