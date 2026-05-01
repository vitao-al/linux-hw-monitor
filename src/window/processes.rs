use std::cell::RefCell;
use std::rc::Rc;
use std::process::Command;

use adw::prelude::*;
use gtk4 as gtk;
use glib;

use crate::window::icons::{app_icon_for_process, service_icon_for_unit};
use crate::i18n::t;

// ─────────────────────────────────────────────────────────────
//  Data types
// ─────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
struct AppProcess {
    name: String,
    user: String,
    pid: u32,
    cpu: f64,
    mem: f64,
    mem_mb: f64,
    state: String,
    nice: i32,
}

#[derive(Clone, Debug)]
struct ServiceUnit {
    unit: String,
    #[allow(dead_code)]
    load: String,
    active: String,
    sub: String,
    description: String,
}

// ─────────────────────────────────────────────────────────────
//  Sort state
// ─────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Debug)]
enum AppColumn {
    Name,
    User,
    Pid,
    Cpu,
    Mem,
    State,
    Nice,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum SvcColumn {
    Unit,
    Status,
    Sub,
    Description,
}

#[derive(Clone, Debug)]
struct AppSort {
    column: AppColumn,
    ascending: bool,
}

#[derive(Clone, Debug)]
struct SvcSort {
    column: SvcColumn,
    ascending: bool,
}

// ─────────────────────────────────────────────────────────────
//  Public builders – Apps
// ─────────────────────────────────────────────────────────────

pub(crate) fn build_apps_page() -> (gtk::ListBox, gtk::Label, gtk::Box) {
    let sort_state: Rc<RefCell<AppSort>> = Rc::new(RefCell::new(AppSort {
        column: AppColumn::Cpu,
        ascending: false,
    }));
    let filter_text: Rc<RefCell<String>> = Rc::new(RefCell::new(String::new()));

    let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
    root.set_margin_top(12);
    root.set_margin_bottom(12);
    root.set_margin_start(12);
    root.set_margin_end(12);

    // title + summary
    let header_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let title = gtk::Label::new(Some(&t("Processes")));
    title.set_xalign(0.0);
    title.add_css_class("title-2");
    title.set_hexpand(true);
    let summary = gtk::Label::new(Some(&t("Loading...")));
    summary.add_css_class("dim-label");
    summary.set_xalign(1.0);
    header_row.append(&title);
    header_row.append(&summary);

    // search bar
    let search = gtk::SearchEntry::new();
    search.set_placeholder_text(Some(&t("Filter by name or user...")));
    search.set_hexpand(true);
    search.set_margin_top(6);
    search.set_margin_bottom(2);

    // bulk action row
    let action_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    action_row.set_margin_top(4);
    action_row.set_margin_bottom(6);
    let kill_user_btn = gtk::Button::builder()
        .label(&t("Kill user processes"))
        .tooltip_text(&t("Terminates user processes (does not touch system processes)"))
        .css_classes(["destructive-action"])
        .build();
    let spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    spacer.set_hexpand(true);
    let refresh_btn = gtk::Button::from_icon_name("view-refresh-symbolic");
    refresh_btn.set_tooltip_text(Some(&t("Refresh now")));
    action_row.append(&kill_user_btn);
    action_row.append(&spacer);
    action_row.append(&refresh_btn);

    // column headers
    let col_header = build_app_column_header();

    // list
    let list = gtk::ListBox::new();
    list.add_css_class("boxed-list");
    list.set_selection_mode(gtk::SelectionMode::Single);
    let scroll = gtk::ScrolledWindow::builder()
        .child(&list)
        .vexpand(true)
        .hexpand(true)
        .build();

    root.append(&header_row);
    root.append(&search);
    root.append(&action_row);
    root.append(&col_header);
    root.append(&scroll);

    // wire sort buttons
    wire_app_sort_buttons(
        &col_header,
        Rc::clone(&sort_state),
        Rc::clone(&filter_text),
        list.clone(),
        summary.clone(),
    );

    // wire filter
    {
        let ft = Rc::clone(&filter_text);
        let ss = Rc::clone(&sort_state);
        let lst = list.clone();
        let sum = summary.clone();
        search.connect_search_changed(move |entry| {
            *ft.borrow_mut() = entry.text().to_string();
            repopulate_apps(&lst, &sum, &ft, &ss);
        });
    }

    // wire refresh button
    {
        let ft = Rc::clone(&filter_text);
        let ss = Rc::clone(&sort_state);
        let lst = list.clone();
        let sum = summary.clone();
        refresh_btn.connect_clicked(move |_| {
            repopulate_apps(&lst, &sum, &ft, &ss);
        });
    }

    // wire kill-all user processes
    {
        let ft = Rc::clone(&filter_text);
        let ss = Rc::clone(&sort_state);
        let lst = list.clone();
        let sum = summary.clone();
        kill_user_btn.connect_clicked(move |_| {
            sum.set_text("Encerrando processos do usuario...");
            let lst_ui = lst.clone();
            let sum_ui = sum.clone();
            let ft_ui = Rc::clone(&ft);
            let ss_ui = Rc::clone(&ss);
            let (tx, rx) = std::sync::mpsc::channel::<String>();
            glib::timeout_add_local(std::time::Duration::from_millis(200), move || {
                if let Ok(msg) = rx.try_recv() {
                    sum_ui.set_text(&msg);
                    repopulate_apps(&lst_ui, &sum_ui, &ft_ui, &ss_ui);
                    return glib::ControlFlow::Break;
                }
                glib::ControlFlow::Continue
            });
            std::thread::spawn(move || {
                let msg = kill_all_user_processes();
                let _ = tx.send(msg);
            });
        });
    }

    (list, summary, root)
}

fn build_app_column_header() -> gtk::Box {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    row.add_css_class("dim-label");
    row.set_margin_start(8);
    row.set_margin_end(8);
    row.set_margin_top(4);
    row.set_margin_bottom(2);

    // (label, hexpand, min_width_px)
    let cols: &[(&str, bool, i32)] = &[
        ("Processo", true,  0),
        ("Usuario",  false, 90),
        ("PID",      false, 60),
        ("CPU %",    false, 64),
        ("Mem %",    false, 60),
        ("Mem MB",   false, 68),
        ("Estado",   false, 80),
        ("Nice",     false, 48),
        ("Acoes",    false, 160),
    ];

    for (label, expand, min_w) in cols {
        let btn = gtk::Button::builder()
            .label(*label)
            .css_classes(["flat"])
            .build();
        btn.set_hexpand(*expand);
        if *expand {
            btn.set_halign(gtk::Align::Fill);
        }
        if *min_w > 0 {
            btn.set_size_request(*min_w, -1);
        }
        if let Some(lbl) = btn.child().and_downcast::<gtk::Label>() {
            lbl.set_xalign(0.0);
            lbl.add_css_class("dim-label");
        }
        btn.set_widget_name(label);
        row.append(&btn);
    }
    row
}

fn wire_app_sort_buttons(
    col_header: &gtk::Box,
    sort_state: Rc<RefCell<AppSort>>,
    filter_text: Rc<RefCell<String>>,
    list: gtk::ListBox,
    summary: gtk::Label,
) {
    let names: &[(&str, AppColumn)] = &[
        ("Processo", AppColumn::Name),
        ("Usuario",  AppColumn::User),
        ("PID",      AppColumn::Pid),
        ("CPU %",    AppColumn::Cpu),
        ("Mem %",    AppColumn::Mem),
        ("Mem MB",   AppColumn::Mem),
        ("Estado",   AppColumn::State),
        ("Nice",     AppColumn::Nice),
    ];

    let mut child = col_header.first_child();
    while let Some(widget) = child {
        child = widget.next_sibling();
        let btn = match widget.downcast::<gtk::Button>() {
            Ok(b) => b,
            Err(_) => continue,
        };
        let name = btn.widget_name().to_string();
        let col_opt = names.iter().find(|(n, _)| *n == name).map(|(_, c)| *c);
        if let Some(col) = col_opt {
            let ss = Rc::clone(&sort_state);
            let ft = Rc::clone(&filter_text);
            let lst = list.clone();
            let sum = summary.clone();
            btn.connect_clicked(move |_| {
                {
                    let mut state = ss.borrow_mut();
                    if state.column == col {
                        state.ascending = !state.ascending;
                    } else {
                        state.column = col;
                        state.ascending = false;
                    }
                }
                repopulate_apps(&lst, &sum, &ft, &ss);
            });
        }
    }
}

pub(crate) fn rebuild_apps_list(list: &gtk::ListBox, summary: &gtk::Label) {
    let sort = Rc::new(RefCell::new(AppSort { column: AppColumn::Cpu, ascending: false }));
    let filter = Rc::new(RefCell::new(String::new()));
    repopulate_apps(list, summary, &filter, &sort);
}

fn repopulate_apps(
    list: &gtk::ListBox,
    summary: &gtk::Label,
    filter_text: &Rc<RefCell<String>>,
    sort_state: &Rc<RefCell<AppSort>>,
) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }

    let filter = filter_text.borrow().to_lowercase();
    let sort = sort_state.borrow().clone();

    let mut rows = fetch_apps();

    if !filter.is_empty() {
        rows.retain(|p| {
            p.name.to_lowercase().contains(&filter) || p.user.to_lowercase().contains(&filter)
        });
    }

    rows.sort_by(|a, b| {
        let ord = match sort.column {
            AppColumn::Name  => a.name.cmp(&b.name),
            AppColumn::User  => a.user.cmp(&b.user),
            AppColumn::Pid   => a.pid.cmp(&b.pid),
            AppColumn::Cpu   => a.cpu.partial_cmp(&b.cpu).unwrap_or(std::cmp::Ordering::Equal),
            AppColumn::Mem   => a.mem.partial_cmp(&b.mem).unwrap_or(std::cmp::Ordering::Equal),
            AppColumn::State => a.state.cmp(&b.state),
            AppColumn::Nice  => a.nice.cmp(&b.nice),
        };
        if sort.ascending { ord } else { ord.reverse() }
    });

    summary.set_text(&format!("{} processos", rows.len()));

    for proc in rows {
        let row = build_app_row(&proc, list, summary, filter_text, sort_state);
        list.append(&row);
    }
}

