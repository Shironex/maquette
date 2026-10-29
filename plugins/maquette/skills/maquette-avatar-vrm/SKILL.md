---
name: maquette-avatar-vrm
description: Prepare a rigged Maquette character for a VRM 1.0 avatar. Use when a rigged character must face +Z in a T-pose for VRM, or when asked to check how its cape, hair or sleeves hold up when the arms are lifted to the T-pose. Early stage, the script only.
---

# Maquette avatar (VRM)

Stage 1 of the VRM export. Today this skill holds one script, `scripts/avatar_normalize.py`, the
same one Maquette runs to normalise a rigged model's rest pose: facing +Z, hips centred, soles on
the ground, arms level along X, that pose made the rest. `scripts/README.md` says how to run it and
what its job file holds.

## Hard rules

- A rest pose change needs a person's yes. Show the before and after pose sheets and the report,
  and wait. Never use a normalised model before that.
- Work on a copy in a scratch folder. Never write into the workspace's versions or a game repo.
- Never change skin weights silently. Cloth or hair that follows the arms is reported, with the
  sheets, and fixed only with the person's agreement.
- Blender runs headless only. Never capture the screen.
