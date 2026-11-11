"""The page light: the one preview rig of every headless Maquette render that
has to show a model's colours the way its page will.

It is a module, never run on its own. render_thumb.py imports it for the
thumbnail of a built asset (--look page) and render_views.py for the standard
views of any model, so both read the same numbers:

    import preview_rig as rig
    rig.world(scene, "#312f2b")     # the even fill; the camera sees the backdrop
    rig.sun(scene)                  # the key
    rig.render(scene, "out.png", 1024)

Where the numbers come from
- The light is the one the Bake light step bakes into a model, as the Build
  scene starter sets it up (tools/build_scene.py): a sun of strength 3.0,
  tilted 0.9 radians from straight overhead, and a grey world of 0.18 as the
  even fill from every side. A white surface gets 0.18 where the sun does not
  reach, 0.77 lying flat in the sun and 1.13 facing it. The starter's sun
  comes from the front right; here it is mirrored to the front left, so the
  three quarter view (from the front right) shows a sunlit side and a fill
  side of every colour.
- The display is the reference page's (apps/site/src/scene/Workbench.tsx):
  three.js' ACES Filmic tone mapping at exposure 1.15, written as sRGB. The
  curve below is three.js' own, applied to the float render with numpy, so no
  Blender view transform (Standard, AgX, Filmic) stands between the preview
  and the page.

What that buys: a colour that reads right here reads right on a page that
shows the baked light (albedo times lightmap times scale) through the same
tone mapping. A page with lights of its own (a warm desk lamp, say) tints the
colours its own way on top; the fill side here is what its even fill shows.

The studio thumbnail's key light, for comparison, puts 2.28 on a white
surface and has no tone curve, so every pale colour clips to white.
"""

import math
import os
import warnings

import bpy
import numpy as np

# Build scene's light (starter/build_scene.py), which Bake light bakes in.
SUN_STRENGTH = 3.0
SUN_TILT = 0.9
# The starter turns its sun 0.8 radians to the front right; mirrored here.
SUN_TURN = -0.8
WORLD_GREY = 0.18
# The reference page's display (apps/site/src/scene/Workbench.tsx).
TONE = "aces-filmic"
EXPOSURE = 1.15

SUN = "maquette_sun"
WORLD = "maquette_page"

# three.js ACESFilmicToneMapping: its input and output matrices, row by row.
_ACES_IN = np.array(
    [
        [0.59719, 0.35458, 0.04823],
        [0.07600, 0.90834, 0.01566],
        [0.02840, 0.13383, 0.83777],
    ]
)
_ACES_OUT = np.array(
    [
        [1.60475, -0.53108, -0.07367],
        [-0.10208, 1.10813, -0.00605],
        [-0.00327, -0.07276, 1.07602],
    ]
)


def flat_light():
    """The light on a white surface lying flat in the sun, fill included."""
    return SUN_STRENGTH * math.cos(SUN_TILT) / math.pi + WORLD_GREY


def tone(light, exposure=EXPOSURE):
    """Scene light (linear, the last axis is r g b) as the page shows it:
    linear display values from 0 to 1."""
    v = np.asarray(light, dtype=np.float64) * (exposure / 0.6)
    v = v @ _ACES_IN.T
    v = (v * (v + 0.0245786) - 0.000090537) / (v * (0.983729 * v + 0.4329510) + 0.238081)
    v = v @ _ACES_OUT.T
    return np.clip(v, 0.0, 1.0)


def encode(shown):
    """Linear display values as sRGB, the way three.js writes them."""
    v = np.clip(np.asarray(shown, dtype=np.float64), 0.0, 1.0)
    return np.where(v <= 0.0031308, v * 12.92, 1.055 * np.power(v, 1 / 2.4) - 0.055)


def decode(hex_color):
    """An sRGB hex colour as linear values."""
    h = hex_color.lstrip("#")
    c = np.array([int(h[i : i + 2], 16) / 255 for i in (0, 2, 4)])
    return np.where(c <= 0.04045, c / 12.92, np.power((c + 0.055) / 1.055, 2.4))


def untone(shown, exposure=EXPOSURE):
    """The scene light the page shows as `shown` (linear display values)."""
    want = np.clip(np.asarray(shown, dtype=np.float64), 1e-5, 0.999)
    light = want.copy()
    for _ in range(200):
        light = light * np.power(want / np.maximum(tone(light, exposure), 1e-7), 0.6)
    return light


def scene_color(hex_color):
    """The emission that shows exactly as `hex_color`: for backdrops, paper
    and anything else that must not be tone mapped twice."""
    return tuple(float(c) for c in untone(decode(hex_color)))


