//! Best-effort GPU detection. Never panics; each backend is independently optional.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::types::{AccelBackend, GpuInfo, GpuVendor};

pub(super) fn detect_gpus() -> Vec<GpuInfo> {
    let mut gpus = Vec::new();

    #[cfg(not(target_os = "macos"))]
    {
        gpus.extend(from_nvml());
        if !gpus.iter().any(|g| g.vendor == GpuVendor::Nvidia) {
            gpus.extend(from_nvidia_smi());
        }
    }

    #[cfg(target_os = "linux")]
    {
        let extras = from_sysfs_drm(&gpus);
        gpus.extend(extras);
    }

    #[cfg(target_os = "macos")]
    {
        gpus.extend(from_system_profiler());
    }

    gpus
}

#[cfg(not(target_os = "macos"))]
fn from_nvml() -> Vec<GpuInfo> {
    let nvml = match nvml_wrapper::Nvml::init() {
        Ok(nvml) => nvml,
        Err(_) => return Vec::new(),
    };

    let count = match nvml.device_count() {
        Ok(count) => count,
        Err(_) => return Vec::new(),
    };

    let mut gpus = Vec::new();
    for index in 0..count {
        let Ok(device) = nvml.device_by_index(index) else {
            continue;
        };
        let name = device
            .name()
            .unwrap_or_else(|_| format!("NVIDIA GPU {index}"));
        let vram_bytes = device.memory_info().ok().map(|mem| mem.total);
        gpus.push(GpuInfo {
            name,
            vendor: GpuVendor::Nvidia,
            vram_bytes,
            backend: AccelBackend::Cuda,
        });
    }
    gpus
}

#[cfg(not(target_os = "macos"))]
fn from_nvidia_smi() -> Vec<GpuInfo> {
    let output = Command::new("nvidia-smi")
        .args([
            "--query-gpu=name,memory.total",
            "--format=csv,noheader,nounits",
        ])
        .output();

    let Ok(output) = output else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }

    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(parse_nvidia_smi_line)
        .collect()
}

pub(crate) fn parse_nvidia_smi_line(line: &str) -> Option<GpuInfo> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }
    let (name, mem) = line.split_once(',')?;
    let name = name.trim();
    if name.is_empty() {
        return None;
    }
    let mib: u64 = mem.trim().parse().ok()?;
    Some(GpuInfo {
        name: name.to_string(),
        vendor: GpuVendor::Nvidia,
        vram_bytes: Some(mib.saturating_mul(1024 * 1024)),
        backend: AccelBackend::Cuda,
    })
}

#[cfg(target_os = "linux")]
fn from_sysfs_drm(already: &[GpuInfo]) -> Vec<GpuInfo> {
    let Ok(entries) = fs::read_dir("/sys/class/drm") else {
        return Vec::new();
    };

    let have_nvidia = already.iter().any(|g| g.vendor == GpuVendor::Nvidia);
    let mut gpus = Vec::new();

    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !is_primary_drm_card(&name) {
            continue;
        }
        let device_dir = entry.path().join("device");
        let Some(gpu) = gpu_from_sysfs_device(&device_dir) else {
            continue;
        };
        if have_nvidia && gpu.vendor == GpuVendor::Nvidia {
            continue;
        }
        if gpus.iter().any(|existing| similar_gpu(existing, &gpu)) {
            continue;
        }
        gpus.push(gpu);
    }

    gpus
}

pub(crate) fn is_primary_drm_card(name: &str) -> bool {
    let rest = match name.strip_prefix("card") {
        Some(rest) => rest,
        None => return false,
    };
    !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit())
}

fn similar_gpu(a: &GpuInfo, b: &GpuInfo) -> bool {
    a.vendor == b.vendor && a.name == b.name
}

#[cfg(target_os = "linux")]
fn gpu_from_sysfs_device(device_dir: &Path) -> Option<GpuInfo> {
    let vendor_id = read_hex_u16(&device_dir.join("vendor"))?;
    let device_id = read_hex_u16(&device_dir.join("device"));
    let vendor = GpuVendor::from_pci_id(vendor_id);

    if vendor == GpuVendor::Unknown {
        return None;
    }

    let name = pci_device_name(vendor_id, device_id)
        .or_else(|| read_trimmed(device_dir.join("product_name")))
        .unwrap_or_else(|| format!("{} GPU", vendor.display_name()));

    let vram_bytes = read_u64(device_dir.join("mem_info_vram_total"))
        .or_else(|| read_u64(device_dir.join("mem_info_vis_vram_total")));

    Some(GpuInfo {
        name,
        vendor,
        vram_bytes,
        backend: AccelBackend::for_vendor(vendor),
    })
}

