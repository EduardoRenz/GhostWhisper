//! Provider DeepSeek (BR-046).
//!
//! A API do DeepSeek é compatível com a de chat da OpenAI
//! (`POST {base_url}/chat/completions` com `Authorization: Bearer`), então o
//! adapter reusa [`super::openai_compatible`].
//!
//! Diferença relevante: o `deepseek-reasoner` **não** aceita `temperature`
//! fora do padrão — o parâmetro nem é enviado (ver
//! `openai_compatible::temperature_supported`).

use crate::domain::llm::{Completion, CompletionRequest, ProviderConfig};
use crate::error::Result;
use crate::ports::chat_bot_provider::ChatBotProvider;

use super::openai_compatible::{chat_completion, chat_completion_streaming};

/// Endpoint oficial do DeepSeek (formato OpenAI).
const BASE_URL: &str = "https://api.deepseek.com/v1";

/// Provider DeepSeek.
pub struct DeepSeekChatBotProvider {
    config: ProviderConfig,
}

impl DeepSeekChatBotProvider {
    /// Cria o provider com a configuração resolvida.
    pub fn new(config: ProviderConfig) -> Self {
        Self { config }
    }
}

impl ChatBotProvider for DeepSeekChatBotProvider {
    fn id(&self) -> String {
        format!("deepseek:{}", self.config.model)
    }

    fn complete(&self, request: &CompletionRequest) -> Result<Completion> {
        chat_completion(&self.config, BASE_URL, request, &[])
    }

    fn complete_streaming(
        &self,
        request: &CompletionRequest,
        on_text: &mut dyn FnMut(&str),
    ) -> Result<Completion> {
        chat_completion_streaming(&self.config, BASE_URL, request, &[], on_text)
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

    /// Sobe um servidor HTTP de uma requisição e devolve a base URL.
    fn spawn_mock(body: &'static str, seen: Arc<Mutex<String>>) -> String {
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
                if let Some(pos) = buffer.windows(4).position(|window| window == b"\r\n\r\n") {
                    let headers = String::from_utf8_lossy(&buffer[..pos]);
                    for line in headers.lines() {
                        if let Some(value) =
                            line.to_ascii_lowercase().strip_prefix("content-length:")
                        {
                            content_length = value.trim().parse().unwrap_or(0);
                        }
                    }
                    let header_len = pos + 4;
                    if buffer.len() >= header_len + content_length {
                        break;
                    }
                }
            }
            if let Ok(mut guard) = seen.lock() {
                *guard = String::from_utf8_lossy(&buffer).to_string();
            }
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = stream.write_all(response.as_bytes());
            let _ = stream.flush();
        });
        format!("http://{addr}")
    }

    #[test]
    fn calls_the_openai_compatible_endpoint() {
        let seen = Arc::new(Mutex::new(String::new()));
        let base_url = spawn_mock(
            r#"{"choices":[{"message":{"content":"olá"}}]}"#,
            Arc::clone(&seen),
        );

        // O `base_url` do provider é fixo; o teste usa o endpoint local
        // chamando o mesmo helper com a URL do mock.
        let config = ProviderConfig {
            kind: ProviderKind::DeepSeek,
            model: "deepseek-chat".into(),
            base_url: Some(base_url),
            api_key: "sk-deepseek".into(),
        };
        let completion = chat_completion(
            &config,
            config.base_url.as_deref().expect("base_url"),
            &CompletionRequest::user("oi"),
            &[],
        )
        .expect("complete");
        assert_eq!(completion.text, "olá");

        let request = seen.lock().expect("lock").clone();
        assert!(request.contains("POST /chat/completions"));
        assert!(request.contains("Bearer sk-deepseek"));
        assert!(request.contains("deepseek-chat"));
    }

    #[test]
    fn id_carries_the_model() {
        let provider = DeepSeekChatBotProvider::new(ProviderConfig {
            kind: ProviderKind::DeepSeek,
            model: "deepseek-reasoner".into(),
            base_url: None,
            api_key: "k".into(),
        });
        assert_eq!(provider.id(), "deepseek:deepseek-reasoner");
    }
}