fn build_app_row(
    proc: &AppProcess,
    list: &gtk::ListBox,
    summary: &gtk::Label,
    filter_text: &Rc<RefCell<String>>,
    sort_state: &Rc<RefCell<AppSort>>,
) -> gtk::ListBoxRow {
    let row = gtk::ListBoxRow::new();
    let line = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    line.set_margin_top(5);
    line.set_margin_bottom(5);
    line.set_margin_start(8);
    line.set_margin_end(8);

    let icon_name = app_icon_for_process(&proc.name);
    let icon = gtk::Image::from_icon_name(&icon_name);
    icon.set_pixel_size(16);
    icon.set_margin_end(6);

    let name_lbl = gtk::Label::new(Some(&proc.name));
    name_lbl.set_xalign(0.0);
    name_lbl.set_hexpand(true);
    name_lbl.set_ellipsize(gtk::pango::EllipsizeMode::End);

    let make_mono = |s: &str, width: i32| {
        let l = gtk::Label::new(Some(s));
        l.add_css_class("monospace");
        l.set_xalign(1.0);
        l.set_width_chars(width);
        l
    };

    let user_lbl  = gtk::Label::new(Some(&proc.user));
    user_lbl.set_xalign(0.0);
    user_lbl.set_width_chars(10);
    user_lbl.set_ellipsize(gtk::pango::EllipsizeMode::End);

    let pid_lbl   = make_mono(&proc.pid.to_string(), 7);
    let cpu_lbl   = make_mono(&format!("{:.1}%", proc.cpu), 6);
    let mem_lbl   = make_mono(&format!("{:.1}%", proc.mem), 6);
    let memb_lbl  = make_mono(&format!("{:.0}", proc.mem_mb), 7);

    let state_lbl = gtk::Label::new(Some(&proc.state));
    state_lbl.set_width_chars(10);
    state_lbl.set_xalign(0.5);
    apply_proc_state_css(&state_lbl, &proc.state);

    let nice_lbl  = make_mono(&proc.nice.to_string(), 4);
    nice_lbl.set_xalign(0.5);

    // action buttons
    let btn_box = gtk::Box::new(gtk::Orientation::Horizontal, 4);
    btn_box.set_margin_start(8);
    let kill_btn = gtk::Button::builder()
        .label(&t("Terminate"))
        .css_classes(["destructive-action", "pill"])
        .tooltip_text(&t("Send SIGTERM to process"))
        .build();
    let force_btn = gtk::Button::builder()
        .label(&t("Force Kill"))
        .css_classes(["pill"])
        .tooltip_text(&t("Send SIGKILL (immediate force)"))
        .build();
    btn_box.append(&kill_btn);
    btn_box.append(&force_btn);

    line.append(&icon);
    line.append(&name_lbl);
    line.append(&user_lbl);
    line.append(&pid_lbl);
    line.append(&cpu_lbl);
    line.append(&mem_lbl);
    line.append(&memb_lbl);
    line.append(&state_lbl);
    line.append(&nice_lbl);
    line.append(&btn_box);
    row.set_child(Some(&line));

    wire_process_action(&kill_btn,  proc.pid, false, list, summary, filter_text, sort_state);
    wire_process_action(&force_btn, proc.pid, true,  list, summary, filter_text, sort_state);

    row
}

