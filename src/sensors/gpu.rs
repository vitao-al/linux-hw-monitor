use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::config::PathConfig;

use super::types::{SensorGroup, SensorUnit, SensorValue};

pub struct GpuSensor;

impl GpuSensor {
    pub fn collect(paths: &PathConfig) -> SensorGroup {
        let mut group = SensorGroup::new("gpu", "GPU", "video-card-symbolic");
        let hwmon_root = paths.sys_root.join("class/hwmon");

        for hw in iter_dirs(&hwmon_root) {
            let Some(name) = read_trimmed(hw.join("name")) else {
                continue;
            };
            if name == "amdgpu" {
                if let Some(v) = read_f64(hw.join("temp1_input")) {
                    group.sensors.push(SensorValue::new("GPU Die Temp", v / 1000.0, SensorUnit::Celsius));
                }
                if let Some(v) = read_f64(hw.join("temp2_input")) {
                    group.sensors.push(SensorValue::new("GPU Hotspot Temp", v / 1000.0, SensorUnit::Celsius));
                }
                if let Some(v) = read_f64(hw.join("power1_average")) {
                    group.sensors.push(SensorValue::new("GPU Power", v / 1_000_000.0, SensorUnit::Watt));
                }
                if let Some(v) = read_f64(hw.join("in0_input")) {
                    group.sensors.push(SensorValue::new("GPU Voltage", v / 1000.0, SensorUnit::Volt));
                }
            }
            if name.contains("i915") {
                if let Some(v) = read_f64(hw.join("temp1_input")) {
                    group.sensors.push(SensorValue::new("iGPU Temp", v / 1000.0, SensorUnit::Celsius));
                }
            }
        }

        let drm_root = paths.sys_root.join("class/drm");
        for card in iter_dirs(&drm_root) {
            let Some(card_name) = card.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if !card_name.starts_with("card") {
                continue;
            }

            if let Some(freq) = read_f64(card.join("gt_cur_freq_mhz")) {
                group.sensors.push(SensorValue::new("Intel GT Clock", freq, SensorUnit::MHz));
            }
            if let Some(freq) = read_amdgpu_dpm(card.join("device/pp_dpm_sclk")) {
                group.sensors.push(SensorValue::new("AMD GPU Clock", freq, SensorUnit::MHz));
            }
            if let Some(freq) = read_amdgpu_dpm(card.join("device/pp_dpm_mclk")) {
                group.sensors.push(SensorValue::new("AMD VRAM Clock", freq, SensorUnit::MHz));
            }
            if let Some(v) = read_f64(card.join("device/gpu_busy_percent")) {
                group.sensors.push(SensorValue::new("GPU Usage", v, SensorUnit::Percent));
            }
            if let Some(v) = read_f64(card.join("device/mem_info_vram_total")) {
                group.sensors.push(SensorValue::new("VRAM Total", v, SensorUnit::Bytes));
            }
            if let Some(v) = read_f64(card.join("device/mem_info_vram_used")) {
                group.sensors.push(SensorValue::new("VRAM Used", v, SensorUnit::Bytes));
            }
        }

        collect_nvidia_smi(&mut group);

        group
    }
}

fn collect_nvidia_smi(group: &mut SensorGroup) {
    let output = Command::new("nvidia-smi")
        .arg("--query-gpu=temperature.gpu,utilization.gpu,utilization.memory,memory.total,memory.used,power.draw,clocks.current.graphics,clocks.current.memory")
        .arg("--format=csv,noheader,nounits")
        .output();

    let Ok(out) = output else {
        return;
    };
    if !out.status.success() {
        return;
    }

    let Ok(body) = String::from_utf8(out.stdout) else {
        return;
    };

    for (idx, line) in body.lines().enumerate() {
        let cols: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
        if cols.len() < 8 {
            continue;
        }
        let prefix = format!("NVIDIA GPU {}", idx);
        if let Ok(v) = cols[0].parse::<f64>() {
            group.sensors.push(SensorValue::new(format!("{} Temp", prefix), v, SensorUnit::Celsius));
        }
        if let Ok(v) = cols[1].parse::<f64>() {
            group.sensors.push(SensorValue::new(format!("{} Usage", prefix), v, SensorUnit::Percent));
        }
        if let Ok(v) = cols[4].parse::<f64>() {
            group.sensors.push(SensorValue::new(format!("{} VRAM Used", prefix), v, SensorUnit::MB));
        }
        if let Ok(v) = cols[5].parse::<f64>() {
            group.sensors.push(SensorValue::new(format!("{} Power", prefix), v, SensorUnit::Watt));
        }
    }
}

fn read_amdgpu_dpm(path: impl AsRef<Path>) -> Option<f64> {
    let Ok(content) = fs::read_to_string(path) else {
        return None;
    };
    for line in content.lines() {
        if !line.contains('*') {
            continue;
        }
        let val = line
            .split_whitespace()
            .find(|s| s.ends_with("Mhz") || s.ends_with("MHz"))
            .map(|s| s.trim_end_matches("Mhz").trim_end_matches("MHz"));
        if let Some(mhz) = val.and_then(|v| v.parse::<f64>().ok()) {
            return Some(mhz);
        }
    }
    None
}

fn iter_dirs(path: &Path) -> Vec<PathBuf> {
    let Ok(read_dir) = fs::read_dir(path) else {
        return Vec::new();
    };
    read_dir
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect()
}

fn read_trimmed(path: impl AsRef<Path>) -> Option<String> {
    let Ok(s) = fs::read_to_string(path) else {
        return None;
    };
    Some(s.trim().to_string())
}

fn read_f64(path: impl AsRef<Path>) -> Option<f64> {
    read_trimmed(path)?.parse::<f64>().ok()
}
