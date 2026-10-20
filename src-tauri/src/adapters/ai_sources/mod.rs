//! Adaptadores das fontes de IA.
//!
//! A implementação de produção vive na camada de aplicação
//! (`application::ai_source_service`); aqui ficam as **dobras de teste** da
//! porta `AiSourceService` (R8).

#[cfg(test)]
pub mod fake_ai_source_service;
#[cfg(test)]
pub mod in_memory_ai_source_service;
