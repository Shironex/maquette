"""Renders a model from the standard views, in the page light.

Maquette runs this headless, with render_thumb.py and preview_rig.py beside
it:

    blender -b --factory-startup --python-exit-code 1 \
        --python render_views.py -- --spec <spec.json>

The spec is JSON: one or two models and the views to take of them.

    {"size": 1024, "samples": 32, "report": "report.json",
     "models": [{"id": "v3", "file": "models/book_stack_v3.glb"}],
     "views": [{"id": "three-quarter", "azimuth": 38, "elevation": 22,
                "fill": 0.74, "outs": {"v3": "three-quarter.png"}},
               {"id": "close-up-1", "azimuth": 38, "elevation": 22,
                "fill": 0.9, "half": 0, "outs": {"v3": "close-up-1.png"}}],
     "sheet": {"out": "sheet.png", "tile": 512, "refs": ["refs/book.png"]}}

A view's camera stands `azimuth` degrees round from the front (towards the
model's own right, +X) and `elevation` degrees up, 90 looking straight down
with the front at the bottom of the picture, and backs off until its target
fills `fill` of the frame. The target is the whole model, or
- "half": 0 or 1, one half of it along its longest side (left then right,
  top then bottom, front then back), with a little overlap;
- "box": two corners in shares of the bounding box, x from left to right, y
  from bottom to top, z from back to front;
- "node": the object of that name with everything under it.

Two models share every camera: the cameras are fitted to the bounds of both
together, each model is rendered alone, and "outs" names a file per model.
So a rebuild and the model it replaces are seen from the very same spot.

The light is preview_rig.py's, the backdrop and floor the thumbnail's. Axes:
glTF's front is +Z, which Blender imports as -Y, so the front view looks
along +Y. The report gives every camera in glTF terms (Y up).

The sheet is every picture on one PNG: a row per model, its views in the
spec's order, with the refs (fitted into a tile each) leading the first row.

Progress: one line per picture, "[render_views] progress 2/5 front". When
it cannot go on it says why in one line, "[render_views] Error: ...".
"""

import json
import math
import os
import sys
import time

import bpy
import numpy as np
from mathutils import Matrix, Vector

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import preview_rig as rig  # noqa: E402
import render_thumb as thumb  # noqa: E402

OVERLAP = 0.1
NODE_MARGIN = 0.15
SHEET_PAPER = "#e7e5df"
MESHES = {"MESH", "CURVE", "SURFACE", "META", "FONT"}


def log(message):
    print(f"[render_views] {message}", flush=True)


def spec():
    argv = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
    if len(argv) != 2 or argv[0] != "--spec":
        raise SystemExit("render_views.py needs --spec <file>")
    with open(argv[1], encoding="utf-8") as fh:
        return json.load(fh)


def gltf_xyz(v):
    """Blender Z up to glTF Y up."""
    return [round(v.x, 5), round(v.z, 5), round(-v.y, 5)]


def add_model(path):
    """Imports one file and returns its objects."""
    before = set(bpy.data.objects)
    ext = os.path.splitext(path)[1].lower()
    if ext in (".glb", ".gltf"):
        bpy.ops.import_scene.gltf(filepath=path)
    elif ext == ".obj":
        bpy.ops.wm.obj_import(filepath=path)
    else:
        raise SystemExit(f"Cannot render {path}: only .glb, .gltf and .obj files.")
    added = [o for o in bpy.data.objects if o not in before]
    for obj in added:
        if obj.type in {"LIGHT", "CAMERA"}:
            obj.hide_render = True
    return added


def drawn(objects):
    return [o for o in objects if o.type in MESHES and not o.hide_render]


def triangles(objects):
    return sum(
        sum(len(p.vertices) - 2 for p in o.data.polygons) for o in objects if o.type == "MESH"
    )


def show_only(models, wanted):
    for model in models:
        for obj in model["meshes"]:
            obj.hide_render = model is not wanted


def axes(azimuth, elevation):
    """The camera's right, up and back for a turn and a tilt in degrees."""
    a, e = math.radians(azimuth), math.radians(elevation)
    back = Vector((math.sin(a) * math.cos(e), -math.cos(a) * math.cos(e), math.sin(e))).normalized()
    right = Vector((math.cos(a), math.sin(a), 0.0))
    return right, back.cross(right).normalized(), back


def half_box(lo, hi, which):
    """One half of the box along its longest side, and what to call it."""
    size = hi - lo
    axis = max(range(3), key=lambda i: size[i])
    names = (("left", "right"), ("front", "back"), ("bottom", "top"))[axis]
    # The first close up takes the top of a tall model, as a person looks.
    first, second = (1, 0) if axis == 2 else (0, 1)
    side = first if which == 0 else second
    reach = size[axis] * (0.5 + OVERLAP / 2)
    a, b = lo.copy(), hi.copy()
    if side == 0:
        b[axis] = lo[axis] + reach
    else:
        a[axis] = hi[axis] - reach
    return a, b, f"the {names[side]} half"


