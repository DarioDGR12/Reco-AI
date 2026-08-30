use std::io::{self, Write};
use std::path::PathBuf;

use owo_colors::OwoColorize;
use reco_catalog::{
    cache_root, download_gguf, huggingface_resolve_url, is_downloaded, local_model_path,
};
use reco_core::store::ChatStore;
use reco_core::{format_gib, Recommendation};

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

    print!("  descargando…");
    let _ = io::stdout().flush();
    let path = download_gguf(&rec.repo_id, &rec.filename, |written, total| {
        let _ = write!(
            io::stderr(),
            "\r  descargando {} / {}   ",
            format_gib(written),
            total.map(format_gib).unwrap_or_else(|| "?".into())
        );
        let _ = io::stderr().flush();
    })
    .map_err(|err| err.to_string())?;
    eprintln!();
    println!("  {} {}", "listo".green(), path.display());
    Ok(path)
}

pub fn open_prueba(rec: &Recommendation, demo: bool) -> Result<(), String> {
    let db = cache_root().join("reco.db");
    let store = ChatStore::open(&db).map_err(|err| err.to_string())?;
    if demo {
        println!(
            "  {} historial en {}",
            "Prueba".bold(),
            db.display()
        );
        println!("  {}", "modo demo: EchoEngine (sin llama.cpp)".dimmed());
    }
    prueba::run(&store, rec, demo).map_err(|err| err.to_string())
}
