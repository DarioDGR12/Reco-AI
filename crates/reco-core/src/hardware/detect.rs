use sysinfo::{RefreshKind, System};

use super::probe::{HardwareProbe, SystemProbe};
use super::types::{CpuInfo, HardwareProfile, MemoryInfo, OsInfo};

/// Detect hardware on this machine. Never panics; missing pieces are omitted.
pub fn detect() -> HardwareProfile {
    detect_with(&SystemProbe)
}

/// Detect using an injected probe (tests, fixtures).
pub fn detect_with(probe: &dyn HardwareProbe) -> HardwareProfile {
    HardwareProfile {
        cpu: probe.cpu(),
        memory: probe.memory(),
        gpus: probe.gpus(),
        os: probe.os(),
    }
}

fn system() -> System {
    let mut sys = System::new_with_specifics(RefreshKind::nothing());
    sys.refresh_cpu_all();
    sys.refresh_memory();
    sys
}

pub(super) fn cpu_from_sysinfo() -> CpuInfo {
    let sys = system();
    let name = sys
        .cpus()
        .first()
        .map(|cpu| cpu.brand().trim().to_string())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "CPU desconocida".to_string());

    CpuInfo {
        name,
        physical_cores: sys.physical_core_count().map(|n| n as u32),
        logical_cores: sys.cpus().len() as u32,
    }
}

pub(super) fn memory_from_sysinfo() -> MemoryInfo {
    let sys = system();
    MemoryInfo {
        total_bytes: sys.total_memory(),
        available_bytes: sys.available_memory(),
    }
}

pub(super) fn os_from_sysinfo() -> OsInfo {
    OsInfo {
        name: System::name().unwrap_or_else(|| std::env::consts::OS.to_string()),
        version: System::os_version().or_else(System::long_os_version),
        arch: System::cpu_arch(),
        kernel: System::kernel_version(),
    }
}
