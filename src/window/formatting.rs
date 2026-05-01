use crate::sensors::types::{SensorData, SensorUnit, SensorValue};

pub(crate) fn extract_group_percent(data: &SensorData, group_id: &str) -> f64 {
    data.groups
        .iter()
        .find(|g| g.id == group_id)
        .and_then(|group| {
            group
                .sensors
                .iter()
                .find(|s| matches!(s.unit, SensorUnit::Percent))
                .map(|s| s.value)
                .or_else(|| {
                    group.sensors.first().map(|s| match (s.min, s.max, s.critical) {
                        (Some(min), Some(max), _) if max > min => ((s.value - min) / (max - min)) * 100.0,
                        (_, _, Some(critical)) if critical > 0.0 => (s.value / critical) * 100.0,
                        _ => s.value,
                    })
                })
        })
        .unwrap_or(0.0)
        .clamp(0.0, 100.0)
}

pub(crate) fn format_sidebar_value(sensor: &SensorValue) -> String {
    format!("{:.1} {}", sensor.value, unit_suffix(sensor.unit))
}

pub(crate) fn unit_suffix(unit: SensorUnit) -> &'static str {
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
