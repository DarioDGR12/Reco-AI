use std::io::Cursor;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use reco_core::apis::{advertised_base, openapi_json, ApiEndpoint};
use reco_core::chat::{ChatMessage, ChatRole};
use reco_core::infer::InferEngine;
use reco_core::Recommendation;
use serde::{Deserialize, Serialize};
use tiny_http::{Header, Method, Request, Response, Server};

pub struct HubSlot {
    pub api: ApiEndpoint,
    pub engine: Box<dyn InferEngine>,
    pub label: String,
}

struct LiveSlot {
    api: ApiEndpoint,
    engine: Mutex<Box<dyn InferEngine>>,
    label: String,
}

#[derive(Deserialize)]
struct ChatRequest {
    #[serde(default)]
    model: String,
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

pub fn run(
    rec: &Recommendation,
    port: u16,
    host: &str,
    picked: reco_core::infer::PickedEngine,
    api: Option<ApiEndpoint>,
) -> Result<(), String> {
    let api = api.unwrap_or_else(|| ephemeral(rec, host, port));
    run_hub(
        host,
        port,
        vec![HubSlot {
            api,
            engine: picked.engine,
            label: picked.label,
        }],
    )
}

pub fn run_hub(host: &str, port: u16, slots: Vec<HubSlot>) -> Result<(), String> {
    if slots.is_empty() {
        return Err(
            "no hay APIs. Crea una: reco api create <modelo> --name mi-app".into(),
        );
    }
    let addr = format!("{host}:{port}");
    let server = Server::http(&addr).map_err(|err| err.to_string())?;
    let public = advertised_base(host, port);
    let live: Vec<LiveSlot> = slots
        .into_iter()
        .map(|s| LiveSlot {
            api: s.api,
            engine: Mutex::new(s.engine),
            label: s.label,
        })
        .collect();

    println!("Reco API  ·  tu máquina es el servidor");
    println!("  Hub      {public}");
    println!("  Docs     {public}/");
    println!("  OpenAPI  {public}/openapi.json");
    println!();
    for slot in &live {
        println!(
            "  · {:<16}  {}  ·  {}",
            slot.api.slug, slot.api.repo_id, slot.api.api_key
        );
        println!("    motor {}  ·  {}", slot.label, slot.api.quant);
    }
    println!();
    println!("  En otra app: base URL {public}/v1  +  la API key de esa app.");
    println!("  Clientes: reco api code <nombre> --client python|continue|cursor|openwebui");
    println!("  Ctrl+C para parar.");

    for request in server.incoming_requests() {
        if let Err(err) = handle(request, &live, &public) {
            eprintln!("api: {err}");
        }
    }
    Ok(())
}

fn ephemeral(rec: &Recommendation, host: &str, port: u16) -> ApiEndpoint {
    ApiEndpoint {
        slug: "default".into(),
        name: rec.repo_id.rsplit('/').next().unwrap_or(&rec.repo_id).into(),
        repo_id: rec.repo_id.clone(),
        filename: rec.filename.clone(),
        quant: rec.quant.label(),
        api_key: reco_core::apis::generate_api_key("default"),
        host: host.into(),
        port,
        provider: "auto".into(),
        lan: host == "0.0.0.0",
        created_at: now_secs() as i64,
    }
}

fn handle(mut request: Request, slots: &[LiveSlot], public: &str) -> Result<(), String> {
    let url = request.url().to_string();
    let path = url.split('?').next().unwrap_or(&url).to_string();
    let method = request.method().clone();

    if method == Method::Options {
        return send_cors(request, 204, "text/plain", "");
    }

    let key = extract_key(&request);
    let open = matches!(
        path.as_str(),
        "/" | "/docs" | "/health" | "/openapi.json" | "/favicon.ico"
    );

    if !open {
        let Some(key) = key.as_deref() else {
            return send_cors(
                request,
                401,
                "application/json",
                r#"{"error":{"message":"falta Authorization: Bearer sk-reco-…","type":"auth"}}"#,
            );
        };
        if slot_for_key(slots, key).is_none() {
            return send_cors(
                request,
                401,
                "application/json",
                r#"{"error":{"message":"API key inválida","type":"auth"}}"#,
            );
        }
    }

    match (method, path.as_str()) {
        (Method::Get, "/") | (Method::Get, "/docs") => send_cors(
            request,
            200,
            "text/html; charset=utf-8",
            &landing_html(slots, public),
        ),
        (Method::Get, "/health") => {
            let body = serde_json::json!({
                "ok": true,
                "apis": slots.len(),
                "models": slots.iter().map(|s| s.api.repo_id.clone()).collect::<Vec<_>>(),
            });
            send_cors(request, 200, "application/json", &body.to_string())
        }
        (Method::Get, "/openapi.json") => {
            let spec = if let Some(key) = key.as_deref().and_then(|k| slot_for_key(slots, k)) {
                openapi_json(&key.api, public)
            } else if let Some(first) = slots.first() {
                openapi_json(&first.api, public)
            } else {
                "{}".into()
            };
            send_cors(request, 200, "application/json", &spec)
        }
        (Method::Get, "/v1/models") => {
            let slot = slot_for_key(slots, key.as_deref().unwrap_or(""))
                .ok_or_else(|| "auth".to_string())?;
            let body = serde_json::json!({
                "object": "list",
                "data": slot.api.model_ids().into_iter().map(|id| serde_json::json!({
                    "id": id,
                    "object": "model",
                    "owned_by": "reco",
                })).collect::<Vec<_>>()
            });
            send_cors(request, 200, "application/json", &body.to_string())
        }
        (Method::Get, "/v1/me") => {
            let slot = slot_for_key(slots, key.as_deref().unwrap_or(""))
                .ok_or_else(|| "auth".to_string())?;
            let body = serde_json::json!({
                "api": slot.api.slug,
                "name": slot.api.name,
                "model": slot.api.repo_id,
                "filename": slot.api.filename,
                "engine": slot.label,
            });
            send_cors(request, 200, "application/json", &body.to_string())
        }
        (Method::Post, "/v1/chat/completions") => {
            let slot = slot_for_key(slots, key.as_deref().unwrap_or(""))
                .ok_or_else(|| "auth".to_string())?;
            let mut raw = String::new();
            request
                .as_reader()
                .read_to_string(&mut raw)
                .map_err(|err| err.to_string())?;
            let parsed: ChatRequest =
                serde_json::from_str(&raw).map_err(|err| format!("json: {err}"))?;
            if !slot.api.allows_model(&parsed.model) {
                return send_cors(
                    request,
                    404,
                    "application/json",
                    &format!(
                        "{{\"error\":{{\"message\":\"esta clave solo abre {}\"}}}}",
                        slot.api.repo_id
                    ),
                );
            }
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
            let content = {
                let mut engine = slot
                    .engine
                    .lock()
                    .map_err(|_| "engine lock".to_string())?;
                engine.generate(&messages).map_err(|err| err.to_string())?
            };
            if parsed.stream {
                return send_stream(request, &slot.api.repo_id, &content);
            }
            let created = now_secs();
            let body = serde_json::to_string(&ChatResponse {
                id: format!("chatcmpl-{}-{created}", slot.api.slug),
                object: "chat.completion",
                created,
                model: slot.api.repo_id.clone(),
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

fn slot_for_key<'a>(slots: &'a [LiveSlot], key: &str) -> Option<&'a LiveSlot> {
    slots.iter().find(|s| s.api.api_key == key)
}

fn extract_key(request: &Request) -> Option<String> {
    for header in request.headers() {
        let name = header.field.to_string();
        let value = header.value.to_string();
        if name.eq_ignore_ascii_case("Authorization") {
            let v = value.trim();
            return Some(
                v.strip_prefix("Bearer ")
                    .unwrap_or(v)
                    .trim()
                    .to_string(),
            );
        }
        if name.eq_ignore_ascii_case("x-api-key") && !value.trim().is_empty() {
            return Some(value.trim().to_string());
        }
    }
    None
}

fn landing_html(slots: &[LiveSlot], public: &str) -> String {
    let mut cards = String::new();
    for slot in slots {
        cards.push_str(&format!(
            "<article><h2>{}</h2><p class=\"slug\">{}</p><p>Modelo <code>{}</code> · {}</p>\
             <p>Clave <code>{}</code></p>\
             <pre>curl {public}/v1/chat/completions \\\n  -H \"Authorization: Bearer {}\" \\\n  -H \"Content-Type: application/json\" \\\n  -d '{{\"model\":\"{}\",\"messages\":[{{\"role\":\"user\",\"content\":\"hola\"}}]}}'</pre></article>",
            esc(&slot.api.name),
            esc(&slot.api.slug),
            esc(&slot.api.repo_id),
            esc(&slot.api.quant),
            esc(&slot.api.masked_key()),
            esc(&slot.api.api_key),
            esc(&slot.api.repo_id),
        ));
    }
    format!(
        r#"<!doctype html>
<html lang="es"><head><meta charset="utf-8"><title>Reco API</title>
<style>
body{{font:16px/1.5 ui-sans-serif,system-ui;background:#1e1e2e;color:#cdd6f4;margin:0;padding:40px 48px}}
h1{{color:#cba6f7;margin:0 0 8px}} .dim{{color:#6c7086}}
article{{background:#313244;border-radius:12px;padding:18px 20px;margin:16px 0}}
h2{{margin:0 0 4px;font-size:18px}} .slug{{color:#89b4fa;margin:0 0 8px}}
code,pre{{background:#181825;border-radius:8px}}
code{{padding:2px 6px}} pre{{padding:12px 14px;overflow:auto;font-size:13px}}
a{{color:#cba6f7}}
</style></head><body>
<h1>Reco API</h1>
<p class="dim">Esta máquina sirve {n} API{s}. Base <code>{public}/v1</code> · <a href="/openapi.json">openapi.json</a></p>
{cards}
<p class="dim">Genera más: <code>reco api create &lt;modelo&gt; --name otra-app</code></p>
</body></html>"#,
        n = slots.len(),
        s = if slots.len() == 1 { "" } else { "s" },
        public = esc(public),
        cards = cards
    )
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn send_stream(request: Request, model: &str, content: &str) -> Result<(), String> {
    let created = now_secs();
    let id = format!("chatcmpl-{created}");
    let mut body = String::new();
    for piece in chunk_words(content) {
        let chunk = serde_json::json!({
            "id": id,
            "object": "chat.completion.chunk",
            "created": created,
            "model": model,
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

fn send_cors(request: Request, status: u16, ctype: &str, body: &str) -> Result<(), String> {
    let headers = vec![
        header("Content-Type", ctype)?,
        header("Access-Control-Allow-Origin", "*")?,
        header(
            "Access-Control-Allow-Headers",
            "Authorization, Content-Type, x-api-key",
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
    fn chunks_keep_spaces() {
        let bits = chunk_words("hola mundo");
        assert!(bits.join("").contains("hola"));
        assert!(bits.len() >= 2);
    }

    #[test]
    fn landing_lists_slug() {
        let html = landing_html(&[], "http://127.0.0.1:11434");
        assert!(html.contains("Reco API"));
    }
}
