use owo_colors::OwoColorize;
use reco_catalog::DownloadedModel;
use reco_core::chat::Conversation;
use reco_core::infer::LlamaCliEngine;
use reco_core::{
    config_path, format_gib, AccelBackend, ApiRegistry, CatalogSource, HardwareProfile, RecoConfig,
    Recommendation,
};

use crate::doctor::{DoctorItem, INSTALL_ONE_LINER};

pub fn print_ai(
    profile: &HardwareProfile,
    recs: &[Recommendation],
    notes: &[String],
    source: CatalogSource,
    catalog_len: usize,
) {
    print_profile_box(profile);
    println!();
    println!(
        "{}  {}  ·  {} repos",
        "Recomendaciones".bold(),
        "40% compat · 20% vel · 20% cal · 20% pop".dimmed(),
        catalog_len
    );
    println!(
        "  fuente: {}",
        match source {
            CatalogSource::HuggingFace => "Hugging Face (en vivo)",
            CatalogSource::Cache => "caché local",
            CatalogSource::Seed => "semilla embebida",
        }
    );
    println!();

    if recs.is_empty() {
        println!("  No hay GGUF que entren cómodos en este hardware.");
        println!(
            "  Prueba {} o un modelo más chico.",
            "reco ai --refresh".cyan()
        );
    } else {
        for (index, rec) in recs.iter().enumerate() {
            println!("  {:>2}.  {}", index + 1, rec.repo_id.bold());
            println!(
                "      {}  ·  {}{}  ·  {}",
                rec.quant.label().cyan(),
                format_gib(rec.size_bytes),
                if rec.size_estimated { " est." } else { "" },
                rec.why.dimmed()
            );
            println!(
                "      score {:>5.1}   compat {:>5.1}  vel {:>5.1}  cal {:>5.1}  pop {:>5.1}",
                rec.total,
                rec.scores.compatibility,
                rec.scores.speed,
                rec.scores.quality,
                rec.scores.popularity
            );
            println!();
        }
        println!("  Siguiente: {} {}", "reco run".cyan(), recs[0].repo_id);
    }

    if !notes.is_empty() {
        println!();
        for note in notes {
            println!("  {}", note.dimmed());
        }
    }
}

pub fn print_hw(profile: &HardwareProfile, json: bool) {
    if json {
        match serde_json::to_string_pretty(profile) {
            Ok(json) => println!("{json}"),
            Err(err) => {
                eprintln!("No se pudo serializar el perfil: {err}");
                std::process::exit(1);
            }
        }
        return;
    }
    print_profile_box(profile);
}

fn print_profile_box(profile: &HardwareProfile) {
    let width = 56usize;
    let rule = "─".repeat(width);

    println!("{}", format!("╭{rule}╮").dimmed());
    title_row(width, "Reco AI");
    title_row(width, "Hardware detectado");
    println!("{}", format!("├{rule}┤").dimmed());

    let cores = match profile.cpu.physical_cores {
        Some(phys) => format!("{phys} núcleos / {} hilos", profile.cpu.logical_cores),
        None => format!("{} hilos", profile.cpu.logical_cores),
    };
    row(width, "CPU", &profile.cpu.name);
    row(width, "", &cores);

    row(
        width,
        "RAM",
        &format!(
            "{} total · {} libre",
            format_gib(profile.memory.total_bytes),
            format_gib(profile.memory.available_bytes)
        ),
    );

    if profile.gpus.is_empty() {
        row(width, "GPU", "ninguna (se usará CPU)");
    } else {
        for (index, gpu) in profile.gpus.iter().enumerate() {
            let label = if index == 0 { "GPU" } else { "" };
            row(width, label, &gpu.name);
            let vram = gpu
                .vram_bytes
                .map(format_gib)
                .unwrap_or_else(|| "VRAM desconocida".into());
            row(
                width,
                "",
                &format!("{} · {}", vram, gpu.backend.display_name()),
            );
        }
    }

    let mut os = profile.os.name.clone();
    if let Some(version) = &profile.os.version {
        os.push(' ');
        os.push_str(version);
    }
    os.push_str(&format!(" ({})", profile.os.arch));
    row(width, "SO", &os);

    let backend = match profile.primary_backend() {
        AccelBackend::Cpu => "inferencia en CPU".to_string(),
        other => format!("aceleración {}", other.display_name()),
    };
    row(width, "OK", &backend);

    println!("{}", format!("╰{rule}╯").dimmed());
}

