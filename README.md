# Linux HW Monitor

Monitor de hardware para Linux inspirado em HWMonitor/HWiNFO, desenvolvido em Rust + GTK4 + libadwaita.

## O que o software faz

- Exibe sensores em tempo real (CPU, GPU, memória, disco, rede, bateria, temperatura e energia, conforme disponibilidade no sistema).
- Mostra medidores principais de CPU/GPU no topo.
- Mantém histórico para gráficos com atualização contínua.
- Possui páginas dedicadas para apps em execução e serviços ativos.
- Permite exportar dados em CSV, JSON e TXT.

## Requisitos

### Dependências principais

- Rust e Cargo
- GTK4 e libadwaita
- Meson + Ninja
- pkg-config

### Pacotes por distro

Fedora:

```bash
sudo dnf install -y rust cargo gcc meson ninja-build pkgconf-pkg-config gtk4-devel libadwaita-devel glib2-devel
```

Ubuntu/Debian:

```bash
sudo apt update
sudo apt install -y rustc cargo build-essential meson ninja-build pkg-config libgtk-4-dev libadwaita-1-dev libglib2.0-dev
```

Arch:

```bash
sudo pacman -S --needed rust cargo base-devel meson ninja pkgconf gtk4 libadwaita glib2
```

## Forma correta de usar o software

### 1) Executar em modo desenvolvimento (recomendado)

No diretório do projeto:

```bash
cargo run
```

Esse é o fluxo mais estável para uso diário durante desenvolvimento.

### 2) Executar via Meson (layout de empacotamento)

```bash
meson setup builddir
meson compile -C builddir
./builddir/linux-hw-monitor
```

### 3) Testar rapidamente dependências e ambiente

```bash
./tools/diagnose.sh
```

Atalho via Makefile:

```bash
make diagnose
```

## Guia de uso da interface

1. Abra a aba Performance para acompanhar sensores e gráficos em tempo real.
2. Clique nos grupos na barra lateral para ver detalhes de cada categoria.
3. Use a aba Apps para inspecionar processos e encerrar PID quando necessário.
4. Use a aba Services para verificar serviços em execução.
5. No cabeçalho, use o botão de exportação para salvar CSV, JSON ou TXT.
6. Em Preferences, ajuste tema, intervalo de atualização e unidades.

## Exportação de dados

- Formatos disponíveis: CSV, JSON e TXT.
- O arquivo é salvo no caminho escolhido no diálogo de salvar.

## Flatpak

### Build automatizado

```bash
./tools/quickstart.sh
```

Ou:

```bash
make quickstart
```

### Build manual

```bash
flatpak-builder --jobs=1 --user --install --force-clean --delete-build-dirs --state-dir .flatpak-state build-flatpak flatpak/io.github.vitao_al.linux-hw-monitor.yml
flatpak run io.github.vitao_al.linux-hw-monitor
```

### Validacao de submissao Flathub (doc oficial)

```bash
flatpak install -y flathub org.flatpak.Builder
flatpak run --command=flathub-build org.flatpak.Builder --install io.github.vitao_al.linux-hw-monitor/io.github.vitao_al.linux-hw-monitor.yml
flatpak run io.github.vitao_al.linux-hw-monitor
flatpak run --command=flatpak-builder-lint org.flatpak.Builder manifest io.github.vitao_al.linux-hw-monitor/io.github.vitao_al.linux-hw-monitor.yml
flatpak run --command=flatpak-builder-lint org.flatpak.Builder repo repo
```

Observacao: para submissao inicial, a PR no flathub/flathub deve abrir contra a branch `new-pr`.

Observação: se o build Flatpak falhar na fase de instalação com a mensagem `File 'linux-hw-monitor' could not be found`, use temporariamente `cargo run` para execução local enquanto o fluxo de empacotamento é ajustado.

### Limpeza segura de cache/estado Flatpak

```bash
make flatpak-clean
```

Esse comando remove apenas diretórios temporários de build (`.flatpak-builder`, `.flatpak-state` e `build-flatpak`) e recria `build-flatpak` vazia.

## Publicacao em lojas Linux

Este repositório agora inclui base de empacotamento para os principais canais Linux:

- Flatpak/Flathub: `packaging/flathub/io.github.vitao_al.linux-hw-monitor.yml`
- Dependency manifest (Cargo): `packaging/flathub/cargo-sources.json`
- Snap Store: `packaging/snap/snapcraft.yaml`
- AUR (Arch): `packaging/aur/PKGBUILD`
- RPM/COPR (Fedora/openSUSE): `packaging/rpm/linux-hw-monitor.spec`
- CI de release: `.github/workflows/release-packages.yml`

### Como publicar por canal

1. Crie uma tag semantica:

```bash
git tag v1.0.1
git push origin v1.0.1
```

2. O workflow de release gera artefatos de distribuicao (tarball, flatpak bundle, snap).

3. Envie para cada loja:

- Flathub: abra PR no repositório Flathub com o manifesto em `packaging/flathub/`.
- Snap Store: execute `snapcraft upload --release=stable <arquivo.snap>` usando token da loja.
- AUR: publique o `PKGBUILD` em um repositório AUR (`linux-hw-monitor`).
- COPR: use o `.spec` em `packaging/rpm/` para criar build automático por tag.

## Icone na grade de aplicativos

O projeto instala:

- Desktop file: `data/io.github.vitao_al.linux-hw-monitor.desktop`
- Icone principal: `data/icons/hicolor/scalable/apps/io.github.vitao_al.linux-hw-monitor.svg`
- Icone simbolico: `data/icons/hicolor/symbolic/apps/io.github.vitao_al.linux-hw-monitor-symbolic.svg`

O script de pos-instalacao atualiza schemas, cache de desktop e cache de icones (`gtk-update-icon-cache`), garantindo aparicao na grade de apps do SO apos instalacao por pacote.

## Dependencias e atualizacoes automaticas

- Flatpak: runtime + dependencias resolvidas pelo Flatpak e atualizacao automatica via loja/`flatpak update`.
- Snap: dependencias resolvidas no pacote Snap e atualizacao automatica via snapd.
- AUR/RPM: dependencias declaradas no pacote; atualizacao via gerenciador da distro (`yay/pacman`, `dnf`, `zypper`, etc).

Nao existe um mecanismo universal de "auto update" para todas as distros fora das lojas. O caminho recomendado e distribuir por lojas/repositorios para herdar o ciclo de atualizacao nativo de cada sistema.

## Testes

```bash
cargo test
```

## Permissões e limitações

- Alguns sensores dependem de permissões adicionais no sistema.
- Encerramento de processos pode exigir autenticação administrativa.
- Em Flatpak, a visibilidade de sensores depende dos `finish-args` do manifesto.

## Estrutura do projeto

- `src/`: aplicação principal.
- `helper/`: helper auxiliar para operações privilegiadas.
- `data/`: desktop file, policy, schemas e ícones.
- `flatpak/`: manifesto Flatpak.
- `tools/`: scripts utilitários (`quickstart` e `diagnose`).
