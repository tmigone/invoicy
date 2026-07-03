use schemars::JsonSchema;
use serde::Deserialize;

use crate::typst::{escape_string, option_string};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GenericInvoice {
    pub company: Company,
    pub client: Client,
    pub invoice: InvoiceMeta,
    pub items: Vec<LineItem>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct Company {
    pub name: String,
    pub address: String,
    pub address2: String,
    pub city_state_zip: String,
    pub country: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct Client {
    pub name: String,
    pub address: String,
    pub address2: String,
    pub city_state_zip: String,
    pub country: String,
    pub tax_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct InvoiceMeta {
    pub number: String,
    pub date: String,
    pub due_date: String,
    pub currency: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
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
                    escape_string(&item.description),
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
            escape_string(&self.company.name),
            escape_string(&self.company.address),
            escape_string(&self.company.address2),
            escape_string(&self.company.city_state_zip),
            escape_string(&self.company.country),
            escape_string(&self.client.name),
            escape_string(&self.client.address),
            escape_string(&self.client.address2),
            escape_string(&self.client.city_state_zip),
            escape_string(&self.client.country),
            option_string(&self.client.tax_id),
            escape_string(&self.invoice.number),
            escape_string(&self.invoice.date),
            escape_string(&self.invoice.due_date),
            escape_string(&self.invoice.currency),
            items_str.join(", "),
            self.total(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_invoice() -> GenericInvoice {
        GenericInvoice {
            company: Company {
                name: "Acme Corp".into(),
                address: "123 Main St".into(),
                address2: "Suite 100".into(),
                city_state_zip: "New York, NY 10001".into(),
                country: "USA".into(),
            },
            client: Client {
                name: "Client Inc".into(),
                address: "456 Oak Ave".into(),
                address2: "".into(),
                city_state_zip: "Los Angeles, CA 90001".into(),
                country: "USA".into(),
                tax_id: Some("12-3456789".into()),
            },
            invoice: InvoiceMeta {
                number: "INV-001".into(),
                date: "2025-01-01".into(),
                due_date: "2025-01-15".into(),
                currency: "USD".into(),
            },
            items: vec![
                LineItem {
                    description: "Consulting".into(),
                    rate: 1000.0,
                },
                LineItem {
                    description: "Development".into(),
                    rate: 2000.0,
                },
            ],
        }
    }

    #[test]
    fn total_sums_items() {
        let invoice = sample_invoice();
        assert_eq!(invoice.total(), 3000.0);
    }

    #[test]
    fn to_typst_dict_contains_company() {
        let invoice = sample_invoice();
        let output = invoice.to_typst_dict();
        assert!(output.contains(r#"name: "Acme Corp""#));
        assert!(output.contains(r#"address: "123 Main St""#));
    }

    #[test]
    fn to_typst_dict_contains_client() {
        let invoice = sample_invoice();
        let output = invoice.to_typst_dict();
        assert!(output.contains(r#"name: "Client Inc""#));
        assert!(output.contains(r#"tax_id: "12-3456789""#));
    }

    #[test]
    fn to_typst_dict_contains_items() {
        let invoice = sample_invoice();
        let output = invoice.to_typst_dict();
        assert!(output.contains(r#"description: "Consulting""#));
        assert!(output.contains("rate: 1000"));
        assert!(output.contains(r#"description: "Development""#));
    }

    #[test]
    fn to_typst_dict_contains_total() {
        let invoice = sample_invoice();
        let output = invoice.to_typst_dict();
        assert!(output.contains("total: 3000"));
    }

    #[test]
    fn to_typst_dict_none_tax_id() {
        let mut invoice = sample_invoice();
        invoice.client.tax_id = None;
        let output = invoice.to_typst_dict();
        assert!(output.contains("tax_id: none"));
    }

    #[test]
    fn to_typst_dict_escapes_quotes() {
        let mut invoice = sample_invoice();
        invoice.company.name = r#"Acme "Best" Corp"#.into();
        let output = invoice.to_typst_dict();
        assert!(output.contains(r#"name: "Acme \"Best\" Corp""#));
    }
}
