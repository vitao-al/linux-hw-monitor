use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::config::PathConfig;

use super::types::{SensorGroup, SensorUnit, SensorValue};

#[derive(Default)]
pub struct CpuSensor {
    last_usage: HashMap<String, (u64, u64)>,
}

impl CpuSensor {
    pub fn collect(&mut self, paths: &PathConfig) -> SensorGroup {
        let mut group = SensorGroup::new("cpu", "CPU", "cpu-symbolic");

        let hwmon_root = paths.sys_root.join("class/hwmon");
        for hw in iter_dirs(&hwmon_root) {
            let name = read_trimmed(hw.join("name"));
            if !matches!(name.as_deref(), Some("coretemp") | Some("k10temp") | Some("zenpower")) {
                continue;
            }

            for temp_input in glob_like(&hw, "temp", "_input") {
                if let Some(raw) = read_i64(&temp_input) {
                    let celsius = raw as f64 / 1000.0;
                    let index = temp_index(&temp_input).unwrap_or_default();
                    let label = read_trimmed(hw.join(format!("temp{}_label", index)))
                        .unwrap_or_else(|| format!("Core {}", index.saturating_sub(1)));
                    let critical = read_i64(hw.join(format!("temp{}_crit", index))).map(|v| v as f64 / 1000.0);
                    group
                        .sensors
                        .push(SensorValue::new(format!("{} Temp", label), celsius, SensorUnit::Celsius).with_range(None, None, critical));
                }
            }

            for voltage in glob_like(&hw, "in", "_input") {
                if let Some(raw_mv) = read_i64(&voltage) {
                    let volts = raw_mv as f64 / 1000.0;
                    let index = in_index(&voltage).unwrap_or(0);
                    let label = read_trimmed(hw.join(format!("in{}_label", index))).unwrap_or_else(|| format!("V{}", index));
                    group.sensors.push(SensorValue::new(label, volts, SensorUnit::Volt));
                }
            }
        }

        let cpu_root = paths.sys_root.join("devices/system/cpu");
        for cpu_dir in iter_dirs(&cpu_root) {
            let name = cpu_dir.file_name().and_then(|n| n.to_str()).unwrap_or_default();
            if !name.starts_with("cpu") || name == "cpufreq" || name == "cpuidle" {
                continue;
            }
            let cur = read_i64(cpu_dir.join("cpufreq/scaling_cur_freq")).map(|v| v as f64 / 1000.0);
            let min = read_i64(cpu_dir.join("cpufreq/scaling_min_freq")).map(|v| v as f64 / 1000.0);
            let max = read_i64(cpu_dir.join("cpufreq/scaling_max_freq")).map(|v| v as f64 / 1000.0);
            if let Some(mhz) = cur {
                group.sensors.push(SensorValue::new(format!("{} Clock", name.to_uppercase()), mhz, SensorUnit::MHz).with_range(min, max, None));
            }
        }

        let stat_path = paths.proc_root.join("stat");
        let usage_now = read_cpu_stat(&stat_path);
        for (cpu, (idle, total)) in usage_now {
            let value = if let Some((last_idle, last_total)) = self.last_usage.get(&cpu) {
                let idle_delta = idle.saturating_sub(*last_idle);
                let total_delta = total.saturating_sub(*last_total);
                if total_delta == 0 {
                    0.0
                } else {
                    ((total_delta - idle_delta) as f64 / total_delta as f64) * 100.0
                }
            } else {
                0.0
            };
            self.last_usage.insert(cpu.clone(), (idle, total));
            group.sensors.push(SensorValue::new(format!("{} Usage", cpu.to_uppercase()), value, SensorUnit::Percent).with_range(Some(0.0), Some(100.0), Some(95.0)));
        }

        if let Some(power) = read_rapl_power(&paths.sys_root) {
            group.sensors.push(SensorValue::new("Package Power", power, SensorUnit::Watt));
        }

        group
    }
}

fn read_rapl_power(sys_root: &Path) -> Option<f64> {
    let base = sys_root.join("class/powercap/intel-rapl/intel-rapl:0");
    let energy_uj = read_i64(base.join("energy_uj"))? as f64;
    let max_range = read_i64(base.join("max_energy_range_uj"))? as f64;
    if max_range <= 0.0 {
        return None;
    }
    Some((energy_uj / max_range) * 100.0)
}

fn read_cpu_stat(path: &Path) -> HashMap<String, (u64, u64)> {
    let mut map = HashMap::new();
    let Ok(content) = fs::read_to_string(path) else {
        return map;
    };

    for line in content.lines() {
        if !line.starts_with("cpu") {
            continue;
        }
        let mut parts = line.split_whitespace();
        let Some(id) = parts.next() else {
            continue;
        };
        let nums: Vec<u64> = parts.filter_map(|s| s.parse::<u64>().ok()).collect();
        if nums.len() < 4 {
            continue;
        }
        let idle = nums[3] + nums.get(4).copied().unwrap_or(0);
        let total = nums.iter().sum::<u64>();
        map.insert(id.to_string(), (idle, total));
    }

    map
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

fn glob_like(dir: &Path, prefix: &str, suffix: &str) -> Vec<PathBuf> {
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

fn temp_index(path: &Path) -> Option<u32> {
    path.file_name()
        .and_then(|n| n.to_str())
        .and_then(|name| name.strip_prefix("temp"))
        .and_then(|s| s.strip_suffix("_input"))
        .and_then(|s| s.parse::<u32>().ok())
}

fn in_index(path: &Path) -> Option<u32> {
    path.file_name()
        .and_then(|n| n.to_str())
        .and_then(|name| name.strip_prefix("in"))
        .and_then(|s| s.strip_suffix("_input"))
        .and_then(|s| s.parse::<u32>().ok())
}

fn read_trimmed(path: impl AsRef<Path>) -> Option<String> {
    let Ok(s) = fs::read_to_string(path) else {
        return None;
    };
    Some(s.trim().to_string())
}

fn read_i64(path: impl AsRef<Path>) -> Option<i64> {
    read_trimmed(path)?.parse::<i64>().ok()
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::read_cpu_stat;

    #[test]
    fn parses_proc_stat() {
        let tmp = TempDir::new().expect("tempdir");
        let p = tmp.path().join("stat");
        let content = "cpu  100 0 100 200 0 0 0 0 0 0\n"
            .to_string()
            + "cpu0 50 0 50 100 0 0 0 0 0 0\n";
        fs::write(&p, content).expect("write stat");

        let parsed = read_cpu_stat(&p);
        assert!(parsed.contains_key("cpu"));
        assert!(parsed.contains_key("cpu0"));
    }
}
