use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use nix::sys::statvfs::statvfs;

use crate::config::PathConfig;

use super::types::{SensorGroup, SensorUnit, SensorValue};

#[derive(Default)]
pub struct StorageSensor {
    last_counters: HashMap<String, (u64, u64)>,
}

impl StorageSensor {
    pub fn collect(&mut self, paths: &PathConfig) -> SensorGroup {
        let mut group = SensorGroup::new("storage", "Storage", "drive-harddisk-symbolic");
        let block_root = paths.sys_root.join("block");
        let diskstats = parse_diskstats(&paths.proc_root.join("diskstats"));

        for dev in iter_dirs(&block_root) {
            let Some(name) = dev.file_name().and_then(|n| n.to_str()).map(ToOwned::to_owned) else {
                continue;
            };
            if is_virtual_disk(&name) {
                continue;
            }

            if let Some((read_sectors, write_sectors)) = diskstats.get(&name) {
                let (last_r, last_w) = self.last_counters.get(&name).copied().unwrap_or((0, 0));
                let read_bps = (read_sectors.saturating_sub(last_r) as f64) * 512.0;
                let write_bps = (write_sectors.saturating_sub(last_w) as f64) * 512.0;
                self.last_counters.insert(name.clone(), (*read_sectors, *write_sectors));

                group.sensors.push(SensorValue::new(format!("{} Read", name), read_bps, SensorUnit::BytesPerSec));
                group
                    .sensors
                    .push(SensorValue::new(format!("{} Write", name), write_bps, SensorUnit::BytesPerSec));
            }

            let vendor = read_trimmed(dev.join("device/vendor")).unwrap_or_else(|| "Unknown".to_string());
            let model = read_trimmed(dev.join("device/model")).unwrap_or_else(|| "Disk".to_string());
            group.sensors.push(SensorValue::new(
                format!("{} {} Present", vendor.trim(), model.trim()),
                1.0,
                SensorUnit::Percent,
            ));

            if let Ok(vfs) = statvfs(Path::new("/")) {
                let blocks = vfs.blocks();
                let free = vfs.blocks_free();
                if blocks > 0 {
                    let used_percent = ((blocks - free) as f64 / blocks as f64) * 100.0;
                    group
                        .sensors
                        .push(SensorValue::new("Filesystem Usage", used_percent, SensorUnit::Percent));
                }
            }
        }

        group
    }
}

pub fn parse_diskstats(path: &Path) -> HashMap<String, (u64, u64)> {
    let mut out = HashMap::new();
    let Ok(content) = fs::read_to_string(path) else {
        return out;
    };

    for line in content.lines() {
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.len() < 14 {
            continue;
        }
        let name = cols[2].to_string();
        let read_sectors = cols[5].parse::<u64>().ok().unwrap_or(0);
        let write_sectors = cols[9].parse::<u64>().ok().unwrap_or(0);
        out.insert(name, (read_sectors, write_sectors));
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

fn is_virtual_disk(name: &str) -> bool {
    ["loop", "ram", "zram", "dm-"]
        .iter()
        .any(|prefix| name.starts_with(prefix))
}

fn read_trimmed(path: impl AsRef<Path>) -> Option<String> {
    let Ok(s) = fs::read_to_string(path) else {
        return None;
    };
    Some(s.trim().to_string())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::parse_diskstats;

    #[test]
    fn parses_diskstats() {
        let tmp = TempDir::new().expect("tempdir");
        let p = tmp.path().join("diskstats");
        fs::write(&p, "8 0 sda 10 0 100 0 5 0 200 0 0 0 0 0\n").expect("write diskstats");
        let parsed = parse_diskstats(&p);
        assert_eq!(parsed.get("sda").copied().unwrap_or((0, 0)).0, 100);
    }
}
