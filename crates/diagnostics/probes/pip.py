import runpy
# Fixed module and fixed arguments chosen by the Rust adapter.
sys.argv = ["pip", "--disable-pip-version-check", "--no-input", sys.argv[1]]
runpy.run_module("pip", run_name="__main__", alter_sys=True)
