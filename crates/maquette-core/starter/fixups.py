"""Fix-ups and real size: repairs recorded per asset, replayed on every version.

The fix-ups and the real size of each asset live in prepare.json, keyed by
asset id. They are applied in this order, whatever order they were added in:

    apply-transforms   bake object transforms into the meshes
    remove-loose       drop loose parts smaller than "minShare" of the model
    merge              join the meshes into one ("byMaterial": one per material)
    split-parts        cut each mesh into parts named part_1, part_2 (top
                       first), "by" "loose" pieces or "material", joining a
                       piece no bigger than "mergeUnder" of the model into
                       its nearest bigger neighbour
    (pad edges)        with "padEdges": true, push each UV island's border
                       colours out into the empty texels around it
    decimate           collapse down to "triangles"
    shrink-textures    scale images down to "maxSize" px on the longest edge
    origin-bottom      put the origin at the bottom centre, on the floor

Edge padding is for scans: a Tripo or Meshy colour atlas is cut into
hundreds of small UV islands with light texels between them. Once a texture
is shrunk (here or by Compress) and the browser builds its mipmaps, those
texels bleed over the seams as thin white lines. Padding fills every texel
no island covers with the colour of the nearest islands, so no gap colour
is left to bleed, at any size or mip level. It runs on the model's own UVs,
before a decimate moves them, and only changes texels the model never
shows.

then the part names ("parts" in the entry): the groups in "merge" are
joined into their first part and each part in "names" is renamed, so the
exported nodes carry the names object states and explode tracks look for.
"hidden" is only for looking in the Inspector and changes nothing here.

then the real size: a uniform scale so the model's "height", "width",
"depth" or "longest" side measures "value" in "unit" (cm or m). The scene
layout's own scale applies on top of that.

Maquette uses this file in two ways, always headless:

1. On its own, to show what the fix-ups do before you keep them:

    blender -b --factory-startup --python-exit-code 1 --python tools/fixups.py -- \
        --input model.glb --plan plan.json --output fixed.glb --stats stats.json

   plan.json is one asset's entry ({"size": ..., "fixups": [...]}). The
   input file is only read; the fixed model goes to --output and the numbers
   before and after every fix-up to --stats.

2. From build_scene.py, which imports it and calls prepare() on the objects
   of each model it imports, before the layout moves them.

Rigged meshes keep their skin: transforms are not applied to them and
merge leaves them apart. Edit freely: this file is yours. Maquette reruns
the Build scene step when it or prepare.json changes.
"""

import argparse
import json
import math
import os
import sys

import bmesh
import bpy
import numpy as np
from mathutils import Matrix, Vector

PREPARE_VERSION = 1
ORDER = [
    "apply-transforms",
    "remove-loose",
    "merge",
    "split-parts",
    "pad-edges",
    "decimate",
    "shrink-textures",
    "origin-bottom",
]
UNIT_METRES = {"cm": 0.01, "m": 1.0}
# The name a model file gives an object, which the export writes back when
# Blender had to rename it (build_scene.py tags imports with it).
SOURCE_NAME = "maquette_name"


def log(message):
    print(f"[fixups] {message}", flush=True)


def live(objects):
    """The objects not deleted yet. A deleted one raises on any access."""
    found = []
    for obj in objects:
        try:
            if obj.name in bpy.data.objects:
                found.append(obj)
        except ReferenceError:
            pass
    return found


def meshes(objects):
    return [o for o in live(objects) if o.type == "MESH"]


def rigged(obj):
    return obj.find_armature() is not None or any(m.type == "ARMATURE" for m in obj.modifiers)


def triangles(obj):
    mesh = obj.data
    mesh.calc_loop_triangles()
    return len(mesh.loop_triangles)


def images(objects):
    found = []
    for obj in meshes(objects):
        for slot in obj.material_slots:
            material = slot.material
            if material is None or material.node_tree is None:
                continue
            for node in material.node_tree.nodes:
                if node.type == "TEX_IMAGE" and node.image is not None and node.image not in found:
                    found.append(node.image)
    return found


