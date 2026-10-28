"""Small Frostveil props with parts, for the Parts tests and the mock UI.
Run headless:

    blender -b --factory-startup --python parts_models.py -- <which> <out.glb>

<which> is one of:
- lantern: a frost lantern as one mesh, the way a generator sends it: a
  base, a post, a cage, a crystal and a lid as loose pieces, plus four tiny
  bits on the cage. Four materials (the lid and the post share "stone"),
  the stone one with a small image texture.
- lantern-parts: the same lantern cut into five objects, part_1 to part_5
  top first (lid, cage, crystal, post, base), as the split fix-up leaves it.
- lantern-tripo: the five parts named tripo_part_0 to tripo_part_4 and no
  texture, as a provider split returns them.
- chest: a rune chest as one mesh, its body and its lid.
- gate-frame, gate-door, gate-steps: the three parts of a sigil gate, each
  made as its own model (the door leaf with its origin on its left hinge).
"""

import sys

import bmesh
import bpy
from mathutils import Matrix, Vector


def reset():
    bpy.ops.wm.read_factory_settings(use_empty=True)


def material(name, rgb, textured=False):
    mat = bpy.data.materials.get(name)
    if mat is not None:
        return mat
    mat = bpy.data.materials.new(name)
    mat.use_nodes = True
    bsdf = mat.node_tree.nodes.get("Principled BSDF")
    bsdf.inputs["Base Color"].default_value = (*rgb, 1.0)
    if textured:
        image = bpy.data.images.new(f"{name}_color", 16, 16)
        image.pixels = [c for i in range(256) for c in ((*rgb, 1.0) if i % 3 else (0.9, 0.9, 1.0, 1.0))]
        image.pack()
        node = mat.node_tree.nodes.new("ShaderNodeTexImage")
        node.image = image
        mat.node_tree.links.new(node.outputs["Color"], bsdf.inputs["Base Color"])
    return mat


def piece(bm, kind, size, at, slot):
    """Adds one closed shape to `bm` at `at` with material slot `slot`."""
    matrix = Matrix.Translation(Vector(at))
    if kind == "box":
        made = bmesh.ops.create_cube(bm, size=1.0, matrix=matrix @ Matrix.Diagonal((*size, 1.0)), calc_uvs=True)
    elif kind == "cone":
        made = bmesh.ops.create_cone(
            bm, cap_ends=True, segments=8, radius1=size[0], radius2=size[1], depth=size[2], matrix=matrix, calc_uvs=True
        )
    else:
        made = bmesh.ops.create_cone(
            bm, cap_ends=False, segments=8, radius1=size[0], radius2=size[0], depth=size[2], matrix=matrix, calc_uvs=True
        )
    faces = {f for v in made["verts"] for f in v.link_faces}
    for face in faces:
        face.material_index = slot


def build(name, materials, pieces):
    """One mesh object from (kind, size, at, material) pieces."""
    mesh = bpy.data.meshes.new(name)
    for mat in materials:
        mesh.materials.append(mat)
    bm = bmesh.new()
    bm.loops.layers.uv.verify()
    for kind, size, at, slot in pieces:
        piece(bm, kind, size, at, slot)
    bm.to_mesh(mesh)
    bm.free()
    obj = bpy.data.objects.new(name, mesh)
    bpy.context.scene.collection.objects.link(obj)
    return obj


def lantern_pieces():
    """(name, kind, size, at, material slot), top first."""
    return [
        ("lid", "cone", (0.14, 0.02, 0.08), (0, 0, 0.6), 0),
        ("cage", "tube", (0.12, 0.12, 0.2), (0, 0, 0.47), 1),
        ("crystal", "cone", (0.05, 0.0, 0.14), (0, 0, 0.44), 2),
        ("post", "tube", (0.04, 0.04, 0.3), (0, 0, 0.21), 0),
        ("base", "box", (0.3, 0.3, 0.06), (0, 0, 0.03), 3),
    ]


def lantern_materials(textured=True):
    return [
        material("stone", (0.42, 0.45, 0.5), textured),
        material("iron", (0.16, 0.17, 0.2)),
        material("crystal_glow", (0.55, 0.8, 1.0)),
        material("granite", (0.3, 0.31, 0.34)),
    ]


BITS = [(0.125, 0.0, 0.52), (-0.125, 0.0, 0.52), (0.0, 0.125, 0.42), (0.0, -0.125, 0.42)]


def lantern():
    pieces = [(k, s, a, m) for _, k, s, a, m in lantern_pieces()]
    pieces += [("box", (0.003, 0.003, 0.003), at, 1) for at in BITS]
    build("frost_lantern", lantern_materials(), pieces)


def lantern_split(names, textured):
    mats = lantern_materials(textured)
    for (_, kind, size, at, slot), name in zip(lantern_pieces(), names):
        obj = build(name, [mats[slot]], [(kind, size, (0, 0, 0), 0)])
        obj.location = at


def chest():
    mats = [material("rune_wood", (0.35, 0.22, 0.12), True), material("frost_iron", (0.5, 0.6, 0.7))]
    build(
        "rune_chest",
        mats,
        [("box", (0.6, 0.4, 0.3), (0, 0, 0.15), 0), ("box", (0.62, 0.42, 0.08), (0, 0, 0.36), 1)],
    )


def gate(which):
    stone = material("veil_stone", (0.4, 0.43, 0.52), True)
    rune = material("sigil_glow", (0.5, 0.75, 1.0))
    if which == "gate-frame":
        build(
            "sigil_gate_frame",
            [stone],
            [
                ("box", (0.3, 0.4, 2.4), (-0.75, 0, 1.2), 0),
                ("box", (0.3, 0.4, 2.4), (0.75, 0, 1.2), 0),
                ("box", (1.8, 0.4, 0.3), (0, 0, 2.55), 0),
            ],
        )
    elif which == "gate-door":
        # The origin sits on the left hinge, so the leaf swings around it.
        build("sigil_gate_door", [stone, rune], [("box", (1.2, 0.1, 2.3), (0.6, 0, 1.15), 0), ("cone", (0.2, 0.2, 0.02), (0.6, -0.06, 1.4), 1)])
    else:
        build(
            "sigil_gate_steps",
            [stone],
            [("box", (2.0, 0.6, 0.15), (0, -0.4, 0.075), 0), ("box", (1.8, 0.4, 0.15), (0, -0.3, 0.225), 0)],
        )


def main():
    argv = sys.argv[sys.argv.index("--") + 1 :]
    which, out = argv[0], argv[1]
    reset()
    if which == "lantern":
        lantern()
    elif which == "lantern-parts":
        lantern_split([f"part_{i}" for i in range(1, 6)], True)
    elif which == "lantern-tripo":
        lantern_split([f"tripo_part_{i}" for i in range(5)], False)
    elif which == "chest":
        chest()
    elif which.startswith("gate-"):
        gate(which)
    else:
        raise SystemExit(f"unknown model {which}")
    bpy.ops.export_scene.gltf(filepath=out, export_format="GLB")


main()
