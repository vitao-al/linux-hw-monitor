use std::process::Command;

use gtk4 as gtk;
use gtk::prelude::*;

use crate::window::icons::{app_icon_for_process, service_icon_for_unit};

struct AppProcess {
    name: String,
    user: String,
    pid: u32,
    cpu: String,
    mem: String,
}

pub(crate) fn build_apps_page() -> (gtk::ListBox, gtk::Label, gtk::Box) {
    let root = gtk::Box::new(gtk::Orientation::Vertical, 8);
    root.set_margin_top(12);
    root.set_margin_bottom(12);
    root.set_margin_start(12);
    root.set_margin_end(12);

    let title = gtk::Label::new(Some("Running Apps"));
    title.set_xalign(0.0);
    title.add_css_class("title-2");

    let summary = gtk::Label::new(Some("Loading..."));
    summary.set_xalign(0.0);
    summary.add_css_class("dim-label");

    let list = gtk::ListBox::new();
    list.add_css_class("boxed-list");
    list.set_selection_mode(gtk::SelectionMode::None);

    let scroll = gtk::ScrolledWindow::builder().child(&list).vexpand(true).hexpand(true).build();

    root.append(&title);
    root.append(&summary);
    root.append(&scroll);
    (list, summary, root)
}

pub(crate) fn build_services_page() -> (gtk::ListBox, gtk::Label, gtk::Box) {
    let root = gtk::Box::new(gtk::Orientation::Vertical, 8);
    root.set_margin_top(12);
    root.set_margin_bottom(12);
    root.set_margin_start(12);
    root.set_margin_end(12);

    let title = gtk::Label::new(Some("Running Services"));
    title.set_xalign(0.0);
    title.add_css_class("title-2");

    let summary = gtk::Label::new(Some("Loading..."));
    summary.set_xalign(0.0);
    summary.add_css_class("dim-label");

    let list = gtk::ListBox::new();
    list.add_css_class("boxed-list");
    list.set_selection_mode(gtk::SelectionMode::None);

    let scroll = gtk::ScrolledWindow::builder().child(&list).vexpand(true).hexpand(true).build();

    root.append(&title);
    root.append(&summary);
    root.append(&scroll);
    (list, summary, root)
}

pub(crate) fn rebuild_apps_list(list: &gtk::ListBox, summary: &gtk::Label) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }

    let rows = fetch_apps();
    summary.set_text(&format!("{} apps detectados", rows.len()));

    for proc in rows {
        let row = gtk::ListBoxRow::new();
        let line = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        line.set_margin_top(6);
        line.set_margin_bottom(6);
        line.set_margin_start(8);
        line.set_margin_end(8);

        let icon_name = app_icon_for_process(&proc.name);
        let icon = gtk::Image::from_icon_name(&icon_name);
        icon.set_pixel_size(16);

        let name_lbl = gtk::Label::new(Some(&proc.name));
        name_lbl.set_xalign(0.0);
        name_lbl.set_hexpand(true);

        let user_lbl = gtk::Label::new(Some(&proc.user));
        user_lbl.add_css_class("monospace");

        let pid_lbl = gtk::Label::new(Some(&proc.pid.to_string()));
        pid_lbl.add_css_class("monospace");
        let cpu_lbl = gtk::Label::new(Some(&proc.cpu));
        cpu_lbl.add_css_class("monospace");
        let mem_lbl = gtk::Label::new(Some(&proc.mem));
        mem_lbl.add_css_class("monospace");

        let kill_btn = gtk::Button::with_label("Encerrar");
        kill_btn.add_css_class("destructive-action");
        let list_ref = list.clone();
        let summary_ref = summary.clone();
        let pid = proc.pid;
        kill_btn.connect_clicked(move |_| {
            summary_ref.set_text(&format!("Encerrando PID {}...", pid));

            let list_ui = list_ref.clone();
            let summary_ui = summary_ref.clone();
            let (tx, rx) = std::sync::mpsc::channel::<String>();

            glib::timeout_add_local(std::time::Duration::from_millis(120), move || {
                if let Ok(msg) = rx.try_recv() {
                    summary_ui.set_text(&msg);
                    rebuild_apps_list(&list_ui, &summary_ui);
                    glib::ControlFlow::Break
                } else {
                    glib::ControlFlow::Continue
                }
            });

            std::thread::spawn(move || {
                let msg = match terminate_process(pid) {
                    Ok(info) => info,
                    Err(err) => format!("Falha ao encerrar PID {}: {}", pid, err),
                };
                let _ = tx.send(msg);
            });
        });

        line.append(&icon);
        line.append(&name_lbl);
        line.append(&user_lbl);
        line.append(&pid_lbl);
        line.append(&cpu_lbl);
        line.append(&mem_lbl);
        line.append(&kill_btn);
        row.set_child(Some(&line));
        list.append(&row);
    }
}

