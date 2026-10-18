//! Porta de **detecção de GPU** (para sugerir o modelo local adequado).
//!
//! Não acelera nada por si: só informa o que a máquina tem, para a UI sugerir um
//! modelo. A aceleração real por GPU é o entregável E3 do PRD
//! `precision-and-performance` (BR-027).

use serde::Serialize;

/// Modelo recomendado quando há GPU **e** aceleração disponível no build.
pub const GPU_RECOMMENDED_MODEL: &str = "large-v3-turbo";
/// Modelo recomendado quando não há GPU utilizável (caminho CPU).
pub const CPU_RECOMMENDED_MODEL: &str = "tiny";

/// Resultado da detecção de GPU e a sugestão de modelo que vem dela.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GpuInfo {
    /// Existe GPU de hardware utilizável?
    pub available: bool,
    /// Backend/vendor detectado (`vulkan`, `drm`, `none`).
    pub backend: String,
    /// Detalhe legível (driver/arquivo ou o motivo da ausência).
    pub detail: String,
    /// O build atual acelera por GPU? (chega no E3).
    pub acceleration_enabled: bool,
    /// Modelo whisper sugerido para este hardware.
    pub recommended_model: String,
}

impl GpuInfo {
    /// Monta o resultado já com o modelo sugerido.
    pub fn new(
        available: bool,
        backend: impl Into<String>,
        detail: impl Into<String>,
        acceleration_enabled: bool,
    ) -> Self {
        let recommended_model = if available && acceleration_enabled {
            GPU_RECOMMENDED_MODEL
        } else {
            CPU_RECOMMENDED_MODEL
        };
        Self {
            available,
            backend: backend.into(),
            detail: detail.into(),
            acceleration_enabled,
            recommended_model: recommended_model.to_string(),
        }
    }

    /// Nada de GPU detectada.
    pub fn none(detail: impl Into<String>, acceleration_enabled: bool) -> Self {
        Self::new(false, "none", detail, acceleration_enabled)
    }
}

/// Descobre a GPU local.
pub trait GpuProbe: Send + Sync {
    /// Executa a detecção (barata; pode ser chamada ao abrir as configurações).
    fn probe(&self) -> GpuInfo;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recommends_the_large_model_only_with_acceleration() {
        let with_gpu = GpuInfo::new(true, "vulkan", "AMD (RADV)", true);
        assert_eq!(with_gpu.recommended_model, GPU_RECOMMENDED_MODEL);

        // GPU presente mas build CPU: continua recomendando o modelo leve.
        let without_accel = GpuInfo::new(true, "vulkan", "AMD (RADV)", false);
        assert_eq!(without_accel.recommended_model, CPU_RECOMMENDED_MODEL);

        let none = GpuInfo::none("sem GPU", false);
        assert!(!none.available);
        assert_eq!(none.backend, "none");
        assert_eq!(none.recommended_model, CPU_RECOMMENDED_MODEL);
    }
}
