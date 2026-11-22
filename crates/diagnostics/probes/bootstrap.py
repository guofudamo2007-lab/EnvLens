# Invoked with -I -S -B. Add package directories without executing .pth or project hooks.
import json
import os
import sys

exe_dir = os.path.dirname(os.path.abspath(sys.executable))
parent = os.path.dirname(exe_dir)
env_root = parent if os.path.isfile(os.path.join(parent, "pyvenv.cfg")) else exe_dir
cfg = os.path.join(env_root, "pyvenv.cfg")
is_venv = os.path.isfile(cfg)
system_sites = not is_venv
if is_venv:
    with open(cfg, encoding="utf-8-sig", errors="replace") as handle:
        for line in handle.read(16384).splitlines():
            key, _, value = line.partition("=")
            if key.strip().lower() == "include-system-site-packages":
                system_sites = value.strip().lower() == "true"
    # CPython < 3.14 sets these in site.main(), disabled for this probe.
    sys.prefix = sys.exec_prefix = env_root

def add_packages(root):
    path = os.path.join(root, "Lib", "site-packages")
    if os.path.isdir(path) and path not in sys.path:
        sys.path.append(path)

if is_venv:
    add_packages(env_root)
if system_sites:
    import site
    if not os.environ.get("PYTHONNOUSERSITE"):
        user_site = site.getusersitepackages()
        if isinstance(user_site, str) and os.path.isdir(user_site) and user_site not in sys.path:
            sys.path.append(user_site)
    add_packages(sys.prefix)
    add_packages(sys.base_prefix)
