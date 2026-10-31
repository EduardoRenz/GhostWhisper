//! Adaptador de captura do áudio do sistema.
//!
//! Lista monitores de saída via `pactl` e grava do monitor selecionado com a API
//! simple do PulseAudio (compatível com PipeWire). O servidor faz o remix para
//! mono 16 kHz, então o cliente só normaliza as amostras.

use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

use libpulse_binding::sample::{Format, Spec};
use libpulse_binding::stream::Direction;
use libpulse_simple_binding::Simple;

use crate::domain::audio::{AudioDevice, TARGET_SAMPLE_RATE};
use crate::error::{GhostWhisperError, Result};
use crate::ports::audio_consumer::AudioConsumer;
use crate::ports::audio_source::AudioSource;

use super::pactl;

/// Captura o monitor de saída do sistema (PulseAudio/PipeWire).
pub struct PulseSystemAudioSource {
    stop: Arc<AtomicBool>,
    running: Arc<AtomicBool>,
    thread: Mutex<Option<JoinHandle<()>>>,
}

impl Default for PulseSystemAudioSource {
    fn default() -> Self {
        Self::new()
    }
}

impl PulseSystemAudioSource {
    /// Cria a fonte (nenhuma captura é iniciada aqui).
    pub fn new() -> Self {
        Self {
            stop: Arc::new(AtomicBool::new(false)),
            running: Arc::new(AtomicBool::new(false)),
            thread: Mutex::new(None),
        }
    }
}

impl AudioSource for PulseSystemAudioSource {
    fn list_devices(&self) -> Result<Vec<AudioDevice>> {
        let default_monitor = default_sink_name().map(|sink| format!("{sink}.monitor"));

        // Caminho preferido: `pactl` (lista completa, inclusive servidores
        // PulseAudio puros). Se o binário não existe (PipeWire sem
        // `pulseaudio-utils`), cai para as ferramentas nativas do PipeWire.
        if let Ok(sources) = pactl::run_pactl(&["list", "short", "sources"]) {
            let devices = parse_monitors(&sources, default_monitor.as_deref());
            if !devices.is_empty() {
                return Ok(devices);
            }
        }
        monitors_without_pactl(default_monitor)
    }

    fn start(&self, device_id: Option<String>, consumer: Arc<dyn AudioConsumer>) -> Result<()> {
        self.stop()?;

        let source_name = match device_id {
            Some(name) => name,
            None => default_monitor_name()?,
        };

        // Abre a conexão **antes** de devolver Ok: se o monitor não existir, o
        // erro chega ao chamador e vira aviso na UI (antes ele só aparecia no
        // log da thread e o canal ficava "ativo" sem capturar).
        let simple = open_capture(&source_name)?;

        self.stop.store(false, Ordering::Relaxed);
        self.running.store(true, Ordering::Relaxed);

        let stop = Arc::clone(&self.stop);
        let running = Arc::clone(&self.running);
        let handle = thread::spawn(move || {
            if let Err(error) = run_capture(simple, consumer, Arc::clone(&stop)) {
                tracing::error!("captura do sistema falhou: {error}");
            }
            running.store(false, Ordering::Relaxed);
        });

        if let Ok(mut guard) = self.thread.lock() {
            *guard = Some(handle);
        }
        Ok(())
    }

    fn stop(&self) -> Result<()> {
        self.stop.store(true, Ordering::Relaxed);
        if let Ok(mut guard) = self.thread.lock() {
            if let Some(handle) = guard.take() {
                let _ = handle.join();
            }
        }
        Ok(())
    }

    fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }
}

/// Nome do monitor do sink padrão (`<sink>.monitor`).
///
/// Usa `pactl` quando disponível; sem ele (PipeWire sem `pulseaudio-utils`),
/// resolve o sink padrão pelo WirePlumber (`wpctl`) ou pelos metadados do
/// PipeWire (`pw-metadata`).
pub(crate) fn default_monitor_name() -> Result<String> {
    default_sink_name()
        .map(|sink| format!("{sink}.monitor"))
        .ok_or_else(|| {
            GhostWhisperError::Other(
                "monitor de saída não encontrado; verifique o sink padrão do PipeWire/PulseAudio"
                    .to_string(),
            )
        })
}

