#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Mutex;

use clap::Parser;
use reco_catalog::{cache_root, is_downloaded, local_model_path};
use reco_core::chat::ChatRole;
use reco_core::infer::{pick_engine, EngineKind, InferEngine, PickedEngine};
use reco_core::store::ChatStore;
use reco_core::RecoConfig;
use serde::Serialize;

#[derive(Parser, Debug)]
#[command(name = "reco-desktop", about = "Ventana Tauri de Prueba")]
struct Args {
    #[arg(long)]
    repo: Option<String>,
    #[arg(long)]
    file: Option<String>,
    #[arg(long, default_value = "auto")]
    provider: String,
    #[arg(long)]
    demo: bool,
}

struct AppState {
    repo_id: String,
    filename: String,
    engine_label: String,
    conversation_id: String,
    engine: Mutex<Box<dyn InferEngine>>,
}

#[derive(Serialize)]
struct SessionInfo {
    repo_id: String,
    filename: String,
    engine_label: String,
}

#[derive(Serialize)]
struct HistoryMessage {
    role: String,
    content: String,
}

fn store() -> Result<ChatStore, String> {
    ChatStore::open(&cache_root().join("reco.db")).map_err(|err| err.to_string())
}

fn pick(repo_id: &str, filename: &str, demo: bool, provider: &str) -> Result<PickedEngine, String> {
    let cfg = RecoConfig::load();
    let kind = if demo {
        EngineKind::Echo
    } else {
        EngineKind::parse(provider).map_err(|err| err.to_string())?
    };
    let path = local_model_path(repo_id, filename);
    let gguf = if is_downloaded(repo_id, filename) || path.is_file() {
        Some(path.as_path())
    } else {
        None
    };
    pick_engine(&cfg, repo_id, gguf, kind).map_err(|err| err.to_string())
}

#[tauri::command]
fn session_info(state: tauri::State<AppState>) -> SessionInfo {
    SessionInfo {
        repo_id: state.repo_id.clone(),
        filename: state.filename.clone(),
        engine_label: state.engine_label.clone(),
    }
}

#[tauri::command]
fn load_history(state: tauri::State<AppState>) -> Result<Vec<HistoryMessage>, String> {
    let store = store()?;
    let msgs = store
        .messages(&state.conversation_id)
        .map_err(|err| err.to_string())?;
    Ok(msgs
        .into_iter()
        .map(|m| HistoryMessage {
            role: m.role.as_str().into(),
            content: m.content,
        })
        .collect())
}

#[tauri::command]
fn send_message(state: tauri::State<AppState>, text: String) -> Result<String, String> {
    let text = text.trim().to_string();
    if text.is_empty() {
        return Ok(String::new());
    }
    let store = store()?;
    store
        .append(&state.conversation_id, ChatRole::User, &text)
        .map_err(|err| err.to_string())?;
    let messages = store
        .messages(&state.conversation_id)
        .map_err(|err| err.to_string())?;
    let reply = {
        let mut engine = state.engine.lock().map_err(|_| "engine lock".to_string())?;
        engine.generate(&messages).map_err(|err| err.to_string())?
    };
    store
        .append(&state.conversation_id, ChatRole::Assistant, &reply)
        .map_err(|err| err.to_string())?;
    Ok(reply)
}

fn main() {
    let args = Args::parse();
    let repo_id = args
        .repo
        .or_else(|| std::env::var("RECO_DESKTOP_REPO").ok())
        .unwrap_or_else(|| "Qwen/Qwen2.5-7B-Instruct-GGUF".into());
    let filename = args
        .file
        .or_else(|| std::env::var("RECO_DESKTOP_FILE").ok())
        .unwrap_or_else(|| "qwen2.5-7b-instruct-q4_k_m.gguf".into());
    let mut picked = pick(&repo_id, &filename, args.demo, &args.provider)
        .unwrap_or_else(|_| pick(&repo_id, &filename, true, "echo").expect("echo engine"));
    let db = store().expect("sqlite");
    let conv = db
        .open_or_create(&repo_id, &filename)
        .expect("conversation");
    if db
        .messages(&conv.id)
        .ok()
        .map(|m| m.is_empty())
        .unwrap_or(true)
    {
        let welcome = picked
            .engine
            .generate(&[])
            .unwrap_or_else(|_| "Prueba lista.".into());
        let _ = db.append(&conv.id, ChatRole::Assistant, &welcome);
    }
    let state = AppState {
        repo_id,
        filename,
        engine_label: picked.label,
        conversation_id: conv.id,
        engine: Mutex::new(picked.engine),
    };
    tauri::Builder::default()
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            session_info,
            load_history,
            send_message
        ])
        .run(tauri::generate_context!())
        .expect("error al abrir Prueba");
}
