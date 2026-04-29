use gtk4 as gtk;
use gtk::prelude::*;

use crate::sensors::types::{SensorUnit, SensorValue};

pub fn build_sensor_row(sensor: &SensorValue) -> gtk::ListBoxRow {
    let row = gtk::ListBoxRow::new();
    let container = gtk::Box::new(gtk::Orientation::Vertical, 4);
    container.set_margin_top(6);
    container.set_margin_bottom(6);
    container.set_margin_start(8);
    container.set_margin_end(8);

    let top = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let label = gtk::Label::new(Some(&sensor.label));
    label.set_xalign(0.0);
    label.set_hexpand(true);

    let value = gtk::Label::new(Some(&format!("{:.2} {}", sensor.value, unit_label(sensor.unit))));
    let min = gtk::Label::new(Some(&format!("min {}", sensor.min.unwrap_or(sensor.value))));
    min.add_css_class("dim-label");
    let max = gtk::Label::new(Some(&format!("max {}", sensor.max.unwrap_or(sensor.value))));
    max.add_css_class("dim-label");

    top.append(&label);
    top.append(&value);
    top.append(&min);
    top.append(&max);

    let progress = gtk::ProgressBar::new();
    let fraction = sensor
        .max
        .filter(|m| *m > 0.0)
        .map(|m| (sensor.value / m).clamp(0.0, 1.0))
        .unwrap_or_else(|| (sensor.value / 100.0).clamp(0.0, 1.0));
    progress.set_fraction(fraction);

    if fraction >= 0.85 {
        progress.add_css_class("error");
    } else if fraction >= 0.7 {
        progress.add_css_class("warning");
    } else {
        progress.add_css_class("success");
    }

    container.append(&top);
    container.append(&progress);
    row.set_child(Some(&container));

    row
}

fn unit_label(unit: SensorUnit) -> &'static str {
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
