"""Renders a square studio thumbnail of one model.

Maquette runs this headless, always with -b:

    blender -b --factory-startup --python-exit-code 1 [model.blend] \
        --python render_thumb.py -- --model <file> --out <png> [--size 256] \
        [--view front] [--plain]

A .glb, .gltf or .obj is imported into an empty scene. A .blend is opened by
Blender itself (it comes before --python) and its own lights, cameras and
world are left out of the render; nothing is ever saved back.

The look matches the app: a graphite backdrop, a soft studio floor that
fades into it with a contact shadow, a three quarter camera framed to the
model's bounds, a warm key, a warm fill and a cool rim light. Cycles on the
CPU with a few samples and the denoiser, so it runs the same everywhere.

--view front puts the camera straight in front, a little above, as a front
ref is drawn. --plain renders the model alone on a transparent ground, no
floor, so its outline is exact: the Downloads matching compares that with
the refs.
"""

import argparse
import math
import os
import sys
import warnings

import bpy
from mathutils import Vector

# Graphite, as the app's studio token (sRGB hex).
BACKDROP = "#312f2b"
FLOOR_GLOW = "#2e2c28"
KEY_COLOR = (1.0, 0.86, 0.72)
FILL_COLOR = (1.0, 0.93, 0.85)
RIM_COLOR = (0.72, 0.82, 1.0)
AZIMUTH = math.radians(38)
ELEVATION = math.radians(22)
FOCAL_MM = 60
FILL = 0.74


def args():
    argv = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
    parser = argparse.ArgumentParser(prog="render_thumb.py")
    parser.add_argument("--model", required=True)
    parser.add_argument("--out", required=True)
    parser.add_argument("--size", type=int, default=256)
    parser.add_argument("--samples", type=int, default=32)
    parser.add_argument("--view", choices=("three-quarter", "front"), default="three-quarter")
    parser.add_argument("--plain", action="store_true")
    return parser.parse_args(argv)


def linear(hex_color):
    def channel(c):
        c = int(c, 16) / 255
        return c / 12.92 if c <= 0.04045 else ((c + 0.055) / 1.055) ** 2.4

    h = hex_color.lstrip("#")
    return (channel(h[0:2]), channel(h[2:4]), channel(h[4:6]), 1.0)


def use_nodes(block):
    # Blender 5 always has node trees and warns about the old switch; 4.x needs it.
    with warnings.catch_warnings():
        warnings.simplefilter("ignore", DeprecationWarning)
        block.use_nodes = True


def load(model):
    ext = os.path.splitext(model)[1].lower()
    if ext in (".glb", ".gltf"):
        bpy.ops.wm.read_factory_settings(use_empty=True)
        bpy.ops.import_scene.gltf(filepath=model)
    elif ext == ".obj":
        # Its .mtl and textures sit beside it, where the importer looks.
        bpy.ops.wm.read_factory_settings(use_empty=True)
        bpy.ops.wm.obj_import(filepath=model)
    elif ext != ".blend":
        raise SystemExit(f"Cannot render {model}: only .glb, .gltf, .obj and .blend files.")
    scene = bpy.context.scene
    # A .blend keeps its objects; its own lights and cameras stay out.
    for obj in scene.objects:
        if obj.type in {"LIGHT", "CAMERA"}:
            obj.hide_render = True
    return scene


def visible_meshes(scene):
    layer = bpy.context.view_layer
    return [
        o
        for o in scene.objects
        if o.type in {"MESH", "CURVE", "SURFACE", "META", "FONT"}
        and not o.hide_render
        and o.visible_get(view_layer=layer)
    ]


def bounds(objects):
    depsgraph = bpy.context.evaluated_depsgraph_get()
    lo = Vector((math.inf, math.inf, math.inf))
    hi = Vector((-math.inf, -math.inf, -math.inf))
    for obj in objects:
        evaluated = obj.evaluated_get(depsgraph)
        for corner in evaluated.bound_box:
            p = evaluated.matrix_world @ Vector(corner)
            lo = Vector(map(min, lo, p))
            hi = Vector(map(max, hi, p))
    if math.isinf(lo.x):
        raise SystemExit("The model has nothing to render.")
    return lo, hi


