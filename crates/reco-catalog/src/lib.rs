//! Hugging Face GGUF index + on-disk cache. Scoring lives in `reco-core`.

mod cache;
mod client;
mod download;
mod seed;

pub use cache::{cache_path, cache_root, load_cache, save_cache};
pub use client::{fetch_huggingface, FetchOptions};
pub use download::{
    dir_size, download_gguf, huggingface_resolve_url, is_downloaded, list_downloaded,
    local_model_path, models_dir, remove_downloaded, scan_models_dir, DownloadedModel,
};
pub use seed::seed_catalog;

use reco_core::{Catalog, CatalogSource};

#[derive(Debug, thiserror::Error)]
pub enum CatalogError {
    #[error("no se pudo hablar con Hugging Face: {0}")]
    Network(String),
    #[error("respuesta de Hugging Face inválida: {0}")]
    Decode(String),
    #[error("caché local: {0}")]
    Cache(String),
}

#[derive(Debug, Clone, Copy)]
pub struct LoadOptions {
    pub refresh: bool,
    pub offline: bool,
    pub limit: u32,
    pub ttl_secs: u64,
}

impl Default for LoadOptions {
    fn default() -> Self {
        Self {
            refresh: false,
            offline: false,
            limit: 80,
            ttl_secs: 12 * 60 * 60,
        }
    }
}

/// Load a catalog: live HF → fresh cache → stale cache → seed.
pub fn load_catalog(opts: LoadOptions) -> (Catalog, Vec<String>) {
    let mut notes = Vec::new();

    if opts.offline {
        if let Some(cached) = load_cache() {
            notes.push("catálogo: caché local (--offline)".into());
            return (cached, notes);
        }
        notes.push("catálogo: semilla embebida (--offline, sin caché)".into());
        return (seed_catalog(), notes);
    }

    if !opts.refresh {
        if let Some(cached) = load_cache() {
            if cache::is_fresh(&cached, opts.ttl_secs) {
                notes.push("catálogo: caché local (aún fresco)".into());
                return (cached, notes);
            }
        }
    }

    match fetch_huggingface(FetchOptions { limit: opts.limit }) {
        Ok(mut live) => {
            if live.models.is_empty() {
                notes.push("Hugging Face no devolvió modelos GGUF; usando respaldo".into());
            } else {
                live.source = CatalogSource::HuggingFace;
                if let Err(err) = save_cache(&live) {
                    notes.push(format!("no se pudo guardar la caché: {err}"));
                } else {
                    notes.push(format!(
                        "catálogo: Hugging Face ({} repos GGUF)",
                        live.models.len()
                    ));
                }
                return (live, notes);
            }
        }
        Err(err) => notes.push(err.to_string()),
    }

    if let Some(cached) = load_cache() {
        notes.push("catálogo: caché local (Hugging Face no disponible)".into());
        return (cached, notes);
    }

    notes.push("catálogo: semilla embebida (sin red ni caché)".into());
    (seed_catalog(), notes)
}
