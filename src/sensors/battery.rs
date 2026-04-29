use std::fs;
use std::path::{Path, PathBuf};

use crate::config::PathConfig;

use super::types::{SensorGroup, SensorUnit, SensorValue};

pub struct BatterySensor;

impl BatterySensor {
    pub fn collect(paths: &PathConfig) -> SensorGroup {
        let mut group = SensorGroup::new("battery", "Battery", "battery-symbolic");
        let power_root = paths.sys_root.join("class/power_supply");

        for bat in iter_battery_dirs(&power_root) {
            let name = bat.file_name().and_then(|n| n.to_str()).unwrap_or("BAT");
            if let Some(cap) = read_f64(bat.join("capacity")) {
                group
                    .sensors
                    .push(SensorValue::new(format!("{} Capacity", name), cap, SensorUnit::Percent).with_range(Some(0.0), Some(100.0), Some(15.0)));
            }

            let energy_full = read_f64(bat.join("energy_full")).or_else(|| read_f64(bat.join("charge_full")));
            let energy_design =
                read_f64(bat.join("energy_full_design")).or_else(|| read_f64(bat.join("charge_full_design")));
            if let (Some(full), Some(design)) = (energy_full, energy_design) {
                if design > 0.0 {
                    group
                        .sensors
                        .push(SensorValue::new(format!("{} Health", name), (full / design) * 100.0, SensorUnit::Percent));
                }
            }

            if let Some(voltage_uv) = read_f64(bat.join("voltage_now")) {
                group.sensors.push(SensorValue::new(format!("{} Voltage", name), voltage_uv / 1_000_000.0, SensorUnit::Volt));
            }
            if let Some(current_ua) = read_f64(bat.join("current_now")) {
                group.sensors.push(SensorValue::new(format!("{} Current", name), current_ua / 1_000_000.0, SensorUnit::Watt));
            }
            if let Some(cycles) = read_f64(bat.join("cycle_count")) {
                group.sensors.push(SensorValue::new(format!("{} Cycle Count", name), cycles, SensorUnit::Percent));
            }
        }

        group
    }
}

fn iter_battery_dirs(path: &Path) -> Vec<PathBuf> {
    let Ok(read_dir) = fs::read_dir(path) else {
        return Vec::new();
    };

    read_dir
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with("BAT"))
                .unwrap_or(false)
        })
        .collect()
}

fn read_f64(path: impl AsRef<Path>) -> Option<f64> {
    let Ok(s) = fs::read_to_string(path) else {
        return None;
    };
    s.trim().parse::<f64>().ok()
}
