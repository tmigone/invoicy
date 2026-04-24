use serde::Deserialize;

use super::{escape_typst_string, typst_option};

#[derive(Debug, Deserialize)]
pub struct GenericInvoice {
    pub company: Company,
    pub client: Client,
    pub invoice: InvoiceMeta,
    pub items: Vec<LineItem>,
}

#[derive(Debug, Deserialize)]
pub struct Company {
    pub name: String,
    pub address: String,
    pub address2: String,
    pub city_state_zip: String,
    pub country: String,
}

#[derive(Debug, Deserialize)]
pub struct Client {
    pub name: String,
    pub address: String,
    pub address2: String,
    pub city_state_zip: String,
    pub country: String,
    pub tax_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct InvoiceMeta {
    pub number: String,
    pub date: String,
    pub due_date: String,
    pub currency: String,
}

#[derive(Debug, Deserialize)]
pub struct LineItem {
    pub description: String,
    pub rate: f64,
}

impl GenericInvoice {
    fn total(&self) -> f64 {
        self.items.iter().map(|item| item.rate).sum()
    }

    pub fn to_typst_dict(&self) -> String {
        let items_str: Vec<String> = self
            .items
            .iter()
            .map(|item| {
                format!(
                    "(description: \"{}\", rate: {})",
                    escape_typst_string(&item.description),
                    item.rate
                )
            })
            .collect();

        format!(
            r#"#let invoice-data = (
  company: (
    name: "{}",
    address: "{}",
    address2: "{}",
    city_state_zip: "{}",
    country: "{}",
  ),
  client: (
    name: "{}",
    address: "{}",
    address2: "{}",
    city_state_zip: "{}",
    country: "{}",
    tax_id: {},
  ),
  invoice: (
    number: "{}",
    date: "{}",
    due_date: "{}",
    currency: "{}",
  ),
  items: ({},),
  total: {},
)
"#,
            escape_typst_string(&self.company.name),
            escape_typst_string(&self.company.address),
            escape_typst_string(&self.company.address2),
            escape_typst_string(&self.company.city_state_zip),
            escape_typst_string(&self.company.country),
            escape_typst_string(&self.client.name),
            escape_typst_string(&self.client.address),
            escape_typst_string(&self.client.address2),
            escape_typst_string(&self.client.city_state_zip),
            escape_typst_string(&self.client.country),
            typst_option(&self.client.tax_id),
            escape_typst_string(&self.invoice.number),
            escape_typst_string(&self.invoice.date),
            escape_typst_string(&self.invoice.due_date),
            escape_typst_string(&self.invoice.currency),
            items_str.join(", "),
            self.total(),
        )
    }
}
