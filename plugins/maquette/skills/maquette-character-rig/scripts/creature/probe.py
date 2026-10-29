# probe.py: dump objects, armature bones (world head/tail, parent), vertex-group stats,
# influence counts, zero-weight verts, connected islands and per-island dominant bones.
# Usage: Blender -b --factory-startup -P probe.py -- in.glb out.json
import bpy, sys, json
from mathutils import Vector
from collections import Counter, defaultdict
glb, outj = sys.argv[sys.argv.index("--")+1:]
bpy.ops.wm.read_factory_settings(use_empty=True)
bpy.ops.import_scene.gltf(filepath=glb)
for o in bpy.data.objects:
    print("OBJ", o.name, o.type, o.parent and o.parent.name, tuple(round(x,3) for x in o.location), tuple(round(x,3) for x in o.scale), len(o.vertex_groups) if o.type=="MESH" else "", len(o.data.vertices) if o.type=="MESH" else "")
arm = next(o for o in bpy.data.objects if o.type == "ARMATURE")
me = next(o for o in bpy.data.objects if o.type == "MESH" and o.vertex_groups)
print("MODS", [(m.type, getattr(m,'object',None) and m.object.name) for m in me.modifiers])
mw = arm.matrix_world
bones = {}
for b in arm.data.bones:
    bones[b.name] = dict(head=list(mw @ b.head_local), tail=list(mw @ b.tail_local), parent=b.parent.name if b.parent else None, children=[c.name for c in b.children])
mesh = me.data
gname = {g.index: g.name for g in me.vertex_groups}
cnt = Counter(); wsum = Counter(); infl = Counter(); zero = 0; over4 = 0
dom = []
for v in mesh.vertices:
    gs = [(g.group, g.weight) for g in v.groups if g.weight > 1e-4]
    infl[len(gs)] += 1
    if not gs: zero += 1; dom.append(None); continue
    if len(gs) > 4: over4 += 1
    for gi, w in gs: cnt[gname[gi]] += 1; wsum[gname[gi]] += w
    dom.append(gname[max(gs, key=lambda x: x[1])[0]])
# islands
parent = list(range(len(mesh.vertices)))
def find(a):
    while parent[a] != a: parent[a] = parent[parent[a]]; a = parent[a]
    return a
for e in mesh.edges:
    a, b = find(e.vertices[0]), find(e.vertices[1])
    if a != b: parent[a] = b
isl = defaultdict(list)
for i in range(len(mesh.vertices)): isl[find(i)].append(i)
print("VERTS", len(mesh.vertices), "FACES", len(mesh.polygons), "ISLANDS", len(isl), "ZERO", zero, "OVER4", over4, "INFL", sorted(infl.items()))
print("MATS", [m.name for m in mesh.materials], "UV", [u.name for u in mesh.uv_layers])
for im in bpy.data.images: print("IMG", im.name, im.size[:], im.file_format)
domcount = Counter(d for d in dom if d)
isl_info = []
for k, vs in sorted(isl.items(), key=lambda kv: -len(kv[1])):
    c = Counter(dom[i] for i in vs)
    co = [me.matrix_world @ mesh.vertices[i].co for i in vs]
    ctr = sum(co, Vector()) / len(co)
    isl_info.append(dict(n=len(vs), ctr=list(ctr), bones=c.most_common(5)))
for b in sorted(bones, key=lambda n: (n.startswith("bone_"), int(n.split("_")[1]) if n.startswith("bone_") else 0, n)):
    d = bones[b]
    print(f"BONE {b:24s} par={str(d['parent']):24s} h={tuple(round(x,3) for x in d['head'])} t={tuple(round(x,3) for x in d['tail'])} nv={cnt[b]} dom={domcount[b]} wsum={wsum[b]:.0f}")
print("ISLANDS top 40:")
for i in isl_info[:40]: print(" ", i['n'], tuple(round(x,3) for x in i['ctr']), i['bones'])
json.dump(dict(bones=bones, cnt=cnt, domcount=domcount, islands=isl_info), open(outj, "w"))
