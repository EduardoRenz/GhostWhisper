//! Download e verificação de modelos (Silero VAD e whisper.cpp).

use std::fs;
use std::io::copy;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::error::{GhostWhisperError, Result};
use crate::ports::model_store::ModelStore;

/// Especificação de um modelo a baixar.
#[derive(Debug, Clone, Copy)]
pub struct ModelSpec {
    pub file_name: &'static str,
    pub url: &'static str,
    /// SHA-256 esperado (hex). `None` quando o provedor não garante hash estável.
    pub sha256: Option<&'static str>,
}

/// Modelo Silero VAD usado pelo whisper.cpp (MIT).
pub const SILERO_VAD: ModelSpec = ModelSpec {
    file_name: "ggml-silero-v5.1.2.bin",
    url: "https://huggingface.co/ggml-org/whisper-vad/resolve/9ffd54a1e1ee413ddf265af9913beaf518d1639b/ggml-silero-v5.1.2.bin",
    sha256: Some("29940d98d42b91fbd05ce489f3ecf7c72f0a42f027e4875919a28fb4c04ea2cf"),
};

/// Especificação dos modelos whisper.cpp suportados.
pub fn whisper_spec(name: &str) -> Option<ModelSpec> {
    let (file_name, url): (&'static str, &'static str) = match name {
        "tiny" => (
            "ggml-tiny.bin",
            concat!(
                "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/",
                "ggml-tiny.bin"
            ),
        ),
        "base" => (
            "ggml-base.bin",
            concat!(
                "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/",
                "ggml-base.bin"
            ),
        ),
        "small" => (
            "ggml-small.bin",
            concat!(
                "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/",
                "ggml-small.bin"
            ),
        ),
        "medium" => (
            "ggml-medium.bin",
            concat!(
                "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/",
                "ggml-medium.bin"
            ),
        ),
        _ => return None,
    };

    Some(ModelSpec {
        file_name,
        url,
        sha256: None,
    })
}

/// SHA-256 (hex) de um buffer.
pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

/// SHA-256 (hex) de um arquivo.
pub fn sha256_file(path: &Path) -> Result<String> {
    let bytes = fs::read(path)?;
    Ok(sha256_hex(&bytes))
}

/// Armazena modelos num diretório local, baixando sob demanda.
pub struct FileModelStore {
    base_dir: PathBuf,
}

impl FileModelStore {
    /// Cria o store apontando para `base_dir` (criado se necessário).
    pub fn new(base_dir: PathBuf) -> Self {
        Self { base_dir }
    }

    fn ensure(&self, spec: &ModelSpec) -> Result<PathBuf> {
        fs::create_dir_all(&self.base_dir)?;
        let path = self.base_dir.join(spec.file_name);

        if path.exists() {
            if let Some(expected) = spec.sha256 {
                if sha256_file(&path).ok().as_deref() == Some(expected) {
                    return Ok(path);
                }
                // Arquivo corrompido/desatualizado: refaz o download.
                let _ = fs::remove_file(&path);
            } else {
                return Ok(path);
            }
        }

        self.download(spec, &path)?;

        if let Some(expected) = spec.sha256 {
            if sha256_file(&path).ok().as_deref() != Some(expected) {
                let _ = fs::remove_file(&path);
                return Err(GhostWhisperError::Other(format!(
                    "hash inválido ao baixar {}",
                    spec.file_name
                )));
            }
        }

        Ok(path)
    }

    fn download(&self, spec: &ModelSpec, path: &Path) -> Result<()> {
        let response = ureq::get(spec.url).call().map_err(|e| {
            GhostWhisperError::Other(format!("falha ao baixar {}: {e}", spec.file_name))
        })?;

        let tmp = path.with_extension("part");
        let mut reader = response.into_reader();
        let mut file = fs::File::create(&tmp)?;
        copy(&mut reader, &mut file)?;
        drop(file);
        fs::rename(&tmp, path)?;
        Ok(())
    }
}

impl ModelStore for FileModelStore {
    fn vad_model_path(&self) -> Result<PathBuf> {
        self.ensure(&SILERO_VAD)
    }

    fn whisper_model_path(&self, name: &str) -> Result<PathBuf> {
        let spec = whisper_spec(name).ok_or_else(|| {
            GhostWhisperError::Other(format!("modelo whisper desconhecido: {name}"))
        })?;
        self.ensure(&spec)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_matches_known_vector() {
        // SHA-256 de "abc".
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn sha256_of_empty() {
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn known_whisper_models_resolve() {
        for name in ["tiny", "base", "small", "medium"] {
            assert!(whisper_spec(name).is_some(), "modelo {name}");
        }
        assert!(whisper_spec("huge").is_none());
    }

    #[test]
    fn returns_existing_file_without_download() {
        let dir = tempfile::tempdir().expect("tempdir");
        let spec = ModelSpec {
            file_name: "local.bin",
            url: "https://example.invalid/local.bin",
            sha256: None,
        };
        let path = dir.path().join(spec.file_name);
        fs::write(&path, b"conteudo").unwrap();

        let store = FileModelStore::new(dir.path().to_path_buf());
        let resolved = store.ensure(&spec).expect("deve reusar o arquivo");
        // Não tentou baixar de uma URL inválida.
        assert_eq!(resolved, path);
    }
}
