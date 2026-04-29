#!/usr/bin/env python3
import os
import subprocess
import sys

prefix = os.environ.get('MESON_INSTALL_PREFIX', '/usr/local')
datadir = os.path.join(prefix, 'share')

if not os.environ.get('DESTDIR'):
    subprocess.call(['glib-compile-schemas', os.path.join(datadir, 'glib-2.0', 'schemas')])
    subprocess.call(['update-desktop-database', os.path.join(datadir, 'applications')])

sys.exit(0)