def floor(scene, lo, hi, size):
    # Wide enough that the top of the frame still lands on it: no horizon.
    radius = size * 60
    bpy.ops.mesh.primitive_circle_add(
        vertices=64, radius=radius, fill_type="NGON", location=((lo.x + hi.x) / 2, (lo.y + hi.y) / 2, lo.z)
    )
    plane = bpy.context.active_object
    plane.name = "maquette_floor"
    mat = bpy.data.materials.new("maquette_floor")
    use_nodes(mat)
    nodes, links = mat.node_tree.nodes, mat.node_tree.links
    nodes.clear()
    out = nodes.new("ShaderNodeOutputMaterial")
    diffuse = nodes.new("ShaderNodeBsdfDiffuse")
    diffuse.inputs["Color"].default_value = linear(FLOOR_GLOW)
    backdrop = nodes.new("ShaderNodeEmission")
    backdrop.inputs["Color"].default_value = linear(BACKDROP)
    mix = nodes.new("ShaderNodeMixShader")
    # A radial falloff from the model outwards: lit floor near the model,
    # the plain backdrop at the edge, so there is no horizon line.
    coords = nodes.new("ShaderNodeTexCoord")
    mapping = nodes.new("ShaderNodeMapping")
    mapping.inputs["Scale"].default_value = (1 / (size * 2.2),) * 3
    gradient = nodes.new("ShaderNodeTexGradient")
    gradient.gradient_type = "SPHERICAL"
    ramp = nodes.new("ShaderNodeValToRGB")
    # The spherical gradient is 1 under the model and 0 at its edge.
    ramp.color_ramp.elements[0].position = 0.0
    ramp.color_ramp.elements[0].color = (0, 0, 0, 1)
    ramp.color_ramp.interpolation = "EASE"
    ramp.color_ramp.elements[1].position = 0.85
    ramp.color_ramp.elements[1].color = (1, 1, 1, 1)
    links.new(coords.outputs["Object"], mapping.inputs["Vector"])
    links.new(mapping.outputs["Vector"], gradient.inputs["Vector"])
    links.new(gradient.outputs["Fac"], ramp.inputs["Fac"])
    links.new(ramp.outputs["Color"], mix.inputs["Fac"])
    links.new(backdrop.outputs["Emission"], mix.inputs[1])
    links.new(diffuse.outputs["BSDF"], mix.inputs[2])
    links.new(mix.outputs["Shader"], out.inputs["Surface"])
    plane.data.materials.append(mat)
    # The floor catches shadows but never reflects light back onto the model.
    plane.visible_glossy = False


def world(scene):
    w = bpy.data.worlds.new("maquette_studio")
    use_nodes(w)
    bg = w.node_tree.nodes.get("Background")
    bg.inputs["Color"].default_value = linear(BACKDROP)
    bg.inputs["Strength"].default_value = 1.0
    # The backdrop is what the camera sees; it lights the model only a little.
    light_path = w.node_tree.nodes.new("ShaderNodeLightPath")
    dim = w.node_tree.nodes.new("ShaderNodeBackground")
    dim.inputs["Color"].default_value = (0.32, 0.31, 0.30, 1)
    dim.inputs["Strength"].default_value = 0.35
    mix = w.node_tree.nodes.new("ShaderNodeMixShader")
    out = w.node_tree.nodes.get("World Output")
    links = w.node_tree.links
    links.new(light_path.outputs["Is Camera Ray"], mix.inputs["Fac"])
    links.new(dim.outputs["Background"], mix.inputs[1])
    links.new(bg.outputs["Background"], mix.inputs[2])
    links.new(mix.outputs["Shader"], out.inputs["Surface"])
    scene.world = w


# The camera's turn and height, set once from --view.
VIEW = {"azimuth": AZIMUTH, "elevation": ELEVATION}
FRONT_ELEVATION = math.radians(6)


def view_dir():
    # glTF fronts face -Y in Blender, so the camera sits in front and to the right.
    azimuth, elevation = VIEW["azimuth"], VIEW["elevation"]
    return Vector(
        (
            math.sin(azimuth) * math.cos(elevation),
            -math.cos(azimuth) * math.cos(elevation),
            math.sin(elevation),
        )
    ).normalized()


