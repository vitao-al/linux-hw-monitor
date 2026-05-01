use gtk4 as gtk;

pub(crate) fn install_visual_defaults() {
    let css = gtk::CssProvider::new();
    css.load_from_data(
        "viewswitcher button { min-height: 34px; padding: 4px 10px; }\
         viewswitcher button label { font-weight: 600; opacity: 0.98; }\
         .dim-label { opacity: 0.85; }\
         .monospace { opacity: 0.95; }\
         image { opacity: 0.95; }\
         .badge { border-radius: 6px; padding: 2px 8px;\
                  font-size: 10px; font-weight: 700;\
                  min-width: 48px; }\
         .badge-low      { background: #26a269; color: #fff; }\
         .badge-med      { background: #cd9309; color: #fff; }\
         .badge-high     { background: #e66100; color: #fff; }\
         .badge-extreme  { background: #c01c28; color: #fff; }",
    );

    if let Some(display) = gtk::gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &css,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}
