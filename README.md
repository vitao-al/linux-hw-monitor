# LinuxHWMonitor

Linux hardware monitor inspired by HWMonitor/HWiNFO, built with Rust + GTK4 + Libadwaita.

## One command for all distros

For first UI tests with automatic dependency and runtime handling:

./tools/quickstart.sh

What this script does automatically:

- Installs `flatpak` and `flatpak-builder` on Fedora, Ubuntu/Debian, Arch and openSUSE.
- Adds Flathub remote in user scope if needed.
- Installs GNOME SDK/Platform 47 in user scope, and falls back to 46 when 47 is unavailable.
- Tries Rust extension branch 24.08 and falls back to 23.08.
- Builds and installs the app with Flatpak.
- Launches the UI.

This path is designed to work without root for runtime installation (`flatpak --user`).

Optional shortcut:

make quickstart

Environment diagnostics:

./tools/diagnose.sh

Or:

make diagnose

## Quick Start (first tests)

### 1) Install dependencies

#### Fedora 39+

```bash
sudo dnf install -y \
  rust cargo \
  gcc \
  meson ninja-build \
  pkgconf-pkg-config \
  gtk4-devel libadwaita-devel \
  glib2-devel
```

#### Ubuntu 22.04+

```bash
sudo apt update
sudo apt install -y \
  rustc cargo \
  build-essential \
  meson ninja-build \
  pkg-config \
  libgtk-4-dev libadwaita-1-dev \
  libglib2.0-dev
```

#### Arch Linux

```bash
sudo pacman -S --needed \
  rust cargo \
  base-devel \
  meson ninja pkgconf \
  gtk4 libadwaita glib2
```

### 2) Run locally with Cargo

From project root:

```bash
cargo run
```

This opens the GTK app and starts live sensor polling.

### 3) Run tests

```bash
cargo test
```

### 4) Build with Meson (same layout used by packaging)

```bash
meson setup builddir
meson compile -C builddir
./builddir/linux-hw-monitor
```

## Flatpak test build

Install builder tools first:

### Fedora

```bash
sudo dnf install -y flatpak flatpak-builder
```

### Ubuntu

```bash
sudo apt install -y flatpak flatpak-builder
```

### Arch

```bash
sudo pacman -S --needed flatpak flatpak-builder
```

Then build and run:

```bash
flatpak-builder --user --install --force-clean build-flatpak flatpak/io.github.usuario.LinuxHWMonitor.yml
flatpak run io.github.usuario.LinuxHWMonitor
```

## Notes for early tests

- Most sensors work without root.
- SMART and DMI data require the privileged helper and proper polkit setup.
- If NVIDIA data is empty, verify `nvidia-smi` works outside the app.
- In sandboxed Flatpak runs, sensor visibility depends on `finish-args` filesystem access.

## Current environment status (this workspace)

Tool check showed:

- `cargo`: missing
- `rustc`: missing
- `meson`: missing
- `ninja`: missing
- `flatpak`: installed
- `flatpak-builder`: missing

Install prerequisites first, then run `cargo run` for the first smoke test.
