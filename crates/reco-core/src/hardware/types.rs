use serde::{Deserialize, Serialize};

/// Snapshot of the machine Reco will recommend and run models on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HardwareProfile {
    pub cpu: CpuInfo,
    pub memory: MemoryInfo,
    pub gpus: Vec<GpuInfo>,
    pub os: OsInfo,
}

impl HardwareProfile {
    /// Acceleration Reco should prefer: first discrete/usable GPU, else CPU.
    pub fn primary_backend(&self) -> AccelBackend {
        self.gpus
            .iter()
            .find(|gpu| gpu.backend != AccelBackend::Cpu)
            .map(|gpu| gpu.backend)
            .unwrap_or(AccelBackend::Cpu)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CpuInfo {
    pub name: String,
    pub physical_cores: Option<u32>,
    pub logical_cores: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryInfo {
    pub total_bytes: u64,
    pub available_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GpuInfo {
    pub name: String,
    pub vendor: GpuVendor,
    pub vram_bytes: Option<u64>,
    pub backend: AccelBackend,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GpuVendor {
    Nvidia,
    Amd,
    Intel,
    Apple,
    Unknown,
}

impl GpuVendor {
    pub fn from_pci_id(id: u16) -> Self {
        match id {
            0x10de => Self::Nvidia,
            0x1002 | 0x1022 => Self::Amd,
            0x8086 => Self::Intel,
            0x106b => Self::Apple,
            _ => Self::Unknown,
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::Nvidia => "NVIDIA",
            Self::Amd => "AMD",
            Self::Intel => "Intel",
            Self::Apple => "Apple",
            Self::Unknown => "GPU",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccelBackend {
    Cuda,
    Metal,
    Vulkan,
    Cpu,
}

impl AccelBackend {
    pub fn display_name(self) -> &'static str {
        match self {
            Self::Cuda => "CUDA",
            Self::Metal => "Metal",
            Self::Vulkan => "Vulkan",
            Self::Cpu => "CPU",
        }
    }

    pub fn for_vendor(vendor: GpuVendor) -> Self {
        match vendor {
            GpuVendor::Nvidia => Self::Cuda,
            GpuVendor::Apple => Self::Metal,
            GpuVendor::Amd | GpuVendor::Intel => Self::Vulkan,
            GpuVendor::Unknown => Self::Cpu,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OsInfo {
    pub name: String,
    pub version: Option<String>,
    pub arch: String,
    pub kernel: Option<String>,
}

/// Format bytes as a short GiB string (e.g. `8.0 GiB`).
pub fn format_gib(bytes: u64) -> String {
    const GIB: f64 = 1024.0 * 1024.0 * 1024.0;
    const MIB: f64 = 1024.0 * 1024.0;
    if bytes > 0 && (bytes as f64) < GIB / 10.0 {
        return format!("{:.1} MiB", bytes as f64 / MIB);
    }
    format!("{:.1} GiB", bytes as f64 / GIB)
}
