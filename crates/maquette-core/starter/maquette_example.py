"""Example for maquette_helpers.py: a desk set that uses every helper once.

A tray with two books, a ribbon marker, a mug and a brass desk bell. Read it
next to maquette_helpers.py and copy the parts a prop of yours needs. To see
it on the Board, add it as a built asset with the helper file listed:

    add_built_asset  script built/maquette_example.py
                     helpers [built/maquette_helpers.py]

Maquette calls build() in an empty scene and exports the object it returns
with everything parented under it. Names, parents, origins and materials reach
the model as they are made here.
"""

import math

import maquette_helpers as mq

PAPER = "#f6f1e4"
# The top of the tray's floor: what the pieces stand on.
FLOOR = 0.008


def materials():
    """Flat colours by name, and the materials with a look of their own."""
    mat = mq.palette(
        {
            "oak": ("#c9a06c", 0.7),
            "cover_blue": ("#3d5a80", 0.6),
            "cover_red": ("#a8443a", 0.6),
            "paper": (PAPER, 0.9),
            "ribbon": ("#e0a9a2", 0.8),
            "ebony": ("#2b2623", 0.5),
        }
    )
    # The cut of each page line: the paper's own colour, darker.
    shade = tuple(channel * 0.45 for channel in mq.color(PAPER)[:3])
    mat["page_line"] = mq.flat_material("page_line", shade, roughness=0.9)
    mat["brass"] = mq.brass()
    mat["glaze"] = mq.glazed_ceramic("glaze", "#e9e4d8")
    return mat


def tray(mat):
    """A box, hollowed with a boolean, then beveled so the new rim is round too."""
    body = mq.box("tray", (0.40, 0.28, 0.016), material=mat["oak"])
    mq.boolean(body, mq.box("tray_hollow", (0.376, 0.256, 0.02), (0, 0, FLOOR)))
    return mq.bevel(body, 0.003)


def ribbon(name, start, drop, material):
    """A ribbon marker from raw vertices and faces: it leaves the pages at
    `start`, turns over their edge and hangs down by `drop`."""
    x, y, z = start
    half, thick, bend = 0.004, 0.0006, 0.004
    edge = y - 0.005
    # Its upper and its lower side: two lines in the y, z plane, bent alike.
    over = [(y + 0.03, z + thick), (edge - thick, z + thick), (edge - thick, z - drop)]
    under = [(y + 0.03, z), (edge, z), (edge, z - drop)]
    vertices, faces = [], []
    for (over_y, over_z), (under_y, under_z) in zip(
        mq.fillet(over, bend + thick), mq.fillet(under, bend)
    ):
        vertices += [(x - half, over_y, over_z), (x + half, over_y, over_z)]
        vertices += [(x + half, under_y, under_z), (x - half, under_y, under_z)]
    # Four vertices at each step along the ribbon, four faces between two steps.
    for a in range(0, len(vertices) - 4, 4):
        faces += [(a + k, a + (k + 1) % 4, a + 4 + (k + 1) % 4, a + 4 + k) for k in range(4)]
    faces += [(0, 1, 2, 3), tuple(range(len(vertices) - 4, len(vertices)))]
    strip = mq.mesh_object(name, vertices, faces, material, outward=True)
    # A raw mesh comes flat shaded: smooth along the bend, crisp at the sides.
    return mq.crisp_edges(strip)


