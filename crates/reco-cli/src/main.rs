mod doctor;
mod prueba;
mod render;
mod run;
mod server;
mod tui;

use std::collections::HashSet;
use std::io::{self, IsTerminal};

use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Shell};
use reco_catalog::{
    cache_root, is_downloaded, list_downloaded, load_catalog, remove_downloaded, LoadOptions,
};
use reco_core::hardware::fixtures;
use reco_core::store::ChatStore;
use reco_core::{
    config_path, detect, recommend, resolve_spec, suggest_repos, RecoConfig, ResolveError,
};
use render::{print_ai, print_config, print_doctor, print_home, print_hw, print_models};

#[derive(Parser)]
#[command(
    name = "reco",
    version,
    about = "Reco AI — elige y corre el modelo que cabe en tu máquina",
    long_about = "Reco lee tu hardware, indexa GGUF en Hugging Face y te deja chatear o servir el modelo en un comando.\n\nSin argumentos muestra el estado de esta máquina.",
    after_help = "Ejemplos:\n  reco                      estado y siguientes pasos\n  reco ai                   catálogo que cabe aquí\n  reco run Qwen2.5-7B       descarga y abre Prueba\n  reco doctor               llama.cpp, claves y caché\n  reco serve Llama-3.1-8B   API local estilo OpenAI"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Catálogo GGUF ordenado para este hardware
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
    /// Perfil de hardware detectado
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
        #[arg(long)]
        demo: bool,
        #[arg(long)]
        no_chat: bool,
        #[arg(long, default_value = "auto")]
        provider: String,
        #[arg(long)]
        offline: bool,
        #[arg(long)]
        refresh: bool,
        #[arg(long, hide = true)]
        fixture: Option<String>,
    },
    /// Reabre Prueba para un modelo
    Chat {
        modelo: String,
        #[arg(long)]
        demo: bool,
        #[arg(long, default_value = "auto")]
        provider: String,
        #[arg(long)]
        offline: bool,
        #[arg(long, hide = true)]
        fixture: Option<String>,
    },
    /// API local compatible con OpenAI
    Serve {
        modelo: String,
        #[arg(long, default_value_t = 11434)]
        port: u16,
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
        #[arg(long)]
        demo: bool,
        #[arg(long, default_value = "auto")]
        provider: String,
        #[arg(long)]
        offline: bool,
        #[arg(long)]
        refresh: bool,
        #[arg(long, hide = true)]
        fixture: Option<String>,
    },
    /// Revisa llama.cpp, claves BYOK y el caché
    Doctor {
        #[arg(long)]
        json: bool,
        #[arg(long, hide = true)]
        fixture: Option<String>,
    },
    /// GGUF ya descargados
    Models {
        #[command(subcommand)]
        action: Option<ModelsCmd>,
        #[arg(long)]
        json: bool,
    },
    /// Claves BYOK y llama-cli
    Config {
        #[command(subcommand)]
        action: ConfigCmd,
    },
    /// Completados para bash, zsh o fish
    Completions {
        shell: Shell,
    },
}

#[derive(Subcommand)]
enum ModelsCmd {
    /// Borra un repo (o repo:archivo.gguf) del caché
    Rm { modelo: String },
}

#[derive(Subcommand)]
enum ConfigCmd {
    Path,
    Show {
        #[arg(long)]
        json: bool,
    },
    Get {
        key: String,
    },
    Set {
        key: String,
        value: String,
    },
    Unset {
        key: String,
    },
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        None => cmd_home(),
        Some(Commands::Ai {
            json,
            list,
            tui: force_tui,
            limit,
            refresh,
            offline,
            fixture,
        }) => cmd_ai(json, list, force_tui, limit, refresh, offline, fixture),
        Some(Commands::Hw { json, fixture }) => {
            print_hw(&resolve_profile(fixture.as_deref()), json);
        }
        Some(Commands::Run {
            modelo,
            dry_run,
            demo,
            no_chat,
            provider,
            offline,
            refresh,
            fixture,
        }) => {
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
                if let Err(err) = run::open_prueba(&rec, demo, &provider) {
                    fail(err);
                }
            }
        }
        Some(Commands::Chat {
            modelo,
            demo,
            provider,
            offline,
            fixture,
        }) => {
            let rec = resolve_model(&modelo, offline, false, fixture.as_deref());
            if let Err(err) = run::open_prueba(&rec, demo, &provider) {
                fail(err);
            }
        }
        Some(Commands::Serve {
            modelo,
            port,
            host,
            demo,
            provider,
            offline,
            refresh,
            fixture,
        }) => {
            let rec = resolve_model(&modelo, offline, refresh, fixture.as_deref());
            let picked = match run::resolve_engine(&rec, demo, &provider) {
                Ok(picked) => picked,
                Err(err) => fail(err),
            };
            if let Err(err) = server::run(&rec, port, &host, picked) {
                fail(err);
            }
        }
        Some(Commands::Doctor { json, fixture }) => {
            let profile = resolve_profile(fixture.as_deref());
            print_doctor(&doctor::collect(&profile), json);
        }
        Some(Commands::Models { action, json }) => match action {
            None => print_models(&list_downloaded(), json),
            Some(ModelsCmd::Rm { modelo }) => cmd_models_rm(&modelo),
        },
        Some(Commands::Config { action }) => cmd_config(action),
        Some(Commands::Completions { shell }) => {
            let mut cmd = Cli::command();
            generate(shell, &mut cmd, "reco", &mut io::stdout());
        }
    }
}

