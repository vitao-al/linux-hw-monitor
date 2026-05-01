use gtk4 as gtk;

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

pub(crate) fn app_icon_for_process(name: &str) -> String {
    let lower = name.to_lowercase();
    if lower.contains("firefox") || lower.contains("chrome") || lower.contains("browser") {
        return best_icon_name(
            &["web-browser-symbolic", "applications-internet-symbolic"],
            "application-x-executable-symbolic",
        );
    }
    if lower.contains("code") || lower.contains("editor") || lower.contains("vim") {
        return best_icon_name(
            &["accessories-text-editor-symbolic", "text-x-generic-symbolic"],
            "application-x-executable-symbolic",
        );
    }
    if lower.contains("steam") || lower.contains("game") {
        return best_icon_name(
            &["applications-games-symbolic", "application-x-executable-symbolic"],
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
