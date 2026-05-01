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
flatpak-builder --jobs=1 --user --install --force-clean --delete-build-dirs --state-dir .flatpak-state build-flatpak flatpak/io.github.usuario.LinuxHWMonitor.yml
flatpak run io.github.usuario.LinuxHWMonitor
```

Observação: se o build Flatpak falhar na fase de instalação com a mensagem `File 'linux-hw-monitor' could not be found`, use temporariamente `cargo run` para execução local enquanto o fluxo de empacotamento é ajustado.

### Limpeza segura de cache/estado Flatpak

```bash
make flatpak-clean
```

Esse comando remove apenas diretórios temporários de build (`.flatpak-builder`, `.flatpak-state` e `build-flatpak`) e recria `build-flatpak` vazia.

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
