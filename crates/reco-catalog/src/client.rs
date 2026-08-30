use reco_core::files::files_from_siblings;
use reco_core::quant::parse_param_count;
use reco_core::{Catalog, CatalogSource, ModelEntry};
use serde::Deserialize;

use crate::CatalogError;

const HF_MODELS: &str = "https://huggingface.co/api/models";
const USER_AGENT: &str = "Reco-AI/0.1 (+https://github.com/DarioDGR12/Reco-AI)";

#[derive(Debug, Clone, Copy)]
pub struct FetchOptions {
    pub limit: u32,
}

impl Default for FetchOptions {
    fn default() -> Self {
        Self { limit: 80 }
    }
}

#[derive(Debug, Deserialize)]
struct HfModel {
    id: String,
    #[serde(default)]
    downloads: u64,
    #[serde(default)]
    likes: u64,
    #[serde(default)]
    tags: Vec<String>,
    pipeline_tag: Option<String>,
    #[serde(default)]
    siblings: Vec<HfSibling>,
}

#[derive(Debug, Deserialize)]
struct HfSibling {
    rfilename: String,
}

pub fn fetch_huggingface(opts: FetchOptions) -> Result<Catalog, CatalogError> {
    let limit = opts.limit.clamp(1, 200);
    let mut request = ureq::get(HF_MODELS)
        .set("User-Agent", USER_AGENT)
        .query("filter", "gguf")
        .query("sort", "downloads")
        .query("direction", "-1")
        .query("limit", &limit.to_string());
    for field in ["siblings", "likes", "tags", "downloads", "pipeline_tag"] {
        request = request.query("expand[]", field);
    }

    let response = request
        .timeout(std::time::Duration::from_secs(30))
        .call()
        .map_err(|err| CatalogError::Network(err.to_string()))?;

    let models: Vec<HfModel> = response
        .into_json()
        .map_err(|err| CatalogError::Decode(err.to_string()))?;

    Ok(catalog_from_hf(models))
}

fn catalog_from_hf(models: Vec<HfModel>) -> Catalog {
    let mut catalog = Catalog::empty(CatalogSource::HuggingFace);
    catalog.fetched_at = Some(now_rfc3339());
    for raw in models {
        let params = parse_param_count(&raw.id)
            .or_else(|| raw.tags.iter().find_map(|tag| parse_param_count(tag)));
        let files = files_from_siblings(
            &raw.id,
            raw.siblings.iter().map(|s| s.rfilename.as_str()),
            params,
        );
        if files.is_empty() {
            continue;
        }
        let entry = ModelEntry {
            repo_id: raw.id,
            downloads: raw.downloads,
            likes: raw.likes,
            tags: raw.tags,
            pipeline_tag: raw.pipeline_tag,
            params,
            files,
        };
        if entry.is_embedding_like() {
            continue;
        }
        catalog.models.push(entry);
    }
    catalog
}

fn now_rfc3339() -> String {
    // Keep this crate free of chrono: a compact UTC-ish stamp is enough for TTL.
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{secs}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_siblings_to_entries() {
        let catalog = catalog_from_hf(vec![HfModel {
            id: "Qwen/Qwen2.5-7B-Instruct-GGUF".into(),
            downloads: 10,
            likes: 2,
            tags: vec!["gguf".into(), "conversational".into()],
            pipeline_tag: Some("text-generation".into()),
            siblings: vec![
                HfSibling {
                    rfilename: "qwen-q4_k_m.gguf".into(),
                },
                HfSibling {
                    rfilename: "README.md".into(),
                },
            ],
        }]);
        assert_eq!(catalog.models.len(), 1);
        assert_eq!(catalog.models[0].files.len(), 1);
        assert_eq!(catalog.models[0].params.unwrap().total_billions, 7.0);
    }
}
