use std::cell::RefCell;
use std::rc::Rc;

use adw::prelude::*;
use gtk4 as gtk;

use crate::config::AppConfig;
use crate::sensors::manager::SensorManager;
use crate::sensors::types::SensorData;
use crate::ui::gauge_widget::GaugeWidget;
use crate::ui::sensor_group::build_group_row;

pub struct MainWindow {
    window: adw::ApplicationWindow,
}

impl MainWindow {
    pub fn new(app: &adw::Application) -> Self {
        let manager = Rc::new(SensorManager::new());
        manager.start(AppConfig::default());

        let split = adw::NavigationSplitView::new();
        split.set_min_sidebar_width(240.0);
        split.set_max_sidebar_width(280.0);

        let categories = gtk::ListBox::new();
        categories.add_css_class("navigation-sidebar");
        let sidebar_scroll = gtk::ScrolledWindow::builder().child(&categories).vexpand(true).hexpand(true).build();
        let sidebar_page = adw::NavigationPage::builder().title("Sensors").child(&sidebar_scroll).build();

        let content_box = gtk::Box::new(gtk::Orientation::Vertical, 12);
        content_box.set_margin_top(12);
        content_box.set_margin_bottom(12);
        content_box.set_margin_start(12);
        content_box.set_margin_end(12);

        let gauge_row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        gauge_row.append(&GaugeWidget::new("CPU").widget);
        gauge_row.append(&GaugeWidget::new("GPU").widget);
        content_box.append(&gauge_row);

        let content_list = gtk::ListBox::new();
        content_list.set_selection_mode(gtk::SelectionMode::None);
        let content_scroll = gtk::ScrolledWindow::builder().child(&content_list).vexpand(true).hexpand(true).build();
        content_box.append(&content_scroll);

        let content_page = adw::NavigationPage::builder().title("Details").child(&content_box).build();

        split.set_sidebar(Some(&sidebar_page));
        split.set_content(Some(&content_page));

        let header = adw::HeaderBar::new();
        let title = gtk::Label::new(Some("Hardware Monitor"));
        title.add_css_class("title-1");
        header.set_title_widget(Some(&title));

        let pref_btn = gtk::Button::from_icon_name("emblem-system-symbolic");
        pref_btn.set_tooltip_text(Some("Preferences"));
        header.pack_end(&pref_btn);

        let about_btn = gtk::Button::from_icon_name("help-about-symbolic");
        about_btn.set_tooltip_text(Some("About"));
        header.pack_end(&about_btn);

        let export_menu = gtk::MenuButton::builder().icon_name("document-save-symbolic").tooltip_text("Export").build();
        let model = gio::Menu::new();
        model.append(Some("Export CSV"), Some("app.export-csv"));
        model.append(Some("Export JSON"), Some("app.export-json"));
        model.append(Some("Export Text"), Some("app.export-text"));
        export_menu.set_menu_model(Some(&model));
        header.pack_end(&export_menu);

        let toolbar = adw::ToolbarView::new();
        toolbar.add_top_bar(&header);
        toolbar.set_content(Some(&split));

        let window = adw::ApplicationWindow::builder()
            .application(app)
            .title("Linux HW Monitor")
            .default_width(1100)
            .default_height(720)
            .content(&toolbar)
            .build();

        let pref_parent = window.clone();
        pref_btn.connect_clicked(move |_| {
            let pref = adw::PreferencesWindow::new();
            pref.set_title(Some("Preferences"));
            pref.set_transient_for(Some(&pref_parent));

            let page = adw::PreferencesPage::new();
            let group = adw::PreferencesGroup::builder().title("General").build();

            let interval = adw::ComboRow::builder().title("Update interval").build();
            let interval_model = gtk::StringList::new(&["1s", "2s", "5s"]);
            interval.set_model(Some(&interval_model));
            interval.set_selected(0);

            let temp = adw::ComboRow::builder().title("Temperature unit").build();
            let temp_model = gtk::StringList::new(&["Celsius", "Fahrenheit"]);
            temp.set_model(Some(&temp_model));

            let data_unit = adw::ComboRow::builder().title("Data unit").build();
            let data_model = gtk::StringList::new(&["SI", "IEC"]);
            data_unit.set_model(Some(&data_model));

            let notify = adw::SwitchRow::builder().title("Critical temperature notifications").active(true).build();

            group.add(&interval);
            group.add(&temp);
            group.add(&data_unit);
            group.add(&notify);
            page.add(&group);
            pref.add(&page);
            pref.present();
        });

        let about_parent = window.clone();
        about_btn.connect_clicked(move |_| {
            let about = adw::AboutWindow::builder()
                .application_name("Linux HW Monitor")
                .application_icon("io.github.usuario.LinuxHWMonitor")
                .developer_name("usuario")
                .version("1.0.0")
                .website("https://github.com/usuario/linux-hw-monitor")
                .issue_url("https://github.com/usuario/linux-hw-monitor/issues")
                .transient_for(&about_parent)
                .build();
            about.present();
        });

        install_actions(app, Rc::clone(&manager));

        let selected_group = Rc::new(RefCell::new(String::from("cpu")));
        let manager_rx = Rc::new(RefCell::new(manager.rx.clone()));

        let categories_clone = categories.clone();
        let selected_group_clone = Rc::clone(&selected_group);
        categories.connect_row_selected(move |_, row| {
            if let Some(row) = row {
                if let Some(id) = row.widget_name().strip_prefix("row-") {
                    *selected_group_clone.borrow_mut() = id.to_string();
                }
            }
            categories_clone.invalidate_filter();
        });

        let categories_ref = categories.clone();
        let content_ref = content_list.clone();
        let selected_ref = Rc::clone(&selected_group);

        glib::timeout_add_seconds_local(1, move || {
            let data = manager_rx.borrow().borrow().clone();
            rebuild_sidebar(&categories_ref, &data);
            rebuild_content(&content_ref, &data, &selected_ref.borrow());
            glib::ControlFlow::Continue
        });

        Self { window }
    }

