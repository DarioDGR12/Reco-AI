use serde::{Deserialize, Serialize};

use crate::quant::GgufQuant;

/// Indexed GGUF catalog (Hugging Face, cache, or bundled seed).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Catalog {
    pub version: u32,
    pub fetched_at: Option<String>,
    pub source: CatalogSource,
    pub models: Vec<ModelEntry>,
}

impl Catalog {
    pub const VERSION: u32 = 1;

    pub fn empty(source: CatalogSource) -> Self {
        Self {
            version: Self::VERSION,
            fetched_at: None,
            source,
            models: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CatalogSource {
    HuggingFace,
    Cache,
    Seed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelEntry {
    pub repo_id: String,
    pub downloads: u64,
    pub likes: u64,
    pub tags: Vec<String>,
    pub pipeline_tag: Option<String>,
    pub params: Option<ModelParams>,
    pub files: Vec<GgufFile>,
}

impl ModelEntry {
    pub fn is_conversational(&self) -> bool {
        self.tags.iter().any(|t| {
            matches!(
                t.as_str(),
                "conversational" | "text-generation" | "instruct"
            )
        }) || self.pipeline_tag.as_deref() == Some("text-generation")
    }

    pub fn is_embedding_like(&self) -> bool {
        let blob = format!(
            "{} {} {}",
            self.repo_id.to_ascii_lowercase(),
            self.tags.join(" ").to_ascii_lowercase(),
            self.pipeline_tag.clone().unwrap_or_default()
        );
        blob.contains("embed")
            || blob.contains("sentence-similarity")
            || blob.contains("feature-extraction")
            || blob.contains("rerank")
            || blob.contains("whisper")
            || blob.contains("diffusion")
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ModelParams {
    /// Total parameter count in billions (30.0 for a 30B-A3B MoE).
    pub total_billions: f64,
    /// Active parameters for MoE, if known.
    pub active_billions: Option<f64>,
}

impl ModelParams {
    pub fn effective_billions(self) -> f64 {
        self.active_billions.unwrap_or(self.total_billions)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GgufFile {
    pub filename: String,
    pub quant: GgufQuant,
    pub size_bytes: u64,
    pub size_estimated: bool,
    pub sharded: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recommendation {
    pub repo_id: String,
    pub filename: String,
    pub quant: GgufQuant,
    pub size_bytes: u64,
    pub size_estimated: bool,
    pub params: Option<ModelParams>,
    pub downloads: u64,
    pub scores: Scores,
    pub total: f32,
    pub why: String,
}

impl Recommendation {
    pub fn spec(&self) -> String {
        format!("{}:{}", self.repo_id, self.filename)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Scores {
    pub compatibility: f32,
    pub speed: f32,
    pub quality: f32,
    pub popularity: f32,
}

impl Scores {
    pub const WEIGHT_COMPAT: f32 = 0.40;
    pub const WEIGHT_SPEED: f32 = 0.20;
    pub const WEIGHT_QUALITY: f32 = 0.20;
    pub const WEIGHT_POPULARITY: f32 = 0.20;

    pub fn weighted_total(self) -> f32 {
        self.compatibility * Self::WEIGHT_COMPAT
            + self.speed * Self::WEIGHT_SPEED
            + self.quality * Self::WEIGHT_QUALITY
            + self.popularity * Self::WEIGHT_POPULARITY
    }
}
