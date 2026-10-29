"""Export: one plain model file per game model, in the engine's format.

Maquette runs this headless, as the Export step of a game project:

    blender -b --factory-startup --python-exit-code 1 blender/game_built.blend \\
        --python tools/export_game.py -- --index web/game-index.json --out web/raw \\
        --format glb --textures png --cap 1024 [--bake blender/bake]

What becomes a file: each collection under "Models" that Build models
tagged "maquette_target" is written to web/raw/<target>, for example
web/raw/props/brazier.glb, in the format of the target's extension (--format
for a target without one). The folder is emptied first.

- .glb: binary glTF with the textures inside as PNG or JPG (--textures png,
  jpg, or keep for each image's own), never KTX2, never meshopt or Draco.
  The rig and its animation clips export with the model: each armature
  carries only its own actions (its active one and its NLA tracks), so a
  rig never carries the clips of another model in the same file.
- With "dropMaps": true in the game index (a Game export setting, for a
  game whose shader reads neither), normal maps and metal-roughness maps
  are left out: each Principled BSDF's Normal, Metallic and Roughness
  inputs are unhooked from their textures and keep their plain values.
- .obj: Wavefront .obj with its .mtl, Y up, and each texture saved next to
  them as a .png or .jpg file the .mtl names without a folder.
- Each model's textures scale down to the "textureCap" the index gives it,
  the texture size of its folder's budget, on their longest edge. --cap is
  for a model the index gives none. A smaller texture stays as it is.
- Objects and materials Build models tagged "maquette_name" export under
  the name their model file gives.
- A model Build models moved apart for the bake goes back to where its
  file has it.
- --bake is the Bake light folder, set only when Bake light is on: each
  baked object gets its lightmap as the emissive texture on its second UV
  set, with the material extras {"lightmap": scale, "lightmapUv": 1}, as
  the website export does. An engine that does not read them can leave
  them.

web/raw/export.json lists what was written, with the triangles and the
texture sizes, for Game export to check.

Edit freely: this file is yours.
"""

import argparse
import json
import os
import shutil
import sys

import bpy

MODELS = "Models"
TARGET = "maquette_target"
OFFSET = "maquette_offset"
SOURCE_NAME = "maquette_name"
TAG_PREFIX = "maquette_"
EXPORTED_TYPES = {"MESH", "EMPTY", "ARMATURE", "CURVE", "SURFACE", "META", "FONT"}


def log(message):
    print(f"[export_game] {message}", flush=True)


def parse_args():
    argv = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
    parser = argparse.ArgumentParser(prog="export_game.py")
    parser.add_argument("--index", default="web/game-index.json", help="the game index")
    parser.add_argument("--out", required=True, help="folder for the model files")
    parser.add_argument("--format", choices=["glb", "obj"], default="glb")
    parser.add_argument("--textures", choices=["png", "jpg", "keep"], default="png")
    parser.add_argument(
        "--cap", type=int, default=1024, help="longest texture edge in px, without a folder budget"
    )
    parser.add_argument("--bake", default=None, help="the Bake light folder, when it ran")
    return parser.parse_args(argv)


# Baked light, as tools/export_web.py hooks it.


def principled(material):
    if material is None or material.node_tree is None:
        return None
    return next((n for n in material.node_tree.nodes if n.type == "BSDF_PRINCIPLED"), None)


def glows(material):
    if material is None or material.node_tree is None:
        return False
    bsdf = principled(material)
    if bsdf is None:
        return any(n.type in {"EMISSION", "BACKGROUND"} for n in material.node_tree.nodes)
    strength = bsdf.inputs["Emission Strength"]
    color = bsdf.inputs["Emission Color"]
    return (strength.is_linked or strength.default_value > 0.0) and (
        color.is_linked or max(color.default_value[:3]) > 0.0
    )


def hook_lightmaps(folder):
    path = os.path.join(folder, "bake.json")
    if not os.path.isfile(path):
        log(f"no {path}, exported without baked light")
        return
    with open(path, encoding="utf-8") as fh:
        report = json.load(fh)
    lit = 0
    for name, entry in (report.get("objects") or {}).items() if report.get("baked") else []:
        obj = bpy.data.objects.get(name)
        image_path = os.path.join(folder, entry["file"])
        if obj is None or obj.type != "MESH" or not os.path.isfile(image_path):
            continue
        uvs = obj.data.uv_layers
        index = uvs.find(entry.get("uv", "lightmap"))
        if index < 0:
            continue
        image = bpy.data.images.load(os.path.abspath(image_path), check_existing=True)
        for slot in obj.material_slots:
            bsdf = principled(slot.material)
            if bsdf is None or glows(slot.material):
                continue
            tree = slot.material.node_tree
            uv = tree.nodes.new("ShaderNodeUVMap")
            uv.uv_map = uvs[index].name
            tex = tree.nodes.new("ShaderNodeTexImage")
            tex.image = image
            tree.links.new(uv.outputs["UV"], tex.inputs["Vector"])
            tree.links.new(tex.outputs["Color"], bsdf.inputs["Emission Color"])
            bsdf.inputs["Emission Strength"].default_value = 1.0
            slot.material["lightmap"] = float(entry["scale"])
            slot.material["lightmapUv"] = index
        lit += 1
    log(f"baked light on {lit} objects")


