---
title: Motion
description: Read the clips a model carries, check its rig and pose the parts of a prop as named states.
sidebar:
  label: Motion
---

Motion lists the project's models that move: a rig, animation clips or object states. The ones that do not move yet sit under No motion yet.

## Clips

Every clip is read from the GLB as it is: name, frames, key rate, keys, channels, keyframe bytes, how far the root travels and whether it loops. Pick a model and a clip to play it in the studio with the skeleton and the path its root walks. Each clip has a ship switch: one switched off stays in the model but is left out of the site from the next Build scene.

## Rig check

The rig check shows the bone count, whether the rig maps onto the humanoid (Mixamo) skeleton, the most bones one vertex weighs on against the web's four, and which humanoid bones are missing.

## Budget

Motion has no budget of its own. Clips are bytes inside the models, so they count in the scene budget in the top bar. Over budget, the view marks the clips that would have to stay off, heaviest first.

## States

A prop's states are poses of its parts. The first state is the model at rest. Every other state moves named parts by an offset in metres and degrees. Between two states a transition has a duration, delay, easing and stagger. Triggers say what starts a move on the site: arriving at a camera stop, a scroll range, a click or a hover. Up to 16 states and 32 triggers per asset.

Save motion writes the states and clip choices into `layout.json`, next to the camera stops. The site build does not play states yet: they travel with `layout.json` for the page to read.
