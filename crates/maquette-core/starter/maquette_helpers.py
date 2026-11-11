"""Helpers for built asset scripts: shapes, crisp shading and flat materials.

Maquette ships this file into the project's built/ folder, next to
maquette_example.py, which uses every helper once. A built asset's script
imports it like any other helper file:

    import maquette_helpers as mq

    def build():
        glaze = mq.glazed_ceramic("glaze", "#e9e3d5")
        profile = mq.fillet([(0, 0), (0.04, 0), (0.04, 0.09), (0.035, 0.09),
                             (0.035, 0.006), (0, 0.006)], 0.003)
        return mq.lathe("mug", profile, material=glaze)

and the asset lists built/maquette_helpers.py among its helpers (helpers in
add_built_asset over MCP, or Add files... under Helpers in the Add built asset
dialog), so the file is staged with the script and counts in its build key.

How the helpers work together:

- Blender space, z up, in metres. The exporter turns it into glTF's y up.
- A shape helper returns its object, already linked to the scene. `location`
  is the bottom centre of the shape (a sphere's centre), so a piece stands
  where it is put.
- Every shape comes shaded smooth with its hard edges kept crisp (see
  crisp_edges). No helper needs a bevel to look clean.
- Materials are one flat colour each: no image, no UV map. A colour is
  "#rrggbb" as a colour picker shows it.
- join() and finish() never drop a material: every face keeps the material it
  had, whatever order the pieces come in.

Maquette updates this file with the starter scripts (Update on the Pipeline
view's banner, or update_starters). Every built asset that lists it then shows
as changed and builds again on the next run. Keep helpers of your own in a
file of your own that imports this one: an edited copy of this file stops
updating until you ask for the new one.

It uses mesh and shader names from Blender 4.1 on, and is tested on Blender 5.2.
"""

import math

import bmesh
import bpy
from mathutils import Matrix, Vector

# Edges whose faces meet at more than this many degrees stay crisp.
CRISP_ANGLE = 30.0


# ---------------------------------------------------------------- materials


def color(value):
    """Linear RGBA for a colour: "#rrggbb" or "#rgb" as a colour picker shows
    it (sRGB), or a tuple of linear values, which passes through."""
    if not isinstance(value, str):
        values = tuple(float(v) for v in value)
        return values if len(values) == 4 else values + (1.0,)
    text = value.lstrip("#")
    if len(text) == 3:
        text = "".join(c * 2 for c in text)
    if len(text) != 6:
        raise ValueError(f"{value!r} is not a colour like #d3a56f")
    channels = [int(text[i : i + 2], 16) / 255 for i in (0, 2, 4)]
    linear = [c / 12.92 if c <= 0.04045 else ((c + 0.055) / 1.055) ** 2.4 for c in channels]
    return (*linear, 1.0)


def _shader(material):
    """The material's Principled BSDF. Blender 5 materials always have nodes;
    older versions need use_nodes."""
    if material.node_tree is None:
        material.use_nodes = True
    for node in material.node_tree.nodes:
        if node.bl_idname == "ShaderNodeBsdfPrincipled":
            return node
    raise ValueError(f"the material {material.name!r} has no Principled BSDF to set")


def flat_material(name, base, roughness=0.8, metallic=0.0, emission=0.0):
    """A material of one flat colour, made once per name: asking for the same
    name again returns that material with the new values. `emission` above 0
    makes it glow in its own colour at that strength."""
    material = bpy.data.materials.get(name) or bpy.data.materials.new(name)
    shader = _shader(material)
    rgba = color(base)
    shader.inputs["Base Color"].default_value = rgba
    shader.inputs["Roughness"].default_value = roughness
    shader.inputs["Metallic"].default_value = metallic
    if emission > 0:
        shader.inputs["Emission Color"].default_value = rgba
    shader.inputs["Emission Strength"].default_value = emission
    # What Blender's solid view shows.
    material.diffuse_color = rgba
    material.roughness = roughness
    material.metallic = metallic
    return material


def palette(colors):
    """A flat material per entry of `colors`, as a dict by name. A value is a
    colour, or (colour, roughness) or (colour, roughness, metallic):

        mat = palette({"oak": "#d3a56f", "steel": ("#b9bcc8", 0.4, 0.6)})
        box("shelf", (0.6, 0.2, 0.02), material=mat["oak"])
    """
    made = {}
    for name, value in colors.items():
        entry = (value,) if isinstance(value, str) else tuple(value)
        made[name] = flat_material(name, *entry)
    return made


