use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::config::PathConfig;

use super::types::{SensorGroup, SensorUnit, SensorValue};

pub struct MemorySensor;

impl MemorySensor {
    pub fn collect(paths: &PathConfig) -> SensorGroup {
        let mut group = SensorGroup::new("memory", "Memory", "media-floppy-symbolic");
        let meminfo = parse_meminfo(&paths.proc_root.join("meminfo"));

        let total = meminfo.get("MemTotal").copied().unwrap_or(0.0);
        let available = meminfo.get("MemAvailable").copied().unwrap_or(0.0);
        let free = meminfo.get("MemFree").copied().unwrap_or(0.0);
        let used = (total - available).max(0.0);
        let used_percent = if total > 0.0 { (used / total) * 100.0 } else { 0.0 };

        group
            .sensors
            .push(SensorValue::new("RAM Used", used / 1024.0, SensorUnit::MB).with_range(Some(0.0), Some(total / 1024.0), Some(95.0)));
        group
            .sensors
            .push(SensorValue::new("RAM Free", free / 1024.0, SensorUnit::MB));
        group
            .sensors
            .push(SensorValue::new("RAM Usage", used_percent, SensorUnit::Percent).with_range(Some(0.0), Some(100.0), Some(95.0)));

        let swap_total = meminfo.get("SwapTotal").copied().unwrap_or(0.0);
        let swap_free = meminfo.get("SwapFree").copied().unwrap_or(0.0);
        if swap_total > 0.0 {
            group.sensors.push(SensorValue::new("Swap Used", (swap_total - swap_free) / 1024.0, SensorUnit::MB).with_range(Some(0.0), Some(swap_total / 1024.0), None));
        }

        group
    }
}

pub fn parse_meminfo(path: &Path) -> HashMap<String, f64> {
    let mut out = HashMap::new();
    let Ok(content) = fs::read_to_string(path) else {
        return out;
    };

    for line in content.lines() {
        let Some((k, v)) = line.split_once(':') else {
            continue;
        };
        let num = v.split_whitespace().next().and_then(|n| n.parse::<f64>().ok());
        if let Some(value) = num {
            out.insert(k.to_string(), value);
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::parse_meminfo;

    #[test]
    fn parses_meminfo() {
        let tmp = TempDir::new().expect("tempdir");
        let p = tmp.path().join("meminfo");
        fs::write(&p, "MemTotal: 1000 kB\nMemAvailable: 500 kB\n").expect("write meminfo");
        let m = parse_meminfo(&p);
        assert_eq!(m.get("MemTotal").copied().unwrap_or(0.0), 1000.0);
    }
}