# Names, as tools/export_web.py keeps them.


def take_source_names():
    found = {}
    for pool in (bpy.data.objects, bpy.data.materials):
        for item in pool:
            if SOURCE_NAME in item:
                found[item.as_pointer()] = str(item[SOURCE_NAME])
    return found


def own_names(items_by_pool, originals):
    """Gives these datablocks their file's names for one export; a holder
    outside the export steps aside. Returns how to undo it."""
    undo = []
    for pool, items in items_by_pool:
        moving = []
        for item in items:
            name = originals.get(item.as_pointer())
            if name and item.name != name:
                moving.append((item, name))
        for i, (item, _) in enumerate(moving):
            undo.append((item, item.name))
            item.name = f"export_tmp_{i}"
        inside = set(items)
        for _, name in moving:
            holder = pool.get(name)
            if holder is not None and holder not in inside:
                undo.append((holder, holder.name))
                holder.name = f"{name}.held"
        for item, name in moving:
            item.name = name
    return undo


def undo_names(undo):
    first = {}
    for item, name in undo:
        first.setdefault(item.as_pointer(), (item, name))
    restore = list(first.values())
    for i, (item, _) in enumerate(restore):
        item.name = f"export_undo_{i}"
    for item, name in restore:
        item.name = name


def drop_tags(collection):
    for item in list(collection.all_objects) + materials_of(collection.all_objects):
        for key in [k for k in item.keys() if k.startswith(TAG_PREFIX)]:
            del item[key]


# Textures.


def materials_of(objects):
    out = []
    for obj in objects:
        for slot in getattr(obj, "material_slots", []):
            if slot.material is not None and slot.material not in out:
                out.append(slot.material)
    return out


def images_of(objects):
    out = []
    for material in materials_of(objects):
        if material.node_tree is None:
            continue
        for node in material.node_tree.nodes:
            image = getattr(node, "image", None)
            if node.type == "TEX_IMAGE" and image is not None and image not in out:
                out.append(image)
    return out


def ext_for(image, textures):
    if textures == "jpg":
        return "jpg"
    if textures == "png":
        return "png"
    return "jpg" if image.file_format == "JPEG" else "png"


def cap_images(images, cap, textures):
    """Scales every image down to the cap and sets the format it ships in."""
    capped = 0
    for image in images:
        width, height = image.size
        edge = max(width, height)
        if edge > cap > 0:
            factor = cap / edge
            image.scale(max(1, round(width * factor)), max(1, round(height * factor)))
            capped += 1
        image.file_format = "JPEG" if ext_for(image, textures) == "jpg" else "PNG"
    return capped


def save_images(images, folder, textures):
    """Saves each image next to an .obj, under a name of its own."""
    os.makedirs(folder, exist_ok=True)
    taken = set()
    for image in images:
        ext = ext_for(image, textures)
        stem = os.path.splitext(bpy.path.basename(image.filepath) or image.name)[0] or "texture"
        name, n = f"{stem}.{ext}", 2
        while name in taken:
            name, n = f"{stem}_{n}.{ext}", n + 1
        taken.add(name)
        path = os.path.join(folder, name)
        image.filepath_raw = path
        image.file_format = "JPEG" if ext == "jpg" else "PNG"
        image.save()
        if image.packed_file is not None:
            image.unpack(method="REMOVE")
            image.filepath = path


DATA_INPUTS = ("Normal", "Metallic", "Roughness")


def drop_data_maps(materials):
    """Unhooks the normal and metal-roughness textures of each Principled
    BSDF, so the export writes neither map. Returns how many links went."""
    dropped = 0
    for material in materials:
        tree = material.node_tree
        if tree is None:
            continue
        for node in tree.nodes:
            if node.type != "BSDF_PRINCIPLED":
                continue
            for name in DATA_INPUTS:
                socket = node.inputs.get(name)
                for link in list(socket.links) if socket is not None else []:
                    tree.links.remove(link)
                    dropped += 1
    return dropped


def texture_rows(images):
    return [
        {"name": image.name, "width": image.size[0], "height": image.size[1],
         "format": image.file_format}
        for image in images
    ]


# Export.


