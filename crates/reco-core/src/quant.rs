use serde::{Deserialize, Serialize};

/// GGUF quantization parsed from a filename or path.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GgufQuant {
    Iq1S,
    Iq2Xxs,
    Iq2Xs,
    Iq2M,
    Iq3Xxs,
    Iq3Xs,
    Iq3M,
    Iq4Xs,
    Iq4Nl,
    Q2K,
    Q3Ks,
    Q3Km,
    Q3Kl,
    Q3K,
    Q40,
    Q41,
    Q4Ks,
    Q4Km,
    Q4K,
    Q50,
    Q51,
    Q5Ks,
    Q5Km,
    Q5K,
    Q6K,
    Q80,
    Bf16,
    F16,
    F32,
    Other(String),
}

impl GgufQuant {
    /// Parse the strongest (longest) quant token in a path.
    pub fn parse(path: &str) -> Option<Self> {
        let name = path.rsplit('/').next().unwrap_or(path);
        let lower = name.to_ascii_lowercase();
        if should_skip_filename(&lower) {
            return None;
        }
        for (needle, quant) in QUANT_TABLE {
            if lower.contains(needle) {
                return Some(quant.clone());
            }
        }
        if lower.ends_with(".gguf") {
            Some(Self::Other(name.to_string()))
        } else {
            None
        }
    }

    pub fn label(&self) -> String {
        match self {
            Self::Iq1S => "IQ1_S".into(),
            Self::Iq2Xxs => "IQ2_XXS".into(),
            Self::Iq2Xs => "IQ2_XS".into(),
            Self::Iq2M => "IQ2_M".into(),
            Self::Iq3Xxs => "IQ3_XXS".into(),
            Self::Iq3Xs => "IQ3_XS".into(),
            Self::Iq3M => "IQ3_M".into(),
            Self::Iq4Xs => "IQ4_XS".into(),
            Self::Iq4Nl => "IQ4_NL".into(),
            Self::Q2K => "Q2_K".into(),
            Self::Q3Ks => "Q3_K_S".into(),
            Self::Q3Km => "Q3_K_M".into(),
            Self::Q3Kl => "Q3_K_L".into(),
            Self::Q3K => "Q3_K".into(),
            Self::Q40 => "Q4_0".into(),
            Self::Q41 => "Q4_1".into(),
            Self::Q4Ks => "Q4_K_S".into(),
            Self::Q4Km => "Q4_K_M".into(),
            Self::Q4K => "Q4_K".into(),
            Self::Q50 => "Q5_0".into(),
            Self::Q51 => "Q5_1".into(),
            Self::Q5Ks => "Q5_K_S".into(),
            Self::Q5Km => "Q5_K_M".into(),
            Self::Q5K => "Q5_K".into(),
            Self::Q6K => "Q6_K".into(),
            Self::Q80 => "Q8_0".into(),
            Self::Bf16 => "BF16".into(),
            Self::F16 => "F16".into(),
            Self::F32 => "F32".into(),
            Self::Other(name) => name.clone(),
        }
    }

    /// Approximate bytes per parameter (weights only).
    pub fn bytes_per_param(&self) -> f64 {
        match self {
            Self::Iq1S => 0.20,
            Self::Iq2Xxs | Self::Iq2Xs => 0.28,
            Self::Iq2M => 0.32,
            Self::Iq3Xxs | Self::Iq3Xs => 0.38,
            Self::Iq3M | Self::Q2K => 0.36,
            Self::Q3Ks => 0.40,
            Self::Q3K | Self::Q3Km => 0.44,
            Self::Q3Kl => 0.49,
            Self::Q40 | Self::Q4Ks => 0.53,
            Self::Q4K | Self::Q4Km | Self::Iq4Xs | Self::Iq4Nl => 0.57,
            Self::Q41 => 0.60,
            Self::Q50 | Self::Q5Ks => 0.66,
            Self::Q5K | Self::Q5Km => 0.69,
            Self::Q51 => 0.71,
            Self::Q6K => 0.78,
            Self::Q80 => 1.06,
            Self::Bf16 | Self::F16 => 2.0,
            Self::F32 => 4.0,
            Self::Other(_) => 0.57,
        }
    }

