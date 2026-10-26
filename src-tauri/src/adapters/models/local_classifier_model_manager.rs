//! Model manager local do classificador (BR-099).
//!
//! Layout em `app_data/models/laya`:
//!
//! ```text
//! laya/
//!   current                         # nome do diretório ativo
//!   laya-multilingual-0.1.0/        # versão instalada e verificada
//!     manifest.json
//!     model.onnx
//!     tokenizer/tokenizer.json
//!   laya-multilingual-0.2.0.part/   # download em andamento (nunca ativo)
//! ```
//!
//! O download vai para `<versão>.part`, cada arquivo é conferido por tamanho e
//! SHA-256 e só então o diretório é renomeado (atômico, mesmo sistema de
//! arquivos). `current` é gravado por `current.part` + rename. Um erro em
//! qualquer etapa apaga o `.part` e **não** toca na versão ativa.

use std::fs::{self, File};
use std::io::{BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

use sha2::{Digest, Sha256};

use crate::domain::classifier_model::{
    is_safe_id, ClassifierModelCatalog, ClassifierModelManifest, InstalledClassifierModel,
};
use crate::ports::classifier_model_manager::{
    map_io_error, ClassifierModelError, ClassifierModelManager,
};
use crate::ports::clock::Clock;
use crate::ports::event_publisher::{
    EventPublisher, SuggestionModelDownloadEvent, SuggestionModelDownloadPhase,
};
use crate::ports::settings_store::SettingsStore;

/// Nome do manifesto gravado dentro da versão.
pub const MANIFEST_FILE: &str = "manifest.json";
/// Ponteiro para a versão ativa.
pub const CURRENT_FILE: &str = "current";
/// Sufixo do diretório de download.
pub const PART_SUFFIX: &str = ".part";
/// Teto do catálogo (2 MB) — evita baixar um corpo inesperado.
const CATALOG_MAX_BYTES: u64 = 2 * 1024 * 1024;
/// Tamanho do bloco de leitura no download/hash.
const CHUNK: usize = 64 * 1024;

/// Origem dos arquivos do modelo (rede no app; memória nos testes).
pub trait ClassifierModelSource: Send + Sync {
    /// Baixa um recurso pequeno para a memória (catálogo).
    fn fetch_bytes(&self, url: &str, max_bytes: u64) -> Result<Vec<u8>, ClassifierModelError>;
    /// Baixa para `dest`, informando o total acumulado do arquivo.
    fn download_to(
        &self,
        url: &str,
        dest: &Path,
        progress: &mut dyn FnMut(u64),
    ) -> Result<(), ClassifierModelError>;
}

/// Download HTTP com `ureq` (mesma biblioteca das fontes de IA).
///
/// Também aceita um caminho absoluto ou `file://` — útil para testar o fluxo
/// inteiro com um artefato local antes do servidor de downloads existir.
pub struct UreqClassifierModelSource;

/// Caminho local quando a URL é `file://` ou absoluta.
fn local_path(url: &str) -> Option<PathBuf> {
    if let Some(rest) = url.strip_prefix("file://") {
        return Some(PathBuf::from(rest));
    }
    let path = Path::new(url);
    if path.is_absolute() {
        Some(path.to_path_buf())
    } else {
        None
    }
}

impl ClassifierModelSource for UreqClassifierModelSource {
    fn fetch_bytes(&self, url: &str, max_bytes: u64) -> Result<Vec<u8>, ClassifierModelError> {
        if let Some(path) = local_path(url) {
            let metadata = fs::metadata(&path).map_err(map_io_error)?;
            if metadata.len() > max_bytes {
                return Err(ClassifierModelError::Unavailable(
                    "catálogo maior que o esperado".to_string(),
                ));
            }
            return fs::read(&path).map_err(map_io_error);
        }

        let response = ureq::get(url)
            .call()
            .map_err(|error| ClassifierModelError::Unavailable(error.to_string()))?;
        let mut reader = response.into_reader().take(max_bytes + 1);
        let mut buffer = Vec::new();
        reader
            .read_to_end(&mut buffer)
            .map_err(|error| ClassifierModelError::Io(error.to_string()))?;
        if buffer.len() as u64 > max_bytes {
            return Err(ClassifierModelError::Unavailable(
                "catálogo maior que o esperado".to_string(),
            ));
        }
        Ok(buffer)
    }

    fn download_to(
        &self,
        url: &str,
        dest: &Path,
        progress: &mut dyn FnMut(u64),
    ) -> Result<(), ClassifierModelError> {
        if let Some(path) = local_path(url) {
            let bytes = fs::read(&path).map_err(map_io_error)?;
            fs::write(dest, &bytes).map_err(map_io_error)?;
            progress(bytes.len() as u64);
            return Ok(());
        }

        let response = ureq::get(url)
            .call()
            .map_err(|error| ClassifierModelError::Unavailable(error.to_string()))?;
        let mut reader = response.into_reader();
        let mut file = File::create(dest).map_err(map_io_error)?;
        let mut buffer = vec![0_u8; CHUNK];
        let mut total = 0_u64;
        loop {
            let read = reader
                .read(&mut buffer)
                .map_err(|error| ClassifierModelError::Io(error.to_string()))?;
            if read == 0 {
                break;
            }
            file.write_all(&buffer[..read]).map_err(map_io_error)?;
            total += read as u64;
            progress(total);
        }
        file.flush().map_err(map_io_error)?;
        Ok(())
    }
}

/// Implementação em disco do model manager.
pub struct LocalClassifierModelManager {
    root: PathBuf,
    /// A URL do catálogo é uma configuração: muda sem reiniciar o app.
    settings: Arc<dyn SettingsStore>,
    source: Arc<dyn ClassifierModelSource>,
    publisher: Arc<dyn EventPublisher>,
    clock: Arc<dyn Clock>,
}

impl LocalClassifierModelManager {
    pub fn new(
        root: PathBuf,
        settings: Arc<dyn SettingsStore>,
        source: Arc<dyn ClassifierModelSource>,
        publisher: Arc<dyn EventPublisher>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            root,
            settings,
            source,
            publisher,
            clock,
        }
    }

    fn catalog_url(&self) -> String {
        self.settings
            .load()
            .map(|settings| settings.suggestion_classifier_model_catalog_url)
            .unwrap_or_default()
    }

    fn ensure_root(&self) -> Result<(), ClassifierModelError> {
        fs::create_dir_all(&self.root).map_err(map_io_error)
    }

    fn current_name(&self) -> Option<String> {
        let value = fs::read_to_string(self.root.join(CURRENT_FILE)).ok()?;
        let value = value.trim();
        if value.is_empty() || !is_safe_id(value) {
            return None;
        }
        Some(value.to_string())
    }

    fn version_dir(&self, model_id: &str, version: &str) -> Result<PathBuf, ClassifierModelError> {
        if !is_safe_id(model_id) || !is_safe_id(version) {
            return Err(ClassifierModelError::UnknownModel(format!(
                "{model_id} {version}"
            )));
        }
        Ok(self.root.join(format!("{model_id}-{version}")))
    }

    fn read_manifest(dir: &Path) -> Result<ClassifierModelManifest, ClassifierModelError> {
        let bytes = fs::read(dir.join(MANIFEST_FILE))
            .map_err(|error| ClassifierModelError::Io(error.to_string()))?;
        let manifest: ClassifierModelManifest = serde_json::from_slice(&bytes)
            .map_err(|error| ClassifierModelError::InvalidManifest(error.to_string()))?;
        manifest
            .validate()
            .map_err(|error| ClassifierModelError::InvalidManifest(error.to_string()))?;
        Ok(manifest)
    }

    fn installed_entry(
        &self,
        dir: &Path,
        manifest: &ClassifierModelManifest,
        current: Option<&str>,
        now_ms: i64,
    ) -> InstalledClassifierModel {
        let name = dir
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_default();
        let installed_at_ms = fs::metadata(dir)
            .and_then(|metadata| metadata.modified())
            .ok()
            .and_then(|modified| modified.duration_since(SystemTime::UNIX_EPOCH).ok())
            .map(|duration| duration.as_millis() as i64)
            .unwrap_or(now_ms);
        InstalledClassifierModel {
            model_id: manifest.model_id.clone(),
            version: manifest.version.clone(),
            checkpoint: manifest.checkpoint.clone(),
            path: dir.to_string_lossy().to_string(),
            size_bytes: manifest.total_size_bytes(),
            active: current == Some(name.as_str()),
            verified: dir_verified(dir, manifest),
            installed_at_ms,
        }
    }

    fn verify_target(
        &self,
        dir: &Path,
        manifest: &ClassifierModelManifest,
    ) -> Result<(), ClassifierModelError> {
        for file in &manifest.files {
            let metadata = fs::metadata(dir.join(&file.path)).map_err(|_| {
                ClassifierModelError::NotInstalled {
                    model_id: manifest.model_id.clone(),
                    version: manifest.version.clone(),
                }
            })?;
            if file.size_bytes > 0 && metadata.len() != file.size_bytes {
                return Err(ClassifierModelError::ChecksumMismatch {
                    file: file.path.clone(),
                });
            }
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn publish(
        &self,
        manifest: &ClassifierModelManifest,
        phase: SuggestionModelDownloadPhase,
        file: Option<&str>,
        file_index: usize,
        file_count: usize,
        bytes_done: u64,
        bytes_total: u64,
        error: Option<String>,
    ) {
        self.publisher
            .emit_suggestion_model_download(&SuggestionModelDownloadEvent {
                model_id: manifest.model_id.clone(),
                version: manifest.version.clone(),
                phase,
                file: file.map(|value| value.to_string()),
                file_index,
                file_count,
                bytes_done,
                bytes_total,
                error,
            });
    }

    fn install_inner(
        &self,
        manifest: &ClassifierModelManifest,
    ) -> Result<InstalledClassifierModel, ClassifierModelError> {
        manifest
            .validate()
            .map_err(|error| ClassifierModelError::InvalidManifest(error.to_string()))?;
        self.ensure_root()?;

        let target = self.version_dir(&manifest.model_id, &manifest.version)?;
        if target.is_dir() {
            if let Ok(existing) = Self::read_manifest(&target) {
                if dir_verified(&target, &existing) {
                    return Ok(self.installed_entry(
                        &target,
                        &existing,
                        self.current_name().as_deref(),
                        self.clock.now_ms(),
                    ));
                }
            }
            // Diretório presente mas incompleto: refaz.
            fs::remove_dir_all(&target).map_err(map_io_error)?;
        }

        let part = self
            .root
            .join(format!("{}{PART_SUFFIX}", manifest.directory_name()));
        if part.exists() {
            fs::remove_dir_all(&part).map_err(map_io_error)?;
        }
        fs::create_dir_all(&part).map_err(map_io_error)?;

        let bytes_total = manifest.total_size_bytes();
        let mut bytes_done = 0_u64;
        let file_count = manifest.files.len();

        let result = (|| -> Result<(), ClassifierModelError> {
            for (index, file) in manifest.files.iter().enumerate() {
                let destination = part.join(&file.path);
                if let Some(parent) = destination.parent() {
                    fs::create_dir_all(parent).map_err(map_io_error)?;
                }
                self.publish(
                    manifest,
                    SuggestionModelDownloadPhase::Downloading,
                    Some(&file.path),
                    index + 1,
                    file_count,
                    bytes_done,
                    bytes_total,
                    None,
                );

                let base = bytes_done;
                let mut on_progress = |file_done: u64| {
                    self.publish(
                        manifest,
                        SuggestionModelDownloadPhase::Downloading,
                        Some(&file.path),
                        index + 1,
                        file_count,
                        base.saturating_add(file_done),
                        bytes_total,
                        None,
                    );
                };
                self.source
                    .download_to(&file.url, &destination, &mut on_progress)?;

                let downloaded = fs::metadata(&destination)
                    .map(|metadata| metadata.len())
                    .unwrap_or(0);
                if file.size_bytes > 0 && downloaded != file.size_bytes {
                    return Err(ClassifierModelError::ChecksumMismatch {
                        file: file.path.clone(),
                    });
                }
                self.publish(
                    manifest,
                    SuggestionModelDownloadPhase::Verifying,
                    Some(&file.path),
                    index + 1,
                    file_count,
                    bytes_done.saturating_add(downloaded),
                    bytes_total,
                    None,
                );
                if sha256_file(&destination)? != file.sha256 {
                    return Err(ClassifierModelError::ChecksumMismatch {
                        file: file.path.clone(),
                    });
                }
                bytes_done = bytes_done.saturating_add(downloaded);
            }

            self.publish(
                manifest,
                SuggestionModelDownloadPhase::Installing,
                None,
                file_count,
                file_count,
                bytes_done,
                bytes_total,
                None,
            );
            let encoded = serde_json::to_vec_pretty(manifest)
                .map_err(|error| ClassifierModelError::InvalidManifest(error.to_string()))?;
            fs::write(part.join(MANIFEST_FILE), encoded).map_err(map_io_error)?;
            fs::rename(&part, &target).map_err(map_io_error)?;
            Ok(())
        })();

        match result {
            Ok(()) => {
                self.publish(
                    manifest,
                    SuggestionModelDownloadPhase::Ready,
                    None,
                    file_count,
                    file_count,
                    bytes_done,
                    bytes_total,
                    None,
                );
                Ok(self.installed_entry(
                    &target,
                    manifest,
                    self.current_name().as_deref(),
                    self.clock.now_ms(),
                ))
            }
            Err(error) => {
                // Nunca deixa um parcial para trás nem mexe em `current`.
                let _ = fs::remove_dir_all(&part);
                self.publish(
                    manifest,
                    SuggestionModelDownloadPhase::Failed,
                    None,
                    0,
                    file_count,
                    bytes_done,
                    bytes_total,
                    Some(error.to_string()),
                );
                Err(error)
            }
        }
    }
}

impl ClassifierModelManager for LocalClassifierModelManager {
    fn catalog(&self) -> Result<ClassifierModelCatalog, ClassifierModelError> {
        let catalog_url = self.catalog_url();
        if catalog_url.trim().is_empty() {
            return Err(ClassifierModelError::Unavailable(
                "catálogo não configurado".to_string(),
            ));
        }
        let bytes = self.source.fetch_bytes(&catalog_url, CATALOG_MAX_BYTES)?;
        let catalog: ClassifierModelCatalog = serde_json::from_slice(&bytes)
            .map_err(|error| ClassifierModelError::Unavailable(error.to_string()))?;
        catalog
            .validate()
            .map_err(|error| ClassifierModelError::InvalidManifest(error.to_string()))?;
        Ok(catalog)
    }

    fn installed(&self) -> Result<Vec<InstalledClassifierModel>, ClassifierModelError> {
        self.ensure_root()?;
        let current = self.current_name();
        let now_ms = self.clock.now_ms();
        let mut models = Vec::new();
        let entries = fs::read_dir(&self.root).map_err(map_io_error)?;
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let name = path
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_default();
            if name.ends_with(PART_SUFFIX) {
                continue;
            }
            let Ok(manifest) = Self::read_manifest(&path) else {
                continue;
            };
            models.push(self.installed_entry(&path, &manifest, current.as_deref(), now_ms));
        }
        models.sort_by(|left, right| {
            right
                .installed_at_ms
                .cmp(&left.installed_at_ms)
                .then_with(|| left.model_id.cmp(&right.model_id))
        });
        Ok(models)
    }

    fn active(&self) -> Option<InstalledClassifierModel> {
        let current = self.current_name()?;
        let dir = self.root.join(&current);
        let manifest = Self::read_manifest(&dir).ok()?;
        Some(self.installed_entry(&dir, &manifest, Some(&current), self.clock.now_ms()))
    }

    fn manifest(
        &self,
        model_id: &str,
        version: &str,
    ) -> Result<ClassifierModelManifest, ClassifierModelError> {
        let dir = self.version_dir(model_id, version)?;
        if !dir.is_dir() {
            return Err(ClassifierModelError::NotInstalled {
                model_id: model_id.to_string(),
                version: version.to_string(),
            });
        }
        Self::read_manifest(&dir)
    }

    fn install(
        &self,
        model_id: &str,
        version: &str,
    ) -> Result<InstalledClassifierModel, ClassifierModelError> {
        // O manifesto ainda não está no disco: vem do catálogo publicado.
        let catalog = self.catalog()?;
        let manifest = catalog
            .find(model_id, version)
            .ok_or_else(|| ClassifierModelError::UnknownModel(format!("{model_id} {version}")))?;
        self.install_inner(manifest)
    }

    fn activate(
        &self,
        model_id: &str,
        version: &str,
    ) -> Result<InstalledClassifierModel, ClassifierModelError> {
        let dir = self.version_dir(model_id, version)?;
        if !dir.is_dir() {
            return Err(ClassifierModelError::NotInstalled {
                model_id: model_id.to_string(),
                version: version.to_string(),
            });
        }
        let manifest = Self::read_manifest(&dir)?;
        self.verify_target(&dir, &manifest)?;
        self.ensure_root()?;
        let pointer = self.root.join(CURRENT_FILE);
        let temporary = self.root.join(format!("{CURRENT_FILE}{PART_SUFFIX}"));
        fs::write(&temporary, manifest.directory_name()).map_err(map_io_error)?;
        fs::rename(&temporary, &pointer).map_err(map_io_error)?;
        Ok(self.installed_entry(
            &dir,
            &manifest,
            Some(&manifest.directory_name()),
            self.clock.now_ms(),
        ))
    }

    fn remove(&self, model_id: &str, version: &str) -> Result<(), ClassifierModelError> {
        let dir = self.version_dir(model_id, version)?;
        if !dir.is_dir() {
            return Err(ClassifierModelError::NotInstalled {
                model_id: model_id.to_string(),
                version: version.to_string(),
            });
        }
        let name = dir
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_default();
        if self.current_name().as_deref() == Some(name.as_str()) {
            return Err(ClassifierModelError::ActiveModel);
        }
        fs::remove_dir_all(&dir).map_err(map_io_error)
    }
}

/// `true` quando todos os arquivos listados existem e têm o tamanho esperado.
///
/// O SHA-256 é conferido no download; aqui a checagem é estrutural (barata o
/// bastante para a UI chamar a cada abertura).
fn dir_verified(dir: &Path, manifest: &ClassifierModelManifest) -> bool {
    manifest
        .files
        .iter()
        .all(|file| match fs::metadata(dir.join(&file.path)) {
            Ok(metadata) => file.size_bytes == 0 || metadata.len() == file.size_bytes,
            Err(_) => false,
        })
}

/// SHA-256 (hex) de um arquivo, lido em blocos (não carrega 1,2 GB na RAM).
pub fn sha256_file(path: &Path) -> Result<String, ClassifierModelError> {
    let file = File::open(path).map_err(map_io_error)?;
    let mut reader = BufReader::new(file);
    let mut hasher = Sha256::new();
    let mut buffer = vec![0_u8; CHUNK];
    loop {
        let read = reader.read(&mut buffer).map_err(map_io_error)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Mutex;

    use crate::adapters::system::system_clock::FixedClock;
    use crate::domain::audio::AudioLevel;
    use crate::domain::classifier_model::{
        ClassifierModelCalibration, ClassifierModelFile, ClassifierModelMetrics,
        CLASSIFIER_MODEL_SCHEMA,
    };
    use crate::domain::transcription::{AsrStatus, TranscriptSegment};
    use crate::ports::event_publisher::{
        JournalUpdate, LlmSourceUsage, RecordingProgressEvent, SessionStateEvent,
        SuggestionCadenceEvent, SuggestionClassificationEvent,
    };
    use crate::ports::session_controller::AudioChannel;
    use crate::ports::suggestion_generator::Suggestion;

    const CATALOG_URL: &str = "https://models.invalid/catalog.json";

    fn sha256_hex(bytes: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        format!("{:x}", hasher.finalize())
    }

    #[derive(Default)]
    struct RecordingPublisher {
        events: Mutex<Vec<SuggestionModelDownloadEvent>>,
    }

    impl RecordingPublisher {
        fn phases(&self) -> Vec<SuggestionModelDownloadPhase> {
            self.events
                .lock()
                .map(|events| events.iter().map(|event| event.phase).collect())
                .unwrap_or_default()
        }
    }

    impl EventPublisher for RecordingPublisher {
        fn emit_segment(&self, _segment: &TranscriptSegment) {}
        fn emit_audio_level(&self, _channel: AudioChannel, _level: AudioLevel) {}
        fn emit_session_state(&self, _state: &SessionStateEvent) {}
        fn emit_insight(&self, _suggestion: &Suggestion) {}
        fn emit_suggestion(&self, _suggestion: &Suggestion) {}
        fn emit_suggestion_cadence(&self, _cadence: &SuggestionCadenceEvent) {}
        fn emit_suggestion_classification(&self, _event: &SuggestionClassificationEvent) {}
        fn emit_suggestion_model_download(&self, event: &SuggestionModelDownloadEvent) {
            if let Ok(mut events) = self.events.lock() {
                events.push(event.clone());
            }
        }
        fn emit_asr_status(&self, _status: &AsrStatus) {}
        fn emit_llm_source_used(&self, _usage: &LlmSourceUsage) {}
        fn emit_journal(&self, _update: &JournalUpdate) {}
        fn emit_recording_progress(&self, _progress: &RecordingProgressEvent) {}
    }

    /// Catálogo + bytes reais dos arquivos (por URL).
    struct Fixture {
        catalog: ClassifierModelCatalog,
        files: HashMap<String, Vec<u8>>,
    }

    fn fixture(version: &str, files: &[(&str, &[u8])]) -> Fixture {
        let manifest = ClassifierModelManifest {
            schema: CLASSIFIER_MODEL_SCHEMA,
            model_id: "laya-multilingual".to_string(),
            version: version.to_string(),
            checkpoint: "multilingual".to_string(),
            layout: "onnx".to_string(),
            label: Some("Laya Multilingual".to_string()),
            size_bytes: 0,
            license: "Apache-2.0".to_string(),
            files: files
                .iter()
                .map(|(path, bytes)| ClassifierModelFile {
                    path: path.to_string(),
                    url: format!("https://models.invalid/{version}/{path}"),
                    size_bytes: bytes.len() as u64,
                    sha256: sha256_hex(bytes),
                })
                .collect(),
            calibration: ClassifierModelCalibration::default(),
            metrics: ClassifierModelMetrics::default(),
            created_at: "2026-10-26".to_string(),
        };
        let catalog = ClassifierModelCatalog {
            schema: CLASSIFIER_MODEL_SCHEMA,
            models: vec![manifest],
        };
        let files = catalog.models[0]
            .files
            .iter()
            .zip(files)
            .map(|(file, (_, bytes))| (file.url.clone(), bytes.to_vec()))
            .collect();
        Fixture { catalog, files }
    }

    struct FakeSource {
        catalog: Vec<u8>,
        files: HashMap<String, Vec<u8>>,
        downloads: AtomicUsize,
    }

    impl FakeSource {
        fn new(catalog: &ClassifierModelCatalog, files: HashMap<String, Vec<u8>>) -> Self {
            Self {
                catalog: serde_json::to_vec(catalog).expect("catálogo"),
                files,
                downloads: AtomicUsize::new(0),
            }
        }

        fn downloads(&self) -> usize {
            self.downloads.load(Ordering::SeqCst)
        }
    }

    impl ClassifierModelSource for FakeSource {
        fn fetch_bytes(&self, url: &str, _max: u64) -> Result<Vec<u8>, ClassifierModelError> {
            if url == CATALOG_URL {
                return Ok(self.catalog.clone());
            }
            Err(ClassifierModelError::Unavailable(url.to_string()))
        }

        fn download_to(
            &self,
            url: &str,
            dest: &Path,
            progress: &mut dyn FnMut(u64),
        ) -> Result<(), ClassifierModelError> {
            self.downloads.fetch_add(1, Ordering::SeqCst);
            let bytes = self
                .files
                .get(url)
                .ok_or_else(|| ClassifierModelError::Unavailable(url.to_string()))?;
            fs::write(dest, bytes).map_err(map_io_error)?;
            progress(bytes.len() as u64);
            Ok(())
        }
    }

    fn build_manager(
        root: &Path,
        fixture: &Fixture,
        publisher: Arc<RecordingPublisher>,
    ) -> (LocalClassifierModelManager, Arc<FakeSource>) {
        use crate::adapters::settings::memory::InMemorySettingsStore;
        use crate::domain::settings::Settings;

        let source = Arc::new(FakeSource::new(&fixture.catalog, fixture.files.clone()));
        let settings = Arc::new(InMemorySettingsStore::with_settings(Settings {
            suggestion_classifier_model_catalog_url: CATALOG_URL.to_string(),
            ..Settings::default()
        }));
        let manager = LocalClassifierModelManager::new(
            root.to_path_buf(),
            settings,
            source.clone(),
            publisher,
            Arc::new(FixedClock(1_000)),
        );
        (manager, source)
    }

    #[test]
    fn local_source_reads_and_copies_files() {
        let temp = tempfile::tempdir().expect("tempdir");
        let source = UreqClassifierModelSource;
        let origin = temp.path().join("catalog.json");
        fs::write(&origin, b"{\"schema\":1}").unwrap();

        let bytes = source
            .fetch_bytes(origin.to_str().unwrap(), 1_024)
            .expect("lê local");
        assert_eq!(bytes, b"{\"schema\":1}");

        let dest = temp.path().join("copy.json");
        let mut total = 0_u64;
        source
            .download_to(origin.to_str().unwrap(), &dest, &mut |done| total = done)
            .expect("copia local");
        assert_eq!(fs::read(&dest).unwrap(), b"{\"schema\":1}");
        assert_eq!(total, 12);
    }

    #[test]
    fn local_catalog_end_to_end_matches_the_dev_artifact() {
        use crate::adapters::settings::memory::InMemorySettingsStore;
        use crate::domain::settings::Settings;

        let temp = tempfile::tempdir().expect("tempdir");
        let artifact = temp.path().join("artifact");
        fs::create_dir_all(&artifact).unwrap();
        let model = artifact.join("model.onnx");
        fs::write(&model, b"fake").unwrap();
        let payload = format!(
            r#"{{"schema":1,"models":[{{"schema":1,"model_id":"laya-multilingual",
              "version":"0.1.0","checkpoint":"multilingual","files":[{{"path":"model.onnx",
              "url":"{}","size_bytes":4,"sha256":"{}"}}]}}]}}"#,
            model.display(),
            sha256_hex(b"fake")
        );
        let catalog_path = temp.path().join("catalog.json");
        fs::write(&catalog_path, payload).unwrap();

        let settings = Arc::new(InMemorySettingsStore::with_settings(Settings {
            suggestion_classifier_model_catalog_url: catalog_path.display().to_string(),
            ..Settings::default()
        }));
        let publisher = Arc::new(RecordingPublisher::default());
        let manager = LocalClassifierModelManager::new(
            temp.path().join("models"),
            settings,
            Arc::new(UreqClassifierModelSource),
            Arc::clone(&publisher) as Arc<dyn EventPublisher>,
            Arc::new(FixedClock(1_000)),
        );

        let installed = manager
            .install("laya-multilingual", "0.1.0")
            .expect("instala");
        assert!(installed.verified);
        assert!(!installed.active);
        let active = manager
            .activate("laya-multilingual", "0.1.0")
            .expect("ativa");
        assert!(active.active);
        assert!(Path::new(&active.path).join("model.onnx").exists());
        assert_eq!(
            fs::read(Path::new(&active.path).join("model.onnx")).unwrap(),
            b"fake"
        );
        assert_eq!(manager.active().expect("ativo").version, "0.1.0");
    }

    #[test]
    fn reads_catalog_without_downloading_model_files() {
        let temp = tempfile::tempdir().expect("tempdir");
        let fixture = fixture("0.1.0", &[("model.onnx", b"modelo")]);
        let publisher = Arc::new(RecordingPublisher::default());
        let (manager, source) = build_manager(temp.path(), &fixture, Arc::clone(&publisher));

        let read = manager.catalog().expect("catálogo");
        assert_eq!(read.recommended().unwrap().version, "0.1.0");
        assert_eq!(source.downloads(), 0, "catálogo não baixa o modelo");
        assert!(publisher.phases().is_empty());
    }

    #[test]
    fn installs_and_activates_a_model() {
        let temp = tempfile::tempdir().expect("tempdir");
        let fixture = fixture(
            "0.1.0",
            &[
                ("model.onnx", b"modelo"),
                ("tokenizer/tokenizer.json", b"{}"),
            ],
        );
        let publisher = Arc::new(RecordingPublisher::default());
        let (manager, source) = build_manager(temp.path(), &fixture, Arc::clone(&publisher));

        let installed = manager
            .install("laya-multilingual", "0.1.0")
            .expect("instala");
        assert!(!installed.active);
        assert!(installed.verified);
        assert_eq!(source.downloads(), 2);
        assert!(Path::new(&installed.path).join(MANIFEST_FILE).exists());
        assert!(manager.active().is_none(), "instalar não ativa");

        let active = manager
            .activate("laya-multilingual", "0.1.0")
            .expect("ativa");
        assert!(active.active);
        assert_eq!(manager.active().expect("ativo").version, "0.1.0");
        assert!(publisher
            .phases()
            .contains(&SuggestionModelDownloadPhase::Ready));
    }

    #[test]
    fn rejects_bad_checksum_and_leaves_nothing_active() {
        let temp = tempfile::tempdir().expect("tempdir");
        let mut fixture = fixture("0.1.0", &[("model.onnx", b"modelo")]);
        let url = fixture.catalog.models[0].files[0].url.clone();
        fixture.files.insert(url, b"corrompido".to_vec());
        let publisher = Arc::new(RecordingPublisher::default());
        let (manager, _source) = build_manager(temp.path(), &fixture, Arc::clone(&publisher));

        let error = manager
            .install("laya-multilingual", "0.1.0")
            .expect_err("checksum");
        assert!(matches!(
            error,
            ClassifierModelError::ChecksumMismatch { .. }
        ));
        assert!(manager.active().is_none());
        assert!(manager.installed().expect("instalados").is_empty());
        assert!(publisher
            .phases()
            .contains(&SuggestionModelDownloadPhase::Failed));
        assert!(
            !temp.path().join("laya-multilingual-0.1.0.part").exists(),
            "parcial removido"
        );
    }

    #[test]
    fn failed_download_keeps_the_previous_active_version() {
        let temp = tempfile::tempdir().expect("tempdir");
        let first = fixture("0.1.0", &[("model.onnx", b"modelo")]);
        let publisher = Arc::new(RecordingPublisher::default());
        let (manager, _source) = build_manager(temp.path(), &first, Arc::clone(&publisher));
        manager
            .install("laya-multilingual", "0.1.0")
            .expect("instala");
        manager
            .activate("laya-multilingual", "0.1.0")
            .expect("ativa");

        let mut next = fixture("0.2.0", &[("model.onnx", b"modelo-v2")]);
        let url = next.catalog.models[0].files[0].url.clone();
        next.files.insert(url, b"outro".to_vec());
        let (manager2, _source2) = build_manager(temp.path(), &next, Arc::clone(&publisher));
        // `active` lê do disco, não do catálogo.
        assert_eq!(manager2.active().expect("ativa").version, "0.1.0");
        assert!(manager2
            .install("laya-multilingual", "0.2.0")
            .expect_err("corrompido")
            .to_string()
            .contains("checksum"));
        assert_eq!(manager2.active().expect("ativa").version, "0.1.0");
    }

    #[test]
    fn never_downloads_without_an_explicit_install() {
        let temp = tempfile::tempdir().expect("tempdir");
        let fixture = fixture("0.1.0", &[("model.onnx", b"modelo")]);
        let publisher = Arc::new(RecordingPublisher::default());
        let (manager, source) = build_manager(temp.path(), &fixture, Arc::clone(&publisher));

        manager.catalog().expect("catálogo");
        manager.installed().expect("instalados");
        assert!(manager.active().is_none());
        assert_eq!(source.downloads(), 0);
    }

    #[test]
    fn refuses_to_remove_the_active_version() {
        let temp = tempfile::tempdir().expect("tempdir");
        let fixture = fixture("0.1.0", &[("model.onnx", b"modelo")]);
        let publisher = Arc::new(RecordingPublisher::default());
        let (manager, _source) = build_manager(temp.path(), &fixture, Arc::clone(&publisher));

        // Instalada mas não ativa: pode remover.
        manager
            .install("laya-multilingual", "0.1.0")
            .expect("instala");
        manager
            .remove("laya-multilingual", "0.1.0")
            .expect("remove inativa");
        assert!(manager.installed().expect("instalados").is_empty());

        manager
            .install("laya-multilingual", "0.1.0")
            .expect("instala");
        manager
            .activate("laya-multilingual", "0.1.0")
            .expect("ativa");
        assert_eq!(
            manager.remove("laya-multilingual", "0.1.0"),
            Err(ClassifierModelError::ActiveModel)
        );
    }

    #[test]
    fn lists_installed_versions_with_the_active_flag() {
        let temp = tempfile::tempdir().expect("tempdir");
        let fixture = fixture("0.1.0", &[("model.onnx", b"modelo")]);
        let publisher = Arc::new(RecordingPublisher::default());
        let (manager, _source) = build_manager(temp.path(), &fixture, Arc::clone(&publisher));
        manager
            .install("laya-multilingual", "0.1.0")
            .expect("instala");
        manager
            .activate("laya-multilingual", "0.1.0")
            .expect("ativa");

        let installed = manager.installed().expect("instalados");
        assert_eq!(installed.len(), 1);
        assert!(installed[0].active);
        assert!(installed[0].verified);
    }

    #[test]
    fn rejects_unknown_versions_with_a_typed_error() {
        let temp = tempfile::tempdir().expect("tempdir");
        let fixture = fixture("0.1.0", &[("model.onnx", b"modelo")]);
        let publisher = Arc::new(RecordingPublisher::default());
        let (manager, _source) = build_manager(temp.path(), &fixture, Arc::clone(&publisher));
        assert!(matches!(
            manager.install("laya-multilingual", "9.9.9"),
            Err(ClassifierModelError::UnknownModel(_))
        ));
        assert!(matches!(
            manager.activate("../x", "0.1.0"),
            Err(ClassifierModelError::UnknownModel(_))
        ));
    }
}
