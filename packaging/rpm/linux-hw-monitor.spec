Name:           linux-hw-monitor
Version:        1.0.0
Release:        1%{?dist}
Summary:        Linux hardware monitor in Rust + GTK4 + libadwaita
License:        MIT
URL:            https://github.com/vitao-al/linux-hw-monitor
Source0:        %{url}/archive/refs/tags/v%{version}.tar.gz

BuildRequires:  meson
BuildRequires:  ninja-build
BuildRequires:  rust
BuildRequires:  cargo
BuildRequires:  gcc
BuildRequires:  pkgconfig(gtk4)
BuildRequires:  pkgconfig(libadwaita-1)
BuildRequires:  pkgconfig(glib-2.0)

Requires:       gtk4
Requires:       libadwaita
Requires:       glib2

%description
Linux HW Monitor monitora sensores, CPU/GPU, memoria, discos e rede em tempo real.

%prep
%autosetup -n linux-hw-monitor-%{version}

%build
%meson -Dprofile=release
%meson_build

%install
%meson_install

%files
%license LICENSE
%{_bindir}/linux-hw-monitor
%{_libexecdir}/linux-hw-monitor-helper
%{_datadir}/applications/io.github.usuario.LinuxHWMonitor.desktop
%{_datadir}/metainfo/io.github.usuario.LinuxHWMonitor.appdata.xml
%{_datadir}/polkit-1/actions/io.github.usuario.LinuxHWMonitor.policy
%{_datadir}/glib-2.0/schemas/io.github.usuario.LinuxHWMonitor.gschema.xml
%{_datadir}/icons/hicolor/scalable/apps/io.github.usuario.LinuxHWMonitor.svg
%{_datadir}/icons/hicolor/symbolic/apps/io.github.usuario.LinuxHWMonitor-symbolic.svg

%changelog
* Thu May 01 2026 vitao-al <vitao@example.com> - 1.0.0-1
- Initial package
