//! Provider custom (endpoint local/self-hosted compatível com OpenAI).

use crate::domain::llm::{Completion, CompletionRequest, ProviderConfig};
use crate::error::{GhostWhisperError, Result};
use crate::ports::chat_bot_provider::ChatBotProvider;

use super::openai_compatible::{chat_completion, chat_completion_streaming};

/// Provider para `base_url` configurável (ex.: Ollama, LM Studio, vLLM).
pub struct CustomChatBotProvider {
    config: ProviderConfig,
}

impl CustomChatBotProvider {
    /// Cria o provider com a configuração resolvida.
    pub fn new(config: ProviderConfig) -> Self {
        Self { config }
    }

    /// `base_url` obrigatória do endpoint custom.
    fn base_url(&self) -> Result<&str> {
        self.config
            .base_url
            .as_deref()
            .filter(|url| !url.trim().is_empty())
            .ok_or_else(|| {
                GhostWhisperError::Config("provider custom exige base_url configurada".to_string())
            })
    }
}

impl ChatBotProvider for CustomChatBotProvider {
    fn id(&self) -> String {
        format!("custom:{}", self.config.model)
    }

    fn complete(&self, request: &CompletionRequest) -> Result<Completion> {
        let base_url = self.base_url()?;
        chat_completion(&self.config, base_url, request, &[])
    }

    fn complete_streaming(
        &self,
        request: &CompletionRequest,
        on_text: &mut dyn FnMut(&str),
    ) -> Result<Completion> {
        let base_url = self.base_url()?;
        chat_completion_streaming(&self.config, base_url, request, &[], on_text)
    }
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};
    use std::thread;

    use super::*;
    use crate::domain::settings::ProviderKind;

    fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
        haystack
            .windows(needle.len())
            .position(|window| window == needle)
    }

    /// Sobe um servidor HTTP de uma requisição e devolve sua base URL.
    fn spawn_mock(
        status_line: &'static str,
        body: &'static str,
        seen: Arc<Mutex<String>>,
    ) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("addr");
        thread::spawn(move || {
            let Ok((mut stream, _)) = listener.accept() else {
                return;
            };
            let mut buffer = Vec::new();
            let mut chunk = [0u8; 1024];
            let mut content_length = 0usize;
            while let Ok(read) = stream.read(&mut chunk) {
                if read == 0 {
                    break;
                }
                buffer.extend_from_slice(&chunk[..read]);
                if let Some(pos) = find(&buffer, b"\r\n\r\n") {
                    let headers = String::from_utf8_lossy(&buffer[..pos]);
                    for line in headers.lines() {
                        if let Some(value) =
                            line.to_ascii_lowercase().strip_prefix("content-length:")
                        {
                            content_length = value.trim().parse().unwrap_or(0);
                        }
                    }
                    let header_len = pos + 4;
                    while buffer.len() < header_len + content_length {
                        match stream.read(&mut chunk) {
                            Ok(0) | Err(_) => break,
                            Ok(read) => buffer.extend_from_slice(&chunk[..read]),
                        }
                    }
                    break;
                }
            }
            if let Ok(mut guard) = seen.lock() {
                *guard = String::from_utf8_lossy(&buffer).to_string();
            }
            let response = format!(
                "{status_line}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = stream.write_all(response.as_bytes());
            let _ = stream.flush();
        });
        format!("http://{addr}")
    }

    fn provider(base_url: String) -> CustomChatBotProvider {
        CustomChatBotProvider::new(ProviderConfig {
            kind: ProviderKind::Custom,
            model: "local-model".into(),
            base_url: Some(base_url),
            api_key: "test-key".into(),
        })
    }

    #[test]
    fn calls_openai_compatible_endpoint() {
        let seen = Arc::new(Mutex::new(String::new()));
        let base_url = spawn_mock(
            "HTTP/1.1 200 OK",
            "{\"choices\":[{\"message\":{\"content\":\"olá\"}}]}",
            Arc::clone(&seen),
        );

        let completion = provider(base_url)
            .complete(&CompletionRequest::user("oi"))
            .expect("complete");
        assert_eq!(completion.text, "olá");
        assert_eq!(completion.model, "local-model");

        let request = seen.lock().expect("lock").clone();
        assert!(request.contains("POST /chat/completions"));
        assert!(request.contains("Bearer test-key"));
        assert!(request.contains("local-model"));
    }

    #[test]
    fn maps_http_error_status() {
        let seen = Arc::new(Mutex::new(String::new()));
        let base_url = spawn_mock(
            "HTTP/1.1 401 Unauthorized",
            "{\"error\":\"invalid key\"}",
            Arc::clone(&seen),
        );

        let error = provider(base_url)
            .complete(&CompletionRequest::user("oi"))
            .unwrap_err();
        let message = error.to_string();
        assert!(message.contains("401"));
        // A chave nunca aparece na mensagem de erro.
        assert!(!message.contains("test-key"));
    }

    #[test]
    fn requires_base_url() {
        let provider = CustomChatBotProvider::new(ProviderConfig {
            kind: ProviderKind::Custom,
            model: "m".into(),
            base_url: None,
            api_key: "k".into(),
        });
        assert!(provider.complete(&CompletionRequest::user("oi")).is_err());
    }
}
