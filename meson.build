project('mirrorman', version: '0.2.0',
  meson_version: '>= 0.50.0',
  default_options: ['warning_level=0']
)

gtk4_dep = dependency('gtk4', version: '>=4.16')
adw_dep = dependency('libadwaita-1', version: '>=1.4')
pygi_dep = dependency('gobject-introspection-1.0', required: false)

message('mirrorman v0.2.0')
message('GTK4: ' + gtk4_dep.version())
message('Adwaita: ' + adw_dep.version())
message('')
message('To run: python3 src/main.py')
message('To build: Just use the source files directly.')

install_data('mirrorman.desktop',
  install_dir: 'share/applications'
)