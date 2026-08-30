mod render;

use clap::{Parser, Subcommand};
use reco_catalog::{load_catalog, LoadOptions};
use reco_core::hardware::fixtures;
use reco_core::{detect, recommend};
use render::{print_ai, print_hw, print_stub};
use serde::Serialize;

#[derive(Parser)]
#[command(
    name = "reco",
    version,
    about = "Reco AI — elige y corre modelos locales según tu hardware",
    long_about = "Unifica el catálogo GGUF de Hugging Face, la detección de hardware y un chat nativo (Prueba).\n\nHoy: reco ai recomienda GGUF según tu máquina. reco run y reco serve llegan en siguientes versiones."
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
        /// Cuántos modelos mostrar
        #[arg(long, default_value_t = 8)]
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
    /// Descarga el modelo (si hace falta) y abre Prueba
    Run {
        /// Identificador del modelo (Hugging Face o alias local)
        modelo: String,
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
        Commands::Run { modelo } => {
            print_stub(
                "run",
                &modelo,
                "Próximamente: descarga el GGUF, elige la cuantización que cabe en tu VRAM y abre Prueba.",
            );
            std::process::exit(1);
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
