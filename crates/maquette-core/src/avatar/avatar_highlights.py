"""Soften the highlights baked into a generated character's base colour texture.

Generated models arrive with their lighting painted into the colour: on black leather, the
shine of the studio light is a bright, nearly white patch in the texture itself. A PBR viewer
lights the model again and those patches pass as gloss; MToon shades flat, so they read as
white blotches. This script writes a copy of each base colour image with only those patches
toned down toward the colour around them. The original image is never changed.

Maquette runs this headless, never with a window:

    blender -b --factory-startup --python-exit-code 1 --python avatar_highlights.py -- \
        --job job.json

job.json lists the images (PNG or JPEG files written from the model) and where each copy goes;
relative paths resolve against the job file's folder:

    {"images": [{"input": "image_1.jpg", "output": "soft_1.jpg"}], "strength": 0.75,
     "report": "report.json"}

A pixel is a baked highlight when all of these hold, each as a soft ramp, not a hard cut:

1. It stands out above its surroundings: brighter than the grey level that rebuilding the
   image from its dark core gives back (an opening by reconstruction with a square about
   1/32 of the image wide). Bright areas wider than that square, or joined to one, are kept
   whole: skin, a light top, a large printed shape.
2. What is around it is dark: the mean colour of the not standing out pixels nearby is dark.
3. It is low in saturation and lies near the line from that colour toward white, the way a
   light's reflection does. Skin, coloured prints and gems leave that line.
4. It is not warmer than its surroundings by much, so the edge of a skin island next to a
   dark one stays skin.

A highlight moves `strength` of the way toward the colour around it, weighted by how sure
each test is, so detail stays visible and nothing is painted over. An image where less than
MIN_COVERAGE of the pixels are highlights is left as it is (no copy is written). The report
says, per image, how much of the texture the highlights cover and whether a copy was written.
"""

import argparse
import json
import os
import sys
import traceback

import numpy as np

TAG = "[avatar]"
WORK_SIZE = 2048       # images wider than this are measured on a smaller copy
WINDOW = 1.0 / 32.0    # the square a highlight must fit in, as a share of the width
LUMA = np.array([0.2126, 0.7152, 0.0722], np.float32)
# The ramps (start, end) of each test, in 0..1 colour values.
STAND_OUT = (0.06, 0.20)   # brighter than its surroundings by this much
DARK = (0.30, 0.20)        # the surroundings at most this bright
GREY = (0.26, 0.12)        # saturation at most this
ON_LINE = (0.09, 0.05)     # this far at most from the line toward white
WARMER = (0.12, 0.07)      # red minus blue above the surroundings' at most this
KEEP_BELOW = 0.05          # a pixel standing out less than this is part of the surroundings
STRONG = 0.5               # a pixel this sure is counted as a highlight
MIN_COVERAGE = 0.002       # an image with fewer highlights than this is left as it is
JPEG_QUALITY = 92


def log(text):
    print(f"{TAG} {text}", flush=True)


def fail(text):
    print(f"{TAG} Error: {text}", flush=True)
    sys.exit(1)


def ramp(edge, x):
    """0 at edge[0], 1 at edge[1], smooth between (either way round)."""
    t = np.clip((x - edge[0]) / (edge[1] - edge[0]), 0.0, 1.0)
    return t * t * (3.0 - 2.0 * t)


def slide(a, n, fn, axis):
    """fn (np.minimum or np.maximum) over windows of n centred on each pixel along axis,
    the edges repeated: log2(n) passes instead of n."""
    r = n // 2
    pad = [(0, 0), (0, 0)]
    pad[axis] = (r, n - 1 - r)
    a = np.pad(a, pad, mode="edge")
    size = a.shape[axis]

    def part(x, start, length):
        return x[start:start + length] if axis == 0 else x[:, start:start + length]

    out, w = a, 1
    while 2 * w <= n:
        length = out.shape[axis] - w
        out = fn(part(out, 0, length), part(out, w, length))
        w *= 2
    if w < n:
        length = size - n + 1
        out = fn(part(out, 0, length), part(out, n - w, length))
    return out


def square(a, n, fn):
    return slide(slide(a, n, fn, 0), n, fn, 1)


def box_mean(a, r):
    """The mean over a (2r+1) square around each pixel, the edges repeated."""
    n = 2 * r + 1
    c = np.pad(a, ((r + 1, r), (r + 1, r)), mode="edge").astype(np.float64)
    c = c.cumsum(0).cumsum(1)
    s = c[n:, n:] - c[:-n, n:] - c[n:, :-n] + c[:-n, :-n]
    return (s / (n * n)).astype(np.float32)


def rebuild(marker, mask, steps):
    """Grows marker under mask a pixel at a time (reconstruction by dilation)."""
    m = np.minimum(marker, mask)
    for _ in range(steps):
        grown = np.minimum(square(m, 3, np.maximum), mask)
        if np.array_equal(grown, m):
            break
        m = grown
    return m


