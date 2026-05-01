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
    let dialog = gtk::FileDialog::builder()
        .title("Exportar dados")
        .initial_name(suggested_name)
        .modal(true)
        .build();

    let parent_weak = parent.downgrade();
    dialog.save(
        parent_weak.upgrade().as_ref(),
        None::<&gio::Cancellable>,
        move |result| {
            if let Ok(file) = result {
                if let Some(path) = file.path() {
                    if let Err(e) = std::fs::write(&path, &contents) {
                        // Show an error toast/dialog if write fails.
                        if let Some(win) = parent_weak.upgrade() {
                            let alert = adw::AlertDialog::builder()
                                .heading("Falha ao salvar")
                                .body(&format!("Não foi possível salvar em {:?}:\n{}", path, e))
                                .build();
                            alert.add_response("ok", "OK");
                            alert.present(&win);
                        }
                    }
                }
            }
        },
    );
}
