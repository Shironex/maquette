"""Renders one model in Maquette's render styles.

Maquette runs this headless, with render_thumb.py beside it:

    blender -b --factory-startup --python-exit-code 1 [model.blend] \
        --python render_styles.py -- --spec <spec.json>

The spec is JSON: the model, what it is called, its real height, and a list
of jobs. Every job renders one style from one camera to one or more files,
and all of them share the one load of the model:

    {"model": "...", "name": "Street lamp", "heightM": 1.84, "version": 2,
     "report": "report.json",
     "jobs": [{"style": "blueprint", "out": "blueprint.png", "size": 2048,
               "azimuth": 38, "elevation": 14, "weight": 1.1, ...}]}

A job with "frames" renders that many frames of one full turn into "out",
a folder, as f0001.png and so on, and notes the moving area. A job with
"sheet" lays finished pictures out in one PNG with a name under each, and
needs no model. See job_defaults for every field.

Blueprint, toon and clay use Workbench, Blender's flat viewport engine, with
Freestyle, its line renderer, for the lines. Shaded is the thumbnail's studio
look in Cycles. render_thumb.py holds the loading, the bounds and the studio.
"""

import json
import math
import os
import sys
import time

import bpy
import numpy
from mathutils import Vector

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import render_thumb as thumb  # noqa: E402

PAPER_TOON = "#f5f1ea"
PAPER_CLAY = "#e7e5df"
INK = "#1b1a18"
OVERLAY = "maquette_overlay"
BACKDROP = "#5e5b56"


def job_defaults():
    return {
        "style": "clay",
        "out": "",
        "size": 1024,
        "azimuth": 38.0,
        "elevation": 14.0,
        # Lines, at 1024 px; a bigger picture gets thicker ones.
        "weight": 1.6,
        "crease": 30.0,
        "lineColor": "#b3d9ff",
        "paperColor": None,
        "clayColor": "#a89e92",
        "hidden": False,
        "marks": True,
        "titleBlock": True,
        "transparent": False,
        "samples": 24,
        # A turn: frames and whether the ground stays out of it.
        "frames": 0,
        "still": True,
        "webp": 0,
        # Pixels of margin around what moves; 0 keeps the whole frame.
        "crop": 0,
        # Notes the moving area with this margin even when the frames stay whole.
        "boxMargin": 0,
        # A sheet: {"tiles": [{"file", "name"}], "columns", "tile", "names", "paper"}.
        "sheet": None,
    }


def spec():
    argv = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
    if len(argv) != 2 or argv[0] != "--spec":
        raise SystemExit("render_styles.py needs --spec <file>")
    with open(argv[1], encoding="utf-8") as f:
        return json.load(f)


def srgb(hex_color):
    return thumb.linear(hex_color)[:3]


# The model -----------------------------------------------------------------


def solid(name, hex_color):
    mat = bpy.data.materials.get(name) or bpy.data.materials.new(name)
    mat.diffuse_color = (*srgb(hex_color), 1.0)
    return mat


def texture_node(material):
    """The picture feeding a material's colour, followed through mix nodes."""
    if not material or not material.node_tree:
        return None
    nodes = material.node_tree.nodes
    for node in nodes:
        if node.type == "BSDF_PRINCIPLED":
            stack = list(node.inputs["Base Color"].links)
            while stack:
                source = stack.pop().from_node
                if source.type == "TEX_IMAGE" and source.image:
                    return source
                for socket in source.inputs:
                    stack.extend(socket.links)
    return next((n for n in nodes if n.type == "TEX_IMAGE" and n.image), None)


def base_color(material):
    """A material's plain colour, so Workbench shows it."""
    if material and material.node_tree:
        for node in material.node_tree.nodes:
            if node.type == "BSDF_PRINCIPLED":
                return tuple(node.inputs["Base Color"].default_value)
    return tuple(material.diffuse_color) if material else (0.8, 0.8, 0.8, 1.0)


