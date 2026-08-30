use crate::model::{Catalog, Recommendation};
use crate::recommend::recommend;
use crate::HardwareProfile;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolveError {
    Empty,
    NotFound(String),
    Ambiguous(Vec<String>),
    NoFit(String),
}

impl std::fmt::Display for ResolveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => write!(f, "indica un modelo, por ejemplo Qwen/Qwen2.5-7B-Instruct-GGUF"),
            Self::NotFound(spec) => write!(f, "no encontré '{spec}' en el catálogo"),
            Self::Ambiguous(ids) => write!(
                f,
                "hay varios repos que coinciden: {}",
                ids.join(", ")
            ),
            Self::NoFit(repo) => write!(
                f,
                "{repo} no tiene un GGUF que entre cómodo en este hardware"
            ),
        }
    }
}

impl std::error::Error for ResolveError {}

/// `repo`, `repo:archivo.gguf`, or a unique substring of the repo id.
pub fn resolve_spec(
    profile: &HardwareProfile,
    catalog: &Catalog,
    spec: &str,
) -> Result<Recommendation, ResolveError> {
    let spec = spec.trim();
    if spec.is_empty() {
        return Err(ResolveError::Empty);
    }

    let (repo_query, file_query) = match spec.split_once(':') {
        Some((repo, file)) if !file.is_empty() => (repo.trim(), Some(file.trim())),
        _ => (spec, None),
    };

    let matches = find_repos(catalog, repo_query);
    let repo_id = match matches.as_slice() {
        [] => return Err(ResolveError::NotFound(repo_query.to_string())),
        [id] => id.clone(),
        more => {
            return Err(ResolveError::Ambiguous(more.to_vec()));
        }
    };

    let Some(model) = catalog.models.iter().find(|m| m.repo_id == repo_id) else {
        return Err(ResolveError::NotFound(repo_id));
    };

    let mut slim = catalog.clone();
    slim.models.retain(|m| m.repo_id == repo_id);
    if let Some(file) = file_query {
        let lower = file.to_ascii_lowercase();
        slim.models[0]
            .files
            .retain(|f| f.filename == file || f.filename.to_ascii_lowercase().ends_with(&lower));
        if slim.models[0].files.is_empty() {
            return Err(ResolveError::NotFound(format!("{repo_id}:{file}")));
        }
    }

    recommend(profile, &slim, 1)
        .into_iter()
        .next()
        .ok_or_else(|| ResolveError::NoFit(model.repo_id.clone()))
}

fn find_repos(catalog: &Catalog, query: &str) -> Vec<String> {
    let q = query.to_ascii_lowercase();
    let exact: Vec<String> = catalog
        .models
        .iter()
        .filter(|m| m.repo_id.eq_ignore_ascii_case(query))
        .map(|m| m.repo_id.clone())
        .collect();
    if !exact.is_empty() {
        return exact;
    }
    catalog
        .models
        .iter()
        .filter(|m| m.repo_id.to_ascii_lowercase().contains(&q))
        .map(|m| m.repo_id.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hardware::fixtures::rtx_4060;
    use crate::model::{CatalogSource, GgufFile, ModelEntry, ModelParams};
    use crate::quant::GgufQuant;

    fn catalog() -> Catalog {
        Catalog {
            version: 1,
            fetched_at: None,
            source: CatalogSource::Seed,
            models: vec![
                entry(
                    "Qwen/Qwen2.5-7B-Instruct-GGUF",
                    "qwen2.5-7b-instruct-q4_k_m.gguf",
                    4_000_000_000,
                ),
                entry(
                    "bartowski/Llama-3.1-8B-Instruct-GGUF",
                    "Llama-3.1-8B-Instruct-Q4_K_M.gguf",
                    4_500_000_000,
                ),
            ],
        }
    }

    fn entry(repo: &str, file: &str, size: u64) -> ModelEntry {
        ModelEntry {
            repo_id: repo.into(),
            downloads: 1_000_000,
            likes: 100,
            tags: vec!["conversational".into(), "text-generation".into()],
            pipeline_tag: Some("text-generation".into()),
            params: Some(ModelParams {
                total_billions: 7.0,
                active_billions: None,
            }),
            files: vec![GgufFile {
                filename: file.into(),
                quant: GgufQuant::Q4Km,
                size_bytes: size,
                size_estimated: true,
                sharded: false,
            }],
        }
    }

    #[test]
    fn exact_repo_and_substring() {
        let profile = rtx_4060();
        let cat = catalog();
        let rec = resolve_spec(&profile, &cat, "Qwen/Qwen2.5-7B-Instruct-GGUF").unwrap();
        assert_eq!(rec.repo_id, "Qwen/Qwen2.5-7B-Instruct-GGUF");
        let rec = resolve_spec(&profile, &cat, "Llama-3.1-8B").unwrap();
        assert!(rec.repo_id.contains("Llama-3.1-8B"));
    }

    #[test]
    fn file_suffix_and_ambiguous() {
        let profile = rtx_4060();
        let cat = catalog();
        let rec = resolve_spec(
            &profile,
            &cat,
            "Qwen/Qwen2.5-7B-Instruct-GGUF:qwen2.5-7b-instruct-q4_k_m.gguf",
        )
        .unwrap();
        assert!(rec.filename.ends_with("q4_k_m.gguf"));
        assert!(matches!(
            resolve_spec(&profile, &cat, "Instruct-GGUF"),
            Err(ResolveError::Ambiguous(_))
        ));
        assert!(matches!(
            resolve_spec(&profile, &cat, "no-such-model"),
            Err(ResolveError::NotFound(_))
        ));
    }
}
