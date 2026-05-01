use adw::prelude::*;
use gettextrs::{bind_textdomain_codeset, bindtextdomain, setlocale, textdomain, LocaleCategory};
use gtk4::glib;

use crate::window::window::MainWindow;

const APP_ID: &str = "io.github.usuario.LinuxHWMonitor";

pub fn build_application() -> adw::Application {
    install_adwaita_warning_filter();
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

fn install_adwaita_warning_filter() {
    glib::log_set_default_handler(|domain, level, message| {
        let suppress = matches!(domain, Some("Adwaita"))
            && level == glib::LogLevel::Warning
            && message.contains("gtk-application-prefer-dark-theme");

        if suppress {
            return;
        }

        glib::log_default_handler(domain, level, Some(message));
    });
}