def brass(name="brass", base="#c2a54f", roughness=0.3):
    """Polished brass that still reads as brass with no environment map. A
    full metal only mirrors what is around it, so with nothing to mirror it
    turns black; this one keeps part of its colour as plain diffuse light and
    tints its highlight, so a single lamp is enough."""
    material = flat_material(name, base, roughness, metallic=0.65)
    shader = _shader(material)
    shader.inputs["Specular Tint"].default_value = color(base)
    return material


def glazed_ceramic(name="ceramic", base="#efeadf", roughness=0.32):
    """Glazed ceramic: a soft body colour under a clear, glossy glaze (a coat
    layer, glTF clearcoat), so a lamp leaves a small sharp highlight on it
    even with no environment map. `roughness` is the body under the glaze."""
    material = flat_material(name, base, roughness)
    shader = _shader(material)
    shader.inputs["Coat Weight"].default_value = 1.0
    shader.inputs["Coat Roughness"].default_value = 0.04
    return material


# ------------------------------------------------------------------ objects


def _link(obj):
    bpy.context.scene.collection.objects.link(obj)
    return obj


def _dress(obj, material, crisp):
    """Gives a new shape its material and its shading."""
    if material is not None:
        obj.data.materials.append(material)
    if crisp is not None:
        crisp_edges(obj, crisp)
    return obj


def _crisp_for(count, crisp=CRISP_ANGLE):
    """The crisp angle for a shape that is round in `count` steps: above the
    angle between two of its sides, so few steps still shade as one curve."""
    if crisp is None or count < 5:
        return crisp
    return max(crisp, min(360 / count + 10, 80))


def _from_bmesh(name, bm, material=None, crisp=CRISP_ANGLE):
    mesh = bpy.data.meshes.new(name)
    bm.to_mesh(mesh)
    bm.free()
    return _dress(_link(bpy.data.objects.new(name, mesh)), material, crisp)


def empty(name, location=(0.0, 0.0, 0.0), parent=None):
    """An empty: the root of a piece, or a pivot such as a hinge."""
    obj = _link(bpy.data.objects.new(name, None))
    obj.empty_display_size = 0.05
    obj.location = location
    if parent is not None:
        parent_to(obj, parent)
    return obj


def mesh_object(name, vertices, faces, material=None, crisp=None, outward=False):
    """A mesh object from raw geometry: `vertices` are (x, y, z) points and
    `faces` are tuples of vertex numbers, counter clockwise seen from outside.
    `outward` turns every face of a closed shape to point out, whatever way
    it was listed. The mesh is flat shaded; give `crisp` an angle in degrees
    to shade it smooth with the edges above that angle kept crisp."""
    points = [tuple(float(c) for c in v) for v in vertices]
    polygons = [tuple(int(i) for i in f) for f in faces]
    for polygon in polygons:
        if len(polygon) < 3 or len(set(polygon)) != len(polygon):
            raise ValueError(f"{name}: the face {polygon} needs three or more different vertices")
        if min(polygon) < 0 or max(polygon) >= len(points):
            raise ValueError(f"{name}: the face {polygon} names a vertex that is not there")
    mesh = bpy.data.meshes.new(name)
    mesh.from_pydata(points, [], polygons)
    mesh.validate()
    mesh.update()
    if outward:
        bm = bmesh.new()
        bm.from_mesh(mesh)
        bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
        bm.to_mesh(mesh)
        bm.free()
    return _dress(_link(bpy.data.objects.new(name, mesh)), material, crisp)


def box(name, size, location=(0.0, 0.0, 0.0), material=None, bevel=0.0, segments=3):
    """A box of `size` (x, y, z) standing on `location`. `bevel` rounds every
    edge by that radius in `segments` steps, with the flat faces kept flat."""
    sx, sy, sz = size
    bm = bmesh.new()
    bmesh.ops.create_cube(bm, size=1.0)
    for vert in bm.verts:
        vert.co = Vector((vert.co.x * sx, vert.co.y * sy, (vert.co.z + 0.5) * sz))
    obj = _from_bmesh(name, bm, material)
    obj.location = location
    return _round_edges(obj, min(bevel, min(size) * 0.49), segments)