def camera(scene, lo, hi):
    data = bpy.data.cameras.new("maquette_thumb")
    data.lens = FOCAL_MM
    data.sensor_fit = "HORIZONTAL"
    cam = bpy.data.objects.new("maquette_thumb", data)
    scene.collection.objects.link(cam)
    center = (lo + hi) / 2
    v = view_dir()
    right = v.cross(Vector((0, 0, 1))).normalized()
    up = right.cross(v).normalized()
    tan = (data.sensor_width / 2) / data.lens
    corners = [Vector((x, y, z)) for x in (lo.x, hi.x) for y in (lo.y, hi.y) for z in (lo.z, hi.z)]
    # Each corner needs the camera far enough back to fit inside the frame.
    distance = 0.0
    for p in corners:
        q = p - center
        reach = max(abs(q.dot(right)), abs(q.dot(up))) / (tan * FILL)
        distance = max(distance, reach + q.dot(v))
    distance = max(distance, 0.01)
    cam.location = center + v * distance
    cam.rotation_euler = (-v).to_track_quat("-Z", "Y").to_euler()
    data.clip_start = distance / 100
    data.clip_end = distance * 100
    scene.camera = cam
    return center, distance


def light(scene, name, color, power, direction, size):
    data = bpy.data.lights.new(name, "AREA")
    data.color = color
    data.energy = power
    data.size = size
    obj = bpy.data.objects.new(name, data)
    scene.collection.objects.link(obj)
    return obj, direction


def lights(scene, center, extent):
    v = view_dir()
    right = v.cross(Vector((0, 0, 1))).normalized()
    reach = extent * 2.2
    power = 120 * extent * extent
    rigs = [
        ("maquette_key", KEY_COLOR, power, (v - right * 0.9 + Vector((0, 0, 0.9))), extent * 1.2),
        ("maquette_fill", FILL_COLOR, power * 0.35, (v + right * 1.1 + Vector((0, 0, 0.2))), extent * 2.0),
        ("maquette_rim", RIM_COLOR, power * 0.9, (-v + Vector((0, 0, 0.8))), extent * 0.8),
    ]
    for name, color, energy, direction, size in rigs:
        obj, d = light(scene, name, color, energy, direction, size)
        d = d.normalized()
        obj.location = center + d * reach
        obj.rotation_euler = (-d).to_track_quat("-Z", "Y").to_euler()


def render(scene, out, size, samples, plain=False):
    scene.render.engine = "CYCLES"
    scene.cycles.device = "CPU"
    scene.cycles.samples = samples
    scene.cycles.use_denoising = True
    scene.cycles.max_bounces = 4
    scene.render.resolution_x = size
    scene.render.resolution_y = size
    scene.render.resolution_percentage = 100
    scene.render.film_transparent = plain
    scene.view_settings.view_transform = "Standard"
    scene.view_settings.look = "None"
    scene.render.image_settings.file_format = "PNG"
    scene.render.image_settings.color_mode = "RGBA" if plain else "RGB"
    scene.render.image_settings.color_depth = "8"
    os.makedirs(os.path.dirname(os.path.abspath(out)), exist_ok=True)
    # Written next to the target and renamed, so a half written PNG is never cached.
    tmp = out + ".part.png"
    scene.render.filepath = tmp
    bpy.ops.render.render(write_still=True)
    os.replace(tmp, out)


def main():
    a = args()
    scene = load(a.model)
    meshes = visible_meshes(scene)
    lo, hi = bounds(meshes)
    extent = max((hi - lo).length, 1e-3)
    if a.view == "front":
        VIEW.update(azimuth=0.0, elevation=FRONT_ELEVATION)
    world(scene)
    if not a.plain:
        floor(scene, lo, hi, extent)
    center, _ = camera(scene, lo, hi)
    lights(scene, center, extent)
    # Blender reads a relative render path from the blend's folder, so pin it.
    render(scene, os.path.abspath(a.out), a.size, a.samples, a.plain)
    print(f"thumbnail {a.out} ({a.size} px)")


if __name__ == "__main__":
    main()