pub(crate) fn rebuild_services_list(list: &gtk::ListBox, summary: &gtk::Label) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }

    let rows = fetch_services();
    summary.set_text(&format!("{} serviços em execução", rows.len()));

    for (unit, desc) in rows {
        let row = gtk::ListBoxRow::new();
        let line = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        line.set_margin_top(6);
        line.set_margin_bottom(6);
        line.set_margin_start(8);
        line.set_margin_end(8);

        let icon_name = service_icon_for_unit(&unit);
        let icon = gtk::Image::from_icon_name(&icon_name);
        icon.set_pixel_size(16);

        let unit_lbl = gtk::Label::new(Some(&unit));
        unit_lbl.add_css_class("monospace");
        unit_lbl.set_xalign(0.0);
        unit_lbl.set_hexpand(true);

        let desc_lbl = gtk::Label::new(Some(&desc));
        desc_lbl.set_xalign(0.0);
        desc_lbl.set_wrap(true);

        line.append(&icon);
        line.append(&unit_lbl);
        line.append(&desc_lbl);
        row.set_child(Some(&line));
        list.append(&row);
    }
}

fn fetch_apps() -> Vec<AppProcess> {
    let output = Command::new("sh")
        .arg("-c")
        .arg("ps -eo comm,user,pid,%cpu,%mem --sort=-%mem | head -n 31")
        .output();

    let Ok(output) = output else {
        return vec![AppProcess {
            name: "Apps indisponíveis".to_string(),
            user: "-".to_string(),
            pid: 0,
            cpu: "-".to_string(),
            mem: "-".to_string(),
        }];
    };

    let text = String::from_utf8_lossy(&output.stdout);
    let mut rows = Vec::new();
    for line in text.lines().skip(1) {
        let cols = line.split_whitespace().collect::<Vec<_>>();
        if cols.len() < 5 {
            continue;
        }

        let Ok(pid) = cols[2].parse::<u32>() else {
            continue;
        };

        rows.push(AppProcess {
            name: cols[0].to_string(),
            user: cols[1].to_string(),
            pid,
            cpu: format!("{}%", cols[3]),
            mem: format!("{}%", cols[4]),
        });
    }
    rows
}

fn terminate_process(pid: u32) -> Result<String, String> {
    let term = Command::new("/bin/kill")
        .arg("-TERM")
        .arg(pid.to_string())
        .output()
        .map_err(|e| format!("kill não disponível: {}", e))?;

    if term.status.success() {
        return Ok(format!("PID {} encerrado", pid));
    }

    let privileged = terminate_process_as_admin(pid);
    if privileged.is_ok() {
        return Ok(format!("PID {} encerrado com permissão admin", pid));
    }

    let err = String::from_utf8_lossy(&term.stderr);
    if err.trim().is_empty() {
        Err("permissão negada ou processo inexistente".to_string())
    } else {
        Err(err.trim().to_string())
    }
}

fn terminate_process_as_admin(pid: u32) -> Result<(), String> {
    let launcher = detect_admin_launcher().ok_or_else(|| {
        "sem mecanismo admin disponível (pkexec/run0/sudo)".to_string()
    })?;

    let mut cmd = Command::new(&launcher[0]);
    for arg in launcher.iter().skip(1) {
        cmd.arg(arg);
    }
    let status = cmd
        .arg("/bin/kill")
        .arg("-TERM")
        .arg(pid.to_string())
        .status()
        .map_err(|e| format!("falha ao solicitar admin: {}", e))?;

    if status.success() {
        return Ok(());
    }

    let mut cmd_kill = Command::new(&launcher[0]);
    for arg in launcher.iter().skip(1) {
        cmd_kill.arg(arg);
    }
    let force = cmd_kill
        .arg("/bin/kill")
        .arg("-KILL")
        .arg(pid.to_string())
        .status()
        .map_err(|e| format!("falha no kill forçado: {}", e))?;

    if force.success() {
        Ok(())
    } else {
        Err("não foi possível encerrar processo com privilégio admin".to_string())
    }
}

fn detect_admin_launcher() -> Option<Vec<String>> {
    let in_flatpak = std::env::var("FLATPAK_ID").is_ok();

    if in_flatpak && command_exists("flatpak-spawn") {
        return Some(vec!["flatpak-spawn".to_string(), "--host".to_string(), "pkexec".to_string()]);
    }
    if command_exists("pkexec") {
        return Some(vec!["pkexec".to_string()]);
    }
    if command_exists("run0") {
        return Some(vec!["run0".to_string()]);
    }
    if command_exists("sudo") {
        return Some(vec!["sudo".to_string()]);
    }
    None
}

fn command_exists(command: &str) -> bool {
    Command::new("sh")
        .arg("-c")
        .arg(format!("command -v {} >/dev/null 2>&1", command))
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn fetch_services() -> Vec<(String, String)> {
    let output = Command::new("sh")
        .arg("-c")
        .arg("systemctl list-units --type=service --state=running --no-legend --no-pager")
        .output();

    let Ok(output) = output else {
        return vec![("serviços indisponíveis".to_string(), "systemctl não acessível".to_string())];
    };

    let text = String::from_utf8_lossy(&output.stdout);
    let mut rows = Vec::new();
    for line in text.lines().take(80) {
        let cols = line.split_whitespace().collect::<Vec<_>>();
        if cols.len() < 5 {
            continue;
        }
        let unit = cols[0].to_string();
        let desc = cols[4..].join(" ");
        rows.push((unit, desc));
    }
    rows
}
