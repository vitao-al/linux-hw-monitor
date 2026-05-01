.PHONY: quickstart diagnose flatpak-build flatpak-run

quickstart:
	./tools/quickstart.sh

diagnose:
	./tools/diagnose.sh

flatpak-build:
	flatpak-builder --jobs=1 --user --install --force-clean build-flatpak flatpak/io.github.usuario.LinuxHWMonitor.yml

flatpak-run:
	flatpak run io.github.usuario.LinuxHWMonitor