def world_bounds(objects):
    """Blender space min and max corners of the meshes, or None without any."""
    lo = Vector((math.inf,) * 3)
    hi = Vector((-math.inf,) * 3)
    for obj in meshes(objects):
        for vertex in obj.data.vertices:
            point = obj.matrix_world @ vertex.co
            lo = Vector(map(min, lo, point))
            hi = Vector(map(max, hi, point))
    if lo.x == math.inf:
        return None
    return lo, hi


def gltf_bounds(objects):
    """[min, max] in glTF axes (Y up): x stays, y is Blender z, z is -y."""
    found = world_bounds(objects)
    if found is None:
        return None
    lo, hi = found
    return [[lo.x, lo.z, -hi.y], [hi.x, hi.z, -lo.y]]


def draw_calls(obj):
    """One per material the mesh's faces use, as the exporter splits it."""
    used = {polygon.material_index for polygon in obj.data.polygons}
    return len(used)


def stats(objects):
    bounds = gltf_bounds(objects)
    edges = [max(image.size[0], image.size[1]) for image in images(objects)]
    return {
        "triangles": sum(triangles(o) for o in meshes(objects)),
        "objects": len(meshes(objects)),
        "drawCalls": sum(draw_calls(o) for o in meshes(objects)),
        "maxTexture": max(edges, default=0),
        "min": [round(v, 6) for v in bounds[0]] if bounds else None,
        "max": [round(v, 6) for v in bounds[1]] if bounds else None,
    }


def roots(objects):
    inside = set(objects)
    return [o for o in objects if o.parent is None or o.parent not in inside]


def single_user(obj):
    if obj.data is not None and obj.data.users > 1:
        obj.data = obj.data.copy()


def apply_transforms(objects, _fixup):
    done = 0
    for obj in meshes(objects):
        if rigged(obj):
            continue
        single_user(obj)
        world = obj.matrix_world.copy()
        obj.parent = None
        obj.data.transform(world)
        obj.matrix_world = Matrix.Identity(4)
        done += 1
    skipped = len(meshes(objects)) - done
    # Empties left with nothing under them only carried those transforms.
    for obj in [o for o in live(objects) if o.type == "EMPTY"]:
        if not obj.children:
            bpy.data.objects.remove(obj)
    return f"{done} meshes, {skipped} rigged kept" if skipped else f"{done} meshes"


def components(bm):
    """Groups of vertices that touch: joined by an edge or at the same spot,
    so a face split off along a UV seam still counts as part of its object."""
    parent = list(range(len(bm.verts)))

    def find(i):
        while parent[i] != i:
            parent[i] = parent[parent[i]]
            i = parent[i]
        return i

    def union(a, b):
        ra, rb = find(a), find(b)
        if ra != rb:
            parent[ra] = rb

    for edge in bm.edges:
        union(edge.verts[0].index, edge.verts[1].index)
    spots = {}
    for vert in bm.verts:
        key = tuple(round(c, 5) for c in vert.co)
        if key in spots:
            union(vert.index, spots[key])
        else:
            spots[key] = vert.index
    groups = {}
    for vert in bm.verts:
        groups.setdefault(find(vert.index), []).append(vert)
    return list(groups.values())


def diagonal(points):
    lo = Vector(map(min, *points)) if len(points) > 1 else points[0]
    hi = Vector(map(max, *points)) if len(points) > 1 else points[0]
    return (hi - lo).length


def remove_loose(objects, fixup):
    found = world_bounds(objects)
    if found is None:
        return "no meshes"
    limit = float(fixup.get("minShare", 0.02)) * (found[1] - found[0]).length
    parts = 0
    for obj in meshes(objects):
        single_user(obj)
        bm = bmesh.new()
        bm.from_mesh(obj.data)
        bm.verts.ensure_lookup_table()
        bm.verts.index_update()
        drop = []
        for group in components(bm):
            points = [obj.matrix_world @ v.co for v in group]
            if diagonal(points) < limit:
                drop.extend(group)
                parts += 1
        if drop:
            bmesh.ops.delete(bm, geom=drop, context="VERTS")
            bm.to_mesh(obj.data)
        bm.free()
    for obj in meshes(objects):
        if len(obj.data.vertices) == 0:
            bpy.data.objects.remove(obj)
    return f"{parts} {'part' if parts == 1 else 'parts'} under {limit:.4g} m"


