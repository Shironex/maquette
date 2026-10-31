# autoconfig.py: a first pass of rigconfig.py from the bone tree and bone positions, so nobody starts from a blank table.
# Reads where bones are, not what they are called: names lie on auto rigs. Legs reach the ground; of the other long
# chains the thickest is the head, it says which way the creature faces; short weighted branches off the trunk are spikes;
# short branches off a limb belong to that limb; bones with no or trivial weight down their whole branch are left out of every chain and stay in the file; a separate branch lying on top of a limb with no real weight of its own is junk. Chains that sit on the other side than their name says are marked "sides swapped". It also counts
# weight that sits far from its chain and nearer another (a tail bone on a paw) and sets STRAY_DIST for it.
# Check the result on the weightmap.py render before trusting it: this is a first pass, the renders decide.
# Usage: -- in.glb out_dir [edits.json]       writes out_dir/rigconfig.py and out_dir/rigconfig.json (the table for people)
# edits.json holds a person's corrections, by a chain's first bone: {"bone": {"readAs": "chain" or "junk", "side": "left" or "right"}}.
import bpy, sys, json, os, re, pprint
from collections import defaultdict
from mathutils import Vector
args = sys.argv[sys.argv.index("--") + 1:]
glb, outd = args[:2]
EDITS = json.load(open(args[2])) if len(args) > 2 else {}
bpy.ops.wm.read_factory_settings(use_empty=True)
bpy.ops.import_scene.gltf(filepath=glb)
arm = next(o for o in bpy.data.objects if o.type == "ARMATURE")
me = next(o for o in bpy.data.objects if o.type == "MESH" and o.vertex_groups)
mw = arm.matrix_world
HEAD = {b.name: mw @ b.head_local for b in arm.data.bones}
TAIL = {b.name: mw @ b.tail_local for b in arm.data.bones}
m = me.data
CO = [me.matrix_world @ v.co for v in m.vertices]
lo = Vector([min(c[i] for c in CO) for i in range(3)]); hi = Vector([max(c[i] for c in CO) for i in range(3)])
SIZE = max(hi - lo); UP = Vector((0, 0, 1))
gname = {g.index: g.name for g in me.vertex_groups}
WEIGHTS = [{gname[g.group]: g.weight for g in v.groups if g.weight > 1e-4} for v in m.vertices]
USED = {b for w in WEIGHTS for b in w}
TOTAL, MOST = defaultdict(float), defaultdict(float)
for w in WEIGHTS:
    for b, x in w.items():
        TOTAL[b] += x; MOST[b] = max(MOST[b], x)
# Trivial weight: nowhere a real share of a vertex, and next to nothing in all. A bone like that moves no surface.
TRIVIAL_MOST, TRIVIAL_TOTAL = 0.05, 0.0005 * max(len(WEIGHTS), 1)
def has_weight(name):
    return not USED or MOST[name] >= TRIVIAL_MOST or TOTAL[name] >= TRIVIAL_TOTAL
carried = {}
def carries(bone):
    if bone.name not in carried:
        carried[bone.name] = any([has_weight(bone.name)] + [carries(k) for k in bone.children])
    return carried[bone.name]
def kids_of(bone):
    return [k for k in bone.children if carries(k)]


def seg_dist(p, a, b):
    ab = b - a
    t = max(0.0, min(1.0, (p - a).dot(ab) / max(ab.dot(ab), 1e-9)))
    return (p - (a + ab * t)).length


def chain_dist(p, bones):
    return min(seg_dist(p, HEAD[b], TAIL[b]) for b in bones)


# chains: follow the longest way down from each root. A fork where two real limbs leave ends a stretch of trunk; a
# short side branch (toes, a dew claw, a nub) stays part of the chain it grows on, so it is never a chain of its own.
LONG = 0.12 * SIZE
bone_len = lambda b: (TAIL[b.name] - HEAD[b.name]).length
below = {}
def reach_of(bone):
    if bone.name not in below:
        below[bone.name] = bone_len(bone) + max((reach_of(k) for k in kids_of(bone)), default=0.0)
    return below[bone.name]
def subtree(bone):
    return [bone.name] + [n for k in kids_of(bone) for n in subtree(k)]
