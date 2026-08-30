//! Shared domain for Reco AI: hardware, catalog types, scoring, chat, inference.

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
pub use config::RecoConfig;
pub use paths::{config_path, data_dir, db_path};
pub use recommend::{memory_budget_bytes, recommend};
pub use resolve::{resolve_spec, ResolveError};
pub use store::ChatStore;
