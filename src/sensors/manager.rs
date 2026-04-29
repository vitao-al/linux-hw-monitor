use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::thread;

use chrono::Utc;
use tokio::runtime::Builder;
use tokio::sync::watch;
use tokio::time::{interval, Duration};

use crate::config::AppConfig;

use super::battery::BatterySensor;
use super::cpu::CpuSensor;
use super::fans::FansSensor;
use super::gpu::GpuSensor;
use super::memory::MemorySensor;
use super::motherboard::MotherboardSensor;
use super::network::NetworkSensor;
use super::storage::StorageSensor;
use super::types::{SensorData, SensorGroup, SensorUnit};

pub struct SensorManager {
    shared: Arc<RwLock<SensorData>>,
    tx: watch::Sender<SensorData>,
    pub rx: watch::Receiver<SensorData>,
}

impl SensorManager {
    pub fn new() -> Self {
        let initial = SensorData::default();
        let (tx, rx) = watch::channel(initial.clone());
        Self {
            shared: Arc::new(RwLock::new(initial)),
            tx,
            rx,
        }
    }

    pub fn shared_data(&self) -> Arc<RwLock<SensorData>> {
        Arc::clone(&self.shared)
    }

    pub fn start(&self, config: AppConfig) {
        let tx = self.tx.clone();
        let shared = Arc::clone(&self.shared);
        thread::spawn(move || {
            let Ok(rt) = Builder::new_multi_thread().enable_time().build() else {
                return;
            };
            rt.block_on(async move {
                let mut ticker = interval(Duration::from_secs(config.update_interval_secs.max(1)));
                let mut cpu = CpuSensor::default();
                let mut storage = StorageSensor::default();
                let mut network = NetworkSensor::default();

                loop {
                    ticker.tick().await;

                    let mut data = SensorData {
                        timestamp: Utc::now().to_rfc3339(),
                        hostname: read_hostname(),
                        groups: Vec::new(),
                    };

                    let path_cfg = &config.path_config;
                    let groups: Vec<SensorGroup> = vec![
                        cpu.collect(path_cfg),
                        GpuSensor::collect(path_cfg),
                        MemorySensor::collect(path_cfg),
                        storage.collect(path_cfg),
                        BatterySensor::collect(path_cfg),
                        network.collect(path_cfg),
                        FansSensor::collect(path_cfg),
                        MotherboardSensor::collect(path_cfg),
                    ];

                    data.groups = groups;

                    if let Ok(mut lock) = shared.write() {
                        *lock = data.clone();
                    }
                    let _ = tx.send(data);
                }
            });
        });
    }

    pub fn export_json(&self) -> String {
        if let Ok(lock) = self.shared.read() {
            return serde_json::to_string_pretty(&*lock).unwrap_or_else(|_| "{}".to_string());
        }
        "{}".to_string()
    }

    pub fn export_csv(&self) -> String {
        let Ok(lock) = self.shared.read() else {
            return "timestamp,sensor,value,unit,min,max\n".to_string();
        };

        let mut out = String::from("timestamp,sensor,value,unit,min,max\n");
        for group in &lock.groups {
            append_group_csv(&mut out, &lock.timestamp, group);
        }
        out
    }

    pub fn export_text(&self) -> String {
        let Ok(lock) = self.shared.read() else {
            return String::new();
        };

        let mut out = String::new();
        for group in &lock.groups {
            out.push_str(&format!("{}\n", group.id));
            for s in &group.sensors {
                out.push_str(&format!("  {}: {:.2} {}\n", s.label, s.value, unit_to_str(s.unit)));
            }
            out.push('\n');
        }
        out
    }
}

fn append_group_csv(out: &mut String, ts: &str, group: &SensorGroup) {
    for sensor in &group.sensors {
        out.push_str(&format!(
            "{},{},{:.4},{},{},{}\n",
            ts,
            sensor.label,
            sensor.value,
            unit_to_str(sensor.unit),
            sensor.min.map(|v| v.to_string()).unwrap_or_default(),
            sensor.max.map(|v| v.to_string()).unwrap_or_default()
        ));
    }
    for sub in &group.subgroups {
        append_group_csv(out, ts, sub);
    }
}

fn unit_to_str(unit: SensorUnit) -> &'static str {
    match unit {
        SensorUnit::Celsius => "C",
        SensorUnit::Fahrenheit => "F",
        SensorUnit::Volt => "V",
        SensorUnit::Millivolt => "mV",
        SensorUnit::Watt => "W",
        SensorUnit::Milliwatt => "mW",
        SensorUnit::MHz => "MHz",
        SensorUnit::GHz => "GHz",
        SensorUnit::Percent => "%",
        SensorUnit::RPM => "RPM",
        SensorUnit::Bytes => "B",
        SensorUnit::BytesPerSec => "B/s",
        SensorUnit::MB => "MB",
        SensorUnit::GB => "GB",
    }
}

fn read_hostname() -> String {
    std::fs::read_to_string("/etc/hostname")
        .map(|s| s.trim().to_string())
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unknown".to_string())
}

pub fn summary_map(data: &SensorData) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for group in &data.groups {
        if let Some(first) = group.sensors.first() {
            map.insert(group.id.clone(), format!("{:.1}", first.value));
        }
    }
    map
}

#[cfg(test)]
mod tests {
    use super::SensorManager;

    #[test]
    fn exports_headers() {
        let manager = SensorManager::new();
        let csv = manager.export_csv();
        assert!(csv.starts_with("timestamp,sensor,value,unit,min,max"));
    }
}