fn cmd_home() {
    let profile = detect();
    let downloaded = list_downloaded();
    let recent = ChatStore::open(&cache_root().join("reco.db"))
        .ok()
        .and_then(|store| store.list_recent(6).ok())
        .unwrap_or_default();
    print_home(&profile, &downloaded, &recent);
}

fn cmd_config(action: ConfigCmd) {
    match action {
        ConfigCmd::Path => println!("{}", config_path().display()),
        ConfigCmd::Show { json } => print_config(&RecoConfig::load().masked(), json),
        ConfigCmd::Get { key } => match RecoConfig::load().get(&key) {
            Ok(value) => {
                if value.is_empty() {
                    println!("—");
                } else {
                    println!("{value}");
                }
            }
            Err(err) => fail(err),
        },
        ConfigCmd::Set { key, value } => {
            let mut cfg = RecoConfig::load_file();
            if let Err(err) = cfg.set(&key, &value) {
                fail(err);
            }
            if let Err(err) = cfg.save() {
                fail(err);
            }
            println!("guardado {key} en {}", config_path().display());
        }
        ConfigCmd::Unset { key } => {
            let mut cfg = RecoConfig::load_file();
            if let Err(err) = cfg.unset(&key) {
                fail(err);
            }
            if let Err(err) = cfg.save() {
                fail(err);
            }
            println!("borrado {key}");
        }
    }
}

fn cmd_models_rm(spec: &str) {
    let (repo, file) = match spec.split_once(':') {
        Some((repo, file)) if !file.is_empty() => (repo, Some(file)),
        _ => (spec, None),
    };
    match remove_downloaded(repo, file) {
        Ok(bytes) => println!("borrado {} ({})", spec, reco_core::format_gib(bytes)),
        Err(err) => fail(err),
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
        let payload = serde_json::json!({
            "hardware": profile,
            "catalog_source": catalog.source,
            "catalog_models": catalog.models.len(),
            "notes": notes,
            "recommendations": recs,
        });
        match serde_json::to_string_pretty(&payload) {
            Ok(text) => println!("{text}"),
            Err(err) => fail(format!("No se pudo serializar: {err}")),
        }
        return;
    }
    if recs.is_empty() {
        print_ai(
            &profile,
            &recs,
            &notes,
            catalog.source,
            catalog.models.len(),
        );
        return;
    }
    let use_tui = (force_tui || io::stdout().is_terminal()) && !list;
    if use_tui {
        let downloaded: HashSet<String> = recs
            .iter()
            .filter(|r| is_downloaded(&r.repo_id, &r.filename))
            .map(|r| format!("{}:{}", r.repo_id, r.filename))
            .collect();
        match tui::run(&profile, recs, catalog.source, downloaded) {
            Ok(Some(chosen)) => {
                if let Err(err) = run::download_recommendation(&chosen, false) {
                    fail(err);
                }
                if let Err(err) = run::open_prueba(&chosen, false, "auto") {
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
        Err(ResolveError::NotFound(spec)) => {
            let hints = suggest_repos(&catalog, &spec, 5);
            if hints.is_empty() {
                fail(format!(
                    "no encontré '{spec}' en el catálogo.\n  Prueba reco ai o reco ai --refresh"
                ));
            } else {
                fail(format!(
                    "no encontré '{spec}'. ¿Era uno de estos?\n  {}",
                    hints.join("\n  ")
                ));
            }
        }
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