def lit_color(hex_color):
    """The base colour a surface lying flat in the sun needs to show as
    `hex_color`: for a floor or a card that should read as that colour."""
    return tuple(float(c) for c in np.clip(untone(decode(hex_color)) / flat_light(), 0.0, 1.0))


def nodes_of(block):
    # Blender 5 always has node trees and warns about the old switch; 4.x needs it.
    with warnings.catch_warnings():
        warnings.simplefilter("ignore", DeprecationWarning)
        block.use_nodes = True
    return block.node_tree


def world(scene, backdrop):
    """The grey world that fills every side evenly. The camera sees
    `backdrop` (an sRGB hex colour) instead of the grey."""
    w = bpy.data.worlds.new(WORLD)
    tree = nodes_of(w)
    nodes, links = tree.nodes, tree.links
    fill = nodes.get("Background")
    fill.inputs["Color"].default_value = (WORLD_GREY, WORLD_GREY, WORLD_GREY, 1.0)
    fill.inputs["Strength"].default_value = 1.0
    seen = nodes.new("ShaderNodeBackground")
    seen.inputs["Color"].default_value = (*scene_color(backdrop), 1.0)
    seen.inputs["Strength"].default_value = 1.0
    path = nodes.new("ShaderNodeLightPath")
    mix = nodes.new("ShaderNodeMixShader")
    links.new(path.outputs["Is Camera Ray"], mix.inputs["Fac"])
    links.new(fill.outputs["Background"], mix.inputs[1])
    links.new(seen.outputs["Background"], mix.inputs[2])
    links.new(mix.outputs["Shader"], nodes.get("World Output").inputs["Surface"])
    scene.world = w
    return w


def sun(scene):
    """The key: Build scene's sun, from the front left."""
    data = bpy.data.lights.new(SUN, "SUN")
    data.energy = SUN_STRENGTH
    obj = bpy.data.objects.new(SUN, data)
    obj.rotation_euler = (SUN_TILT, 0.0, SUN_TURN)
    scene.collection.objects.link(obj)
    return obj


def ground(plane):
    """Makes `plane` a floor that only catches the model's shadow: it
    neither blocks the fill from below nor bounces light back up, so a model
    gets the same even fill as in a bake."""
    plane.visible_diffuse = False
    plane.visible_glossy = False
    plane.visible_transmission = False
    plane.visible_shadow = False


def read_float(path):
    """A float image file as linear (h, w, 3)."""
    image = bpy.data.images.load(path)
    image.colorspace_settings.is_data = True
    w, h = image.size
    data = np.empty(w * h * image.channels, dtype=np.float32)
    image.pixels.foreach_get(data)
    channels = image.channels
    bpy.data.images.remove(image)
    return data.reshape(h, w, channels)[..., :3]


def write_png(srgb, path):
    """sRGB values (h, w, 3) from 0 to 1 as an 8 bit PNG."""
    h, w, _ = srgb.shape
    image = bpy.data.images.new("maquette_page_out", w, h, alpha=False)
    rgba = np.concatenate([srgb, np.ones((h, w, 1))], axis=2).astype(np.float32)
    image.pixels.foreach_set(rgba.ravel())
    image.filepath_raw = path
    image.file_format = "PNG"
    image.save()
    bpy.data.images.remove(image)


def render(scene, out, size, samples=32, height=None):
    """Renders the scene's camera to the PNG `out`, `size` pixels wide (and
    square unless `height` says otherwise), in Cycles on the CPU, and shows
    the light as the page does. Returns the light it measured, linear
    (h, w, 3), rows from the bottom up."""
    scene.render.engine = "CYCLES"
    scene.cycles.device = "CPU"
    scene.cycles.samples = samples
    scene.cycles.use_denoising = True
    scene.cycles.max_bounces = 4
    scene.render.resolution_x = size
    scene.render.resolution_y = height or size
    scene.render.resolution_percentage = 100
    scene.render.film_transparent = False
    scene.render.image_settings.file_format = "OPEN_EXR"
    scene.render.image_settings.color_mode = "RGB"
    scene.render.image_settings.color_depth = "32"
    out = os.path.abspath(out)
    os.makedirs(os.path.dirname(out), exist_ok=True)
    raw = out + ".part.exr"
    scene.render.filepath = raw
    bpy.ops.render.render(write_still=True)
    light = read_float(raw)
    os.remove(raw)
    # Written next to the target and renamed, so a half written PNG is never read.
    part = out + ".part.png"
    write_png(encode(tone(light)), part)
    os.replace(part, out)
    return light
