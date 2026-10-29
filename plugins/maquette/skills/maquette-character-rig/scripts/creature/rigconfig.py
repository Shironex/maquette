# rigconfig.py: EXAMPLE per-creature config for the creature scripts in this folder.
#
# The example describes a made-up crystal spider: a torso with a neck and head, two cheek plates,
# eight legs (four per side, three bones each), two back spikes, and one DUPLICATE leg chain that
# the auto-rigger threw into the rear legs ("junk"). Copy this file next to your own work, rename
# nothing, edit the tables after reading the rig_map renders (weightmap.py) and the probe output,
# then point the scripts at it:
#
#   RIG_CONFIG_DIR=/path/to/your/work Blender -b --factory-startup -P fix.py -- in.glb out.glb
#
# Without RIG_CONFIG_DIR the scripts read the rigconfig.py beside them (this example).
#
# Conventions
#   - The creature faces -Y in Blender space (front view looks at its face). Its LEFT is +X.
#   - Auto-rigger Left/Right names are not trustworthy on non-humanoid rigs (two "Left" chains can
#     sit on the same side). Name groups by what the renders show, not by what the bones say.
#   - Chain names that start with "leg_" or "rear_" are posed as legs; "body", "neck" and "head"
#     use TRUNK_POSES; anything else (cheeks, spikes) is tilted sideways.

# Facing: the world direction (Blender space, Z up) the creature looks along. Views, pose axes and the
# walk all derive from it. A Tripo quadruped can import facing -X: read probe.py's bone heads (where is
# the head?) before the first render.
FRONT = (0, -1, 0)

# --- groups used for colouring, posing and the pose test (weightmap.py, posetest.py, posing.py) ---
CHAINS = {
    "body":     ["tripo::Root", "tripo::Spine_0"],
    "neck":     ["tripo::Spine_1", "tripo::Spine_2"],
    "head":     ["tripo::Spine_3"],
    "cheek_L":  ["tripo::Spine_4"],
    "cheek_R":  ["bone_17"],
    "leg_L1":   ["bone_1", "bone_2", "bone_3"],
    "leg_R1":   ["bone_4", "bone_5", "bone_6"],
    "leg_L2":   ["bone_7", "bone_8", "bone_9"],
    "leg_R2":   ["bone_10", "bone_11", "bone_12"],
    "leg_L3":   ["bone_13", "bone_14", "bone_15"],
    "leg_R3":   ["bone_18", "bone_19", "bone_20"],
    "leg_L4":   ["bone_21", "bone_22", "bone_23"],
    "leg_R4":   ["bone_24", "bone_25", "bone_26"],
    "spike_30": ["bone_30", "bone_31"],
    "spike_32": ["bone_32", "bone_33"],
    "junk_40":  ["bone_40", "bone_41", "bone_42"],   # the duplicate chain
}

# (bone, world axis, degrees) steps used by posing.pose_chain for the trunk groups
TRUNK_POSES = {
    "body": [("tripo::Spine_0", (1, 0, 0), 20), ("tripo::Spine_0", (0, 0, 1), 15)],
    "neck": [("tripo::Spine_1", (1, 0, 0), -20), ("tripo::Spine_2", (0, 0, 1), 20)],
    "head": [("tripo::Spine_3", (1, 0, 0), -25), ("tripo::Spine_3", (0, 0, 1), 25)],
}

# --- weight repair (fix.py) ---
# TORSO is one owner. Legs and spikes each own their own bones. A piece of the mesh follows the
# owner that holds most of its summed weight; bones of any other owner are dropped from it.
TORSO = ["tripo::Root", "tripo::Spine_0", "tripo::Spine_1", "tripo::Spine_2", "tripo::Spine_3",
         "tripo::Spine_4", "bone_17"]
OWNERS = {
    "torso": TORSO,
    "leg_L1": ["bone_1", "bone_2", "bone_3"],    "leg_R1": ["bone_4", "bone_5", "bone_6"],
    "leg_L2": ["bone_7", "bone_8", "bone_9"],    "leg_R2": ["bone_10", "bone_11", "bone_12"],
    "leg_L3": ["bone_13", "bone_14", "bone_15"], "leg_R3": ["bone_18", "bone_19", "bone_20"],
    "leg_L4": ["bone_21", "bone_22", "bone_23"], "leg_R4": ["bone_24", "bone_25", "bone_26"],
    "spike_30": ["bone_30", "bone_31"],          "spike_32": ["bone_32", "bone_33"],
}
# Duplicate chains the rigger added. Their weights are removed (the bones stay, with zero weight).
JUNK = ["bone_40", "bone_41", "bone_42"]
# Left/right bone pairs for mirror repair of pieces that carry too much junk weight.
MIRROR_PAIRS = [("tripo::Spine_4", "bone_17"), ("bone_1", "bone_4"), ("bone_2", "bone_5"),
                ("bone_3", "bone_6"), ("bone_7", "bone_10"), ("bone_8", "bone_11"),
                ("bone_9", "bone_12"), ("bone_13", "bone_18"), ("bone_14", "bone_19"),
                ("bone_15", "bone_20"), ("bone_21", "bone_24"), ("bone_22", "bone_25"),
                ("bone_23", "bone_26")]
MIRROR_JUNK = 0.30   # a piece with more junk weight than this is rebuilt from its twin if cleaner
RIGID_EXT = 0.15     # welded pieces smaller than this follow ONE bone (crystals, joint balls, plates)
SECONDARY = 0.30     # a leg owner with this share of a big piece marks a welded part (a leg in a plate)
FALLBACK_BONE = "tripo::Spine_0"   # for a vertex left with no weight at all
# Pieces: pieces.py welds edge islands that share positions. If that merges the model into one piece
# (WELDED_PIECES 1: a continuous shell whose edge islands are only UV islands), set WELD_PIECES = False
# and PIECE_RULES = False and fix with REMAP and STRAY_DIST instead (fix.py header).
WELD_PIECES = True
PIECE_RULES = True
# Secondary bones no clip will key (a tuft off an elbow): their skin goes to these bones, nearest first.
REMAP = {}            # e.g. {"bone_17": ["leg_bone_upper", "leg_bone_lower"]}
# Weight of an owner's bones on vertices farther than this from all of them (and nearer another owner)
# is removed and refilled from neighbours: a tail bone driving a paw.
STRAY_DIST = {}       # e.g. {"tail": 0.12, "leg_L1": 0.12}

# --- procedural walk (walkclip.py): leg -> (its first two bones, phase 0 or 0.5) ---
# Two alternating sets (a generalised tripod gait). Pair each leg with its opposite neighbour.
LEGS = {
    "L1": (["bone_1", "bone_2"], 0.0),   "R1": (["bone_4", "bone_5"], 0.5),
    "L2": (["bone_7", "bone_8"], 0.5),   "R2": (["bone_10", "bone_11"], 0.0),
    "L3": (["bone_13", "bone_14"], 0.0), "R3": (["bone_18", "bone_19"], 0.5),
    "L4": (["bone_21", "bone_22"], 0.5), "R4": (["bone_24", "bone_25"], 0.0),
}
SPIKES = ["bone_30", "bone_32"]    # tiny out-of-phase shimmer
HEAD_BONE = "tripo::Spine_3"        # small sway
YAW, LIFT, TUCK = 14.0, 24.0, -14.0   # degrees: hip swing, lift, knee tuck
