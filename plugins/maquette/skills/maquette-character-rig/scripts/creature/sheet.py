# sheet.py: put pose sheets (or any PNGs) side by side in one image, e.g. the before and after
# sheets that posetest.py writes. Usage: -- out.png a.png b.png [...]
import bpy, sys; sys.path.insert(0, __import__("os").path.dirname(__import__("os").path.abspath(sys.argv[sys.argv.index("--")-1])))
from shlib import montage
a = sys.argv[sys.argv.index("--")+1:]
montage([a[1:]], a[0], gap=12)
print("SHEET", a[0])
