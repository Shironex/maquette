"""Export one model of a pack, then open what it wrote again to check it.

Maquette runs this headless, once per model of a pack, with fixups.py in
the same folder:

    blender -b --factory-startup --python-exit-code 1 --python export_pack.py -- \\
        --job job.json --report report.json

job.json says what to read and what to write:

    {"input": "/abs/model.glb", "key": "cast_iron_stove", "out": "/abs/pack/cast_iron_stove",
     "plan": {...}, "obj": true, "glb": false, "glbOut": null, "rename": true,
     "stripClips": false, "texture": 1024}

- "plan" is one asset's real size and fix-ups, as prepare.json holds them,
  with the pack's own added: a decimate to the kind's budget, shrink
  textures to the pack's size and the origin at the bottom centre. fixups.py
  applies them in its fixed order, so a pack model matches the project's.
- "obj" writes <key>.obj with its .mtl and each texture as a PNG beside
  them, Y up, metres. A rigged model is written in its rest pose.
- "glb" writes <key>.glb with the textures inside, the rig and its clips;
  "glbOut" puts it somewhere else (the compress step's input).
- "rename" names the materials body, body_2 and so on.
- "stripClips" drops every animation clip before anything is written: the
  clips came from Mixamo, which allows using them but not sharing them as
  files. The bones stay.

Then each written file is imported into an empty scene, the way the other
side will open it, and measured: triangles, height, where it stands, its
materials, textures that did not load, and clips left. report.json holds
the numbers before and after, the files and what the check found.
"""

import argparse
import json
import os
import sys

import bpy

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import fixups  # noqa: E402

# How much of its surface a decimated model may lose before thin parts
# (tines, spokes, handles) count as gone.
THIN_LOSS = 0.06
# How far off the floor or off centre the pivot may sit, in metres.
PIVOT_SLACK = 0.01


def log(message):
    print(f"[export_pack] {message}", flush=True)


def parse_args():
    argv = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
    parser = argparse.ArgumentParser(prog="export_pack.py")
    parser.add_argument("--job", required=True)
    parser.add_argument("--report", required=True)
    return parser.parse_args(argv)


def import_model(path):
    ext = os.path.splitext(path)[1].lower()
    path = os.path.abspath(path)
    if ext == ".obj":
        bpy.ops.wm.obj_import(filepath=path, forward_axis="NEGATIVE_Z", up_axis="Y")
    elif ext == ".fbx":
        bpy.ops.import_scene.fbx(filepath=path)
    else:
        try:
            bpy.ops.import_scene.gltf(filepath=path, disable_bone_shape=True)
        except TypeError:  # an importer without the option
            bpy.ops.import_scene.gltf(filepath=path)
    bpy.context.view_layer.update()
    return list(bpy.data.objects)


def surface(objects):
    total = 0.0
    for obj in fixups.meshes(objects):
        scale = obj.matrix_world.to_scale()
        factor = abs(scale.x * scale.y * scale.z) ** (2.0 / 3.0)
        total += sum(p.area for p in obj.data.polygons) * factor
    return total


def materials_of(objects):
    found = []
    for obj in fixups.meshes(objects):
        for slot in obj.material_slots:
            if slot.material is not None and slot.material not in found:
                found.append(slot.material)
    return found


def rename_materials(objects):
    names = []
    for i, material in enumerate(materials_of(objects)):
        material.name = "body" if i == 0 else f"body_{i + 1}"
        names.append(material.name)
    return names


def strip_clips(objects):
    removed = len(bpy.data.actions)
    for obj in objects:
        data = obj.animation_data
        if data is None:
            continue
        data.action = None
        for track in list(data.nla_tracks):
            data.nla_tracks.remove(track)
    for action in list(bpy.data.actions):
        bpy.data.actions.remove(action)
    return removed


def armatures(objects):
    return [o for o in objects if o.type == "ARMATURE"]


def select(objects):
    bpy.ops.object.select_all(action="DESELECT")
    for obj in objects:
        obj.hide_set(False)
        obj.select_set(True)
    bpy.context.view_layer.objects.active = objects[0]


def role(image, objects):
    """What an image feeds: "" for base colour, "normal", or "map"."""
    for material in materials_of(objects):
        if material.node_tree is None:
            continue
        for link in material.node_tree.links:
            node = link.from_node
            if node.type != "TEX_IMAGE" or node.image != image:
                continue
            if link.to_socket.name == "Base Color":
                return ""
            if link.to_node.type == "NORMAL_MAP":
                return "normal"
    return "map"


def save_images(objects, folder, key):
    """Each texture as a PNG next to the .obj, named after the model:
    <key>.png for the base colour, <key>_normal.png, <key>_map.png."""
    taken = set()
    written = []
    for image in fixups.images(objects):
        part = role(image, objects)
        stem = f"{key}_{part}" if part else key
        name, n = f"{stem}.png", 2
        while name in taken:
            name, n = f"{stem}_{n}.png", n + 1
        taken.add(name)
        path = os.path.join(folder, name)
        image.filepath_raw = path
        image.file_format = "PNG"
        image.save()
        written.append(name)
    return written


