use std::cell::{Cell, RefCell};
use std::rc::Rc;

use adw::prelude::*;
use gtk4 as gtk;

use crate::config::AppConfig;
use crate::sensors::manager::SensorManager;
use crate::sensors::types::{SensorData, SensorValue};
use crate::ui::gauge_widget::GaugeWidget;
use crate::ui::graph_widget::build_graph_widget;
use crate::window::actions::install_actions;
use crate::window::cpu_overview::rebuild_cpu_overview;
use crate::window::formatting::{extract_group_percent, format_sidebar_value};
use crate::window::icons::{best_icon_name, preferred_icon_for_group};
use crate::window::processes::{
    build_apps_page, build_services_page, rebuild_apps_list, rebuild_services_list,
};
use crate::window::style::install_visual_defaults;

pub struct MainWindow {
    window: adw::ApplicationWindow,
}

impl MainWindow {
    pub fn new(app: &adw::Application) -> Self {
        let manager = Rc::new(SensorManager::new());
        manager.start(AppConfig::default());
        install_visual_defaults();

        let split = adw::NavigationSplitView::new();
        split.set_min_sidebar_width(240.0);
        split.set_max_sidebar_width(280.0);

        let categories = gtk::ListBox::new();
        categories.add_css_class("navigation-sidebar");
        let sidebar_scroll = gtk::ScrolledWindow::builder()
            .child(&categories)
            .vexpand(true)
            .hexpand(true)
            .build();
        let sidebar_page = adw::NavigationPage::builder()
            .title("Sensors")
            .child(&sidebar_scroll)
            .build();

        let content_box = gtk::Box::new(gtk::Orientation::Vertical, 12);
        content_box.set_margin_top(12);
        content_box.set_margin_bottom(12);
        content_box.set_margin_start(12);
        content_box.set_margin_end(12);

        let gauge_row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        let cpu_gauge = Rc::new(GaugeWidget::new("CPU"));
        let gpu_gauge = Rc::new(GaugeWidget::new("GPU"));
        gauge_row.append(&cpu_gauge.widget);
        gauge_row.append(&gpu_gauge.widget);
        content_box.append(&gauge_row);

        let cpu_overview = gtk::Frame::new(None);
        cpu_overview.add_css_class("card");
        content_box.append(&cpu_overview);

        let content_panel = gtk::Box::new(gtk::Orientation::Vertical, 12);
        let content_scroll = gtk::ScrolledWindow::builder()
            .child(&content_panel)
            .vexpand(true)
            .hexpand(true)
            .build();
        content_box.append(&content_scroll);

        let content_page = adw::NavigationPage::builder()
            .title("Details")
            .child(&content_box)
            .build();

        split.set_sidebar(Some(&sidebar_page));
        split.set_content(Some(&content_page));

        let apps_page = build_apps_page();
        let apps_list = apps_page.0;
        let apps_summary = apps_page.1;

        let services_page = build_services_page();
        let services_list = services_page.0;
        let services_summary = services_page.1;

        let view_stack = adw::ViewStack::new();
        let perf_page = view_stack.add_titled(&split, Some("performance"), "Performance");
        let apps_page_stack = view_stack.add_titled(&apps_page.2, Some("apps"), "Apps");
        let services_page_stack =
            view_stack.add_titled(&services_page.2, Some("services"), "Services");

        perf_page.set_icon_name(Some(&best_icon_name(
            &["utilities-system-monitor-symbolic", "computer-symbolic"],
            "applications-system-symbolic",
        )));
        apps_page_stack.set_icon_name(Some(&best_icon_name(
            &[
                "application-x-executable-symbolic",
                "applications-system-symbolic",
            ],
            "applications-system-symbolic",
        )));
        services_page_stack.set_icon_name(Some(&best_icon_name(
            &["system-run-symbolic", "applications-system-symbolic"],
            "applications-system-symbolic",
        )));

        let header = adw::HeaderBar::new();
        let switcher = adw::ViewSwitcher::new();
        switcher.set_stack(Some(&view_stack));
        switcher.set_policy(adw::ViewSwitcherPolicy::Wide);
        switcher.set_hexpand(true);
        header.set_title_widget(Some(&switcher));

        let pref_btn = gtk::Button::from_icon_name("emblem-system-symbolic");
        pref_btn.set_tooltip_text(Some("Preferences"));
        header.pack_end(&pref_btn);

        let about_btn = gtk::Button::from_icon_name("help-about-symbolic");
        about_btn.set_tooltip_text(Some("About"));
        header.pack_end(&about_btn);

        let export_menu = gtk::MenuButton::builder()
            .icon_name("document-save-symbolic")
            .tooltip_text("Export")
            .build();
        let model = gio::Menu::new();
        model.append(Some("Export CSV"), Some("app.export-csv"));
        model.append(Some("Export JSON"), Some("app.export-json"));
        model.append(Some("Export Text"), Some("app.export-text"));
        export_menu.set_menu_model(Some(&model));
        header.pack_end(&export_menu);

        let toolbar = adw::ToolbarView::new();
        toolbar.add_top_bar(&header);
        toolbar.set_content(Some(&view_stack));

        let window = adw::ApplicationWindow::builder()
            .application(app)
            .title("Linux HW Monitor")
            .default_width(1100)
            .default_height(720)
            .content(&toolbar)
            .build();

        let style_manager = adw::StyleManager::default();
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

            let notify = adw::SwitchRow::builder()
                .title("Critical temperature notifications")
                .active(true)
                .build();

            let theme = adw::ComboRow::builder().title("Theme").build();
            let theme_model = gtk::StringList::new(&["System", "Light", "Dark"]);
            theme.set_model(Some(&theme_model));
            let selected_theme = match style_manager.color_scheme() {
                adw::ColorScheme::ForceLight => 1,
                adw::ColorScheme::ForceDark => 2,
                _ => 0,
            };
            theme.set_selected(selected_theme);
            let style_manager_ref = style_manager.clone();
            theme.connect_selected_notify(move |row| {
                let scheme = match row.selected() {
                    1 => adw::ColorScheme::ForceLight,
                    2 => adw::ColorScheme::ForceDark,
                    _ => adw::ColorScheme::Default,
                };
                style_manager_ref.set_color_scheme(scheme);
            });

            group.add(&interval);
            group.add(&temp);
            group.add(&data_unit);
            group.add(&notify);
            group.add(&theme);
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

        install_actions(app, Rc::clone(&manager), window.clone());

        let selected_group = Rc::new(RefCell::new(String::from("cpu")));
        let manager_rx = Rc::new(RefCell::new(manager.rx.clone()));

        let selected_group_clone = Rc::clone(&selected_group);
        categories.connect_row_selected(move |_, row| {
            if let Some(row) = row {
                if let Some(id) = row.widget_name().strip_prefix("row-") {
                    *selected_group_clone.borrow_mut() = id.to_string();
                }
            }
        });

        let categories_ref = categories.clone();
        let content_ref = content_panel.clone();
        let cpu_overview_ref = cpu_overview.clone();
        let selected_ref = Rc::clone(&selected_group);
        let cpu_gauge_ref = Rc::clone(&cpu_gauge);
        let gpu_gauge_ref = Rc::clone(&gpu_gauge);
        let apps_list_ref = apps_list.clone();
        let apps_summary_ref = apps_summary.clone();
        let services_list_ref = services_list.clone();
        let services_summary_ref = services_summary.clone();
        let refresh_tick = Rc::new(Cell::new(0u32));
        let refresh_tick_ref = Rc::clone(&refresh_tick);

        rebuild_apps_list(&apps_list, &apps_summary);
        rebuild_services_list(&services_list, &services_summary);

        glib::timeout_add_seconds_local(1, move || {
            let data = manager_rx.borrow().borrow().clone();
            let selected = selected_ref.borrow().clone();
            rebuild_sidebar(&categories_ref, &data, &selected);
            rebuild_content(&content_ref, &data, &selected);
            rebuild_cpu_overview(&cpu_overview_ref, &data);
            cpu_gauge_ref.set_value_percent(extract_group_percent(&data, "cpu"));
            gpu_gauge_ref.set_value_percent(extract_group_percent(&data, "gpu"));

            let next_tick = refresh_tick_ref.get().wrapping_add(1);
            refresh_tick_ref.set(next_tick);
            if next_tick == 1 || next_tick % 5 == 0 {
                rebuild_apps_list(&apps_list_ref, &apps_summary_ref);
                rebuild_services_list(&services_list_ref, &services_summary_ref);
            }

            glib::ControlFlow::Continue
        });

        Self { window }
    }