def measure(rgb):
    """How sure each pixel of rgb (h, w, 3 in 0..1) is a baked highlight, and the colour
    around it."""
    h, w, _ = rgb.shape
    r = max(2, round(w * WINDOW / 2))  # the square is 2r + 1 wide
    y = rgb @ LUMA
    core = rebuild(square(y, 2 * r + 1, np.minimum), y, 4 * r)
    stand = y - core
    keep = (stand < KEEP_BELOW).astype(np.float32)
    weight = np.maximum(box_mean(keep, 2 * r), 1e-3)
    around = np.stack([box_mean(rgb[..., c] * keep, 2 * r) for c in range(3)], -1)
    around /= weight[..., None]
    around_y = around @ LUMA
    top = rgb.max(-1)
    sat = np.where(top > 1e-4, (top - rgb.min(-1)) / np.maximum(top, 1e-4), 0.0)
    t = np.clip((y - around_y) / np.maximum(1.0 - around_y, 1e-3), 0.0, 1.0)
    line = np.abs(rgb - (around + t[..., None] * (1.0 - around))).max(-1)
    warmer = (rgb[..., 0] - rgb[..., 2]) - (around[..., 0] - around[..., 2])
    sure = (ramp(STAND_OUT, stand) * ramp(DARK, around_y) * ramp(GREY, sat)
            * ramp(ON_LINE, line) * ramp(WARMER, warmer))
    return sure.astype(np.float32), around.astype(np.float32), around_y < DARK[0]


def shrink(a, f):
    h, w = a.shape[0] // f * f, a.shape[1] // f * f
    a = a[:h, :w]
    return a.reshape(h // f, f, w // f, f, *a.shape[2:]).mean(axis=(1, 3))


def grow(a, f, shape):
    a = np.repeat(np.repeat(a, f, axis=0), f, axis=1)
    pad = [(0, shape[0] - a.shape[0]), (0, shape[1] - a.shape[1])] + [(0, 0)] * (a.ndim - 2)
    return np.pad(a, pad, mode="edge")


def soften(rgb, strength):
    """The softened copy of rgb, how sure each pixel was a highlight, and the dark mask."""
    f = max(1, -(-rgb.shape[1] // WORK_SIZE))
    sure, around, dark = measure(rgb if f == 1 else shrink(rgb, f))
    # The toning reaches a pixel past each highlight's edge, so it leaves no seam.
    blend = 0.5 * (sure + square(sure, 3, np.maximum))
    if f > 1:
        shape = rgb.shape[:2]
        sure = grow(sure, f, shape)
        blend = box_mean(grow(blend, f, shape), f)
        around = grow(around, f, shape)
        dark = grow(dark, f, shape)
    out = rgb - (strength * blend)[..., None] * (rgb - around)
    return np.clip(out, 0.0, 1.0), sure, dark


def stats(sure, dark):
    strong = sure >= STRONG
    return {
        "coverage": float(strong.mean()),
        "darkCoverage": float(strong[dark].mean()) if dark.any() else 0.0,
        "touched": float((sure > 0.05).mean()),
    }


def load_pixels(path):
    import bpy
    image = bpy.data.images.load(path)
    if image.is_float:
        return image, None
    w, h = image.size
    px = np.empty(w * h * image.channels, np.float32)
    image.pixels.foreach_get(px)
    return image, px.reshape(h, w, image.channels)


def save_pixels(image, px, path):
    import bpy
    h, w, channels = px.shape
    out = bpy.data.images.new("softened", w, h, alpha=channels == 4)
    full = np.ones((h, w, 4), np.float32)
    full[..., :channels] = px
    out.pixels.foreach_set(full.ravel())
    jpeg = path.lower().endswith((".jpg", ".jpeg"))
    out.file_format = "JPEG" if jpeg else "PNG"
    out.filepath_raw = path
    if jpeg:
        out.save(filepath=path, quality=JPEG_QUALITY)
    else:
        out.save(filepath=path)


def run_image(entry, strength):
    image, px = load_pixels(entry["input"])
    found = {"input": entry["input"], "output": None, "width": image.size[0],
             "height": image.size[1], "coverage": 0.0, "darkCoverage": 0.0, "touched": 0.0}
    if px is None:
        found["skipped"] = "a high bit depth image, left as it is"
        return found
    rgb = px[..., :3]
    softened, sure, dark = soften(rgb, strength)
    found.update(stats(sure, dark))
    if found["coverage"] < MIN_COVERAGE:
        found["skipped"] = "too few baked highlights to change"
        return found
    out = px.copy()
    out[..., :3] = softened
    save_pixels(image, out, entry["output"])
    found["output"] = entry["output"]
    log(f"softened {found['coverage'] * 100:.1f}% of {os.path.basename(entry['input'])}")
    return found


def parse_args():
    argv = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else sys.argv[1:]
    parser = argparse.ArgumentParser()
    parser.add_argument("--job", required=True)
    return parser.parse_args(argv)


def load_job(path):
    with open(path, encoding="utf-8") as fh:
        job = json.load(fh)
    base = os.path.dirname(os.path.abspath(path))

    def resolve(p):
        return p if os.path.isabs(p) else os.path.join(base, p)

    job["report"] = resolve(job["report"])
    for entry in job["images"]:
        entry["input"] = resolve(entry["input"])
        entry["output"] = resolve(entry["output"])
    return job


def main():
    job = load_job(parse_args().job)
    strength = float(job.get("strength", 0.75))
    if not 0.0 <= strength <= 1.0:
        fail("strength must be between 0 and 1")
    report = {"strength": strength, "images": [run_image(e, strength) for e in job["images"]]}
    with open(job["report"], "w", encoding="utf-8") as fh:
        json.dump(report, fh, indent=2)
    log(f"wrote {job['report']}")


if __name__ == "__main__":
    try:
        main()
    except SystemExit:
        raise
    except Exception:
        traceback.print_exc()
        fail("softening highlights stopped: " + traceback.format_exc().strip().splitlines()[-1])