fn apply_proc_state_css(label: &gtk::Label, state: &str) {
    if state.starts_with('R') || state.contains("run") {
        label.add_css_class("success");
    } else if state.starts_with('Z') || state.contains("zombie") {
        label.add_css_class("error");
    } else if state.starts_with('D') || state.starts_with('T') {
        label.add_css_class("warning");
    }
}

fn wire_process_action(
    btn: &gtk::Button,
    pid: u32,
    force: bool,
    list: &gtk::ListBox,
    summary: &gtk::Label,
    filter_text: &Rc<RefCell<String>>,
    sort_state: &Rc<RefCell<AppSort>>,
) {
    let list_ref = list.clone();
    let sum_ref  = summary.clone();
    let ft = Rc::clone(filter_text);
    let ss = Rc::clone(sort_state);
    btn.connect_clicked(move |_| {
        sum_ref.set_text(&format!("Encerrando PID {}...", pid));
        let lst_ui = list_ref.clone();
        let sum_ui = sum_ref.clone();
        let ft_ui  = Rc::clone(&ft);
        let ss_ui  = Rc::clone(&ss);
        let (tx, rx) = std::sync::mpsc::channel::<String>();
        glib::timeout_add_local(std::time::Duration::from_millis(150), move || {
            if let Ok(msg) = rx.try_recv() {
                sum_ui.set_text(&msg);
                repopulate_apps(&lst_ui, &sum_ui, &ft_ui, &ss_ui);
                return glib::ControlFlow::Break;
            }
            glib::ControlFlow::Continue
        });
        std::thread::spawn(move || {
            let sig = if force { "KILL" } else { "TERM" };
            let msg = kill_process_signal(pid, sig);
            let _ = tx.send(msg);
        });
    });
}

