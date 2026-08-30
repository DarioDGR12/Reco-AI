use reco_catalog::{cache_root, dir_size, list_downloaded, load_cache};
use reco_core::infer::LlamaCliEngine;
use reco_core::{config_path, format_gib, RecoConfig};

#[derive(Debug, Clone, serde::Serialize)]
pub struct DoctorItem {
    pub ok: Option<bool>,
    pub name: String,
    pub detail: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
}

pub fn collect(profile: &reco_core::HardwareProfile) -> Vec<DoctorItem> {
    let mut items = Vec::new();
    let cfg = RecoConfig::load();

    let gpu = profile
        .gpus
        .first()
        .map(|g| {
            let vram = g.vram_bytes.map(format_gib).unwrap_or_else(|| "?".into());
            format!("{} · {vram} · {}", g.name, g.backend.display_name())
        })
        .unwrap_or_else(|| "sin GPU (CPU)".into());
    items.push(DoctorItem {
        ok: Some(true),
        name: "hardware".into(),
        detail: format!(
            "{} · {} RAM · {gpu}",
            profile.cpu.name,
            format_gib(profile.memory.total_bytes)
        ),
        hint: None,
    });

    match LlamaCliEngine::find_binary(cfg.llama.cli.as_deref()) {
        Some(path) => items.push(DoctorItem {
            ok: Some(true),
            name: "llama-cli".into(),
            detail: path.display().to_string(),
            hint: None,
        }),
        None => items.push(DoctorItem {
            ok: Some(false),
            name: "llama-cli".into(),
            detail: "no está en PATH".into(),
            hint: Some(
                "instala llama.cpp o: reco config set llama-cli /ruta/llama-cli".into(),
            ),
        }),
    }

    if cfg.byok.openai_key.is_empty() {
        items.push(DoctorItem {
            ok: None,
            name: "openai".into(),
            detail: "sin clave".into(),
            hint: Some("reco config set openai-key sk-...  (opcional, BYOK)".into()),
        });
    } else {
        items.push(DoctorItem {
            ok: Some(true),
            name: "openai".into(),
            detail: format!(
                "{} · {}",
                reco_core::config::mask_secret(&cfg.byok.openai_key),
                cfg.byok.openai_model
            ),
            hint: None,
        });
    }

    if cfg.byok.anthropic_key.is_empty() {
        items.push(DoctorItem {
            ok: None,
            name: "anthropic".into(),
            detail: "sin clave".into(),
            hint: None,
        });
    } else {
        items.push(DoctorItem {
            ok: Some(true),
            name: "anthropic".into(),
            detail: format!(
                "{} · {}",
                reco_core::config::mask_secret(&cfg.byok.anthropic_key),
                cfg.byok.anthropic_model
            ),
            hint: None,
        });
    }

    let downloaded = list_downloaded();
    let model_bytes: u64 = downloaded.iter().map(|m| m.size_bytes).sum();
    items.push(DoctorItem {
        ok: Some(true),
        name: "modelos".into(),
        detail: format!(
            "{} GGUF · {} en {}",
            downloaded.len(),
            format_gib(model_bytes),
            reco_catalog::models_dir().display()
        ),
        hint: None,
    });

    let cache = cache_root();
    let catalog = if load_cache().is_some() {
        "caché de catálogo presente"
    } else {
        "sin caché de catálogo (se usará HF o la semilla)"
    };
    items.push(DoctorItem {
        ok: Some(true),
        name: "caché".into(),
        detail: format!("{} · {}", format_gib(dir_size(&cache)), catalog),
        hint: None,
    });

    items.push(DoctorItem {
        ok: Some(true),
        name: "config".into(),
        detail: config_path().display().to_string(),
        hint: None,
    });

    let apis = reco_core::ApiRegistry::load();
    items.push(DoctorItem {
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
    });

    items
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
    }
}
