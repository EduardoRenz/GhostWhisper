//! Porta do model manager do classificador (BR-099).
//!
//! O usuário final não tem Python nem repositório git: o app baixa o artefato
//! ONNX versionado, confere SHA-256, instala atomicamente e ativa uma versão.
//! Nenhum download acontece sem uma chamada explícita de `install`.

use std::path::PathBuf;

use crate::domain::classifier_model::{
    ClassifierModelCatalog, ClassifierModelManifest, InstalledClassifierModel,
};

/// Falhas tipadas do model manager.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ClassifierModelError {
    #[error("catálogo de modelos indisponível: {0}")]
    Unavailable(String),
    #[error("modelo desconhecido no catálogo: {0}")]
    UnknownModel(String),
    #[error("modelo não instalado: {model_id} {version}")]
    NotInstalled { model_id: String, version: String },
    #[error("manifesto inválido: {0}")]
    InvalidManifest(String),
    #[error("checksum inválido em {file}")]
    ChecksumMismatch { file: String },
    #[error("espaço insuficiente para instalar o modelo")]
    NoSpace,
    /// Reservado para o cancelamento explícito (não usado no MVP).
    #[allow(dead_code)]
    #[error("download cancelado")]
    Cancelled,
    #[error("a versão ativa não pode ser removida")]
    ActiveModel,
    #[error("erro de E/S: {0}")]
    Io(String),
}

impl From<std::io::Error> for ClassifierModelError {
    fn from(error: std::io::Error) -> Self {
        map_io_error(error)
    }
}

/// Converte E/S em erro tipado (ENOSPC vira `NoSpace`).
pub fn map_io_error(error: std::io::Error) -> ClassifierModelError {
    // ENOSPC em Linux/macOS; 28 no Windows também é ERROR_HANDLE_DISK_FULL.
    if error.raw_os_error() == Some(28) {
        return ClassifierModelError::NoSpace;
    }
    ClassifierModelError::Io(error.to_string())
}

/// Catálogo, instalação e ativação de artefatos do classificador.
///
/// Adaptador: `LocalClassifierModelManager`; dobra de teste: implementação em
/// memória no próprio adaptador.
pub trait ClassifierModelManager: Send + Sync {
    /// Lê o catálogo publicado (rede). Nunca baixa o modelo.
    fn catalog(&self) -> Result<ClassifierModelCatalog, ClassifierModelError>;

    /// Versões instaladas no diretório local.
    fn installed(&self) -> Result<Vec<InstalledClassifierModel>, ClassifierModelError>;

    /// Versão ativa (ponteiro `current`), se houver.
    fn active(&self) -> Option<InstalledClassifierModel>;

    /// Diretório da versão ativa (atalho para o adaptador do classificador).
    #[allow(dead_code)]
    fn active_path(&self) -> Option<PathBuf> {
        self.active().map(|model| PathBuf::from(model.path))
    }

    /// Manifesto de uma versão instalada (usado pelo adaptador Laya).
    #[allow(dead_code)]
    fn manifest(
        &self,
        model_id: &str,
        version: &str,
    ) -> Result<ClassifierModelManifest, ClassifierModelError>;

    /// Baixa, verifica e instala **sem** ativar.
    ///
    /// É a única operação que faz download; chamadas repetidas reutilizam uma
    /// instalação verificada.
    fn install(
        &self,
        model_id: &str,
        version: &str,
    ) -> Result<InstalledClassifierModel, ClassifierModelError>;

    /// Aponta `current` para uma versão instalada e verificada.
    fn activate(
        &self,
        model_id: &str,
        version: &str,
    ) -> Result<InstalledClassifierModel, ClassifierModelError>;

    /// Remove uma versão instalada (a ativa é protegida).
    fn remove(&self, model_id: &str, version: &str) -> Result<(), ClassifierModelError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enospc_maps_to_no_space() {
        let error = std::io::Error::from_raw_os_error(28);
        assert_eq!(map_io_error(error), ClassifierModelError::NoSpace);
    }

    #[test]
    fn other_io_errors_keep_the_message() {
        let error = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "sem permissão");
        assert!(matches!(
            map_io_error(error),
            ClassifierModelError::Io(message) if message.contains("permissão")
        ));
    }

    #[test]
    fn errors_are_human_readable() {
        assert_eq!(
            ClassifierModelError::ChecksumMismatch {
                file: "model.onnx".to_string()
            }
            .to_string(),
            "checksum inválido em model.onnx"
        );
        assert_eq!(
            ClassifierModelError::NotInstalled {
                model_id: "laya".to_string(),
                version: "0.1.0".to_string()
            }
            .to_string(),
            "modelo não instalado: laya 0.1.0"
        );
    }
}