limbs, trunk, branches, twigs = [], [], [], {}
def read_line(start):
    line, side, bone, run = [], [], start, 0.0
    while True:
        line.append(bone.name); run += bone_len(bone)
        kids = sorted(kids_of(bone), key=reach_of, reverse=True)
        if not kids:
            limbs.append(line); twigs[line[0]] = [n for s in side for n in s]
            return
        real = [k for k in kids if reach_of(k) >= LONG and reach_of(k) >= 0.4 * reach_of(kids[0])]
        if len(real) >= 2 and reach_of(kids[0]) >= 0.6 * run:
            trunk.extend(line)
            branches.extend(side + [subtree(k) for k in kids if k not in real])
            for k in real: read_line(k)
            return
        side += [subtree(k) for k in kids[1:]]
        bone = kids[0]
for root in [b for b in arm.data.bones if not b.parent and carries(b)]:
    read_line(root)
IGNORED = sorted(b.name for b in arm.data.bones if not carries(b))
length = lambda s: sum((TAIL[b] - HEAD[b]).length for b in s)
whole = lambda s: s + twigs.get(s[0], [])
def carries_real_weight(s):
    """It is the heaviest bone on most of the vertices it touches: emptying it would leave them nothing like them."""
    bones = set(whole(s))
    touched = [w for w in WEIGHTS if bones & set(w)]
    own = sum(1 for w in touched if max(w, key=w.get) in bones)
    return own >= 8 and own * 2 >= len(touched)
# junk: a separate branch whose bones all lie on another, longer limb and carry no real weight of their own
junk = []
for s in sorted(limbs + branches, key=length):
    for o in limbs:
        if o is s or o in junk or length(o) <= length(s):
            continue
        if all(chain_dist(HEAD[b], whole(o)) < 0.05 * SIZE and chain_dist(TAIL[b], whole(o)) < 0.05 * SIZE for b in s) and not carries_real_weight(s):
            junk.append(s)
            break
# a person's word beats the read
edit_of = lambda s: EDITS.get(s[0], {})
junk = [s for s in junk if edit_of(s).get("readAs") != "chain"]
junk += [s for s in limbs if s not in junk and edit_of(s).get("readAs") == "junk"]
limbs = [s for s in limbs if s not in junk]
branches = [s for s in branches if s not in junk]
reach = lo.z + 0.25 * (hi.z - lo.z)
legs = [s for s in limbs if TAIL[s[-1]].z < reach and HEAD[s[0]].z - TAIL[s[-1]].z > 0.2 * (hi.z - lo.z)]
others = [s for s in limbs if s not in legs]
long_ = [s for s in others if length(s) >= 0.12 * SIZE]
spikes = [s for s in others if s not in long_] + branches


def thickness(s):
    near = [chain_dist(c, s) for c, w in zip(CO, WEIGHTS) if any(b in w and w[b] > 0.5 for b in s)]
    return sum(near) / max(len(near), 1)


head = max(long_, key=thickness) if long_ else (trunk[-1:] or spikes[0])
long_ = [s for s in long_ if s is not head]
tails = long_
body_at = sum((HEAD[b] for b in trunk), Vector()) / max(len(trunk), 1) if trunk else (lo + hi) / 2
flat = TAIL[head[-1]] - body_at; flat.z = 0
if flat.length < 1e-4: flat = Vector((0, -1, 0))
flat.normalize()
axis = max((Vector((1, 0, 0)), Vector((-1, 0, 0)), Vector((0, 1, 0)), Vector((0, -1, 0))), key=lambda a: a.dot(flat))
FRONT = axis if axis.dot(flat) > 0.94 else flat
LEFT = UP.cross(FRONT).normalized()
centre = sum((HEAD[s[0]] for s in legs), Vector()) / max(len(legs), 1) if legs else body_at


def named_side(names):
    n = " ".join(names).lower()
    if re.search(r"left|(^|[_.:\s])[fbr]?l([_.:\s\d]|$)", n): return "L"
    if re.search(r"right|(^|[_.:\s])[fb]?r([_.:\s\d]|$)", n): return "R"
    return None


leg_rows = []
for s in legs:
    a = HEAD[s[0]] - centre
    by_hand = {"left": "L", "right": "R"}.get(edit_of(s).get("side"))
    leg_rows.append(dict(bones=s, side=by_hand or ("L" if a.dot(LEFT) > 0 else "R"), by_hand=bool(by_hand), along=a.dot(FRONT), named=named_side(s)))
