mod render;
mod run;
mod tui;

use std::io::{self, IsTerminal};

use clap::{Parser, Subcommand};
use reco_catalog::{load_catalog, LoadOptions};
use reco_core::hardware::fixtures;
use reco_core::{detect, recommend, resolve_spec};
use render::{print_ai, print_hw, print_stub};
use serde::Serialize;

#[derive(Parser)]
#[command(
    name = "reco",
    version,
    about = "Reco AI — elige y corre modelos locales según tu hardware",
    long_about = "Unifica el catálogo GGUF de Hugging Face, la detección de hardware y un chat nativo (Prueba).\n\nreco ai abre una TUI para elegir. reco run descarga el GGUF. Prueba (Tauri) llega después."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Recomienda modelos que corren bien en este hardware
    Ai {
        /// Imprime el perfil y las recomendaciones en JSON
        #[arg(long)]
        json: bool,
        /// Lista en texto (sin TUI)
        #[arg(long)]
        list: bool,
        /// Fuerza la TUI aunque stdout no sea un TTY
        #[arg(long)]
        tui: bool,
        /// Cuántos modelos mostrar
        #[arg(long, default_value_t = 12)]
        limit: usize,
        /// Ignora la caché y vuelve a llamar a Hugging Face
        #[arg(long)]
        refresh: bool,
        /// No llama a la red (caché o semilla)
        #[arg(long)]
        offline: bool,
        /// Perfil de hardware ficticio: rtx4060, apple-m3, cpu
        #[arg(long, hide = true)]
        fixture: Option<String>,
    },
    /// Muestra el perfil de hardware detectado
    Hw {
        /// Imprime el perfil en JSON
        #[arg(long)]
        json: bool,
        #[arg(long, hide = true)]
        fixture: Option<String>,
    },
    /// Descarga el GGUF (elige la cuantización que cabe) y prepara Prueba
    Run {
        /// `org/repo`, substring, o `org/repo:archivo.gguf`
        modelo: String,
        /// Solo muestra URL y destino
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        offline: bool,
        #[arg(long)]
        refresh: bool,
        #[arg(long, hide = true)]
        fixture: Option<String>,
    },
    /// Levanta un servidor local con una API key `sk-...`
    Serve {
        /// Identificador del modelo a servir
        modelo: String,
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
        } => {
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
                    Err(err) => {
                        eprintln!("No se pudo serializar: {err}");
                        std::process::exit(1);
                    }
                }
                return;
            }

            let use_tui = (force_tui || io::stdout().is_terminal()) && !list;
            if use_tui {
                match tui::run(&profile, recs, catalog.source) {
                    Ok(Some(chosen)) => {
                        if let Err(err) = run::download_recommendation(&chosen, false) {
                            eprintln!("{err}");
                            std::process::exit(1);
                        }
                    }
                    Ok(None) => {}
                    Err(err) => {
                        eprintln!("TUI: {err}");
                        std::process::exit(1);
                    }
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
        Commands::Hw { json, fixture } => {
            let profile = resolve_profile(fixture.as_deref());
            print_hw(&profile, json);
        }
        Commands::Run {
            modelo,
            dry_run,
            offline,
            refresh,
            fixture,
        } => {
            let profile = resolve_profile(fixture.as_deref());
            let (catalog, _) = load_catalog(LoadOptions {
                refresh,
                offline,
                limit: 80,
                ..LoadOptions::default()
            });
            match resolve_spec(&profile, &catalog, &modelo) {
                Ok(rec) => {
                    if let Err(err) = run::download_recommendation(&rec, dry_run) {
                        eprintln!("{err}");
                        std::process::exit(1);
                    }
                }
                Err(err) => {
                    eprintln!("{err}");
                    std::process::exit(1);
                }
            }
        }
        Commands::Serve { modelo } => {
            print_stub(
                "serve",
                &modelo,
                "Próximamente: servidor local compatible con otras apps y una API key `sk-...`.",
            );
            std::process::exit(1);
        }
    }
}

fn resolve_profile(fixture: Option<&str>) -> reco_core::HardwareProfile {
    match fixture {
        Some(name) => fixtures::by_name(name).unwrap_or_else(|| {
            eprintln!("Fixture desconocido '{name}'. Usa: rtx4060, apple-m3, cpu");
            std::process::exit(2);
        }),
        None => detect(),
    }
}
