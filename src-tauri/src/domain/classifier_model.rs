//! Manifesto e catálogo do modelo do classificador (BR-098/BR-099).
//!
//! O artefato é um diretório ONNX versionado, publicado fora do binário. O
//! manifesto descreve **todos** os arquivos com tamanho e SHA-256; o model
//! manager só ativa um diretório cujos arquivos conferem. Regras puras (sem
//! I/O) para que a instalação seja testável sem rede.

use std::path::{Component, Path};

use serde::{Deserialize, Serialize};

/// Versão de schema aceita pelo app.
pub const CLASSIFIER_MODEL_SCHEMA: u32 = 1;

/// Tamanho máximo aceito para um caminho relativo do manifesto.
const MAX_PATH_CHARS: usize = 240;

/// Um arquivo do artefato.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClassifierModelFile {
    /// Caminho relativo dentro do diretório do modelo (`tokenizer/tokenizer.json`).
    pub path: String,
    /// URL absoluta para download.
    pub url: String,
    /// Tamanho esperado em bytes (`0` = não verificado por tamanho).
    #[serde(default)]
    pub size_bytes: u64,
    /// SHA-256 em hex minúsculo.
    pub sha256: String,
}

/// Calibração publicada junto do checkpoint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ClassifierModelCalibration {
    /// Limiar sugerido para `should_suggest`.
    #[serde(default)]
    pub threshold: Option<f32>,
    /// Erro esperado de calibração (ECE).
    #[serde(default)]
    pub ece: Option<f32>,
    #[serde(default)]
    pub notes: Option<String>,
}

/// Métricas do eval que acompanham o artefato.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ClassifierModelMetrics {
    #[serde(default)]
    pub accuracy: Option<f32>,
    #[serde(default)]
    pub false_positive_rate: Option<f32>,
    #[serde(default)]
    pub p50_ms: Option<u64>,
    #[serde(default)]
    pub p95_ms: Option<u64>,
}

/// Manifesto de uma versão do modelo.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClassifierModelManifest {
    pub schema: u32,
    /// Ex.: `laya-multilingual`.
    pub model_id: String,
    /// Ex.: `0.1.0`.
    pub version: String,
    /// Ex.: `multilingual`, `english`.
    pub checkpoint: String,
    /// Ex.: `onnx`.
    #[serde(default = "default_layout")]
    pub layout: String,
    /// Rótulo amigável (opcional; a UI cai no `model_id`).
    #[serde(default)]
    pub label: Option<String>,
    /// Soma dos arquivos (bytes).
    #[serde(default)]
    pub size_bytes: u64,
    #[serde(default)]
    pub license: String,
    pub files: Vec<ClassifierModelFile>,
    #[serde(default)]
    pub calibration: ClassifierModelCalibration,
    #[serde(default)]
    pub metrics: ClassifierModelMetrics,
    #[serde(default)]
    pub created_at: String,
}

fn default_layout() -> String {
    "onnx".to_string()
}

/// Catálogo publicado (`catalog.json`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClassifierModelCatalog {
    pub schema: u32,
    /// Ordenado: o primeiro é o recomendado.
    pub models: Vec<ClassifierModelManifest>,
}

/// Versão instalada no disco.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstalledClassifierModel {
    pub model_id: String,
    pub version: String,
    pub checkpoint: String,
    /// Caminho absoluto do diretório do modelo.
    pub path: String,
    pub size_bytes: u64,
    /// `true` quando o ponteiro `current` aponta para esta versão.
    pub active: bool,
    /// `true` quando o manifesto e os arquivos conferem.
    pub verified: bool,
    pub installed_at_ms: i64,
}

/// Falhas de validação do manifesto.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ClassifierManifestError {
    #[error("schema de modelo não suportado: {0}")]
    UnsupportedSchema(u32),
    #[error("campo obrigatório vazio: {0}")]
    Empty(&'static str),
    #[error("manifesto sem arquivos")]
    NoFiles,
    #[error("catálogo sem modelos")]
    EmptyCatalog,
    #[error("caminho inválido no manifesto: {0}")]
    UnsafePath(String),
    #[error("hash inválido para {0}")]
    InvalidHash(String),
    #[error("arquivo repetido no manifesto: {0}")]
    DuplicateFile(String),
}