// ─────────────────────────────────────────────────────────────
//  Public builders – Services
// ─────────────────────────────────────────────────────────────

pub(crate) fn build_services_page() -> (gtk::ListBox, gtk::Label, gtk::Box) {
    let sort_state: Rc<RefCell<SvcSort>> = Rc::new(RefCell::new(SvcSort {
        column: SvcColumn::Unit,
        ascending: true,
    }));
    let filter_text: Rc<RefCell<String>> = Rc::new(RefCell::new(String::new()));
    let show_all: Rc<RefCell<bool>> = Rc::new(RefCell::new(false));

    let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
    root.set_margin_top(12);
    root.set_margin_bottom(12);
    root.set_margin_start(12);
    root.set_margin_end(12);

    // title + summary
    let header_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let title = gtk::Label::new(Some(&t("Services")));
    title.set_xalign(0.0);
    title.add_css_class("title-2");
    title.set_hexpand(true);
    let summary = gtk::Label::new(Some(&t("Loading...")));
    summary.add_css_class("dim-label");
    header_row.append(&title);
    header_row.append(&summary);

    // filter row
    let filter_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    filter_row.set_margin_top(6);
    filter_row.set_margin_bottom(2);
    let search = gtk::SearchEntry::new();
    search.set_placeholder_text(Some(&t("Filter by name or description...")));
    search.set_hexpand(true);
    let show_all_btn = gtk::ToggleButton::builder()
        .label("Mostrar todos")
        .tooltip_text("Mostra tambem servicos parados/inativos")
        .build();
    filter_row.append(&search);
    filter_row.append(&show_all_btn);

    // bulk actions
    let action_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    action_row.set_margin_top(4);
    action_row.set_margin_bottom(6);
    let stop_user_btn = gtk::Button::builder()
        .label(&t("Stop user services"))
        .tooltip_text(&t("Stops only the current user's services"))
        .css_classes(["destructive-action"])
        .build();
    let spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    spacer.set_hexpand(true);
    let refresh_btn = gtk::Button::from_icon_name("view-refresh-symbolic");
    refresh_btn.set_tooltip_text(Some(&t("Refresh now")));
    action_row.append(&stop_user_btn);
    action_row.append(&spacer);
    action_row.append(&refresh_btn);

    // column header
    let col_header = build_svc_column_header();

    let list = gtk::ListBox::new();
    list.add_css_class("boxed-list");
    list.set_selection_mode(gtk::SelectionMode::Single);
    let scroll = gtk::ScrolledWindow::builder()
        .child(&list)
        .vexpand(true)
        .hexpand(true)
        .build();

    root.append(&header_row);
    root.append(&filter_row);
    root.append(&action_row);
    root.append(&col_header);
    root.append(&scroll);

    // wire sort
    wire_svc_sort_buttons(
        &col_header,
        Rc::clone(&sort_state),
        Rc::clone(&filter_text),
        Rc::clone(&show_all),
        list.clone(),
        summary.clone(),
    );

    // wire filter
    {
        let ft = Rc::clone(&filter_text);
        let ss = Rc::clone(&sort_state);
        let sa = Rc::clone(&show_all);
        let lst = list.clone();
        let sum = summary.clone();
        search.connect_search_changed(move |entry| {
            *ft.borrow_mut() = entry.text().to_string();
            repopulate_services(&lst, &sum, &ft, &ss, &sa);
        });
    }

    // wire show-all
    {
        let ft = Rc::clone(&filter_text);
        let ss = Rc::clone(&sort_state);
        let sa = Rc::clone(&show_all);
        let lst = list.clone();
        let sum = summary.clone();
        show_all_btn.connect_active_notify(move |btn| {
            *sa.borrow_mut() = btn.is_active();
            repopulate_services(&lst, &sum, &ft, &ss, &sa);
        });
    }

    // wire refresh
    {
        let ft = Rc::clone(&filter_text);
        let ss = Rc::clone(&sort_state);
        let sa = Rc::clone(&show_all);
        let lst = list.clone();
        let sum = summary.clone();
        refresh_btn.connect_clicked(move |_| {
            repopulate_services(&lst, &sum, &ft, &ss, &sa);
        });
    }

    // wire stop-all user services
    {
        let ft = Rc::clone(&filter_text);
        let ss = Rc::clone(&sort_state);
        let sa = Rc::clone(&show_all);
        let lst = list.clone();
        let sum = summary.clone();
        stop_user_btn.connect_clicked(move |_| {
            sum.set_text("Parando servicos do usuario...");
            let lst_ui = lst.clone();
            let sum_ui = sum.clone();
            let ft_ui  = Rc::clone(&ft);
            let ss_ui  = Rc::clone(&ss);
            let sa_ui  = Rc::clone(&sa);
            let (tx, rx) = std::sync::mpsc::channel::<String>();
            glib::timeout_add_local(std::time::Duration::from_millis(400), move || {
                if let Ok(msg) = rx.try_recv() {
                    sum_ui.set_text(&msg);
                    repopulate_services(&lst_ui, &sum_ui, &ft_ui, &ss_ui, &sa_ui);
                    return glib::ControlFlow::Break;
                }
                glib::ControlFlow::Continue
            });
            std::thread::spawn(move || {
                let msg = stop_all_user_services();
                let _ = tx.send(msg);
            });
        });
    }

    (list, summary, root)
}

