import re
import warnings
warnings.simplefilter("ignore")

def classify(exc):
    msg = str(exc).lower()
    if "driver" in msg and ("too old" in msg or "insufficient" in msg):
        return "insufficient_driver"
    if "no nvidia driver" in msg or "driver not found" in msg:
        return "driver_not_found"
    if "no cuda" in msg or "no gpu" in msg:
        return "no_visible_device"
    return type(exc).__name__[:60]

def clean(value):
    return re.sub(r"[^A-Za-z0-9 ._()+:/-]", "", str(value))[:120]

out = dict(installed=True, mask=os.environ.get("CUDA_VISIBLE_DEVICES") in ("", "-1"))
try:
    import torch
except ModuleNotFoundError as exc:
    out.update(installed=exc.name != "torch", error=classify(exc))
except Exception as exc:
    out["error"] = classify(exc)
else:
    out.update(version=clean(torch.__version__), cuda=clean(torch.version.cuda) if torch.version.cuda else None,
               hip=bool(getattr(torch.version, "hip", None)), xpu=bool(getattr(torch.version, "xpu", None)))
    try:
        out["available"] = bool(torch.cuda.is_available())
        out["count"] = int(torch.cuda.device_count())
        out["devices"] = [clean(torch.cuda.get_device_name(i)) for i in range(min(out["count"],16))]
        if out["cuda"] and not out["available"]:
            try:
                torch.cuda.init()
            except Exception as exc:
                out["init_error"] = classify(exc)
        elif out["available"] and sys.argv[1] == "verify":
            try:
                out["smoke"] = bool(torch.ones(1, device="cuda:0").add_(1).item() == 2)
            except Exception as exc:
                out.update(smoke=False, smoke_error=classify(exc))
    except Exception as exc:
        out["device_error"] = classify(exc)
print(json.dumps(out))
