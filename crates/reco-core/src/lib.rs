//! Shared domain for Reco AI: hardware, catalog types, scoring, chat, inference.

pub mod apis;
pub mod chat;
pub mod config;
pub mod files;
pub mod hardware;
pub mod infer;
pub mod model;
pub mod paths;
pub mod quant;
pub mod recommend;
pub mod resolve;
pub mod store;

pub use hardware::{
    detect, detect_with, format_gib, AccelBackend, CpuInfo, GpuInfo, GpuVendor, HardwareProbe,
    HardwareProfile, MemoryInfo, OsInfo,
};
pub use model::{
    Catalog, CatalogSource, GgufFile, ModelEntry, ModelParams, Recommendation, Scores,
};
pub use quant::GgufQuant;
pub use apis::{
    advertised_base, generate_api_key, generate_client, slugify, write_client_kit, ApiEndpoint,
    ApiRegistry, ClientKind,
};
pub use config::RecoConfig;
pub use paths::{apis_path, clients_dir, config_path, data_dir, db_path};
pub use recommend::{memory_budget_bytes, recommend};
pub use resolve::{resolve_spec, suggest_repos, ResolveError};
pub use store::ChatStore;