fn title_row(width: usize, text: &str) {
    println!("{} {}{}", "│".dimmed(), pad(text, width), "│".dimmed());
}

fn row(width: usize, label: &str, value: &str) {
    let label_col = if label.is_empty() {
        "    ".to_string()
    } else {
        format!("{:<4}", label)
    };
    let text = format!("{label_col}{value}");
    println!("{} {}{}", "│".dimmed(), pad(&text, width), "│".dimmed());
}

pub fn print_home(
    profile: &HardwareProfile,
    downloaded: &[DownloadedModel],
    recent: &[Conversation],
) {
    print_profile_box(profile);
    println!();
    let cfg = RecoConfig::load();
    let llama = LlamaCliEngine::find_binary(cfg.llama.cli.as_deref());
    let has_byok = !cfg.byok.openai_key.is_empty() || !cfg.byok.anthropic_key.is_empty();
    let needs_install = llama.is_none() && !has_byok;
    let bytes: u64 = downloaded.iter().map(|m| m.size_bytes).sum();

    if needs_install {
        println!("{}", "Falta llama-cli".bold());
        println!("  {}", INSTALL_ONE_LINER.cyan());
        println!();
    }

    println!("{}", "Estado".bold());
    println!(
        "  {}  {}",
        "motor   ".dimmed(),
        match &llama {
            Some(path) => format!("llama-cli · {}", path.display()),
            None if !cfg.byok.openai_key.is_empty() => {
                format!("OpenAI · {}", cfg.byok.openai_model)
            }
            None if !cfg.byok.anthropic_key.is_empty() => {
                format!("Anthropic · {}", cfg.byok.anthropic_model)
            }
            None => "echo · reco setup".to_string(),
        }
    );
    println!(
        "  {}  {} modelos · {}",
        "local   ".dimmed(),
        downloaded.len(),
        format_gib(bytes)
    );
    println!("  {}  {}", "config  ".dimmed(), config_path().display());
    let apis = ApiRegistry::load();
    if apis.endpoints.is_empty() {
        println!(
            "  {}  ninguna · reco api create <modelo> --name mi-app",
            "apis    ".dimmed()
        );
    } else {
        println!(
            "  {}  {} listas · reco api start",
            "apis    ".dimmed(),
            apis.endpoints.len()
        );
    }
    println!();

    if !downloaded.is_empty() {
        println!("{}", "En disco".bold());
        for model in downloaded.iter().take(5) {
            println!(
                "  {}  {}  {}",
                model.repo_id.cyan(),
                model.filename.dimmed(),
                format_gib(model.size_bytes)
            );
        }
        if downloaded.len() > 5 {
            println!("  {}", format!("… y {} más", downloaded.len() - 5).dimmed());
        }
        println!();
    }

    if !recent.is_empty() {
        println!("{}", "Chats recientes".bold());
        for conv in recent.iter().take(4) {
            println!(
                "  {}  {}",
                conv.title.cyan(),
                format!("{} · {}", conv.repo_id, conv.filename).dimmed()
            );
        }
        println!();
    }

    println!("{}", "Siguiente".bold());
    println!(
        "  {}  checklist (llama-cli, ventana, modelos)",
        "reco setup".cyan()
    );
    println!(
        "  {}  ventana Prueba (catálogo + chat)",
        "reco desktop".cyan()
    );
    println!("  {}  catálogo que cabe en esta máquina", "reco ai".cyan());
    println!(
        "  {}  descarga y abre la ventana",
        "reco run <modelo>".cyan()
    );
    println!("  {}  llama.cpp, claves y caché", "reco doctor".cyan());
}

pub fn print_doctor(items: &[DoctorItem], json: bool) {
    if json {
        print_json(items);
        return;
    }
    print_checklist("Reco doctor", items);
}

pub fn print_setup(items: &[DoctorItem], json: bool) {
    if json {
        let payload = serde_json::json!({
            "items": items,
            "install": INSTALL_ONE_LINER,
        });
        match serde_json::to_string_pretty(&payload) {
            Ok(text) => println!("{text}"),
            Err(err) => {
                eprintln!("No se pudo serializar: {err}");
                std::process::exit(1);
            }
        }
        return;
    }
    print_checklist("Reco setup", items);
    let llama_missing = items
        .iter()
        .any(|item| item.name == "llama-cli" && item.ok == Some(false));
    println!();
    println!("{}", "Siguiente".bold());
    if llama_missing {
        println!("  {}", INSTALL_ONE_LINER.cyan());
    }
    println!("  {}  ventana Prueba", "reco desktop".cyan());
    println!("  {}  catálogo", "reco ai".cyan());
}

