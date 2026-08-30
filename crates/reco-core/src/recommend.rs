use crate::format_gib;
use crate::hardware::{AccelBackend, HardwareProfile};
use crate::model::{Catalog, GgufFile, ModelEntry, Recommendation, Scores};

const MIN_COMPAT: f32 = 28.0;

/// Rank GGUF files for this machine. Best quant per repo, then top `limit`.
pub fn recommend(profile: &HardwareProfile, catalog: &Catalog, limit: usize) -> Vec<Recommendation> {
    let budget = memory_budget_bytes(profile);
    let on_gpu = profile.primary_backend() != AccelBackend::Cpu;
    let max_downloads = catalog
        .models
        .iter()
        .map(|m| m.downloads)
        .max()
        .unwrap_or(1)
        .max(1);
    let max_likes = catalog.models.iter().map(|m| m.likes).max().unwrap_or(1).max(1);

    let mut best_by_repo: Vec<Recommendation> = Vec::new();

    for model in &catalog.models {
        if model.is_embedding_like() || model.files.is_empty() {
            continue;
        }
        let mut best: Option<Recommendation> = None;
        for file in &model.files {
            if file.sharded
                || file.quant.is_too_heavy_for_chat()
                || file.quant.is_too_low_for_chat()
            {
                continue;
            }
            let rec = score_one(profile, model, file, budget, on_gpu, max_downloads, max_likes);
            if rec.scores.compatibility < MIN_COMPAT {
                continue;
            }
            if best
                .as_ref()
                .is_none_or(|current| rec.total > current.total)
            {
                best = Some(rec);
            }
        }
        if let Some(rec) = best {
            best_by_repo.push(rec);
        }
    }

    best_by_repo.sort_by(|a, b| {
        b.total
            .partial_cmp(&a.total)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    best_by_repo.truncate(limit.max(1));
    best_by_repo
}

pub fn memory_budget_bytes(profile: &HardwareProfile) -> u64 {
    if let Some(gpu) = profile.gpus.iter().find(|g| g.backend != AccelBackend::Cpu) {
        if let Some(vram) = gpu.vram_bytes {
            if gpu.vendor == crate::hardware::GpuVendor::Apple {
                return (vram as f64 * 0.42) as u64;
            }
            return (vram as f64 * 0.85) as u64;
        }
    }
    if profile.os.name.to_ascii_lowercase().contains("macos")
        || profile.os.arch.contains("arm")
            && profile
                .cpu
                .name
                .to_ascii_lowercase()
                .contains("apple")
    {
        return (profile.memory.total_bytes as f64 * 0.42) as u64;
    }
    let available = profile.memory.available_bytes.max(profile.memory.total_bytes / 2);
    (available as f64 * 0.55) as u64
}

fn score_one(
    profile: &HardwareProfile,
    model: &ModelEntry,
    file: &GgufFile,
    budget: u64,
    on_gpu: bool,
    max_downloads: u64,
    max_likes: u64,
) -> Recommendation {
    let compatibility = compatibility_score(file.size_bytes, budget);
    let speed = speed_score(model, file, on_gpu, profile);
    let quality = quality_score(model, file);
    let popularity = popularity_score(model.downloads, model.likes, max_downloads, max_likes);
    let scores = Scores {
        compatibility,
        speed,
        quality,
        popularity,
    };
    let total = scores.weighted_total();
    Recommendation {
        repo_id: model.repo_id.clone(),
        filename: file.filename.clone(),
        quant: file.quant.clone(),
        size_bytes: file.size_bytes,
        size_estimated: file.size_estimated,
        params: model.params,
        downloads: model.downloads,
        scores,
        total,
        why: why(profile, file, budget, &scores, on_gpu),
    }
}

fn compatibility_score(size: u64, budget: u64) -> f32 {
    if budget == 0 {
        return 0.0;
    }
    let ratio = size as f64 / budget as f64;
    if ratio <= 0.20 {
        62.0
    } else if ratio <= 0.40 {
        78.0
    } else if ratio <= 0.78 {
        100.0
    } else if ratio <= 0.92 {
        88.0
    } else if ratio <= 1.00 {
        70.0
    } else if ratio <= 1.12 {
        42.0
    } else if ratio <= 1.35 {
        22.0
    } else {
        8.0
    }
}

fn speed_score(model: &ModelEntry, file: &GgufFile, on_gpu: bool, profile: &HardwareProfile) -> f32 {
    let params = model
        .params
        .map(|p| p.effective_billions())
        .unwrap_or(7.0);
    let size_part = if on_gpu {
        if params <= 4.0 {
            96.0
        } else if params <= 9.0 {
            88.0
        } else if params <= 14.0 {
            68.0
        } else if params <= 32.0 {
            42.0
        } else {
            22.0
        }
    } else {
        let cores = profile.cpu.logical_cores.max(1) as f32;
        let core_bonus = (cores / 16.0).min(1.0) * 8.0;
        let base = if params <= 1.5 {
            88.0
        } else if params <= 4.0 {
            72.0
        } else if params <= 8.0 {
            46.0
        } else {
            18.0
        };
        base + core_bonus
    };
    (size_part * 0.65 + file.quant.speed_score() * 0.35).min(100.0)
}

fn quality_score(model: &ModelEntry, file: &GgufFile) -> f32 {
    let params = model.params.map(|p| p.total_billions).unwrap_or(7.0);
    let param_score = if params <= 1.0 {
        38.0
    } else if params <= 2.0 {
        48.0
    } else if params <= 4.0 {
        58.0
    } else if params <= 8.0 {
        72.0
    } else if params <= 10.0 {
        78.0
    } else if params <= 15.0 {
        84.0
    } else if params <= 35.0 {
        90.0
    } else {
        95.0
    };
    let mut score = param_score * 0.58 + file.quant.quality_score() * 0.42;
    if model.is_conversational() {
        score += 4.0;
    }
    score.min(100.0)
}

fn popularity_score(downloads: u64, likes: u64, max_downloads: u64, max_likes: u64) -> f32 {
    let d = log_norm(downloads, max_downloads) * 80.0;
    let l = log_norm(likes, max_likes) * 20.0;
    (d + l).min(100.0)
}

fn log_norm(value: u64, max: u64) -> f32 {
    let v = (value as f64 + 1.0).log10();
    let m = (max as f64 + 1.0).log10().max(0.001);
    ((v / m) as f32).clamp(0.0, 1.0)
}

fn why(
    profile: &HardwareProfile,
    file: &GgufFile,
    budget: u64,
    scores: &Scores,
    on_gpu: bool,
) -> String {
    let size = format_gib(file.size_bytes);
    let room = format_gib(budget);
    let target = if on_gpu {
        if let Some(gpu) = profile.gpus.first() {
            format!("cabe en {} ({})", gpu.name, room)
        } else {
            format!("cabe en {room} de presupuesto")
        }
    } else {
        format!("cabe en {room} de RAM")
    };

    let mut bits = Vec::new();
    bits.push(if file.size_estimated {
        format!("{size} est.")
    } else {
        size
    });

    if scores.compatibility >= 85.0 {
        bits.push(target);
    } else if scores.compatibility >= 50.0 {
        bits.push(format!("justo para {room}"));
    }

    let mut top = [
        ("rápido", scores.speed),
        ("calidad", scores.quality),
        ("popular", scores.popularity),
    ];
    top.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    if top[0].1 >= 70.0 {
        bits.push(top[0].0.to_string());
    }
    bits.join(" · ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hardware::fixtures::{apple_unified, cpu_only, rtx_4060};
    use crate::model::{CatalogSource, GgufFile, ModelEntry, ModelParams};
    use crate::quant::GgufQuant;

    fn catalog() -> Catalog {
        Catalog {
            version: 1,
            fetched_at: None,
            source: CatalogSource::Seed,
            models: vec![
                model(
                    "Qwen/Qwen2.5-7B-Instruct-GGUF",
                    7.0,
                    None,
                    2_000_000,
                    400,
                    4_700_000_000,
                    GgufQuant::Q4Km,
                    "qwen-7b-q4_k_m.gguf",
                ),
                model(
                    "meta/Llama-3.1-70B-Instruct-GGUF",
                    70.0,
                    None,
                    5_000_000,
                    900,
                    40_000_000_000,
                    GgufQuant::Q4Km,
                    "llama-70b-q4_k_m.gguf",
                ),
                model(
                    "unsloth/Llama-3.2-3B-Instruct-GGUF",
                    3.0,
                    None,
                    800_000,
                    120,
                    2_000_000_000,
                    GgufQuant::Q4Km,
                    "llama-3b-q4_k_m.gguf",
                ),
                model(
                    "Qwen/Qwen2.5-1.5B-Instruct-GGUF",
                    1.5,
                    None,
                    400_000,
                    80,
                    1_100_000_000,
                    GgufQuant::Q4Km,
                    "qwen-1.5b-q4_k_m.gguf",
                ),
            ],
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn model(
        repo: &str,
        billions: f64,
        active: Option<f64>,
        downloads: u64,
        likes: u64,
        size: u64,
        quant: GgufQuant,
        file: &str,
    ) -> ModelEntry {
        ModelEntry {
            repo_id: repo.into(),
            downloads,
            likes,
            tags: vec!["conversational".into(), "text-generation".into()],
            pipeline_tag: Some("text-generation".into()),
            params: Some(ModelParams {
                total_billions: billions,
                active_billions: active,
            }),
            files: vec![GgufFile {
                filename: file.into(),
                quant,
                size_bytes: size,
                size_estimated: true,
                sharded: false,
            }],
        }
    }

    #[test]
    fn rtx_4060_prefers_7b_over_70b() {
        let recs = recommend(&rtx_4060(), &catalog(), 8);
        assert!(!recs.is_empty());
        assert!(
            recs[0].repo_id.contains("7B"),
            "top should be 7B, got {}",
            recs[0].repo_id
        );
        assert!(
            recs.iter().all(|r| !r.repo_id.contains("70B")),
            "70B must not fit an 8 GB card"
        );
    }

    #[test]
    fn cpu_small_ram_prefers_tiny_models() {
        let recs = recommend(&cpu_only(), &catalog(), 4);
        assert!(!recs.is_empty());
        assert!(
            recs[0].repo_id.contains("1.5B") || recs[0].repo_id.contains("3B"),
            "CPU 8 GB should pick a small model, got {}",
            recs[0].repo_id
        );
    }

    #[test]
    fn apple_unified_likes_7b() {
        let recs = recommend(&apple_unified(), &catalog(), 4);
        assert!(recs.iter().any(|r| r.repo_id.contains("7B")));
    }

    #[test]
    fn skips_iq1_even_if_it_fits() {
        let mut cat = catalog();
        cat.models.push(model(
            "unsloth/Huge-30B-A3B-GGUF",
            30.0,
            Some(3.0),
            12_000_000,
            900,
            5_800_000_000,
            GgufQuant::Iq1S,
            "huge-iq1_s.gguf",
        ));
        let recs = recommend(&rtx_4060(), &cat, 8);
        assert!(recs.iter().all(|r| r.quant != GgufQuant::Iq1S));
    }

    #[test]
    fn weights_sum_to_one() {
        let sum =
            Scores::WEIGHT_COMPAT + Scores::WEIGHT_SPEED + Scores::WEIGHT_QUALITY + Scores::WEIGHT_POPULARITY;
        assert!((sum - 1.0).abs() < f32::EPSILON);
    }
}
