use super::fixtures::{apple_unified, cpu_only, rtx_4060};
use super::gpu::{
    is_primary_drm_card, lookup_pci_ids, parse_nvidia_smi_line, parse_system_profiler,
    parse_vram_label, vendor_from_name,
};
use super::types::{
    format_gib, AccelBackend, CpuInfo, GpuInfo, GpuVendor, HardwareProfile, MemoryInfo, OsInfo,
};
use super::{detect, detect_with, HardwareProbe};

struct MockProbe {
    profile: HardwareProfile,
}

impl HardwareProbe for MockProbe {
    fn cpu(&self) -> CpuInfo {
        self.profile.cpu.clone()
    }
    fn memory(&self) -> MemoryInfo {
        self.profile.memory.clone()
    }
    fn gpus(&self) -> Vec<GpuInfo> {
        self.profile.gpus.clone()
    }
    fn os(&self) -> OsInfo {
        self.profile.os.clone()
    }
}

const GIB: u64 = 1024 * 1024 * 1024;

#[test]
fn mock_rtx_4060_roundtrips_json() {
    let profile = rtx_4060();
    let json = serde_json::to_string_pretty(&profile).unwrap();
    let back: HardwareProfile = serde_json::from_str(&json).unwrap();
    assert_eq!(profile, back);
    assert!(json.contains("\"vendor\": \"nvidia\""));
    assert!(json.contains("\"backend\": \"cuda\""));
    assert_eq!(profile.primary_backend(), AccelBackend::Cuda);
}

#[test]
fn mock_cpu_only_uses_cpu_backend() {
    let profile = detect_with(&MockProbe {
        profile: cpu_only(),
    });
    assert!(profile.gpus.is_empty());
    assert_eq!(profile.primary_backend(), AccelBackend::Cpu);
    assert_eq!(profile.memory.total_bytes, 8 * GIB);
}

#[test]
fn mock_apple_unified_uses_metal() {
    let profile = detect_with(&MockProbe {
        profile: apple_unified(),
    });
    assert_eq!(profile.primary_backend(), AccelBackend::Metal);
    assert_eq!(profile.gpus[0].vendor, GpuVendor::Apple);
}

#[test]
fn detect_live_machine_does_not_panic() {
    let profile = detect();
    assert!(
        profile.memory.total_bytes > 0,
        "RAM should be reported on CI"
    );
    assert!(!profile.cpu.name.is_empty(), "CPU name should not be empty");
    assert!(!profile.os.arch.is_empty(), "arch should not be empty");
    let _ = serde_json::to_string(&profile).unwrap();
}

#[test]
fn format_gib_one_decimal() {
    assert_eq!(format_gib(8 * GIB), "8.0 GiB");
    assert_eq!(format_gib(GIB / 2), "0.5 GiB");
}

#[test]
fn parse_nvidia_smi_csv() {
    let gpu = parse_nvidia_smi_line("NVIDIA GeForce RTX 4060, 8188").unwrap();
    assert_eq!(gpu.name, "NVIDIA GeForce RTX 4060");
    assert_eq!(gpu.vendor, GpuVendor::Nvidia);
    assert_eq!(gpu.backend, AccelBackend::Cuda);
    assert_eq!(gpu.vram_bytes, Some(8188 * 1024 * 1024));
    assert!(parse_nvidia_smi_line("").is_none());
}

#[test]
fn drm_card_names() {
    assert!(is_primary_drm_card("card0"));
    assert!(is_primary_drm_card("card1"));
    assert!(!is_primary_drm_card("card0-HDMI-A-1"));
    assert!(!is_primary_drm_card("renderD128"));
}

#[test]
fn pci_ids_lookup() {
    let ids = "\
10de  NVIDIA Corporation\n\
\t2882  AD107 [GeForce RTX 4060]\n\
1002  Advanced Micro Devices, Inc. [AMD/ATI]\n\
";
    assert_eq!(
        lookup_pci_ids(ids, 0x10de, Some(0x2882)).as_deref(),
        Some("AD107 [GeForce RTX 4060]")
    );
    assert_eq!(
        lookup_pci_ids(ids, 0x10de, Some(0x0000)).as_deref(),
        Some("NVIDIA Corporation")
    );
}

#[test]
fn vram_labels() {
    assert_eq!(parse_vram_label("8 GB"), Some(8 * GIB));
    assert_eq!(parse_vram_label("8188 MB"), Some(8188 * 1024 * 1024));
}

#[test]
fn vendor_from_model_name() {
    assert_eq!(
        vendor_from_name("NVIDIA GeForce RTX 4060"),
        GpuVendor::Nvidia
    );
    assert_eq!(vendor_from_name("Apple M3"), GpuVendor::Apple);
    assert_eq!(vendor_from_name("AMD Radeon RX 7600"), GpuVendor::Amd);
    assert_eq!(vendor_from_name("Intel Iris Xe"), GpuVendor::Intel);
}

#[test]
fn system_profiler_apple_m3() {
    let text = "\
Graphics/Displays:

    Chipset Model: Apple M3
    Type: GPU
    Bus: Built-In
    VRAM (Total): 16 GB
    Metal Support: Metal 3
";
    let gpus = parse_system_profiler(text);
    assert_eq!(gpus.len(), 1);
    assert_eq!(gpus[0].name, "Apple M3");
    assert_eq!(gpus[0].vendor, GpuVendor::Apple);
    assert_eq!(gpus[0].backend, AccelBackend::Metal);
    assert_eq!(gpus[0].vram_bytes, Some(16 * GIB));
}