fn build_svc_column_header() -> gtk::Box {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    row.add_css_class("dim-label");
    row.set_margin_start(8);
    row.set_margin_end(8);
    row.set_margin_top(4);
    row.set_margin_bottom(2);

    let cols: &[(&str, bool, i32)] = &[
        ("Service",     true,  0),
        ("Status",      false, 85),
        ("Sub-state",   false, 95),
        ("Description", true,  0),
        ("Actions",     false, 200),
    ];

    for (label, expand, min_w) in cols {
        let btn = gtk::Button::builder()
            .label(*label)
            .css_classes(["flat"])
            .build();
        btn.set_hexpand(*expand);
        if *expand {
            btn.set_halign(gtk::Align::Fill);
        }
        if *min_w > 0 {
            btn.set_size_request(*min_w, -1);
        }
        if let Some(lbl) = btn.child().and_downcast::<gtk::Label>() {
            lbl.set_xalign(0.0);
            lbl.add_css_class("dim-label");
        }
        btn.set_widget_name(label);
        row.append(&btn);
    }
    row
}

fn wire_svc_sort_buttons(
    col_header: &gtk::Box,
    sort_state: Rc<RefCell<SvcSort>>,
    filter_text: Rc<RefCell<String>>,
    show_all: Rc<RefCell<bool>>,
    list: gtk::ListBox,
    summary: gtk::Label,
) {
    let names: &[(&str, SvcColumn)] = &[
        ("Service",     SvcColumn::Unit),
        ("Status",      SvcColumn::Status),
        ("Sub-state",   SvcColumn::Sub),
        ("Description", SvcColumn::Description),
    ];

    let mut child = col_header.first_child();
    while let Some(widget) = child {
        child = widget.next_sibling();
        let btn = match widget.downcast::<gtk::Button>() {
            Ok(b) => b,
            Err(_) => continue,
        };
        let name = btn.widget_name().to_string();
        let col_opt = names.iter().find(|(n, _)| *n == name).map(|(_, c)| *c);
        if let Some(col) = col_opt {
            let ss = Rc::clone(&sort_state);
            let ft = Rc::clone(&filter_text);
            let sa = Rc::clone(&show_all);
            let lst = list.clone();
            let sum = summary.clone();
            btn.connect_clicked(move |_| {
                {
                    let mut state = ss.borrow_mut();
                    if state.column == col {
                        state.ascending = !state.ascending;
                    } else {
                        state.column = col;
                        state.ascending = true;
                    }
                }
                repopulate_services(&lst, &sum, &ft, &ss, &sa);
            });
        }
    }
}

pub(crate) fn rebuild_services_list(list: &gtk::ListBox, summary: &gtk::Label) {
    let sort = Rc::new(RefCell::new(SvcSort { column: SvcColumn::Unit, ascending: true }));
    let filter = Rc::new(RefCell::new(String::new()));
    let show_all = Rc::new(RefCell::new(false));
    repopulate_services(list, summary, &filter, &sort, &show_all);
}

