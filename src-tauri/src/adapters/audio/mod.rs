//! Adaptadores de áudio (captura, conversão e consumo).

pub mod cpal_microphone_source;
pub mod level_meter;
pub mod microphone_front_end;
pub mod pactl;
pub mod pactl_sink_helper;
pub mod pcm_resampler;
pub mod pulse_system_audio_source;