fn read_trimmed(path: PathBuf) -> Option<String> {
    fs::read_to_string(path)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn read_hex_u16(path: &Path) -> Option<u16> {
    let raw = fs::read_to_string(path).ok()?;
    let hex = raw.trim().trim_start_matches("0x");
    u16::from_str_radix(hex, 16).ok()
}

fn read_u64(path: PathBuf) -> Option<u64> {
    fs::read_to_string(path)
        .ok()?
        .trim()
        .parse()
        .ok()
        .filter(|n| *n > 0)
}

fn pci_device_name(vendor_id: u16, device_id: Option<u16>) -> Option<String> {
    let ids = fs::read_to_string("/usr/share/hwdata/pci.ids")
        .or_else(|_| fs::read_to_string("/usr/share/misc/pci.ids"))
        .ok()?;
    lookup_pci_ids(&ids, vendor_id, device_id)
}

pub(crate) fn lookup_pci_ids(ids: &str, vendor_id: u16, device_id: Option<u16>) -> Option<String> {
    let vendor_key = format!("{vendor_id:04x}");
    let device_key = device_id.map(|id| format!("{id:04x}"));

    let mut in_vendor = false;
    let mut vendor_name: Option<String> = None;
    let mut device_name: Option<String> = None;

    for line in ids.lines() {
        if line.starts_with('#') || line.trim().is_empty() {
            continue;
        }
        if let Some(rest) = line.strip_prefix('\t') {
            if !in_vendor || rest.starts_with('\t') {
                continue;
            }
            let Some(device_key) = device_key.as_deref() else {
                continue;
            };
            if rest.len() >= 4 && rest[..4].eq_ignore_ascii_case(device_key) {
                device_name = Some(rest[4..].trim().to_string());
                break;
            }
            continue;
        }
        if line.len() >= 4 && line[..4].eq_ignore_ascii_case(&vendor_key) {
            in_vendor = true;
            vendor_name = Some(line[4..].trim().to_string());
            continue;
        }
        if in_vendor {
            break;
        }
    }

    match (vendor_name, device_name) {
        (Some(_), Some(device)) => Some(device),
        (Some(vendor), None) => Some(vendor),
        _ => None,
    }
}

#[cfg(target_os = "macos")]
fn from_system_profiler() -> Vec<GpuInfo> {
    let output = Command::new("system_profiler")
        .args(["SPDisplaysDataType", "-detailLevel", "mini"])
        .output();

    let Ok(output) = output else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }

    parse_system_profiler(&String::from_utf8_lossy(&output.stdout))
}

#[cfg(any(test, target_os = "macos"))]
pub(crate) fn parse_system_profiler(text: &str) -> Vec<GpuInfo> {
    let mut gpus = Vec::new();
    let mut current_name: Option<String> = None;
    let mut current_vram: Option<u64> = None;

    let flush = |gpus: &mut Vec<GpuInfo>, name: &mut Option<String>, vram: &mut Option<u64>| {
        if let Some(name) = name.take() {
            let vendor = vendor_from_name(&name);
            gpus.push(GpuInfo {
                name,
                vendor,
                vram_bytes: *vram,
                backend: AccelBackend::for_vendor(vendor),
            });
            *vram = None;
        }
    };

    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(name) = trimmed.strip_suffix(':') {
            if line.starts_with("        ") || line.starts_with('\t') {
                continue;
            }
            if looks_like_chipset_header(name) {
                flush(&mut gpus, &mut current_name, &mut current_vram);
                current_name = Some(name.trim().to_string());
            }
            continue;
        }
        if let Some(model) = key_value(trimmed, "Chipset Model") {
            flush(&mut gpus, &mut current_name, &mut current_vram);
            current_name = Some(model);
            continue;
        }
        if let Some(vram) = key_value(trimmed, "VRAM (Total)")
            .or_else(|| key_value(trimmed, "VRAM (Dynamic, Max)"))
            .or_else(|| key_value(trimmed, "VRAM"))
        {
            current_vram = parse_vram_label(&vram);
        }
    }
    flush(&mut gpus, &mut current_name, &mut current_vram);
    gpus
}

#[cfg(any(test, target_os = "macos"))]
fn looks_like_chipset_header(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.contains("apple")
        || lower.contains("radeon")
        || lower.contains("geforce")
        || lower.contains("iris")
        || lower.contains("intel")
        || lower.contains("gpu")
}

#[cfg(any(test, target_os = "macos"))]
fn key_value(line: &str, key: &str) -> Option<String> {
    let prefix = format!("{key}:");
    line.strip_prefix(&prefix)
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

#[cfg(any(test, target_os = "macos"))]
pub(crate) fn parse_vram_label(label: &str) -> Option<u64> {
    let mut parts = label.split_whitespace();
    let amount: f64 = parts.next()?.replace(',', ".").parse().ok()?;
    let unit = parts.next().unwrap_or("MB").to_ascii_uppercase();
    let multiplier = if unit.starts_with("G") {
        1024.0 * 1024.0 * 1024.0
    } else if unit.starts_with("M") {
        1024.0 * 1024.0
    } else {
        return None;
    };
    Some((amount * multiplier) as u64)
}

#[cfg(any(test, target_os = "macos"))]
pub(crate) fn vendor_from_name(name: &str) -> GpuVendor {
    let lower = name.to_ascii_lowercase();
    if lower.contains("nvidia") || lower.contains("geforce") || lower.contains("rtx") {
        GpuVendor::Nvidia
    } else if lower.contains("amd") || lower.contains("radeon") {
        GpuVendor::Amd
    } else if lower.contains("apple")
        || lower.contains("m1")
        || lower.contains("m2")
        || lower.contains("m3")
        || lower.contains("m4")
    {
        GpuVendor::Apple
    } else if lower.contains("intel") || lower.contains("iris") || lower.contains("uhd") {
        GpuVendor::Intel
    } else {
        GpuVendor::Unknown
    }
}