def export_obj(objects, path):
    rigs = armatures(objects)
    for rig in rigs:
        rig.data.pose_position = "REST"
    bpy.context.view_layer.update()
    select(objects)
    bpy.ops.wm.obj_export(
        filepath=path,
        export_selected_objects=True,
        export_materials=True,
        export_animation=False,
        path_mode="STRIP",
        forward_axis="NEGATIVE_Z",
        up_axis="Y",
    )
    for rig in rigs:
        rig.data.pose_position = "POSE"
    bpy.context.view_layer.update()


def export_glb(objects, path, clips):
    select(objects)
    options = dict(
        filepath=path,
        export_format="GLB",
        use_selection=True,
        export_apply=True,
        export_yup=True,
        export_cameras=False,
        export_lights=False,
        export_skins=True,
        export_animations=clips,
        export_draco_mesh_compression_enable=False,
    )
    try:
        bpy.ops.export_scene.gltf(**options, export_anim_single_armature=False)
    except TypeError:  # an exporter without the option
        bpy.ops.export_scene.gltf(**options)


def missing_maps(mtl):
    """The texture files an .mtl names that are not beside it."""
    if not os.path.isfile(mtl):
        return []
    folder = os.path.dirname(mtl)
    gone = []
    with open(mtl, encoding="utf-8", errors="replace") as fh:
        for line in fh:
            words = line.split()
            if len(words) >= 2 and words[0].lower().startswith(("map_", "bump", "norm")):
                name = words[-1].replace("\\", "/")
                if not os.path.isfile(os.path.join(folder, name)):
                    gone.append(name)
    return gone


def check(path):
    """Opens a written file in an empty scene and measures it."""
    bpy.ops.wm.read_factory_settings(use_empty=True)
    objects = import_model(path)
    numbers = fixups.stats(objects)
    lo, hi = numbers["min"], numbers["max"]
    unloaded = [i.name for i in bpy.data.images if i.size[0] == 0 and i.source == "FILE"]
    found = {
        "file": os.path.basename(path),
        "triangles": numbers["triangles"],
        "objects": numbers["objects"],
        "heightM": round(hi[1] - lo[1], 4) if lo else None,
        "floorM": round(lo[1], 4) if lo else None,
        "centre": [round((lo[0] + hi[0]) / 2, 4), round((lo[2] + hi[2]) / 2, 4)] if lo else None,
        "materials": [m.name for m in bpy.data.materials if m.users],
        "unloaded": unloaded,
        "clips": len(bpy.data.actions),
        "joints": sum(len(a.data.bones) for a in armatures(objects)),
    }
    if path.lower().endswith(".obj"):
        found["missing"] = missing_maps(os.path.splitext(path)[0] + ".mtl")
    return found


def main():
    args = parse_args()
    with open(args.job, encoding="utf-8") as fh:
        job = json.load(fh)
    key, out = job["key"], os.path.abspath(job["out"])
    os.makedirs(out, exist_ok=True)
    bpy.ops.wm.read_factory_settings(use_empty=True)
    objects = import_model(job["input"])
    if not fixups.meshes(objects):
        log(f"Error: {os.path.basename(job['input'])} has no mesh to export")
        sys.exit(1)
    before = fixups.stats(objects)
    area_before = surface(objects)
    objects, steps, scale = fixups.prepare(objects, job.get("plan") or {})
    after = fixups.stats(objects)
    area_after = surface(objects)
    removed = strip_clips(objects) if job.get("stripClips") else 0
    names = rename_materials(objects) if job.get("rename") else []
    written = []
    if job.get("obj"):
        path = os.path.join(out, f"{key}.obj")
        textures = save_images(objects, out, key)
        export_obj(objects, path)
        written += [f"{key}.obj", f"{key}.mtl"] + textures
    if job.get("glb"):
        target = job.get("glbOut") or os.path.join(out, f"{key}.glb")
        os.makedirs(os.path.dirname(os.path.abspath(target)), exist_ok=True)
        export_glb(objects, os.path.abspath(target), not job.get("stripClips"))
        written.append(os.path.basename(target))
    checks = []
    for name in written:
        if name.endswith((".obj", ".glb")):
            folder = os.path.dirname(job["glbOut"]) if name.endswith(".glb") and job.get("glbOut") else out
            checks.append(check(os.path.join(folder, name)))
    loss = 1.0 - area_after / area_before if area_before > 0 else 0.0
    report = {
        "key": key,
        "before": before,
        "after": after,
        "steps": steps,
        "scale": scale,
        "surfaceLoss": round(loss, 4),
        "thin": loss > THIN_LOSS,
        "clipsRemoved": removed,
        "materials": names,
        "files": written,
        "checks": checks,
        "pivotSlack": PIVOT_SLACK,
    }
    with open(args.report, "w", encoding="utf-8") as fh:
        json.dump(report, fh, indent=2)
    log(f"wrote {', '.join(written)}: {before['triangles']} to {after['triangles']} triangles")


main()
