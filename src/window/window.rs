use std::cell::{Cell, RefCell};
use std::rc::Rc;

use adw::prelude::*;
use gtk4 as gtk;

use crate::config::AppConfig;
use crate::sensors::manager::SensorManager;
use crate::sensors::types::{SensorData, SensorValue};
use crate::ui::gauge_widget::GaugeWidget;
use crate::ui::graph_widget::{build_graph_widget, group_color};
use crate::window::actions::install_actions;

/// Per-group sidebar entry: (id, label, visible).
type SidebarPrefs = Vec<(String, String, bool)>;
use crate::window::cpu_overview::rebuild_cpu_overview;
use crate::window::formatting::{extract_group_percent, format_sidebar_value};
use crate::window::icons::{best_icon_name, preferred_icon_for_group};
use crate::window::processes::{
    build_apps_page, build_services_page, rebuild_apps_list, rebuild_services_list,
};
use crate::i18n::t;
use crate::window::stress::build_stress_page;
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
            .title(&t("Sensors"))
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
            .title(&t("Details"))
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

        let stress_page_widget = build_stress_page();

        let view_stack = adw::ViewStack::new();
        let perf_page = view_stack.add_titled(&split, Some("performance"), &t("Performance"));
        let apps_page_stack = view_stack.add_titled(&apps_page.2, Some("apps"), &t("Apps"));
        let services_page_stack =
            view_stack.add_titled(&services_page.2, Some("services"), &t("Services"));
        let stress_page_stack =
            view_stack.add_titled(&stress_page_widget, Some("stress"), &t("Stress Test"));

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
        stress_page_stack.set_icon_name(Some(&best_icon_name(
            &["temperature-symbolic", "dialog-warning-symbolic", "utilities-system-monitor-symbolic"],
            "utilities-system-monitor-symbolic",
        )));

        let header = adw::HeaderBar::new();
        let switcher = adw::ViewSwitcher::new();
        switcher.set_stack(Some(&view_stack));
        switcher.set_policy(adw::ViewSwitcherPolicy::Wide);
        switcher.set_hexpand(true);
        header.set_title_widget(Some(&switcher));

        let pref_btn = gtk::Button::from_icon_name("emblem-system-symbolic");
        pref_btn.set_tooltip_text(Some(&t("Preferences")));
        header.pack_end(&pref_btn);

        let about_btn = gtk::Button::from_icon_name("help-about-symbolic");
        about_btn.set_tooltip_text(Some(&t("About")));
        header.pack_end(&about_btn);

        let export_menu = gtk::MenuButton::builder()
            .icon_name("document-save-symbolic")
            .tooltip_text(&t("Export data"))
            .build();
        let model = gio::Menu::new();
        model.append(Some(&t("Export CSV")), Some("app.export-csv"));
        model.append(Some(&t("Export JSON")), Some("app.export-json"));
        model.append(Some(&t("Export Text")), Some("app.export-text"));
        export_menu.set_menu_model(Some(&model));
        header.pack_end(&export_menu);

        let toolbar = adw::ToolbarView::new();
        toolbar.add_top_bar(&header);
        toolbar.set_content(Some(&view_stack));

        let window = adw::ApplicationWindow::builder()
            .application(app)