    pub fn present(&self) {
        self.window.present();
    }
}

fn rebuild_sidebar(list: &gtk::ListBox, data: &SensorData, selected: &str) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }

    let mut selected_row: Option<gtk::ListBoxRow> = None;

    for group in &data.groups {
        let row = gtk::ListBoxRow::new();
        row.set_widget_name(&format!("row-{}", group.id));

        let line = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        line.set_margin_top(6);
        line.set_margin_bottom(6);
        line.set_margin_start(8);
        line.set_margin_end(8);

        let icon_name = preferred_icon_for_group(group);
        let icon = gtk::Image::from_icon_name(&icon_name);
        icon.set_pixel_size(16);
        icon.set_opacity(0.9);

        let history = group
            .sensors
            .first()
            .map(|s| s.history.iter().copied().collect::<Vec<_>>())
            .unwrap_or_default();
        let sparkline = build_graph_widget(&history, 76, 32);
        let label = gtk::Label::new(Some(&group.label));
        label.set_hexpand(true);
        label.set_xalign(0.0);

        let summary = group
            .sensors
            .first()
            .map(format_sidebar_value)
            .unwrap_or_else(|| "N/A".to_string());
        let value = gtk::Label::new(Some(&summary));
        value.add_css_class("dim-label");

        line.append(&icon);
        line.append(&sparkline);
        line.append(&label);
        line.append(&value);
        row.set_child(Some(&line));
        if group.id == selected {
            selected_row = Some(row.clone());
        }
        list.append(&row);
    }

    if let Some(row) = selected_row {
        list.select_row(Some(&row));
    } else if let Some(first) = list.row_at_index(0) {
        list.select_row(Some(&first));
    }
}