def cylinder(
    name,
    radius,
    height,
    location=(0.0, 0.0, 0.0),
    material=None,
    segments=64,
    top_radius=None,
    bevel=0.0,
    caps=True,
):
    """An upright cylinder standing on `location`, or a cone when `top_radius`
    differs from `radius`. `bevel` rounds the rims."""
    bm = bmesh.new()
    bmesh.ops.create_cone(
        bm,
        cap_ends=caps,
        cap_tris=False,
        segments=segments,
        radius1=radius,
        radius2=radius if top_radius is None else top_radius,
        depth=height,
    )
    bmesh.ops.translate(bm, verts=bm.verts, vec=(0.0, 0.0, height / 2))
    obj = _from_bmesh(name, bm, material, _crisp_for(segments))
    obj.location = location
    return _round_edges(obj, bevel, 3, _crisp_for(segments))


def sphere(
    name, radius, location=(0.0, 0.0, 0.0), material=None, segments=32, rings=16, scale=(1, 1, 1)
):
    """A sphere with its centre at `location`. `scale` squashes or stretches
    it into an ellipsoid along x, y and z."""
    bm = bmesh.new()
    bmesh.ops.create_uvsphere(bm, u_segments=segments, v_segments=rings, radius=radius)
    for vert in bm.verts:
        vert.co = Vector((vert.co.x * scale[0], vert.co.y * scale[1], vert.co.z * scale[2]))
    obj = _from_bmesh(name, bm, material, 180.0)
    obj.location = location
    return obj


# ----------------------------------------------------- outlines and profiles


def rounded_rect(
    width, height, radius, segments=8, center=(0.0, 0.0), corners=(True, True, True, True)
):
    """The outline of a rectangle with rounded corners: 2D points, counter
    clockwise from the bottom left. `corners` says which are rounded (bottom
    left, bottom right, top right, top left); `segments` is the steps in each
    quarter turn."""
    radius = min(radius, width / 2, height / 2)
    cx, cy = center
    hx, hy = width / 2, height / 2
    turns = [(-hx, -hy, 180), (hx, -hy, 270), (hx, hy, 0), (-hx, hy, 90)]
    points = []
    for (x, y, start), rounded in zip(turns, corners):
        if not rounded or radius <= 0:
            points.append((cx + x, cy + y))
            continue
        ox = x + radius if x < 0 else x - radius
        oy = y + radius if y < 0 else y - radius
        for step in range(segments + 1):
            angle = math.radians(start + 90 * step / segments)
            points.append((cx + ox + radius * math.cos(angle), cy + oy + radius * math.sin(angle)))
    return points


def fillet(points, radius, segments=8, closed=False):
    """Rounds the corners of a line through `points` (2D or 3D) with arcs: a
    lathe profile gets its rounded foot and lip this way, and a tube's path
    its bends. `radius` is one number for every corner, or one per point (0
    leaves that corner sharp). `segments` is the steps in a quarter turn.
    Where two arcs want more of a side than it has, they share it, so a wall
    with two rounded corners close together ends in a full round lip.
    `closed` also rounds the corner between the last point and the first."""
    line = [Vector(p) for p in points]
    count = len(line)
    radii = list(radius) if isinstance(radius, (list, tuple)) else [radius] * count
    if len(radii) != count:
        raise ValueError(f"fillet() got {len(radii)} radii for {count} points")
    if count < 3:
        return [tuple(p) for p in line]

    # What each corner asks for: how far along its two sides the arc starts.
    corners = []
    for i, corner in enumerate(line):
        before = line[i - 1] - corner
        after = line[(i + 1) % count] - corner
        ends = not closed and i in (0, count - 1)
        if ends or radii[i] <= 0 or before.length < 1e-9 or after.length < 1e-9:
            corners.append(None)
            continue
        angle = before.angle(after)
        if angle < 1e-4 or math.pi - angle < 1e-4:
            corners.append(None)
            continue
        corners.append((before, after, angle, radii[i] / math.tan(angle / 2)))

    def share(i, other, side):
        """The reach corner `i` gets of the side it shares with corner `other`."""
        wanted = corners[i][3]
        rival = corners[other][3] if corners[other] is not None else 0.0
        return wanted if wanted + rival <= side else side * wanted / (wanted + rival)

    out = []
    for i, corner in enumerate(line):
        if corners[i] is None:
            out.append(corner)
            continue
        before, after, angle, _ = corners[i]
        reach = min(
            share(i, (i - 1) % count, before.length), share(i, (i + 1) % count, after.length)
        )
        turn = math.pi - angle
        start = corner + before.normalized() * reach
        end = corner + after.normalized() * reach
        middle = (before.normalized() + after.normalized()).normalized()
        centre = corner + middle * (reach / math.cos(angle / 2))
        a, b = start - centre, end - centre
        steps = max(1, round(segments * turn / (math.pi / 2)))
        for step in range(steps + 1):
            t = step / steps
            out.append(
                centre + (a * math.sin((1 - t) * turn) + b * math.sin(t * turn)) / math.sin(turn)
            )
    return [tuple(p) for p in out]