    pub fn quality_score(&self) -> f32 {
        match self {
            Self::F32 => 100.0,
            Self::F16 | Self::Bf16 => 97.0,
            Self::Q80 => 93.0,
            Self::Q6K => 88.0,
            Self::Q51 | Self::Q5Km | Self::Q5K => 84.0,
            Self::Q50 | Self::Q5Ks => 80.0,
            Self::Q4Km | Self::Q4K | Self::Iq4Nl => 76.0,
            Self::Q41 | Self::Q4Ks | Self::Iq4Xs => 72.0,
            Self::Q40 => 68.0,
            Self::Q3Kl => 62.0,
            Self::Q3Km | Self::Q3K => 58.0,
            Self::Q3Ks | Self::Iq3M => 52.0,
            Self::Q2K | Self::Iq3Xs | Self::Iq3Xxs => 44.0,
            Self::Iq2M | Self::Iq2Xs | Self::Iq2Xxs => 36.0,
            Self::Iq1S => 24.0,
            Self::Other(_) => 60.0,
        }
    }

    /// Higher = faster decode (smaller weights).
    pub fn speed_score(&self) -> f32 {
        match self {
            Self::Iq1S | Self::Iq2Xxs | Self::Iq2Xs => 96.0,
            Self::Iq2M | Self::Q2K => 92.0,
            Self::Iq3Xxs | Self::Iq3Xs | Self::Q3Ks => 88.0,
            Self::Q3Km | Self::Q3K | Self::Iq3M => 84.0,
            Self::Q3Kl | Self::Q40 | Self::Q4Ks => 80.0,
            Self::Q4Km | Self::Q4K | Self::Iq4Xs | Self::Iq4Nl => 76.0,
            Self::Q41 | Self::Q50 | Self::Q5Ks => 68.0,
            Self::Q5Km | Self::Q5K | Self::Q51 => 62.0,
            Self::Q6K => 54.0,
            Self::Q80 => 42.0,
            Self::Bf16 | Self::F16 => 28.0,
            Self::F32 => 12.0,
            Self::Other(_) => 70.0,
        }
    }

    pub fn is_too_heavy_for_chat(&self) -> bool {
        matches!(self, Self::F32)
    }

    /// IQ1/IQ2 and similar are a last resort — they fit, but chat quality collapses.
    pub fn is_too_low_for_chat(&self) -> bool {
        self.quality_score() < 52.0
    }
}

/// Longest-first so `q4_k_m` wins over `q4_k` and `q4_0`.
const QUANT_TABLE: &[(&str, GgufQuant)] = &[
    ("iq4_nl", GgufQuant::Iq4Nl),
    ("iq4_xs", GgufQuant::Iq4Xs),
    ("iq3_xxs", GgufQuant::Iq3Xxs),
    ("iq3_xs", GgufQuant::Iq3Xs),
    ("iq3_m", GgufQuant::Iq3M),
    ("iq2_xxs", GgufQuant::Iq2Xxs),
    ("iq2_xs", GgufQuant::Iq2Xs),
    ("iq2_m", GgufQuant::Iq2M),
    ("iq1_s", GgufQuant::Iq1S),
    ("q4_k_m", GgufQuant::Q4Km),
    ("q5_k_m", GgufQuant::Q5Km),
    ("q3_k_m", GgufQuant::Q3Km),
    ("q3_k_l", GgufQuant::Q3Kl),
    ("q4_k_s", GgufQuant::Q4Ks),
    ("q5_k_s", GgufQuant::Q5Ks),
    ("q3_k_s", GgufQuant::Q3Ks),
    ("q6_k", GgufQuant::Q6K),
    ("q3_k", GgufQuant::Q3K),
    ("q4_k", GgufQuant::Q4K),
    ("q5_k", GgufQuant::Q5K),
    ("q2_k", GgufQuant::Q2K),
    ("q8_0", GgufQuant::Q80),
    ("q5_1", GgufQuant::Q51),
    ("q5_0", GgufQuant::Q50),
    ("q4_1", GgufQuant::Q41),
    ("q4_0", GgufQuant::Q40),
    ("bf16", GgufQuant::Bf16),
    ("fp16", GgufQuant::F16),
    ("f16", GgufQuant::F16),
    ("fp32", GgufQuant::F32),
    ("f32", GgufQuant::F32),
];

