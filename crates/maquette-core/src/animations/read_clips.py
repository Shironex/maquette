"""Read animation clips out of FBX or glTF files and keep only the skeleton.

Maquette runs this headless, never with a window:

    blender -b --factory-startup --python-exit-code 1 --python read_clips.py -- \
        --jobs jobs.json --out clips --report report.json

jobs.json lists the files to read, each with an id, its path and, for a
model that already holds clips, the clips to keep and the rate they were
sampled at:

    [{"id": "0", "file": "in/0/0.fbx", "name": "Standing Dodge Left.fbx", "clips": null, "fps": null}]

`name` is the file's own name, which a lone take is named after.

Each file is read into an empty scene on its own. The mesh and its skin are
dropped; the armature with the most bones is kept, and every action that
moves it becomes one clip, written as `<out>/<id>/<n>.glb`: the bones and
the keys, nothing else. An action whose keys never change (a single pose)
is reported with `hasKeys` false and not written.

The report has, per file: the bones with their parents and rest heads and
tails (metres, glTF axes, Y up), the hips height, whether it had a skin,
and per clip its frames, rate, length, a hash of its keys (so the same clip
twice is found), how far and which way the root travels, whether the last
frame matches the first, and the written file's size.
"""

import argparse
import hashlib
import json
import math
import os
import struct
import sys
import traceback

import bpy

# Bones that carry root motion when the rig names one, lowest first.
HIPS_NAMES = ("hips", "pelvis", "hip", "root")
# How far the root must travel to count as root motion, metres.
ROOT_MIN = 0.05
LOOP_TOLERANCE = 1e-3


def log(message):
    print(f"[clips] {message}", flush=True)


def parse_args():
    argv = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
    parser = argparse.ArgumentParser(prog="read_clips.py")
    parser.add_argument("--jobs", required=True, help="the files to read")
    parser.add_argument("--out", required=True, help="where the clips go")
    parser.add_argument("--report", required=True, help="where the report goes")
    return parser.parse_args(argv)


def gltf_axes(v):
    """Blender (Z up) to glTF (Y up), rounded for the report."""
    return [round(v.x, 5), round(v.z, 5), round(-v.y, 5)]


def fcurves_of(action):
    """Every F-curve of an action, layered (Blender 4.4 and later) or not."""
    layers = getattr(action, "layers", None)
    if layers:
        return [fc for layer in layers for strip in layer.strips for bag in strip.channelbags for fc in bag.fcurves]
    return list(getattr(action, "fcurves", []))


def keys_hash(curves):
    digest = hashlib.sha256()
    for fc in sorted(curves, key=lambda c: (c.data_path, c.array_index)):
        digest.update(f"{fc.data_path}[{fc.array_index}]".encode())
        for point in fc.keyframe_points:
            digest.update(f"{point.co[0]:.3f}:{point.co[1]:.5f};".encode())
    return digest.hexdigest()


def moves(curves):
    """True when any channel's value changes over its keys."""
    for fc in curves:
        values = [p.co[1] for p in fc.keyframe_points]
        if len(values) > 1 and max(values) - min(values) > 1e-6:
            return True
    return False


def read_file(path, fps):
    bpy.ops.wm.read_factory_settings(use_empty=True)
    scene = bpy.context.scene
    if fps:
        scene.render.fps = int(round(fps))
        scene.render.fps_base = 1.0
    ext = os.path.splitext(path)[1].lower()
    if ext == ".fbx":
        bpy.ops.import_scene.fbx(filepath=path, use_anim=True, ignore_leaf_bones=False)
    elif ext in (".glb", ".gltf"):
        bpy.ops.import_scene.gltf(filepath=path)
    else:
        raise ValueError(f"{os.path.basename(path)} is not an FBX or glTF file")
    return scene


def pick_armature():
    arms = [o for o in bpy.data.objects if o.type == "ARMATURE"]
    if not arms:
        return None
    return max(arms, key=lambda o: len(o.data.bones))


def hips_of(arm):
    bones = list(arm.data.bones)
    for wanted in HIPS_NAMES:
        for b in bones:
            if b.name.rsplit(":", 1)[-1].lower() == wanted:
                return b
    roots = [b for b in bones if b.parent is None]
    return max(roots, key=lambda b: len(b.children_recursive)) if roots else None


def bones_of(arm):
    world = arm.matrix_world
    return [
        {
            "name": b.name,
            "parent": b.parent.name if b.parent else None,
            "head": gltf_axes(world @ b.head_local),
            "tail": gltf_axes(world @ b.tail_local),
        }
        for b in arm.data.bones
    ]


def drop_meshes():
    meshes = [o for o in bpy.data.objects if o.type == "MESH"]
    bpy.data.batch_remove(meshes)
    return len(meshes)


def assign(arm, action):
    data = arm.animation_data or arm.animation_data_create()
    data.action = action
    slots = getattr(action, "slots", None)
    if slots and getattr(data, "action_slot", None) is None:
        data.action_slot = slots[0]


def pose_at(scene, arm, frame, hips_name):
    scene.frame_set(frame)
    graph = bpy.context.evaluated_depsgraph_get()
    ev = arm.evaluated_get(graph)
    rots = {pb.name: pb.matrix_basis.to_quaternion() for pb in ev.pose.bones}
    hips = ev.matrix_world @ ev.pose.bones[hips_name].head if hips_name else None
    return rots, hips


