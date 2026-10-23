//! Adaptadores de sistema (integração com o SO).

pub mod env_locale_source;
#[cfg(test)]
pub mod fake_locale_source;
pub mod system_clock;
pub mod vulkan_gpu_probe;
pub mod x11_pointer_source;
