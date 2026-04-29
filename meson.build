project('linux-hw-monitor', 'rust', version: '1.0.0')

profile = get_option('profile')

cargo = find_program('cargo', required: true)

if profile == 'release'
  cargo_args = ['build', '--release']
  target_dir = 'release'
else
  cargo_args = ['build']
  target_dir = 'debug'
endif

custom_target(
  'linux-hw-monitor-bin',
  output: 'linux-hw-monitor',
  command: [
    'sh',
    '-c',
    ' '.join([
      cargo.full_path(),
      ' '.join(cargo_args),
      '&& cp target/' + target_dir + '/linux-hw-monitor @OUTPUT@'
    ])
  ],
  build_by_default: true,
  install: true,
  install_dir: get_option('bindir')
)

custom_target(
  'linux-hw-monitor-helper-bin',
  output: 'linux-hw-monitor-helper',
  command: [
    'sh',
    '-c',
    'cd helper && ' + ' '.join([
      cargo.full_path(),
      ' '.join(cargo_args),
      '&& cp target/' + target_dir + '/linux-hw-monitor-helper @OUTPUT@'
    ])
  ],
  build_by_default: true,
  install: true,
  install_dir: get_option('libexecdir')
)

install_data('data/io.github.usuario.LinuxHWMonitor.desktop', install_dir: get_option('datadir') / 'applications')
install_data('data/io.github.usuario.LinuxHWMonitor.appdata.xml', install_dir: get_option('datadir') / 'metainfo')
install_data('data/io.github.usuario.LinuxHWMonitor.gschema.xml', install_dir: get_option('datadir') / 'glib-2.0' / 'schemas')
install_data('data/io.github.usuario.LinuxHWMonitor.policy', install_dir: get_option('datadir') / 'polkit-1' / 'actions')
install_data('data/icons/hicolor/scalable/apps/io.github.usuario.LinuxHWMonitor.svg', install_dir: get_option('datadir') / 'icons' / 'hicolor' / 'scalable' / 'apps')
install_data('data/icons/hicolor/symbolic/apps/io.github.usuario.LinuxHWMonitor-symbolic.svg', install_dir: get_option('datadir') / 'icons' / 'hicolor' / 'symbolic' / 'apps')

meson.add_install_script('build-aux/meson/postinstall.py')