def extrude(name, outline, depth, material=None, plane="XY", offset=0.0, crisp=CRISP_ANGLE):
    """A solid from a 2D `outline` pushed out by `depth`: a book cover, a
    bracket, a rounded slab. In the "XY" plane the outline lies flat at height
    `offset` and rises along z; in "XZ" it stands at y = `offset` and runs
    along y; in "YZ" it stands at x = `offset` and runs along x. A rounded
    part of the outline shades smooth and the two end faces stay crisp."""
    place = {
        "XY": lambda a, b, c: (a, b, c),
        "XZ": lambda a, b, c: (a, c, b),
        "YZ": lambda a, b, c: (c, a, b),
    }.get(plane)
    if place is None:
        raise ValueError(f"{name}: the plane is XY, XZ or YZ, not {plane!r}")
    bm = bmesh.new()
    face = bm.faces.new([bm.verts.new(place(a, b, offset)) for a, b in outline])
    made = bmesh.ops.extrude_face_region(bm, geom=[face])["geom"]
    lid = [item for item in made if isinstance(item, bmesh.types.BMVert)]
    bmesh.ops.translate(bm, verts=lid, vec=place(0.0, 0.0, depth))
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    return _from_bmesh(name, bm, material, crisp)


def lathe(
    name, profile, material=None, segments=64, location=(0.0, 0.0, 0.0), crisp=CRISP_ANGLE
):
    """A body turned on a lathe: the `profile`, a line of (radius, height)
    points, spun once around the z axis. A mug is one line from the centre of
    its base, out and up the outside, over the lip and down the inside to the
    centre of its floor; round the foot and the lip with fillet() first. A
    point with radius 0 sits on the axis, and a profile that starts and ends
    there makes a closed solid. `location` is where the axis meets height 0."""
    line = []
    for radius, height in profile:
        point = (float(radius) if radius > 1e-7 else 0.0, float(height))
        if not line or math.dist(point, line[-1]) > 1e-7:
            line.append(point)
    if len(line) < 2:
        raise ValueError(f"{name}: a lathe profile needs two or more different points")
    bm = bmesh.new()
    rings = []
    for radius, height in line:
        if radius == 0.0:
            rings.append([bm.verts.new((0.0, 0.0, height))])
            continue
        ring = []
        for step in range(segments):
            angle = 2 * math.pi * step / segments
            ring.append(bm.verts.new((radius * math.cos(angle), radius * math.sin(angle), height)))
        rings.append(ring)
    for low, high in zip(rings, rings[1:]):
        if len(low) == 1 and len(high) == 1:
            continue
        for step in range(segments):
            after = (step + 1) % segments
            if len(low) == 1:
                bm.faces.new((low[0], high[after], high[step]))
            elif len(high) == 1:
                bm.faces.new((low[step], low[after], high[0]))
            else:
                bm.faces.new((low[step], low[after], high[after], high[step]))
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    obj = _from_bmesh(name, bm, material, _crisp_for(segments, crisp))
    obj.location = location
    return obj


