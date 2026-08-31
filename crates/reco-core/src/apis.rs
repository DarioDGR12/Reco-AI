//! Named, key-scoped APIs so other apps talk to models on this machine.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::config::mask_secret;
use crate::paths::{apis_path, clients_dir, config_dir};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApiEndpoint {
    pub slug: String,
    pub name: String,
    pub repo_id: String,
    pub filename: String,
    pub quant: String,
    pub api_key: String,
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_provider")]
    pub provider: String,
    #[serde(default)]
    pub lan: bool,
    pub created_at: i64,
}

fn default_host() -> String {
    "127.0.0.1".into()
}
fn default_port() -> u16 {
    11434
}
fn default_provider() -> String {
    "auto".into()
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApiRegistry {
    #[serde(default)]
    pub endpoints: Vec<ApiEndpoint>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientKind {
    Curl,
    Python,
    Javascript,
    Continue,
    Cursor,
    OpenWebui,
    Langchain,
    Env,
    OpenApi,
}

impl ClientKind {
    pub fn parse(s: &str) -> Result<Self, String> {
        match s {
            "curl" => Ok(Self::Curl),
            "python" | "py" => Ok(Self::Python),
            "js" | "javascript" | "node" => Ok(Self::Javascript),
            "continue" => Ok(Self::Continue),
            "cursor" => Ok(Self::Cursor),
            "openwebui" | "webui" => Ok(Self::OpenWebui),
            "langchain" | "lc" => Ok(Self::Langchain),
            "env" => Ok(Self::Env),
            "openapi" => Ok(Self::OpenApi),
            other => Err(format!(
                "cliente desconocido: {other} (curl|python|js|continue|cursor|openwebui|langchain|env|openapi)"
            )),
        }
    }

    pub fn all() -> &'static [Self] {
        &[
            Self::Curl,
            Self::Python,
            Self::Javascript,
            Self::Continue,
            Self::Cursor,
            Self::OpenWebui,
            Self::Langchain,
            Self::Env,
            Self::OpenApi,
        ]
    }

    pub fn filename(self) -> &'static str {
        match self {
            Self::Curl => "curl.sh",
            Self::Python => "openai.py",
            Self::Javascript => "openai.mjs",
            Self::Continue => "continue.json",
            Self::Cursor => "cursor.md",
            Self::OpenWebui => "openwebui.env",
            Self::Langchain => "langchain.py",
            Self::Env => ".env",
            Self::OpenApi => "openapi.json",
        }
    }
}

impl ApiRegistry {
    pub fn load() -> Self {
        let Ok(raw) = std::fs::read_to_string(apis_path()) else {
            return Self::default();
        };
        serde_json::from_str(&raw).unwrap_or_default()
    }

    pub fn save(&self) -> Result<(), String> {
        std::fs::create_dir_all(config_dir()).map_err(|err| err.to_string())?;
        let json = serde_json::to_string_pretty(self).map_err(|err| err.to_string())?;
        std::fs::write(apis_path(), json).map_err(|err| err.to_string())
    }

    pub fn get(&self, slug: &str) -> Option<&ApiEndpoint> {
        self.endpoints
            .iter()
            .find(|e| e.slug == slug || e.name.eq_ignore_ascii_case(slug))
    }

    pub fn get_mut(&mut self, slug: &str) -> Option<&mut ApiEndpoint> {
        self.endpoints
            .iter_mut()
            .find(|e| e.slug == slug || e.name.eq_ignore_ascii_case(slug))
    }

    pub fn by_key(&self, key: &str) -> Option<&ApiEndpoint> {
        self.endpoints.iter().find(|e| e.api_key == key)
    }

    pub fn remove(&mut self, slug: &str) -> Option<ApiEndpoint> {
        let idx = self
            .endpoints
            .iter()
            .position(|e| e.slug == slug || e.name.eq_ignore_ascii_case(slug))?;
        Some(self.endpoints.remove(idx))
    }

    pub fn next_port(&self) -> u16 {
        let mut port = 11434;
        let used: Vec<u16> = self.endpoints.iter().map(|e| e.port).collect();
        while used.contains(&port) {
            port = port.saturating_add(1);
        }
        port
    }

    #[allow(clippy::too_many_arguments)]
    pub fn create(
        &mut self,
        name: &str,
        repo_id: &str,
        filename: &str,
        quant: &str,
        provider: &str,
        lan: bool,
        port: Option<u16>,
    ) -> Result<ApiEndpoint, String> {
        let slug = slugify(name);
        if slug.is_empty() {
            return Err("el nombre no puede quedar vacío".into());
        }
        if self.get(&slug).is_some() {
            return Err(format!("ya existe una API '{slug}'"));
        }
        let host = if lan {
            "0.0.0.0".into()
        } else {
            "127.0.0.1".into()
        };
        let ep = ApiEndpoint {
            slug: slug.clone(),
            name: name.trim().to_string(),
            repo_id: repo_id.into(),
            filename: filename.into(),
            quant: quant.into(),
            api_key: generate_api_key(&slug),
            host,
            port: port.unwrap_or_else(|| self.next_port()),
            provider: provider.into(),
            lan,
            created_at: now_secs(),
        };
        self.endpoints.push(ep.clone());
        self.save()?;
        Ok(ep)
    }
}

