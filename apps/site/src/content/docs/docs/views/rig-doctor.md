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

## Reading the numbers

Every cell gives three numbers for its pose, in mm at a model one metre tall (its longest side for a creature). They are of the whole mesh in that pose, so every view of the pose shares them, and the tiles under the sheet name the pose their worst number came from.

- **Stretch** is how much the worst single edge changed length from rest. Any rig stretches an edge across a bent joint, so a big number here does not mean the weights are wrong.
- **Past body** is that stretch after taking off what the pose does anyway, measured against the kinder of two healthy skins: the mesh with every vertex stuck to its nearest bone, and the model's own weights on nearby bones evened over a few rings. A blend across a robe is fine by the second; a hard step between neighbours, or weight on a far bone, is not. This is the part the weights answer for. It is the stretch that half a percent of the surface reaches, so a handful of sliver edges in a dense mesh cannot set it: a pose reads as stretching wrongly when that much of the surface passes 80 mm.
- **Seam** is how far vertices that share a position at rest came apart: a real crack in the surface, from 5 mm. A split surface has no edge across it, so stretch does not see a tear.

When a single edge passes 80 mm but too little of the surface does to call a tear, the cell says **look here** in amber and names the nearest bone. These edges are real: on a robed character they are often a rim of slivers where a layer on the leg meets a layer on the trunk, mostly under the cloth. The row is not flagged and stops nothing, so play the clip in Motion and decide.

A cell says **looks fine** only when it has no defect, neither number passes its limit and no edge asks for a look, and the Looks fine panel lists those same rows.

After the fix, each row shows its before and after and which way they went, and the header counts the rows that moved by 5 mm or more. Stretch often rises where a seam closes: the surface that tore now holds, so its edges stretch instead. When a pose got worse, a warning sits by **Add as v2**. It does not block, so play those clips in Motion and decide.

Click a cell to enlarge it; in a comparison, before and draft open together under the same camera. Each row's camera fits that pose, so nothing leaves the frame. The sheet from **Save sheet** carries a label over each picture.

## Check the chains

Bones with no weight, or next to none, down their whole branch make no chain and do not count as legs. They stay in the file, and a line under the table names them.

A creature's chains are read from the bone tree, so the guess needs a person. The **Chains** table lists what each one was read as, with a **Not checked yet** mark until you answer. **Edit** lets you change what a chain is read as, and a leg's side. The body, neck and head, and the grouped spike rows stay as read.

- **Chains are right** marks them **Checked by you** and keeps the sheets.
- **Save and start over** keeps your corrections and sends the case back to the pose test. An existing draft is removed.

**Add as v2** waits for this answer on the creature route. An agent can still run the fix on a copy, but cannot land it.

## Walk proof

Once a creature is fixed, **Walk clip proof** makes a short walk from its chains and renders it as a sheet. It is proof that the draft moves, nothing more: the clip goes to its own file next to the draft and is never added to it. If the proof fails, the fix stands and the error shows on the row. You start it from the app; it has no agent step.

## The joint move

A chain that sits off its mesh gets its weights mirrored from the other side. Moving its joints changes the rest pose, so it is never the default: an approval card asks, and names the clips the move would break.

## One continuous mesh

When one island holds nearly the whole model, the per island fix would tear sleeves and legs, so it does not run. On a humanoid, a narrow reweight moves only the arm and hand weight far from the arms, and the doctor says so. There is no second round.

On a creature with one continuous shell, its islands are only UV islands, so the per piece rules stay off and the fix only takes stray weight off the chains.

## When it stops

- **It refuses** a model with no skin, several skinned meshes, or interleaved or compressed data, and says why. The model keeps its Inbox checks, and Accept as is stays one click away. **Redo...** opens the usual redo sheet for its asset. That spends credits, so it is your call in the sheet; the doctor never starts one.
- **A check got worse** after the fix: the draft is held and v1 stays.
- **The fix changed nothing**: no weight moved, so there is no draft to add and every defect stays open. The case stands where its pose test left it: correct the chains and run it again, or keep the version as it is.
- **Two rounds** left a tear (a seam still open or stretch past the body): the doctor stops instead of guessing a third time. The draft is kept, not added.

## Agents

`start_rig_doctor`, `run_rig_doctor`, `get_rig_doctor` and `list_rig_doctors` can start and read the doctor. They cannot approve anything: when an agent's fix is ready, an approval card opens the sheets for you, and you add it in the app. The doctor never starts a Tripo or Meshy send.
