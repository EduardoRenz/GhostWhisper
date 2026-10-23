//! Dobras de teste compartilhadas da camada de aplicação.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::domain::ai_route::AiRouteScope;
use crate::domain::llm::{Completion, CompletionRequest};
use crate::domain::transcription::{SpeechQuality, TranscribedSpan};
use crate::error::{GhostWhisperError, Result};
use crate::ports::llm_controller::LlmController;
use crate::ports::speech_to_text::SpeechToText;

/// Lê uma requisição HTTP completa de `stream` e devolve os bytes brutos.
fn read_request(stream: &mut impl Read) -> String {
    let mut buffer = Vec::new();
    let mut chunk = [0u8; 1024];
    let mut content_length = 0usize;
    while let Ok(read) = stream.read(&mut chunk) {
        if read == 0 {
            break;
        }
        buffer.extend_from_slice(&chunk[..read]);
        if let Some(pos) = buffer.windows(4).position(|w| w == b"\r\n\r\n") {
            let headers = String::from_utf8_lossy(&buffer[..pos]);
            for line in headers.lines() {
                if let Some(value) = line.to_ascii_lowercase().strip_prefix("content-length:") {
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
    String::from_utf8_lossy(&buffer).to_string()
}

/// Sobe um servidor HTTP de uma requisição (formato OpenAI) e devolve a base
/// URL e o texto bruto da requisição recebida.
pub fn spawn_openai_mock(body: &'static str) -> (String, Arc<Mutex<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().expect("addr");
    let seen = Arc::new(Mutex::new(String::new()));
    let captured = Arc::clone(&seen);

    thread::spawn(move || {
        let Ok((mut stream, _)) = listener.accept() else {
            return;
        };
        let raw = read_request(&mut stream);
        if let Ok(mut guard) = captured.lock() {
            *guard = raw;
        }
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        let _ = stream.write_all(response.as_bytes());
        let _ = stream.flush();
    });

    (format!("http://{addr}"), seen)
}

/// Sobe um servidor que responde em sequência a cada requisição recebida,
/// devolvendo a base URL e o texto bruto de **todas** as requisições.
pub fn spawn_openai_sequence_mock(
    replies: Vec<(u16, String)>,
) -> (String, Arc<Mutex<Vec<String>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().expect("addr");
    let seen: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let captured = Arc::clone(&seen);

    thread::spawn(move || {
        for (status, body) in replies {
            let Ok((mut stream, _)) = listener.accept() else {
                return;
            };
            let raw = read_request(&mut stream);
            if let Ok(mut guard) = captured.lock() {
                guard.push(raw);
            }
            let reason = if status == 200 { "OK" } else { "Bad Request" };
            let response = format!(
                "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = stream.write_all(response.as_bytes());
            let _ = stream.flush();
        }
    });

    (format!("http://{addr}"), seen)
}

/// `LlmController` de teste: resposta fixa, fila opcional, falha e captura dos pedidos.
pub struct FakeLlmController {
    response: Mutex<String>,
    responses: Mutex<Vec<String>>,
    failure: Mutex<Option<String>>,
    requests: Mutex<Vec<CompletionRequest>>,
    scopes: Mutex<Vec<AiRouteScope>>,
    /// Atraso artificial do passe rápido (`complete_for`).
    quick_delay: Mutex<Option<Duration>>,
    /// Atraso artificial do passe completo (`complete_streaming_for`).
    full_delay: Mutex<Option<Duration>>,
}

impl FakeLlmController {
    /// Cria o fake com resposta `"ok"`.
    pub fn new() -> Self {
        Self {
            response: Mutex::new("ok".to_string()),
            responses: Mutex::new(Vec::new()),
            failure: Mutex::new(None),
            requests: Mutex::new(Vec::new()),
            scopes: Mutex::new(Vec::new()),
            quick_delay: Mutex::new(None),
            full_delay: Mutex::new(None),
        }
    }

    /// Define a resposta devolvida (enquanto a fila estiver vazia).
    pub fn with_response(self, response: impl Into<String>) -> Self {
        *self.response.lock().unwrap() = response.into();
        self
    }

    /// Define uma fila de respostas, consumida uma por chamada.
    pub fn with_responses(self, responses: Vec<String>) -> Self {
        *self.responses.lock().unwrap() = responses;
        self
    }

    /// Faz `complete` falhar com a mensagem dada.
    pub fn with_failure(self, message: impl Into<String>) -> Self {
        *self.failure.lock().unwrap() = Some(message.into());
        self
    }

    /// Atrasa o passe rápido para testar a ordem rascunho × completo.
    pub fn with_quick_delay(self, delay: Duration) -> Self {
        *self.quick_delay.lock().unwrap() = Some(delay);
        self
    }

    /// Atrasa o passe completo para garantir que o rascunho chegue antes.
    pub fn with_full_delay(self, delay: Duration) -> Self {
        *self.full_delay.lock().unwrap() = Some(delay);
        self
    }

    /// Último pedido recebido (para inspeção em testes).
    pub fn last_request(&self) -> Option<CompletionRequest> {
        self.requests.lock().unwrap().last().cloned()
    }

    /// Todos os pedidos recebidos, em ordem.
    pub fn requests(&self) -> Vec<CompletionRequest> {
        self.requests.lock().unwrap().clone()
    }

    /// Quantidade de chamadas.
    pub fn call_count(&self) -> usize {
        self.requests.lock().unwrap().len()
    }

    /// Escopos pedidos (BR-048), em ordem.
    pub fn scopes(&self) -> Vec<AiRouteScope> {
        self.scopes.lock().unwrap().clone()
    }
}

impl Default for FakeLlmController {
    fn default() -> Self {
        Self::new()
    }
}

impl LlmController for FakeLlmController {
    fn provider_label(&self) -> Result<String> {
        Ok("fake:test".into())
    }

    fn validate(&self) -> Result<String> {
        Ok("ok".into())
    }

    fn complete(&self, request: &CompletionRequest) -> Result<Completion> {
        self.requests.lock().unwrap().push(request.clone());
        self.scopes.lock().unwrap().push(AiRouteScope::Default);
        self.respond()
    }

    fn complete_for(
        &self,
        scope: &AiRouteScope,
        request: &CompletionRequest,
    ) -> Result<Completion> {
        self.requests.lock().unwrap().push(request.clone());
        self.scopes.lock().unwrap().push(scope.clone());
        if let Some(delay) = *self.quick_delay.lock().unwrap() {
            thread::sleep(delay);
        }
        self.respond()
    }

    fn complete_streaming_for(
        &self,
        scope: &AiRouteScope,
        request: &CompletionRequest,
        on_text: &mut dyn FnMut(&str),
    ) -> Result<Completion> {
        self.requests.lock().unwrap().push(request.clone());
        self.scopes.lock().unwrap().push(scope.clone());
        if let Some(delay) = *self.full_delay.lock().unwrap() {
            thread::sleep(delay);
        }
        let completion = self.respond()?;
        on_text(&completion.text);
        Ok(completion)
    }
}

impl FakeLlmController {
    /// Resposta programada (fila, fixa ou falha).
    fn respond(&self) -> Result<Completion> {
        if let Some(message) = self.failure.lock().unwrap().clone() {
            return Err(GhostWhisperError::Other(message));
        }
        let queued = {
            let mut responses = self.responses.lock().unwrap();
            if responses.is_empty() {
                None
            } else {
                Some(responses.remove(0))
            }
        };
        let text = queued.unwrap_or_else(|| self.response.lock().unwrap().clone());
        Ok(Completion {
            text,
            model: "fake".into(),
            provider: None,
            usage: None,
        })
    }
}

/// `SpeechToText` de teste: resposta fixa e captura das qualidades pedidas.
pub struct FakeSpeechToText {
    response: Mutex<String>,
    qualities: Mutex<Vec<SpeechQuality>>,
    calls: AtomicUsize,
}

impl FakeSpeechToText {
    /// Cria o fake com resposta `"ok"`.
    pub fn new() -> Self {
        Self {
            response: Mutex::new("ok".into()),
            qualities: Mutex::new(Vec::new()),
            calls: AtomicUsize::new(0),
        }
    }

    /// Define o texto retornado.
    pub fn with_response(self, response: impl Into<String>) -> Self {
        *self.response.lock().unwrap() = response.into();
        self
    }

    /// Qualidades pedidas, em ordem.
    pub fn qualities(&self) -> Vec<SpeechQuality> {
        self.qualities.lock().unwrap().clone()
    }

    /// Quantidade de chamadas.
    pub fn call_count(&self) -> usize {
        self.calls.load(Ordering::Relaxed)
    }
}

impl Default for FakeSpeechToText {
    fn default() -> Self {
        Self::new()
    }
}

impl SpeechToText for FakeSpeechToText {
    fn transcribe(&self, _samples: &[f32], quality: SpeechQuality) -> Result<Vec<TranscribedSpan>> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        self.qualities.lock().unwrap().push(quality);
        Ok(vec![TranscribedSpan {
            text: self.response.lock().unwrap().clone(),
            start_ms: 0,
            end_ms: 500,
        }])
    }
}
