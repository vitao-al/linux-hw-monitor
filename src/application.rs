use adw::prelude::*;
use gettextrs::{bind_textdomain_codeset, bindtextdomain, setlocale, textdomain, LocaleCategory};

use crate::window::window::MainWindow;

const APP_ID: &str = "io.github.usuario.LinuxHWMonitor";

pub fn build_application() -> adw::Application {
    setup_i18n();

    let app = adw::Application::builder().application_id(APP_ID).build();

    app.connect_activate(|app| {
        let win = MainWindow::new(app);
        win.present();
    });

    app
}

fn setup_i18n() {
    let _ = setlocale(LocaleCategory::LcAll, "");
    let _ = bindtextdomain(APP_ID, "/usr/share/locale");
    let _ = bind_textdomain_codeset(APP_ID, "UTF-8");
    let _ = textdomain(APP_ID);
}
