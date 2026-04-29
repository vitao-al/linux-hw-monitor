use std::collections::VecDeque;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SensorValue {
    pub label: String,
    pub value: f64,
    pub unit: SensorUnit,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub critical: Option<f64>,
    pub history: VecDeque<f64>,
}

impl SensorValue {
    pub fn new(label: impl Into<String>, value: f64, unit: SensorUnit) -> Self {
        let mut history = VecDeque::with_capacity(60);
        history.push_back(value);
        Self {
            label: label.into(),
            value,
            unit,
            min: None,
            max: None,
            critical: None,
            history,
        }
    }

    pub fn with_range(mut self, min: Option<f64>, max: Option<f64>, critical: Option<f64>) -> Self {
        self.min = min;
        self.max = max;
        self.critical = critical;
        self
    }

    pub fn push_history(&mut self, value: f64) {
        if self.history.len() == 60 {
            let _ = self.history.pop_front();
        }
        self.history.push_back(value);
        self.value = value;
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum SensorUnit {
    Celsius,
    Fahrenheit,
    Volt,
    Millivolt,
    Watt,
    Milliwatt,
    MHz,
    GHz,
    Percent,
    RPM,
    Bytes,
    BytesPerSec,
    MB,
    GB,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SensorGroup {
    pub id: String,
    pub label: String,
    pub icon: String,
    pub sensors: Vec<SensorValue>,
    pub subgroups: Vec<SensorGroup>,
}

impl SensorGroup {
    pub fn new(id: impl Into<String>, label: impl Into<String>, icon: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            icon: icon.into(),
            sensors: Vec::new(),
            subgroups: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct SensorData {
    pub timestamp: String,
    pub hostname: String,
    pub groups: Vec<SensorGroup>,
}
