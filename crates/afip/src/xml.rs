//! Small XML helpers over `roxmltree`.
//!
//! ARCA's SOAP responses vary in namespace prefixes, so lookups here match on
//! the element's *local* name and ignore namespaces.

use roxmltree::Document;

use crate::error::{Error, Result};

/// Return the text content of the first element with local name `local`.
pub fn first_text(xml: &str, local: &str) -> Option<String> {
    let doc = Document::parse(xml).ok()?;
    doc.descendants()
        .find(|n| n.is_element() && n.tag_name().name() == local)
        .and_then(|n| n.text().map(str::to_owned))
}

/// For each element with local name `parent`, return a lookup of its direct
/// child `(local_name -> text)`. Used to read repeated records like
/// `<Obs><Code/><Msg/></Obs>` or `<Err>`.
pub fn records(xml: &str, parent: &str) -> Vec<Vec<(String, String)>> {
    let Ok(doc) = Document::parse(xml) else {
        return Vec::new();
    };
    doc.descendants()
        .filter(|n| n.is_element() && n.tag_name().name() == parent)
        .map(|n| {
            n.children()
                .filter(|c| c.is_element())
                .filter_map(|c| {
                    c.text()
                        .map(|t| (c.tag_name().name().to_owned(), t.to_owned()))
                })
                .collect()
        })
        .collect()
}

/// Return `Err(SoapFault)` if the envelope carries a SOAP `<Fault>`.
pub fn check_soap_fault(xml: &str, service: &str) -> Result<()> {
    let doc = Document::parse(xml)?;
    let fault = doc
        .descendants()
        .any(|n| n.is_element() && n.tag_name().name() == "Fault");
    if fault {
        let message = first_text(xml, "faultstring").unwrap_or_else(|| "unknown fault".into());
        return Err(Error::SoapFault {
            service: service.to_string(),
            message,
        });
    }
    Ok(())
}