impl ClassifierModelManifest {
    /// Valida schema, identificadores, caminhos e hashes.
    pub fn validate(&self) -> Result<(), ClassifierManifestError> {
        if self.schema != CLASSIFIER_MODEL_SCHEMA {
            return Err(ClassifierManifestError::UnsupportedSchema(self.schema));
        }
        for (field, value) in [
            ("model_id", self.model_id.as_str()),
            ("version", self.version.as_str()),
            ("checkpoint", self.checkpoint.as_str()),
        ] {
            if !is_safe_id(value) {
                return Err(ClassifierManifestError::Empty(field));
            }
        }
        if self.files.is_empty() {
            return Err(ClassifierManifestError::NoFiles);
        }

        let mut seen = std::collections::HashSet::new();
        for file in &self.files {
            if !is_safe_relative_path(&file.path) {
                return Err(ClassifierManifestError::UnsafePath(file.path.clone()));
            }
            if !seen.insert(file.path.as_str()) {
                return Err(ClassifierManifestError::DuplicateFile(file.path.clone()));
            }
            if !is_sha256(&file.sha256) {
                return Err(ClassifierManifestError::InvalidHash(file.path.clone()));
            }
        }
        Ok(())
    }

    /// Soma dos arquivos (usa `size_bytes` do manifesto ou a soma dos arquivos).
    pub fn total_size_bytes(&self) -> u64 {
        if self.size_bytes > 0 {
            return self.size_bytes;
        }
        self.files.iter().map(|file| file.size_bytes).sum()
    }

    /// Nome do diretório da versão (`<model_id>-<version>`).
    pub fn directory_name(&self) -> String {
        format!("{}-{}", self.model_id, self.version)
    }
}

impl ClassifierModelCatalog {
    /// Valida o catálogo inteiro (schema + cada manifesto).
    pub fn validate(&self) -> Result<(), ClassifierManifestError> {
        if self.schema != CLASSIFIER_MODEL_SCHEMA {
            return Err(ClassifierManifestError::UnsupportedSchema(self.schema));
        }
        if self.models.is_empty() {
            return Err(ClassifierManifestError::EmptyCatalog);
        }
        for model in &self.models {
            model.validate()?;
        }
        Ok(())
    }

    /// Primeiro modelo do catálogo (recomendado).
    ///
    /// A UI lê `models[0]`; o helper fica para o eval/CLI e para testes.
    #[allow(dead_code)]
    pub fn recommended(&self) -> Option<&ClassifierModelManifest> {
        self.models.first()
    }

    /// Busca por `model_id` + `version`.
    pub fn find(&self, model_id: &str, version: &str) -> Option<&ClassifierModelManifest> {
        self.models
            .iter()
            .find(|model| model.model_id == model_id && model.version == version)
    }
}

/// Identificador seguro para nome de diretório (sem separadores).
pub fn is_safe_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
}

/// Caminho relativo seguro: sem raiz, sem `..`, sem prefixo de disco.
pub fn is_safe_relative_path(value: &str) -> bool {
    if value.is_empty() || value.len() > MAX_PATH_CHARS {
        return false;
    }
    if value.contains('\\') || value.starts_with('/') {
        return false;
    }
    let path = Path::new(value);
    path.components()
        .all(|component| matches!(component, Component::Normal(_)))
}