class Model:
    """The loaded model: its objects, their own materials and the turn."""

    def __init__(self, scene):
        self.scene = scene
        self.meshes = thumb.visible_meshes(scene)
        self.lo, self.hi = thumb.bounds(self.meshes)
        self.textured = False
        self.tinted = False
        self.materials = {}
        colors = set()
        for obj in self.meshes:
            slots = [slot.material for slot in obj.material_slots]
            self.materials[obj.name] = slots
            for mat in slots:
                node = texture_node(mat)
                if node:
                    self.textured = True
                    mat.node_tree.nodes.active = node
                elif mat:
                    color = base_color(mat)
                    mat.diffuse_color = color
                    colors.add(tuple(round(c, 2) for c in color[:3]))
        self.tinted = len(colors) > 1 or any(max(c) - min(c) > 0.04 for c in colors)
        self.triangles = 0
        self.edges = 0
        depsgraph = bpy.context.evaluated_depsgraph_get()
        for obj in self.meshes:
            mesh = obj.evaluated_get(depsgraph).data
            if hasattr(mesh, "loop_triangles"):
                mesh.calc_loop_triangles()
                self.triangles += len(mesh.loop_triangles)
                self.edges += len(mesh.edges)
        self.points = self.world_points(depsgraph)
        self.pivot = bpy.data.objects.new("maquette_pivot", None)
        scene.collection.objects.link(self.pivot)
        self.pivot.location = ((self.lo.x + self.hi.x) / 2, (self.lo.y + self.hi.y) / 2, 0)
        for obj in scene.objects:
            if obj.parent is None and obj is not self.pivot and obj.type not in {"LIGHT", "CAMERA"}:
                world = obj.matrix_world.copy()
                obj.parent = self.pivot
                obj.matrix_parent_inverse = self.pivot.matrix_world.inverted()
                obj.matrix_world = world

    def world_points(self, depsgraph, most=6000):
        """Up to `most` of the model's vertices in the world, for framing."""
        points = []
        for obj in self.meshes:
            data = obj.evaluated_get(depsgraph).data
            if not hasattr(data, "vertices") or not len(data.vertices):
                continue
            step = max(1, len(data.vertices) // most)
            matrix = obj.evaluated_get(depsgraph).matrix_world
            points.extend(matrix @ data.vertices[i].co for i in range(0, len(data.vertices), step))
        if not points:
            lo, hi = self.lo, self.hi
            points = [Vector((x, y, z)) for x in (lo.x, hi.x) for y in (lo.y, hi.y) for z in (lo.z, hi.z)]
        return points

    def turn(self, degrees):
        self.pivot.rotation_euler = (0, 0, math.radians(degrees))
        bpy.context.view_layer.update()

    def own_materials(self):
        for obj in self.meshes:
            for slot, mat in zip(obj.material_slots, self.materials[obj.name]):
                slot.link = "OBJECT"
                slot.material = mat

    def one_material(self, material):
        for obj in self.meshes:
            if not obj.material_slots and hasattr(obj.data, "materials"):
                obj.data.materials.append(None)
            for slot in obj.material_slots:
                slot.link = "OBJECT"
                slot.material = material

    def corners(self, turning):
        if not turning:
            return self.points
        centre = self.pivot.location
        radius = max(math.hypot(p.x - centre.x, p.y - centre.y) for p in self.points)
        rim = []
        for z in (self.lo.z, self.hi.z):
            for step in range(24):
                a = step * math.tau / 24
                rim.append(Vector((centre.x + radius * math.cos(a), centre.y + radius * math.sin(a), z)))
        return rim


# The camera ----------------------------------------------------------------


def place_camera(scene, points, azimuth, elevation, fill):
    data = bpy.data.cameras.get("maquette_cam") or bpy.data.cameras.new("maquette_cam")
    data.lens = thumb.FOCAL_MM
    data.sensor_fit = "HORIZONTAL"
    cam = bpy.data.objects.get("maquette_cam")
    if cam is None:
        cam = bpy.data.objects.new("maquette_cam", data)
        scene.collection.objects.link(cam)
    az, el = math.radians(azimuth), math.radians(elevation)
    v = Vector((math.sin(az) * math.cos(el), -math.cos(az) * math.cos(el), math.sin(el))).normalized()
    right = v.cross(Vector((0, 0, 1))).normalized()
    up = right.cross(v).normalized()
    tan = (data.sensor_width / 2) / data.lens
    # Centre on what the camera sees, so a tall thin model sits in the middle.
    us = [p.dot(right) for p in points]
    vs = [p.dot(up) for p in points]
    ws = [p.dot(v) for p in points]
    centre = right * ((min(us) + max(us)) / 2) + up * ((min(vs) + max(vs)) / 2) + v * ((min(ws) + max(ws)) / 2)
    distance = 0.01
    for p in points:
        q = p - centre
        reach = max(abs(q.dot(right)), abs(q.dot(up))) / (tan * fill)
        distance = max(distance, reach + q.dot(v))
    cam.location = centre + v * distance
    cam.rotation_euler = (-v).to_track_quat("-Z", "Y").to_euler()
    data.clip_start = distance / 100
    data.clip_end = distance * 100
    scene.camera = cam
    bpy.context.view_layer.update()
    return cam, centre, distance


# The looks -----------------------------------------------------------------


def workbench(scene, job, light, color_type, paper):
    scene.render.engine = "BLENDER_WORKBENCH"
    shading = scene.display.shading
    shading.light = light
    shading.color_type = color_type
    shading.background_type = "VIEWPORT"
    shading.background_color = srgb(paper)
    shading.show_cavity = False
    shading.show_shadows = False
    shading.show_specular_highlight = False
    shading.show_object_outline = False
    shading.show_backface_culling = False
    shading.show_xray = False
    scene.display.render_aa = "32"
    scene.world = None


def lines(scene, job, color, min_px, crease=None, hidden_color=None):
    layer = bpy.context.view_layer
    scene.render.use_freestyle = True
    scene.render.line_thickness_mode = "ABSOLUTE"
    scene.render.line_thickness = 1.0
    layer.use_freestyle = True
    settings = layer.freestyle_settings
    settings.mode = "EDITOR"
    settings.crease_angle = math.radians(crease if crease is not None else job["crease"])
    settings.use_culling = True
    settings.use_smoothness = False
    settings.as_render_pass = False
    for lineset in list(settings.linesets):
        settings.linesets.remove(lineset)
    scale = job["size"] / 1024
    width = max(job["weight"] * scale, 0.5)

    def add(name, visibility, tone, thick, dashed):
        lineset = settings.linesets.new(name)
        lineset.select_silhouette = True
        lineset.select_border = True
        lineset.select_crease = True
        lineset.select_contour = True
        lineset.select_external_contour = True
        lineset.select_material_boundary = True
        lineset.select_by_visibility = True
        lineset.visibility = visibility
        lineset.select_by_edge_types = True
        lineset.select_by_collection = bool(bpy.data.collections.get(OVERLAY))
        if lineset.select_by_collection:
            lineset.collection = bpy.data.collections[OVERLAY]
            lineset.collection_negation = "EXCLUSIVE"
        style = lineset.linestyle
        style.color = tone
        style.thickness = thick
        style.caps = "ROUND"
        style.use_chaining = True
        style.chaining = "PLAIN"
        style.use_same_object = True
        # Thin, dense models throw off specks; a stroke must be this long.
        style.use_length_min = True
        style.length_min = min_px * scale
        style.use_dashed_line = dashed
        if dashed:
            style.dash1, style.gap1 = int(6 * scale) or 1, int(5 * scale) or 1

    add("maquette_lines", "VISIBLE", color, width, False)
    if hidden_color:
        add("maquette_hidden", "HIDDEN", hidden_color, width * 0.6, True)


def no_lines(scene):
    scene.render.use_freestyle = False
    bpy.context.view_layer.use_freestyle = False


def use_clear_view(scene):
    scene.view_settings.view_transform = "Standard"
    scene.view_settings.look = "None"


def look_blueprint(scene, model, job):
    paper = job["paperColor"] or "#0f2a55"
    workbench(scene, job, "FLAT", "MATERIAL", paper)
    model.one_material(solid("maquette_paper", paper))
    line = srgb(job["lineColor"])
    fade = tuple(a * 0.55 + b * 0.45 for a, b in zip(line, srgb(paper)))
    lines(scene, job, line, 5, hidden_color=fade if job["hidden"] else None)
    return paper


def look_toon(scene, model, job):
    paper = job["paperColor"] or PAPER_TOON
    workbench(scene, job, "FLAT", "TEXTURE" if model.textured else "MATERIAL", paper)
    model.own_materials()
    shading = scene.display.shading
    shading.show_shadows = True
    shading.shadow_intensity = 0.32
    scene.display.light_direction = (-0.45, -0.55, 0.7)
    scene.display.shadow_shift = 0.08
    scene.display.shadow_focus = 0.0
    lines(scene, job, srgb(INK), 14, crease=max(job["crease"], 60.0))
    return paper


def look_clay(scene, model, job):
    paper = job["paperColor"] or PAPER_CLAY
    workbench(scene, job, "STUDIO", "SINGLE", paper)
    shading = scene.display.shading
    shading.single_color = srgb(job["clayColor"])
    shading.studio_light = "paint.sl"
    shading.show_cavity = True
    shading.cavity_type = "BOTH"
    shading.cavity_ridge_factor = 0.6
    shading.cavity_valley_factor = 1.2
    shading.curvature_ridge_factor = 0.6
    shading.curvature_valley_factor = 0.9
    shading.show_shadows = True
    shading.shadow_intensity = 0.18
    scene.display.light_direction = (-0.4, -0.5, 0.75)
    scene.display.shadow_shift = 0.08
    scene.display.shadow_focus = 0.4
    model.own_materials()
    no_lines(scene)
    return paper


def look_shaded(scene, model, job, centre):
    model.own_materials()
    no_lines(scene)
    extent = max((model.hi - model.lo).length, 1e-3)
    paper = job["paperColor"] or BACKDROP
    thumb.BACKDROP = paper
    thumb.FLOOR_GLOW = paper
    thumb.world(scene)
    keep_floor = not job["transparent"] and not (job["frames"] and job["still"])
    if keep_floor and not bpy.data.objects.get("maquette_floor"):
        thumb.floor(scene, model.lo, model.hi, extent)
    if bpy.data.objects.get("maquette_floor"):
        bpy.data.objects["maquette_floor"].hide_render = not keep_floor
    thumb.VIEW.update(azimuth=math.radians(job["azimuth"]), elevation=math.radians(job["elevation"]))
    if not bpy.data.objects.get("maquette_key"):
        thumb.lights(scene, centre, extent)
    scene.render.engine = "CYCLES"
    scene.cycles.device = "CPU"
    scene.cycles.samples = job["samples"]
    scene.cycles.use_denoising = True
    scene.cycles.max_bounces = 4
    return paper


def apply_look(scene, model, job, centre):
    style = job["style"]
    if style == "blueprint":
        return look_blueprint(scene, model, job)
    if style == "toon":
        return look_toon(scene, model, job)
    if style == "shaded":
        return look_shaded(scene, model, job, centre)
    return look_clay(scene, model, job)


# The drawing's furniture: grid, marks and title block -----------------------


def overlay_collection(scene):
    found = bpy.data.collections.get(OVERLAY)
    if found:
        for obj in list(found.objects):
            bpy.data.objects.remove(obj)
        return found
    found = bpy.data.collections.new(OVERLAY)
    scene.collection.children.link(found)
    return found


class Sheet:
    """Draws on a plane square to the camera, in frame units: -1 to 1 across."""

    def __init__(self, cam, collection, depth, material):
        self.cam, self.collection, self.depth, self.material = cam, collection, depth, material
        self.half = depth * (cam.data.sensor_width / 2) / cam.data.lens

    def point(self, x, y):
        return (x * self.half, y * self.half, -self.depth)

    def add(self, name, data):
        obj = bpy.data.objects.new(name, data)
        self.collection.objects.link(obj)
        obj.parent = self.cam
        return obj

    def bars(self, name, bars):
        """Thin quads, each (x0, y0, x1, y1, thickness)."""
        verts, faces = [], []
        for x0, y0, x1, y1, t in bars:
            dx, dy = x1 - x0, y1 - y0
            n = max(math.hypot(dx, dy), 1e-6)
            nx, ny = -dy / n * t / 2, dx / n * t / 2
            i = len(verts)
            verts += [self.point(x0 + nx, y0 + ny), self.point(x1 + nx, y1 + ny), self.point(x1 - nx, y1 - ny), self.point(x0 - nx, y0 - ny)]
            faces.append((i, i + 3, i + 2, i + 1))
        mesh = bpy.data.meshes.new(name)
        mesh.from_pydata(verts, [], faces)
        mesh.materials.append(self.material)
        return self.add(name, mesh)

    def text(self, name, body, x, y, height, align="LEFT", turn=0.0, spacing=1.0):
        curve = bpy.data.curves.new(name, "FONT")
        curve.body = body
        curve.size = height * self.half
        curve.align_x = align
        curve.space_character = spacing
        curve.materials.append(self.material)
        obj = self.add(name, curve)
        obj.location = self.point(x, y)
        obj.rotation_euler = (0, 0, turn)
        return obj


def screen_box(scene, cam, points):
    """The model's box on screen, in frame units."""
    from bpy_extras.object_utils import world_to_camera_view

    xs, ys = [], []
    for p in points:
        c = world_to_camera_view(scene, cam, p)
        xs.append(c.x * 2 - 1)
        ys.append(c.y * 2 - 1)
    return min(xs), min(ys), max(xs), max(ys)


def furniture(scene, cam, model, s, job, distance):
    collection = bpy.data.collections[OVERLAY]
    if job["style"] != "blueprint":
        return
    line = job["lineColor"]
    paper = job["paperColor"] or "#0f2a55"
    px = 2.0 / 1024
    if not job["transparent"]:
        ln, pp = srgb(line), srgb(paper)
        minor = solid("maquette_grid", paper)
        minor.diffuse_color = (*(a * 0.07 + b * 0.93 for a, b in zip(ln, pp)), 1.0)
        major = solid("maquette_grid_major", paper)
        major.diffuse_color = (*(a * 0.16 + b * 0.84 for a, b in zip(ln, pp)), 1.0)
        far = distance * 4
        steps = [i / 16 for i in range(-16, 17)]
        Sheet(cam, collection, far, minor).bars(
            "grid", [b for v in steps if round(v * 16) % 4 for b in ((v, -1, v, 1, px), (-1, v, 1, v, px))]
        )
        Sheet(cam, collection, far * 0.99, major).bars(
            "grid_major", [b for v in steps if not round(v * 16) % 4 for b in ((v, -1, v, 1, px * 1.4), (-1, v, 1, v, px * 1.4))]
        )
    near = max(cam.data.clip_start * 3, distance * 0.05)
    ink = solid("maquette_marks", line)
    sheet = Sheet(cam, collection, near, ink)
    measured = max(model.hi.z - model.lo.z, 1e-6)
    real_h = s.get("heightM") or measured
    per_unit = real_h / measured
    left, bottom, right, top = screen_box(scene, cam, model.points)
    t = px * 1.3
    if job["marks"]:
        x = max(left - 0.09, -0.94)
        y = max(bottom - 0.09, -0.86)
        tick = 0.018
        sheet.bars(
            "marks",
            [
                (x, bottom, x, top, t),
                (x - tick, bottom, x + tick, bottom, t),
                (x - tick, top, x + tick, top, t),
                (left, y, right, y, t),
                (left, y - tick, left, y + tick, t),
                (right, y - tick, right, y + tick, t),
            ],
        )
        size = 0.034
        sheet.text("height", f"{real_h:.2f} m", x - 0.016, (bottom + top) / 2, size, "CENTER", math.pi / 2)
        width_real = (model.hi.x - model.lo.x) * per_unit
        sheet.text("width", f"{width_real:.2f} m", (left + right) / 2, y - 0.046, size, "CENTER")
    if job["titleBlock"]:
        w, h = 0.62, 0.15
        x1, y0 = 0.94, -0.94
        x0, y1 = x1 - w, y0 + h
        mid = y0 + h * 0.42
        sheet.bars(
            "title_box",
            [
                (x0, y0, x1, y0, t),
                (x0, y1, x1, y1, t),
                (x0, y0, x0, y1, t),
                (x1, y0, x1, y1, t),
                (x0, mid, x1, mid, t),
            ],
        )
        name = (s.get("name") or "model").upper()
        size = 0.042 if len(name) <= 20 else 0.03
        sheet.text("title", name[:34], x0 + 0.018, mid + 0.026, size, spacing=1.08)
        version = f"  V{s['version']}" if s.get("version") else ""
        sheet.text("title_sub", f"H {real_h:.2f} M{version}", x0 + 0.018, y0 + 0.02, 0.03, spacing=1.06)


# Output --------------------------------------------------------------------


def still(scene, job, path, alpha):
    scene.render.resolution_x = scene.render.resolution_y = job["size"]
    scene.render.resolution_percentage = 100
    scene.render.film_transparent = alpha
    use_clear_view(scene)
    image = scene.render.image_settings
    image.file_format = "PNG"
    image.color_mode = "RGBA" if alpha else "RGB"
    image.color_depth = "8"
    os.makedirs(os.path.dirname(os.path.abspath(path)), exist_ok=True)
    tmp = path + ".part.png"
    scene.render.filepath = tmp
    bpy.ops.render.render(write_still=True)
    os.replace(tmp, path)


def lines_only(path, line, paper):
    """A blueprint with no paper: each pixel's line colour turned into alpha."""
    image = bpy.data.images.load(path)
    image.colorspace_settings.name = "Non-Color"
    w, h = image.size
    px = numpy.empty(w * h * 4, dtype=numpy.float32)
    image.pixels.foreach_get(px)
    px = px.reshape(h, w, 4)
    ln = numpy.array(gamma_of(line), dtype=numpy.float32)
    pp = numpy.array(gamma_of(paper), dtype=numpy.float32)
    along = ln - pp
    alpha = numpy.clip(((px[:, :, :3] - pp) @ along) / max(float(along @ along), 1e-6), 0, 1)
    px[:, :, :3] = ln
    px[:, :, 3] = alpha
    save_pixels(path, px, w, h)
    bpy.data.images.remove(image)


def gamma_of(hex_color):
    h = hex_color.lstrip("#")
    return [int(h[i : i + 2], 16) / 255 for i in (0, 2, 4)]


def save_pixels(path, px, w, h):
    image = bpy.data.images.new("maquette_out", w, h, alpha=True)
    image.colorspace_settings.name = "Non-Color"
    image.pixels.foreach_set(px.astype(numpy.float32).ravel())
    image.filepath_raw = path
    image.file_format = "PNG"
    image.save()
    bpy.data.images.remove(image)


def run_still(scene, model, s, job):
    model.turn(0)
    scene.render.resolution_x = scene.render.resolution_y = job["size"]
    furnished = job["style"] == "blueprint" and (job["marks"] or job["titleBlock"])
    cam, centre, distance = place_camera(scene, model.corners(False), job["azimuth"], job["elevation"], fill_of(job))
    # Up a little, so the width mark clears the title block.
    cam.data.shift_y = -0.03 if furnished else 0.0
    bpy.context.view_layer.update()
    overlay_collection(scene)
    paper = apply_look(scene, model, job, centre)
    furniture(scene, cam, model, s, job, distance)
    path = os.path.abspath(job["out"])
    blueprint_alpha = job["transparent"] and job["style"] == "blueprint"
    still(scene, job, path, job["transparent"] and not blueprint_alpha)
    if blueprint_alpha:
        lines_only(path, job["lineColor"], paper)


def fill_of(job):
    # A blueprint keeps room for its marks and title block.
    if job["style"] == "blueprint" and (job["marks"] or job["titleBlock"]):
        return 0.72
    return 0.84


def read_pixels(path):
    image = bpy.data.images.load(path)
    image.colorspace_settings.name = "Non-Color"
    w, h = image.size
    px = numpy.empty(w * h * 4, dtype=numpy.float32)
    image.pixels.foreach_get(px)
    bpy.data.images.remove(image)
    return px.reshape(h, w, 4)


def run_turn(scene, model, job):
    """One full turn: the frames, the box they move in, then cropped and flat."""
    overlay_collection(scene)
    scene.render.resolution_x = scene.render.resolution_y = job["size"]
    points = model.corners(True)
    cam, centre, _ = place_camera(scene, points, job["azimuth"], job["elevation"], 0.9)
    cam.data.shift_y = 0.0
    paper = apply_look(scene, model, job, centre)
    out = os.path.abspath(job["out"])
    os.makedirs(out, exist_ok=True)
    frames = job["frames"]
    transparent = job["still"] or job["style"] != "shaded"
    moved = None
    first = None
    raws = []
    for n in range(frames):
        model.turn(360.0 * n / frames)
        path = os.path.join(out, f"raw{n + 1:04d}.png")
        still(scene, job, path, transparent)
        raws.append(path)
        px = read_pixels(path)
        if transparent:
            mask = px[:, :, 3] > 0.01
        elif first is None:
            first = px
            mask = numpy.zeros(px.shape[:2], dtype=bool)
        else:
            mask = numpy.abs(px[:, :, :3] - first[:, :, :3]).max(axis=2) > 0.04
        moved = mask if moved is None else moved | mask
    model.turn(0)
    x, y, w, h = crop_box(moved, job["crop"])
    flat = numpy.array(gamma_of(paper), dtype=numpy.float32)
    height = moved.shape[0]
    for n, path in enumerate(raws):
        px = read_pixels(path)
        if transparent:
            alpha = px[:, :, 3:4]
            px[:, :, :3] = px[:, :, :3] * alpha + flat * (1 - alpha)
        px[:, :, 3] = 1.0
        # Pixels run bottom up in Blender, the box top down.
        cut = px[height - y - h : height - y, x : x + w]
        target = os.path.join(out, f"f{n + 1:04d}.png")
        save_pixels(target, cut, w, h)
        if job["webp"]:
            save_webp(scene, target, job["webp"])
        os.remove(path)
    if job["boxMargin"]:
        x, y, w, h = crop_box(moved, job["boxMargin"])
    return {"moving": {"x": x, "y": y, "w": w, "h": h, "canvas": job["size"]}, "paper": paper}


def crop_box(moved, margin):
    """What moved, with a margin, top down and in even pixels."""
    height, width = moved.shape
    rows = numpy.flatnonzero(moved.any(axis=1))
    cols = numpy.flatnonzero(moved.any(axis=0))
    if not len(rows) or not margin:
        return 0, 0, width - width % 2, height - height % 2
    x0 = max(int(cols[0]) - margin, 0)
    x1 = min(int(cols[-1]) + 1 + margin, width)
    y0 = max(height - int(rows[-1]) - 1 - margin, 0)
    y1 = min(height - int(rows[0]) + margin, height)
    w, h = x1 - x0, y1 - y0
    return x0, y0, w - w % 2, h - h % 2


def save_webp(scene, png, quality):
    image = bpy.data.images.load(png)
    settings = scene.render.image_settings
    settings.file_format = "WEBP"
    settings.color_mode = "RGB"
    settings.quality = quality
    image.save_render(png[: -len(".png")] + ".webp", scene=scene)
    bpy.data.images.remove(image)
    settings.file_format = "PNG"


def run_sheet(scene, job):
    """Finished pictures in rows on paper, each with its name below."""
    sheet = job["sheet"]
    tiles = sheet["tiles"]
    columns = max(1, min(sheet.get("columns", 4), len(tiles)))
    rows = math.ceil(len(tiles) / columns)
    tile = sheet.get("tile", 512)
    names = sheet.get("names", True)
    gap = max(8, tile // 32)
    label = (tile // 9 + gap) if names else 0
    width = columns * tile + (columns + 1) * gap
    height = rows * (tile + label) + (rows + 1) * gap
    paper = sheet.get("paper") or PAPER_CLAY
    cam_data = bpy.data.cameras.new("maquette_sheet")
    cam_data.type = "ORTHO"
    cam_data.ortho_scale = max(width, height)
    cam = bpy.data.objects.new("maquette_sheet", cam_data)
    scene.collection.objects.link(cam)
    cam.location = (width / 2, height / 2, 100)
    scene.camera = cam
    ink = solid("maquette_sheet_ink", sheet.get("ink") or "#3a3834")
    for n, entry in enumerate(tiles):
        col, row = n % columns, n // columns
        left = gap + col * (tile + gap)
        top = height - gap - row * (tile + label + gap)
        picture = bpy.data.images.load(os.path.abspath(entry["file"]))
        picture.colorspace_settings.name = "sRGB"
        mat = bpy.data.materials.new(f"tile{n}")
        thumb.use_nodes(mat)
        node = mat.node_tree.nodes.new("ShaderNodeTexImage")
        node.image = picture
        node.interpolation = "Closest"
        mat.node_tree.nodes.active = node
        mesh = bpy.data.meshes.new(f"tile{n}")
        mesh.from_pydata(
            [(left, top - tile, 0), (left + tile, top - tile, 0), (left + tile, top, 0), (left, top, 0)],
            [],
            [(0, 1, 2, 3)],
        )
        uv = mesh.uv_layers.new()
        for loop, co in zip(uv.data, ((0, 0), (1, 0), (1, 1), (0, 1))):
            loop.uv = co
        mesh.materials.append(mat)
        scene.collection.objects.link(bpy.data.objects.new(f"tile{n}", mesh))
        if names and entry.get("name"):
            curve = bpy.data.curves.new(f"name{n}", "FONT")
            curve.body = entry["name"]
            curve.size = tile / 14
            curve.materials.append(ink)
            text = bpy.data.objects.new(f"name{n}", curve)
            text.location = (left + 2, top - tile - tile / 11, 0)
            scene.collection.objects.link(text)
    job = {**job, "size": max(width, height)}
    workbench(scene, job, "FLAT", "TEXTURE", paper)
    no_lines(scene)
    scene.render.resolution_x = width
    scene.render.resolution_y = height
    scene.render.resolution_percentage = 100
    scene.render.film_transparent = False
    use_clear_view(scene)
    image = scene.render.image_settings
    image.file_format = "PNG"
    image.color_mode = "RGB"
    out = os.path.abspath(job["out"])
    os.makedirs(os.path.dirname(out), exist_ok=True)
    tmp = out + ".part.png"
    scene.render.filepath = tmp
    bpy.ops.render.render(write_still=True)
    os.replace(tmp, out)


def main():
    s = spec()
    started = time.time()
    report = {"jobs": []}
    if s.get("model"):
        scene = thumb.load(s["model"])
        model = Model(scene)
        report.update(textured=model.textured, tinted=model.tinted, triangles=model.triangles, edges=model.edges)
    else:
        bpy.ops.wm.read_factory_settings(use_empty=True)
        scene = bpy.context.scene
        model = None
    for raw in s["jobs"]:
        job = {**job_defaults(), **raw}
        begun = time.time()
        extra = {}
        if job["sheet"]:
            run_sheet(scene, job)
        elif job["frames"]:
            extra = run_turn(scene, model, job)
        else:
            run_still(scene, model, s, job)
        report["jobs"].append({"out": job["out"], "seconds": round(time.time() - begun, 2), **extra})
    report["seconds"] = round(time.time() - started, 2)
    if s.get("report"):
        with open(s["report"], "w", encoding="utf-8") as f:
            json.dump(report, f)
    print("render_styles", json.dumps({k: v for k, v in report.items() if k != "jobs"}))


main()
