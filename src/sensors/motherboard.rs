use std::fs;
use std::path::{Path, PathBuf};

use crate::config::PathConfig;

use super::types::{SensorGroup, SensorUnit, SensorValue};

pub struct MotherboardSensor;

impl MotherboardSensor {
    pub fn collect(paths: &PathConfig) -> SensorGroup {
        let mut group = SensorGroup::new("motherboard", "Motherboard", "computer-symbolic");
        let hwmon_root = paths.sys_root.join("class/hwmon");

        for hw in iter_dirs(&hwmon_root) {
            let Some(name) = read_trimmed(hw.join("name")) else {
                continue;
            };
            if !is_board_chip(&name) {
                continue;
            }
            for input in collect_prefixed(&hw, "in", "_input") {
                let Some(raw_mv) = read_f64(&input) else {
                    continue;
                };
                let idx = index_of(&input, "in", "_input").unwrap_or(0);
                let label = read_trimmed(hw.join(format!("in{}_label", idx))).unwrap_or_else(|| format!("IN{}", idx));
                group.sensors.push(SensorValue::new(label, raw_mv / 1000.0, SensorUnit::Volt));
            }
            for input in collect_prefixed(&hw, "temp", "_input") {
                let Some(raw_mc) = read_f64(&input) else {
                    continue;
                };
                let idx = index_of(&input, "temp", "_input").unwrap_or(0);
                let label = read_trimmed(hw.join(format!("temp{}_label", idx))).unwrap_or_else(|| format!("TEMP{}", idx));
                group.sensors.push(SensorValue::new(label, raw_mc / 1000.0, SensorUnit::Celsius));
            }
        }

        let dmi = paths.sys_root.join("devices/virtual/dmi/id");
        for (file, label) in [
            ("board_name", "Board Name"),
            ("board_vendor", "Board Vendor"),
            ("board_version", "Board Version"),
            ("bios_version", "BIOS Version"),
            ("bios_date", "BIOS Date"),
        ] {
            if let Some(v) = read_trimmed(dmi.join(file)) {
                group.sensors.push(SensorValue::new(format!("{}: {}", label, v), 1.0, SensorUnit::Percent));
            }
        }

        group
    }
}

fn is_board_chip(name: &str) -> bool {
    ["nct6775", "it8728", "w83795", "ite"]
        .iter()
        .any(|chip| name.contains(chip))
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

fn collect_prefixed(dir: &Path, prefix: &str, suffix: &str) -> Vec<PathBuf> {
    let Ok(read_dir) = fs::read_dir(dir) else {
        return Vec::new();
    };
    read_dir
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|path| {
            path.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with(prefix) && n.ends_with(suffix))
                .unwrap_or(false)
        })
        .collect()
}

fn index_of(path: &Path, prefix: &str, suffix: &str) -> Option<u32> {
    path.file_name()
        .and_then(|n| n.to_str())
        .and_then(|name| name.strip_prefix(prefix))
        .and_then(|s| s.strip_suffix(suffix))
        .and_then(|s| s.parse::<u32>().ok())
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
