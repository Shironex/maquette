---
title: Board
description: One card per asset, moving from Concept to Ship, plus a dense list layout and a switcher for projects.
sidebar:
  label: Board
---

The Board follows each asset through the stages: Concept, Refs, 3D model, Blender, Web and Ship. A card shows where the asset stands: queued, generating, waiting for your model, or done.

## Layout

Board layout in the header switches between the stage columns and a list. The list has one dense row per asset: its picture, kind, stage, newest version, triangles, size, status and last change. The size is the shipped build once Compress ran, otherwise the model file, marked source.

A column heading sorts by it, and again flips the order. The stage filter and the search narrow the rows. The layout, sort, filter and search are remembered per project on this machine.

## Selecting and sending

Tick the boxes on cards (Shift selects a range) and press Send to 3D in the selection bar to send several assets in one batch. With a row focused in the list, the arrow keys move, Shift and an arrow extends the selection, Space ticks the row and Enter opens its menu. See all [keyboard shortcuts](/docs/views/shortcuts/).

A card's menu has Send to 3D, Real size and fix-ups, Rename and Copy to project. Rename changes only the name on the card: the asset's id, files and versions keep their names. Copy to project copies the asset with its refs, every model version, thumbnails and finished sends, and leaves the original where it is. Remove from project, the last item, takes the asset off the Board (Remove... does the same for selected cards). Its files and settings move to the workspace trash, nothing is deleted, and the Removed button in the Board header puts it back. Sends waiting for a model are stopped first; an asset whose model is being made right now is refused until it is done.

**Built asset** adds a model a Blender script in the project makes. **By hand** opens the checklist of sends waiting for a model from the provider's site. A send there closes on its own once its asset gets a model another way (the Inbox, the library), also for a model that came in while Maquette was closed.

## Projects

The project switcher at the top of the rail (Cmd+P on macOS, Ctrl+P on Windows) lists projects with their picture, template, asset count and age. A search field shows once there are more than six. Switching starts every view of the new project clean. Runs, Lab images and provider jobs keep going in the project they belong to. The Inbox is shared by the whole workspace.
