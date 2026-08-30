use serde::{Deserialize, Serialize};

use crate::paths::{config_dir, config_path};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct RecoConfig {
    #[serde(default)]
    pub byok: ByokConfig,
    #[serde(default)]
    pub llama: LlamaConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ByokConfig {
    #[serde(default)]
    pub openai_key: String,
    #[serde(default = "default_openai_base")]
    pub openai_base: String,
    #[serde(default = "default_openai_model")]
    pub openai_model: String,
    #[serde(default)]
    pub anthropic_key: String,
    #[serde(default = "default_anthropic_model")]
    pub anthropic_model: String,
}

impl Default for ByokConfig {
    fn default() -> Self {
        Self {
            openai_key: String::new(),
            openai_base: default_openai_base(),
            openai_model: default_openai_model(),
            anthropic_key: String::new(),
            anthropic_model: default_anthropic_model(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LlamaConfig {
    #[serde(default)]
    pub cli: Option<String>,
    #[serde(default = "default_n_predict")]
    pub n_predict: u32,
    #[serde(default = "default_n_ctx")]
    pub n_ctx: u32,
    #[serde(default = "default_n_gpu_layers")]
    pub n_gpu_layers: i32,
}

impl Default for LlamaConfig {
    fn default() -> Self {
        Self {
            cli: None,
            n_predict: default_n_predict(),
            n_ctx: default_n_ctx(),
            n_gpu_layers: default_n_gpu_layers(),
        }
    }
}

fn default_openai_base() -> String {
    "https://api.openai.com/v1".into()
}
fn default_openai_model() -> String {
    "gpt-4o-mini".into()
}
fn default_anthropic_model() -> String {
    "claude-sonnet-4-20250514".into()
}
fn default_n_predict() -> u32 {
    256
}
fn default_n_ctx() -> u32 {
    2048
}
fn default_n_gpu_layers() -> i32 {
    99
}

impl RecoConfig {
    pub fn load() -> Self {
        let path = config_path();
        let Ok(raw) = std::fs::read_to_string(&path) else {
            return Self::default().with_env();
        };
        serde_json::from_str::<Self>(&raw)
            .unwrap_or_default()
            .with_env()
    }

    pub fn save(&self) -> Result<(), String> {
        let dir = config_dir();
        std::fs::create_dir_all(&dir).map_err(|err| err.to_string())?;
        let json = serde_json::to_string_pretty(self).map_err(|err| err.to_string())?;
        std::fs::write(config_path(), json).map_err(|err| err.to_string())
    }

    pub fn with_env(mut self) -> Self {
        if let Ok(key) = std::env::var("OPENAI_API_KEY") {
            if !key.is_empty() {
                self.byok.openai_key = key;
            }
        }
        if let Ok(key) = std::env::var("ANTHROPIC_API_KEY") {
            if !key.is_empty() {
                self.byok.anthropic_key = key;
            }
        }
        if let Ok(base) = std::env::var("OPENAI_BASE_URL") {
            if !base.is_empty() {
                self.byok.openai_base = base;
            }
        }
        if let Ok(cli) = std::env::var("RECO_LLAMA_CLI") {
            if !cli.is_empty() {
                self.llama.cli = Some(cli);
            }
        }
        self
    }

    pub fn set(&mut self, key: &str, value: &str) -> Result<(), String> {
        match key {
            "openai-key" | "openai_key" => self.byok.openai_key = value.into(),
            "openai-base" | "openai_base" => self.byok.openai_base = value.into(),
            "openai-model" | "openai_model" => self.byok.openai_model = value.into(),
            "anthropic-key" | "anthropic_key" => self.byok.anthropic_key = value.into(),
            "anthropic-model" | "anthropic_model" => self.byok.anthropic_model = value.into(),
            "llama-cli" | "llama_cli" => self.llama.cli = Some(value.into()),
            "n-predict" | "n_predict" => {
                self.llama.n_predict = value.parse().map_err(|_| "n-predict debe ser un número")?;
            }
            "n-ctx" | "n_ctx" => {
                self.llama.n_ctx = value.parse().map_err(|_| "n-ctx debe ser un número")?;
            }
            "n-gpu-layers" | "n_gpu_layers" => {
                self.llama.n_gpu_layers = value
                    .parse()
                    .map_err(|_| "n-gpu-layers debe ser un número")?;
            }
            other => return Err(format!("clave desconocida: {other}")),
        }
        Ok(())
    }

    pub fn masked(&self) -> Self {
        let mut copy = self.clone();
        copy.byok.openai_key = mask(&self.byok.openai_key);
        copy.byok.anthropic_key = mask(&self.byok.anthropic_key);
        copy
    }
}

fn mask(key: &str) -> String {
    if key.is_empty() {
        return String::new();
    }
    if key.len() <= 8 {
        return "****".into();
    }
    format!("{}…{}", &key[..4], &key[key.len() - 4..])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_and_mask_keys() {
        let mut cfg = RecoConfig::default();
        cfg.set("openai-key", "sk-abcdefghijklmnopqrstuvwxyz").unwrap();
        let masked = cfg.masked();
        assert!(masked.byok.openai_key.starts_with("sk-a"));
        assert!(!masked.byok.openai_key.contains("mnop"));
        assert!(cfg.set("nope", "x").is_err());
    }
}