def book(name, size, cover, mat, marker=False):
    """A hardback lying flat, its spine at -x: a cover pushed out along the
    spine from its end outline, around a block of pages with page lines."""
    width, length, thick = size
    board, square, radius, steps = 0.0025, 0.003, thick * 0.4, 8
    # The cover seen from its end, outside and inside: two rectangles in the
    # x, z plane, rounded at the spine only. Each runs counter clockwise from
    # its lower spine corner past its two fore edge corners.
    spine = (True, False, False, True)
    outer = mq.rounded_rect(width, thick, radius, steps, (0, thick / 2), spine)
    inner = mq.rounded_rect(
        width - board, thick - 2 * board, radius - board, steps, (board / 2, thick / 2), spine
    )
    # A C around the pages: the outside as far as its lower fore edge corner,
    # the inside backwards (lower board, spine, upper board), the outside home.
    turn = steps + 2
    back = inner[::-1]
    outline = outer[:turn] + back[turn:] + back[:turn] + outer[turn:]
    # No bevel: the rounded spine shades smooth and the ends stay crisp.
    wrap = mq.extrude(name, outline, length, mat[cover], plane="XZ", offset=-length / 2)
    pages = mq.page_block(
        f"{name}_pages",
        (width - radius - square, length - 2 * square, thick - 2 * board),
        ((radius - square) / 2, 0, board),
        mat["paper"],
        line_material=mat["page_line"],
    )
    parts = [wrap, pages]
    if marker:
        tail = (0.02, -length / 2 + square, thick / 2)
        parts.append(ribbon(f"{name}_ribbon", tail, thick / 2 + 0.02, mat["ribbon"]))
    # One mesh, and every face keeps the material it was made with.
    return mq.join(parts, name)


def mug(mat):
    """A lathed body: one profile from the middle of the base, up the outside,
    over the lip and down the inside. fillet() rounds the foot, the lip and
    the floor. The handle is a tube along a rounded path."""
    radius, height, wall = 0.041, 0.095, 0.0055
    profile = [(0, 0), (radius, 0), (radius, height)]
    profile += [(radius - wall, height), (radius - wall, 0.007), (0, 0.007)]
    body = mq.lathe("mug", mq.fillet(profile, (0, 0.012, 0.004, 0.004, 0.008, 0)), mat["glaze"])
    grip = [(radius - 0.003, 0, 0.076), (radius + 0.027, 0, 0.079)]
    grip += [(radius + 0.030, 0, 0.044), (radius - 0.003, 0, 0.021)]
    handle = mq.tube("mug_handle", mq.fillet(grip, 0.016), 0.0055, mat["glaze"])
    return mq.join([body, handle], "mug")


def bell(mat):
    """A brass desk bell on an ebony base. Its button is a child object with
    its origin where it meets the dome, so a page can press it."""
    base = mq.cylinder("bell", 0.040, 0.008, material=mat["ebony"], bevel=0.002)
    # A dome is a lathe too: one corner of its profile rounded all the way.
    arc = mq.fillet([(0, 0), (0.035, 0), (0.035, 0.030), (0, 0.030)], (0, 0, 0.028, 0))
    dome = mq.lathe("bell_dome", arc, mat["brass"], location=(0, 0, 0.008))
    whole = mq.join([base, dome], "bell")
    button = mq.sphere("bell_button", 0.007, (0, 0, 0.042), mat["brass"])
    mq.set_origin(button, (0, 0, 0.038))
    mq.parent_to(button, whole)
    return whole


def build():
    mat = materials()
    root = mq.empty("desk_set")

    low = book("book_low", (0.150, 0.215, 0.034), "cover_blue", mat)
    low.location = (-0.085, 0.0, FLOOR)
    top = book("book_top", (0.130, 0.190, 0.024), "cover_red", mat, marker=True)
    # The second book lies on whatever the first one's height came to.
    top.location = (-0.080, -0.020, mq.bounds(low)[1].z)
    top.rotation_euler.z = math.radians(7)

    cup = mug(mat)
    cup.location = (0.095, 0.050, FLOOR)
    cup.rotation_euler.z = math.radians(-40)
    desk_bell = bell(mat)
    desk_bell.location = (0.100, -0.075, FLOOR)

    for piece in (tray(mat), low, top, cup, desk_bell):
        mq.finish(piece, parent=root)
    # The origin of the model: the bottom centre of the whole set.
    mq.ground(root)
    print(f"desk set: {mq.triangles(root)} triangles")
    return root