fn print_json(items: &[DoctorItem]) {
    match serde_json::to_string_pretty(items) {
        Ok(text) => println!("{text}"),
        Err(err) => {
            eprintln!("No se pudo serializar: {err}");
            std::process::exit(1);
        }
    }
}

fn print_checklist(title: &str, items: &[DoctorItem]) {
    println!("{}", title.bold());
    println!();
    for item in items {
        let mark = match item.ok {
            Some(true) => "✓".green().to_string(),
            Some(false) => "✗".red().to_string(),
            None => "·".dimmed().to_string(),
        };
        println!("  {mark}  {:<10}  {}", item.name, item.detail);
        if let Some(hint) = &item.hint {
            println!("      {}", hint.dimmed());
        }
    }
}

pub fn print_models(models: &[DownloadedModel], json: bool) {
    if json {
        let rows: Vec<serde_json::Value> = models
            .iter()
            .map(|m| {
                serde_json::json!({
                    "repo_id": m.repo_id,
                    "filename": m.filename,
                    "size_bytes": m.size_bytes,
                    "path": m.path,
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&rows).unwrap_or_default()
        );
        return;
    }
    if models.is_empty() {
        println!("{}", "No hay GGUF descargados.".bold());
        println!("  {}", "reco ai   elige uno ·  reco run <modelo>".dimmed());
        return;
    }
    let total: u64 = models.iter().map(|m| m.size_bytes).sum();
    println!(
        "{}  {} · {}",
        "Modelos locales".bold(),
        format_gib(total),
        reco_catalog::models_dir().display().to_string().dimmed()
    );
    println!();
    let mut last = "";
    for model in models {
        if model.repo_id != last {
            println!("  {}", model.repo_id.bold());
            last = &model.repo_id;
        }
        println!(
            "    {}  {}",
            model.filename.cyan(),
            format_gib(model.size_bytes).dimmed()
        );
    }
    println!();
    println!("  {}", "reco models rm <repo>   borra del caché".dimmed());
}

pub fn print_config(cfg: &RecoConfig, json: bool) {
    if json {
        match serde_json::to_string_pretty(cfg) {
            Ok(text) => println!("{text}"),
            Err(err) => {
                eprintln!("No se pudo serializar: {err}");
                std::process::exit(1);
            }
        }
        return;
    }
    println!("{}  {}", "Config".bold(), config_path().display().dimmed());
    println!();
    println!(
        "  {}  {}",
        "openai-key     ".dimmed(),
        empty_or(&cfg.byok.openai_key)
    );
    println!("  {}  {}", "openai-base    ".dimmed(), cfg.byok.openai_base);
    println!(
        "  {}  {}",
        "openai-model   ".dimmed(),
        cfg.byok.openai_model
    );
    println!(
        "  {}  {}",
        "anthropic-key  ".dimmed(),
        empty_or(&cfg.byok.anthropic_key)
    );
    println!(
        "  {}  {}",
        "anthropic-model".dimmed(),
        cfg.byok.anthropic_model
    );
    println!(
        "  {}  {}",
        "llama-cli      ".dimmed(),
        cfg.llama.cli.as_deref().unwrap_or("—")
    );
    println!("  {}  {}", "n-predict      ".dimmed(), cfg.llama.n_predict);
    println!("  {}  {}", "n-ctx          ".dimmed(), cfg.llama.n_ctx);
    println!(
        "  {}  {}",
        "n-gpu-layers   ".dimmed(),
        cfg.llama.n_gpu_layers
    );
}

fn empty_or(value: &str) -> &str {
    if value.is_empty() {
        "—"
    } else {
        value
    }
}

fn pad(text: &str, width: usize) -> String {
    let visible = text.chars().count();
    if visible >= width {
        return text
            .chars()
            .take(width.saturating_sub(1))
            .collect::<String>()
            + "…";
    }
    format!("{text}{}", " ".repeat(width - visible))
}
