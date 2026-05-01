use gtk4 as gtk;
use gtk::prelude::*;

use crate::sensors::types::{SensorGroup, SensorUnit, SensorValue};

pub fn build_group_row(group: &SensorGroup) -> gtk::ListBoxRow {
    let row = gtk::ListBoxRow::new();
    let outer = gtk::Box::new(gtk::Orientation::Vertical, 10);

    let header = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    header.set_margin_top(4);
    header.set_margin_bottom(4);
    header.set_margin_start(8);
    header.set_margin_end(8);
    let col_sensor = gtk::Label::new(Some("Sensor"));
    col_sensor.set_hexpand(true);
    col_sensor.set_xalign(0.0);
    col_sensor.add_css_class("dim-label");
    let col_value = gtk::Label::new(Some("Valor"));
    col_value.set_xalign(1.0);
    col_value.add_css_class("dim-label");
    header.append(&col_sensor);
    header.append(&col_value);
    outer.append(&header);

    append_group_rows(&outer, group, false);
    row.set_child(Some(&outer));
    row
}

fn append_group_rows(container: &gtk::Box, group: &SensorGroup, show_group_header: bool) {
    if show_group_header {
        let subgroup = gtk::Label::new(Some(&group.label));
        subgroup.set_xalign(0.0);
        subgroup.add_css_class("heading");
        container.append(&subgroup);
    }

    for sensor in &group.sensors {
        let line = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        line.add_css_class("card");
        line.set_margin_top(2);
        line.set_margin_bottom(2);
        line.set_margin_start(8);
        line.set_margin_end(8);

        let label = gtk::Label::new(Some(&sensor.label));
        label.set_hexpand(true);
        label.set_xalign(0.0);

        let value = gtk::Label::new(Some(&format_sensor_value(sensor)));
        value.set_xalign(1.0);
        value.add_css_class("monospace");

        line.append(&label);
        line.append(&value);
        container.append(&line);
    }

    for subgroup in &group.subgroups {
        append_group_rows(container, subgroup, true);
    }
}

fn format_sensor_value(sensor: &SensorValue) -> String {
    match sensor.unit {
        SensorUnit::Bytes | SensorUnit::BytesPerSec => human_bytes(sensor.value, sensor.unit),
        _ => format!("{:.2} {}", sensor.value, unit_suffix(sensor.unit)),
    }
}

fn human_bytes(value: f64, unit: SensorUnit) -> String {
    let per_sec = matches!(unit, SensorUnit::BytesPerSec);
    let suffix = if per_sec { "/s" } else { "" };
    let abs = value.abs();
    if abs >= 1024.0_f64 * 1024.0_f64 * 1024.0_f64 {
        format!("{:.2} GB{}", value / (1024.0_f64 * 1024.0_f64 * 1024.0_f64), suffix)
    } else if abs >= 1024.0_f64 * 1024.0_f64 {
        format!("{:.2} MB{}", value / (1024.0_f64 * 1024.0_f64), suffix)
    } else if abs >= 1024.0_f64 {
        format!("{:.2} KB{}", value / 1024.0_f64, suffix)
    } else {
        format!("{:.0} B{}", value, suffix)
    }
}

fn unit_suffix(unit: SensorUnit) -> &'static str {
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