.title(&t("Linux HW Monitor"))
            .default_width(1100)
            .default_height(720)
            .content(&toolbar)
            .build();

        // Sidebar customization state: None = natural order, all visible.
        // Each entry: (group_id, group_label, visible).
        let sidebar_prefs: Rc<RefCell<Option<SidebarPrefs>>> =
            Rc::new(RefCell::new(None));
        // Last-known group list so the preferences dialog can list groups even
        // before the user opens it.
        let last_groups: Rc<RefCell<Vec<(String, String)>>> =
            Rc::new(RefCell::new(vec![]));

        let style_manager = adw::StyleManager::default();
        let pref_parent = window.clone();
        let sidebar_prefs_pref = Rc::clone(&sidebar_prefs);
        let last_groups_pref = Rc::clone(&last_groups);
        pref_btn.connect_clicked(move |_| {
            let pref = adw::PreferencesWindow::new();
            pref.set_title(Some(&t("Preferences")));
            pref.set_transient_for(Some(&pref_parent));

            let page = adw::PreferencesPage::new();
            let group = adw::PreferencesGroup::builder().title(&t("General")).build();

            let interval = adw::ComboRow::builder().title(&t("Update interval")).build();
            let interval_model = gtk::StringList::new(&["1s", "2s", "5s"]);
            interval.set_model(Some(&interval_model));
            interval.set_selected(0);

            let temp = adw::ComboRow::builder().title(&t("Temperature unit")).build();
            let temp_model = gtk::StringList::new(&[&t("Celsius"), &t("Fahrenheit")]);
            temp.set_model(Some(&temp_model));

            let data_unit = adw::ComboRow::builder().title(&t("Data unit")).build();
            let data_model = gtk::StringList::new(&["SI", "IEC"]);
            data_unit.set_model(Some(&data_model));

            let notify = adw::SwitchRow::builder()
                .title(&t("Critical temperature notifications"))
                .active(true)
                .build();

            let theme = adw::ComboRow::builder().title(&t("Theme")).build();
            let theme_model = gtk::StringList::new(&[&t("System"), &t("Light"), &t("Dark")]);
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

            // ── Sidebar page ──────────────────────────────────────────────
            let sidebar_page = adw::PreferencesPage::builder()
                .title(&t("Sidebar"))
                .icon_name("view-list-symbolic")
                .build();

            let sidebar_group = adw::PreferencesGroup::builder()
                .title(&t("Sidebar buttons"))
                .description(&t("Choose which categories appear in the sidebar and their order."))
                .build();

            // Snapshot the current prefs (or build from last_groups if not set).
            let current_entries: SidebarPrefs = {
                let prefs_guard = sidebar_prefs_pref.borrow();
                if let Some(entries) = prefs_guard.as_ref() {
                    entries.clone()
                } else {
                    last_groups_pref
                        .borrow()
                        .iter()
                        .map(|(id, label)| (id.clone(), label.clone(), true))
                        .collect()
                }
            };

            // Working copy inside the dialog (Rc so buttons can share it).
            let working: Rc<RefCell<SidebarPrefs>> =
                Rc::new(RefCell::new(current_entries));

            // ListBox to display editable rows.
            let entry_list = gtk::ListBox::new();
            entry_list.add_css_class("boxed-list");
            entry_list.set_selection_mode(gtk::SelectionMode::None);

            // Helper: rebuild the entry_list from the working copy.
            fn repopulate_entry_list(
                entry_list: &gtk::ListBox,
                working: &Rc<RefCell<SidebarPrefs>>,
                sidebar_prefs: &Rc<RefCell<Option<SidebarPrefs>>>,
            ) {
                while let Some(child) = entry_list.first_child() {
                    entry_list.remove(&child);
                }
                let entries = working.borrow();
                let n = entries.len();
                for (idx, (id, label, visible)) in entries.iter().enumerate() {
                    let row_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);
                    row_box.set_margin_top(4);
                    row_box.set_margin_bottom(4);
                    row_box.set_margin_start(8);
                    row_box.set_margin_end(8);

                    // Visibility toggle.
                    let toggle = gtk::Switch::new();
                    toggle.set_active(*visible);
                    toggle.set_valign(gtk::Align::Center);

                    let lbl = gtk::Label::new(Some(label));
                    lbl.set_hexpand(true);
                    lbl.set_xalign(0.0);

                    // Up / down buttons.
                    let up_btn = gtk::Button::from_icon_name("go-up-symbolic");
                    up_btn.set_sensitive(idx > 0);
                    up_btn.add_css_class("flat");
                    up_btn.set_valign(gtk::Align::Center);

                    let down_btn = gtk::Button::from_icon_name("go-down-symbolic");
                    down_btn.set_sensitive(idx + 1 < n);
                    down_btn.add_css_class("flat");
                    down_btn.set_valign(gtk::Align::Center);

                    row_box.append(&toggle);
                    row_box.append(&lbl);
                    row_box.append(&up_btn);
                    row_box.append(&down_btn);

                    let row = gtk::ListBoxRow::new();
                    row.set_child(Some(&row_box));
                    entry_list.append(&row);

                    // Wire toggle.
                    let id_clone = id.clone();
                    let working_t = Rc::clone(working);
                    let prefs_t = Rc::clone(sidebar_prefs);
                    toggle.connect_active_notify(move |sw| {
                        let mut w = working_t.borrow_mut();
                        if let Some(e) = w.iter_mut().find(|(eid, _, _)| eid == &id_clone) {
                            e.2 = sw.is_active();
                        }
                        *prefs_t.borrow_mut() = Some(w.clone());
                    });

                    // Wire up button.
                    let working_u = Rc::clone(working);
                    let prefs_u = Rc::clone(sidebar_prefs);
                    let list_u = entry_list.clone();
                    let w2 = Rc::clone(working);
                    let p2 = Rc::clone(sidebar_prefs);
                    up_btn.connect_clicked(move |_| {
                        let mut entries = working_u.borrow_mut();
                        if idx > 0 {
                            entries.swap(idx, idx - 1);
                            *prefs_u.borrow_mut() = Some(entries.clone());
                        }
                        drop(entries);
                        repopulate_entry_list(&list_u, &w2, &p2);
                    });

                    // Wire down button.
                    let working_d = Rc::clone(working);
                    let prefs_d = Rc::clone(sidebar_prefs);
                    let list_d = entry_list.clone();
                    let w3 = Rc::clone(working);
                    let p3 = Rc::clone(sidebar_prefs);
                    down_btn.connect_clicked(move |_| {
                        let mut entries = working_d.borrow_mut();
                        if idx + 1 < n {
                            entries.swap(idx, idx + 1);
                            *prefs_d.borrow_mut() = Some(entries.clone());
                        }
                        drop(entries);
                        repopulate_entry_list(&list_d, &w3, &p3);
                    });
                }
            }

            repopulate_entry_list(&entry_list, &working, &sidebar_prefs_pref);

            // "Reset to defaults" button clears the custom prefs.
            let reset_btn = gtk::Button::builder()
                .label(&t("Reset to Defaults"))
                .css_classes(["destructive-action"])
                .halign(gtk::Align::End)
                .build();
            let prefs_reset = Rc::clone(&sidebar_prefs_pref);
            let working_reset = Rc::clone(&working);
            let list_reset = entry_list.clone();
            let last_groups_reset = Rc::clone(&last_groups_pref);
            let prefs_reset2 = Rc::clone(&sidebar_prefs_pref);
            reset_btn.connect_clicked(move |_| {
                *prefs_reset.borrow_mut() = None;
                let defaults: SidebarPrefs = last_groups_reset
                    .borrow()
                    .iter()
                    .map(|(id, label)| (id.clone(), label.clone(), true))
                    .collect();
                *working_reset.borrow_mut() = defaults;
                repopulate_entry_list(&list_reset, &working_reset, &prefs_reset2);
            });

            sidebar_group.add(&entry_list);
            sidebar_group.set_header_suffix(Some(&reset_btn));
            sidebar_page.add(&sidebar_group);
            pref.add(&sidebar_page);

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
        let sidebar_prefs_ref = Rc::clone(&sidebar_prefs);
        let last_groups_ref = Rc::clone(&last_groups);

        rebuild_apps_list(&apps_list, &apps_summary);
        rebuild_services_list(&services_list, &services_summary);

        glib::timeout_add_seconds_local(1, move || {
            let data = manager_rx.borrow().borrow().clone();
            let selected = selected_ref.borrow().clone();

            // Keep last_groups in sync so the preferences dialog has labels.
            {
                let mut lg = last_groups_ref.borrow_mut();
                *lg = data.groups.iter().map(|g| (g.id.clone(), g.label.clone())).collect();
            }

            rebuild_sidebar(&categories_ref, &data, &selected, &sidebar_prefs_ref);
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

fn rebuild_sidebar(
    list: &gtk::ListBox,
    data: &SensorData,
    selected: &str,
    prefs: &Rc<RefCell<Option<SidebarPrefs>>>,
) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }

    // Build the ordered, filtered group list.
    let ordered_groups: Vec<_> = {
        let prefs_borrow = prefs.borrow();
        if let Some(entries) = prefs_borrow.as_ref() {
            // Use the user-defined order; skip hidden groups.
            entries
                .iter()
                .filter(|(_, _, visible)| *visible)
                .filter_map(|(id, _, _)| data.groups.iter().find(|g| &g.id == id))
                .collect()
        } else {
            data.groups.iter().collect()
        }
    };

    let mut selected_row: Option<gtk::ListBoxRow> = None;

    for group in ordered_groups {
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

    let subtitle = gtk::Label::new(Some(&format!("{} {}", group.sensors.len(), t("sensors"))));
    subtitle.add_css_class("dim-label");
    subtitle.set_xalign(1.0);
    header.append(&title);
    header.append(&subtitle);
    container.append(&header);

    let graph_grid = gtk::Grid::new();
    graph_grid.set_column_spacing(10);
    graph_grid.set_row_spacing(10);

    let color = group_color(&group.id);
    for (idx, sensor) in group.sensors.iter().enumerate() {
        let card = build_sensor_card(sensor, color);
        let col = (idx % 2) as i32;
        let row = (idx / 2) as i32;
        graph_grid.attach(&card, col, row, 1, 1);
    }

    container.append(&graph_grid);
}

fn build_sensor_card(sensor: &SensorValue, color: (f64, f64, f64)) -> gtk::Frame {
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
    let graph = build_graph_widget(&history, 360, 120, color);

    card.append(&top);
    card.append(&graph);
    frame.set_child(Some(&card));
    frame
}
