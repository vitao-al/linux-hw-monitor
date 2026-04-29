use gtk4 as gtk;
use gtk::prelude::*;

use crate::sensors::types::SensorGroup;

pub fn build_group_row(group: &SensorGroup) -> gtk::ListBoxRow {
    let row = gtk::ListBoxRow::new();
    let outer = gtk::Box::new(gtk::Orientation::Vertical, 8);

    let expander = adw::ExpanderRow::builder().title(&group.label).build();
    for sensor in &group.sensors {
        let pref = adw::ActionRow::builder()
            .title(&sensor.label)
            .subtitle(&format!("{:.2}", sensor.value))
            .build();
        expander.add_row(&pref);
    }

    outer.append(&expander);
    row.set_child(Some(&outer));
    row
}
