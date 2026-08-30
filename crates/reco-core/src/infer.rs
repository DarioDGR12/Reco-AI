//! Pluggable inference: local llama-cli, OpenAI-compatible HTTP, or echo (demo).

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::chat::{format_chatml, ChatMessage, ChatRole};
use crate::config::RecoConfig;

#[derive(Debug)]
pub struct InferError(pub String);

impl std::fmt::Display for InferError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for InferError {}

pub trait InferEngine: Send + Sync {
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
        let last = last_user(messages);
        if last.is_empty() {
            return Ok(format!(
                "Reco AI · {} listo. Escribe algo para empezar.",
                self.model_label
            ));
        }
        Ok(format!(
            "Reco AI · modo demo · {}\nRecibí: «{last}».\nPara hablar de verdad: instala llama.cpp (llama-cli en PATH) o `reco config set openai-key`.",
            self.model_label
        ))
    }
}

/// Spawn `llama-cli` / `llama-completion` (llama.cpp).
#[derive(Debug, Clone)]
pub struct LlamaCliEngine {
    pub binary: PathBuf,
    pub model_path: PathBuf,
    pub n_predict: u32,
    pub n_ctx: u32,
    pub n_gpu_layers: i32,
}

impl LlamaCliEngine {
    pub fn find_binary(explicit: Option<&str>) -> Option<PathBuf> {
        if let Some(raw) = explicit.filter(|s| !s.is_empty()) {
            let path = PathBuf::from(raw);
            if path.is_file() {
                return Some(path);
            }
            if let Some(found) = which(raw) {
                return Some(found);
            }
        }
        if let Ok(env) = std::env::var("RECO_LLAMA_CLI") {
            let path = PathBuf::from(&env);
            if path.is_file() {
                return Some(path);
            }
            if let Some(found) = which(&env) {
                return Some(found);
            }
        }
        for name in ["llama-cli", "llama-completion", "main"] {
            if let Some(found) = which(name) {
                return Some(found);
            }
        }
        None
    }
}

impl InferEngine for LlamaCliEngine {
    fn generate(&mut self, messages: &[ChatMessage]) -> Result<String, InferError> {
        if messages.is_empty() {
            return Ok(format!(
                "Reco AI · llama-cli · {} listo.",
                self.model_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("modelo")
            ));
        }
        let prompt = format_chatml(messages);
        let output = Command::new(&self.binary)
            .arg("-m")
            .arg(&self.model_path)
            .arg("-n")
            .arg(self.n_predict.to_string())
            .arg("-c")
            .arg(self.n_ctx.to_string())
            .arg("-ngl")
            .arg(self.n_gpu_layers.to_string())
            .arg("-p")
            .arg(&prompt)
            .arg("--log-disable")
            .output()
            .map_err(|err| InferError(format!("no pude lanzar llama-cli: {err}")))?;
        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            return Err(InferError(format!(
                "llama-cli salió con {}: {}",
                output.status,
                err.chars().take(400).collect::<String>()
            )));
        }
        let raw = String::from_utf8_lossy(&output.stdout);
        let cleaned = strip_prompt_echo(&raw, &prompt);
        if cleaned.trim().is_empty() {
            Ok(raw.trim().to_string())
        } else {
            Ok(cleaned.trim().to_string())
        }
    }
}

fn strip_prompt_echo(output: &str, prompt: &str) -> String {
    if let Some(rest) = output.strip_prefix(prompt) {
        return rest.to_string();
    }
    if let Some(idx) = output.find("<|im_start|>assistant") {
        return output[idx..]
            .trim_start_matches("<|im_start|>assistant")
            .trim_start_matches('\n')
            .to_string();
    }
    output.to_string()
}

/// Chat Completions compatible (OpenAI, Groq, local proxies) or Anthropic Messages.
#[derive(Debug, Clone)]
pub struct OpenAiCompatibleEngine {
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    pub anthropic: bool,
}

impl InferEngine for OpenAiCompatibleEngine {
    fn generate(&mut self, messages: &[ChatMessage]) -> Result<String, InferError> {
        if messages.is_empty() {
            let kind = if self.anthropic { "anthropic" } else { "openai" };
            return Ok(format!("Reco AI · {kind} · {} listo.", self.model));
        }
        if self.anthropic {
            complete_anthropic(self, messages)
        } else {
            complete_openai(self, messages)
        }
    }
}

pub fn openai_chat_body(model: &str, messages: &[ChatMessage], max_tokens: u32) -> serde_json::Value {
    serde_json::json!({
        "model": model,
        "messages": messages.iter().map(|m| serde_json::json!({
            "role": m.role.as_str(),
            "content": m.content,
        })).collect::<Vec<_>>(),
        "max_tokens": max_tokens,
        "temperature": 0.7
    })
}