def share_box(lo, hi, box):
    """A box given in shares of the bounds, in glTF's axes."""
    (x0, y0, z0), (x1, y1, z1) = box
    size = hi - lo
    # glTF z runs from the back (Blender +Y) to the front (-Y).
    a = Vector((lo.x + x0 * size.x, hi.y - z1 * size.y, lo.z + y0 * size.z))
    b = Vector((lo.x + x1 * size.x, hi.y - z0 * size.y, lo.z + y1 * size.z))
    return Vector(map(min, a, b)), Vector(map(max, a, b)), "the box asked for"


def node_box(models, name):
    """The bounds of the object called `name` and everything under it."""
    for model in models:
        root = next((o for o in model["objects"] if o.name == name), None)
        if root is None:
            continue
        family = [root] + list(root.children_recursive)
        meshes = [o for o in family if o in model["meshes"]]
        if not meshes:
            continue
        lo, hi = thumb.bounds(meshes)
        pad = (hi - lo).length * NODE_MARGIN / 2
        grow = Vector((pad, pad, pad))
        return lo - grow, hi + grow, f"the part {name}"
    names = sorted({o.name for m in models for o in m["meshes"]})
    raise SystemExit(f"There is no object called {name!r} to close in on. Objects: {', '.join(names)}.")


def target_of(view, models, lo, hi):
    if view.get("node"):
        return node_box(models, view["node"])
    if view.get("box"):
        return share_box(lo, hi, view["box"])
    if view.get("half") is not None:
        return half_box(lo, hi, int(view["half"]))
    return lo, hi, "the whole model"


def place_camera(scene, view, lo, hi, extent):
    """A camera for `view` framing the box from lo to hi."""
    data = bpy.data.cameras.new(f"maquette_{view['id']}")
    data.lens = thumb.FOCAL_MM
    data.sensor_fit = "HORIZONTAL"
    cam = bpy.data.objects.new(data.name, data)
    scene.collection.objects.link(cam)
    right, up, back = axes(view["azimuth"], view["elevation"])
    centre = (lo + hi) / 2
    tan = (data.sensor_width / 2) / data.lens
    fill = view.get("fill", thumb.FILL)
    distance = 0.0
    for x in (lo.x, hi.x):
        for y in (lo.y, hi.y):
            for z in (lo.z, hi.z):
                q = Vector((x, y, z)) - centre
                reach = max(abs(q.dot(right)), abs(q.dot(up))) / (tan * fill)
                distance = max(distance, reach + q.dot(back))
    distance = max(distance, 0.01)
    place = centre + back * distance
    cam.matrix_world = Matrix(
        (
            (right.x, up.x, back.x, place.x),
            (right.y, up.y, back.y, place.y),
            (right.z, up.z, back.z, place.z),
            (0.0, 0.0, 0.0, 1.0),
        )
    )
    data.clip_start = distance / 100
    data.clip_end = max(distance, extent) * 200
    fov = 2 * math.degrees(math.atan(tan))
    return cam, {"position": gltf_xyz(place), "target": gltf_xyz(centre), "fov": round(fov, 3)}


def fit(picture, tile, paper):
    """`picture` (h, w, 3 or 4, sRGB from 0 to 1) inside a square tile."""
    h, w = picture.shape[:2]
    scale = tile / max(h, w)
    th, tw = max(1, round(h * scale)), max(1, round(w * scale))
    # A few samples per target pixel, so a big ref does not alias.
    n = max(1, min(4, math.ceil(1 / scale)))
    ys = ((np.arange(th * n) + 0.5) / (th * n) * h).astype(int).clip(0, h - 1)
    xs = ((np.arange(tw * n) + 0.5) / (tw * n) * w).astype(int).clip(0, w - 1)
    small = picture[ys][:, xs].reshape(th, n, tw, n, picture.shape[2]).mean(axis=(1, 3))
    if small.shape[2] == 4:
        alpha = small[..., 3:4]
        small = small[..., :3] * alpha + paper * (1 - alpha)
    out = np.empty((tile, tile, 3))
    out[...] = paper
    top, left = (tile - th) // 2, (tile - tw) // 2
    out[top : top + th, left : left + tw] = small[..., :3]
    return out


def read_picture(path):
    """An image file as sRGB values, rows from the bottom up."""
    image = bpy.data.images.load(path)
    w, h = image.size
    channels = image.channels
    if w == 0 or h == 0 or channels == 0:
        bpy.data.images.remove(image)
        raise ValueError("it is not a picture Blender reads")
    data = np.empty(w * h * channels, dtype=np.float32)
    image.pixels.foreach_get(data)
    bpy.data.images.remove(image)
    px = data.reshape(h, w, channels)
    if channels == 1:
        px = np.repeat(px, 3, axis=2)
    elif channels == 2:
        px = np.concatenate([np.repeat(px[..., :1], 3, axis=2), px[..., 1:]], axis=2)
    return px


