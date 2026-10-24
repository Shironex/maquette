"""Bake light: one Cycles lightmap per mesh, for the web export.

Maquette runs this headless, as the Bake light step:

    blender -b --factory-startup --python-exit-code 1 blender/scene_built.blend \
        --python tools/bake_light.py -- --out-dir blender/bake --out-blend blender/scene_baked.blend \
        [--size 512] [--samples 16] [--type diffuse|combined]

What it bakes
- Every mesh in the scene that renders, except the helpers in the "Studio"
  collection (they still cast light and shadow into the bake).
- --type diffuse (the default) bakes Cycles DIFFUSE with only the DIRECT and
  INDIRECT passes: the light falling on the surface (sun, sky, bounce,
  contact shadows) without its colour. The web page multiplies the albedo
  by it. --type combined bakes the full shaded look (colour included), for
  when you want to show the bake as it is.

The second UV
- Each mesh gets a UV layer named "lightmap", filled by Smart UV Project
  when it is missing (an existing "lightmap" layer is kept, so you can lay
  it out by hand). The first UV layer stays as it is and keeps the textures.
  A mesh without any UV gets an empty "UVMap" first, so "lightmap" is
  always the second set (TEXCOORD_1 in the GLB).
- Linked duplicates are made single user first and every object gets its
  own copies of its materials, so each object has its own lightmap.
  Meshes without a material get a plain grey one so they bake.

The files, in --out-dir
- <object name in snake case>_light.png per object: the light divided by
  the object's scale, stored as sRGB. The scale is at least 1 and is raised
  until the bright spots fit, so light = pixel * scale.
- bake.json: {"baked": true, "type", "size", "samples", "objects": {<object
  name>: {"file", "size", "scale", "uv": "lightmap"}}}. An empty scene
  writes "baked": false and a note, so the pipeline still passes.
The scene, with the lightmap UVs and the material copies, is saved to
--out-blend. The bake target image nodes are removed again before saving.

Speed: the defaults are made for a quick template run (512 px, 16 samples,
CPU), a few seconds per object. For a final bake raise them on the step's
args, for example --size 1024 --samples 256. Low samples look grainy in
the shadows; more samples cost time linearly.

How export uses the maps: tools/export_web.py reads bake.json, keeps both
UV sets, adds each lightmap as the emissive texture on the "lightmap" UV
and sets the material extras {"lightmap": scale, "lightmapUv": 1}. The
page swaps such a material for an unlit one: albedo * lightmap * scale.

Progress: one line per object as it starts and a final one, in the form
"[bake_light] progress 2/5 shop_house", which Maquette shows as a bar.
"""

import argparse
import glob
import json
import math
import os
import re
import sys

import bpy
import numpy as np

STUDIO = "Studio"
UV = "lightmap"
TARGET = "maquette_bake_target"
MARGIN = 4


def log(message):
    print(f"[bake_light] {message}", flush=True)


def parse_args():
    argv = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
    parser = argparse.ArgumentParser(prog="bake_light.py")
    parser.add_argument("--out-dir", required=True, help="folder for baked maps and bake.json")
    parser.add_argument("--out-blend", required=True, help="where to save the baked scene")
    parser.add_argument("--size", type=int, default=512, help="lightmap edge in pixels")
    parser.add_argument("--samples", type=int, default=16, help="Cycles samples per lightmap")
    parser.add_argument(
        "--type", choices=["diffuse", "combined"], default="diffuse", help="what to bake"
    )
    return parser.parse_args(argv)


def snake(name):
    text = re.sub(r"[^0-9a-zA-Z]+", "_", name).strip("_").lower()
    return text or "object"


def bake_targets(scene):
    """The meshes that get a lightmap, in a stable order."""
    studio = bpy.data.collections.get(STUDIO)
    skip = set(studio.all_objects) if studio is not None else set()
    meshes = [
        o
        for o in scene.objects
        if o.type == "MESH" and o not in skip and not o.hide_render and len(o.data.polygons) > 0
    ]
    return sorted(meshes, key=lambda o: o.name)


def setup_cycles(scene, samples):
    scene.render.engine = "CYCLES"
    # CPU works everywhere without a kernel compile; switch to GPU here if
    # you have one set up.
    scene.cycles.device = "CPU"
    scene.cycles.samples = samples
    scene.cycles.use_adaptive_sampling = False
    scene.cycles.use_denoising = False
    scene.cycles.max_bounces = 4
    scene.cycles.diffuse_bounces = 2
    scene.cycles.sample_clamp_indirect = 4.0
    scene.render.bake.margin = MARGIN
    scene.render.bake.margin_type = "EXTEND"


def with_nodes(mat):
    # Blender 5 materials always have nodes; older versions need use_nodes.
    if mat.node_tree is None:
        mat.use_nodes = True
    return mat


def plain_material(obj):
    return with_nodes(bpy.data.materials.new(f"{obj.name} plain"))


def single_user(obj):
    """Own mesh data and own materials, so this object's bake is its own."""
    if obj.data.users > 1:
        obj.data = obj.data.copy()
    if not obj.material_slots:
        obj.data.materials.append(plain_material(obj))
    for slot in obj.material_slots:
        if slot.material is None:
            slot.material = plain_material(obj)
        elif slot.material.users > 1:
            slot.material = slot.material.copy()
        with_nodes(slot.material)


def select_only(obj):
    for other in bpy.context.view_layer.objects:
        other.select_set(False)
    obj.select_set(True)
    bpy.context.view_layer.objects.active = obj