leg_rows.sort(key=lambda r: -r["along"])
four = len(leg_rows) == 4 and sum(r["along"] > 0 for r in leg_rows) == 2
per = defaultdict(int)
for r in leg_rows:
    if four:
        r["name"] = "leg_" + ("F" if r["along"] > 0 else "R") + r["side"]
    else:
        per[r["side"]] += 1
        r["name"] = f"leg_{r['side']}{per[r['side']]}"
# two legs put on one side by hand would share a name and lose a chain
taken = set()
for r in leg_rows:
    while r["name"] in taken: r["name"] += "_2"
    taken.add(r["name"])
legs_by = {r["name"]: r["bones"] for r in leg_rows}
# The head starts at the last bone with a real share of the chain: a nose or jaw tip past it moves with it, and bending the tip alone would not test the head.
own = defaultdict(int)
for w in WEIGHTS:
    if w: own[max(w, key=w.get)] += 1
head_from = max((i for i, b in enumerate(head) if own[b] >= 0.25 * max(own[x] for x in head)), default=len(head) - 1)
neck, head_only = head[:head_from], head[head_from:]
CHAINS = {"body": trunk or [head[0]]}
if neck: CHAINS["neck"] = neck
CHAINS["head"] = head_only
head_twigs = twigs.get(head[0], [])
CHAINS.update(legs_by)
for i, s in enumerate(tails): CHAINS["tail" if i == 0 else f"tail_{i}"] = s
for i, s in enumerate(spikes): CHAINS[f"spike_{i}"] = s
for i, s in enumerate(junk): CHAINS[f"junk_{i}"] = whole(s)
TORSO = CHAINS["body"] + neck + head_only + head_twigs
OWNERS = {"torso": TORSO}
for name, bones in CHAINS.items():
    if name not in ("body", "neck", "head") and not name.startswith("junk"):
        OWNERS[name] = whole(bones)
# Toes and nubs: they follow their chain's owner and colour, but a pose only bends the chain's main line.
TWIGS = {b: name for name, bones in CHAINS.items() if name not in ("body", "neck", "head") and not name.startswith("junk") for b in twigs.get(bones[0], [])}
TWIGS.update({b: "head" for b in head_twigs})
JUNK = [b for s in junk for b in whole(s)]
# mirror twins: the leg at the same place on the other side, bone by bone
MIRROR_PAIRS = []
lefts = [r for r in leg_rows if r["side"] == "L"]
rights = [r for r in leg_rows if r["side"] == "R"]
for l_ in lefts:
    if rights:
        r_ = min(rights, key=lambda r: abs(r["along"] - l_["along"]))
        MIRROR_PAIRS += list(zip(l_["bones"], r_["bones"]))
# stray weight: bones of an owner on vertices far from them and nearer another owner
FAR = 0.12 * SIZE
stray = defaultdict(int)
owner_of = {b: o for o, bs in OWNERS.items() for b in bs}
for c, w in zip(CO, WEIGHTS):
    for b, x in w.items():
        o = owner_of.get(b)
        if o and o != "torso" and x > 0.05 and chain_dist(c, OWNERS[o]) > FAR:
            near = min(OWNERS, key=lambda k: chain_dist(c, OWNERS[k]))
            if near != o:
                stray[(near, o)] += 1
                break
STRAY_DIST = {o: round(FAR, 4) for o in {o for _, o in stray}}
# rigid pieces: are there any, or is this one continuous shell?
par = list(range(len(m.vertices)))
def f(a):
    while par[a] != a:
        par[a] = par[par[a]]; a = par[a]
    return a
for e in m.edges:
    a, b = f(e.vertices[0]), f(e.vertices[1])
    if a != b: par[a] = b
seen = {}
for v in m.vertices:
    k = tuple(round(c, 5) for c in v.co)
    if k in seen:
        a, b = f(v.index), f(seen[k])
        if a != b: par[a] = b
    else:
        seen[k] = v.index
pieces = defaultdict(list)
for i in range(len(m.vertices)): pieces[f(i)].append(i)
RIGID_EXT = round(0.15 * SIZE, 4)
def extent(vs):
    cs = [CO[i] for i in vs]
    return max(max(c[k] for c in cs) - min(c[k] for c in cs) for k in range(3))
