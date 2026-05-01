#!/usr/bin/env bash
set -euo pipefail

APP_ID="io.github.vitao_al.linux-hw-monitor"

section() {
  printf "\n== %s ==\n" "$1"
}

check_cmd() {
  local cmd="$1"
  if command -v "$cmd" >/dev/null 2>&1; then
    printf "%-18s OK (%s)\n" "$cmd" "$(command -v "$cmd")"
  else
    printf "%-18s MISSING\n" "$cmd"
  fi
}

section "Tools"
for t in cargo rustc meson ninja flatpak flatpak-builder pkg-config; do
  check_cmd "$t"
done

section "Flatpak remotes"
if command -v flatpak >/dev/null 2>&1; then
  flatpak remotes || true
else
  echo "flatpak not installed"
fi

section "GNOME runtimes"
if command -v flatpak >/dev/null 2>&1; then
  flatpak list --runtime | grep -E "org.gnome.(Sdk|Platform)" || echo "No GNOME SDK/Platform installed"
fi

section "Rust setup"
if command -v rustc >/dev/null 2>&1; then
  rustc --version || true
fi
if command -v cargo >/dev/null 2>&1; then
  cargo --version || true
fi

section "Project checks"
if [[ -f "Cargo.toml" ]]; then
  echo "Cargo.toml found"
else
  echo "Cargo.toml not found"
fi

if [[ -f "flatpak/io.github.vitao_al.linux-hw-monitor.yml" ]]; then
  echo "Flatpak manifest found"
else
  echo "Flatpak manifest not found"
fi

section "Flatpak app"
if command -v flatpak >/dev/null 2>&1; then
  flatpak info "$APP_ID" >/dev/null 2>&1 && echo "${APP_ID} installed" || echo "${APP_ID} not installed"
fi
