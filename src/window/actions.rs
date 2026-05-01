use std::rc::Rc;

use adw::prelude::*;
use gtk4 as gtk;

use crate::sensors::manager::SensorManager;

pub(crate) fn install_actions(app: &adw::Application, manager: Rc<SensorManager>, parent: adw::ApplicationWindow) {
    let csv_action = gio::SimpleAction::new("export-csv", None);
    let manager_csv = Rc::clone(&manager);
    let parent_csv = parent.clone();
    csv_action.connect_activate(move |_, _| {
        save_with_dialog(&parent_csv, "linux-hw-monitor-export.csv", manager_csv.export_csv());
    });
    app.add_action(&csv_action);

    let json_action = gio::SimpleAction::new("export-json", None);
    let manager_json = Rc::clone(&manager);
    let parent_json = parent.clone();
    json_action.connect_activate(move |_, _| {
        save_with_dialog(&parent_json, "linux-hw-monitor-export.json", manager_json.export_json());
    });
    app.add_action(&json_action);

    let txt_action = gio::SimpleAction::new("export-text", None);
    let parent_txt = parent;
    txt_action.connect_activate(move |_, _| {
        save_with_dialog(&parent_txt, "linux-hw-monitor-export.txt", manager.export_text());
    });
    app.add_action(&txt_action);
}

fn save_with_dialog(parent: &adw::ApplicationWindow, suggested_name: &str, contents: String) {
    let dialog = gtk::FileChooserNative::builder()
        .title("Export data")
        .transient_for(parent)
        .action(gtk::FileChooserAction::Save)
        .accept_label("Save")
        .cancel_label("Cancel")
        .build();
    dialog.set_current_name(suggested_name);

    dialog.connect_response(move |dlg, response| {
        if response == gtk::ResponseType::Accept {
            if let Some(file) = dlg.file() {
                if let Some(path) = file.path() {
                    let _ = std::fs::write(path, &contents);
                }
            }
        }
        dlg.destroy();
    });

    dialog.show();
}