def write_sheet(sheet, rows):
    """Every picture on one PNG: `rows` of sRGB tiles, the refs first."""
    tile = int(sheet.get("tile", 512))
    paper = rig.decode(sheet.get("paper") or SHEET_PAPER)
    paper = rig.encode(paper)
    refs = []
    for path in sheet.get("refs") or []:
        # A ref that cannot be read leaves the sheet; it never stops a render.
        try:
            refs.append(fit(read_picture(path), tile, paper))
        except (RuntimeError, ValueError, OSError) as err:
            log(f"left the ref {path} off the sheet: {err}")
    blank = np.empty((tile, tile, 3))
    blank[...] = paper
    lines = []
    for n, tiles in enumerate(rows):
        lead = refs if n == 0 else [blank] * len(refs)
        lines.append(lead + [fit(t, tile, paper) for t in tiles])
    gap = max(8, tile // 32)
    columns = max(len(line) for line in lines)
    width = columns * tile + (columns + 1) * gap
    height = len(lines) * tile + (len(lines) + 1) * gap
    page = np.empty((height, width, 3))
    page[...] = paper
    for row, line in enumerate(lines):
        # Rows from the bottom up, so the first row is the top one.
        bottom = height - (row + 1) * (tile + gap)
        for col, picture in enumerate(line):
            left = gap + col * (tile + gap)
            page[bottom : bottom + tile, left : left + tile] = picture
    out = os.path.abspath(sheet["out"])
    os.makedirs(os.path.dirname(out), exist_ok=True)
    part = out + ".part.png"
    rig.write_png(page, part)
    os.replace(part, out)


def main():
    s = spec()
    started = time.time()
    size = int(s.get("size", 1024))
    samples = int(s.get("samples", 32))
    bpy.ops.wm.read_factory_settings(use_empty=True)
    scene = bpy.context.scene
    models = []
    for entry in s["models"]:
        objects = add_model(os.path.abspath(entry["file"]))
        meshes = drawn(objects)
        if not meshes:
            raise SystemExit(f"{entry['file']} has nothing to render.")
        models.append({"id": entry["id"], "objects": objects, "meshes": meshes})
    bpy.context.view_layer.update()
    lo = Vector((math.inf,) * 3)
    hi = Vector((-math.inf,) * 3)
    described = []
    for model in models:
        a, b = thumb.bounds(model["meshes"])
        lo, hi = Vector(map(min, lo, a)), Vector(map(max, hi, b))
        # glTF's min and max: its z is Blender's -y.
        low, high = gltf_xyz(Vector((a.x, b.y, a.z))), gltf_xyz(Vector((b.x, a.y, b.z)))
        described.append(
            {"id": model["id"], "min": low, "max": high, "triangles": triangles(model["meshes"])}
        )
    extent = max((hi - lo).length, 1e-3)
    thumb.page_stage(scene, rig, lo, hi, extent)
    # The same scene from every camera: keep what Cycles built of it.
    scene.render.use_persistent_data = True
    total = len(s["views"]) * len(models)
    done = 0
    taken = []
    rows = [[] for _ in models]
    for view in s["views"]:
        a, b, frames = target_of(view, models, lo, hi)
        cam, placed = place_camera(scene, view, a, b, extent)
        scene.camera = cam
        for n, model in enumerate(models):
            out = view["outs"].get(model["id"])
            if not out:
                continue
            log(f"progress {done}/{total} {view['id']}")
            show_only(models, model)
            light = rig.render(scene, out, size, samples)
            rows[n].append(rig.encode(rig.tone(light)))
            done += 1
        taken.append(
            {
                "id": view["id"],
                "azimuth": view["azimuth"],
                "elevation": view["elevation"],
                "frames": frames,
                **placed,
            }
        )
    log(f"progress {total}/{total} done")
    if s.get("sheet"):
        write_sheet(s["sheet"], [r for r in rows if r])
    report = {
        "size": size,
        "samples": samples,
        "min": gltf_xyz(Vector((lo.x, hi.y, lo.z))),
        "max": gltf_xyz(Vector((hi.x, lo.y, hi.z))),
        "models": described,
        "views": taken,
        "light": {
            "sun": rig.SUN_STRENGTH,
            "fill": rig.WORLD_GREY,
            "tone": rig.TONE,
            "exposure": rig.EXPOSURE,
        },
        "seconds": round(time.time() - started, 2),
    }
    if s.get("report"):
        with open(s["report"], "w", encoding="utf-8") as fh:
            json.dump(report, fh)
    print("render_views", json.dumps({"pictures": done, "seconds": report["seconds"]}))


if __name__ == "__main__":
    try:
        main()
    except SystemExit as stop:
        # Said in one line, which Maquette shows as why the views did not render.
        if stop.code not in (None, 0):
            log(f"Error: {stop.code}")
        raise
    except Exception as err:
        log(f"Error: {type(err).__name__}: {err}")
        raise