/// Executa um utilitário e devolve o stdout quando ele termina com sucesso.
fn command_stdout(program: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(program).args(args).output().ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Nome do sink padrão, na ordem: PulseAudio (`pactl`), WirePlumber e
/// metadados do PipeWire.
fn default_sink_name() -> Option<String> {
    if let Some(output) = command_stdout("pactl", &["get-default-sink"]) {
        let name = output.trim();
        if !name.is_empty() {
            return Some(name.to_string());
        }
    }
    if let Some(output) = command_stdout("wpctl", &["inspect", "@DEFAULT_AUDIO_SINK@"]) {
        if let Some(name) = parse_wpctl_node_name(&output) {
            return Some(name);
        }
    }
    if let Some(output) = command_stdout("pw-metadata", &["-n", "default"]) {
        if let Some(name) = parse_pw_metadata_default_sink(&output) {
            return Some(name);
        }
    }
    None
}

/// Extrai `node.name = "..."` da saída de `wpctl inspect`.
fn parse_wpctl_node_name(output: &str) -> Option<String> {
    output.lines().find_map(|line| {
        let (_, value) = line.split_once("node.name = ")?;
        let value = value.trim().trim_matches('"');
        (!value.is_empty()).then(|| value.to_string())
    })
}

/// Extrai o nome do sink de `default.audio.sink` no `pw-metadata`.
///
/// Linha: `update: id:0 key:'default.audio.sink' value:'{"name":"..."}' …`.
fn parse_pw_metadata_default_sink(output: &str) -> Option<String> {
    output.lines().find_map(|line| {
        let (_, rest) = line.split_once("key:'default.audio.sink'")?;
        let start = rest.find("value:'")? + "value:'".len();
        let end = rest[start..].find("' type:")? + start;
        let value: serde_json::Value = serde_json::from_str(&rest[start..end]).ok()?;
        value.get("name")?.as_str().map(str::to_string)
    })
}

/// Sinks (`media.class = "Audio/Sink"`) listados por `pw-cli ls Node`.
fn parse_pw_cli_sinks(output: &str) -> Vec<String> {
    let mut sinks = Vec::new();
    let mut name: Option<String> = None;
    for line in output.lines() {
        let line = line.trim();
        if line.starts_with("id ") {
            name = None;
            continue;
        }
        if let Some(value) = line.strip_prefix("node.name = ") {
            name = Some(value.trim().trim_matches('"').to_string());
        } else if let Some(value) = line.strip_prefix("media.class = ") {
            if value.trim().trim_matches('"') == "Audio/Sink" {
                if let Some(name) = name.take() {
                    if !name.is_empty() && !sinks.contains(&name) {
                        sinks.push(name);
                    }
                }
            }
        }
    }
    sinks
}

/// Fallback de monitores sem `pactl`: usa os sinks do PipeWire e cobre o caso
/// em que nem o `pw-cli` está disponível com o monitor padrão.
fn monitors_without_pactl(default_monitor: Option<String>) -> Result<Vec<AudioDevice>> {
    let monitors: Vec<String> = command_stdout("pw-cli", &["ls", "Node"])
        .map(|output| {
            parse_pw_cli_sinks(&output)
                .into_iter()
                .map(|sink| format!("{sink}.monitor"))
                .collect()
        })
        .unwrap_or_default();

    let mut devices: Vec<AudioDevice> = monitors
        .into_iter()
        .map(|id| AudioDevice {
            is_default: default_monitor.as_deref() == Some(id.as_str()),
            id: id.clone(),
            name: id,
        })
        .collect();

    if devices.is_empty() {
        if let Some(default) = default_monitor {
            devices.push(AudioDevice {
                id: default.clone(),
                name: default,
                is_default: true,
            });
        }
    }

    if devices.is_empty() {
        return Err(GhostWhisperError::Other(
            "nenhum monitor de saída encontrado (PipeWire/PulseAudio)".to_string(),
        ));
    }
    Ok(devices)
}

/// Abre a captura do monitor (síncrono: erros chegam a quem chamou).
fn open_capture(source_name: &str) -> Result<Simple> {
    let spec = Spec {
        format: Format::S16le,
        channels: 1,
        rate: TARGET_SAMPLE_RATE,
    };

    Simple::new(
        None,
        "GhostWhisper",
        Direction::Record,
        Some(source_name),
        "system-capture",
        &spec,
        None,
        None,
    )
    .map_err(|e| {
        GhostWhisperError::Other(format!(
            "falha ao abrir captura do sistema ({source_name}): {e}"
        ))
    })
}

/// Grava do monitor normalizando para f32 mono 16 kHz.
fn run_capture(
    simple: Simple,
    consumer: Arc<dyn AudioConsumer>,
    stop: Arc<AtomicBool>,
) -> Result<()> {
    let mut buffer = vec![0u8; 4096];
    while !stop.load(Ordering::Relaxed) {
        match simple.read(&mut buffer) {
            Ok(()) => {
                let samples: Vec<f32> = buffer
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]) as f32 / 32768.0)
                    .collect();
                if !samples.is_empty() {
                    consumer.consume(&samples);
                }
            }
            Err(error) => {
                tracing::error!("erro na leitura do áudio do sistema: {error}");
                break;
            }
        }
    }

    Ok(())
}

