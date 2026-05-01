use gtk4 as gtk;

pub(crate) fn install_visual_defaults() {
    let css = gtk::CssProvider::new();
    css.load_from_data(
        "viewswitcher button { min-height: 34px; padding: 4px 10px; }\
         viewswitcher button label { font-weight: 600; opacity: 0.98; }\
         .dim-label { opacity: 0.85; }\
         .monospace { opacity: 0.95; }\
         image { opacity: 0.95; }",
    );

    if let Some(display) = gtk::gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &css,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}
