//! Porta de tradução de texto (inbound port / caso de uso).

use crate::error::Result;

/// Traduz um trecho para o idioma de destino configurado.
///
/// Implementado pela aplicação (`TranslationService`) e chamado pelo IPC.
pub trait Translator: Send + Sync {
    /// Traduz `text`; erros do provider são repassados sem vazar segredos.
    fn translate(&self, text: &str) -> Result<String>;
}
