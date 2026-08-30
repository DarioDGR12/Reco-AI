use owo_colors::OwoColorize;
use reco_core::{format_gib, AccelBackend, HardwareProfile};

pub fn print_ai(profile: &HardwareProfile, json: bool) {
    if json {
        print_json(profile);
        return;
    }
    print_profile_box(profile);
    println!();
    println!("{}", "Recomendaciones".bold());
    println!("  El catálogo de Hugging Face aún no está indexado.");
    println!(
        "  Cuando lo esté, Reco ponderará 40% compatibilidad, 20% velocidad, 20% calidad y 20% popularidad."
    );
    println!("  Usa {} para el perfil crudo.", "reco hw --json".cyan());
}

pub fn print_hw(profile: &HardwareProfile, json: bool) {
    if json {
        print_json(profile);
        return;
    }
    print_profile_box(profile);
}

pub fn print_stub(command: &str, modelo: &str, next: &str) {
    println!(
        "{} {} todavía no está implementado.",
        "reco".bold(),
        command.bold()
    );
    println!("  Modelo pedido: {}", modelo.cyan());
    println!("  {next}");
}

fn print_json(profile: &HardwareProfile) {
    match serde_json::to_string_pretty(profile) {
        Ok(json) => println!("{json}"),
        Err(err) => {
            eprintln!("No se pudo serializar el perfil: {err}");
            std::process::exit(1);
        }
    }
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
