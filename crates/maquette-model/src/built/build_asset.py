"""Build one built asset: run its script in an empty scene and export a GLB.

Maquette runs this headless, never with a window:

    blender -b --factory-startup --python-exit-code 1 --python build_asset.py -- \
        --script scripts/laptop.py --output built.glb --report report.json [--root laptop]

The script is imported as a module named after its file (laptop.py is the
module "laptop", so its `if __name__ == "__main__":` block does not run),
with its folder first on sys.path, so helpers next to it import as usual.

- When the module has a `build()` function it is called with no arguments.
  An object it returns is the root: that object and everything parented
  under it is exported. A script without `build()` builds the scene while
  it is imported.
- `--root NAME` names the root object instead, so one script that builds
  several pieces in one scene can feed several assets.
- Without a root, every object of the scene is exported.

Cameras and lights are never exported. Object names, the parent tree,
origins and material names are kept as the script made them, so a hinge
pivot such as "lid" or a material such as "screen" reaches the GLB. The
report lists the exported objects and materials and the triangle count.
"""

import argparse
import importlib.util
import json
import os
import sys
import traceback

import bpy

EXPORTED_TYPES = {"MESH", "EMPTY", "ARMATURE", "CURVE", "SURFACE", "META", "FONT"}


def log(message):
    print(f"[built] {message}", flush=True)


def parse_args():
    argv = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
    parser = argparse.ArgumentParser(prog="build_asset.py")
    parser.add_argument("--script", required=True, help="the asset's script")
    parser.add_argument("--output", required=True, help="where the GLB goes")
    parser.add_argument("--report", required=True, help="where the report goes")
    parser.add_argument("--root", default=None, help="the root object to export")
    return parser.parse_args(argv)


def load(script):
    """Imports the script as a module named after its file."""
    folder = os.path.dirname(script)
    name = os.path.splitext(os.path.basename(script))[0]
    sys.path.insert(0, folder)
    spec = importlib.util.spec_from_file_location(name, script)
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


def subtree(root):
    return [root] + list(root.children_recursive)


def pick_objects(module, root_name):
    root = None
    build = getattr(module, "build", None)
    if callable(build):
        made = build()
        if isinstance(made, bpy.types.Object):
            root = made
    if root_name:
        root = bpy.data.objects.get(root_name)
        if root is None:
            raise SystemExit(f"the script made no object called {root_name!r}")
    bpy.context.view_layer.update()
    objects = subtree(root) if root is not None else list(bpy.context.scene.objects)
    objects = [o for o in objects if o.type in EXPORTED_TYPES]
    if not objects:
        raise SystemExit("the script made nothing to export (no mesh, empty or armature)")
    return root, objects


def export(path, objects):
    bpy.ops.object.select_all(action="DESELECT")
    for obj in objects:
        obj.hide_set(False)
        obj.select_set(True)
    bpy.context.view_layer.objects.active = objects[0]
    bpy.ops.export_scene.gltf(
        filepath=os.path.abspath(path),
        export_format="GLB",
        use_selection=True,
        export_apply=True,
        export_extras=True,
        export_cameras=False,
        export_lights=False,
        export_yup=True,
    )


def triangles(objects):
    total = 0
    for obj in objects:
        if obj.type == "MESH":
            total += sum(len(p.vertices) - 2 for p in obj.data.polygons)
    return total


def main():
    args = parse_args()
    script = os.path.abspath(args.script)
    bpy.ops.wm.read_factory_settings(use_empty=True)
    log(f"building {os.path.basename(script)}")
    try:
        module = load(script)
        root, objects = pick_objects(module, args.root)
    except SystemExit as stop:
        log(f"Error: {stop}")
        sys.exit(1)
    except Exception as err:
        traceback.print_exc()
        log(f"Error: {type(err).__name__}: {err}")
        sys.exit(1)
    os.makedirs(os.path.dirname(os.path.abspath(args.output)) or ".", exist_ok=True)
    export(args.output, objects)
    materials = sorted(
        {s.material.name for o in objects for s in getattr(o, "material_slots", []) if s.material}
    )
    report = {
        "root": root.name if root is not None else None,
        "objects": [o.name for o in objects],
        "materials": materials,
        "triangles": triangles(objects),
    }
    with open(args.report, "w", encoding="utf-8") as fh:
        json.dump(report, fh, indent=2)
    log(f"wrote {args.output}: {len(objects)} objects, {report['triangles']} triangles")


main()
