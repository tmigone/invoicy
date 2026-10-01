//! QR code images for the invoice footer.

use qrcode::render::svg;
use qrcode::{EcLevel, QrCode};

/// An SVG QR code encoding `text`. No quiet zone: the footer leaves white
/// space around it already, so the image lines up with the other elements.
pub(crate) fn svg(text: &str) -> Result<Vec<u8>, String> {
    let code = QrCode::with_error_correction_level(text, EcLevel::M)
        .map_err(|e| format!("cannot build QR code: {e}"))?;
    Ok(code
        .render::<svg::Color>()
        .quiet_zone(false)
        .build()
        .into_bytes())
}
