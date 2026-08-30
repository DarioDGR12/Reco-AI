mod prueba;
mod render;
mod run;
mod server;
mod tui;

use std::io::{self, IsTerminal};

use clap::{Parser, Subcommand};
use reco_catalog::{load_catalog, LoadOptions};
use reco_core::hardware::fixtures;
use reco_core::{detect, recommend, resolve_spec};
use render::{print_ai, print_hw};
use serde::Serialize;

#[derive(Parser)]
#[command(
    name = "reco",
    version,
    about = "Reco AI — elige y corre modelos locales según tu hardware",
    long_about = "Catálogo GGUF de Hugging Face + hardware + TUI + descarga + Prueba (chat) + reco serve.\n\nLa ventana Tauri y llama.cpp real llegan después; hoy Prueba y serve usan un motor demo intercambiable."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Recomienda modelos que corren bien en este hardware
    Ai {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        list: bool,
        #[arg(long)]
        tui: bool,
        #[arg(long, default_value_t = 12)]
        limit: usize,
        #[arg(long)]
        refresh: bool,
        #[arg(long)]
        offline: bool,
        #[arg(long, hide = true)]
        fixture: Option<String>,
    },
    /// Muestra el perfil de hardware detectado
    Hw {
        #[arg(long)]
        json: bool,
        #[arg(long, hide = true)]
        fixture: Option<String>,
    },
    /// Descarga el GGUF y abre Prueba
    Run {
        modelo: String,
        #[arg(long)]
        dry_run: bool,
        /// Abre Prueba con EchoEngine, sin descargar
        #[arg(long)]
        demo: bool,
        /// Descarga (si hace falta) y no abre el chat
        #[arg(long)]
        no_chat: bool,
        #[arg(long)]
        offline: bool,
        #[arg(long)]
        refresh: bool,
        #[arg(long, hide = true)]
        fixture: Option<String>,
    },
    /// Reabre Prueba (historial SQLite) para un modelo
    Chat {
        modelo: String,
        #[arg(long)]
        demo: bool,
        #[arg(long)]
        offline: bool,
        #[arg(long, hide = true)]
        fixture: Option<String>,
    },
    /// API local estilo OpenAI + API key `sk-reco-...`
    Serve {
        modelo: String,
        #[arg(long, default_value_t = 11434)]
        port: u16,
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
        #[arg(long)]
        demo: bool,
        #[arg(long)]
        offline: bool,
        #[arg(long)]
        refresh: bool,
        #[arg(long, hide = true)]
        fixture: Option<String>,
    },
}

#[derive(Serialize)]
struct AiJson<'a> {
    hardware: &'a reco_core::HardwareProfile,
    catalog_source: reco_core::CatalogSource,
    catalog_models: usize,
    notes: &'a [String],
    recommendations: &'a [reco_core::Recommendation],
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Ai {
            json,
            list,
            tui: force_tui,
            limit,
            refresh,
            offline,
            fixture,
        } => cmd_ai(json, list, force_tui, limit, refresh, offline, fixture),
        Commands::Hw { json, fixture } => {
            print_hw(&resolve_profile(fixture.as_deref()), json);
        }
        Commands::Run {
            modelo,
            dry_run,
            demo,
            no_chat,
            offline,
            refresh,
            fixture,
        } => {
            let rec = resolve_model(&modelo, offline, refresh, fixture.as_deref());
            if dry_run {
                if let Err(err) = run::download_recommendation(&rec, true) {
                    fail(err);
                }
                return;
            }
            if !demo {
                if let Err(err) = run::download_recommendation(&rec, false) {
                    fail(err);
                }
            }
            if !no_chat {
                if let Err(err) = run::open_prueba(&rec, true) {
                    fail(err);
                }
            }
        }
        Commands::Chat {
            modelo,
            demo,
            offline,
            fixture,
        } => {
            let rec = resolve_model(&modelo, offline, false, fixture.as_deref());
            if let Err(err) = run::open_prueba(&rec, demo) {
                fail(err);
            }
        }
        Commands::Serve {
            modelo,
            port,
            host,
            demo,
            offline,
            refresh,
            fixture,
        } => {
            let rec = resolve_model(&modelo, offline, refresh, fixture.as_deref());
            if let Err(err) = server::run(&rec, port, &host, demo) {
                fail(err);
            }
        }
    }
}

fn cmd_ai(
    json: bool,
    list: bool,
    force_tui: bool,
    limit: usize,
    refresh: bool,
    offline: bool,
    fixture: Option<String>,
) {
    let profile = resolve_profile(fixture.as_deref());
    if !json && !offline {
        eprintln!("Indexando catálogo GGUF de Hugging Face…");
    }
    let (catalog, notes) = load_catalog(LoadOptions {
        refresh,
        offline,
        limit: 80,
        ..LoadOptions::default()
    });
    let recs = recommend(&profile, &catalog, limit.max(1));
    if json {
        let payload = AiJson {
            hardware: &profile,
            catalog_source: catalog.source,
            catalog_models: catalog.models.len(),
            notes: &notes,
            recommendations: &recs,
        };
        match serde_json::to_string_pretty(&payload) {
            Ok(text) => println!("{text}"),
            Err(err) => fail(format!("No se pudo serializar: {err}")),
        }
        return;
    }
    let use_tui = (force_tui || io::stdout().is_terminal()) && !list;
    if use_tui {
        match tui::run(&profile, recs, catalog.source) {
            Ok(Some(chosen)) => {
                if let Err(err) = run::download_recommendation(&chosen, false) {
                    fail(err);
                }
                if let Err(err) = run::open_prueba(&chosen, true) {
                    fail(err);
                }
            }
            Ok(None) => {}
            Err(err) => fail(format!("TUI: {err}")),
        }
    } else {
        print_ai(
            &profile,
            &recs,
            &notes,
            catalog.source,
            catalog.models.len(),
        );
    }
}

fn resolve_model(
    modelo: &str,
    offline: bool,
    refresh: bool,
    fixture: Option<&str>,
) -> reco_core::Recommendation {
    let profile = resolve_profile(fixture);
    let (catalog, _) = load_catalog(LoadOptions {
        refresh,
        offline,
        limit: 80,
        ..LoadOptions::default()
    });
    match resolve_spec(&profile, &catalog, modelo) {
        Ok(rec) => rec,
        Err(err) => fail(err.to_string()),
    }
}

fn resolve_profile(fixture: Option<&str>) -> reco_core::HardwareProfile {
    match fixture {
        Some(name) => fixtures::by_name(name).unwrap_or_else(|| {
            fail(format!(
                "Fixture desconocido '{name}'. Usa: rtx4060, apple-m3, cpu"
            ));
        }),
        None => detect(),
    }
}

fn fail(message: impl std::fmt::Display) -> ! {
    eprintln!("{message}");
    std::process::exit(1);
}
