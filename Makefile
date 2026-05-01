.PHONY: quickstart diagnose flatpak-build flatpak-run flatpak-clean

quickstart:
	./tools/quickstart.sh

diagnose:
	./tools/diagnose.sh

flatpak-build:
	flatpak-builder --jobs=1 --user --install --force-clean --delete-build-dirs --state-dir .flatpak-state build-flatpak flatpak/io.github.usuario.LinuxHWMonitor.yml

flatpak-run:
	flatpak run io.github.usuario.LinuxHWMonitor

flatpak-clean:
	rm -rf .flatpak-builder .flatpak-state build-flatpak
	mkdir -p build-flatpak
