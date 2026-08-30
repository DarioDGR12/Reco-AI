//! Fixed profiles for tests, docs, and `reco ai --fixture`.

use super::types::{
    AccelBackend, CpuInfo, GpuInfo, GpuVendor, HardwareProfile, MemoryInfo, OsInfo,
};

const GIB: u64 = 1024 * 1024 * 1024;

pub fn rtx_4060() -> HardwareProfile {
    HardwareProfile {
        cpu: CpuInfo {
            name: "AMD Ryzen 7 5800X".into(),
            physical_cores: Some(8),
            logical_cores: 16,
        },
        memory: MemoryInfo {
            total_bytes: 32 * GIB,
            available_bytes: 18 * GIB,
        },
        gpus: vec![GpuInfo {
            name: "NVIDIA GeForce RTX 4060".into(),
            vendor: GpuVendor::Nvidia,
            vram_bytes: Some(8 * GIB),
            backend: AccelBackend::Cuda,
        }],
        os: OsInfo {
            name: "Linux".into(),
            version: Some("24.04".into()),
            arch: "x86_64".into(),
            kernel: Some("6.12.0".into()),
        },
    }
}

pub fn cpu_only() -> HardwareProfile {
    HardwareProfile {
        cpu: CpuInfo {
            name: "Intel Core i5-8250U".into(),
            physical_cores: Some(4),
            logical_cores: 8,
        },
        memory: MemoryInfo {
            total_bytes: 8 * GIB,
            available_bytes: 3 * GIB,
        },
        gpus: vec![],
        os: OsInfo {
            name: "Linux".into(),
            version: None,
            arch: "x86_64".into(),
            kernel: None,
        },
    }
}

pub fn apple_unified() -> HardwareProfile {
    HardwareProfile {
        cpu: CpuInfo {
            name: "Apple M3".into(),
            physical_cores: Some(8),
            logical_cores: 8,
        },
        memory: MemoryInfo {
            total_bytes: 16 * GIB,
            available_bytes: 8 * GIB,
        },
        gpus: vec![GpuInfo {
            name: "Apple M3".into(),
            vendor: GpuVendor::Apple,
            vram_bytes: Some(16 * GIB),
            backend: AccelBackend::Metal,
        }],
        os: OsInfo {
            name: "macOS".into(),
            version: Some("15.0".into()),
            arch: "arm64".into(),
            kernel: Some("24.0.0".into()),
        },
    }
}

pub fn by_name(name: &str) -> Option<HardwareProfile> {
    match name.to_ascii_lowercase().as_str() {
        "rtx4060" | "rtx-4060" | "4060" => Some(rtx_4060()),
        "cpu" | "cpu-only" => Some(cpu_only()),
        "apple" | "m3" | "apple-m3" => Some(apple_unified()),
        _ => None,
    }
}
