use reco_catalog::{cache_root, dir_size, list_downloaded, load_cache};
use reco_core::infer::LlamaCliEngine;
use reco_core::{config_path, format_gib, RecoConfig};

/// One-liner from the public repo. Do not run it from the CLI; only print it.
pub const INSTALL_ONE_LINER: &str =
    "curl -fsSL https://raw.githubusercontent.com/DarioDGR12/Reco-AI/main/scripts/install.sh | bash";

#[derive(Debug, Clone, serde::Serialize)]
pub struct DoctorItem {
    pub ok: Option<bool>,
    pub name: String,
    pub detail: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
}

pub fn collect(profile: &reco_core::HardwareProfile) -> Vec<DoctorItem> {
    let cfg = RecoConfig::load();
    vec![
        hardware_item(profile),
        llama_item(&cfg),
        openai_item(&cfg),
        anthropic_item(&cfg),
        modelos_item(),
        cache_item(),
        config_item(),
        ventana_item(),
        apis_item(),
    ]
}

/// Onboarding subset: hardware, llama-cli, ventana, config, modelos.
pub fn collect_setup(profile: &reco_core::HardwareProfile) -> Vec<DoctorItem> {
    let cfg = RecoConfig::load();
    vec![
        hardware_item(profile),
        llama_item(&cfg),
        ventana_item(),
        config_item(),
        modelos_item(),
    ]
}

fn hardware_item(profile: &reco_core::HardwareProfile) -> DoctorItem {
    let gpu = profile
        .gpus
        .first()
        .map(|g| {
            let vram = g.vram_bytes.map(format_gib).unwrap_or_else(|| "?".into());
            format!("{} · {vram} · {}", g.name, g.backend.display_name())
        })
        .unwrap_or_else(|| "sin GPU (CPU)".into());
    DoctorItem {
        ok: Some(true),
        name: "hardware".into(),
        detail: format!(
            "{} · {} RAM · {gpu}",
            profile.cpu.name,
            format_gib(profile.memory.total_bytes)
        ),
        hint: None,
    }
}

fn llama_item(cfg: &RecoConfig) -> DoctorItem {
    match LlamaCliEngine::find_binary(cfg.llama.cli.as_deref()) {
        Some(path) => DoctorItem {
            ok: Some(true),
            name: "llama-cli".into(),
            detail: path.display().to_string(),
            hint: None,
        },
        None => DoctorItem {
            ok: Some(false),
            name: "llama-cli".into(),
            detail: "no está en PATH".into(),
            hint: Some(llama_missing_hint()),
        },
    }
}

pub(crate) fn llama_missing_hint() -> String {
    INSTALL_ONE_LINER.to_string()
}

fn openai_item(cfg: &RecoConfig) -> DoctorItem {
    if cfg.byok.openai_key.is_empty() {
        DoctorItem {
            ok: None,
            name: "openai".into(),
            detail: "sin clave".into(),
            hint: Some("reco config set openai-key sk-...  (opcional, BYOK)".into()),
        }
    } else {
        DoctorItem {
            ok: Some(true),
            name: "openai".into(),
            detail: format!(
                "{} · {}",
                reco_core::config::mask_secret(&cfg.byok.openai_key),
                cfg.byok.openai_model
            ),
            hint: None,
        }
    }
}

fn anthropic_item(cfg: &RecoConfig) -> DoctorItem {
    if cfg.byok.anthropic_key.is_empty() {
        DoctorItem {
            ok: None,
            name: "anthropic".into(),
            detail: "sin clave".into(),
            hint: None,
        }
    } else {
        DoctorItem {
            ok: Some(true),
            name: "anthropic".into(),
            detail: format!(
                "{} · {}",
                reco_core::config::mask_secret(&cfg.byok.anthropic_key),
                cfg.byok.anthropic_model
            ),
            hint: None,
        }
    }
}

fn modelos_item() -> DoctorItem {
    let downloaded = list_downloaded();
    let model_bytes: u64 = downloaded.iter().map(|m| m.size_bytes).sum();
    DoctorItem {
        ok: Some(true),
        name: "modelos".into(),
        detail: format!(
            "{} GGUF · {} en {}",
            downloaded.len(),
            format_gib(model_bytes),
            reco_catalog::models_dir().display()
        ),
        hint: None,
    }
}

fn cache_item() -> DoctorItem {
    let cache = cache_root();
    let catalog = if load_cache().is_some() {
        "caché de catálogo presente"
    } else {
        "sin caché de catálogo (se usará HF o la semilla)"
    };
    DoctorItem {
        ok: Some(true),
        name: "caché".into(),
        detail: format!("{} · {}", format_gib(dir_size(&cache)), catalog),
        hint: None,
    }
}

fn config_item() -> DoctorItem {
    DoctorItem {
        ok: Some(true),
        name: "config".into(),
        detail: config_path().display().to_string(),
        hint: None,
    }
}

fn ventana_item() -> DoctorItem {
    match crate::run::desktop_binary() {
        Some(path) => DoctorItem {
            ok: Some(true),
            name: "ventana".into(),
            detail: path.display().to_string(),
            hint: Some("reco desktop   ·   reco run abre esta ventana".into()),
        },
        None => DoctorItem {
            ok: None,
            name: "ventana".into(),
            detail: "reco-desktop no está instalado".into(),
            hint: Some(ventana_missing_hint()),
        },
    }
}

pub(crate) fn ventana_missing_hint() -> String {
    INSTALL_ONE_LINER.to_string()
}

fn apis_item() -> DoctorItem {
    let apis = reco_core::ApiRegistry::load();
    DoctorItem {
        ok: Some(true),
        name: "apis".into(),
        detail: if apis.endpoints.is_empty() {
            "ninguna generada".into()
        } else {
            format!(
                "{} · {}",
                apis.endpoints.len(),
                apis.endpoints
                    .iter()
                    .map(|e| e.slug.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        },
        hint: if apis.endpoints.is_empty() {
            Some("reco api create <modelo> --name mi-app".into())
        } else {
            Some("reco api start   hub en esta máquina".into())
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reco_core::hardware::fixtures::rtx_4060;

    #[test]
    fn report_has_hardware_and_llama() {
        let items = collect(&rtx_4060());
        assert!(items.iter().any(|i| i.name == "hardware"));
        assert!(items.iter().any(|i| i.name == "llama-cli"));
        assert!(items.iter().any(|i| i.name == "config"));
        assert!(items.iter().any(|i| i.name == "ventana"));
    }

    #[test]
    fn setup_and_doctor_have_hardware_llama_ventana() {
        let profile = rtx_4060();
        for items in [collect(&profile), collect_setup(&profile)] {
            assert!(items.iter().any(|i| i.name == "hardware"));
            assert!(items.iter().any(|i| i.name == "llama-cli"));
            assert!(items.iter().any(|i| i.name == "ventana"));
        }
        let setup_items = collect_setup(&profile);
        let setup_names: Vec<_> = setup_items.iter().map(|i| i.name.as_str()).collect();
        assert_eq!(
            setup_names,
            ["hardware", "llama-cli", "ventana", "config", "modelos"]
        );
    }

    #[test]
    fn missing_hints_point_at_install_sh() {
        assert!(INSTALL_ONE_LINER.contains("scripts/install.sh"));
        assert!(llama_missing_hint().contains("install.sh"));
        assert!(ventana_missing_hint().contains("install.sh"));
        assert_eq!(llama_missing_hint(), INSTALL_ONE_LINER);
        assert_eq!(ventana_missing_hint(), INSTALL_ONE_LINER);
    }
}