/// Extrai monitores de saída da saída de `pactl list short sources`.
///
/// Linha: `<índice>\t<nome>\t<driver>\t<spec>\t<estado>`.
pub fn parse_monitors(output: &str, default_monitor: Option<&str>) -> Vec<AudioDevice> {
    output
        .lines()
        .filter_map(|line| {
            let mut fields = line.split('\t');
            let _index = fields.next()?;
            let name = fields.next()?.trim();
            if name.is_empty() || !name.ends_with(".monitor") {
                return None;
            }
            Some(AudioDevice {
                id: name.to_string(),
                name: name.to_string(),
                is_default: default_monitor == Some(name),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str =
        "49\talsa_output.usb-Headset.monitor\tPipeWire\ts16le 2ch 48000Hz\tSUSPENDED\n\
50\talsa_input.usb-Headset.mono-fallback\tPipeWire\ts16le 1ch 48000Hz\tSUSPENDED\n\
51\talsa_output.pci.hdmi.monitor\tPipeWire\ts32le 2ch 48000Hz\tSUSPENDED";

    #[test]
    fn lists_only_monitors() {
        let devices = parse_monitors(SAMPLE, None);
        assert_eq!(devices.len(), 2);
        assert_eq!(devices[0].id, "alsa_output.usb-Headset.monitor");
        assert_eq!(devices[1].id, "alsa_output.pci.hdmi.monitor");
    }

    #[test]
    fn marks_default_monitor() {
        let devices = parse_monitors(SAMPLE, Some("alsa_output.pci.hdmi.monitor"));
        let default = devices.iter().find(|d| d.is_default).expect("default");
        assert_eq!(default.id, "alsa_output.pci.hdmi.monitor");
    }

    /// ISS-087: sem `pactl`, o sink padrão vem do WirePlumber.
    #[test]
    fn parses_the_default_sink_from_wpctl() {
        let output = "id 46, type PipeWire:Interface:Node/3\n\
            \t* media.class = \"Audio/Sink\"\n\
            \t* node.description = \"H510 PRO\"\n\
            \t* node.name = \"alsa_output.usb-H510.analog-stereo\"\n\
            \t* object.id = \"46\"\n";
        assert_eq!(
            parse_wpctl_node_name(output).as_deref(),
            Some("alsa_output.usb-H510.analog-stereo")
        );
    }

    /// ISS-087: fallback pelos metadados do PipeWire.
    #[test]
    fn parses_the_default_sink_from_pipewire_metadata() {
        let output = "update: id:0 key:'default.configured.audio.sink' \
            value:'{\"name\":\"ignored\"}' type:'Spa:String:JSON'\n\
            update: id:0 key:'default.audio.sink' \
            value:'{\"name\":\"alsa_output.usb-H510.analog-stereo\"}' \
            type:'Spa:String:JSON'\n";
        assert_eq!(
            parse_pw_metadata_default_sink(output).as_deref(),
            Some("alsa_output.usb-H510.analog-stereo")
        );
    }

    /// ISS-087: fallback pelo `pw-cli` lista só os sinks (não as entradas).
    #[test]
    fn parses_sinks_from_pw_cli() {
        let output = "\tid 44, type PipeWire:Interface:Node/3\n\
            \t\tfactory.id = \"19\"\n\
            \t\tnode.name = \"alsa_output.usb-a.analog-stereo\"\n\
            \t\tmedia.class = \"Audio/Sink\"\n\
            \tid 45, type PipeWire:Interface:Node/3\n\
            \t\tfactory.id = \"19\"\n\
            \t\tnode.name = \"alsa_input.usb-a.mono-fallback\"\n\
            \t\tmedia.class = \"Audio/Source\"\n\
            \tid 46, type PipeWire:Interface:Node/3\n\
            \t\tfactory.id = \"19\"\n\
            \t\tnode.name = \"alsa_output.pci-hdmi.stereo\"\n\
            \t\tmedia.class = \"Audio/Sink\"\n";
        assert_eq!(
            parse_pw_cli_sinks(output),
            vec![
                "alsa_output.usb-a.analog-stereo".to_string(),
                "alsa_output.pci-hdmi.stereo".to_string()
            ]
        );
    }

    #[test]
    fn ignores_the_configured_metadata_key() {
        // A chave `default.configured.audio.sink` não pode ser confundida com
        // `default.audio.sink`.
        let output = "update: id:0 key:'default.configured.audio.sink' \
            value:'{\"name\":\"configured\"}' type:'Spa:String:JSON'\n";
        assert_eq!(parse_pw_metadata_default_sink(output), None);
    }

    #[test]
    fn source_starts_not_running() {
        assert!(!PulseSystemAudioSource::new().is_running());
    }

    // Testes de integração com o servidor real (rode com `--ignored`).

    #[test]
    #[ignore = "requer servidor Pulse/PipeWire em execução"]
    fn lists_monitors_on_real_server() {
        let devices = PulseSystemAudioSource::new().list_devices().expect("lista");
        assert!(!devices.is_empty(), "esperado ao menos um monitor");
        assert!(
            devices.iter().any(|d| d.is_default),
            "esperado monitor padrão"
        );
    }

    /// ISS-087: o nome resolvido sem `pactl` abre no libpulse.
    #[test]
    #[ignore = "requer servidor Pulse/PipeWire em execução"]
    fn opens_the_default_monitor_on_real_server() {
        let name = default_monitor_name().expect("monitor padrão");
        let capture = open_capture(&name).expect("abrir captura");
        drop(capture);
    }

    #[test]
    #[ignore = "requer servidor Pulse/PipeWire e reprodução de áudio"]
    fn captures_default_monitor_when_audio_plays() {
        use std::io::Write;
        use std::sync::atomic::AtomicUsize;

        struct Counter(AtomicUsize);
        impl AudioConsumer for Counter {
            fn consume(&self, samples: &[f32]) {
                self.0.fetch_add(samples.len(), Ordering::Relaxed);
            }
        }

        let source = PulseSystemAudioSource::new();
        let counter = Arc::new(Counter(AtomicUsize::new(0)));
        source.start(None, counter.clone()).expect("start");
        std::thread::sleep(std::time::Duration::from_millis(400));

        // Toca um tom no sink padrão para gerar dados no monitor.
        let path = std::env::temp_dir().join("ghostwhisper_tone.wav");
        let rate = 48_000u32;
        let samples = rate; // 1 s
        let data_len = samples * 2;
        {
            let mut file = std::fs::File::create(&path).expect("wav");
            file.write_all(b"RIFF").unwrap();
            file.write_all(&(36 + data_len).to_le_bytes()).unwrap();
            file.write_all(b"WAVEfmt ").unwrap();
            file.write_all(&16u32.to_le_bytes()).unwrap();
            file.write_all(&1u16.to_le_bytes()).unwrap();
            file.write_all(&1u16.to_le_bytes()).unwrap();
            file.write_all(&rate.to_le_bytes()).unwrap();
            file.write_all(&(rate * 2).to_le_bytes()).unwrap();
            file.write_all(&2u16.to_le_bytes()).unwrap();
            file.write_all(&16u16.to_le_bytes()).unwrap();
            file.write_all(b"data").unwrap();
            file.write_all(&data_len.to_le_bytes()).unwrap();
            for i in 0..samples {
                let t = i as f32 / rate as f32;
                let v = (t * 440.0 * std::f32::consts::PI * 2.0).sin() * 0.3;
                file.write_all(&((v * 32767.0) as i16).to_le_bytes())
                    .unwrap();
            }
        }
        let _ = std::process::Command::new("paplay").arg(&path).status();
        std::thread::sleep(std::time::Duration::from_millis(300));
        source.stop().expect("stop");
        let _ = std::fs::remove_file(&path);

        let captured = counter.0.load(Ordering::Relaxed);
        assert!(captured > 0, "nenhuma amostra capturada do monitor");
    }
}