    pub fn present(&self) {
        self.window.present();
    }
}

fn rebuild_sidebar(list: &gtk::ListBox, data: &SensorData) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }

    for group in &data.groups {
        let row = gtk::ListBoxRow::new();
        row.set_widget_name(&format!("row-{}", group.id));

        let line = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        line.set_margin_top(6);
        line.set_margin_bottom(6);
        line.set_margin_start(8);
        line.set_margin_end(8);

        let icon = gtk::Image::from_icon_name(&group.icon);
        icon.set_pixel_size(16);
        let label = gtk::Label::new(Some(&group.label));
        label.set_hexpand(true);
        label.set_xalign(0.0);

        let summary = group
            .sensors
            .first()
            .map(|s| format!("{:.1}", s.value))
            .unwrap_or_else(|| "N/A".to_string());
        let value = gtk::Label::new(Some(&summary));
        value.add_css_class("dim-label");

        line.append(&icon);
        line.append(&label);
        line.append(&value);
        row.set_child(Some(&line));
        list.append(&row);
    }
}

fn rebuild_content(list: &gtk::ListBox, data: &SensorData, selected: &str) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }

    let group = data.groups.iter().find(|g| g.id == selected).or_else(|| data.groups.first());
    let Some(group) = group else {
        return;
    };

    let header = gtk::ListBoxRow::new();
    let title = gtk::Label::new(Some(&group.label));
    title.set_xalign(0.0);
    title.add_css_class("title-2");
    header.set_child(Some(&title));
    list.append(&header);

    let row = build_group_row(group);
    list.append(&row);
}

fn install_actions(app: &adw::Application, manager: Rc<SensorManager>) {
    let csv_action = gio::SimpleAction::new("export-csv", None);
    let manager_csv = Rc::clone(&manager);
    csv_action.connect_activate(move |_, _| {
        let _ = std::fs::write("linux-hw-monitor-export.csv", manager_csv.export_csv());
    });
    app.add_action(&csv_action);

    let json_action = gio::SimpleAction::new("export-json", None);
    let manager_json = Rc::clone(&manager);
    json_action.connect_activate(move |_, _| {
        let _ = std::fs::write("linux-hw-monitor-export.json", manager_json.export_json());
    });
    app.add_action(&json_action);

    let txt_action = gio::SimpleAction::new("export-text", None);
    txt_action.connect_activate(move |_, _| {
        let _ = std::fs::write("linux-hw-monitor-export.txt", manager.export_text());
    });
    app.add_action(&txt_action);
}