def join(group):
    if len(group) < 2:
        return 0
    active = group[0]
    with bpy.context.temp_override(
        active_object=active,
        object=active,
        selected_objects=group,
        selected_editable_objects=group,
    ):
        bpy.ops.object.join()
    return len(group) - 1


def merge(objects, fixup):
    loose = [o for o in meshes(objects) if not rigged(o)]
    if fixup.get("byMaterial"):
        by_look = {}
        for obj in loose:
            key = tuple(sorted(s.material.name for s in obj.material_slots if s.material))
            by_look.setdefault(key, []).append(obj)
        groups = list(by_look.values())
    else:
        groups = [loose]
    joined = sum(join(group) for group in groups)
    if joined == 0:
        return "nothing to join"
    return f"{len(loose)} meshes joined into {len(groups)}"


def face_groups(bm, by):
    """A group per face: its loose piece, or its material slot."""
    if by == "material":
        return [face.material_index for face in bm.faces]
    owner = {}
    for i, group in enumerate(components(bm)):
        for vert in group:
            owner[vert.index] = i
    return [owner[face.verts[0].index] for face in bm.faces]


def box_gap(a, b):
    """How far apart two (lo, hi) boxes are, 0 when they touch."""
    out = [max(a[0][i] - b[1][i], b[0][i] - a[1][i], 0.0) for i in range(3)]
    return Vector(out).length


def plan_split(obj, bm, by, limit):
    """The faces of each part and how many small pieces joined another.

    A piece whose box diagonal is at most `limit` metres joins the nearest
    bigger piece; when every piece is that small the largest one stays."""
    groups = {}
    for face, group in zip(bm.faces, face_groups(bm, by)):
        groups.setdefault(group, []).append(face.index)
    boxes = {}
    for group, faces in groups.items():
        points = [obj.matrix_world @ v.co for i in faces for v in bm.faces[i].verts]
        boxes[group] = (Vector(map(min, *points)), Vector(map(max, *points)))
    size = {g: (box[1] - box[0]).length for g, box in boxes.items()}
    big = [g for g in groups if size[g] > limit] or [max(groups, key=size.get)]
    parts = {g: list(groups[g]) for g in big}
    for group in groups:
        if group in parts:
            continue
        home = min(big, key=lambda b: (box_gap(boxes[group], boxes[b]), -size[b]))
        parts[home].extend(groups[group])
    return list(parts.values()), len(groups), len(groups) - len(big)


def cut(obj, faces):
    """A copy of `obj` with only `faces`, its origin at their centre."""
    part = obj.copy()
    part.data = obj.data.copy()
    for collection in obj.users_collection:
        collection.objects.link(part)
    bm = bmesh.new()
    bm.from_mesh(part.data)
    bm.faces.ensure_lookup_table()
    keep = set(faces)
    bmesh.ops.delete(bm, geom=[f for f in bm.faces if f.index not in keep], context="FACES")
    bmesh.ops.delete(bm, geom=[v for v in bm.verts if not v.link_faces], context="VERTS")
    bm.to_mesh(part.data)
    bm.free()
    points = [v.co for v in part.data.vertices]
    centre = (Vector(map(min, *points)) + Vector(map(max, *points))) / 2
    part.data.transform(Matrix.Translation(-centre))
    part.matrix_basis = part.matrix_basis @ Matrix.Translation(centre)
    return part


def split_parts(objects, fixup):
    by = "material" if fixup.get("by") == "material" else "loose"
    found = world_bounds(objects)
    if found is None:
        return "no meshes", None
    limit = float(fixup.get("mergeUnder", 0.01)) * (found[1] - found[0]).length
    numbers = {"pieces": 0, "merged": 0, "byLoose": 0, "byMaterial": 0}
    made = []
    for obj in [o for o in meshes(objects) if not rigged(o) and o.data.polygons]:
        bm = bmesh.new()
        bm.from_mesh(obj.data)
        bm.verts.ensure_lookup_table()
        bm.faces.ensure_lookup_table()
        bm.verts.index_update()
        plans = {way: plan_split(obj, bm, way, limit) for way in ("loose", "material")}
        bm.free()
        numbers["byLoose"] += len(plans["loose"][0])
        numbers["byMaterial"] += len(plans["material"][0])
        parts, pieces, merged = plans[by]
        numbers["pieces"] += pieces
        numbers["merged"] += merged
        if len(parts) < 2:
            continue
        cuts = [cut(obj, faces) for faces in parts]
        for child in list(obj.children):
            world = child.matrix_world.copy()
            child.parent = cuts[0]
            child.matrix_world = world
        bpy.data.objects.remove(obj)
        made.extend(cuts)
    bpy.context.view_layer.update()
    # Top first, then left to right, so the names follow the model.
    made.sort(key=lambda o: (-round(o.matrix_world.translation.z, 4), o.matrix_world.translation.x))
    for i, part in enumerate(made, start=1):
        name = f"part_{i}"
        part.name = name
        part.data.name = name
        part[SOURCE_NAME] = name
    objects.extend(made)
    word = "material" if by == "material" else "piece"
    if not made:
        return f"one {word}, nothing to split", numbers
    note = f"{len(made)} parts, {numbers['pieces']} {word}s, {numbers['merged']} tiny bits merged in"
    return note, numbers