fn repopulate_services(
    list: &gtk::ListBox,
    summary: &gtk::Label,
    filter_text: &Rc<RefCell<String>>,
    sort_state: &Rc<RefCell<SvcSort>>,
    show_all: &Rc<RefCell<bool>>,
) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }

    let filter  = filter_text.borrow().to_lowercase();
    let sort    = sort_state.borrow().clone();
    let all     = *show_all.borrow();

    let mut rows = fetch_services(all);

    if !filter.is_empty() {
        rows.retain(|s| {
            s.unit.to_lowercase().contains(&filter)
                || s.description.to_lowercase().contains(&filter)
        });
    }

    rows.sort_by(|a, b| {
        let ord = match sort.column {
            SvcColumn::Unit        => a.unit.cmp(&b.unit),
            SvcColumn::Status      => a.active.cmp(&b.active),
            SvcColumn::Sub         => a.sub.cmp(&b.sub),
            SvcColumn::Description => a.description.cmp(&b.description),
        };
        if sort.ascending { ord } else { ord.reverse() }
    });

    summary.set_text(&format!("{} servicos", rows.len()));

    for svc in rows {
        let row = build_svc_row(&svc, list, summary, filter_text, sort_state, show_all);
        list.append(&row);
    }
}

fn build_svc_row(
    svc: &ServiceUnit,
    list: &gtk::ListBox,
    summary: &gtk::Label,
    filter_text: &Rc<RefCell<String>>,
    sort_state: &Rc<RefCell<SvcSort>>,
    show_all: &Rc<RefCell<bool>>,
) -> gtk::ListBoxRow {
    let row = gtk::ListBoxRow::new();
    let line = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    line.set_margin_top(5);
    line.set_margin_bottom(5);
    line.set_margin_start(8);
    line.set_margin_end(8);

    let icon_name = service_icon_for_unit(&svc.unit);
    let icon = gtk::Image::from_icon_name(&icon_name);
    icon.set_pixel_size(16);
    icon.set_margin_end(6);

    let unit_lbl = gtk::Label::new(Some(&svc.unit));
    unit_lbl.set_xalign(0.0);
    unit_lbl.set_hexpand(true);
    unit_lbl.set_ellipsize(gtk::pango::EllipsizeMode::End);

    let status_lbl = gtk::Label::new(Some(&svc.active));
    status_lbl.set_width_chars(10);
    status_lbl.set_xalign(0.5);
    apply_service_active_css(&status_lbl, &svc.active);

    let sub_lbl = gtk::Label::new(Some(&svc.sub));
    sub_lbl.set_width_chars(11);
    sub_lbl.set_xalign(0.5);

    let desc_lbl = gtk::Label::new(Some(&svc.description));
    desc_lbl.set_xalign(0.0);
    desc_lbl.set_hexpand(true);
    desc_lbl.set_ellipsize(gtk::pango::EllipsizeMode::End);

    // action buttons
    let btn_box = gtk::Box::new(gtk::Orientation::Horizontal, 4);
    btn_box.set_margin_start(8);

    let is_running = svc.active == "active" || svc.sub == "running";
    let is_user = svc.unit.ends_with(" (user)");

    let start_btn = gtk::Button::builder()
        .label("Iniciar")
        .css_classes(["suggested-action", "pill"])
        .tooltip_text("systemctl start")
        .build();
    start_btn.set_sensitive(!is_running);

    let stop_btn = gtk::Button::builder()
        .label("Parar")
        .css_classes(["destructive-action", "pill"])
        .tooltip_text("systemctl stop")
        .build();
    stop_btn.set_sensitive(is_running);

    let restart_btn = gtk::Button::builder()
        .label("Reiniciar")
        .css_classes(["pill"])
        .tooltip_text("systemctl restart")
        .build();

    btn_box.append(&start_btn);
    btn_box.append(&stop_btn);
    btn_box.append(&restart_btn);

    line.append(&icon);
    line.append(&unit_lbl);
    line.append(&status_lbl);
    line.append(&sub_lbl);
    line.append(&desc_lbl);
    line.append(&btn_box);
    row.set_child(Some(&line));

    wire_svc_action(&start_btn,   &svc.unit, "start",   is_user, list, summary, filter_text, sort_state, show_all);
    wire_svc_action(&stop_btn,    &svc.unit, "stop",    is_user, list, summary, filter_text, sort_state, show_all);
    wire_svc_action(&restart_btn, &svc.unit, "restart", is_user, list, summary, filter_text, sort_state, show_all);

    row
}

fn apply_service_active_css(label: &gtk::Label, state: &str) {
    match state {
        "active"         => label.add_css_class("success"),
        "failed"         => label.add_css_class("error"),
        "activating" | "deactivating" => label.add_css_class("warning"),
        _                => label.add_css_class("dim-label"),
    }
}

