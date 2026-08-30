//! Hardware detection used by `reco ai` / `reco hw` and, later, the recommender.

mod detect;
mod gpu;
mod probe;
mod types;

pub use detect::{detect, detect_with};
pub use probe::{HardwareProbe, SystemProbe};
pub use types::{
    format_gib, AccelBackend, CpuInfo, GpuInfo, GpuVendor, HardwareProfile, MemoryInfo, OsInfo,
};

#[cfg(test)]
mod tests;