impl ApiEndpoint {
    pub fn base_url(&self) -> String {
        advertised_base(&self.host, self.port)
    }

    pub fn model_ids(&self) -> Vec<String> {
        vec![self.repo_id.clone(), self.slug.clone()]
    }

    pub fn allows_model(&self, model: &str) -> bool {
        if model.is_empty() || model == "default" {
            return true;
        }
        self.repo_id.eq_ignore_ascii_case(model)
            || self.slug.eq_ignore_ascii_case(model)
            || self.filename.eq_ignore_ascii_case(model)
    }

    pub fn masked_key(&self) -> String {
        mask_secret(&self.api_key)
    }

    pub fn rotate_key(&mut self) {
        self.api_key = generate_api_key(&self.slug);
    }
}

pub fn slugify(name: &str) -> String {
    let mut out = String::new();
    let mut dash = false;
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            dash = false;
        } else if !dash && !out.is_empty() {
            out.push('-');
            dash = true;
        }
    }
    out.trim_matches('-').to_string()
}

pub fn generate_api_key(slug: &str) -> String {
    let mut buf = [0_u8; 12];
    let _ = getrandom::getrandom(&mut buf);
    let hex: String = buf.iter().map(|b| format!("{b:02x}")).collect();
    let tag = if slug.is_empty() { "api" } else { slug };
    format!("sk-reco-{tag}-{hex}")
}

pub fn advertised_base(host: &str, port: u16) -> String {
    if host == "0.0.0.0" || host == "::" {
        if let Some(ip) = lan_ip() {
            return format!("http://{ip}:{port}");
        }
        return format!("http://127.0.0.1:{port}");
    }
    format!("http://{host}:{port}")
}

pub fn lan_ip() -> Option<String> {
    use std::net::UdpSocket;
    let sock = UdpSocket::bind("0.0.0.0:0").ok()?;
    sock.connect("1.1.1.1:80").ok()?;
    let ip = sock.local_addr().ok()?.ip();
    if ip.is_loopback() {
        return None;
    }
    Some(ip.to_string())
}

pub fn generate_client(ep: &ApiEndpoint, kind: ClientKind, base: &str) -> String {
    let model = &ep.repo_id;
    let key = &ep.api_key;
    let v1 = format!("{base}/v1");
    match kind {
        ClientKind::Curl => format!(
            "#!/usr/bin/env bash\ncurl {base}/v1/chat/completions \\\n  -H \"Authorization: Bearer {key}\" \\\n  -H \"Content-Type: application/json\" \\\n  -d '{{\"model\":\"{model}\",\"messages\":[{{\"role\":\"user\",\"content\":\"hola\"}}]}}'\n"
        ),
        ClientKind::Python => format!(
            "from openai import OpenAI\n\nclient = OpenAI(base_url=\"{v1}\", api_key=\"{key}\")\nr = client.chat.completions.create(\n    model=\"{model}\",\n    messages=[{{ \"role\": \"user\", \"content\": \"hola\" }}],\n)\nprint(r.choices[0].message.content)\n"
        ),
        ClientKind::Javascript => format!(
            "import OpenAI from \"openai\";\n\nconst client = new OpenAI({{\n  baseURL: \"{v1}\",\n  apiKey: \"{key}\",\n}});\nconst r = await client.chat.completions.create({{\n  model: \"{model}\",\n  messages: [{{ role: \"user\", content: \"hola\" }}],\n}});\nconsole.log(r.choices[0].message.content);\n"
        ),
        ClientKind::Continue => serde_json::to_string_pretty(&serde_json::json!({
            "models": [{
                "title": format!("Reco · {}", ep.name),
                "provider": "openai",
                "model": model,
                "apiBase": v1,
                "apiKey": key
            }]
        }))
        .unwrap_or_default(),
        ClientKind::Cursor => format!(
            "En Cursor / cualquier cliente OpenAI-compatible:\n\n  OpenAI Base URL   {v1}\n  OpenAI API Key    {key}\n  Model             {model}\n\nReco corre en esta máquina. La otra app solo necesita esa URL y esa clave.\n"
        ),
        ClientKind::OpenWebui => format!(
            "OPENAI_API_BASE_URL={v1}\nOPENAI_API_KEY={key}\n# En Open WebUI: Settings → Connections → OpenAI, pega base y clave.\n"
        ),
        ClientKind::Langchain => format!(
            "from langchain_openai import ChatOpenAI\n\nllm = ChatOpenAI(\n    base_url=\"{v1}\",\n    api_key=\"{key}\",\n    model=\"{model}\",\n)\nprint(llm.invoke(\"hola\").content)\n"
        ),
        ClientKind::Env => format!(
            "OPENAI_BASE_URL={v1}\nOPENAI_API_KEY={key}\nOPENAI_MODEL={model}\nRECO_API={}\nRECO_SLUG={}\n",
            ep.name, ep.slug
        ),
        ClientKind::OpenApi => openapi_json(ep, base),
    }
}