def part_name(obj):
    """The name the model file gives the part."""
    return obj.get(SOURCE_NAME) or obj.name


def apply_parts(objects, parts):
    """Joins the groups in parts["merge"], then renames by parts["names"]."""
    names = parts.get("names") or {}
    joined = renamed = 0
    for group in parts.get("merge") or []:
        by_name = {part_name(o): o for o in meshes(objects) if not rigged(o)}
        found = [by_name[p] for p in group if p in by_name]
        joined += join(found)
    for obj in meshes(objects):
        new = names.get(part_name(obj))
        if new:
            obj.name = new
            obj.data.name = new
            obj[SOURCE_NAME] = new
            renamed += 1
    return f"{renamed} parts renamed, {joined} joined into another"


def decimate(objects, fixup):
    target = int(fixup.get("triangles", 0))
    total = sum(triangles(o) for o in meshes(objects))
    if target <= 0 or total <= target:
        return f"already {total} triangles"
    ratio = target / total
    skipped = 0
    for obj in meshes(objects):
        if obj.data.shape_keys is not None:
            skipped += 1
            continue
        single_user(obj)
        # Only the decimate is evaluated: a skin or other modifier stays live.
        others = [m for m in obj.modifiers if m.show_viewport]
        for modifier in others:
            modifier.show_viewport = False
        modifier = obj.modifiers.new("maquette_decimate", "DECIMATE")
        modifier.decimate_type = "COLLAPSE"
        modifier.ratio = ratio
        depsgraph = bpy.context.evaluated_depsgraph_get()
        depsgraph.update()
        evaluated = obj.evaluated_get(depsgraph)
        mesh = bpy.data.meshes.new_from_object(
            evaluated, preserve_all_data_layers=True, depsgraph=depsgraph
        )
        obj.modifiers.remove(modifier)
        for modifier in others:
            modifier.show_viewport = True
        old = obj.data
        obj.data = mesh
        if old.users == 0:
            bpy.data.meshes.remove(old)
    note = f"ratio {ratio:.3f}"
    return f"{note}, {skipped} meshes with shape keys kept" if skipped else note


def shrink_textures(objects, fixup):
    limit = int(fixup.get("maxSize", 2048))
    shrunk = 0
    for image in images(objects):
        width, height = image.size
        edge = max(width, height)
        if edge <= limit or edge == 0:
            continue
        factor = limit / edge
        image.scale(max(1, round(width * factor)), max(1, round(height * factor)))
        # The exporter writes packed bytes as they are, so pack the scaled pixels.
        image.pack()
        shrunk += 1
    return f"{shrunk} textures to {limit} px"


# Edge padding. Triangles up to this many pixels across are rasterized
# together; bigger ones one by one.
SMALL_TRIANGLE_PX = 8
# How much of an image the islands must cover before its gaps are filled:
# less means the UVs do not describe the image (another UV set, a tiling
# texture), and it is left alone.
MIN_ISLAND_SHARE = 0.02
# Exact neighbour steps before the smooth fill takes over.
EXACT_STEPS = 4