rigid = sum(1 for vs in pieces.values() if extent(vs) < RIGID_EXT)
welded = len(pieces) > 1
TRUNK_POSES = {"body": [(CHAINS["body"][-1], tuple(LEFT), 20), (CHAINS["body"][-1], (0, 0, 1), 15)], "head": [(head_only[0], tuple(LEFT), -25), (head_only[0], (0, 0, 1), 25)]}
if neck: TRUNK_POSES["neck"] = [(neck[0], tuple(LEFT), -20), (neck[-1], (0, 0, 1), 20)]
LEGS = {r["name"][4:]: (r["bones"][:2], 0.0 if (i % 2 == 0) else 0.5) for i, r in enumerate(leg_rows)}
values = dict(FRONT=tuple(FRONT), CHAINS=CHAINS, TWIGS=TWIGS, TRUNK_POSES=TRUNK_POSES, TORSO=TORSO, OWNERS=OWNERS, JUNK=JUNK, MIRROR_PAIRS=MIRROR_PAIRS, MIRROR_JUNK=0.30, RIGID_EXT=RIGID_EXT,
              SECONDARY=0.30, FALLBACK_BONE=CHAINS["body"][0], WELD_PIECES=welded, PIECE_RULES=welded, REMAP={}, STRAY_DIST=STRAY_DIST, LEGS=LEGS, SPIKES=[s[0] for s in spikes],
              HEAD_BONE=head_only[0], YAW=14.0, LIFT=24.0, TUCK=-14.0)
os.makedirs(outd, exist_ok=True)
with open(os.path.join(outd, "rigconfig.py"), "w") as out:
    out.write("# rigconfig.py: written by autoconfig.py from the bone tree. Check it on the weightmap.py render.\n")
    for k, v in values.items():
        out.write(f"{k} = {pprint.pformat(v, width=110)}\n")


def side_text(bones):
    a = sum((HEAD[b] for b in bones), Vector()) / len(bones) - centre
    return "left" if a.dot(LEFT) > 0.02 * SIZE else "right" if a.dot(LEFT) < -0.02 * SIZE else "centre"


table = [dict(chain="body", bones=len(CHAINS["body"]), side="centre", read_as="chain", note=None)]
table.append(dict(chain="neck, head", bones=len(head) + len(head_twigs), side="centre", read_as="chain", note=None))
for r in leg_rows:
    swapped = r["named"] and r["named"] != r["side"]
    stray_here = sum(n for (near, o), n in stray.items() if o == r["name"])
    side = {"L": "left", "R": "right"}[r["side"]]
    text = f"{side}, by hand" if r["by_hand"] else f"named {r['named']}, sits {side}" if swapped else f"{side}, by position"
    table.append(dict(chain=r["name"], key=r["bones"][0], bones=len(whole(r["bones"])), side=text, read_as="swapped" if swapped else "stray" if stray_here else "chain", note=f"{stray_here} vertices of another chain" if stray_here else None))
for i, s in enumerate(tails):
    table.append(dict(chain="tail" if i == 0 else f"tail_{i}", key=s[0], bones=len(whole(s)), side=side_text(s), read_as="chain", note=None))
if spikes:
    table.append(dict(chain="spikes", bones=sum(len(s) for s in spikes), side="on the body", read_as="chain", note=None))
for s in junk:
    table.append(dict(chain=s[0], key=s[0], bones=len(whole(s)), side="on other chains", read_as="junk", note=None))
facing = next((n for n, a in (("+X", (1, 0, 0)), ("-X", (-1, 0, 0)), ("-Z", (0, 1, 0)), ("+Z", (0, -1, 0))) if FRONT.dot(Vector(a)) > 0.94), "at an angle")
summary = dict(front=facing, legs=len(legs), swapped=sum(1 for r in leg_rows if r["named"] and r["named"] != r["side"]), junk=[s[0] for s in junk], junk_bones=len(JUNK), ignored=IGNORED,
               rigid_pieces=rigid, pieces=len(pieces), one_shell=not welded, chains=table, bones=len(arm.data.bones), chains_to_test=len(CHAINS),
               stray=[dict(on=near, owner=o, vertices=n) for (near, o), n in sorted(stray.items(), key=lambda kv: -kv[1])])
json.dump(summary, open(os.path.join(outd, "rigconfig.json"), "w"), indent=1)
print("AUTOCONFIG", json.dumps({k: summary[k] for k in ("front", "legs", "swapped", "junk", "ignored", "rigid_pieces", "pieces", "stray")}))
