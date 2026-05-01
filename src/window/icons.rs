use gtk4 as gtk;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use crate::sensors::types::SensorGroup;

pub(crate) fn preferred_icon_for_group(group: &SensorGroup) -> String {
    match group.id.as_str() {
        "cpu" => best_icon_name(
            &["cpu-symbolic", "processor-symbolic", "utilities-system-monitor-symbolic"],
            "applications-system-symbolic",
        ),
        "gpu" => best_icon_name(
            &["video-card-symbolic", "video-display-symbolic", "computer-symbolic"],
            "applications-system-symbolic",
        ),
        "memory" => best_icon_name(
            &["media-flash-symbolic", "drive-harddisk-symbolic", "computer-symbolic"],
            "applications-system-symbolic",
        ),
        "storage" => best_icon_name(
            &["drive-harddisk-symbolic", "drive-removable-media-symbolic"],
            "applications-system-symbolic",
        ),
        "battery" => best_icon_name(
            &["battery-good-symbolic", "battery-symbolic"],
            "applications-system-symbolic",
        ),
        "network" => best_icon_name(
            &["network-wired-symbolic", "network-workgroup-symbolic"],
            "applications-system-symbolic",
        ),
        "fans" => best_icon_name(
            &["weather-windy-symbolic", "preferences-system-symbolic"],
            "applications-system-symbolic",
        ),
        "motherboard" => best_icon_name(
            &["computer-symbolic", "drive-harddisk-system-symbolic"],
            "applications-system-symbolic",
        ),
        _ => best_icon_name(
            &["applications-system-symbolic", "application-x-executable-symbolic"],
            "applications-system-symbolic",
        ),
    }
}

pub(crate) fn best_icon_name(candidates: &[&str], fallback: &str) -> String {
    let Some(display) = gtk::gdk::Display::default() else {
        return fallback.to_string();
    };

    let theme = gtk::IconTheme::for_display(&display);
    for candidate in candidates {
        if theme.has_icon(candidate) {
            return (*candidate).to_string();
        }
    }
    fallback.to_string()
}

pub(crate) fn app_icon_for_process(name: &str, pid: u32) -> String {
    let mut candidates: Vec<String> = Vec::new();
    if let Some(exe_name) = process_exe_name(pid) {
        candidates.push(exe_name);
    }
    if let Some(cmd_name) = process_cmd_name(pid) {
        candidates.push(cmd_name);
    }
    candidates.push(name.to_string());

    for cand in candidates {
        if let Some(icon) = desktop_icon_for_exec(&cand) {
            return icon;
        }
    }

    let lower = name.to_lowercase();
    if lower.contains("firefox") || lower.contains("chrome") || lower.contains("browser") {
        return best_icon_name(
            &["firefox", "google-chrome", "chromium", "web-browser", "applications-internet"],
            "application-x-executable-symbolic",
        );
    }
    if lower.contains("code") || lower.contains("editor") || lower.contains("vim") {
        return best_icon_name(
            &["code", "org.gnome.TextEditor", "accessories-text-editor", "text-x-generic"],
            "application-x-executable-symbolic",
        );
    }
    if lower.contains("steam") || lower.contains("game") {
        return best_icon_name(
            &["steam", "applications-games", "application-x-executable"],
            "application-x-executable-symbolic",
        );
    }
    if lower.contains("gnome-shell") || lower.contains("plasmashell") {
        return best_icon_name(
            &["desktop-symbolic", "computer-symbolic"],
            "application-x-executable-symbolic",
        );
    }
    if lower.contains("sh") || lower.contains("bash") || lower.contains("zsh") || lower.contains("terminal") {
        return best_icon_name(
            &["utilities-terminal-symbolic", "system-run-symbolic"],
            "application-x-executable-symbolic",
        );
    }

    best_icon_name(
        &["application-x-executable-symbolic", "applications-system-symbolic"],
        "application-x-executable-symbolic",
    )
}

pub(crate) fn service_icon_for_unit(unit: &str) -> String {
    let clean_unit = unit.trim_end_matches(" (user)");
    let service_name = clean_unit.trim_end_matches(".service");

    if let Some(icon) = desktop_icon_for_exec(service_name) {
        return icon;
    }
    if let Some(head) = service_name.split('-').next() {
        if let Some(icon) = desktop_icon_for_exec(head) {
            return icon;
        }
    }

    let lower = unit.to_lowercase();
    if lower.contains("network") || lower.contains("networkmanager") {
        return best_icon_name(
            &["network-wired-symbolic", "network-workgroup-symbolic"],
            "system-run-symbolic",
        );
    }
    if lower.contains("bluetooth") {
        return best_icon_name(&["bluetooth-symbolic", "system-run-symbolic"], "system-run-symbolic");
    }
    if lower.contains("pipewire") || lower.contains("pulseaudio") || lower.contains("alsa") {
        return best_icon_name(
            &["audio-headphones-symbolic", "audio-speakers-symbolic"],
            "system-run-symbolic",
        );
    }
    if lower.contains("docker") || lower.contains("podman") {
        return best_icon_name(&["folder-remote-symbolic", "system-run-symbolic"], "system-run-symbolic");
    }
    if lower.contains("ssh") {
        return best_icon_name(
            &["network-server-symbolic", "network-workgroup-symbolic"],
            "system-run-symbolic",
        );
    }
    if lower.contains("cups") || lower.contains("print") {
        return best_icon_name(&["printer-symbolic", "system-run-symbolic"], "system-run-symbolic");
    }

    best_icon_name(
        &["system-run-symbolic", "applications-system-symbolic"],
        "system-run-symbolic",
    )
}