def ensure_lightmap_uv(obj):
    """Adds the "lightmap" UV as the second set when missing and lays it out."""
    uvs = obj.data.uv_layers
    if UV in uvs:
        return False
    if len(uvs) == 0:
        uvs.new(name="UVMap")
    first = uvs[0]
    layer = uvs.new(name=UV)
    uvs.active = layer
    first.active_render = True  # textures keep reading the first UV
    select_only(obj)
    bpy.ops.object.mode_set(mode="EDIT")
    bpy.ops.mesh.select_all(action="SELECT")
    bpy.ops.uv.smart_project(angle_limit=math.radians(66), island_margin=0.02)
    bpy.ops.object.mode_set(mode="OBJECT")
    return True


def set_target(obj, image):
    """An unconnected, active Image Texture node in every material: the bake target."""
    for slot in obj.material_slots:
        tree = slot.material.node_tree
        node = tree.nodes.get(TARGET) or tree.nodes.new("ShaderNodeTexImage")
        node.name = TARGET
        node.image = image
        tree.nodes.active = node


def remove_targets():
    for mat in bpy.data.materials:
        if mat.node_tree is not None and TARGET in mat.node_tree.nodes:
            mat.node_tree.nodes.remove(mat.node_tree.nodes[TARGET])


def srgb(x):
    x = np.clip(x, 0.0, 1.0)
    return np.where(x <= 0.0031308, 12.92 * x, 1.055 * np.power(x, 1 / 2.4) - 0.055)


def save_png(rgb, path):
    """Writes linear rgb (h, w, 3) as an sRGB PNG."""
    h, w, _ = rgb.shape
    img = bpy.data.images.new("maquette_bake_out", w, h, alpha=False)
    rgba = np.concatenate([srgb(rgb), np.ones((h, w, 1), np.float32)], axis=2)
    img.pixels.foreach_set(rgba.astype(np.float32).ravel())
    img.filepath_raw = path
    img.file_format = "PNG"
    img.save()
    bpy.data.images.remove(img)


def bake_one(obj, size, kind):
    """Bakes one object and returns its light as linear (h, w, 3)."""
    image = bpy.data.images.new(f"{obj.name}_light", size, size, alpha=False, float_buffer=True)
    image.colorspace_settings.is_data = True
    uvs = obj.data.uv_layers
    render_uv = next((i for i, layer in enumerate(uvs) if layer.active_render), 0)
    uvs.active = uvs[UV]  # the bake writes through the active UV
    set_target(obj, image)
    select_only(obj)
    if kind == "combined":
        bpy.ops.object.bake(type="COMBINED", margin=MARGIN, use_clear=True)
    else:
        bpy.ops.object.bake(
            type="DIFFUSE", pass_filter={"DIRECT", "INDIRECT"}, margin=MARGIN, use_clear=True
        )
    uvs.active = uvs[0]
    uvs[render_uv].active_render = True
    data = np.empty(size * size * 4, dtype=np.float32)
    image.pixels.foreach_get(data)
    bpy.data.images.remove(image)
    return data.reshape(size, size, 4)[..., :3].copy()


def scale_for(light):
    """At least 1, raised in quarter steps until all but the brightest 0.1% fit."""
    peak = float(np.percentile(light.max(axis=2), 99.9))
    return float(max(1.0, math.ceil(peak * 4) / 4))


def write_report(out_dir, report):
    with open(os.path.join(out_dir, "bake.json"), "w", encoding="utf-8") as fh:
        json.dump(report, fh, indent=2)
        fh.write("\n")


def save_blend(path):
    out = os.path.abspath(path)
    if out == os.path.abspath(bpy.data.filepath):
        log("--out-blend is the open blend, refusing to write over the build")
        sys.exit(1)
    os.makedirs(os.path.dirname(out), exist_ok=True)
    bpy.ops.wm.save_as_mainfile(filepath=out, copy=True)


def main():
    args = parse_args()
    os.makedirs(args.out_dir, exist_ok=True)
    # Maps of objects that are gone would be picked up by the export.
    for old in glob.glob(os.path.join(args.out_dir, "*_light.png")):
        os.remove(old)
    scene = bpy.context.scene
    scene.frame_set(scene.frame_start)
    targets = bake_targets(scene)
    if not targets:
        log("no meshes to bake, the scene keeps real-time lighting")
        write_report(
            args.out_dir,
            {"baked": False, "objects": {}, "note": "No meshes in the scene, nothing was baked."},
        )
        save_blend(args.out_blend)
        log(f"saved {args.out_blend}")
        return

    setup_cycles(scene, args.samples)
    for obj in targets:
        single_user(obj)
    laid_out = sum(ensure_lightmap_uv(obj) for obj in targets)
    log(f"{len(targets)} meshes, {laid_out} new lightmap UVs, {args.type} at {args.size} px, {args.samples} samples")

    objects = {}
    files = set()
    total = len(targets)
    for done, obj in enumerate(targets):
        log(f"progress {done}/{total} {obj.name}")
        light = bake_one(obj, args.size, args.type)
        scale = scale_for(light)
        stem, n = snake(obj.name), 2
        file = f"{stem}_light.png"
        while file in files:
            file, n = f"{stem}_{n}_light.png", n + 1
        files.add(file)
        save_png(light / scale, os.path.join(args.out_dir, file))
        objects[obj.name] = {"file": file, "size": args.size, "scale": scale, "uv": UV}
    log(f"progress {total}/{total} done")

    remove_targets()
    write_report(
        args.out_dir,
        {
            "baked": True,
            "type": args.type,
            "size": args.size,
            "samples": args.samples,
            "objects": objects,
        },
    )
    save_blend(args.out_blend)
    log(f"baked {total} lightmaps into {args.out_dir}, saved {args.out_blend}")


main()
