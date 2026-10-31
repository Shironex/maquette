# Common auto-rig defects

How to recognise each in a pose sheet or a probe, and what fixes it.

## Cloth, capes, robes, hair or tails weighted to arm, hand or finger bones

- Seen as: the `raise`, `forward` and `hand` poses drag the cape or hem out with the arms; hair
  strands follow a hand. `get_inbox_checks` may warn "weighted to finger bones" (a heuristic).
- `probe.py` reports vertices with hand-chain weight above 0.05 that sit more than 15 cm from the
  hand.
- Fix: `humanoid/fixweights.py`. Each connected island is decided as a whole (a cuff belongs to
  the arm; a cape welded to a sleeve keeps only the tube around the arm), the rest gets trunk
  weights, and a geodesic falloff band blends the two so a welded sleeve stretches a little instead
  of tearing.

## One continuous mesh

- Seen as: `probe.py` says `one_island` (one island holds over 90 percent of the vertices). Hair,
  cloth and body are one surface.
- Skip `fixweights.py`: it decides per island, so it would tear sleeves and legs, and it refuses
  here. Use `humanoid/narrow.py`, a narrow reweight that moves only the arm and hand weight far
  from the arms, with a faded tail. Say that is what ran. Round 2 does not apply: finish by hand.

## A limb chain placed off its mesh

- Seen as: arm bones sit inside the cape or torso; the arm pose moves cloth, not the arm.
  `fixweights.py` marks the limb `misplaced` in its report and gives the mesh's real joint
  positions (`registered_joints`).
- Fix: the weights are repaired by mirroring the good side onto this limb. Moving the joints
  (`rigmove.py`) is optional and changes the rest pose: only with the person's approval.

## Non-humanoid: Left and Right names that do not match the real sides

- Seen as: two chains both called Left sit on the same side. Trust the renders, not the names.
- Fix: name groups by position in `rigconfig.py` (`CHAINS`, and the `L`/`R` in `LEGS` and
  `MIRROR_PAIRS`), and keep the creature's left on +X.

## Non-humanoid: junk duplicate chains

- Seen as: extra bones on top of the rear legs or back that carry weight; legs jitter or bend
  twice. `weightmap.py` shows them as a second colour on one leg.
- Not junk: a short branch off a limb (toes, a dew claw) belongs to that limb, and a bone that is the heaviest on most of the vertices it touches carries real weight. `autoconfig.py` reads both that way, and neither may go under `JUNK`.
- Fix: list them under `JUNK`. `fix.py` removes their weights (the bones stay, at zero weight, so
  names and hierarchy do not change).

## Non-humanoid: rigid pieces split between bones

- Seen as: a crystal, plate or joint ball bends in the pose test. `posetest.py` shows a deviation
  above about 4 mm (model about 1 unit wide) on a piece that should be rigid.
- Fix: pieces smaller than `RIGID_EXT` follow exactly one bone. Raise or lower the threshold per
  creature after looking at `piecemap.py`.

## Things not to do

- A hard distance cut inside one piece (it tears cloth).
- More than 4 influences, or weights that do not sum to 1.
- Touching UVs, texture, bone names or the rest pose to "improve" a result.
- Shipping a procedural test clip as if it were an animation.