def tube(name, path, radius, material=None, sides=16, closed=False, caps=True):
    """A round tube of `radius` along the 3D points of `path`: a handle, a
    cable, a wire ring. Round the bends with fillet() first. `closed` joins
    the last point to the first (a ring); an open tube gets flat end `caps`."""
    line = []
    for point in path:
        point = Vector(point).to_3d()
        if not line or (point - line[-1]).length > 1e-9:
            line.append(point)
    if closed and len(line) > 2 and (line[0] - line[-1]).length < 1e-9:
        line.pop()
    count = len(line)
    if count < 2 or (closed and count < 3):
        raise ValueError(f"{name}: a tube needs a path of two or more different points")

    # At each point: the way the tube runs, the way it bends, and how much
    # wider the ring is across the bend so the tube stays round on both sides.
    runs = []
    for i, point in enumerate(line):
        ahead = (line[(i + 1) % count] - point).normalized() if closed or i < count - 1 else None
        behind = (point - line[i - 1]).normalized() if closed or i > 0 else None
        if ahead is None:
            ahead = behind
        if behind is None:
            behind = ahead
        run = ahead + behind
        run = run.normalized() if run.length > 1e-9 else ahead
        bend = ahead - behind
        bend = bend.normalized() if bend.length > 1e-9 else None
        runs.append((run, bend, 1 / max(run.dot(ahead), 0.25) - 1))

    # A side direction carried along the path, so the tube never twists.
    side = runs[0][0].orthogonal().normalized()
    carried = [side]
    for (before, _, _), (after, _, _) in zip(runs, runs[1:]):
        side = (before.rotation_difference(after) @ side).normalized()
        carried.append(side)
    twist = 0.0
    if closed:
        # What is left over after one lap is spread along the ring.
        first = runs[0][0]
        back = (runs[-1][0].rotation_difference(first) @ side).normalized()
        twist = math.atan2(first.dot(back.cross(carried[0])), back.dot(carried[0]))

    bm = bmesh.new()
    rings = []
    for i, (point, (run, bend, stretch)) in enumerate(zip(line, runs)):
        across = Matrix.Rotation(twist * i / count, 3, run) @ carried[i]
        up = run.cross(across)
        ring = []
        for step in range(sides):
            angle = 2 * math.pi * step / sides
            spoke = across * math.cos(angle) + up * math.sin(angle)
            if bend is not None:
                spoke += bend * (spoke.dot(bend) * stretch)
            ring.append(bm.verts.new(point + spoke * radius))
        rings.append(ring)
    pairs = list(zip(rings, rings[1:])) + ([(rings[-1], rings[0])] if closed else [])
    for low, high in pairs:
        for step in range(sides):
            after = (step + 1) % sides
            bm.faces.new((low[step], low[after], high[after], high[step]))
    if caps and not closed:
        bm.faces.new(list(reversed(rings[0])))
        bm.faces.new(rings[-1])
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    return _from_bmesh(name, bm, material, _crisp_for(sides))


# The corners of a page block, counter clockwise from the left front, and for
# each spine the side it is on (the corner that side starts at) and which
# corners are away from it, so they may be round.
_PAGE_SPINES = {
    "-X": (3, (False, True, True, False)),
    "+X": (1, (True, False, False, True)),
    "-Y": (0, (False, False, True, True)),
    "+Y": (2, (True, True, False, False)),
    None: (None, (True, True, True, True)),
}
# The share of the distance from one page line to the next that the cut takes.
_PAGE_CUT = 0.2


