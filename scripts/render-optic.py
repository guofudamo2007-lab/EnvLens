"""Bake the dashboard's optical assembly to a small transparent WebP. Requires Pillow and NumPy."""

from pathlib import Path
import numpy as np
from PIL import Image

WIDTH, HEIGHT = 1000, 660
view = np.array([0.34, 0.57, 0.75])
view /= np.linalg.norm(view)
right = np.cross([0, 1, 0], view)
right /= np.linalg.norm(right)
up = np.cross(view, right)
y, x = np.mgrid[0:HEIGHT, 0:WIDTH]
u = (x - WIDTH / 2) / 284
v = (HEIGHT / 2 - y) / 284
angle = np.radians(15)
u, v = u * np.cos(angle) - v * np.sin(angle), u * np.sin(angle) + v * np.cos(angle)
origin = u[..., None] * right + v[..., None] * up + 5 * view
depth = np.full(u.shape, np.inf)
pixels = np.zeros((HEIGHT, WIDTH, 4), dtype=np.uint8)
TAU = 2 * np.pi


def unit(a):
    return a / np.maximum(np.linalg.norm(a, axis=-1, keepdims=True), 1e-8)


def lobes(a, b, spread):
    return np.exp((np.sum(a * unit(np.array(b)), axis=-1) - 1) * spread)


def paint(t, mask, normal, material, detail=None):
    visible = mask & (t > 0) & (t < depth)
    if not visible.any():
        return
    p = (origin - t[..., None] * view)[visible]
    n = unit(normal[visible])
    refl = 2 * np.sum(n * view, axis=1, keepdims=True) * n - view
    rad = np.hypot(p[:, 0], p[:, 2])
    phase = np.arctan2(p[:, 2], p[:, 0])
    soft = lobes(refl, (-0.72, 0.72, -0.65), 5)
    silver = lobes(refl, (0.65, 0.27, 0.72), 22)
    edge = lobes(refl, (-0.85, 0.24, 0.45), 16)
    rim = (1 - np.clip(n @ view, 0, 1)) ** 3
    stripe = np.exp(-(((refl[:, 0] + 0.29 * refl[:, 1] + 0.28) / 0.105) ** 2))
    glint = np.exp(-(((refl[:, 0] - 0.48 * refl[:, 1] - 0.41) / 0.045) ** 2))
    if material in ("glass", "wafer"):
        iris = np.mod(phase + 1.5 * np.log(np.maximum(rad, 0.012)), TAU / 9) / (TAU / 9)
        aperture = np.clip((rad - 0.22) * 6, 0, 1)
        rings = np.exp(-(((rad - 0.63) / 0.014) ** 2)) + 0.6 * np.exp(
            -(((rad - 0.37) / 0.009) ** 2)
        )
        sweep = np.exp(-(((p[:, 0] + 0.5 * p[:, 2] - 0.10) / 0.48) ** 2))
        base = np.array([0.027, 0.081, 0.101]) + np.outer(
            aperture * (0.3 + 0.7 * iris) * sweep, [0.08, 0.25, 0.24]
        )
        base += np.outer(np.maximum(0.0, 1 - rad) * sweep, [0.01, 0.14, 0.19])
        base += np.outer(rings * (0.2 + 0.6 * sweep), [0.13, 0.23, 0.26])
        base += np.outer(soft * 0.23 + stripe * 0.50 + edge * 0.33, [0.33, 0.78, 0.72])
        base += np.outer(glint * 0.54 + silver * 0.37, [0.54, 0.57, 0.94])
        coat = np.exp(-(((p[:, 0] + 0.74 * p[:, 2] - 0.28) / 0.16) ** 2)) * (
            0.16 + 0.36 * rad
        )
        base += np.outer(coat, [0.28, 0.30, 0.50])
        dark_core = 1 - 0.54 * np.exp(-((rad / 0.40) ** 2))
        base *= dark_core[:, None]
        base += np.outer(
            rim + 0.23 * np.clip((rad - 0.68) * 4, 0, 1), [0.15, 0.34, 0.37]
        )
        if material == "wafer":
            base = 0.66 * base + np.outer(
                0.28 + (0.5 + 0.5 * np.sin(phase)) * rad, [0.13, 0.24, 0.46]
            )
    else:
        albedo = {
            "titanium": (0.26, 0.29, 0.30),
            "black": (0.083, 0.099, 0.106),
            "edge": (0.47, 0.52, 0.53),
            "seal": (0.044, 0.069, 0.072),
            "sensor": (0.14, 0.22, 0.23),
        }[material]
        diffuse = 0.43 + 0.62 * np.maximum(0, n @ unit(np.array([-0.58, 1.0, 0.56])))
        base = np.outer(diffuse, albedo) + np.outer(
            soft * 0.65 + silver * 0.5 + edge * 0.5 + stripe * 0.27 + glint * 0.22,
            [0.71, 0.77, 0.76],
        ) * (0.36 if material in ("black", "seal") else 1)
        base += np.outer(rim, [0.05, 0.063, 0.07])
        if material in ("titanium", "edge") and np.mean(n[:, 1]) > 0.75:
            brushing = (
                0.62 + 0.60 * np.sin(phase + 0.37) ** 4 + 0.11 * np.sin(rad * 1400)
            )
            base *= brushing[:, None]
        if detail == "knurl":
            grain = (np.cos(phase * 128 + 0.85 * p[:, 1]) * 0.5 + 0.5) ** 2
            base *= 0.47 + 0.5 * grain[:, None]
        if detail == "scale":
            tick = np.abs(np.sin(phase * 64)) < 0.105
            tick &= rad > 0.858
            major = (np.abs(np.sin(phase * 16)) < 0.10) & (rad > 0.809)
            base[tick | major] = [0.69, 0.74, 0.72]
            mark = (np.abs(phase - 0.91) < 0.045) & (rad > 0.795)
            base[mark] = [0.85, 0.98, 0.40]
        if detail == "blades":
            blade = np.mod(phase + 1.3 * np.log(np.maximum(rad, 0.01)), TAU / 9) / (
                TAU / 9
            )
            base *= 0.40 + 0.6 * blade[:, None]
    color = np.uint8(np.clip(base, 0, 1) * 255 + 0.5)
    pixels[visible, :3] = color
    pixels[visible, 3] = 255
    depth[visible] = t[visible]


