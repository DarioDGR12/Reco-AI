mod render;

use clap::{Parser, Subcommand};
use reco_core::detect;
use render::{print_ai, print_hw, print_stub};

#[derive(Parser)]
#[command(
    name = "reco",
    version,
    about = "Reco AI — elige y corre modelos locales según tu hardware",
    long_about = "Unifica el catálogo GGUF de Hugging Face, la detección de hardware y un chat nativo (Prueba).\n\nHoy: reco ai / reco hw detectan tu máquina. reco run y reco serve llegan en siguientes versiones."
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
    },
    /// Muestra el perfil de hardware detectado
    Hw {
        /// Imprime el perfil en JSON
        #[arg(long)]
        json: bool,
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

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Ai { json } => {
            let profile = detect();
            print_ai(&profile, json);
        }
        Commands::Hw { json } => {
            let profile = detect();
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