def page_block(
    name,
    size,
    location=(0.0, 0.0, 0.0),
    material=None,
    lines=None,
    depth=0.0005,
    spine="-X",
    corner_radius=0.0,
    line_material=None,
):
    """The pages of a book or a notebook lying flat: a block of `size` (x, y,
    thickness) whose edges carry fine, evenly spaced page lines as real
    geometry, so they hold up close and in a baked light. `lines` is how many
    (one for about every 2 mm of thickness when left out) and `depth` how
    far each is cut in. The side named by `spine` ("-X", "+X", "-Y", "+Y", or
    None for a loose stack) is where the pages are bound: it stays plain.
    `corner_radius` rounds the corners away from the spine, as a notebook's.
    `line_material` colours the cut of each line; without it the lines read
    by their shading alone."""
    width, length, thickness = size
    if spine not in _PAGE_SPINES:
        raise ValueError(f"{name}: the spine is -X, +X, -Y, +Y or None, not {spine!r}")
    if lines is None:
        lines = max(3, min(32, round(thickness / 0.002)))
    lines = max(0, int(lines))
    bound, rounded = _PAGE_SPINES[spine]
    half_x, half_y = width / 2, length / 2
    full = [(-half_x, -half_y), (half_x, -half_y), (half_x, half_y), (-half_x, half_y)]
    steps = 4

    def outline(inset):
        """The block's outline pulled in by `inset` on every side but the spine."""
        left = -half_x + (0.0 if spine == "-X" else inset)
        right = half_x - (0.0 if spine == "+X" else inset)
        front = -half_y + (0.0 if spine == "-Y" else inset)
        back = half_y - (0.0 if spine == "+Y" else inset)
        turns = [(left, front, 180), (right, front, 270), (right, back, 0), (left, back, 90)]
        points = []
        for corner, (x, y, start) in enumerate(turns):
            if rounded[corner] and corner_radius > 0:
                # The same count of points at every inset, so the rings line up.
                radius = max(corner_radius - inset, 0.0)
                ox = x + radius if x == left else x - radius
                oy = y + radius if y == front else y - radius
                for step in range(steps + 1):
                    angle = math.radians(start + 90 * step / steps)
                    points.append((ox + radius * math.cos(angle), oy + radius * math.sin(angle)))
            else:
                points.append((x, y))
            if corner == bound and lines:
                # Where the cuts of the two sides next to the spine come out on
                # it: every ring has these points, so the spine side has no gap.
                (ax, ay), (bx, by) = full[corner], full[(corner + 1) % 4]
                along = Vector((bx - ax, by - ay)).normalized() * depth
                points.append((ax + along.x, ay + along.y))
                points.append((bx - along.x, by - along.y))
        return points

    # Heights, bottom to top: bundles of pages with a thin cut between each two.
    pitch = thickness / (lines + 1 - _PAGE_CUT)
    cut = pitch * _PAGE_CUT
    levels = [(0.0, 0.0)]
    for line in range(lines):
        low = (line + 1) * pitch - cut
        levels += [(low, 0.0), (low, depth), (low + cut, depth), (low + cut, 0.0)]
    levels.append((thickness, 0.0))

    bm = bmesh.new()
    placed = {}

    def vert_at(x, y, z):
        key = (round(x, 7), round(y, 7), round(z, 7))
        if key not in placed:
            placed[key] = bm.verts.new((x, y, z))
        return placed[key]

    def face(verts, material_index=0):
        """A face through `verts`, skipping repeats: a step has no width at the spine."""
        unique = list(dict.fromkeys(verts))
        if len(unique) >= 3:
            bm.faces.new(unique).material_index = material_index

    rings = [[vert_at(x, y, z) for x, y in outline(inset)] for z, inset in levels]
    face(list(reversed(rings[0])))
    face(rings[-1])
    for index, (low, high) in enumerate(zip(rings, rings[1:])):
        in_cut = levels[index][1] > 0 and levels[index + 1][1] > 0
        for step in range(len(low)):
            after = (step + 1) % len(low)
            face(
                (low[step], low[after], high[after], high[step]),
                1 if in_cut and line_material is not None else 0,
            )
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    obj = _from_bmesh(name, bm)
    if material is not None or line_material is not None:
        obj.data.materials.append(material)
    if line_material is not None:
        obj.data.materials.append(line_material)
    obj.location = location
    return obj


# ------------------------------------------------------- shading and cutting


def crisp_edges(obj, angle=CRISP_ANGLE):
    """Shades the object smooth and keeps every edge whose faces meet at more
    than `angle` degrees crisp, with no bevel: a rounded spine or a turned
    foot shades as one soft surface while the ends of a cover stay sharp.
    Along a smooth run the shading leans toward the larger faces, so a flat
    face stays flat right up to the curve next to it. No geometry is added,
    so no thin bevel faces are left to show as hairlines along a curve.
    Modifiers still on the object become part of its mesh first."""
    _apply_modifiers(obj)
    mesh = obj.data
    if "custom_normal" in mesh.attributes:
        mesh.attributes.remove(mesh.attributes["custom_normal"])
    mesh.shade_smooth()
    mesh.set_sharp_from_angle(angle=math.radians(angle))
    modifier = obj.modifiers.new("flat faces", "WEIGHTED_NORMAL")
    modifier.mode = "FACE_AREA"
    modifier.keep_sharp = True
    return _apply_modifiers(obj)


