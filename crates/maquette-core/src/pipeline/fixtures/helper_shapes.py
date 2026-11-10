"""Test fixture: one of every shape the helpers make, in the variants that are
easy to get wrong, and what each mesh is like.

Each shape is measured as a mesh: the edges that do not join exactly two
faces ("open"), faces with next to no area ("flat"), edges kept crisp
("crisp"), whether its faces point out, where it stands and its material
slots.
"""

import json
import math
import os
import sys

import bmesh
import bpy

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import maquette_helpers as mq  # noqa: E402

mat = mq.palette({"oak": "#c9a06c", "paper": ("#f6f1e4", 0.9), "ink": "#3d3a36"})
found = {}
# A face under a thousandth of a square millimetre is a sliver, not a face:
# the smallest one any helper means to make is thirty times that.
SLIVER = 1e-9


def measure(obj):
    """What the mesh of `obj` is like."""
    bm = bmesh.new()
    bm.from_mesh(obj.data)
    low, high = mq.bounds(obj)
    found[obj.name] = {
        "open": sum(1 for edge in bm.edges if not edge.is_manifold),
        "flat": sum(1 for face in bm.faces if face.calc_area() < SLIVER),
        "crisp": sum(1 for edge in bm.edges if not edge.smooth),
        "outward": bm.calc_volume(signed=True) > 0,
        "low": [round(v, 5) for v in low],
        "high": [round(v, 5) for v in high],
        "slots": [slot.name if slot else None for slot in obj.data.materials],
        "triangles": mq.triangles(obj),
    }
    bm.free()
    return obj


def circle(radius, steps, height, wave=0.0):
    """Points around a circle, with a wave of that height along it."""
    points = []
    for i in range(steps):
        angle = 2 * math.pi * i / steps
        lift = height + wave * math.sin(3 * angle)
        points.append((radius * math.cos(angle), radius * math.sin(angle), lift))
    return points


# Boxes and round shapes: each stands on its location.
measure(mq.box("box", (0.2, 0.1, 0.05), (0.3, 0, 0), mat["oak"]))
measure(mq.box("box_bevel", (0.2, 0.1, 0.05), (0.3, 0, 0), mat["oak"], bevel=0.005))
measure(mq.cylinder("cylinder_12", 0.03, 0.1, (0, 0, 0.2), mat["oak"], segments=12))
measure(mq.cylinder("cylinder_bevel", 0.03, 0.1, material=mat["oak"], bevel=0.004))
measure(mq.cylinder("cone", 0.03, 0.1, material=mat["oak"], top_radius=0.0))
measure(mq.sphere("sphere_8", 0.03, (0, 0, 0.03), mat["oak"], segments=8, rings=4))

# Outlines pushed out along each axis.
outline = mq.rounded_rect(0.2, 0.1, 0.02, corners=(True, False, False, True))
for plane in ("XY", "XZ", "YZ"):
    measure(mq.extrude(f"extrude_{plane}", outline, 0.3, mat["oak"], plane=plane, offset=0.5))

# A mug: one profile, closed at the axis, its two lip corners sharing the wall.
profile = [(0, 0), (0.04, 0), (0.04, 0.09), (0.035, 0.09), (0.035, 0.006), (0, 0.006)]
rounded = mq.fillet(profile, (0, 0.012, 0.004, 0.004, 0.008, 0))
found["lip"] = {
    "top": round(max(height for _, height in rounded), 6),
    "points": len(rounded),
    "left_sharp": len(mq.fillet(profile, 0)),
}
measure(mq.lathe("lathe", rounded, mat["paper"], location=(0, 0, 0.1)))
measure(mq.lathe("lathe_12", rounded, mat["paper"], segments=12))

# Tubes: open with caps, with a sharp turn, and closed into rings.
bends = mq.fillet([(0, 0, 0), (0, 0, 0.1), (0.1, 0, 0.1), (0.1, 0.1, 0.1)], 0.03)
measure(mq.tube("tube", bends, 0.005, mat["oak"]))
measure(mq.tube("tube_12", bends, 0.005, mat["oak"], sides=12))
measure(mq.tube("tube_turn", [(0, 0, 0), (0, 0, 0.1), (0.1, 0, 0.1)], 0.005, mat["oak"]))
measure(mq.tube("ring", circle(0.05, 32, 0.01), 0.004, mat["oak"], closed=True))
measure(mq.tube("ring_3d", circle(0.05, 48, 0.05, 0.02), 0.004, mat["oak"], closed=True))

# Page blocks: every side the spine can be on, a loose stack, rounded
# corners (one smaller than the cut), two materials and no lines at all.
size = (0.14, 0.2, 0.02)
for spine in ("-X", "+X", "-Y", "+Y"):
    measure(mq.page_block(f"pages_{spine}", size, (0, 0, 0.1), mat["paper"], spine=spine))
measure(mq.page_block("pages_loose", size, material=mat["paper"], spine=None))
measure(mq.page_block("pages_round", size, material=mat["paper"], corner_radius=0.008))
measure(mq.page_block("pages_tight", size, material=mat["paper"], spine=None, corner_radius=0.0003))
measure(mq.page_block("pages_two", size, material=mat["paper"], line_material=mat["ink"], lines=5))
measure(mq.page_block("pages_lines_only", size, line_material=mat["ink"], lines=5))
measure(mq.page_block("pages_plain", size, material=mat["paper"], lines=0))

# A raw mesh listed inside out, a cut and a bevel after it.
pyramid = [(0, 0, 0), (0.1, 0, 0), (0.1, 0.1, 0), (0, 0.1, 0), (0.05, 0.05, 0.08)]
inside_out = [(0, 1, 2, 3), (4, 1, 0), (4, 2, 1), (4, 3, 2), (4, 0, 3)]
measure(mq.mesh_object("raw", pyramid, inside_out, mat["oak"], outward=True))
cup = mq.box("cup", (0.08, 0.08, 0.1), material=mat["oak"], bevel=0.004)
mq.boolean(cup, mq.cylinder("cup_hole", 0.03, 0.2, (0, 0, 0.01)))
measure(cup)
found["cup"]["cutter_left"] = "cup_hole" in bpy.data.objects
tray = mq.box("tray", (0.2, 0.1, 0.02), material=mat["oak"])
mq.boolean(tray, mq.box("tray_hollow", (0.18, 0.08, 0.02), (0, 0, 0.01)))
measure(mq.bevel(tray, 0.002))

# Standing a piece on its root, and moving an origin without the mesh.
root = mq.empty("root", (1, 2, 0))
mq.parent_to(mq.box("high", (0.1, 0.1, 0.1), (1.3, 2.2, 0.5), mat["oak"]), root)
mq.parent_to(mq.box("low", (0.1, 0.1, 0.3), (1.0, 2.0, 0.2), mat["oak"]), root)
mq.ground(root)
low, high = mq.bounds(root)
found["grounded"] = {"low": [round(v, 5) for v in low], "high": [round(v, 5) for v in high]}
lid = mq.box("lid", (0.2, 0.1, 0.01), (0, 3, 0.1), mat["oak"])
mq.set_origin(lid, (0, 3.05, 0.1))
measure(lid)
found["lid"]["origin"] = [round(v, 5) for v in lid.matrix_world.translation]

print("SHAPES " + json.dumps(found))
