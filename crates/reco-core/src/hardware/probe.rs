use super::types::{CpuInfo, GpuInfo, MemoryInfo, OsInfo};

/// Abstraction over hardware sampling so tests can inject fixed profiles.
pub trait HardwareProbe {
    fn cpu(&self) -> CpuInfo;
    fn memory(&self) -> MemoryInfo;
    fn gpus(&self) -> Vec<GpuInfo>;
    fn os(&self) -> OsInfo;
}

/// Live probe that reads the current machine.
pub struct SystemProbe;

impl HardwareProbe for SystemProbe {
    fn cpu(&self) -> CpuInfo {
        super::detect::cpu_from_sysinfo()
    }

    fn memory(&self) -> MemoryInfo {
        super::detect::memory_from_sysinfo()
    }

    fn gpus(&self) -> Vec<GpuInfo> {
        super::gpu::detect_gpus()
    }

    fn os(&self) -> OsInfo {
        super::detect::os_from_sysinfo()
    }
}
