use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::cache::cache_root;
use crate::CatalogError;

const USER_AGENT: &str = "Reco-AI/0.1 (+https://github.com/DarioDGR12/Reco-AI)";
const CHUNK: usize = 64 * 1024;

pub fn models_dir() -> PathBuf {
    cache_root().join("models")
}

pub fn local_model_path(repo_id: &str, filename: &str) -> PathBuf {
    models_dir().join(repo_id).join(filename)
}

pub fn huggingface_resolve_url(repo_id: &str, filename: &str) -> String {
    let file_path = filename
        .split('/')
        .map(encode_segment)
        .collect::<Vec<_>>()
        .join("/");
    format!("https://huggingface.co/{repo_id}/resolve/main/{file_path}")
}

fn encode_segment(segment: &str) -> String {
    let mut out = String::new();
    for byte in segment.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char);
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

pub fn is_downloaded(repo_id: &str, filename: &str) -> bool {
    let path = local_model_path(repo_id, filename);
    fs::metadata(path)
        .map(|meta| meta.is_file() && meta.len() > 0)
        .unwrap_or(false)
}

pub fn download_gguf(
    repo_id: &str,
    filename: &str,
    mut on_progress: impl FnMut(u64, Option<u64>),
) -> Result<PathBuf, CatalogError> {
    let dest = local_model_path(repo_id, filename);
    if is_downloaded(repo_id, filename) {
        on_progress(dest_len(&dest), Some(dest_len(&dest)).filter(|n| *n > 0));
        return Ok(dest);
    }

    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).map_err(|err| CatalogError::Cache(err.to_string()))?;
    }

    let url = huggingface_resolve_url(repo_id, filename);
    let agent = ureq::builder()
        .timeout_connect(Duration::from_secs(30))
        .timeout_read(Duration::from_secs(30 * 60))
        .build();
    let response = agent
        .get(&url)
        .set("User-Agent", USER_AGENT)
        .call()
        .map_err(|err| CatalogError::Network(err.to_string()))?;

    let total = response
        .header("Content-Length")
        .and_then(|v| v.parse::<u64>().ok());
    let mut reader = response.into_reader();
    let part = dest.with_extension("gguf.part");
    let mut file = File::create(&part).map_err(|err| CatalogError::Cache(err.to_string()))?;
    let mut buf = vec![0_u8; CHUNK];
    let mut written = 0_u64;

    loop {
        let n = reader
            .read(&mut buf)
            .map_err(|err| CatalogError::Network(err.to_string()))?;
        if n == 0 {
            break;
        }
        file.write_all(&buf[..n])
            .map_err(|err| CatalogError::Cache(err.to_string()))?;
        written += n as u64;
        on_progress(written, total);
    }
    file.flush()
        .map_err(|err| CatalogError::Cache(err.to_string()))?;
    drop(file);
    fs::rename(&part, &dest).map_err(|err| CatalogError::Cache(err.to_string()))?;
    Ok(dest)
}

fn dest_len(path: &Path) -> u64 {
    fs::metadata(path).map(|m| m.len()).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_url_encodes_spaces_keeps_slash() {
        let url = huggingface_resolve_url("Qwen/Qwen2.5-7B-Instruct-GGUF", "Q4_K_M/model q4.gguf");
        assert!(
            url.starts_with("https://huggingface.co/Qwen/Qwen2.5-7B-Instruct-GGUF/resolve/main/")
        );
        assert!(url.contains("Q4_K_M/"));
        assert!(url.contains("model%20q4.gguf"));
    }

    #[test]
    fn local_path_nests_repo() {
        let path = local_model_path("bartowski/Llama-3.1-8B-Instruct-GGUF", "model.gguf");
        assert!(path.ends_with("bartowski/Llama-3.1-8B-Instruct-GGUF/model.gguf"));
    }
}
