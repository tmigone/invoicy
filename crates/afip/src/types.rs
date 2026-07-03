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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum Concepto {
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum DocTipo {
    Cuit = 80,
    Cuil = 86,
    Dni = 96,
    ConsumidorFinal = 99,
}

impl DocTipo {
    pub fn code(self) -> u8 {
        self as u8
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
    /// Receptor's condición frente al IVA (`CondicionIVAReceptorId`),
    /// mandatory since RG 5616/2024. `5` = consumidor final.
    pub condicion_iva_receptor: u8,
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
            condicion_iva_receptor: 5,
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
