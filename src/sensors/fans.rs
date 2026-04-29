use std::fs;
use std::path::{Path, PathBuf};

use crate::config::PathConfig;

use super::types::{SensorGroup, SensorUnit, SensorValue};

pub struct FansSensor;

impl FansSensor {
    pub fn collect(paths: &PathConfig) -> SensorGroup {
        let mut group = SensorGroup::new("fans", "Fans", "weather-windy-symbolic");
        let hwmon_root = paths.sys_root.join("class/hwmon");

        for hw in iter_dirs(&hwmon_root) {
            let chip = read_trimmed(hw.join("name")).unwrap_or_else(|| "hwmon".to_string());
            for fan in collect_prefixed(&hw, "fan", "_input") {
                let Some(rpm) = read_f64(&fan) else {
                    continue;
                };
                let idx = index_of(&fan, "fan", "_input").unwrap_or(0);
                let label = read_trimmed(hw.join(format!("fan{}_label", idx))).unwrap_or_else(|| format!("{} Fan {}", chip, idx));
                group.sensors.push(SensorValue::new(label, rpm, SensorUnit::RPM));
            }
            for pwm in collect_prefixed(&hw, "pwm", "") {
                let Some(raw) = read_f64(&pwm) else {
                    continue;
                };
                let idx = index_of(&pwm, "pwm", "").unwrap_or(0);
                let duty = (raw / 255.0) * 100.0;
                group
                    .sensors
                    .push(SensorValue::new(format!("{} PWM {}", chip, idx), duty, SensorUnit::Percent).with_range(Some(0.0), Some(100.0), None));
            }
        }

        group
    }
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
                .map(|n| {
                    n.starts_with(prefix)
                        && (if suffix.is_empty() {
                            !n.contains("_enable")
                        } else {
                            n.ends_with(suffix)
                        })
                })
                .unwrap_or(false)
        })
        .collect()
}

fn index_of(path: &Path, prefix: &str, suffix: &str) -> Option<u32> {
    path.file_name()
        .and_then(|n| n.to_str())
        .and_then(|name| name.strip_prefix(prefix))
        .and_then(|s| {
            if suffix.is_empty() {
                Some(s)
            } else {
                s.strip_suffix(suffix)
            }
        })
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
