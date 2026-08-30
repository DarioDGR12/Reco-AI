#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Mutex;

use clap::Parser;
use reco_catalog::{
    cache_root, download_gguf, is_downloaded, list_downloaded, load_catalog, local_model_path,
    LoadOptions,
};
use reco_core::chat::ChatRole;
use reco_core::infer::{pick_engine, EngineKind, InferEngine, PickedEngine};
use reco_core::store::ChatStore;
use reco_core::{detect, format_gib, recommend, RecoConfig};
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

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
    #[arg(long)]
    offline: bool,
    #[arg(long)]
    refresh: bool,
}

struct Session {
    repo_id: String,
    filename: String,
    engine_label: String,
    conversation_id: String,
    engine: Box<dyn InferEngine>,
}

struct AppState {
    demo: bool,
    provider: String,
    offline: bool,
    refresh: bool,
    session: Mutex<Option<Session>>,
}

#[derive(Serialize)]
struct HardwareCard {
    cpu: String,
    ram: String,
    gpu: String,
    backend: String,
}

#[derive(Serialize)]
struct SessionInfo {
    has_model: bool,
    repo_id: String,
    filename: String,
    engine_label: String,
    demo: bool,
    hardware: HardwareCard,
}

#[derive(Serialize)]
struct HistoryMessage {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct ConvoCard {
    id: String,
    title: String,
    active: bool,
}

#[derive(Serialize)]
struct CatalogCard {
    repo_id: String,
    filename: String,
    quant: String,
    size: String,
    size_estimated: bool,
    downloads: u64,
    total: f32,
    why: String,
    downloaded: bool,
    scores: reco_core::Scores,
    params: Option<String>,
}

#[derive(Serialize)]
struct CatalogPage {
    source: String,
    notes: Vec<String>,
    models: Vec<CatalogCard>,
}

#[derive(Clone, Serialize)]
struct DownloadProgress {
    written: u64,
    total: Option<u64>,
    done: bool,
}

fn store() -> Result<ChatStore, String> {
    ChatStore::open(&cache_root().join("reco.db")).map_err(|err| err.to_string())
}

fn hardware_card() -> HardwareCard {
    let profile = detect();
    let gpu = profile
        .gpus
        .first()
        .map(|gpu| match gpu.vram_bytes {
            Some(bytes) => format!("{} · {}", gpu.name, format_gib(bytes)),
            None => gpu.name.clone(),
        })
        .unwrap_or_else(|| "sin GPU (CPU)".into());
    HardwareCard {
        cpu: profile.cpu.name,
        ram: format_gib(profile.memory.total_bytes),
        gpu,
        backend: profile.primary_backend().display_name().into(),
    }
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

fn open_session(repo_id: &str, filename: &str, demo: bool, provider: &str) -> Result<Session, String> {
    let mut picked = pick(repo_id, filename, demo, provider)
        .or_else(|_| pick(repo_id, filename, true, "echo"))?;
    let db = store()?;
    let conv = db
        .open_or_create(repo_id, filename)
        .map_err(|err| err.to_string())?;
    if db
        .messages(&conv.id)
        .ok()
        .map(|msgs| msgs.is_empty())
        .unwrap_or(true)
    {
        let welcome = picked
            .engine
            .generate(&[])
            .unwrap_or_else(|_| "Prueba lista.".into());
        db.append(&conv.id, ChatRole::Assistant, &welcome)
            .map_err(|err| err.to_string())?;
    }
    Ok(Session {
        repo_id: repo_id.into(),
        filename: filename.into(),
        engine_label: picked.label,
        conversation_id: conv.id,
        engine: picked.engine,
    })
}

#[tauri::command]
fn session_info(state: State<AppState>) -> Result<SessionInfo, String> {
    let guard = state.session.lock().map_err(|_| "lock".to_string())?;
    Ok(SessionInfo {
        has_model: guard.is_some(),
        repo_id: guard.as_ref().map(|s| s.repo_id.clone()).unwrap_or_default(),
        filename: guard
            .as_ref()
            .map(|s| s.filename.clone())
            .unwrap_or_default(),
        engine_label: guard
            .as_ref()
            .map(|s| s.engine_label.clone())
            .unwrap_or_default(),
        demo: state.demo,
        hardware: hardware_card(),
    })
}

#[tauri::command]
fn list_catalog(state: State<AppState>, refresh: Option<bool>) -> Result<CatalogPage, String> {
    let profile = detect();
    let (catalog, notes) = load_catalog(LoadOptions {
        refresh: refresh.unwrap_or(state.refresh),
        offline: state.offline,
        limit: 80,
        ..LoadOptions::default()
    });
    let recs = recommend(&profile, &catalog, 16);
    let source = match catalog.source {
        reco_core::CatalogSource::HuggingFace => "Hugging Face",
        reco_core::CatalogSource::Cache => "caché local",
        reco_core::CatalogSource::Seed => "semilla embebida",
    }
    .to_string();
    let models = recs
        .into_iter()
        .map(|rec| CatalogCard {
            downloaded: is_downloaded(&rec.repo_id, &rec.filename),
            quant: rec.quant.label(),
            size: format_gib(rec.size_bytes),
            size_estimated: rec.size_estimated,
            params: rec.params.map(|p| format!("{:.0}B", p.effective_billions())),
            repo_id: rec.repo_id,
            filename: rec.filename,
            downloads: rec.downloads,
            total: rec.total,
            why: rec.why,
            scores: rec.scores,
        })
        .collect();
    Ok(CatalogPage {
        source,
        notes,
        models,
    })
}

#[tauri::command]
fn list_local_models() -> Result<Vec<CatalogCard>, String> {
    Ok(list_downloaded()
        .into_iter()
        .map(|model| CatalogCard {
            downloaded: true,
            quant: reco_core::GgufQuant::parse(&model.filename)
                .map(|q| q.label())
                .unwrap_or_else(|| "GGUF".into()),
            size: format_gib(model.size_bytes),
            size_estimated: false,
            params: None,
            repo_id: model.repo_id,
            filename: model.filename,
            downloads: 0,
            total: 0.0,
            why: "ya está en disco".into(),
            scores: reco_core::Scores {
                compatibility: 0.0,
                speed: 0.0,
                quality: 0.0,
                popularity: 0.0,
            },
        })
        .collect())
}

#[tauri::command]
fn select_model(
    state: State<AppState>,
    repo_id: String,
    filename: String,
    demo: Option<bool>,
) -> Result<SessionInfo, String> {
    let demo = demo.unwrap_or(state.demo);
    let session = open_session(&repo_id, &filename, demo, &state.provider)?;
    *state.session.lock().map_err(|_| "lock".to_string())? = Some(session);
    session_info(state)
}

#[tauri::command]
fn download_model(app: AppHandle, repo_id: String, filename: String) -> Result<(), String> {
    download_gguf(&repo_id, &filename, |written, total| {
        let _ = app.emit(
            "download-progress",
            DownloadProgress {
                written,
                total,
                done: false,
            },
        );
    })
    .map_err(|err| err.to_string())?;
    let _ = app.emit(
        "download-progress",
        DownloadProgress {
            written: 0,
            total: None,
            done: true,
        },
    );
    Ok(())
}

#[tauri::command]
fn load_history(state: State<AppState>) -> Result<Vec<HistoryMessage>, String> {
    let store = store()?;
    let id = {
        let guard = state.session.lock().map_err(|_| "lock".to_string())?;
        guard
            .as_ref()
            .map(|s| s.conversation_id.clone())
            .ok_or_else(|| "elige un modelo primero".to_string())?
    };
    let msgs = store.messages(&id).map_err(|err| err.to_string())?;
    Ok(msgs
        .into_iter()
        .map(|m| HistoryMessage {
            role: m.role.as_str().into(),
            content: m.content,
        })
        .collect())
}

#[tauri::command]
fn list_conversations(state: State<AppState>) -> Result<Vec<ConvoCard>, String> {
    let store = store()?;
    let guard = state.session.lock().map_err(|_| "lock".to_string())?;
    let session = guard.as_ref().ok_or_else(|| "elige un modelo primero".to_string())?;
    Ok(store
        .list_recent(24)
        .map_err(|err| err.to_string())?
        .into_iter()
        .filter(|c| c.repo_id == session.repo_id && c.filename == session.filename)
        .map(|c| ConvoCard {
            active: c.id == session.conversation_id,
            id: c.id,
            title: c.title,
        })
        .collect())
}

#[tauri::command]
fn open_conversation(state: State<AppState>, id: String) -> Result<(), String> {
    let mut guard = state.session.lock().map_err(|_| "lock".to_string())?;
    let session = guard
        .as_mut()
        .ok_or_else(|| "elige un modelo primero".to_string())?;
    session.conversation_id = id;
    Ok(())
}

#[tauri::command]
fn new_conversation(state: State<AppState>) -> Result<(), String> {
    let store = store()?;
    let mut guard = state.session.lock().map_err(|_| "lock".to_string())?;
    let session = guard
        .as_mut()
        .ok_or_else(|| "elige un modelo primero".to_string())?;
    let conv = store
        .new_conversation(&session.repo_id, &session.filename)
        .map_err(|err| err.to_string())?;
    let welcome = session
        .engine
        .generate(&[])
        .unwrap_or_else(|_| "Prueba lista.".into());
    store
        .append(&conv.id, ChatRole::Assistant, &welcome)
        .map_err(|err| err.to_string())?;
    session.conversation_id = conv.id;
    Ok(())
}

#[tauri::command]
fn send_message(state: State<AppState>, text: String) -> Result<String, String> {
    let text = text.trim().to_string();
    if text.is_empty() {
        return Ok(String::new());
    }
    let store = store()?;
    let mut guard = state.session.lock().map_err(|_| "lock".to_string())?;
    let session = guard
        .as_mut()
        .ok_or_else(|| "elige un modelo primero".to_string())?;
    store
        .append(&session.conversation_id, ChatRole::User, &text)
        .map_err(|err| err.to_string())?;
    let messages = store
        .messages(&session.conversation_id)
        .map_err(|err| err.to_string())?;
    let reply = session
        .engine
        .generate(&messages)
        .map_err(|err| err.to_string())?;
    store
        .append(&session.conversation_id, ChatRole::Assistant, &reply)
        .map_err(|err| err.to_string())?;
    Ok(reply)
}

fn main() {
    let args = Args::parse();
    let repo_id = args
        .repo
        .or_else(|| std::env::var("RECO_DESKTOP_REPO").ok())
        .filter(|s| !s.is_empty());
    let filename = args
        .file
        .or_else(|| std::env::var("RECO_DESKTOP_FILE").ok())
        .filter(|s| !s.is_empty());
    let session = match (repo_id.as_deref(), filename.as_deref()) {
        (Some(repo), Some(file)) => Some(
            open_session(repo, file, args.demo, &args.provider)
                .unwrap_or_else(|_| open_session(repo, file, true, "echo").expect("echo engine")),
        ),
        _ => None,
    };
    let state = AppState {
        demo: args.demo,
        provider: args.provider,
        offline: args.offline,
        refresh: args.refresh,
        session: Mutex::new(session),
    };
    tauri::Builder::default()
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            session_info,
            list_catalog,
            list_local_models,
            select_model,
            download_model,
            load_history,
            send_message,
            list_conversations,
            open_conversation,
            new_conversation
        ])
        .run(tauri::generate_context!())
        .expect("error al abrir Prueba");
}