fn process_exe_name(pid: u32) -> Option<String> {
    let path = format!("/proc/{pid}/exe");
    let exe = fs::read_link(path).ok()?;
    basename_string(&exe)
}

fn process_cmd_name(pid: u32) -> Option<String> {
    let path = format!("/proc/{pid}/cmdline");
    let raw = fs::read(path).ok()?;
    let first = raw.split(|b| *b == 0).next()?;
    if first.is_empty() {
        return None;
    }
    let cmd = String::from_utf8_lossy(first).to_string();
    let p = PathBuf::from(cmd);
    basename_string(&p)
}

fn basename_string(path: &Path) -> Option<String> {
    path.file_name()
        .and_then(OsStr::to_str)
        .map(|s| s.to_string())
}

fn desktop_icon_for_exec(exec_hint: &str) -> Option<String> {
    let key = exec_hint
        .trim()
        .trim_matches('"')
        .trim_end_matches(".service")
        .rsplit('/')
        .next()?
        .to_lowercase();
    if key.is_empty() {
        return None;
    }

    let map = desktop_icon_index();
    let icon = map.get(&key)?;

    let display = gtk::gdk::Display::default()?;
    let theme = gtk::IconTheme::for_display(&display);
    if theme.has_icon(icon) {
        return Some(icon.clone());
    }

    None
}

fn desktop_icon_index() -> &'static HashMap<String, String> {
    static INDEX: OnceLock<HashMap<String, String>> = OnceLock::new();
    INDEX.get_or_init(build_desktop_icon_index)
}

fn build_desktop_icon_index() -> HashMap<String, String> {
    let mut out = HashMap::new();

    let mut roots = vec![
        PathBuf::from("/usr/share/applications"),
        PathBuf::from("/usr/local/share/applications"),
        PathBuf::from("/var/lib/flatpak/exports/share/applications"),
    ];
    if let Ok(home) = std::env::var("HOME") {
        roots.push(PathBuf::from(format!("{home}/.local/share/applications")));
        roots.push(PathBuf::from(format!("{home}/.local/share/flatpak/exports/share/applications")));
    }

    for root in roots {
        visit_desktop_dir(&root, &mut out);
    }

    out
}

fn visit_desktop_dir(root: &Path, out: &mut HashMap<String, String>) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };

    for ent in entries.flatten() {
        let path = ent.path();
        if path.is_dir() {
            visit_desktop_dir(&path, out);
            continue;
        }
        if path.extension().and_then(OsStr::to_str) != Some("desktop") {
            continue;
        }
        index_desktop_file(&path, out);
    }
}

fn index_desktop_file(path: &Path, out: &mut HashMap<String, String>) {
    let Ok(text) = fs::read_to_string(path) else {
        return;
    };

    let mut icon = None::<String>;
    let mut exec = None::<String>;
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('#') {
            continue;
        }
        if icon.is_none() && line.starts_with("Icon=") {
            let v = line.trim_start_matches("Icon=").trim();
            if !v.is_empty() && !v.starts_with('/') {
                icon = Some(v.to_string());
            }
        }
        if exec.is_none() && line.starts_with("Exec=") {
            let v = line.trim_start_matches("Exec=").trim();
            if let Some(parsed) = parse_exec_name(v) {
                exec = Some(parsed);
            }
        }
        if icon.is_some() && exec.is_some() {
            break;
        }
    }

    let Some(icon_name) = icon else {
        return;
    };

    if let Some(exec_name) = exec {
        out.entry(exec_name.to_lowercase()).or_insert_with(|| icon_name.clone());
    }

    if let Some(stem) = path.file_stem().and_then(OsStr::to_str) {
        let stem_key = stem.split('.').next_back().unwrap_or(stem).to_lowercase();
        out.entry(stem_key).or_insert(icon_name);
    }
}

fn parse_exec_name(exec_line: &str) -> Option<String> {
    let mut tokens = exec_line
        .split_whitespace()
        .map(|s| s.trim_matches('"'))
        .filter(|s| !s.is_empty());

    let mut cmd = tokens.next()?;
    if cmd == "env" {
        for tok in tokens.by_ref() {
            if tok.contains('=') {
                continue;
            }
            cmd = tok;
            break;
        }
    }
    if cmd.is_empty() {
        return None;
    }

    let cmd = cmd.split('/').next_back().unwrap_or(cmd);
    let cmd = cmd.split('%').next().unwrap_or(cmd).trim();
    if cmd.is_empty() {
        None
    } else {
        Some(cmd.to_string())
    }
}
