use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::config::PathConfig;

use super::types::{SensorGroup, SensorUnit, SensorValue};

#[derive(Default)]
pub struct NetworkSensor {
    last: HashMap<String, (u64, u64)>,
}

impl NetworkSensor {
    pub fn collect(&mut self, paths: &PathConfig) -> SensorGroup {
        let mut group = SensorGroup::new("network", "Network", "network-wireless-symbolic");
        let now = parse_net_dev(&paths.proc_root.join("net/dev"));
        let net_root = paths.sys_root.join("class/net");

        for iface in iter_dirs(&net_root) {
            let Some(name) = iface.file_name().and_then(|n| n.to_str()).map(ToOwned::to_owned) else {
                continue;
            };
            if name == "lo" {
                continue;
            }

            let (rx, tx) = now.get(&name).copied().unwrap_or((0, 0));
            let (last_rx, last_tx) = self.last.get(&name).copied().unwrap_or((rx, tx));
            self.last.insert(name.clone(), (rx, tx));

            group
                .sensors
                .push(SensorValue::new(format!("{} RX", name), rx.saturating_sub(last_rx) as f64, SensorUnit::BytesPerSec));
            group
                .sensors
                .push(SensorValue::new(format!("{} TX", name), tx.saturating_sub(last_tx) as f64, SensorUnit::BytesPerSec));

            if let Some(speed) = read_u64(iface.join("speed")) {
                group.sensors.push(SensorValue::new(format!("{} Link Speed", name), speed as f64, SensorUnit::MHz));
            }
            if let Some(state) = read_trimmed(iface.join("operstate")) {
                group.sensors.push(SensorValue::new(
                    format!("{} State {}", name, state),
                    if state == "up" { 1.0 } else { 0.0 },
                    SensorUnit::Percent,
                ));
            }
        }

        group
    }
}

pub fn parse_net_dev(path: &Path) -> HashMap<String, (u64, u64)> {
    let mut out = HashMap::new();
    let Ok(content) = fs::read_to_string(path) else {
        return out;
    };

    for line in content.lines().skip(2) {
        let Some((iface, data)) = line.split_once(':') else {
            continue;
        };
        let cols: Vec<&str> = data.split_whitespace().collect();
        if cols.len() < 16 {
            continue;
        }
        let rx = cols[0].parse::<u64>().ok().unwrap_or(0);
        let tx = cols[8].parse::<u64>().ok().unwrap_or(0);
        out.insert(iface.trim().to_string(), (rx, tx));
    }

    out
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

fn read_u64(path: impl AsRef<Path>) -> Option<u64> {
    read_trimmed(path)?.parse::<u64>().ok()
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::parse_net_dev;

    #[test]
    fn parses_net_dev() {
        let tmp = TempDir::new().expect("tempdir");
        let p = tmp.path().join("dev");
        fs::write(
            &p,
            "Inter-|   Receive                                                |  Transmit\n face |bytes    packets errs drop fifo frame compressed multicast|bytes packets errs drop fifo colls carrier compressed\neth0: 100 0 0 0 0 0 0 0 200 0 0 0 0 0 0 0\n",
        )
        .expect("write dev");

        let parsed = parse_net_dev(&p);
        assert_eq!(parsed.get("eth0").copied().unwrap_or((0, 0)), (100, 200));
    }
}