def _apply_modifiers(obj):
    """Makes the object's modifiers part of its mesh."""
    if not obj.modifiers:
        return obj
    depsgraph = bpy.context.evaluated_depsgraph_get()
    mesh = bpy.data.meshes.new_from_object(
        obj.evaluated_get(depsgraph), preserve_all_data_layers=True, depsgraph=depsgraph
    )
    old = obj.data
    name = old.name
    obj.modifiers.clear()
    obj.data = mesh
    if old.users == 0:
        bpy.data.meshes.remove(old)
    mesh.name = name
    return obj


def _round_edges(obj, width, segments, angle=CRISP_ANGLE):
    if width <= 0:
        return obj
    modifier = obj.modifiers.new("bevel", "BEVEL")
    modifier.width = width
    modifier.segments = segments
    modifier.limit_method = "ANGLE"
    modifier.angle_limit = math.radians(angle)
    modifier.profile = 0.5
    modifier.harden_normals = True
    modifier.miter_outer = "MITER_ARC"
    modifier.use_clamp_overlap = True
    return _apply_modifiers(obj)


def bevel(obj, width, segments=3, angle=CRISP_ANGLE):
    """Rounds every edge sharper than `angle` degrees by `width`, in
    `segments` steps, and keeps the flat faces flat. Meant for boxes and
    rims; along the end of a rounded outline it leaves thin faces, where
    crisp_edges() is the cleaner choice."""
    return _round_edges(obj, width, segments, angle)


def boolean(target, cutter, operation="DIFFERENCE", keep_cutter=False, crisp=CRISP_ANGLE):
    """Cuts `cutter` out of `target` ("DIFFERENCE"), adds it ("UNION") or
    keeps what they share ("INTERSECT"), then removes the cutter unless
    `keep_cutter`. The new faces take the target's first material, and the
    result is shaded again with its edges above `crisp` degrees kept crisp."""
    bpy.context.view_layer.update()
    modifier = target.modifiers.new("boolean", "BOOLEAN")
    modifier.operation = operation
    modifier.object = cutter
    modifier.solver = "EXACT"
    _apply_modifiers(target)
    if not keep_cutter:
        mesh = cutter.data
        bpy.data.objects.remove(cutter)
        if mesh.users == 0:
            bpy.data.meshes.remove(mesh)
    _drop_spare_slots(target.data)
    if crisp is not None:
        crisp_edges(target, crisp)
    return target


# ---------------------------------------------------------- putting together


def _used_slots(mesh):
    """The numbers of the material slots the mesh's faces use."""
    indices = [0] * len(mesh.polygons)
    mesh.polygons.foreach_get("material_index", indices)
    return set(indices)


def _drop_spare_slots(mesh):
    """Removes the empty material slots no face uses, as a boolean leaves one."""
    used = _used_slots(mesh)
    for index in reversed(range(len(mesh.materials))):
        if mesh.materials[index] is None and index not in used:
            mesh.materials.pop(index=index)


def parent_to(obj, parent):
    """Parents `obj` under `parent` and leaves it standing where it is."""
    bpy.context.view_layer.update()
    world = obj.matrix_world.copy()
    obj.parent = parent
    obj.matrix_parent_inverse = parent.matrix_world.inverted()
    obj.matrix_world = world
    return obj


