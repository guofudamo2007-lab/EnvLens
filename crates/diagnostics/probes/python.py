from importlib import metadata
import platform

versions = {}
for package in ("pip", "torch", "torchvision", "torchaudio", "numpy", "transformers", "diffusers", "accelerate"):
    try:
        version = metadata.version(package)
        if version and len(version) <= 80 and all(c.isalnum() or c in ".+-_!" for c in version):
            versions[package] = version
    except metadata.PackageNotFoundError:
        pass
kind = "venv" if is_venv else "conda" if os.path.isdir(os.path.join(sys.prefix, "conda-meta")) else "system"
print(json.dumps(dict(executable=sys.executable, version=platform.python_version(), prefix=sys.prefix,
                      base_prefix=sys.base_prefix, kind=kind, base_exists=os.path.isdir(sys.base_prefix), packages=versions)))
