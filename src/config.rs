use std::path::PathBuf;

#[derive(Clone, Debug)]
pub enum TemperatureUnit {
    Celsius,
    Fahrenheit,
}

#[derive(Clone, Debug)]
pub enum DataUnit {
    Si,
    Iec,
}

#[derive(Clone, Debug)]
pub struct PathConfig {
    pub sys_root: PathBuf,
    pub proc_root: PathBuf,
}

impl Default for PathConfig {
    fn default() -> Self {
        Self {
            sys_root: PathBuf::from("/sys"),
            proc_root: PathBuf::from("/proc"),
        }
    }
}

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub update_interval_secs: u64,
    pub temperature_unit: TemperatureUnit,
    pub data_unit: DataUnit,
    pub path_config: PathConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            update_interval_secs: 1,
            temperature_unit: TemperatureUnit::Celsius,
            data_unit: DataUnit::Si,
            path_config: PathConfig::default(),
        }
    }
}