def image_users(objects, image):
    """(mesh object, uv layer name, material indices) for every mesh whose
    materials sample `image`."""
    users = []
    for obj in meshes(objects):
        slots = []
        layer = None
        for index, slot in enumerate(obj.material_slots):
            material = slot.material
            if material is None or material.node_tree is None:
                continue
            for node in material.node_tree.nodes:
                if node.type != "TEX_IMAGE" or node.image != image:
                    continue
                slots.append(index)
                link = node.inputs["Vector"].links[0] if node.inputs["Vector"].links else None
                if link is not None and link.from_node.type == "UVMAP" and link.from_node.uv_map:
                    layer = link.from_node.uv_map
        uvs = obj.data.uv_layers
        if not slots or len(uvs) == 0:
            continue
        name = layer if layer and layer in uvs else next((u.name for u in uvs if u.active_render), uvs[0].name)
        users.append((obj, name, slots))
    return users


def uv_triangles(obj, layer, slots, width, height):
    """The UV triangles of the mesh's faces that use `slots`, in pixels."""
    mesh = obj.data
    mesh.calc_loop_triangles()
    count = len(mesh.loop_triangles)
    if count == 0:
        return np.zeros((0, 3, 2), np.float64)
    loops = np.empty(count * 3, np.int32)
    mesh.loop_triangles.foreach_get("loops", loops)
    material = np.empty(count, np.int32)
    mesh.loop_triangles.foreach_get("material_index", material)
    uv = np.empty(len(mesh.loops) * 2, np.float64)
    mesh.uv_layers[layer].data.foreach_get("uv", uv)
    tris = uv.reshape(-1, 2)[loops].reshape(-1, 3, 2)[np.isin(material, slots)]
    # A triangle in a repeated tile lands on the image it samples.
    tris = tris - np.floor(tris.mean(axis=1, keepdims=True))
    return tris * np.array([width, height], np.float64)


def inside(tris, px, py):
    """Which pixel centres (px, py) lie in their triangle, either winding,
    with a little slack so shared edges never leave a gap."""
    a, b, c = (tris[:, i, None, :] for i in range(3))

    def edge(p, q):
        return (q[..., 0] - p[..., 0]) * (py - p[..., 1]) - (q[..., 1] - p[..., 1]) * (px - p[..., 0])

    e0, e1, e2 = edge(a, b), edge(b, c), edge(c, a)
    slack = -1e-6
    return ((e0 >= slack) & (e1 >= slack) & (e2 >= slack)) | (
        (e0 <= -slack) & (e1 <= -slack) & (e2 <= -slack)
    )


def rasterize(mask, tris):
    """Marks the pixels the triangles cover, and each vertex's pixel, so a
    triangle smaller than a pixel still counts."""
    height, width = mask.shape
    if len(tris) == 0:
        return
    vx = np.clip(tris[..., 0].astype(np.int64), 0, width - 1)
    vy = np.clip(tris[..., 1].astype(np.int64), 0, height - 1)
    mask[vy.ravel(), vx.ravel()] = True
    lo = np.floor(tris.min(axis=1)).astype(np.int64)
    hi = np.ceil(tris.max(axis=1)).astype(np.int64)
    span = (hi - lo).max(axis=1)
    small = span <= SMALL_TRIANGLE_PX
    grid = np.arange(SMALL_TRIANGLE_PX + 1)
    gx, gy = (g.ravel() for g in np.meshgrid(grid, grid))
    picked = np.flatnonzero(small)
    for start in range(0, len(picked), 20000):
        chunk = picked[start : start + 20000]
        x = lo[chunk, 0, None] + gx
        y = lo[chunk, 1, None] + gy
        hit = inside(tris[chunk], x + 0.5, y + 0.5)
        hit &= (x >= 0) & (x < width) & (y >= 0) & (y < height)
        mask[y[hit], x[hit]] = True
    for i in np.flatnonzero(~small):
        x0, y0 = np.maximum(lo[i], 0)
        x1, y1 = np.minimum(hi[i], [width, height])
        if x1 <= x0 or y1 <= y0:
            continue
        xs, ys = np.meshgrid(np.arange(x0, x1), np.arange(y0, y1))
        hit = inside(tris[i : i + 1], xs.ravel()[None] + 0.5, ys.ravel()[None] + 0.5)[0]
        mask[ys.ravel()[hit], xs.ravel()[hit]] = True


def grow(mask):
    """The mask one pixel wider in every direction."""
    out = mask.copy()
    out[1:, :] |= mask[:-1, :]
    out[:-1, :] |= mask[1:, :]
    out[:, 1:] |= out[:, :-1].copy()
    out[:, :-1] |= out[:, 1:].copy()
    return out


