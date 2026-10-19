//! Detecção de GPU no Linux via **Vulkan** (ICDs) e nós de render do DRM.
//!
//! Linux-first: procura instalações do Vulkan em
//! `VK_ICD_FILENAMES`, `/usr/share/vulkan/icd.d` e `/etc/vulkan/icd.d`, lê o
//! `library_path` de cada ICD e **descarta rasterizers de software**
//! (lavapipe/llvmpipe/swrast), que não ajudam o Whisper. Sem ICD, cai para a
//! presença de `/dev/dri/renderD*`.
//!
//! A classificação é uma função **pura** (testável sem GPU); a leitura de disco é
//! a casca fina em volta.

use std::path::Path;

use crate::ports::gpu_probe::{GpuInfo, GpuProbe};

/// Diretórios padrão de ICDs do Vulkan.
const ICD_DIRS: [&str; 2] = ["/usr/share/vulkan/icd.d", "/etc/vulkan/icd.d"];
/// Diretório dos nós de render do DRM.
const DRI_DIR: &str = "/dev/dri";

/// Detecção real do sistema de arquivos.
pub struct VulkanGpuProbe;

impl VulkanGpuProbe {
    /// Cria o probe.
    pub fn new() -> Self {
        Self
    }
}

impl Default for VulkanGpuProbe {
    fn default() -> Self {
        Self::new()
    }
}

/// O build atual acelera por GPU?
///
/// As features são declaradas no `Cargo.toml` e repassadas ao `whisper-rs`; por
/// padrão o app é CPU. Ligar GPU é o entregável E3 do PRD `precision-and-performance`.
pub fn acceleration_enabled() -> bool {
    cfg!(any(
        feature = "vulkan",
        feature = "cuda",
        feature = "hipblas",
        feature = "metal"
    ))
}

/// Um candidato a ICD: (nome do arquivo, `library_path`).
type IcdEntry = (String, String);

impl GpuProbe for VulkanGpuProbe {
    fn probe(&self) -> GpuInfo {
        let enabled = acceleration_enabled();
        let entries = collect_icd_entries();
        if let Some((vendor, file)) = classify_icds(&entries) {
            return GpuInfo::new(
                true,
                "vulkan",
                format!("{vendor} (Vulkan · {file})"),
                enabled,
            );
        }
        if has_render_node(Path::new(DRI_DIR)) {
            return GpuInfo::new(
                true,
                "drm",
                "nó de render do DRM presente, sem ICD Vulkan reconhecido".to_string(),
                enabled,
            );
        }
        GpuInfo::none("nenhuma GPU Vulkan/DRM detectada", enabled)
    }
}

/// Lê os ICDs do ambiente e dos diretórios padrão.
fn collect_icd_entries() -> Vec<IcdEntry> {
    let mut entries = Vec::new();
    if let Ok(raw) = std::env::var("VK_ICD_FILENAMES") {
        for path in raw.split(':').filter(|value| !value.trim().is_empty()) {
            push_entry(&mut entries, Path::new(path));
        }
    }
    for dir in ICD_DIRS {
        let Ok(read_dir) = std::fs::read_dir(dir) else {
            continue;
        };
        for entry in read_dir.flatten() {
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) == Some("json") {
                push_entry(&mut entries, &path);
            }
        }
    }
    entries
}

/// Lê um ICD (se for arquivo) e acrescenta à lista.
fn push_entry(entries: &mut Vec<IcdEntry>, path: &Path) {
    if !path.is_file() {
        return;
    }
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_string();
    let library = std::fs::read_to_string(path)
        .ok()
        .and_then(|json| parse_icd_library(&json))
        .unwrap_or_default();
    entries.push((name, library));
}

/// Extrai o campo `library_path` do JSON de um ICD (sem dependência de JSON
/// completo: é um objeto raso e o campo é uma string simples).
pub fn parse_icd_library(json: &str) -> Option<String> {
    let key = "\"library_path\"";
    let start = json.find(key)? + key.len();
    let rest = json[start..].trim_start();
    let rest = rest.strip_prefix(':')?.trim_start();
    let rest = rest.strip_prefix('"')?;
    let end = rest.find('"')?;
    let value = &rest[..end];
    (!value.is_empty()).then(|| value.to_string())
}