fn wire_svc_action(
    btn: &gtk::Button,
    unit: &str,
    action: &'static str,
    is_user: bool,
    list: &gtk::ListBox,
    summary: &gtk::Label,
    filter_text: &Rc<RefCell<String>>,
    sort_state: &Rc<RefCell<SvcSort>>,
    show_all: &Rc<RefCell<bool>>,
) {
    let unit = unit.to_string();
    let lst  = list.clone();
    let sum  = summary.clone();
    let ft   = Rc::clone(filter_text);
    let ss   = Rc::clone(sort_state);
    let sa   = Rc::clone(show_all);
    btn.connect_clicked(move |_| {
        sum.set_text(&format!("{} {}...", action, unit));
        let lst_ui = lst.clone();
        let sum_ui = sum.clone();
        let ft_ui  = Rc::clone(&ft);
        let ss_ui  = Rc::clone(&ss);
        let sa_ui  = Rc::clone(&sa);
        let unit2  = unit.clone();
        let (tx, rx) = std::sync::mpsc::channel::<String>();
        glib::timeout_add_local(std::time::Duration::from_millis(500), move || {
            if let Ok(msg) = rx.try_recv() {
                sum_ui.set_text(&msg);
                repopulate_services(&lst_ui, &sum_ui, &ft_ui, &ss_ui, &sa_ui);
                return glib::ControlFlow::Break;
            }
            glib::ControlFlow::Continue
        });
        std::thread::spawn(move || {
            let msg = run_systemctl_action(&unit2, action, is_user);
            let _ = tx.send(msg);
        });
    });
}

// ─────────────────────────────────────────────────────────────
//  Data fetching
// ─────────────────────────────────────────────────────────────

fn fetch_apps() -> Vec<AppProcess> {
    let output = Command::new("sh")
        .arg("-c")
        .arg("ps -eo comm,user,pid,%cpu,%mem,rsz,stat,ni --no-headers --sort=-%cpu | head -n 200")
        .output();

    let Ok(out) = output else {
        return vec![AppProcess {
            name: "ps indisponivel".into(), user: "-".into(),
            pid: 0, cpu: 0.0, mem: 0.0, mem_mb: 0.0, state: "-".into(), nice: 0,
        }];
    };

    let text = String::from_utf8_lossy(&out.stdout);
    let mut rows = Vec::new();
    for line in text.lines() {
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.len() < 8 { continue; }
        let Ok(pid) = cols[2].parse::<u32>() else { continue };
        let cpu    = cols[3].parse::<f64>().unwrap_or(0.0);
        let mem    = cols[4].parse::<f64>().unwrap_or(0.0);
        let rsz    = cols[5].parse::<f64>().unwrap_or(0.0);
        let mem_mb = rsz / 1024.0;
        let state  = match cols[6].chars().next() {
            Some('R') => "R run",
            Some('S') => "S sleep",
            Some('D') => "D wait",
            Some('Z') => "Z zombie",
            Some('T') => "T stop",
            _         => "?",
        };
        let nice = cols[7].parse::<i32>().unwrap_or(0);
        rows.push(AppProcess {
            name: cols[0].to_string(), user: cols[1].to_string(),
            pid, cpu, mem, mem_mb, state: state.to_string(), nice,
        });
    }
    rows
}

fn fetch_services(all: bool) -> Vec<ServiceUnit> {
    let state_flag = if all { "--state=loaded" } else { "--state=running" };

    let mut rows = Vec::new();

    // system services
    let cmd = format!(
        "systemctl list-units --type=service {} --no-legend --no-pager --plain",
        state_flag
    );
    if let Ok(out) = Command::new("sh").arg("-c").arg(&cmd).output() {
        let text = String::from_utf8_lossy(&out.stdout);
        for line in text.lines().take(200) {
            let cols: Vec<&str> = line.split_whitespace().collect();
            if cols.len() < 4 { continue; }
            rows.push(ServiceUnit {
                unit:        cols[0].to_string(),
                load:        cols[1].to_string(),
                active:      cols[2].to_string(),
                sub:         cols[3].to_string(),
                description: cols.get(4..).map(|s| s.join(" ")).unwrap_or_default(),
            });
        }
    }

    // user services
    let ucmd = format!(
        "systemctl --user list-units --type=service {} --no-legend --no-pager --plain",
        state_flag
    );
    if let Ok(out) = Command::new("sh").arg("-c").arg(&ucmd).output() {
        let text = String::from_utf8_lossy(&out.stdout);
        for line in text.lines().take(200) {
            let cols: Vec<&str> = line.split_whitespace().collect();
            if cols.len() < 4 { continue; }
            rows.push(ServiceUnit {
                unit:        format!("{} (user)", cols[0]),
                load:        cols[1].to_string(),
                active:      cols[2].to_string(),
                sub:         cols[3].to_string(),
                description: cols.get(4..).map(|s| s.join(" ")).unwrap_or_default(),
            });
        }
    }

    rows
}

// ─────────────────────────────────────────────────────────────
//  Process actions
// ─────────────────────────────────────────────────────────────

