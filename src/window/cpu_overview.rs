use std::fs;

use gtk4 as gtk;
use gtk::prelude::*;

use crate::sensors::types::{SensorData, SensorGroup, SensorUnit};
use crate::window::formatting::format_sidebar_value;

pub(crate) fn rebuild_cpu_overview(frame: &gtk::Frame, data: &SensorData) {
    let Some(cpu) = data.groups.iter().find(|g| g.id == "cpu") else {
        frame.set_child(None::<&gtk::Widget>);
        return;
    };

    let model = read_cpu_model();
    let usage = cpu
        .sensors
        .iter()
        .find(|s| s.label == "CPU Usage")
        .map(format_sidebar_value)
        .unwrap_or_else(|| "N/A".to_string());

    let avg_clock = average_clock(cpu).unwrap_or_else(|| "N/A".to_string());
    let (l1, l2, l3) = read_cpu_cache_sizes();

    let root = gtk::Box::new(gtk::Orientation::Vertical, 6);
    root.set_margin_top(8);
    root.set_margin_bottom(8);
    root.set_margin_start(10);
    root.set_margin_end(10);

    let title = gtk::Label::new(Some(&model));
    title.set_xalign(0.0);
    title.add_css_class("title-4");
    root.append(&title);

    append_overview_line(&root, "Utilization", &usage);
    append_overview_line(&root, "Speed", &avg_clock);
    append_overview_line(&root, "L1 cache", &l1);
    append_overview_line(&root, "L2 cache", &l2);
    append_overview_line(&root, "L3 cache", &l3);

    frame.set_child(Some(&root));
}

fn append_overview_line(parent: &gtk::Box, key: &str, value: &str) {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let label = gtk::Label::new(Some(key));
    label.set_xalign(0.0);
    label.set_hexpand(true);
    label.add_css_class("dim-label");
    let value_lbl = gtk::Label::new(Some(value));
    value_lbl.set_xalign(1.0);
    value_lbl.add_css_class("monospace");
    row.append(&label);
    row.append(&value_lbl);
    parent.append(&row);
}

fn average_clock(cpu: &SensorGroup) -> Option<String> {
    let mut total = 0.0;
    let mut count = 0u32;
    for sensor in &cpu.sensors {
        if sensor.label.ends_with("Clock") && matches!(sensor.unit, SensorUnit::MHz) {
            total += sensor.value;
            count = count.saturating_add(1);
        }
    }

    if count == 0 {
        None
    } else {
        Some(format!("{:.2} GHz", (total / count as f64) / 1000.0))
    }
}

fn read_cpu_model() -> String {
    let Ok(contents) = fs::read_to_string("/proc/cpuinfo") else {
        return "CPU".to_string();
    };

    for line in contents.lines() {
        if let Some((_, value)) = line.split_once(':') {
            if line.starts_with("model name") {
                return value.trim().to_string();
            }
        }
    }
    "CPU".to_string()
}

fn read_cpu_cache_sizes() -> (String, String, String) {
    let mut l1 = String::from("N/A");
    let mut l2 = String::from("N/A");
    let mut l3 = String::from("N/A");

    let base = "/sys/devices/system/cpu/cpu0/cache";
    for idx in 0..8 {
        let level = fs::read_to_string(format!("{}/index{}/level", base, idx))
            .ok()
            .map(|s| s.trim().to_string());
        let size = fs::read_to_string(format!("{}/index{}/size", base, idx))
            .ok()
            .map(|s| s.trim().to_string());

        let (Some(level), Some(size)) = (level, size) else {
            continue;
        };

        match level.as_str() {
            "1" if l1 == "N/A" => l1 = size,
            "2" if l2 == "N/A" => l2 = size,
            "3" if l3 == "N/A" => l3 = size,
            _ => {}
        }
    }

    (l1, l2, l3)
}