def same_pose(a, b, skip):
    for name, q in a.items():
        if name == skip:
            continue
        if abs(q.dot(b[name])) < 1.0 - LOOP_TOLERANCE:
            return False
    return True


def direction(dx, dy):
    """A character faces -Y in Blender: its left is +X."""
    if abs(dx) >= abs(dy):
        return "left" if dx > 0 else "right"
    return "forward" if dy < 0 else "back"


def measure(scene, arm, action, hips):
    start, end = (int(round(f)) for f in action.frame_range)
    hips_name = hips.name if hips else None
    first, first_hips = pose_at(scene, arm, start, hips_name)
    last, last_hips = pose_at(scene, arm, end, hips_name)
    root = None
    loops = same_pose(first, last, hips_name)
    if first_hips is not None:
        dx, dy = last_hips.x - first_hips.x, last_hips.y - first_hips.y
        travel = math.hypot(dx, dy)
        if travel >= ROOT_MIN:
            root = {"metres": round(travel, 3), "direction": direction(dx, dy)}
        if abs(last_hips.z - first_hips.z) >= LOOP_TOLERANCE:
            loops = False
    fps = scene.render.fps / scene.render.fps_base
    frames = max(end - start, 1)
    return {
        "start": start,
        "end": end,
        "frames": frames,
        "fps": fps,
        "length": round(frames / fps, 4),
        "rootMotion": root,
        "loops": loops,
    }


def name_animation(path, name):
    """Names the GLB's one animation: the exporter calls it after its track."""
    with open(path, "rb") as fh:
        data = fh.read()
    length = struct.unpack_from("<I", data, 12)[0]
    doc = json.loads(data[20 : 20 + length])
    for anim in doc.get("animations", []):
        anim["name"] = name
    text = json.dumps(doc, separators=(",", ":")).encode()
    text += b" " * (-len(text) % 4)
    rest = data[20 + length :]
    head = struct.pack("<III", 0x46546C67, 2, 12 + 8 + len(text) + len(rest))
    with open(path, "wb") as fh:
        fh.write(head + struct.pack("<II", len(text), 0x4E4F534A) + text + rest)


def export(arm, action, name, path):
    action.name = name
    assign(arm, action)
    bpy.ops.object.select_all(action="DESELECT")
    arm.hide_set(False)
    arm.select_set(True)
    bpy.context.view_layer.objects.active = arm
    os.makedirs(os.path.dirname(path), exist_ok=True)
    bpy.ops.export_scene.gltf(
        filepath=path,
        export_format="GLB",
        use_selection=True,
        export_animations=True,
        export_animation_mode="ACTIVE_ACTIONS",
        export_force_sampling=True,
        export_def_bones=False,
        export_cameras=False,
        export_lights=False,
        export_yup=True,
    )
    name_animation(path, name)
    return os.path.getsize(path)


def clip_name(action, arm, path, count):
    """The take's name; an FBX's lone take is named after its file, since
    Mixamo calls every take mixamo.com."""
    name = action.name.split("|")[-1]
    suffix = f"_{arm.name}"
    if name.endswith(suffix):
        name = name[: -len(suffix)]
    fbx = path.lower().endswith(".fbx")
    if (fbx and count == 1) or name == "mixamo.com":
        name = os.path.splitext(os.path.basename(path))[0]
    return name


def read_job(job, out_dir):
    path = os.path.abspath(job["file"])
    scene = read_file(path, job.get("fps"))
    skinned = any(o.type == "MESH" and o.find_armature() for o in bpy.data.objects)
    arm = pick_armature()
    result = {"id": job["id"], "file": job["file"], "error": None, "skinned": skinned, "clips": []}
    if arm is None:
        result["error"] = "it has no armature"
        result["bones"] = []
        result["hipsHeight"] = 0.0
        return result
    result["meshes"] = drop_meshes()
    hips = hips_of(arm)
    result["bones"] = bones_of(arm)
    result["hipsHeight"] = round((arm.matrix_world @ hips.head_local).z, 4) if hips else 0.0
    wanted = job.get("clips")
    actions = [a for a in bpy.data.actions if fcurves_of(a)]
    for n, action in enumerate(actions):
        curves = fcurves_of(action)
        name = clip_name(action, arm, job.get("name") or path, len(actions))
        if wanted is not None and name not in wanted:
            continue
        clip = {"action": action.name, "name": name, "hash": keys_hash(curves), "hasKeys": moves(curves)}
        assign(arm, action)
        clip.update(measure(scene, arm, action, hips))
        if clip["hasKeys"]:
            out = os.path.join(out_dir, job["id"], f"{n}.glb")
            clip["bytes"] = export(arm, action, name, out)
            clip["out"] = os.path.relpath(out, out_dir).replace(os.sep, "/")
        result["clips"].append(clip)
    log(f"{os.path.basename(path)}: {len(result['bones'])} bones, {len(result['clips'])} clips")
    return result


def main():
    args = parse_args()
    with open(args.jobs, encoding="utf-8") as fh:
        jobs = json.load(fh)
    out_dir = os.path.abspath(args.out)
    files = []
    for job in jobs:
        try:
            files.append(read_job(job, out_dir))
        except Exception as err:
            traceback.print_exc()
            log(f"Error: {job['file']}: {err}")
            files.append({"id": job["id"], "file": job["file"], "error": str(err), "clips": [], "bones": []})
    with open(args.report, "w", encoding="utf-8") as fh:
        json.dump({"files": files}, fh, indent=2)
    log(f"read {len(files)} files")


main()