pub fn should_skip_filename(lower: &str) -> bool {
    if !lower.ends_with(".gguf") {
        return true;
    }
    lower.contains("mmproj")
        || lower.contains("mm_proj")
        || lower.contains("projector")
        || lower.contains("encoder")
        || lower.contains("imatrix")
        || lower.contains("importance")
}

pub fn is_sharded(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower.contains("-of-") || lower.contains("_of_")
}

/// Parse `7B`, `3.8B`, `30B-A3B` from a repo id or filename.
pub fn parse_param_count(text: &str) -> Option<crate::model::ModelParams> {
    let upper = text.to_ascii_uppercase();
    let bytes = upper.as_bytes();
    let mut best: Option<crate::model::ModelParams> = None;

    let mut i = 0;
    while i < bytes.len() {
        if bytes[i].is_ascii_digit() {
            let start = i;
            while i < bytes.len() && (bytes[i].is_ascii_digit() || bytes[i] == b'.') {
                i += 1;
            }
            let num = match std::str::from_utf8(&bytes[start..i]) {
                Ok(n) => n,
                Err(_) => continue,
            };
            if num.is_empty() || num == "." {
                continue;
            }
            let value: f64 = match num.parse() {
                Ok(v) => v,
                Err(_) => continue,
            };
            if i < bytes.len() && bytes[i] == b'B' {
                let mut active = None;
                let after = i + 1;
                if after + 1 < bytes.len() && bytes[after] == b'-' && bytes[after + 1] == b'A' {
                    let mut j = after + 2;
                    let a0 = j;
                    while j < bytes.len() && (bytes[j].is_ascii_digit() || bytes[j] == b'.') {
                        j += 1;
                    }
                    if j > a0 && j < bytes.len() && bytes[j] == b'B' {
                        if let Ok(a) = std::str::from_utf8(&bytes[a0..j]).unwrap_or("").parse() {
                            active = Some(a);
                        }
                    }
                }
                if (0.1..=1000.0).contains(&value) {
                    let candidate = crate::model::ModelParams {
                        total_billions: value,
                        active_billions: active,
                    };
                    best = Some(match best {
                        Some(prev) if prev.total_billions >= candidate.total_billions => prev,
                        _ => candidate,
                    });
                }
                i += 1;
                continue;
            }
            if i < bytes.len() && bytes[i] == b'M' && (100.0..=20000.0).contains(&value) {
                let candidate = crate::model::ModelParams {
                    total_billions: value / 1000.0,
                    active_billions: None,
                };
                best = Some(match best {
                    Some(prev) if prev.total_billions >= candidate.total_billions => prev,
                    _ => candidate,
                });
            }
        } else {
            i += 1;
        }
    }
    best
}

pub fn estimate_size_bytes(params: crate::model::ModelParams, quant: &GgufQuant) -> u64 {
    let weights = params.total_billions * 1_000_000_000.0 * quant.bytes_per_param();
    let overhead = 192.0 * 1024.0 * 1024.0;
    (weights + overhead) as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn longest_quant_wins() {
        assert_eq!(
            GgufQuant::parse("Qwen/Q4_K_M/model-Q4_K_M.gguf"),
            Some(GgufQuant::Q4Km)
        );
        assert_eq!(
            GgufQuant::parse("foo-q4_0.gguf"),
            Some(GgufQuant::Q40)
        );
        assert_eq!(
            GgufQuant::parse("bar-iq4_xs.gguf"),
            Some(GgufQuant::Iq4Xs)
        );
        assert!(GgufQuant::parse("llama-mmproj-f16.gguf").is_none());
        assert!(is_sharded("q4_k_m-00001-of-00002.gguf"));
    }

    #[test]
    fn parse_moe_and_millions() {
        let moe = parse_param_count("unsloth/Qwen3-Coder-30B-A3B-Instruct-GGUF").unwrap();
        assert_eq!(moe.total_billions, 30.0);
        assert_eq!(moe.active_billions, Some(3.0));
        let small = parse_param_count("org/phi-3.5-mini-3.8B-GGUF").unwrap();
        assert!((small.total_billions - 3.8).abs() < 0.01);
        let millions = parse_param_count("tiny-360M-GGUF").unwrap();
        assert!((millions.total_billions - 0.36).abs() < 0.001);
    }
}