/// É um rasterizer de software (não vale como GPU)?
fn is_software(lower: &str) -> bool {
    ["lvp", "lavapipe", "llvmpipe", "swrast", "softpipe", "zink"]
        .iter()
        .any(|needle| lower.contains(needle))
}

/// Nome legível do vendor a partir do arquivo/biblioteca do ICD.
fn vendor_name(lower: &str) -> Option<&'static str> {
    if lower.contains("nvidia") {
        Some("NVIDIA")
    } else if lower.contains("radeon") || lower.contains("radv") || lower.contains("amd") {
        Some("AMD (RADV)")
    } else if lower.contains("intel") {
        Some("Intel")
    } else if lower.contains("mesa") || lower.contains("v3d") || lower.contains("panfrost") {
        Some("Mesa")
    } else {
        None
    }
}

/// Classifica os ICDs: devolve `(vendor legível, arquivo)` do primeiro hardware.
pub fn classify_icds(entries: &[IcdEntry]) -> Option<(String, String)> {
    for (file, library) in entries {
        let lower = format!("{file} {library}").to_lowercase();
        if is_software(&lower) {
            continue;
        }
        if let Some(vendor) = vendor_name(&lower) {
            return Some((vendor.to_string(), file.clone()));
        }
    }
    None
}

/// Existe algum nó de render (`renderD*`) no diretório?
fn has_render_node(dir: &Path) -> bool {
    let Ok(read_dir) = std::fs::read_dir(dir) else {
        return false;
    };
    read_dir.flatten().any(|entry| {
        entry
            .file_name()
            .to_str()
            .is_some_and(|name| name.starts_with("renderD"))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(file: &str, lib: &str) -> IcdEntry {
        (file.to_string(), lib.to_string())
    }

    #[test]
    fn parses_library_path_from_an_icd_json() {
        let json = r#"{ "file_format_version": "1.0.0", "ICD": { "library_path": "/usr/lib/libvulkan_radeon.so", "api_version": "1.3.0" } }"#;
        assert_eq!(
            parse_icd_library(json).as_deref(),
            Some("/usr/lib/libvulkan_radeon.so")
        );
        assert_eq!(parse_icd_library("{}"), None);
        assert_eq!(parse_icd_library(r#"{"library_path": ""}"#), None);
    }

    #[test]
    fn ignores_software_rasterizers() {
        let entries = vec![
            entry("lvp_icd.x86_64.json", "/usr/lib/libvulkan_lvp.so"),
            entry("intel_icd.x86_64.json", "/usr/lib/libvulkan_intel.so"),
        ];
        assert_eq!(
            classify_icds(&entries),
            Some(("Intel".to_string(), "intel_icd.x86_64.json".to_string()))
        );
    }

    #[test]
    fn detects_common_vendors() {
        assert_eq!(
            classify_icds(&[entry("nvidia_icd.json", "libGLX_nvidia.so")]),
            Some(("NVIDIA".to_string(), "nvidia_icd.json".to_string()))
        );
        assert_eq!(
            classify_icds(&[entry("radeon_icd.x86_64.json", "libvulkan_radeon.so")]),
            Some((
                "AMD (RADV)".to_string(),
                "radeon_icd.x86_64.json".to_string()
            ))
        );
    }

    #[test]
    fn software_only_is_not_a_gpu() {
        let entries = vec![entry("lvp_icd.x86_64.json", "libvulkan_lvp.so")];
        assert_eq!(classify_icds(&entries), None);
    }

    #[test]
    fn probe_reports_a_serializable_result() {
        // Roda no host atual: o importante é não entrar em pânico e devolver o
        // modelo sugerido coerente com `available`/`acceleration_enabled`.
        let info = VulkanGpuProbe::new().probe();
        if info.available && info.acceleration_enabled {
            assert_eq!(info.recommended_model, "large-v3-turbo");
        } else {
            assert_eq!(info.recommended_model, "tiny");
        }
        assert!(!info.backend.is_empty());
    }
}
