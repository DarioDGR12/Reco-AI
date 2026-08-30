//! Shared domain for Reco AI.
//!
//! The first implemented piece is hardware detection. Catalog indexing,
//! recommendation scoring, and inference will land in later crates/modules.

pub mod hardware;

pub use hardware::{
    detect, detect_with, format_gib, AccelBackend, CpuInfo, GpuInfo, GpuVendor, HardwareProbe,
    HardwareProfile, MemoryInfo, OsInfo,
};