fn complete_openai(
    eng: &OpenAiCompatibleEngine,
    messages: &[ChatMessage],
) -> Result<String, InferError> {
    let url = format!("{}/chat/completions", eng.base_url.trim_end_matches('/'));
    let body = openai_chat_body(&eng.model, messages, 256);
    let resp: serde_json::Value = ureq::post(&url)
        .set("Authorization", &format!("Bearer {}", eng.api_key))
        .set("Content-Type", "application/json")
        .send_json(body)
        .map_err(|err| InferError(format!("OpenAI: {err}")))?
        .into_json()
        .map_err(|err| InferError(format!("OpenAI JSON: {err}")))?;
    resp["choices"][0]["message"]["content"]
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| InferError("respuesta OpenAI sin content".into()))
}

fn complete_anthropic(
    eng: &OpenAiCompatibleEngine,
    messages: &[ChatMessage],
) -> Result<String, InferError> {
    let url = format!("{}/v1/messages", eng.base_url.trim_end_matches('/'));
    let system = messages
        .iter()
        .filter(|m| m.role == ChatRole::System)
        .map(|m| m.content.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    let turns: Vec<serde_json::Value> = messages
        .iter()
        .filter(|m| m.role != ChatRole::System)
        .map(|m| {
            serde_json::json!({
                "role": if m.role == ChatRole::Assistant { "assistant" } else { "user" },
                "content": m.content,
            })
        })
        .collect();
    let body = serde_json::json!({
        "model": eng.model,
        "max_tokens": 256,
        "system": system,
        "messages": turns
    });
    let resp: serde_json::Value = ureq::post(&url)
        .set("x-api-key", &eng.api_key)
        .set("anthropic-version", "2023-06-01")
        .set("Content-Type", "application/json")
        .send_json(body)
        .map_err(|err| InferError(format!("Anthropic: {err}")))?
        .into_json()
        .map_err(|err| InferError(format!("Anthropic JSON: {err}")))?;
    let blocks = resp["content"]
        .as_array()
        .ok_or_else(|| InferError("Anthropic sin content".into()))?;
    let mut text = String::new();
    for block in blocks {
        if block["type"].as_str() == Some("text") {
            if let Some(t) = block["text"].as_str() {
                text.push_str(t);
            }
        }
    }
    if text.is_empty() {
        return Err(InferError("Anthropic vacío".into()));
    }
    Ok(text)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineKind {
    Echo,
    Llama,
    OpenAi,
    Anthropic,
    Auto,
}

impl EngineKind {
    pub fn parse(s: &str) -> Result<Self, InferError> {
        match s {
            "echo" | "demo" => Ok(Self::Echo),
            "llama" | "local" => Ok(Self::Llama),
            "openai" => Ok(Self::OpenAi),
            "anthropic" => Ok(Self::Anthropic),
            "auto" => Ok(Self::Auto),
            other => Err(InferError(format!(
                "proveedor desconocido: {other} (echo|llama|openai|anthropic|auto)"
            ))),
        }
    }
}

pub struct PickedEngine {
    pub engine: Box<dyn InferEngine>,
    pub label: String,
    pub hint: Option<String>,
}

/// Choose an engine from config, optional GGUF path, and provider override.
pub fn pick_engine(
    cfg: &RecoConfig,
    model_id: &str,
    gguf_path: Option<&Path>,
    kind: EngineKind,
) -> Result<PickedEngine, InferError> {
    match kind {
        EngineKind::Echo => Ok(echo_picked(model_id, true)),
        EngineKind::Llama => pick_llama(cfg, model_id, gguf_path),
        EngineKind::OpenAi => pick_openai(cfg),
        EngineKind::Anthropic => pick_anthropic(cfg),
        EngineKind::Auto => {
            if let Ok(picked) = pick_llama(cfg, model_id, gguf_path) {
                return Ok(picked);
            }
            if let Ok(picked) = pick_openai(cfg) {
                return Ok(picked);
            }
            if let Ok(picked) = pick_anthropic(cfg) {
                return Ok(picked);
            }
            Ok(echo_picked(model_id, false))
        }
    }
}

fn echo_picked(model_id: &str, explicit: bool) -> PickedEngine {
    let hint = if explicit {
        Some("modo demo (echo). Instala llama.cpp o configura BYOK.".into())
    } else {
        Some(
            "sin llama-cli ni BYOK: echo. `reco config set openai-key sk-...` o instala llama.cpp."
                .into(),
        )
    };
    PickedEngine {
        engine: Box::new(EchoEngine::new(model_id)),
        label: "echo".into(),
        hint,
    }
}

fn pick_llama(
    cfg: &RecoConfig,
    model_id: &str,
    gguf_path: Option<&Path>,
) -> Result<PickedEngine, InferError> {
    let path = gguf_path
        .filter(|p| p.is_file())
        .ok_or_else(|| InferError("no hay GGUF local para llama-cli".into()))?;
    let bin = LlamaCliEngine::find_binary(cfg.llama.cli.as_deref()).ok_or_else(|| {
        InferError(
            "llama-cli no está en PATH. Instala llama.cpp o `reco config set llama-cli /ruta`."
                .into(),
        )
    })?;
    Ok(PickedEngine {
        engine: Box::new(LlamaCliEngine {
            binary: bin,
            model_path: path.to_path_buf(),
            n_predict: cfg.llama.n_predict,
            n_ctx: cfg.llama.n_ctx,
            n_gpu_layers: cfg.llama.n_gpu_layers,
        }),
        label: format!("llama-cli · {model_id}"),
        hint: None,
    })
}

fn pick_openai(cfg: &RecoConfig) -> Result<PickedEngine, InferError> {
    let key = cfg
        .byok
        .openai_key
        .as_str()
        .trim();
    if key.is_empty() {
        return Err(InferError("falta openai-key (OPENAI_API_KEY)".into()));
    }
    Ok(PickedEngine {
        engine: Box::new(OpenAiCompatibleEngine {
            base_url: cfg.byok.openai_base.clone(),
            api_key: key.to_string(),
            model: cfg.byok.openai_model.clone(),
            anthropic: false,
        }),
        label: format!("openai · {}", cfg.byok.openai_model),
        hint: None,
    })
}

fn pick_anthropic(cfg: &RecoConfig) -> Result<PickedEngine, InferError> {
    let key = cfg.byok.anthropic_key.as_str().trim();
    if key.is_empty() {
        return Err(InferError("falta anthropic-key (ANTHROPIC_API_KEY)".into()));
    }
    Ok(PickedEngine {
        engine: Box::new(OpenAiCompatibleEngine {
            base_url: "https://api.anthropic.com".into(),
            api_key: key.to_string(),
            model: cfg.byok.anthropic_model.clone(),
            anthropic: true,
        }),
        label: format!("anthropic · {}", cfg.byok.anthropic_model),
        hint: None,
    })
}

/// Builds the prompt string engines should feed the model.
pub fn prompt_from_messages(messages: &[ChatMessage]) -> String {
    format_chatml(messages)
}

fn last_user(messages: &[ChatMessage]) -> &str {
    messages
        .iter()
        .rev()
        .find(|m| m.role == ChatRole::User)
        .map(|m| m.content.trim())
        .unwrap_or("")
}

fn which(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn openai_body_has_roles() {
        let body = openai_chat_body(
            "gpt-4o-mini",
            &[ChatMessage {
                role: ChatRole::User,
                content: "hi".into(),
            }],
            32,
        );
        assert_eq!(body["model"], "gpt-4o-mini");
        assert_eq!(body["messages"][0]["role"], "user");
        assert_eq!(body["max_tokens"], 32);
    }

    #[test]
    fn pick_auto_falls_to_echo() {
        let cfg = RecoConfig::default();
        let picked = pick_engine(&cfg, "demo", None, EngineKind::Auto).unwrap();
        assert_eq!(picked.label, "echo");
        assert!(picked.hint.is_some());
    }

    #[test]
    fn pick_openai_needs_key() {
        let cfg = RecoConfig::default();
        assert!(pick_engine(&cfg, "x", None, EngineKind::OpenAi).is_err());
    }

    #[test]
    fn pick_openai_with_key() {
        let mut cfg = RecoConfig::default();
        cfg.byok.openai_key = "sk-test".into();
        let picked = pick_engine(&cfg, "x", None, EngineKind::OpenAi).unwrap();
        assert!(picked.label.starts_with("openai"));
    }

    #[test]
    fn strip_echoed_prompt() {
        assert_eq!(strip_prompt_echo("PROMPTyes", "PROMPT"), "yes");
    }

    #[test]
    fn engine_kind_rejects_unknown() {
        assert!(EngineKind::parse("nope").is_err());
        assert_eq!(EngineKind::parse("local").unwrap(), EngineKind::Llama);
    }

    #[test]
    fn llama_cli_finds_explicit_file() {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("reco-llama-find-{stamp}"));
        std::fs::create_dir_all(&dir).unwrap();
        let fake = dir.join("llama-cli");
        std::fs::write(&fake, b"#!/bin/sh\necho ok\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&fake).unwrap().permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&fake, perms).unwrap();
        }
        let found = LlamaCliEngine::find_binary(Some(fake.to_str().unwrap()));
        assert_eq!(found.as_deref(), Some(fake.as_path()));
        let _ = std::fs::remove_dir_all(dir);
    }
}