pub fn openapi_json(ep: &ApiEndpoint, base: &str) -> String {
    serde_json::to_string_pretty(&serde_json::json!({
        "openapi": "3.1.0",
        "info": {
            "title": format!("Reco · {}", ep.name),
            "version": "0.3.0",
            "description": format!(
                "API OpenAI-compatible servida por Reco en esta máquina. Modelo {} ({})",
                ep.repo_id, ep.quant
            )
        },
        "servers": [{ "url": base }],
        "security": [{ "bearer": [] }],
        "components": {
            "securitySchemes": {
                "bearer": { "type": "http", "scheme": "bearer" }
            }
        },
        "paths": {
            "/health": { "get": { "summary": "Liveness" } },
            "/v1/models": { "get": { "summary": "Modelos que abre esta clave" } },
            "/v1/chat/completions": {
                "post": {
                    "summary": "Chat Completions",
                    "requestBody": {
                        "content": {
                            "application/json": {
                                "schema": {
                                    "type": "object",
                                    "properties": {
                                        "model": { "type": "string", "example": ep.repo_id },
                                        "messages": { "type": "array" },
                                        "stream": { "type": "boolean" }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }))
    .unwrap_or_default()
}

pub fn write_client_kit(ep: &ApiEndpoint) -> Result<PathBuf, String> {
    let dir = clients_dir().join(&ep.slug);
    std::fs::create_dir_all(&dir).map_err(|err| err.to_string())?;
    let base = ep.base_url();
    for kind in ClientKind::all() {
        let body = generate_client(ep, *kind, &base);
        std::fs::write(dir.join(kind.filename()), body).map_err(|err| err.to_string())?;
    }
    let readme = format!(
        "# {} — Reco API\n\nTu máquina es el servidor.\n\n- Base: {}\n- Modelo: {}\n- Clave: {}\n\nArranca: `reco api start {}`\n",
        ep.name,
        base,
        ep.repo_id,
        ep.api_key,
        ep.slug
    );
    std::fs::write(dir.join("README.md"), readme).map_err(|err| err.to_string())?;
    Ok(dir)
}

fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> ApiEndpoint {
        ApiEndpoint {
            slug: "mi-app".into(),
            name: "Mi App".into(),
            repo_id: "Qwen/Qwen2.5-7B-Instruct-GGUF".into(),
            filename: "q4.gguf".into(),
            quant: "Q4_K_M".into(),
            api_key: "sk-reco-mi-app-abc123".into(),
            host: "127.0.0.1".into(),
            port: 11434,
            provider: "auto".into(),
            lan: false,
            created_at: 1,
        }
    }

    #[test]
    fn slug_from_name() {
        assert_eq!(slugify("Mi App"), "mi-app");
        assert_eq!(slugify("Continue.dev"), "continue-dev");
        assert_eq!(slugify("///"), "");
    }

    #[test]
    fn key_embeds_slug() {
        let key = generate_api_key("continue");
        assert!(key.starts_with("sk-reco-continue-"));
        assert!(key.len() > 24);
    }

    #[test]
    fn python_snippet_has_base_and_model() {
        let ep = sample();
        let py = generate_client(&ep, ClientKind::Python, "http://127.0.0.1:11434");
        assert!(py.contains("OpenAI"));
        assert!(py.contains("sk-reco-mi-app-abc123"));
        assert!(py.contains("Qwen/Qwen2.5-7B-Instruct-GGUF"));
    }

    #[test]
    fn allows_slug_or_repo() {
        let ep = sample();
        assert!(ep.allows_model("mi-app"));
        assert!(ep.allows_model("Qwen/Qwen2.5-7B-Instruct-GGUF"));
        assert!(!ep.allows_model("otro"));
    }

    #[test]
    fn next_port_skips_used() {
        let mut reg = ApiRegistry::default();
        reg.endpoints.push(sample());
        assert_eq!(reg.next_port(), 11435);
    }

    #[test]
    fn client_kind_parse() {
        assert_eq!(ClientKind::parse("py").unwrap(), ClientKind::Python);
        assert!(ClientKind::parse("nope").is_err());
    }
}
