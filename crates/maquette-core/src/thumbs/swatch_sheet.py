"""Renders a palette as a swatch sheet in the page light.

    blender -b --factory-startup --python-exit-code 1 --python swatch_sheet.py -- \
        --out sheet.png [--swatches "lavender=#c9b8e8,cream=#f3e9d2"] \
        [--columns 5] [--width 1600] [--report sheet.json] [--plain] [--glb sheet.glb]

preview_rig.py must sit beside it. Each colour is a tile lying flat in the
rig's sun, seen from straight above, so the whole tile shows one colour: what
a flat sunlit surface of that base colour looks like on the page. Along its
lower edge runs a strip of the colour exactly as it was written (an emission
the rig's tone curve is undone for), with its name and hex below, so the
sheet shows at a glance how far the page light moves each colour.

--report writes what was measured, as JSON: per swatch its "hex", the sRGB
the middle of its tile shows ("shown"), the light there ("light", linear)
and the sRGB its strip shows ("written", the hex again), with the rig's
numbers. --plain leaves the strips and the names out.
--glb also writes the bare tiles as a GLB (Y up), to show the same colours
on a page and hold the two side by side.

The palette below is the reference one: pale colours first, since they are
the ones a light that is too strong washes out.
"""

import argparse
import json
import math
import os
import sys

import bpy
import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import preview_rig as rig  # noqa: E402

PALETTE = [
    ("lavender", "#c9b8e8"),
    ("cream", "#f3e9d2"),
    ("blush", "#f2c6c9"),
    ("mint", "#bfe3cf"),
    ("sky", "#b9d7f0"),
    ("butter", "#f6e08a"),
    ("terracotta", "#c8674b"),
    ("moss", "#6f8f4e"),
    ("denim", "#3f6aa1"),
    ("plum", "#5b3a6e"),
    ("paper", "#f7f5ef"),
    ("grey", "#777777"),
    ("charcoal", "#2d2b29"),
    ("white", "#ffffff"),
    ("ink", "#1b1a18"),
]
BACKDROP = "#1a1917"
INK = "#b9b4aa"
TILE = 0.06
GAP = 0.02
THICK = 0.006
STRIP = TILE / 5
LABEL = TILE / 2.6
ROUGHNESS = 0.5


def parse_args():
    argv = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
    parser = argparse.ArgumentParser(prog="swatch_sheet.py")
    parser.add_argument("--out", required=True)
    parser.add_argument("--swatches", default="", help="name=#hex pairs, comma separated")
    parser.add_argument("--columns", type=int, default=5)
    parser.add_argument("--width", type=int, default=1600)
    parser.add_argument("--samples", type=int, default=64)
    parser.add_argument("--report", default="")
    parser.add_argument("--plain", action="store_true")
    parser.add_argument("--glb", default="")
    return parser.parse_args(argv)


def palette(text):
    if not text.strip():
        return PALETTE
    found = []
    for pair in text.split(","):
        name, _, hex_color = pair.partition("=")
        hex_color = hex_color.strip()
        if len(hex_color) != 7 or not hex_color.startswith("#"):
            raise SystemExit(f"A swatch is name=#rrggbb, not {pair!r}.")
        found.append((name.strip() or hex_color, hex_color))
    return found


def box(name, size, location, material):
    bpy.ops.mesh.primitive_cube_add(size=1, location=location)
    obj = bpy.context.active_object
    obj.name = name
    obj.scale = size
    bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
    obj.data.materials.append(material)
    return obj


def lit(name, hex_color):
    """A plain material of this base colour, as a built script makes one."""
    mat = bpy.data.materials.new(name)
    tree = rig.nodes_of(mat)
    bsdf = tree.nodes.get("Principled BSDF")
    bsdf.inputs["Base Color"].default_value = (*rig.decode(hex_color), 1.0)
    bsdf.inputs["Roughness"].default_value = ROUGHNESS
    bsdf.inputs["Metallic"].default_value = 0.0
    return mat


def exact(name, hex_color):
    """A material the page light does not move: it shows as `hex_color`."""
    mat = bpy.data.materials.new(name)
    tree = rig.nodes_of(mat)
    tree.nodes.clear()
    out = tree.nodes.new("ShaderNodeOutputMaterial")
    glow = tree.nodes.new("ShaderNodeEmission")
    glow.inputs["Color"].default_value = (*rig.scene_color(hex_color), 1.0)
    tree.links.new(glow.outputs["Emission"], out.inputs["Surface"])
    return mat


