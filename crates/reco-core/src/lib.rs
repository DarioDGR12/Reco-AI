//! Shared domain for Reco AI: hardware detection, GGUF types, and scoring.

pub mod files;
pub mod hardware;
pub mod model;
pub mod quant;
pub mod recommend;
pub mod resolve;

pub use hardware::{
    detect, detect_with, format_gib, AccelBackend, CpuInfo, GpuInfo, GpuVendor, HardwareProbe,
    HardwareProfile, MemoryInfo, OsInfo,
};
pub use model::{
    Catalog, CatalogSource, GgufFile, ModelEntry, ModelParams, Recommendation, Scores,
};
pub use quant::GgufQuant;
pub use recommend::{memory_budget_bytes, recommend};
pub use resolve::{resolve_spec, ResolveError};
