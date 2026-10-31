# groups.py: colour helpers over the CHAINS table in rigconfig.py (bone -> chain -> colour).
import colorsys
from shlib import CHAINS
import rigconfig as _rc
BONE2CHAIN = {b: c for c, bs in CHAINS.items() for b in bs}
BONE2CHAIN.update(getattr(_rc, "TWIGS", {}))   # toes and nubs read in the colour of their chain
def chain_color(chain, seg=0):
    names = list(CHAINS); i = names.index(chain)
    h = (i * 0.618034) % 1.0
    if chain == "body": return (0.75, 0.75, 0.75, 1)
    v = [1.0, 0.72, 0.45][min(seg, 2)]
    r, g, b = colorsys.hsv_to_rgb(h, 0.85, v); return (r, g, b, 1)
def bone_color(b):
    c = BONE2CHAIN.get(b, "body"); return chain_color(c, CHAINS[c].index(b) if b in CHAINS[c] else 0)
