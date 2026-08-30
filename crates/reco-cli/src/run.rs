use std::io::{self, Write};
use std::path::PathBuf;
use std::process::Command;

use owo_colors::OwoColorize;
use reco_catalog::{
    cache_root, download_gguf, huggingface_resolve_url, is_downloaded, local_model_path,
};
use reco_core::infer::{pick_engine, EngineKind, PickedEngine};
use reco_core::store::ChatStore;
use reco_core::{format_gib, RecoConfig, Recommendation};

use crate::prueba;

pub fn download_recommendation(rec: &Recommendation, dry_run: bool) -> Result<PathBuf, String> {
    let url = huggingface_resolve_url(&rec.repo_id, &rec.filename);
    let dest = local_model_path(&rec.repo_id, &rec.filename);

    println!("{} {}", "Modelo".bold(), rec.repo_id);
    println!(
        "  {}  ·  {}{}",
        rec.quant.label().cyan(),
        format_gib(rec.size_bytes),
        if rec.size_estimated { " est." } else { "" }
    );
    println!("  url   {url}");
    println!("  dest  {}", dest.display());

    if dry_run {
        println!("  {}", "dry-run: no se descarga nada.".dimmed());
        return Ok(dest);
    }

    if is_downloaded(&rec.repo_id, &rec.filename) {
        println!("  {}", "ya está en el caché local.".green());
        return Ok(dest);
    }

    let started = std::time::Instant::now();
    let path = download_gguf(&rec.repo_id, &rec.filename, |written, total| {
        let _ = write!(
            io::stderr(),
            "\r  {} {}  ",
            "descargando".cyan(),
            progress_bar(written, total, started.elapsed().as_secs_f64())
        );
        let _ = io::stderr().flush();
    })
    .map_err(|err| err.to_string())?;
    eprintln!();
    println!("  {} {}", "listo".green(), path.display());
    Ok(path)
}

pub fn resolve_engine(
    rec: &Recommendation,
    demo: bool,
    provider: &str,
) -> Result<PickedEngine, String> {
    let cfg = RecoConfig::load();
    let kind = if demo {
        EngineKind::Echo
    } else {
        EngineKind::parse(provider).map_err(|err| err.to_string())?
    };
    let path = local_model_path(&rec.repo_id, &rec.filename);
    let gguf = if path.is_file() {
        Some(path.as_path())
    } else {
        None
    };
    pick_engine(&cfg, &rec.repo_id, gguf, kind).map_err(|err| err.to_string())
}

pub fn open_prueba(rec: &Recommendation, demo: bool, provider: &str) -> Result<(), String> {
    if !demo && try_launch_desktop(rec, provider)? {
        return Ok(());
    }
    let picked = resolve_engine(rec, demo, provider)?;
    let db = cache_root().join("reco.db");
    let store = ChatStore::open(&db).map_err(|err| err.to_string())?;
    println!("{} historial en {}", "Prueba".bold(), db.display());
    println!("  motor  {}", picked.label.cyan());
    if let Some(hint) = &picked.hint {
        println!("  {}", hint.dimmed());
    }
    prueba::run(&store, rec, picked).map_err(|err| err.to_string())
}

fn try_launch_desktop(rec: &Recommendation, provider: &str) -> Result<bool, String> {
    let mut candidates = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.join("reco-desktop"));
            candidates.push(dir.join("reco-prueba"));
        }
    }
    for name in ["reco-desktop", "reco-prueba"] {
        candidates.push(PathBuf::from(name));
    }
    for bin in candidates {
        let mut cmd = Command::new(&bin);
        cmd.arg("--repo")
            .arg(&rec.repo_id)
            .arg("--file")
            .arg(&rec.filename)
            .arg("--provider")
            .arg(provider);
        match cmd.status() {
            Ok(_) => return Ok(true),
            Err(err) if err.kind() == io::ErrorKind::NotFound => continue,
            Err(err) => {
                return Err(format!("no pude abrir {}: {err}", bin.display()));
            }
        }
    }
    Ok(false)
}

fn progress_bar(written: u64, total: Option<u64>, elapsed: f64) -> String {
    let speed = if elapsed > 0.2 {
        format!("  {}", format_gib((written as f64 / elapsed) as u64) + "/s")
    } else {
        String::new()
    };
    match total.filter(|n| *n > 0) {
        Some(total) => {
            let pct = (written as f64 / total as f64).clamp(0.0, 1.0);
            let fill = (pct * 22.0).round() as usize;
            let bar = format!("{}{}", "█".repeat(fill), "░".repeat(22 - fill));
            format!(
                "[{bar}] {:>5.1}%  {} / {}{speed}",
                pct * 100.0,
                format_gib(written),
                format_gib(total)
            )
        }
        None => format!("{}{speed}", format_gib(written)),
    }
}
