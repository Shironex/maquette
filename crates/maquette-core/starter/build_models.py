"""Build models: put each game model that needs work into one Blender file.

Maquette runs this headless, as the Build models step of a game project:

    blender -b --factory-startup --python-exit-code 1 --python tools/build_models.py -- \\
        --index web/game-index.json --prepare prepare.json --out blender/game_built.blend [--spread]

- The game index (web/game-index.json, written by Maquette before every run)
  lists every asset with its newest model ("source"), where it goes in the
  game ("target") and "asIs". A model marked asIs is still the repo file it
  was imported from, with no fix-ups: it is left out here and Game export
  copies it as it is.
- Every other model is imported (.glb or .obj) into its own collection
  under "Models", named after its asset and tagged "maquette_target" with
  its target, which Export reads.
- Blender keeps names unique, so a second model's "Material" comes in as
  "Material.001". Each renamed object and material is tagged
  "maquette_name" with the name from its file, and Export writes that name
  back.
- Each asset's fix-ups and real size (prepare.json) are applied by
  fixups.py next to this script, as Build scene does for a website. An
  asset set to skip the bake has its objects tagged "maquette_skip_bake".
- --spread (set when Bake light is on) stands the models apart along X so
  they do not shade each other in the bake. Each collection remembers how
  far it moved ("maquette_offset") and Export puts it back, so every model
  ships where its file has it.
- The result is saved to --out, also with no model to build, so the next
  step always finds it. The models in models/ are never written.

Edit freely: this file is yours. Maquette only reruns the step when this
script, fixups.py, the index, a model or prepare.json changes.
"""

import argparse
import json
import os
import re
import struct
import sys

import bpy
from mathutils import Vector

MODELS = "Models"
SOURCE_NAME = "maquette_name"
TARGET = "maquette_target"
OFFSET = "maquette_offset"
SKIP_BAKE = "maquette_skip_bake"
SUFFIX = re.compile(r"^(.*)\.\d{3,}$")
INDEX_VERSION = 1
GAP = 1.0


def log(message):
    print(f"[build_models] {message}", flush=True)


def parse_args():
    argv = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
    parser = argparse.ArgumentParser(prog="build_models.py")
    parser.add_argument("--index", default="web/game-index.json", help="the game index")
    parser.add_argument("--prepare", default="prepare.json", help="real sizes and fix-ups")
    parser.add_argument("--out", required=True, help="where to save the built blend")
    parser.add_argument("--spread", action="store_true", help="stand the models apart")
    return parser.parse_args(argv)


def read_index(path):
    with open(path, encoding="utf-8") as fh:
        index = json.load(fh)
    if index.get("version", 1) > INDEX_VERSION:
        sys.exit(f"[build_models] {path} comes from a newer Maquette")
    return index


def file_names(path):
    """The object and material names a model file gives."""
    try:
        if path.lower().endswith(".obj"):
            objects, materials = set(), set()
            with open(path, encoding="utf-8", errors="replace") as fh:
                for line in fh:
                    if line.startswith(("o ", "g ")):
                        objects.add(line[2:].strip())
                    elif line.startswith("usemtl "):
                        materials.add(line[7:].strip())
            return objects, materials
        with open(path, "rb") as fh:
            data = fh.read()
        length = struct.unpack_from("<I", data, 12)[0]
        doc = json.loads(data[20 : 20 + length])
    except (OSError, ValueError, struct.error):
        return set(), set()
    nodes = {n.get("name") for n in doc.get("nodes", []) if n.get("name")}
    materials = {m.get("name") for m in doc.get("materials", []) if m.get("name")}
    return nodes, materials


def keep_source_names(path, objects):
    """Tags what Blender renamed on import with the name from the file."""
    nodes, materials = file_names(path)

    def original(name, names):
        if name in names:
            return None
        match = SUFFIX.match(name)
        return match.group(1) if match and match.group(1) in names else None

    for obj in objects:
        was = original(obj.name, nodes)
        if was:
            obj[SOURCE_NAME] = was
        for slot in getattr(obj, "material_slots", []):
            material = slot.material
            if material is None or SOURCE_NAME in material:
                continue
            was = original(material.name, materials)
            if was:
                material[SOURCE_NAME] = was


def import_file(path):
    if path.lower().endswith(".obj"):
        bpy.ops.wm.obj_import(filepath=path)
        return
    try:
        # No icosphere bone shapes for rigged models: they would export as a mesh.
        bpy.ops.import_scene.gltf(filepath=path, disable_bone_shape=True)
    except TypeError:  # an importer without the option
        bpy.ops.import_scene.gltf(filepath=path)


def import_model(model, parent):
    """Imports one model into a new collection named after its asset."""
    path = os.path.abspath(model["source"])
    if not os.path.isfile(path):
        log(f"{model['asset']}: {model['source']} is missing, left out")
        return None
    before = set(bpy.data.objects)
    import_file(path)
    added = [o for o in bpy.data.objects if o not in before]
    keep_source_names(path, added)
    target = bpy.data.collections.new(model["asset"])
    target[TARGET] = model["target"]
    parent.children.link(target)
    for obj in added:
        for owner in list(obj.users_collection):
            owner.objects.unlink(obj)
        target.objects.link(obj)
    log(f"imported {model['source']} for {model['target']}: {len(added)} objects")
    return target


def load_fixups():
    """fixups.py from this script's folder, or None when it is missing."""
    sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
    try:
        import fixups
    except ImportError as err:
        log(f"fixups.py is missing, so real sizes and fix-ups are left out ({err})")
        return None
    return fixups


def prepare(collections, path):
    fixups = load_fixups()
    if fixups is None:
        return
    entries = fixups.read_entries(path)
    for collection in collections:
        entry = entries.get(collection.name)
        if not entry:
            continue
        if entry.get("skipBake") is True:
            for obj in collection.all_objects:
                obj[SKIP_BAKE] = True
        say = lambda message, name=collection.name: log(f"{name} {message}")
        fixups.prepare(list(collection.all_objects), entry, say)
    bpy.context.view_layer.update()


def width(collection):
    xs = [
        (obj.matrix_world @ Vector(corner)).x
        for obj in collection.all_objects
        if obj.type == "MESH"
        for corner in obj.bound_box
    ]
    return (min(xs), max(xs)) if xs else (0.0, 0.0)


def spread(collections):
    """Stands the models side by side along X and remembers the move."""
    x = 0.0
    for collection in collections:
        lo, hi = width(collection)
        move = x - lo
        for obj in collection.all_objects:
            if obj.parent is None:
                obj.location.x += move
        collection[OFFSET] = move
        x += (hi - lo) + GAP
    bpy.context.view_layer.update()
    log(f"stood {len(collections)} models apart for the bake")


def main():
    args = parse_args()
    index = read_index(args.index)
    bpy.ops.wm.read_factory_settings(use_empty=True)
    scene = bpy.context.scene
    parent = bpy.data.collections.new(MODELS)
    scene.collection.children.link(parent)
    work = [m for m in index.get("models", []) if not m.get("asIs")]
    kept = len(index.get("models", [])) - len(work)
    if kept:
        log(f"{kept} models are the game's own files, untouched: Game export copies them")
    added = [c for c in (import_model(m, parent) for m in work) if c is not None]
    bpy.context.view_layer.update()
    prepare(added, args.prepare)
    if args.spread and len(added) > 1:
        spread(added)
    out = os.path.abspath(args.out)
    os.makedirs(os.path.dirname(out), exist_ok=True)
    bpy.ops.wm.save_as_mainfile(filepath=out)
    log(f"saved {args.out} with {len(added)} models")


main()
