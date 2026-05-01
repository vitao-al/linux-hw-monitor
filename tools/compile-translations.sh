#!/usr/bin/env bash
# compile-translations.sh — Compile .po files to .mo and optionally install them.
#
# Usage:
#   ./tools/compile-translations.sh             # compile only → po/mo/
#   ./tools/compile-translations.sh --install   # compile + install to /usr/share/locale
#
# Requirements: msgfmt (gettext package)

set -euo pipefail

APP_ID="io.github.vitao_al.linux-hw-monitor"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
PO_DIR="${REPO_ROOT}/po"
MO_OUTDIR="${PO_DIR}/mo"
INSTALL_PREFIX="${INSTALL_PREFIX:-/usr}"
LOCALE_INSTALL="${INSTALL_PREFIX}/share/locale"

INSTALL=0
for arg in "$@"; do
    [[ "$arg" == "--install" ]] && INSTALL=1
done

if ! command -v msgfmt &>/dev/null; then
    echo "Error: msgfmt not found. Install the 'gettext' package." >&2
    exit 1
fi

echo "Compiling translations for ${APP_ID}…"
mkdir -p "${MO_OUTDIR}"

COMPILED=0
for po_file in "${PO_DIR}"/*.po; do
    lang="$(basename "${po_file}" .po)"
    mo_dir="${MO_OUTDIR}/${lang}/LC_MESSAGES"
    mkdir -p "${mo_dir}"
    mo_file="${mo_dir}/${APP_ID}.mo"

    msgfmt --output-file="${mo_file}" "${po_file}"
    echo "  [OK] ${lang} → ${mo_file}"
    COMPILED=$((COMPILED + 1))

    if [[ "$INSTALL" == "1" ]]; then
        install_dir="${LOCALE_INSTALL}/${lang}/LC_MESSAGES"
        sudo mkdir -p "${install_dir}"
        sudo cp "${mo_file}" "${install_dir}/${APP_ID}.mo"
        echo "       installed → ${install_dir}/${APP_ID}.mo"
    fi
done

echo "Done. ${COMPILED} language(s) compiled."
if [[ "$INSTALL" == "1" ]]; then
    echo "Translations installed to ${LOCALE_INSTALL}."
fi
