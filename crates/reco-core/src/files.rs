use crate::model::{GgufFile, ModelParams};
use crate::quant::{estimate_size_bytes, is_sharded, GgufQuant};

/// Turn Hugging Face sibling filenames into scored GGUF files.
pub fn files_from_siblings(
    repo_id: &str,
    names: impl IntoIterator<Item = impl AsRef<str>>,
    params: Option<ModelParams>,
) -> Vec<GgufFile> {
    let params = params.or_else(|| crate::quant::parse_param_count(repo_id));
    let mut files = Vec::new();

    for name in names {
        let name = name.as_ref();
        let Some(quant) = GgufQuant::parse(name) else {
            continue;
        };
        if matches!(quant, GgufQuant::Other(_)) && !name.to_ascii_lowercase().ends_with(".gguf") {
            continue;
        }
        let sharded = is_sharded(name);
        let size_bytes = params
            .map(|p| estimate_size_bytes(p, &quant))
            .unwrap_or(0);
        files.push(GgufFile {
            filename: name.to_string(),
            quant,
            size_bytes,
            size_estimated: true,
            sharded,
        });
    }

    files.sort_by(|a, b| a.filename.cmp(&b.filename));
    files.dedup_by(|a, b| a.filename == b.filename);
    files
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quant::GgufQuant;

    #[test]
    fn skips_mmproj_and_parses_q4km() {
        let files = files_from_siblings(
            "Qwen/Qwen2.5-7B-Instruct-GGUF",
            [
                "qwen2.5-7b-instruct-q4_k_m.gguf",
                "qwen2.5-7b-instruct-mmproj-f16.gguf",
                "README.md",
                "qwen2.5-7b-instruct-q4_k_m-00001-of-00002.gguf",
            ],
            None,
        );
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].quant, GgufQuant::Q4Km);
        assert!(files[0].sharded);
        assert!(!files[1].sharded);
        assert!(files.iter().all(|f| f.size_bytes > 3_000_000_000));
    }
}
