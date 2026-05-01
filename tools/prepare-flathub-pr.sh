#!/usr/bin/env bash
set -euo pipefail

APP_ID="io.github.usuario.LinuxHWMonitor"
WORKDIR="${1:-$PWD/.flathub-pr}"

if ! command -v git >/dev/null 2>&1; then
  echo "git nao encontrado" >&2
  exit 1
fi

rm -rf "$WORKDIR"
mkdir -p "$WORKDIR"

git clone --depth=1 https://github.com/flathub/flathub.git "$WORKDIR/flathub"
mkdir -p "$WORKDIR/flathub/$APP_ID"
cp "$PWD/packaging/flathub/$APP_ID.flathub.yml" "$WORKDIR/flathub/$APP_ID/$APP_ID.yml"

cat > "$WORKDIR/flathub/$APP_ID/README" << 'EOF'
Maintainer repo: https://github.com/vitao-al/linux-hw-monitor
EOF

echo

echo "PR base preparada em: $WORKDIR/flathub"
echo "Proximos passos:"
echo "  1) cd $WORKDIR/flathub"
echo "  2) git checkout -b add-$APP_ID"
echo "  3) git add $APP_ID"
echo "  4) git commit -m 'Add $APP_ID'"
echo "  5) git remote add fork <URL_DO_SEU_FORK_FLATHUB>"
echo "  6) git push fork add-$APP_ID"
echo "  7) Abrir PR no flathub/flathub"
