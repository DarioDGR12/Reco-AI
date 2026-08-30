use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::cache::cache_root;
use crate::CatalogError;

const USER_AGENT: &str = "Reco-AI/0.2 (+https://github.com/DarioDGR12/Reco-AI)";
const CHUNK: usize = 64 * 1024;

#[derive(Debug, Clone)]
pub struct DownloadedModel {
    pub repo_id: String,
    pub filename: String,
    pub size_bytes: u64,
    pub path: PathBuf,
}

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
    let part = dest.with_extension("gguf.part");
    let existing = dest_len(&part);
    let mut request = agent.get(&url).set("User-Agent", USER_AGENT);
    if existing > 0 {
        request = request.set("Range", &format!("bytes={existing}-"));
    }
    let response = request
        .call()
        .map_err(|err| CatalogError::Network(err.to_string()))?;

    let status = response.status();
    let resume = status == 206 && existing > 0;
    let chunk_len = response
        .header("Content-Length")
        .and_then(|v| v.parse::<u64>().ok());
    let total = if resume {
        chunk_len.map(|n| existing + n)
    } else {
        chunk_len
    };
    let mut reader = response.into_reader();
    let mut file = if resume {
        OpenOptions::new()
            .append(true)
            .open(&part)
            .map_err(|err| CatalogError::Cache(err.to_string()))?
    } else {
        File::create(&part).map_err(|err| CatalogError::Cache(err.to_string()))?
    };
    let mut buf = vec![0_u8; CHUNK];
    let mut written = if resume { existing } else { 0 };

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

pub fn list_downloaded() -> Vec<DownloadedModel> {
    scan_models_dir(&models_dir())
}

pub fn scan_models_dir(dir: &Path) -> Vec<DownloadedModel> {
    let mut out = Vec::new();
    let Ok(orgs) = fs::read_dir(dir) else {
        return out;
    };
    for org in orgs.flatten() {
        if !org.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        let org_name = org.file_name();
        let Ok(repos) = fs::read_dir(org.path()) else {
            continue;
        };
        for repo in repos.flatten() {
            if !repo.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                continue;
            }
            let repo_id = format!(
                "{}/{}",
                org_name.to_string_lossy(),
                repo.file_name().to_string_lossy()
            );
            let Ok(files) = fs::read_dir(repo.path()) else {
                continue;
            };
            for file in files.flatten() {
                let name = file.file_name();
                let name = name.to_string_lossy();
                if !name.to_ascii_lowercase().ends_with(".gguf") || name.ends_with(".part") {
                    continue;
                }
                let path = file.path();
                let size = dest_len(&path);
                if size == 0 {
                    continue;
                }
                out.push(DownloadedModel {
                    repo_id: repo_id.clone(),
                    filename: name.into_owned(),
                    size_bytes: size,
                    path,
                });
            }
        }
    }
    out.sort_by(|a, b| a.repo_id.cmp(&b.repo_id).then(a.filename.cmp(&b.filename)));
    out
}

pub fn remove_downloaded(repo_id: &str, filename: Option<&str>) -> Result<u64, CatalogError> {
    let mut freed = 0_u64;
    if let Some(name) = filename {
        let path = local_model_path(repo_id, name);
        freed += dest_len(&path);
        if path.exists() {
            fs::remove_file(&path).map_err(|err| CatalogError::Cache(err.to_string()))?;
        }
        return Ok(freed);
    }
    let dir = models_dir().join(repo_id);
    if dir.is_dir() {
        freed += dir_size(&dir);
        fs::remove_dir_all(&dir).map_err(|err| CatalogError::Cache(err.to_string()))?;
    }
    Ok(freed)
}

pub fn dir_size(path: &Path) -> u64 {
    let mut total = 0;
    let Ok(entries) = fs::read_dir(path) else {
        return dest_len(path);
    };
    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_dir() {
            total += dir_size(&p);
        } else {
            total += dest_len(&p);
        }
    }
    total
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

    #[test]
    fn scan_nested_gguf() {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("reco-scan-{stamp}"));
        let dest = root
            .join("bartowski")
            .join("Llama-3.1-8B-Instruct-GGUF");
        std::fs::create_dir_all(&dest).unwrap();
        std::fs::write(dest.join("model.gguf"), b"gguf").unwrap();
        std::fs::write(dest.join("model.gguf.part"), b"no").unwrap();
        let found = scan_models_dir(&root);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].repo_id, "bartowski/Llama-3.1-8B-Instruct-GGUF");
        let _ = std::fs::remove_dir_all(root);
    }
}
