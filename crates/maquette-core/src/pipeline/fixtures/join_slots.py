"""Test fixture: joins pieces whose material slots come in different orders
and prints which material the faces of every piece have, before and after.

Five boxes stand in a row. The first two bring one material each, the third
brings the same two in the other order (its caps blue, its sides red), the
fourth has no material and the fifth is mirrored. join() has to keep every
face on its own material and facing out; finish() must change no slot.
"""

import json
import os
import sys

import bpy

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import maquette_helpers as mq  # noqa: E402

PIECES = ["red", "blue", "both", "bare", "mirrored"]
GAP = 0.2

red = mq.flat_material("red", "#c0392b")
blue = mq.flat_material("blue", "#2e6fb5")
green = mq.flat_material("green", "#3c9a5f")
fill = mq.flat_material("fill", "#f2f2f2")


def piece(name, material=None):
    return mq.box(name, (0.1, 0.1, 0.1), (GAP * PIECES.index(name), 0, 0), material)


def faces(obj):
    """The materials of the caps and of the sides of every piece of `obj`."""
    found = {}
    for polygon in obj.data.polygons:
        place = obj.matrix_world @ polygon.center
        which = PIECES[round(place.x / GAP)]
        key = f"{which} {'cap' if abs(polygon.normal.z) > 0.5 else 'side'}"
        slots = obj.data.materials
        material = slots[polygon.material_index] if len(slots) else None
        found.setdefault(key, set()).add(material.name if material else None)
    return {key: sorted(names, key=str) for key, names in sorted(found.items())}


def outward(obj):
    """True when every face points away from the middle of its piece."""
    for polygon in obj.data.polygons:
        middle = (GAP * round(polygon.center.x / GAP), 0.0, 0.05)
        away = [c - m for c, m in zip(polygon.center, middle)]
        if sum(a * n for a, n in zip(away, polygon.normal)) <= 0:
            return False
    return True


both = piece("both", blue)
both.data.materials.append(red)
for polygon in both.data.polygons:
    polygon.material_index = 0 if abs(polygon.normal.z) > 0.5 else 1
mirrored = piece("mirrored", green)
mirrored.scale = (-1, 1, 1)
pieces = [piece("red", red), piece("blue", blue), both, piece("bare"), mirrored]

# Where each piece stands is only known once the scene has caught up.
bpy.context.view_layer.update()
before = {}
for obj in pieces:
    before.update(faces(obj))
joined = mq.join(pieces, "joined")
said = {
    "before": before,
    "joined": faces(joined),
    "slots": [m.name if m else None for m in joined.data.materials],
    "outward": outward(joined),
}
mq.finish(joined)
said["untouchedSlots"] = [m.name if m else None for m in joined.data.materials]
mq.finish(joined, material=fill)
said["finished"] = faces(joined)
said["finishedSlots"] = [m.name if m else None for m in joined.data.materials]
print("SLOTS " + json.dumps(said))
