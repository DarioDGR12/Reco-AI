use crate::chat::{format_chatml, ChatMessage};

#[derive(Debug)]
pub struct InferError(pub String);

impl std::fmt::Display for InferError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for InferError {}

pub trait InferEngine {
    fn generate(&mut self, messages: &[ChatMessage]) -> Result<String, InferError>;
}

/// Deterministic engine for tests, `--demo`, and GIFs.
pub struct EchoEngine {
    pub model_label: String,
}

impl EchoEngine {
    pub fn new(model_label: impl Into<String>) -> Self {
        Self {
            model_label: model_label.into(),
        }
    }
}

impl InferEngine for EchoEngine {
    fn generate(&mut self, messages: &[ChatMessage]) -> Result<String, InferError> {
        let last = messages
            .iter()
            .rev()
            .find(|m| m.role == crate::chat::ChatRole::User)
            .map(|m| m.content.trim())
            .unwrap_or("");
        if last.is_empty() {
            return Ok(format!(
                "Reco AI · {} listo. Escribe algo para empezar.",
                self.model_label
            ));
        }
        Ok(format!(
            "[{}] (demo, sin llama.cpp) Recibí: «{last}».\nCuando compiles con --features llama y tengas el GGUF local, esta respuesta será del modelo.",
            self.model_label
        ))
    }
}

/// Builds the prompt string engines should feed the model.
pub fn prompt_from_messages(messages: &[ChatMessage]) -> String {
    format_chatml(messages)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chat::ChatRole;

    #[test]
    fn echo_mentions_user_text() {
        let mut engine = EchoEngine::new("Qwen-7B");
        let reply = engine
            .generate(&[ChatMessage {
                role: ChatRole::User,
                content: "explica Q4_K_M".into(),
            }])
            .unwrap();
        assert!(reply.contains("Q4_K_M"));
        assert!(reply.contains("demo"));
    }
}