/// SHA-256 em hex minúsculo.
pub fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(path: &str) -> ClassifierModelFile {
        ClassifierModelFile {
            path: path.to_string(),
            url: format!("https://example.invalid/{path}"),
            size_bytes: 10,
            sha256: "a".repeat(64),
        }
    }

    fn manifest() -> ClassifierModelManifest {
        ClassifierModelManifest {
            schema: CLASSIFIER_MODEL_SCHEMA,
            model_id: "laya-multilingual".to_string(),
            version: "0.1.0".to_string(),
            checkpoint: "multilingual".to_string(),
            layout: "onnx".to_string(),
            label: None,
            size_bytes: 0,
            license: "Apache-2.0".to_string(),
            files: vec![file("model.onnx"), file("tokenizer/tokenizer.json")],
            calibration: ClassifierModelCalibration::default(),
            metrics: ClassifierModelMetrics::default(),
            created_at: "2026-10-26".to_string(),
        }
    }

    #[test]
    fn parses_a_catalog_and_finds_the_recommended_model() {
        let catalog: ClassifierModelCatalog = serde_json::from_str(
            r#"{
              "schema": 1,
              "models": [{
                "schema": 1,
                "model_id": "laya-multilingual",
                "version": "0.1.0",
                "checkpoint": "multilingual",
                "files": [{
                  "path": "model.onnx",
                  "url": "https://example.invalid/model.onnx",
                  "size_bytes": 10,
                  "sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                }]
              }]
            }"#,
        )
        .expect("catálogo");

        assert!(catalog.validate().is_ok());
        assert_eq!(
            catalog.recommended().map(|model| model.model_id.as_str()),
            Some("laya-multilingual")
        );
        assert_eq!(
            catalog
                .find("laya-multilingual", "0.1.0")
                .map(|m| m.layout.as_str()),
            Some("onnx")
        );
        assert!(catalog.find("laya-multilingual", "9.9.9").is_none());
    }

    #[test]
    fn rejects_path_traversal_and_absolute_paths() {
        for unsafe_path in ["../secret", "/etc/passwd", "a/../../b", "a\\b"] {
            let mut manifest = manifest();
            manifest.files = vec![file(unsafe_path)];
            assert!(
                matches!(
                    manifest.validate(),
                    Err(ClassifierManifestError::UnsafePath(_))
                ),
                "caminho {unsafe_path}"
            );
        }
    }

    #[test]
    fn rejects_short_or_uppercase_hashes() {
        let mut too_short = manifest();
        too_short.files[0].sha256 = "abc".to_string();
        assert!(matches!(
            too_short.validate(),
            Err(ClassifierManifestError::InvalidHash(_))
        ));

        let mut uppercase = manifest();
        uppercase.files[0].sha256 = "A".repeat(64);
        assert!(matches!(
            uppercase.validate(),
            Err(ClassifierManifestError::InvalidHash(_))
        ));
    }

    #[test]
    fn rejects_empty_files_and_duplicates() {
        let mut empty = manifest();
        empty.files.clear();
        assert_eq!(empty.validate(), Err(ClassifierManifestError::NoFiles));

        let mut duplicate = manifest();
        duplicate.files = vec![file("model.onnx"), file("model.onnx")];
        assert!(matches!(
            duplicate.validate(),
            Err(ClassifierManifestError::DuplicateFile(_))
        ));
    }

    #[test]
    fn rejects_unsafe_model_ids() {
        for value in ["", "../x", "a/b", "a b"] {
            let mut manifest = manifest();
            manifest.model_id = value.to_string();
            assert!(manifest.validate().is_err(), "id {value}");
        }
    }

    #[test]
    fn directory_name_and_total_size_are_derived() {
        let manifest = manifest();
        assert_eq!(manifest.directory_name(), "laya-multilingual-0.1.0");
        assert_eq!(manifest.total_size_bytes(), 20);

        let mut with_declared = manifest;
        with_declared.size_bytes = 1_000;
        assert_eq!(with_declared.total_size_bytes(), 1_000);
    }

    #[test]
    fn installed_model_serializes_for_the_ui() {
        let installed = InstalledClassifierModel {
            model_id: "laya-multilingual".to_string(),
            version: "0.1.0".to_string(),
            checkpoint: "multilingual".to_string(),
            path: "/tmp/model".to_string(),
            size_bytes: 10,
            active: true,
            verified: true,
            installed_at_ms: 1,
        };
        let json = serde_json::to_value(installed).expect("json");
        assert_eq!(json["active"], true);
        assert_eq!(json["model_id"], "laya-multilingual");
    }
}
