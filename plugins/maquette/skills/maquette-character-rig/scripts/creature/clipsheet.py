# clipsheet.py: render frames of a GLB's first clip (e.g. walkclip.py output) as one strip, so the clip can
# be judged from PNGs without opening Blender. Rows: side view, then three quarter front.
# Usage: -- clip.glb out.png [frames=6]
import bpy, sys; sys.path.insert(0, __import__("os").path.dirname(__import__("os").path.abspath(sys.argv[sys.argv.index("--")-1])))
from shlib import *
a = sys.argv[sys.argv.index("--")+1:]; glb, out = a[:2]; n = int(a[2]) if len(a) > 2 else 6
arm, me, stray = load(glb)
for s in stray: s.hide_render = True
sc = bpy.context.scene
act = arm.animation_data.action if arm.animation_data and arm.animation_data.action else (bpy.data.actions[0] if bpy.data.actions else None)
if act is None: raise SystemExit("no clip in " + glb)
if not arm.animation_data or arm.animation_data.action is None: arm.animation_data_create(); arm.animation_data.action = act
f0, f1 = (int(x) for x in act.frame_range)
lo, hi = bounds(me); ctr = (lo + hi) / 2; size = max(hi - lo) * 1.15
import os; base = os.path.splitext(os.path.abspath(out))[0]
rows = []
for yaw, pitch in ((90, 8), (35, 20)):
    row = []
    for k in range(n):
        fr = f0 + round((f1 - f0) * k / n); sc.frame_set(fr)
        p = f"{base}_{yaw}_{k}.png"; shot(p, ctr, 2.0, yaw, pitch, res=(420, 340), color="TEXTURE", ortho=size, text=f"{act.name} frame {fr}")
        row.append(p)
    rows.append(row)
montage(rows, out)
for r in rows:
    for p in r: os.remove(p)
print("SHEET", out, "frames", f0, "to", f1)