fn kill_process_signal(pid: u32, signal: &str) -> String {
    let res = Command::new("/bin/kill")
        .arg(format!("-{}", signal))
        .arg(pid.to_string())
        .output();

    match res {
        Ok(out) if out.status.success() => format!("PID {} encerrado ({})", pid, signal),
        _ => {
            if let Some(launcher) = detect_admin_launcher() {
                let mut cmd = Command::new(&launcher[0]);
                for a in launcher.iter().skip(1) { cmd.arg(a); }
                let ok = cmd.arg("/bin/kill")
                    .arg(format!("-{}", signal))
                    .arg(pid.to_string())
                    .status()
                    .map(|s| s.success())
                    .unwrap_or(false);
                if ok {
                    return format!("PID {} terminated with admin ({})", pid, signal);
                }
            }
            format!("Failed to terminate PID {} (permission denied?)", pid)
        }
    }
}

fn kill_all_user_processes() -> String {
    let my_user = std::env::var("USER").unwrap_or_else(|_| "root".into());
    let my_pid  = std::process::id();
    let protect = [
        "systemd", "dbus-daemon", "sd-pam", "login", "sshd", "bash", "sh",
        "fish", "zsh", "kwin_wayland", "plasmashell", "Xorg", "gnome-shell",
        "mutter", "compositor", "pulseaudio", "pipewire",
        "linux-hw-moni",  // self
    ];

    let out = Command::new("sh")
        .arg("-c")
        .arg(format!("ps -u '{}' -o pid,comm --no-headers", my_user))
        .output();
    let Ok(out) = out else { return "Failed to list processes".into(); };

    let text = String::from_utf8_lossy(&out.stdout);
    let mut killed = 0u32;
    let mut skipped = 0u32;
    for line in text.lines() {
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.len() < 2 { continue; }
        let Ok(pid) = cols[0].parse::<u32>() else { continue };
        let comm = cols[1];
        if pid == my_pid || pid == 1 || protect.iter().any(|&p| comm.starts_with(p)) {
            skipped += 1;
            continue;
        }
        let _ = Command::new("/bin/kill").arg("-TERM").arg(pid.to_string()).status();
        killed += 1;
    }
    format!("{} processes terminated, {} protected/skipped", killed, skipped)
}

// ─────────────────────────────────────────────────────────────
//  Service actions
// ─────────────────────────────────────────────────────────────

fn run_systemctl_action(unit: &str, action: &str, is_user: bool) -> String {
    let (clean_unit, user_flag) = if unit.ends_with(" (user)") {
        (unit.trim_end_matches(" (user)"), true)
    } else {
        (unit, is_user)
    };

    let mut args = vec!["systemctl".to_string()];
    if user_flag { args.push("--user".to_string()); }
    args.push(action.to_string());
    args.push(clean_unit.to_string());

    let res = Command::new(&args[0]).args(&args[1..]).output();
    match res {
        Ok(out) if out.status.success() => format!("{} -> {} OK", clean_unit, action),
        Ok(out) => {
            let err = String::from_utf8_lossy(&out.stderr);
            if !user_flag {
                if let Some(launcher) = detect_admin_launcher() {
                    let mut cmd = Command::new(&launcher[0]);
                    for a in launcher.iter().skip(1) { cmd.arg(a); }
                    for a in &args { cmd.arg(a); }
                    if matches!(cmd.status(), Ok(s) if s.success()) {
                        return format!("{} -> {} OK (admin)", clean_unit, action);
                    }
                }
            }
            format!("Failed: {}", err.trim())
        }
        Err(e) => format!("Error: {}", e),
    }
}

fn stop_all_user_services() -> String {
    let protect = [
        "dbus.service", "pulseaudio.service", "pipewire.service",
        "pipewire-pulse.service", "wireplumber.service",
    ];
    let out = Command::new("sh")
        .arg("-c")
        .arg("systemctl --user list-units --type=service --state=running --no-legend --no-pager --plain")
        .output();
    let Ok(out) = out else { return "Failed to list user services".into(); };
    let text = String::from_utf8_lossy(&out.stdout);
    let mut stopped = 0u32;
    for line in text.lines() {
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.is_empty() { continue; }
        let unit = cols[0];
        if protect.contains(&unit) { continue; }
        let _ = Command::new("systemctl").args(["--user", "stop", unit]).status();
        stopped += 1;
    }
    format!("{} servicos do usuario parados", stopped)
}

// ─────────────────────────────────────────────────────────────
//  Helpers
// ─────────────────────────────────────────────────────────────

fn detect_admin_launcher() -> Option<Vec<String>> {
    let in_flatpak = std::env::var("FLATPAK_ID").is_ok();
    if in_flatpak && command_exists("flatpak-spawn") {
        return Some(vec!["flatpak-spawn".into(), "--host".into(), "pkexec".into()]);
    }
    if command_exists("pkexec") { return Some(vec!["pkexec".into()]); }
    if command_exists("run0")   { return Some(vec!["run0".into()]);   }
    if command_exists("sudo")   { return Some(vec!["sudo".into()]);   }
    None
}

fn command_exists(cmd: &str) -> bool {
    Command::new("sh")
        .arg("-c")
        .arg(format!("command -v {} >/dev/null 2>&1", cmd))
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}