def select(objects):
    bpy.ops.object.select_all(action="DESELECT")
    for obj in objects:
        obj.hide_set(False)
        obj.select_set(True)
    bpy.context.view_layer.objects.active = objects[0]


def export_glb(path, objects, textures):
    select(objects)
    options = dict(
        filepath=path,
        export_format="GLB",
        use_selection=True,
        export_apply=True,
        export_extras=True,
        export_cameras=False,
        export_lights=False,
        export_yup=True,
        export_skins=True,
        export_animations=True,
        export_image_format="JPEG" if textures == "jpg" else "AUTO",
        export_draco_mesh_compression_enable=False,
    )
    try:
        # Off, a lone armature does not take every action in the file: it
        # carries its own (its active action and NLA tracks) and no other's.
        bpy.ops.export_scene.gltf(**options, export_anim_single_armature=False)
    except TypeError:  # an exporter without the option
        bpy.ops.export_scene.gltf(**options)


def export_obj(path, objects):
    select(objects)
    bpy.ops.wm.obj_export(
        filepath=path,
        export_selected_objects=True,
        export_materials=True,
        path_mode="STRIP",
        forward_axis="NEGATIVE_Z",
        up_axis="Y",
    )


def triangles(objects):
    depsgraph = bpy.context.evaluated_depsgraph_get()
    total = 0
    for obj in objects:
        if obj.type != "MESH":
            continue
        mesh = obj.evaluated_get(depsgraph).to_mesh()
        mesh.calc_loop_triangles()
        total += len(mesh.loop_triangles)
        obj.evaluated_get(depsgraph).to_mesh_clear()
    return total


def read_index(path):
    """The game index, or None when it cannot be read."""
    try:
        with open(path, encoding="utf-8") as fh:
            return json.load(fh)
    except (OSError, ValueError):
        return None


def caps_of(index, path, fallback):
    """Each target's texture cap from the game index, its folder's budget."""
    if index is None:
        log(f"no {path}, every texture caps at {fallback} px")
        return {}
    return {
        m["target"]: int(m.get("textureCap") or fallback)
        for m in index.get("models") or []
        if m.get("target")
    }


def export_one(collection, args, originals, caps, drop_maps):
    target = str(collection[TARGET])
    cap = caps.get(target, args.cap)
    move = float(collection.get(OFFSET, 0.0))
    if move:
        for obj in collection.all_objects:
            if obj.parent is None:
                obj.location.x -= move
        bpy.context.view_layer.update()
    objects = [o for o in collection.all_objects if o.type in EXPORTED_TYPES]
    if not objects:
        log(f"{collection.name}: nothing to export")
        return None
    ext = os.path.splitext(target)[1].lower().lstrip(".")
    fmt = ext if ext in ("glb", "obj") else args.format
    path = os.path.abspath(os.path.join(args.out, target))
    os.makedirs(os.path.dirname(path), exist_ok=True)
    dropped = drop_data_maps(materials_of(objects)) if drop_maps else 0
    images = images_of(objects)
    capped = cap_images(images, cap, args.textures)
    undo = own_names(
        [(bpy.data.objects, objects), (bpy.data.materials, materials_of(objects))], originals
    )
    drop_tags(collection)
    try:
        if fmt == "obj":
            save_images(images, os.path.dirname(path), args.textures)
            export_obj(path, objects)
        else:
            export_glb(path, objects, args.textures)
    finally:
        undo_names(undo)
    count = triangles(objects)
    shrunk = f", {capped} textures to {cap} px" if capped else ""
    shrunk += f", {dropped} normal and metal-roughness links dropped" if dropped else ""
    log(f"wrote {target} ({count} triangles{shrunk}, {os.path.getsize(path) // 1024} KB)")
    return {"asset": collection.name, "target": target, "triangles": count, "cap": cap,
            "textures": texture_rows(images), "droppedMaps": dropped}


def main():
    args = parse_args()
    if os.path.isdir(args.out):
        shutil.rmtree(args.out)
    os.makedirs(args.out, exist_ok=True)
    if args.bake:
        hook_lightmaps(args.bake)
    originals = take_source_names()
    index = read_index(args.index)
    caps = caps_of(index, args.index, args.cap)
    drop_maps = bool((index or {}).get("dropMaps"))
    models = bpy.data.collections.get(MODELS)
    tagged = [c for c in (models.children if models else []) if TARGET in c]
    written = [w for w in (export_one(c, args, originals, caps, drop_maps) for c in tagged) if w]
    report = {"version": 1, "format": args.format, "textures": args.textures,
              "cap": args.cap, "dropMaps": drop_maps, "models": written}
    with open(os.path.join(args.out, "export.json"), "w", encoding="utf-8") as fh:
        json.dump(report, fh, indent=2)
    log(f"exported {len(written)} models to {args.out}")


main()