def join(objects, name=None):
    """Joins mesh objects into the first one and returns it, renamed to
    `name` when given. The result keeps the first object's origin and parent,
    every face stays where it stands, and every face keeps its own material:
    the materials of all the pieces become the slots of the result, each
    once, in the order they first appear. Faces of a piece with no material
    stay without one."""
    pieces = [obj for obj in objects if obj is not None]
    if not pieces:
        raise ValueError("join() needs one or more objects")
    for piece in pieces:
        if piece.type != "MESH":
            raise ValueError(f"join() takes mesh objects, and {piece.name} is not one")
    first = pieces[0]
    bpy.context.view_layer.update()
    depsgraph = bpy.context.evaluated_depsgraph_get()
    to_first = first.matrix_world.inverted()
    slots = []
    bm = bmesh.new()
    for piece in pieces:
        mesh = bpy.data.meshes.new_from_object(
            piece.evaluated_get(depsgraph), preserve_all_data_layers=True, depsgraph=depsgraph
        )
        matrix = to_first @ piece.matrix_world
        mesh.transform(matrix)
        if matrix.determinant() < 0:
            # A mirrored piece: turn its faces to point out again. The fine
            # shading does not survive the turn, the crisp edges do.
            mesh.flip_normals()
            if "custom_normal" in mesh.attributes:
                mesh.attributes.remove(mesh.attributes["custom_normal"])
        # Where each of this piece's slots lands in the joined mesh. An empty
        # slot only counts when a face uses it.
        own = list(mesh.materials) or [None]
        used = {min(index, len(own) - 1) for index in _used_slots(mesh)}
        lands = {}
        for index, material in enumerate(own):
            if material is None and index not in used:
                continue
            if material not in slots:
                slots.append(material)
            lands[index] = slots.index(material)
        before = len(bm.faces)
        bm.from_mesh(mesh)
        bm.faces.ensure_lookup_table()
        for face in bm.faces[before:]:
            face.material_index = lands[min(face.material_index, len(own) - 1)]
        bpy.data.meshes.remove(mesh)
    joined = bpy.data.meshes.new(name or first.data.name)
    bm.to_mesh(joined)
    bm.free()
    for material in slots:
        joined.materials.append(material)

    old = first.data
    first.modifiers.clear()
    first.data = joined
    if old.users == 0:
        bpy.data.meshes.remove(old)
    for piece in pieces[1:]:
        for child in list(piece.children):
            parent_to(child, first)
        mesh = piece.data
        bpy.data.objects.remove(piece)
        if mesh.users == 0:
            bpy.data.meshes.remove(mesh)
    if name:
        first.name = name
        joined.name = name
    return first


def finish(obj, parent=None, material=None):
    """The last step for a piece: makes its modifiers part of the mesh and
    parents it under `parent`, standing where it is. Its material slots and
    the slot of every face stay exactly as they are. `material` only fills in
    for faces that have no material yet."""
    _apply_modifiers(obj)
    if material is not None and obj.type == "MESH":
        mesh = obj.data
        if len(mesh.materials) == 0:
            mesh.materials.append(material)
        used = _used_slots(mesh)
        for index, slot in enumerate(mesh.materials):
            if slot is None and index in used:
                mesh.materials[index] = material
    if parent is not None:
        parent_to(obj, parent)
    return obj


def _family(root):
    """`root` and everything parented under it."""
    return [root] + list(root.children_recursive)


def bounds(root):
    """The lowest and highest corner, in world space, of the meshes of
    `root` and everything under it."""
    bpy.context.view_layer.update()
    low = Vector((math.inf, math.inf, math.inf))
    high = -low
    for obj in _family(root):
        if obj.type != "MESH":
            continue
        for vert in obj.data.vertices:
            point = obj.matrix_world @ vert.co
            low = Vector(map(min, low, point))
            high = Vector(map(max, high, point))
    if low.x == math.inf:
        raise ValueError(f"{root.name} has no mesh to measure")
    return low, high


def set_origin(obj, point):
    """Moves a mesh object's origin to the world space `point` without moving
    its geometry: the pivot of a lid, the centre a wheel turns on."""
    bpy.context.view_layer.update()
    world = obj.matrix_world.copy()
    local = world.inverted() @ Vector(point)
    obj.data.transform(Matrix.Translation(-local))
    obj.matrix_world = world @ Matrix.Translation(local)
    for child in obj.children:
        child.matrix_parent_inverse = Matrix.Translation(-local) @ child.matrix_parent_inverse
    return obj


def ground(root):
    """Stands the piece on its root: everything under `root` (and its own
    mesh) moves so the bottom centre of the whole piece is at the root's
    origin. With the root at the world origin, that is where Maquette expects
    a model's origin, so the piece neither floats nor sinks when placed."""
    low, high = bounds(root)
    target = root.matrix_world.translation
    shift = target - Vector(((low.x + high.x) / 2, (low.y + high.y) / 2, low.z))
    if shift.length < 1e-9:
        return root
    if root.type == "MESH":
        root.data.transform(Matrix.Translation(root.matrix_world.to_3x3().inverted() @ shift))
    for child in root.children:
        child.matrix_world = Matrix.Translation(shift) @ child.matrix_world
    bpy.context.view_layer.update()
    return root


def triangles(root):
    """How many triangles the meshes of `root` and everything under it make."""
    return sum(
        sum(len(polygon.vertices) - 2 for polygon in obj.data.polygons)
        for obj in _family(root)
        if obj.type == "MESH"
    )