fn rebuild_content(container: &gtk::Box, data: &SensorData, selected: &str) {
    while let Some(child) = container.first_child() {
        container.remove(&child);
    }

    let group = data
        .groups
        .iter()
        .find(|g| g.id == selected)
        .or_else(|| data.groups.first());
    let Some(group) = group else {
        return;
    };

    let header = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let title = gtk::Label::new(Some(&group.label));
    title.set_xalign(0.0);
    title.add_css_class("title-2");
    title.set_hexpand(true);

    let subtitle = gtk::Label::new(Some(&format!("{} sensores", group.sensors.len())));
    subtitle.add_css_class("dim-label");
    subtitle.set_xalign(1.0);
    header.append(&title);
    header.append(&subtitle);
    container.append(&header);

    let graph_grid = gtk::Grid::new();
    graph_grid.set_column_spacing(10);
    graph_grid.set_row_spacing(10);

    for (idx, sensor) in group.sensors.iter().enumerate() {
        let card = build_sensor_card(sensor);
        let col = (idx % 2) as i32;
        let row = (idx / 2) as i32;
        graph_grid.attach(&card, col, row, 1, 1);
    }

    container.append(&graph_grid);
}

fn build_sensor_card(sensor: &SensorValue) -> gtk::Frame {
    let frame = gtk::Frame::new(None);
    frame.add_css_class("card");

    let card = gtk::Box::new(gtk::Orientation::Vertical, 6);
    card.set_margin_top(8);
    card.set_margin_bottom(8);
    card.set_margin_start(8);
    card.set_margin_end(8);

    let top = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let label = gtk::Label::new(Some(&sensor.label));
    label.set_xalign(0.0);
    label.set_hexpand(true);
    let value = gtk::Label::new(Some(&format_sidebar_value(sensor)));
    value.add_css_class("monospace");
    value.set_xalign(1.0);
    top.append(&label);
    top.append(&value);

    let history = sensor.history.iter().copied().collect::<Vec<_>>();
    let graph = build_graph_widget(&history, 360, 120);

    card.append(&top);
    card.append(&graph);
    frame.set_child(Some(&card));
    frame
}
