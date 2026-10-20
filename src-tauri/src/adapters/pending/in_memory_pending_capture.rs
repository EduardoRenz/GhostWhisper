//! Print pendente em memória (transitório por definição).
//!
//! Não há persistência: o print anexado existe só até a pergunta ser enviada
//! ou removida (BR-055). O que fica no banco é a cópia ligada à entrada do
//! diário.

use std::sync::Mutex;

use crate::error::{GhostWhisperError, Result};
use crate::ports::pending_capture::PendingCapture;
use crate::ports::screen_capture::CapturedImage;

/// Guarda o print pendente em memória.
#[derive(Default)]
pub struct InMemoryPendingCapture {
    image: Mutex<Option<CapturedImage>>,
}

impl InMemoryPendingCapture {
    /// Cria um armazenamento vazio.
    pub fn new() -> Self {
        Self::default()
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Option<CapturedImage>>> {
        self.image
            .lock()
            .map_err(|_| GhostWhisperError::Other("lock do print envenenado".to_string()))
    }
}

impl PendingCapture for InMemoryPendingCapture {
    fn set(&self, image: CapturedImage) -> Result<()> {
        *self.lock()? = Some(image);
        Ok(())
    }

    fn get(&self) -> Result<Option<CapturedImage>> {
        Ok(self.lock()?.clone())
    }

    fn clear(&self) -> Result<()> {
        *self.lock()? = None;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn image() -> CapturedImage {
        CapturedImage {
            mime: "image/jpeg".to_string(),
            data: vec![1, 2, 3],
            width: 1280,
            height: 720,
        }
    }

    #[test]
    fn stores_reads_and_clears() {
        let store = InMemoryPendingCapture::new();
        assert!(store.get().expect("get").is_none());

        store.set(image()).expect("set");
        assert_eq!(
            store.get().expect("get").expect("presente").data,
            vec![1, 2, 3]
        );

        store.clear().expect("clear");
        assert!(store.get().expect("get").is_none());
    }
}