def caption(text, x, y, material):
    curve = bpy.data.curves.new(f"caption {text}", "FONT")
    curve.body = text
    curve.size = TILE / 6.5
    curve.align_x = "CENTER"
    curve.materials.append(material)
    obj = bpy.data.objects.new(curve.name, curve)
    obj.location = (x, y, 0.0)
    bpy.context.scene.collection.objects.link(obj)
    return obj


def build(swatches, columns, plain):
    """The tiles, and where each one's middle is."""
    rows = math.ceil(len(swatches) / columns)
    cell_h = TILE + (0 if plain else STRIP + LABEL)
    step_x, step_y = TILE + GAP, cell_h + GAP
    ink = None if plain else exact("caption", INK)
    tiles, places = [], []
    for i, (name, hex_color) in enumerate(swatches):
        col, row = i % columns, i // columns
        x = (col - (columns - 1) / 2) * step_x
        # The first row is the far one, at the top of the picture.
        top = ((rows - 1) / 2 - row) * step_y + cell_h / 2
        y = top - TILE / 2
        tiles.append(box(name, (TILE, TILE, THICK), (x, y, THICK / 2), lit(name, hex_color)))
        strip_y = top - TILE - STRIP / 2
        places.append((name, hex_color, x, y, None if plain else strip_y))
        if plain:
            continue
        box(f"{name} as written", (TILE, STRIP, THICK), (x, strip_y, THICK / 2), exact(f"{name} as written", hex_color))
        caption(name, x, top - TILE - STRIP - LABEL * 0.45, ink)
        caption(hex_color, x, top - TILE - STRIP - LABEL * 0.9, ink)
    width = columns * step_x + GAP
    height = rows * step_y + GAP
    return tiles, places, width, height


def camera(scene, width, height):
    data = bpy.data.cameras.new("maquette_sheet")
    data.type = "ORTHO"
    data.sensor_fit = "HORIZONTAL"
    data.ortho_scale = width
    cam = bpy.data.objects.new(data.name, data)
    cam.location = (0.0, 0.0, 2.0)
    scene.collection.objects.link(cam)
    scene.camera = cam


def export(path, tiles):
    bpy.ops.object.select_all(action="DESELECT")
    for tile in tiles:
        tile.select_set(True)
    bpy.ops.export_scene.gltf(
        filepath=os.path.abspath(path),
        export_format="GLB",
        use_selection=True,
        export_apply=True,
        export_yup=True,
    )


def main():
    args = parse_args()
    swatches = palette(args.swatches)
    bpy.ops.wm.read_factory_settings(use_empty=True)
    scene = bpy.context.scene
    tiles, places, width, height = build(swatches, max(1, args.columns), args.plain)
    if args.glb:
        export(args.glb, tiles)
    rig.world(scene, BACKDROP)
    rig.sun(scene)
    camera(scene, width, height)
    w = args.width
    h = max(1, round(w * height / width))
    light = rig.render(scene, args.out, w, args.samples, height=h)
    shown = rig.encode(rig.tone(light)) * 255
    def mean(values, x, y, reach):
        px, py = int((x / width + 0.5) * w), int((y / height + 0.5) * h)
        r = max(1, int(reach / width * w))
        window = values[py - r : py + r + 1, px - r : px + r + 1]
        return window.reshape(-1, 3).mean(axis=0)

    rows = []
    for name, hex_color, x, y, strip_y in places:
        row = {
            "name": name,
            "hex": hex_color,
            "shown": [round(float(v), 1) for v in mean(shown, x, y, TILE / 4)],
            "light": [round(float(v), 4) for v in mean(light, x, y, TILE / 4)],
        }
        if strip_y is not None:
            row["written"] = [round(float(v), 1) for v in mean(shown, x, strip_y, STRIP / 4)]
        rows.append(row)
    report = {
        "sun": rig.SUN_STRENGTH,
        "fill": rig.WORLD_GREY,
        "tone": rig.TONE,
        "exposure": rig.EXPOSURE,
        "flatLight": round(rig.flat_light(), 4),
        "width": w,
        "height": h,
        "swatches": rows,
    }
    if args.report:
        with open(args.report, "w", encoding="utf-8") as fh:
            json.dump(report, fh, indent=1)
            fh.write("\n")
    print("swatch_sheet", json.dumps({"swatches": len(rows), "out": args.out}))


if __name__ == "__main__":
    main()
