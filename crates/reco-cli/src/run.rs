use std::io::{self, Write};
use std::path::{Path, PathBuf};
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

pub fn open_prueba(
    rec: &Recommendation,
    demo: bool,
    provider: &str,
    force_tui: bool,
) -> Result<(), String> {
    if !force_tui {
        match try_launch_desktop(Some(rec), demo, provider) {
            Ok(true) => {
                println!("{} {}", "Prueba".bold(), "ventana Tauri".cyan());
                return Ok(());
            }
            Ok(false) => {
                eprintln!(
                    "{}",
                    "sin reco-desktop; abriendo Prueba en la terminal.".dimmed()
                );
                eprintln!(
                    "  {}",
                    "ventana: scripts/build-desktop.sh   ·   forzar TUI: --tui".dimmed()
                );
            }
            Err(err) => return Err(err),
        }
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

pub fn open_desktop_picker(demo: bool, provider: &str) -> Result<(), String> {
    match try_launch_desktop(None, demo, provider) {
        Ok(true) => {
            println!("{} {}", "Prueba".bold(), "ventana Tauri".cyan());
            Ok(())
        }
        Ok(false) => Err(
            "no encontré reco-desktop. Compílalo con scripts/build-desktop.sh y déjalo en PATH, en ~/.cargo/bin o junto a reco.".into(),
        ),
        Err(err) => Err(err),
    }
}

/// First existing `reco-desktop` / `reco-prueba`, if any.
pub fn desktop_binary() -> Option<PathBuf> {
    for candidate in desktop_candidates() {
        if is_path_name(&candidate) {
            if let Some(found) = look_on_path(&candidate.to_string_lossy()) {
                return Some(found);
            }
            continue;
        }
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

pub fn desktop_candidates() -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut push = |path: PathBuf| {
        if !out.contains(&path) {
            out.push(path);
        }
    };

    if let Ok(explicit) = std::env::var("RECO_DESKTOP") {
        if !explicit.is_empty() {
            push(PathBuf::from(explicit));
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            push(dir.join("reco-desktop"));
            push(dir.join("reco-prueba"));
        }
    }
    if let Ok(cargo_home) = std::env::var("CARGO_HOME") {
        let cargo_bin = PathBuf::from(cargo_home).join("bin");
        push(cargo_bin.join("reco-desktop"));
        push(cargo_bin.join("reco-prueba"));
    }
    if let Some(home) = std::env::var_os("HOME") {
        let home = PathBuf::from(home);
        push(home.join(".cargo/bin/reco-desktop"));
        push(home.join(".cargo/bin/reco-prueba"));
        push(home.join(".local/bin/reco-desktop"));
        push(home.join(".local/bin/reco-prueba"));
    }
    push(PathBuf::from("reco-desktop"));
    push(PathBuf::from("reco-prueba"));
    out
}

fn is_path_name(path: &Path) -> bool {
    path.components().count() == 1
}

fn look_on_path(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path).find_map(|dir| {
        let candidate = dir.join(name);
        candidate.is_file().then_some(candidate)
    })
}

fn try_launch_desktop(
    rec: Option<&Recommendation>,
    demo: bool,
    provider: &str,
) -> Result<bool, String> {
    for bin in desktop_candidates() {
        let mut cmd = Command::new(&bin);
        if let Some(rec) = rec {
            cmd.arg("--repo")
                .arg(&rec.repo_id)
                .arg("--file")
                .arg(&rec.filename);
        }
        cmd.arg("--provider").arg(provider);
        if demo {
            cmd.arg("--demo");
        }
        match cmd.status() {
            Ok(status) if status.success() => return Ok(true),
            Ok(status) => {
                return Err(format!("{} salió con {}", display_bin(&bin), status));
            }
            Err(err) if err.kind() == io::ErrorKind::NotFound => continue,
            Err(err) => {
                return Err(format!("no pude abrir {}: {err}", display_bin(&bin)));
            }
        }
    }
    Ok(false)
}

fn display_bin(path: &Path) -> String {
    path.display().to_string()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn candidates_always_include_path_names() {
        let list = desktop_candidates();
        assert!(list.iter().any(|p| p == Path::new("reco-desktop")));
        assert!(list.iter().any(|p| p == Path::new("reco-prueba")));
    }

    #[test]
    fn reco_desktop_env_is_first_candidate() {
        std::env::set_var("RECO_DESKTOP", "/tmp/custom-reco-desktop-1de9");
        let list = desktop_candidates();
        std::env::remove_var("RECO_DESKTOP");
        assert_eq!(list[0], PathBuf::from("/tmp/custom-reco-desktop-1de9"));
    }

    #[test]
    fn home_cargo_bin_is_searched() {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/home/reco".into());
        let expected = PathBuf::from(&home).join(".cargo/bin/reco-desktop");
        assert!(desktop_candidates().contains(&expected));
    }
}
