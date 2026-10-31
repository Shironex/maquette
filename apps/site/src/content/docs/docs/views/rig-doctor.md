---
title: Rig doctor
description: Fixes the skin weights of a rigged model in headless Blender, on a copy, and lands the result as the next version.
---

The rig doctor fixes what auto-rigs get wrong: cloth and hair weighted to arm or finger bones, chains that sit off their mesh, junk duplicate chains, rigid pieces split between bones. It lives in the Inbox: **Fix in rig doctor** on a rig check that did not pass, and in the menu of a Board card with a model. It has no sidebar item.

## What runs when

The read and the probe run when a rigged model arrives and take seconds. The pose test and the fix take minutes in Blender and run only when you click, or when an agent calls `run_rig_doctor`.

## The steps

1. **Read the model.** The route comes from the bone names: humanoid for Mixamo names, creature otherwise. The **Route** toggle overrides the pick until a draft exists: a humanoid can take the creature route, and the humanoid route needs Mixamo names.
2. **Accept as v1.** The original stays as it came, and the doctor compares against it.
3. **Pose test before.** The same poses from each side, with defects named by place. A creature's chains are read from the bone tree first.
4. **Fix.** Per island weights with smooth seams; for a creature, stray weight comes off, rigid pieces follow one bone and junk chains are emptied.
5. **Verify.** It reports what changed. Only `JOINTS_0` and `WEIGHTS_0` may change, with at most 4 influences, normalised, and no vertex without weight.
6. **Land as v2.** **Add as v2** sends the draft through the Inbox checks. v1 stays, and Compare works as for a redo.

Mesh, UVs, texture, bone names, hierarchy and rest pose stay untouched. The doctor works on a copy in `cache/rig/` and never edits an Inbox file or an accepted version.

The status bar shows a line while a run goes on, such as "Rig doctor: pose test after 9 of 13".

## Check the chains

A creature's chains are read from the bone tree, so the guess needs a person. The **Chains** table lists what each one was read as, with a **Not checked yet** mark until you answer. **Edit** lets you change what a chain is read as, and a leg's side. The body, neck and head, and the grouped spike rows stay as read.

- **Chains are right** marks them **Checked by you** and keeps the sheets.
- **Save and start over** keeps your corrections and sends the case back to the pose test. An existing draft is removed.

**Add as v2** waits for this answer on the creature route. An agent can still run the fix on a copy, but cannot land it.

## Walk proof

Once a creature is fixed, **Walk clip proof** makes a short walk from its chains and renders it as a sheet. It is proof that the draft moves, nothing more: the clip goes to its own file next to the draft and is never added to it. If the proof fails, the fix stands and the error shows on the row. You start it from the app; it has no agent step.

## The joint move

A chain that sits off its mesh gets its weights mirrored from the other side. Moving its joints changes the rest pose, so it is never the default: an approval card asks, and names the clips the move would break.

## One continuous mesh

When one island holds nearly the whole model, the per island fix would tear sleeves and legs, so it does not run. A narrow reweight moves only the arm and hand weight far from the arms, and the doctor says so. There is no second round.

## When it stops

- **It refuses** a model with no skin, several skinned meshes, or interleaved or compressed data, and says why. The model keeps its Inbox checks, and Accept as is stays one click away. **Redo...** opens the usual redo sheet for its asset. That spends credits, so it is your call in the sheet; the doctor never starts one.
- **A check got worse** after the fix: the draft is held and v1 stays.
- **Two rounds** left a tear: the doctor stops instead of guessing a third time. The draft is kept, not added.

## Agents

`start_rig_doctor`, `run_rig_doctor`, `get_rig_doctor` and `list_rig_doctors` can start and read the doctor. They cannot approve anything: when an agent's fix is ready, an approval card opens the sheets for you, and you add it in the app. The doctor never starts a Tripo or Meshy send.
