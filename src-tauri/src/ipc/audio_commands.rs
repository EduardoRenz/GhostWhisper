//! Comandos IPC de áudio (microfone e áudio do sistema).

use std::sync::Arc;

use serde::Serialize;
use tauri::{AppHandle, State};

use crate::adapters::audio::level_meter::LevelMeter;
use crate::domain::audio::AudioDevice;
use crate::error::Result;
use crate::ports::audio_consumer::AudioConsumer;
use crate::ports::session_controller::AudioChannel;
use crate::state::AppState;

/// Dispositivo de áudio exposto à UI.
#[derive(Debug, Serialize)]
pub struct AudioDeviceDto {
    pub id: String,
    pub name: String,
    pub is_default: bool,
}

impl From<AudioDevice> for AudioDeviceDto {
    fn from(value: AudioDevice) -> Self {
        Self {
            id: value.id,
            name: value.name,
            is_default: value.is_default,
        }
    }
}

fn level_meter(app: AppHandle, channel: AudioChannel) -> Arc<dyn AudioConsumer> {
    Arc::new(LevelMeter::new(app, channel))
}

// --- Microfone ---------------------------------------------------------------

/// Lista os dispositivos de entrada de áudio.
#[tauri::command]
pub fn audio_list_devices(state: State<'_, AppState>) -> Result<Vec<AudioDeviceDto>> {
    Ok(state
        .audio
        .list_devices()?
        .into_iter()
        .map(AudioDeviceDto::from)
        .collect())
}

/// Inicia a captura do microfone (`device_id` omitido = padrão).
#[tauri::command]
pub fn audio_start(
    app: AppHandle,
    state: State<'_, AppState>,
    device_id: Option<String>,
) -> Result<()> {
    state
        .audio
        .start(device_id, level_meter(app, AudioChannel::Microphone))
}

/// Interrompe a captura do microfone.
#[tauri::command]
pub fn audio_stop(state: State<'_, AppState>) -> Result<()> {
    state.audio.stop()
}

/// Informa se há captura de microfone ativa.
#[tauri::command]
pub fn audio_is_running(state: State<'_, AppState>) -> Result<bool> {
    Ok(state.audio.is_running())
}

// --- Áudio do sistema --------------------------------------------------------

/// Lista os monitores de saída disponíveis (áudio do sistema).
#[tauri::command]
pub fn system_audio_list_devices(state: State<'_, AppState>) -> Result<Vec<AudioDeviceDto>> {
    Ok(state
        .system_audio
        .list_devices()?
        .into_iter()
        .map(AudioDeviceDto::from)
        .collect())
}

/// Inicia a captura do áudio do sistema (`device_id` omitido = monitor padrão).
#[tauri::command]
pub fn system_audio_start(
    app: AppHandle,
    state: State<'_, AppState>,
    device_id: Option<String>,
) -> Result<()> {
    state
        .system_audio
        .start(device_id, level_meter(app, AudioChannel::System))
}

/// Interrompe a captura do áudio do sistema.
#[tauri::command]
pub fn system_audio_stop(state: State<'_, AppState>) -> Result<()> {
    state.system_audio.stop()
}

/// Informa se há captura de áudio do sistema ativa.
#[tauri::command]
pub fn system_audio_is_running(state: State<'_, AppState>) -> Result<bool> {
    Ok(state.system_audio.is_running())
}

/// Cria um sink virtual "GhostWhisper" (retorna o índice do módulo).
#[tauri::command]
pub fn system_sink_create(state: State<'_, AppState>, name: String) -> Result<u32> {
    state.virtual_sink.create(&name)
}

/// Remove o sink virtual pelo índice do módulo.
#[tauri::command]
pub fn system_sink_remove(state: State<'_, AppState>, module_index: u32) -> Result<()> {
    state.virtual_sink.remove(module_index)
}
