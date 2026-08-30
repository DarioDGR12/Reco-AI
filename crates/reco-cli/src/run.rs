use std::io::{self, Write};

use owo_colors::OwoColorize;
use reco_catalog::{download_gguf, huggingface_resolve_url, is_downloaded, local_model_path};
use reco_core::{format_gib, Recommendation};

pub fn download_recommendation(rec: &Recommendation, dry_run: bool) -> Result<(), String> {
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
        return Ok(());
    }

    if is_downloaded(&rec.repo_id, &rec.filename) {
        println!("  {}", "ya está en el caché local.".green());
        print_next_steps(&dest);
        return Ok(());
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
    print_next_steps(&path);
    Ok(())
}

fn print_next_steps(path: &std::path::Path) {
    println!();
    println!("{}", "Prueba (Tauri + llama.cpp) todavía no abre la ventana.".dimmed());
    println!(
        "  El GGUF ya está en disco. Siguiente: chat nativo sobre {}",
        path.file_name().and_then(|n| n.to_str()).unwrap_or("este archivo")
    );
}
