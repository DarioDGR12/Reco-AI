use std::io::Cursor;

use reco_core::chat::{ChatMessage, ChatRole};
use reco_core::infer::{InferEngine, PickedEngine};
use reco_core::Recommendation;
use serde::{Deserialize, Serialize};
use tiny_http::{Header, Method, Request, Response, Server};

#[derive(Deserialize)]
struct ChatRequest {
    #[serde(default)]
    messages: Vec<ApiMessage>,
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
    model: String,
    choices: Vec<Choice>,
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
struct ModelsResponse {
    object: &'static str,
    data: Vec<ModelCard>,
}

#[derive(Serialize)]
struct ModelCard {
    id: String,
    object: &'static str,
}

pub fn run(
    rec: &Recommendation,
    port: u16,
    host: &str,
    mut picked: PickedEngine,
) -> Result<(), String> {
    let key = new_api_key();
    let addr = format!("{host}:{port}");
    let server = Server::http(&addr).map_err(|err| err.to_string())?;

    println!("Servidor local en http://{addr}");
    println!("Modelo     {}", rec.repo_id);
    println!("Cuantiz.   {}", rec.quant.label());
    println!("Motor      {}", picked.label);
    println!("API key    {key}");
    if let Some(hint) = &picked.hint {
        println!("Nota       {hint}");
    }
    println!();
    println!("curl http://{addr}/v1/chat/completions \\");
    println!("  -H \"Authorization: Bearer {key}\" \\");
    println!("  -H \"Content-Type: application/json\" \\");
    println!("  -d '{{\"messages\":[{{\"role\":\"user\",\"content\":\"hola\"}}]}}'");
    println!();
    println!("Ctrl+C para parar.");

    for request in server.incoming_requests() {
        if let Err(err) = handle(request, rec, &key, &mut *picked.engine) {
            eprintln!("serve: {err}");
        }
    }
    Ok(())
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

    if path != "/health" && !authorized(&request, key) {
        return send(
            request,
            401,
            r#"{"error":{"message":"API key inválida","type":"auth"}}"#,
        );
    }

    match (method, path) {
        (Method::Get, "/health") => send(request, 200, r#"{"ok":true}"#),
        (Method::Get, "/v1/models") => {
            let body = serde_json::to_string(&ModelsResponse {
                object: "list",
                data: vec![ModelCard {
                    id: rec.repo_id.clone(),
                    object: "model",
                }],
            })
            .map_err(|err| err.to_string())?;
            send(request, 200, &body)
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
            let body = serde_json::to_string(&ChatResponse {
                id: format!("chatcmpl-{}", rec.quant.label()),
                object: "chat.completion",
                model: rec.repo_id.clone(),
                choices: vec![Choice {
                    index: 0,
                    message: ApiOutMessage {
                        role: "assistant",
                        content,
                    },
                    finish_reason: "stop",
                }],
            })
            .map_err(|err| err.to_string())?;
            send(request, 200, &body)
        }
        _ => send(request, 404, r#"{"error":{"message":"not found"}}"#),
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

fn send(request: Request, status: u16, body: &str) -> Result<(), String> {
    let header = Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..])
        .map_err(|_| "header".to_string())?;
    let response = Response::new(
        status.into(),
        vec![header],
        Cursor::new(body.to_string()),
        Some(body.len()),
        None,
    );
    request.respond(response).map_err(|err| err.to_string())
}

fn new_api_key() -> String {
    let mut buf = [0_u8; 16];
    let _ = getrandom::getrandom(&mut buf);
    let hex: String = buf.iter().map(|b| format!("{b:02x}")).collect();
    format!("sk-reco-{hex}")
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
}
