use reco_core::files::files_from_siblings;
use reco_core::quant::parse_param_count;
use reco_core::{Catalog, CatalogSource, ModelEntry, ModelParams};

struct Seed {
    repo_id: &'static str,
    billions: f64,
    downloads: u64,
    likes: u64,
    files: &'static [&'static str],
}

const SEED: &[Seed] = &[
    Seed {
        repo_id: "Qwen/Qwen2.5-7B-Instruct-GGUF",
        billions: 7.0,
        downloads: 2_400_000,
        likes: 420,
        files: &[
            "qwen2.5-7b-instruct-q4_k_m.gguf",
            "qwen2.5-7b-instruct-q5_k_m.gguf",
            "qwen2.5-7b-instruct-q8_0.gguf",
        ],
    },
    Seed {
        repo_id: "Qwen/Qwen2.5-3B-Instruct-GGUF",
        billions: 3.0,
        downloads: 980_000,
        likes: 180,
        files: &[
            "qwen2.5-3b-instruct-q4_k_m.gguf",
            "qwen2.5-3b-instruct-q5_k_m.gguf",
            "qwen2.5-3b-instruct-q8_0.gguf",
        ],
    },
    Seed {
        repo_id: "Qwen/Qwen2.5-1.5B-Instruct-GGUF",
        billions: 1.5,
        downloads: 720_000,
        likes: 110,
        files: &[
            "qwen2.5-1.5b-instruct-q4_k_m.gguf",
            "qwen2.5-1.5b-instruct-q8_0.gguf",
        ],
    },
    Seed {
        repo_id: "bartowski/Llama-3.1-8B-Instruct-GGUF",
        billions: 8.0,
        downloads: 3_100_000,
        likes: 610,
        files: &[
            "Llama-3.1-8B-Instruct-Q4_K_M.gguf",
            "Llama-3.1-8B-Instruct-Q5_K_M.gguf",
            "Llama-3.1-8B-Instruct-Q8_0.gguf",
        ],
    },
    Seed {
        repo_id: "bartowski/Mistral-7B-Instruct-v0.3-GGUF",
        billions: 7.0,
        downloads: 1_600_000,
        likes: 290,
        files: &[
            "Mistral-7B-Instruct-v0.3-Q4_K_M.gguf",
            "Mistral-7B-Instruct-v0.3-Q5_K_M.gguf",
        ],
    },
    Seed {
        repo_id: "unsloth/Llama-3.2-3B-Instruct-GGUF",
        billions: 3.0,
        downloads: 1_200_000,
        likes: 210,
        files: &[
            "Llama-3.2-3B-Instruct-Q4_K_M.gguf",
            "Llama-3.2-3B-Instruct-Q8_0.gguf",
        ],
    },
    Seed {
        repo_id: "unsloth/Llama-3.2-1B-Instruct-GGUF",
        billions: 1.0,
        downloads: 860_000,
        likes: 140,
        files: &["Llama-3.2-1B-Instruct-Q4_K_M.gguf"],
    },
    Seed {
        repo_id: "microsoft/Phi-3.5-mini-instruct-GGUF",
        billions: 3.8,
        downloads: 1_050_000,
        likes: 260,
        files: &[
            "Phi-3.5-mini-instruct-Q4_K_M.gguf",
            "Phi-3.5-mini-instruct-Q5_K_M.gguf",
        ],
    },
    Seed {
        repo_id: "bartowski/gemma-2-9b-it-GGUF",
        billions: 9.0,
        downloads: 1_400_000,
        likes: 330,
        files: &["gemma-2-9b-it-Q4_K_M.gguf", "gemma-2-9b-it-Q3_K_M.gguf"],
    },
    Seed {
        repo_id: "microsoft/Phi-4-GGUF",
        billions: 14.7,
        downloads: 900_000,
        likes: 200,
        files: &["Phi-4-Q4_K_M.gguf", "Phi-4-Q3_K_M.gguf"],
    },
    Seed {
        repo_id: "Qwen/Qwen2.5-14B-Instruct-GGUF",
        billions: 14.0,
        downloads: 1_100_000,
        likes: 250,
        files: &[
            "qwen2.5-14b-instruct-q4_k_m.gguf",
            "qwen2.5-14b-instruct-q3_k_m.gguf",
        ],
    },
    Seed {
        repo_id: "bartowski/Llama-3.1-70B-Instruct-GGUF",
        billions: 70.0,
        downloads: 2_200_000,
        likes: 540,
        files: &["Llama-3.1-70B-Instruct-Q4_K_M.gguf"],
    },
];

pub fn seed_catalog() -> Catalog {
    let mut catalog = Catalog::empty(CatalogSource::Seed);
    for seed in SEED {
        let params = parse_param_count(seed.repo_id).unwrap_or(ModelParams {
            total_billions: seed.billions,
            active_billions: None,
        });
        let files = files_from_siblings(seed.repo_id, seed.files.iter().copied(), Some(params));
        catalog.models.push(ModelEntry {
            repo_id: seed.repo_id.to_string(),
            downloads: seed.downloads,
            likes: seed.likes,
            tags: vec![
                "gguf".into(),
                "conversational".into(),
                "text-generation".into(),
            ],
            pipeline_tag: Some("text-generation".into()),
            params: Some(params),
            files,
        });
    }
    catalog
}

#[cfg(test)]
mod tests {
    use super::*;
    use reco_core::hardware::fixtures::rtx_4060;
    use reco_core::recommend;

    #[test]
    fn seed_is_usable_on_4060() {
        let catalog = seed_catalog();
        assert!(catalog.models.len() >= 8);
        assert!(catalog.models.iter().all(|m| !m.files.is_empty()));
        let recs = recommend(&rtx_4060(), &catalog, 5);
        assert!(!recs.is_empty());
        assert!(
            recs.iter().all(|r| !r.repo_id.contains("70B")),
            "70B should not be recommended on 8 GB"
        );
    }
}
