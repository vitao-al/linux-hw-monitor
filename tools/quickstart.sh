#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

APP_ID="io.github.vitao_al.linux-hw-monitor"
MANIFEST="${PROJECT_ROOT}/flatpak/io.github.vitao_al.linux-hw-monitor.yml"
BUILD_DIR="${PROJECT_ROOT}/build-flatpak"
STATE_DIR="${PROJECT_ROOT}/.flatpak-state"
TMP_MANIFEST="${PROJECT_ROOT}/.flatpak.manifest.autogen.yml"

log() {
  printf "[linux-hw-monitor] %s\n" "$*"
}

need_cmd() {
  command -v "$1" >/dev/null 2>&1
}

install_base_tools() {
  if need_cmd flatpak && need_cmd flatpak-builder; then
    return 0
  fi

  if [[ "${EUID}" -ne 0 ]]; then
    SUDO="sudo"
  else
    SUDO=""
  fi

  if need_cmd dnf; then
    log "Instalando ferramentas com dnf"
    ${SUDO} dnf install -y flatpak flatpak-builder
    return 0
  fi

  if need_cmd apt-get; then
    log "Instalando ferramentas com apt"
    ${SUDO} apt-get update
    ${SUDO} apt-get install -y flatpak flatpak-builder
    return 0
  fi

  if need_cmd pacman; then
    log "Instalando ferramentas com pacman"
    ${SUDO} pacman -S --noconfirm --needed flatpak flatpak-builder
    return 0
  fi

  if need_cmd zypper; then
    log "Instalando ferramentas com zypper"
    ${SUDO} zypper install -y flatpak flatpak-builder
    return 0
  fi

  log "Nao foi possivel detectar o gerenciador de pacotes automaticamente."
  log "Instale manualmente: flatpak e flatpak-builder"
  exit 1
}

ensure_flathub() {
  if ! flatpak remotes --user --columns=name | grep -qx "flathub"; then
    log "Adicionando Flathub"
    flatpak remote-add --user --if-not-exists flathub https://flathub.org/repo/flathub.flatpakrepo
  fi
}

install_runtimes() {
  local sdk_ok=0
  local platform_ok=0

  log "Tentando instalar runtime GNOME 47"
  if flatpak install --user -y flathub org.gnome.Sdk//47; then
    sdk_ok=47
  fi
  if flatpak install --user -y flathub org.gnome.Platform//47; then
    platform_ok=47
  fi

  if [[ "$sdk_ok" == "0" || "$platform_ok" == "0" ]]; then
    log "GNOME 47 indisponivel. Tentando GNOME 46"
    if flatpak install --user -y flathub org.gnome.Sdk//46 && flatpak install --user -y flathub org.gnome.Platform//46; then
      sdk_ok=46
      platform_ok=46
    fi
  fi

  if [[ "$sdk_ok" == "0" || "$platform_ok" == "0" ]]; then
    log "Falha ao instalar org.gnome.Sdk/org.gnome.Platform nas versoes 47 ou 46"
    exit 1
  fi

  log "Instalando extensao Rust stable (24.08 -> 23.08 fallback)"
  flatpak install --user -y flathub org.freedesktop.Sdk.Extension.rust-stable//24.08 \
    || flatpak install --user -y flathub org.freedesktop.Sdk.Extension.rust-stable//23.08 \
    || true

  RUNTIME_VERSION="$sdk_ok"
  export RUNTIME_VERSION
}

prepare_manifest() {
  local original_runtime
  original_runtime=$(awk -F"'" '/^runtime-version:/ {print $2}' "$MANIFEST" || true)

  if [[ -z "${original_runtime}" ]]; then
    cp "$MANIFEST" "$TMP_MANIFEST"
    return 0
  fi

  if [[ "$original_runtime" == "$RUNTIME_VERSION" ]]; then
    cp "$MANIFEST" "$TMP_MANIFEST"
    return 0
  fi

  log "Ajustando manifesto temporario: runtime-version ${original_runtime} -> ${RUNTIME_VERSION}"
  sed "s/runtime-version: '${original_runtime}'/runtime-version: '${RUNTIME_VERSION}'/" "$MANIFEST" > "$TMP_MANIFEST"
}

build_and_run() {
  log "Buildando Flatpak"
  # Remove legacy default state dir to avoid old oversized caches.
  rm -rf "${PROJECT_ROOT}/.flatpak-builder"

  flatpak-builder \
    --jobs=1 \
    --user \
    --install \
    --force-clean \
    --delete-build-dirs \
    --state-dir "$STATE_DIR" \
    "$BUILD_DIR" \
    "$TMP_MANIFEST"

  log "Abrindo UI"
  flatpak run "$APP_ID"
}

cleanup() {
  rm -f "$TMP_MANIFEST"
}

main() {
  trap cleanup EXIT

  cd "${PROJECT_ROOT}"

  if [[ ! -f "$MANIFEST" ]]; then
    log "Manifesto nao encontrado em ${MANIFEST}"
    exit 1
  fi

  install_base_tools
  ensure_flathub
  install_runtimes
  prepare_manifest
  build_and_run
}

main "$@"
