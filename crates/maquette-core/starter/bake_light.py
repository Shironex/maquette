"""Bake light: one Cycles lightmap per mesh, for the web export.

Maquette runs this headless, as the Bake light step:

    blender -b --factory-startup --python-exit-code 1 blender/scene_built.blend \
        --python tools/bake_light.py -- --out-dir blender/bake --out-blend blender/scene_baked.blend \
        [--size 1024] [--samples 32] [--denoise | --no-denoise] [--type diffuse|combined]

What it bakes
- Every mesh in the scene that renders, except
  - the helpers in the "Studio" collection,
  - meshes tagged "maquette_skip_bake" (Build scene tags every object of an
    asset set to skip the bake, which a generated scan is by default: its
    thousands of tiny UV islands turn a lightmap into flat lit facets),
  - meshes whose every material glows on its own (an emissive or unlit
    material, such as a window's sky): baking would throw away their glow.
  Skipped meshes still cast light and shadow into the bake of the others.
  They get no lightmap and no second UV set, and bake.json lists them.
- --type diffuse (the default) bakes Cycles DIFFUSE with only the DIRECT and
  INDIRECT passes: the light falling on the surface (sun, sky, bounce,
  contact shadows) without its colour. The web page multiplies the albedo
  by it. --type combined bakes the full shaded look (colour included), for
  when you want to show the bake as it is.
- --denoise (the default) runs each lightmap through Blender's denoiser
  (OpenImageDenoise, the compositor's Denoise node), after filling the
  texels between the UV islands so their black never bleeds in.
  --no-denoise keeps the raw samples.

The second UV
- Each baked mesh gets a UV layer named "lightmap", filled by Smart UV
  Project when it is missing (an existing "lightmap" layer is kept, so you
  can lay it out by hand). The first UV layer stays as it is and keeps the
  textures. A mesh without any UV gets an empty "UVMap" first, so
  "lightmap" is always the second set (TEXCOORD_1 in the GLB).
- Linked duplicates are made single user first and every object gets its
  own copies of its materials, so each object has its own lightmap.
  Meshes without a material get a plain grey one so they bake.

The files, in --out-dir
- <object name in snake case>_light.png per baked object: the light divided
  by the object's scale, stored as sRGB. The scale is at least 1 and is
  raised until the bright spots fit, so light = pixel * scale.
- bake.json: {"baked": true, "type", "size", "samples", "denoise",
  "objects": {<object name>: {"file", "size", "scale", "uv": "lightmap"}},
  "skipped": {<object name>: "skip-bake" | "glows"}}. An empty scene writes
  "baked": false and a note, so the pipeline still passes.
The scene, with the lightmap UVs and the material copies, is saved to
--out-blend. The bake target image nodes are removed again before saving.

Speed: the defaults (1024 px, 32 samples, denoised, CPU) are made for close
camera stops in a small scene. The time grows about linearly with the
pixels times the samples, per baked mesh: 1024 px at 32 samples takes about
8 times as long as 512 px at 16. The denoiser adds about half a second per
map and is what makes few samples clean. For a quick look lower them on the
step's args (Pipeline, Bake settings), for example --size 512 --samples 16.

How export uses the maps: tools/export_web.py reads bake.json, keeps both
UV sets, adds each lightmap as the emissive texture on the "lightmap" UV of
every material that does not glow itself, and sets the material extras
{"lightmap": scale, "lightmapUv": 1}. The page moves such a texture to a
light map: albedo * lightmap * scale.

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
import time

import bpy
import numpy as np

STUDIO = "Studio"
UV = "lightmap"
TARGET = "maquette_bake_target"
SKIP_TAG = "maquette_skip_bake"
DENOISE_SCENE = "maquette_denoise"


def log(message):
    print(f"[bake_light] {message}", flush=True)


def parse_args():
    argv = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
    parser = argparse.ArgumentParser(prog="bake_light.py")
    parser.add_argument("--out-dir", required=True, help="folder for baked maps and bake.json")
    parser.add_argument("--out-blend", required=True, help="where to save the baked scene")
    parser.add_argument("--size", type=int, default=1024, help="lightmap edge in pixels")
    parser.add_argument("--samples", type=int, default=32, help="Cycles samples per lightmap")
    parser.add_argument(
        "--denoise",
        action=argparse.BooleanOptionalAction,
        default=True,
        help="denoise each lightmap (OpenImageDenoise)",
    )
    parser.add_argument(
        "--type", choices=["diffuse", "combined"], default="diffuse", help="what to bake"
    )
    return parser.parse_args(argv)


def snake(name):
    text = re.sub(r"[^0-9a-zA-Z]+", "_", name).strip("_").lower()
    return text or "object"


def margin_for(size):
    """The bake margin in pixels: wider for bigger maps, so mip levels keep it."""
    return max(4, size // 128)


def principled(material):
    if material is None or material.node_tree is None:
        return None
    return next((n for n in material.node_tree.nodes if n.type == "BSDF_PRINCIPLED"), None)


def glows(material):
    """True for a material that lights itself: an unlit emission or
    background shader, or a Principled BSDF with a live emission."""
    if material is None or material.node_tree is None:
        return False
    bsdf = principled(material)
    if bsdf is None:
        return any(n.type in {"EMISSION", "BACKGROUND"} for n in material.node_tree.nodes)
    strength = bsdf.inputs["Emission Strength"]
    color = bsdf.inputs["Emission Color"]
    strong = strength.is_linked or strength.default_value > 0.0
    lit = color.is_linked or max(color.default_value[:3]) > 0.0
    return strong and lit


def skip_reason(obj):
    """Why a mesh gets no lightmap, or None when it is baked."""
    if obj.get(SKIP_TAG):
        return "skip-bake"
    materials = [s.material for s in obj.material_slots if s.material is not None]
    if materials and all(glows(m) for m in materials):
        return "glows"
    return None


def bake_targets(scene):
    """The meshes that get a lightmap in a stable order, and the skipped ones."""
    studio = bpy.data.collections.get(STUDIO)
    helpers = set(studio.all_objects) if studio is not None else set()
    meshes = [
        o
        for o in scene.objects
        if o.type == "MESH" and o not in helpers and not o.hide_render and len(o.data.polygons) > 0
    ]
    targets, skipped = [], {}
    for obj in sorted(meshes, key=lambda o: o.name):
        reason = skip_reason(obj)
        if reason is None:
            targets.append(obj)
        else:
            skipped[obj.name] = reason
    return targets, skipped


def setup_cycles(scene, samples, size):
    scene.render.engine = "CYCLES"
    # CPU works everywhere without a kernel compile; switch to GPU here if
    # you have one set up.
    scene.cycles.device = "CPU"
    scene.cycles.samples = samples
    scene.cycles.use_adaptive_sampling = False
    # Cycles does not denoise bakes; --denoise does it on the maps instead.
    scene.cycles.use_denoising = False
    scene.cycles.max_bounces = 4
    scene.cycles.diffuse_bounces = 2
    scene.cycles.sample_clamp_indirect = 4.0
    scene.render.bake.margin = margin_for(size)
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
    """Bakes one object and returns its light as linear (h, w, 3) and the
    mask of the texels the bake wrote (the islands and their margin)."""
    image = bpy.data.images.new(f"{obj.name}_light", size, size, alpha=True, float_buffer=True)
    image.colorspace_settings.is_data = True
    uvs = obj.data.uv_layers
    render_uv = next((i for i, layer in enumerate(uvs) if layer.active_render), 0)
    uvs.active = uvs[UV]  # the bake writes through the active UV
    set_target(obj, image)
    select_only(obj)
    margin = margin_for(size)
    if kind == "combined":
        bpy.ops.object.bake(type="COMBINED", margin=margin, use_clear=True)
    else:
        bpy.ops.object.bake(
            type="DIFFUSE", pass_filter={"DIRECT", "INDIRECT"}, margin=margin, use_clear=True
        )
    uvs.active = uvs[0]
    uvs[render_uv].active_render = True
    data = np.empty(size * size * 4, dtype=np.float32)
    image.pixels.foreach_get(data)
    bpy.data.images.remove(image)
    pixels = data.reshape(size, size, 4)
    return pixels[..., :3].copy(), pixels[..., 3] > 0.5


def fill_gaps(rgb, mask):
    """Fills the texels outside `mask` from the nearest islands (push pull),
    so the denoiser and the mip levels never pull black into an island."""
    if mask.all() or not mask.any():
        return rgb
    levels = [(rgb * mask[..., None], mask.astype(np.float32))]
    while min(levels[-1][1].shape) > 1:
        colour, weight = levels[-1]
        h, w = weight.shape
        h2, w2 = h - h % 2, w - w % 2
        c = colour[:h2, :w2].reshape(h2 // 2, 2, w2 // 2, 2, 3).sum(axis=(1, 3))
        wt = weight[:h2, :w2].reshape(h2 // 2, 2, w2 // 2, 2).sum(axis=(1, 3))
        levels.append((c, wt))
    colour, weight = levels[-1]
    filled = colour / np.maximum(weight, 1e-8)[..., None]
    for colour, weight in reversed(levels[:-1]):
        h, w = weight.shape
        up = np.repeat(np.repeat(filled, 2, axis=0), 2, axis=1)
        up = np.pad(up, ((0, h - up.shape[0]), (0, w - up.shape[1]), (0, 0)), mode="edge")
        own = colour / np.maximum(weight, 1e-8)[..., None]
        known = (weight > 0)[..., None]
        filled = np.where(known, own, up)
    return np.where(mask[..., None], rgb, filled).astype(np.float32)


class Denoiser:
    """OpenImageDenoise through the compositor's Denoise node, run on a
    small scene of its own: an Image node feeds the denoiser, a render
    writes the result to a float EXR, and that is read back."""

    def __init__(self, work_dir):
        self.path = os.path.join(work_dir, ".maquette_denoise.exr")
        scene = bpy.data.scenes.new(DENOISE_SCENE)
        scene.render.engine = "BLENDER_WORKBENCH"
        scene.render.resolution_percentage = 100
        scene.render.use_compositing = True
        scene.render.image_settings.file_format = "OPEN_EXR"
        scene.render.image_settings.color_depth = "32"
        scene.render.filepath = self.path
        camera = bpy.data.objects.new(DENOISE_SCENE, bpy.data.cameras.new(DENOISE_SCENE))
        scene.collection.objects.link(camera)
        scene.camera = camera
        tree = bpy.data.node_groups.new(DENOISE_SCENE, "CompositorNodeTree")
        tree.interface.new_socket("Image", in_out="OUTPUT", socket_type="NodeSocketColor")
        self.source = tree.nodes.new("CompositorNodeImage")
        node = tree.nodes.new("CompositorNodeDenoise")
        node.inputs["HDR"].default_value = True
        out = tree.nodes.new("NodeGroupOutput")
        tree.links.new(self.source.outputs["Image"], node.inputs["Image"])
        tree.links.new(node.outputs["Image"], out.inputs[0])
        scene.compositing_node_group = tree
        self.scene = scene

    def run(self, rgb):
        h, w, _ = rgb.shape
        image = bpy.data.images.new("maquette_denoise_in", w, h, alpha=False, float_buffer=True)
        image.colorspace_settings.is_data = True
        image.pixels.foreach_set(
            np.concatenate([rgb, np.ones((h, w, 1), np.float32)], axis=2).astype(np.float32).ravel()
        )
        self.source.image = image
        self.scene.render.resolution_x = w
        self.scene.render.resolution_y = h
        bpy.ops.render.render(write_still=True, scene=self.scene.name)
        result = bpy.data.images.load(self.path)
        result.colorspace_settings.is_data = True
        data = np.empty(w * h * 4, dtype=np.float32)
        result.pixels.foreach_get(data)
        bpy.data.images.remove(result)
        bpy.data.images.remove(image)
        return np.maximum(data.reshape(h, w, 4)[..., :3], 0.0)

    def close(self):
        tree = self.scene.compositing_node_group
        bpy.data.scenes.remove(self.scene)
        if tree is not None:
            bpy.data.node_groups.remove(tree)
        if os.path.exists(self.path):
            os.remove(self.path)


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


def describe_skipped(skipped):
    by_reason = {}
    for name, reason in skipped.items():
        by_reason.setdefault(reason, []).append(name)
    words = {"skip-bake": "set to skip the bake", "glows": "glow on their own"}
    return ", ".join(f"{len(names)} {words.get(r, r)}" for r, names in sorted(by_reason.items()))


def main():
    args = parse_args()
    os.makedirs(args.out_dir, exist_ok=True)
    # Maps of objects that are gone would be picked up by the export.
    for old in glob.glob(os.path.join(args.out_dir, "*_light.png")):
        os.remove(old)
    scene = bpy.context.scene
    scene.frame_set(scene.frame_start)
    targets, skipped = bake_targets(scene)
    if skipped:
        log(f"no lightmap for {len(skipped)} meshes: {describe_skipped(skipped)}")
    if not targets:
        log("no meshes to bake, the scene keeps real-time lighting")
        write_report(
            args.out_dir,
            {
                "baked": False,
                "objects": {},
                "skipped": skipped,
                "note": "No meshes to bake in the scene, nothing was baked.",
            },
        )
        save_blend(args.out_blend)
        log(f"saved {args.out_blend}")
        return

    setup_cycles(scene, args.samples, args.size)
    for obj in targets:
        single_user(obj)
    laid_out = sum(ensure_lightmap_uv(obj) for obj in targets)
    log(
        f"{len(targets)} meshes, {laid_out} new lightmap UVs, {args.type} at {args.size} px, "
        f"{args.samples} samples, {'denoised' if args.denoise else 'not denoised'}"
    )
    denoiser = Denoiser(args.out_dir) if args.denoise else None

    objects = {}
    files = set()
    total = len(targets)
    started = time.monotonic()
    try:
        for done, obj in enumerate(targets):
            log(f"progress {done}/{total} {obj.name}")
            light, mask = bake_one(obj, args.size, args.type)
            light = fill_gaps(light, mask)
            if denoiser is not None:
                light = denoiser.run(light)
            scale = scale_for(light)
            stem, n = snake(obj.name), 2
            file = f"{stem}_light.png"
            while file in files:
                file, n = f"{stem}_{n}_light.png", n + 1
            files.add(file)
            save_png(light / scale, os.path.join(args.out_dir, file))
            objects[obj.name] = {"file": file, "size": args.size, "scale": scale, "uv": UV}
    finally:
        if denoiser is not None:
            denoiser.close()
    log(f"progress {total}/{total} done in {time.monotonic() - started:.1f} s")

    remove_targets()
    write_report(
        args.out_dir,
        {
            "baked": True,
            "type": args.type,
            "size": args.size,
            "samples": args.samples,
            "denoise": args.denoise,
            "objects": objects,
            "skipped": skipped,
        },
    )
    save_blend(args.out_blend)
    log(f"baked {total} lightmaps into {args.out_dir}, saved {args.out_blend}")


main()