def neighbour_step(pixels, known):
    """Fills each unknown pixel that touches known ones with their mean."""
    total = np.zeros_like(pixels)
    count = np.zeros(known.shape, np.float32)
    weight = known.astype(np.float32)
    for dy, dx in ((-1, 0), (1, 0), (0, -1), (0, 1), (-1, -1), (-1, 1), (1, -1), (1, 1)):
        shifted_w = np.roll(np.roll(weight, dy, 0), dx, 1)
        total += np.roll(np.roll(pixels * weight[..., None], dy, 0), dx, 1)
        count += shifted_w
    fill = ~known & (count > 0)
    pixels[fill] = total[fill] / count[fill, None]
    return known | fill


def pull_push(pixels, known):
    """Fills every unknown pixel smoothly from the known ones around it: a
    pyramid of mask weighted averages, read back from coarse to fine."""
    levels = []
    color = pixels * known[..., None]
    weight = known.astype(np.float32)
    while True:
        levels.append((color, weight))
        h, w = weight.shape
        if h == 1 and w == 1:
            break
        ph, pw = h + h % 2, w + w % 2
        color = np.pad(color, ((0, ph - h), (0, pw - w), (0, 0)))
        weight = np.pad(weight, ((0, ph - h), (0, pw - w)))
        color = color.reshape(ph // 2, 2, pw // 2, 2, -1).sum(axis=(1, 3))
        weight = weight.reshape(ph // 2, 2, pw // 2, 2).sum(axis=(1, 3))
    color, weight = levels[-1]
    filled = color / np.maximum(weight, 1e-8)[..., None]
    for color, weight in reversed(levels[:-1]):
        h, w = weight.shape
        up = filled.repeat(2, axis=0).repeat(2, axis=1)[:h, :w]
        mean = color / np.maximum(weight, 1e-8)[..., None]
        filled = np.where((weight > 0)[..., None], mean, up)
    pixels[~known] = filled[~known]


def pad_image(image, users):
    """Pads one image; returns the share of it the islands cover, or None
    when it was left alone."""
    width, height = image.size
    if width == 0 or height == 0:
        return None
    mask = np.zeros((height, width), bool)
    for obj, layer, slots in users:
        rasterize(mask, uv_triangles(obj, layer, slots, width, height))
    # Bilinear filtering reads one texel past an island's edge.
    mask = grow(mask)
    share = float(mask.mean())
    if share < MIN_ISLAND_SHARE or share >= 1.0:
        return None
    channels = image.channels
    flat = np.empty(width * height * channels, np.float32)
    image.pixels.foreach_get(flat)
    pixels = flat.reshape(height, width, channels)
    known = mask.copy()
    for _ in range(EXACT_STEPS):
        known = neighbour_step(pixels, known)
    pull_push(pixels, known)
    image.pixels.foreach_set(pixels.ravel())
    # The exporter writes packed bytes as they are, so pack the new pixels.
    image.pack()
    return share


def pad_edges(objects):
    padded = []
    for image in images(objects):
        users = image_users(objects, image)
        share = pad_image(image, users) if users else None
        if share is not None:
            padded.append(f"{image.name} ({share:.0%} islands)")
    if not padded:
        return "no texture with UV gaps to pad"
    return f"padded {len(padded)} textures: {', '.join(padded)}"


def origin_bottom(objects, _fixup):
    found = world_bounds(objects)
    if found is None:
        return "no meshes"
    lo, hi = found
    shift = Vector(((lo.x + hi.x) / 2, (lo.y + hi.y) / 2, lo.z))
    for obj in roots(objects):
        obj.matrix_world = Matrix.Translation(-shift) @ obj.matrix_world
    return f"moved by {-shift.x:.4g}, {-shift.z:.4g}, {shift.y:.4g} m"


OPS = {
    "apply-transforms": apply_transforms,
    "remove-loose": remove_loose,
    "merge": merge,
    "split-parts": split_parts,
    "decimate": decimate,
    "pad-edges": lambda objects, _fixup: pad_edges(objects),
    "shrink-textures": shrink_textures,
    "origin-bottom": origin_bottom,
}


def size_scale(objects, size):
    """The uniform scale that gives the model its real size, or None."""
    bounds = gltf_bounds(objects)
    if not size or bounds is None:
        return None
    extent = [hi - lo for lo, hi in zip(bounds[0], bounds[1])]
    axis = size.get("axis", "longest")
    measured = {"width": extent[0], "height": extent[1], "depth": extent[2]}.get(
        axis, max(extent)
    )
    target = float(size.get("value", 0)) * UNIT_METRES.get(size.get("unit", "cm"), 0.01)
    if measured <= 1e-9 or target <= 0:
        return None
    return target / measured


def prepare(objects, entry, say=log):
    """Applies one asset's fix-ups, then its real size, to its objects.

    Returns the objects that are left and one report per fix-up."""
    objects = live(objects)
    steps = []
    wanted = {f.get("op"): f for f in entry.get("fixups") or []}
    if entry.get("padEdges") is True:
        wanted["pad-edges"] = {"op": "pad-edges"}
    for op in ORDER:
        if op not in wanted:
            continue
        before = stats(objects)
        bpy.context.view_layer.update()
        done = OPS[op](objects, wanted[op])
        note, extra = done if isinstance(done, tuple) else (done, None)
        bpy.context.view_layer.update()
        # Merge and remove-loose delete objects; a split adds the parts it
        # cuts to the list and deletes the meshes it cut them from.
        objects = live(objects)
        after = stats(objects)
        say(f"{op}: {note}")
        step = {"op": op, "before": before, "after": after, "note": note}
        if extra is not None:
            step["split"] = extra
        steps.append(step)
    unknown = sorted(set(wanted) - set(ORDER))
    if unknown:
        say(f"skipped unknown fix-ups: {', '.join(str(u) for u in unknown)}")
    if entry.get("parts"):
        say(f"parts: {apply_parts(objects, entry['parts'])}")
        bpy.context.view_layer.update()
        objects = live(objects)
    scale = size_scale(objects, entry.get("size"))
    if scale is not None:
        for obj in roots(objects):
            obj.matrix_world = Matrix.Scale(scale, 4) @ obj.matrix_world
        bpy.context.view_layer.update()
        say(f"real size: scaled by {scale:.5g}")
    return objects, steps, scale


def read_entries(path):
    """prepare.json as asset id to entry; empty when missing or newer."""
    if not path or not os.path.isfile(path):
        return {}
    try:
        with open(path, encoding="utf-8") as fh:
            found = json.load(fh)
    except (OSError, ValueError) as err:
        log(f"could not read {path}: {err}")
        return {}
    if found.get("version", 1) > PREPARE_VERSION:
        log(f"{path} comes from a newer Maquette, so it is left out")
        return {}
    return found.get("models") or {}


def parse_args():
    argv = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
    parser = argparse.ArgumentParser(prog="fixups.py")
    parser.add_argument("--input", required=True, help="the model to read, never written")
    parser.add_argument("--plan", required=True, help="one asset's size and fix-ups, as JSON")
    parser.add_argument("--output", required=True, help="where the fixed GLB goes")
    parser.add_argument("--stats", required=True, help="where the numbers go, as JSON")
    return parser.parse_args(argv)


def main():
    args = parse_args()
    with open(args.plan, encoding="utf-8") as fh:
        plan = json.load(fh)
    bpy.ops.wm.read_factory_settings(use_empty=True)
    try:
        bpy.ops.import_scene.gltf(filepath=os.path.abspath(args.input), disable_bone_shape=True)
    except TypeError:  # an importer without the option
        bpy.ops.import_scene.gltf(filepath=os.path.abspath(args.input))
    objects = list(bpy.data.objects)
    bpy.context.view_layer.update()
    before = stats(objects)
    objects, steps, scale = prepare(objects, plan)
    after = stats(objects)
    out = os.path.abspath(args.output)
    os.makedirs(os.path.dirname(out), exist_ok=True)
    bpy.ops.export_scene.gltf(filepath=out, export_format="GLB")
    with open(args.stats, "w", encoding="utf-8") as fh:
        json.dump({"before": before, "after": after, "steps": steps, "scale": scale}, fh, indent=2)
    log(f"wrote {args.output}: {before['triangles']} to {after['triangles']} triangles")


if __name__ == "__main__":
    main()