def surface(radius, inner, y, material, detail=None, curve=0):
    # A shallow paraboloid approximates the convex multi-coated glass.
    t = (origin[..., 1] - y) / view[1]
    for _ in range(18 if curve else 1):
        p = origin - t[..., None] * view
        r = np.hypot(p[..., 0], p[..., 2])
        if curve:
            t = (
                origin[..., 1]
                - y
                - curve * (1 - (np.minimum(r, radius * 1.2) / radius) ** 2)
            ) / view[1]
    p = origin - t[..., None] * view
    r = np.hypot(p[..., 0], p[..., 2])
    n = np.stack(
        [
            2 * curve * p[..., 0] / radius**2,
            np.ones_like(r),
            2 * curve * p[..., 2] / radius**2,
        ],
        axis=-1,
    )
    valid = (
        (r <= radius)
        & (r >= inner)
        & (np.abs(p[..., 1] - y - curve * (1 - (r / radius) ** 2)) < 0.004)
    )
    paint(t, valid, n, material, detail)


def wall(r0, r1, lo, hi, mat, detail=None):
    slope = (r1 - r0) / (hi - lo)
    ox, oy, oz = origin.transpose(2, 0, 1)
    dx, dy, dz = view
    ry = r0 + slope * (oy - lo)
    a = dx * dx + dz * dz - (slope * dy) ** 2
    b = -2 * (ox * dx + oz * dz - ry * slope * dy)
    c = ox * ox + oz * oz - ry * ry
    disc = b * b - 4 * a * c
    for sign in (-1, 1):
        t = (-b + sign * np.sqrt(np.maximum(disc, 0))) / (2 * a)
        p = origin - t[..., None] * view
        r = np.maximum(np.hypot(p[..., 0], p[..., 2]), 1e-6)
        n = np.stack([p[..., 0] / r, np.full_like(r, -slope), p[..., 2] / r], axis=-1)
        paint(t, (disc >= 0) & (p[..., 1] >= lo) & (p[..., 1] <= hi), n, mat, detail)


# Sensor carrier, a thin coated element and a knurled focusing ring.
wall(0.88, 0.99, -0.60, -0.55, "titanium")
wall(0.99, 0.99, -0.55, -0.48, "black", "knurl")
wall(0.99, 0.96, -0.48, -0.45, "edge")
surface(0.96, 0.00, -0.45, "black", "blades")
surface(0.65, 0, -0.441, "glass", curve=0.045)
wall(0.87, 0.90, -0.39, -0.365, "edge")
wall(0.90, 0.90, -0.365, -0.34, "seal")
surface(0.90, 0.64, -0.34, "titanium")
wall(0.76, 0.81, -0.16, -0.14, "edge")
wall(0.81, 0.81, -0.14, -0.095, "sensor")
wall(0.81, 0.78, -0.095, -0.07, "edge")
surface(0.78, 0, -0.07, "wafer", curve=0.085)
wall(0.94, 0.985, 0.135, 0.16, "edge")
wall(0.985, 0.985, 0.16, 0.259, "black", "knurl")
wall(0.985, 0.948, 0.259, 0.29, "titanium")
surface(0.948, 0.735, 0.29, "black", "scale")
wall(0.735, 0.71, 0.255, 0.29, "edge")
surface(0.714, 0, 0.259, "glass", curve=0.135)
# Floating protective optical element with a narrow bright ground edge.
wall(0.765, 0.788, 0.53, 0.542, "edge")
wall(0.788, 0.788, 0.542, 0.566, "sensor")
wall(0.788, 0.772, 0.566, 0.578, "edge")
surface(0.772, 0, 0.578, "glass", curve=0.148)
img = Image.fromarray(pixels)
bounds = img.getbbox()
img = img.crop((bounds[0] - 12, bounds[1] - 12, bounds[2] + 12, bounds[3] + 12))
img.thumbnail((750, 525), Image.Resampling.LANCZOS)
dest = Path(__file__).resolve().parents[1] / "src/assets/optic.webp"
dest.parent.mkdir(parents=True, exist_ok=True)
img.save(dest, quality=91, method=6)
print(dest.name, img.size, dest.stat().st_size)
