use std::io::Cursor;
use std::time::{SystemTime, UNIX_EPOCH};

use reco_core::chat::{ChatMessage, ChatRole};
use reco_core::config::RecoConfig;
use reco_core::infer::{InferEngine, PickedEngine};
use reco_core::Recommendation;
use serde::{Deserialize, Serialize};
use tiny_http::{Header, Method, Request, Response, Server};

#[derive(Deserialize)]
struct ChatRequest {
    #[serde(default)]
    messages: Vec<ApiMessage>,
    #[serde(default)]
    stream: bool,
}

#[derive(Deserialize)]
struct ApiMessage {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct ChatResponse {
    id: String,
    object: &'static str,
    created: u64,
    model: String,
    choices: Vec<Choice>,
    usage: Usage,
}

#[derive(Serialize)]
struct Choice {
    index: u32,
    message: ApiOutMessage,
    finish_reason: &'static str,
}

#[derive(Serialize)]
struct ApiOutMessage {
    role: &'static str,
    content: String,
}

#[derive(Serialize)]
struct Usage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

#[derive(Serialize)]
struct ModelsResponse {
    object: &'static str,
    data: Vec<ModelCard>,
}

#[derive(Serialize)]
struct ModelCard {
    id: String,
    object: &'static str,
    owned_by: &'static str,
}

const LANDING: &str = r#"<!doctype html>
<html lang="es"><head><meta charset="utf-8"><title>Reco serve</title>
<style>
  body{font:16px/1.5 ui-sans-serif,system-ui;background:#1e1e2e;color:#cdd6f4;margin:0;padding:48px}
  a{color:#cba6f7} code{background:#313244;padding:2px 6px;border-radius:4px}
  pre{background:#313244;padding:16px;border-radius:8px;overflow:auto}
</style></head><body>
<h1>Reco serve</h1>
<p>API local compatible con OpenAI. Autoriza con <code>Authorization: Bearer sk-reco-…</code>.</p>
<ul>
  <li><code>GET /health</code></li>
  <li><code>GET /v1/models</code></li>
  <li><code>POST /v1/chat/completions</code> — acepta <code>stream: true</code></li>
</ul>
<p>Úsalo en Continue, Open WebUI, Cursor u otro cliente que hable Chat Completions.</p>
</body></html>"#;

pub fn run(
    rec: &Recommendation,
    port: u16,
    host: &str,
    mut picked: PickedEngine,
) -> Result<(), String> {
    let key = load_or_create_key();
    let addr = format!("{host}:{port}");
    let server = Server::http(&addr).map_err(|err| err.to_string())?;

    println!("Reco serve");
    println!("  URL      http://{addr}");
    println!("  Docs     http://{addr}/");
    println!("  Modelo   {}", rec.repo_id);
    println!("  Quant    {}", rec.quant.label());
    println!("  Motor    {}", picked.label);
    println!("  API key  {key}");
    if let Some(hint) = &picked.hint {
        println!("  Nota     {hint}");
    }
    println!();
    println!("  curl http://{addr}/v1/chat/completions \\");
    println!("    -H \"Authorization: Bearer {key}\" \\");
    println!("    -H \"Content-Type: application/json\" \\");
    println!("    -d '{{\"messages\":[{{\"role\":\"user\",\"content\":\"hola\"}}]}}'");
    println!();
    println!("  Ctrl+C para parar. La clave se guarda en la config y se reutiliza.");

    for request in server.incoming_requests() {
        if let Err(err) = handle(request, rec, &key, &mut *picked.engine) {
            eprintln!("serve: {err}");
        }
    }
    Ok(())
}

fn load_or_create_key() -> String {
    let live = RecoConfig::load();
    if live.serve.api_key.starts_with("sk-reco-") {
        return live.serve.api_key;
    }
    let key = new_api_key();
    let mut disk = RecoConfig::load_file();
    disk.serve.api_key = key.clone();
    let _ = disk.save();
    key
}

fn handle(
    mut request: Request,
    rec: &Recommendation,
    key: &str,
    engine: &mut dyn InferEngine,
) -> Result<(), String> {
    let url = request.url().to_string();
    let path = url.split('?').next().unwrap_or(&url);
    let method = request.method().clone();

    if method == Method::Options {
        return send_cors(request, 204, "text/plain", "");
    }

    let open = matches!(path, "/" | "/health" | "/docs");
    if !open && !authorized(&request, key) {
        return send_cors(
            request,
            401,
            "application/json",
            r#"{"error":{"message":"API key inválida","type":"auth"}}"#,
        );
    }

    match (method, path) {
        (Method::Get, "/") | (Method::Get, "/docs") => {
            send_cors(request, 200, "text/html; charset=utf-8", LANDING)
        }
        (Method::Get, "/health") => send_cors(request, 200, "application/json", r#"{"ok":true}"#),
        (Method::Get, "/v1/models") => {
            let body = serde_json::to_string(&ModelsResponse {
                object: "list",
                data: vec![ModelCard {
                    id: rec.repo_id.clone(),
                    object: "model",
                    owned_by: "reco",
                }],
            })
            .map_err(|err| err.to_string())?;
            send_cors(request, 200, "application/json", &body)
        }
        (Method::Post, "/v1/chat/completions") => {
            let mut raw = String::new();
            request
                .as_reader()
                .read_to_string(&mut raw)
                .map_err(|err| err.to_string())?;
            let parsed: ChatRequest =
                serde_json::from_str(&raw).map_err(|err| format!("json: {err}"))?;
            let messages: Vec<ChatMessage> = parsed
                .messages
                .into_iter()
                .filter_map(|m| {
                    ChatRole::parse(&m.role).map(|role| ChatMessage {
                        role,
                        content: m.content,
                    })
                })
                .collect();
            let content = engine.generate(&messages).map_err(|err| err.to_string())?;
            if parsed.stream {
                return send_stream(request, rec, &content);
            }
            let created = now_secs();
            let body = serde_json::to_string(&ChatResponse {
                id: format!("chatcmpl-{created}"),
                object: "chat.completion",
                created,
                model: rec.repo_id.clone(),
                choices: vec![Choice {
                    index: 0,
                    message: ApiOutMessage {
                        role: "assistant",
                        content: content.clone(),
                    },
                    finish_reason: "stop",
                }],
                usage: estimate_usage(&messages, &content),
            })
            .map_err(|err| err.to_string())?;
            send_cors(request, 200, "application/json", &body)
        }
        _ => send_cors(
            request,
            404,
            "application/json",
            r#"{"error":{"message":"not found"}}"#,
        ),
    }
}

fn send_stream(request: Request, rec: &Recommendation, content: &str) -> Result<(), String> {
    let created = now_secs();
    let id = format!("chatcmpl-{created}");
    let mut body = String::new();
    for piece in chunk_words(content) {
        let chunk = serde_json::json!({
            "id": id,
            "object": "chat.completion.chunk",
            "created": created,
            "model": rec.repo_id,
            "choices": [{
                "index": 0,
                "delta": {"content": piece},
                "finish_reason": null
            }]
        });
        body.push_str("data: ");
        body.push_str(&chunk.to_string());
        body.push_str("\n\n");
    }
    body.push_str("data: [DONE]\n\n");
    send_cors(request, 200, "text/event-stream", &body)
}

fn chunk_words(content: &str) -> Vec<String> {
    let mut out = Vec::new();
    for word in content.split_inclusive(char::is_whitespace) {
        if !word.is_empty() {
            out.push(word.to_string());
        }
    }
    if out.is_empty() {
        out.push(content.to_string());
    }
    out
}

fn estimate_usage(messages: &[ChatMessage], content: &str) -> Usage {
    let prompt: u32 = messages
        .iter()
        .map(|m| (m.content.split_whitespace().count() as u32).max(1))
        .sum();
    let completion = (content.split_whitespace().count() as u32).max(1);
    Usage {
        prompt_tokens: prompt,
        completion_tokens: completion,
        total_tokens: prompt + completion,
    }
}

fn authorized(request: &Request, key: &str) -> bool {
    request.headers().iter().any(|header| {
        let name = header.field.to_string();
        if !name.eq_ignore_ascii_case("Authorization") {
            return false;
        }
        let value = header.value.to_string();
        value == key || value == format!("Bearer {key}")
    })
}

fn send_cors(request: Request, status: u16, ctype: &str, body: &str) -> Result<(), String> {
    let headers = vec![
        header("Content-Type", ctype)?,
        header("Access-Control-Allow-Origin", "*")?,
        header(
            "Access-Control-Allow-Headers",
            "Authorization, Content-Type",
        )?,
        header("Access-Control-Allow-Methods", "GET, POST, OPTIONS")?,
    ];
    let response = Response::new(
        status.into(),
        headers,
        Cursor::new(body.to_string()),
        Some(body.len()),
        None,
    );
    request.respond(response).map_err(|err| err.to_string())
}

fn header(name: &str, value: &str) -> Result<Header, String> {
    Header::from_bytes(name.as_bytes(), value.as_bytes()).map_err(|_| "header".to_string())
}

fn new_api_key() -> String {
    let mut buf = [0_u8; 16];
    let _ = getrandom::getrandom(&mut buf);
    let hex: String = buf.iter().map(|b| format!("{b:02x}")).collect();
    format!("sk-reco-{hex}")
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_has_sk_prefix() {
        let key = new_api_key();
        assert!(key.starts_with("sk-reco-"));
        assert!(key.len() > 16);
    }

    #[test]
    fn chunks_keep_spaces() {
        let bits = chunk_words("hola mundo");
        assert!(bits.join("").contains("hola"));
        assert!(bits.len() >= 2);
    }
}
