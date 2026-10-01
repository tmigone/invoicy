use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct GenericInvoice {
    pub company: Company,
    pub client: Client,
    pub invoice: InvoiceMeta,
    pub items: Vec<LineItem>,
}

impl GenericInvoice {
    pub fn total(&self) -> f64 {
        self.items.iter().map(|item| item.rate).sum()
    }
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct Company {
    pub name: String,
    pub address: String,
    pub address2: String,
    pub city_state_zip: String,
    pub country: String,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct Client {
    pub name: String,
    pub address: String,
    pub address2: String,
    pub city_state_zip: String,
    pub country: String,
    pub tax_id: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct InvoiceMeta {
    pub number: String,
    pub date: String,
    pub due_date: String,
    pub currency: String,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct LineItem {
    pub description: String,
    pub rate: f64,
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
}
