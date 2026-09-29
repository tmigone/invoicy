use serde::{Deserialize, Serialize};

/// Voucher / comprobante type codes (ARCA "CbteTipo").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u16)]
pub enum VoucherType {
    FacturaC = 11,
    NotaDebitoC = 12,
    NotaCreditoC = 13,
}

impl VoucherType {
    pub fn code(self) -> u16 {
        self as u16
    }
}

/// "Concepto" — what is being billed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum Concepto {
    #[default]
    Productos = 1,
    Servicios = 2,
    ProductosYServicios = 3,
}

impl Concepto {
    pub fn code(self) -> u8 {
        self as u8
    }

    /// Services (and mixed) require service-period + payment-due dates.
    pub fn requires_service_dates(self) -> bool {
        matches!(self, Concepto::Servicios | Concepto::ProductosYServicios)
    }
}

/// Receptor document type. `99` / `0` is the anonymous "consumidor final".
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum DocTipo {
    Cuit = 80,
    Cuil = 86,
    Dni = 96,
    #[default]
    ConsumidorFinal = 99,
}

impl DocTipo {
    pub fn code(self) -> u8 {
        self as u8
    }

    /// Description as listed by `FEParamGetTiposDoc`.
    pub fn label(self) -> &'static str {
        match self {
            DocTipo::Cuit => "CUIT",
            DocTipo::Cuil => "CUIL",
            DocTipo::Dni => "DNI",
            DocTipo::ConsumidorFinal => "Doc. (Otro)",
        }
    }
}

/// Receptor's condición frente al IVA (`CondicionIVAReceptorId`), mandatory
/// on every voucher since RG 5616/2024. Codes and descriptions as listed by
/// `FEParamGetCondicionIvaReceptor`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum CondicionIva {
    ResponsableInscripto = 1,
    Exento = 4,
    #[default]
    ConsumidorFinal = 5,
    Monotributo = 6,
    NoCategorizado = 7,
    ProveedorDelExterior = 8,
    ClienteDelExterior = 9,
    Liberado = 10,
    MonotributistaSocial = 13,
    NoAlcanzado = 15,
    MonotributoTrabajadorIndependientePromovido = 16,
}

impl CondicionIva {
    pub fn code(self) -> u8 {
        self as u8
    }

    /// Description as printed on the voucher.
    pub fn label(self) -> &'static str {
        match self {
            CondicionIva::ResponsableInscripto => "IVA Responsable Inscripto",
            CondicionIva::Exento => "IVA Sujeto Exento",
            CondicionIva::ConsumidorFinal => "Consumidor Final",
            CondicionIva::Monotributo => "Responsable Monotributo",
            CondicionIva::NoCategorizado => "Sujeto No Categorizado",
            CondicionIva::ProveedorDelExterior => "Proveedor del Exterior",
            CondicionIva::ClienteDelExterior => "Cliente del Exterior",
            CondicionIva::Liberado => "IVA Liberado – Ley N° 19.640",
            CondicionIva::MonotributistaSocial => "Monotributista Social",
            CondicionIva::NoAlcanzado => "IVA No Alcanzado",
            CondicionIva::MonotributoTrabajadorIndependientePromovido => {
                "Monotributo Trabajador Independiente Promovido"
            }
        }
    }
}

/// A Factura C request. Amounts are in pesos; for monotributo there is no
/// discriminated IVA so the net equals the total.
#[derive(Debug, Clone)]
pub struct FacturaC {
    pub concepto: Concepto,
    pub doc_tipo: DocTipo,
    /// Receptor document number (0 for consumidor final).
    pub doc_nro: u64,
    /// Total amount in pesos.
    pub importe_total: f64,
    /// Voucher date (YYYYMMDD). `None` uses today in AR timezone.
    pub fecha: Option<u32>,
    /// Service period start (YYYYMMDD), required for service concepts.
    pub fecha_servicio_desde: Option<u32>,
    /// Service period end (YYYYMMDD), required for service concepts.
    pub fecha_servicio_hasta: Option<u32>,
    /// Payment due date (YYYYMMDD), required for service concepts.
    pub fecha_vto_pago: Option<u32>,
    /// Receptor's condición frente al IVA.
    pub condicion_iva_receptor: CondicionIva,
}

impl FacturaC {
    /// A simple Factura C to consumidor final for `importe` pesos.
    pub fn consumidor_final(importe: f64) -> Self {
        Self {
            concepto: Concepto::Productos,
            doc_tipo: DocTipo::ConsumidorFinal,
            doc_nro: 0,
            importe_total: importe,
            fecha: None,
            fecha_servicio_desde: None,
            fecha_servicio_hasta: None,
            fecha_vto_pago: None,
            condicion_iva_receptor: CondicionIva::ConsumidorFinal,
        }
    }
}

/// A voucher as returned by `FECompConsultar` (query by number).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoucherInfo {
    pub numero: u64,
    pub tipo: u16,
    pub punto_venta: u32,
    /// Voucher date (YYYYMMDD).
    pub fecha: u32,
    pub importe_total: f64,
    pub doc_tipo: u16,
    pub doc_nro: u64,
    /// CAE (`CodAutorizacion`), if authorized.
    pub cae: Option<String>,
    /// CAE expiration (`FchVto`, YYYYMMDD), if authorized.
    pub cae_vencimiento: Option<String>,
    /// `A` (aprobado), `R` (rechazado), `P` (parcial), or empty.
    pub resultado: String,
}

/// Successful authorization result from `FECAESolicitar`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaeResult {
    /// Authorization code.
    pub cae: String,
    /// CAE expiration date (YYYYMMDD).
    pub cae_vencimiento: String,
    /// Assigned voucher number.
    pub numero: u64,
    /// Punto de venta.
    pub punto_venta: u32,
    /// Voucher type code.
    pub tipo: u16,
    /// Voucher date (YYYYMMDD).
    pub fecha: u32,
    /// Total amount.
    pub importe_total: f64,
    /// Non-fatal observations returned by ARCA, if any.
    pub observaciones: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn condicion_iva_codes_and_names() {
        assert_eq!(CondicionIva::default(), CondicionIva::ConsumidorFinal);
        assert_eq!(CondicionIva::ConsumidorFinal.code(), 5);
        assert_eq!(CondicionIva::ResponsableInscripto.code(), 1);
        assert_eq!(CondicionIva::Monotributo.code(), 6);
        let parsed: CondicionIva = serde_json::from_str("\"responsable_inscripto\"").unwrap();
        assert_eq!(parsed, CondicionIva::ResponsableInscripto);
    }
}
