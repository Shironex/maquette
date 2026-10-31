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

Tick the boxes on cards (Shift selects a range) and press Send to 3D in the selection bar to send several assets in one batch. Splitting a model into parts on Tripo or Meshy is a send too: it waits in the same 3D queue, under the same approval for agents, and its credits show in the Credits sheet (see [Parts](/docs/views/parts/#split-on-tripo-or-meshy)). With a row focused in the list, the arrow keys move, Shift and an arrow extends the selection, Space ticks the row and Enter opens its menu. See all [keyboard shortcuts](/docs/views/shortcuts/).

A card's menu has Send to 3D, Refs and views, Real size and fix-ups, Rename, Copy to project and Add a model file as the next version, which copies a `.glb`, `.gltf`, `.fbx` or `.obj` you fixed yourself into the inbox for that asset (see [Inbox](/docs/views/inbox/#downloads-from-by-hand-sends)). Rename changes only the name on the card: the asset's id, files and versions keep their names. Copy to project copies the asset with its refs, every model version, thumbnails and finished sends, and leaves the original where it is. Inspect 3D model opens the newest version in a 3D quick view over the Board, the same preview as the Inbox: drag to orbit, scroll to zoom, Shaded, Wireframe, UV and (for a rigged model) Bones, with the version, triangles and size, and Open in Inspector for the full Compare tab. Escape or a click outside closes it. Like Add to library and Open in Blender, it shows only once the asset has a model. A right-click on a card opens the same menu as its dots. Remove from project, the last item, takes the asset off the Board (Remove... does the same for selected cards). Its files and settings move to the workspace trash, nothing is deleted, and the Removed button in the Board header puts it back. Sends waiting for a model are stopped first; an asset whose model is being made right now is refused until it is done.

## Refs and views

Refs and views... on a card opens the asset's refs sheet. Each ref shows one view of the object: Front, Left, Back, Right or Other. The four view slots sit in send order with a count (1 of 4 views), and the Other tray keeps refs that stay with the asset but never go as views. The chip on an image changes its view; picking a view another ref holds swaps the two. Tags save at once, on the asset in `project.json`, never in the file names. Add refs takes any image already in `refs/`, and Leave out of the refs drops one from the asset while the file stays.

An asset nobody tagged reads its views from the order its refs always went in (its Lab picks, else the files named after it): the first is the front, then left, back and right, and any more are other. Sends behave as before until a tag changes.

A send carries the view slots front first, then left, back and right, skipping an empty one, and leaves Other refs home even when they are picked. Tripo's multi-view gets each image as its view, gaps allowed. By hand folders name the images by view: `front.png`, `left.png`, `back.png`, `right.png`, and a By hand Tripo card with more than the front asks for Multiview to 3D with each image in the slot of its view.

**Make the other views** draws the missing views from the front with Codex, the same jobs and lane as the [Lab](/docs/views/lab/): tick the views (empty ones are ticked, a filled one draws again only when ticked), pick 1 to 4 images per view, whether the project's style board goes along and, if you like, notes for every view (the sword in the left hand). Each image is one Codex job, started by the Draw button like any Lab brief. The candidates come back per view in the sheet and in the Lab direction "<asset> views"; **Use as Left** copies one into `refs/` and into its slot. When the slot is filled the button reads **Replace Left**: the old ref leaves the asset and moves to `refs/originals/`, kept, unless another asset or a send still uses it, in which case its file stays in `refs/`.

**Clean background** (on an image's chip) makes a copy with the object kept and the clutter cleared, on studio grey or transparent, optionally centered and padded to a square. It invents nothing the frame cut off unless Draw what the frame cut off is on. The sheet shows the original and the copy side by side; **Use the clean copy** puts the copy in the ref's place with the same view and moves the original to `refs/originals/`, kept. **Keep the original** changes nothing.

## Weak ref warnings

Send to 3D and the batch panel check the picked refs before any credit is spent, on this machine, with nothing sent anywhere:

- **Small**: the image's short side is under 512 px.
- **Crops the object**: the object touches a frame edge.
- **Busy background**: the background around the object is not plain.
- **One view only**: multi-view is on but one image goes.
- **Looks different from the front**: a side or the back does not match the front in its main colours (a small colour count of the object, with the background left out, by hue so softer light still matches) or its proportions (how tall the object stands in the frame; for the back also its height to width, since a side shows the depth and may be narrower or wider). The warning says which, for example "Back looks different from the front: its colours do not match". It only says the views look different: light or a redraw can do that too. Skipped on a busy background, and proportions are skipped when the object is cropped.

The pixel checks read PNG refs; a JPEG or WebP gets the size check only. Each warning offers its fix: Clean background, Make the other views (both open the refs sheet, Make the other views draws a view that looks different again from the front) or Leave out this image. Warnings never block Send. The footer counts them next to the cost (2 ref warnings), each image carries its own flag, and the batch panel gives each asset a line (or Refs look good), a banner with Show only these, and repeats the count in its approval: "2 assets go with ref warnings."

An asset that already has a model, for example one it got from the library, also gets a note above the settings: "Stone well already has a model (v1, from the library entry stone-well). This send makes v2; the library entry is not changed." It is not a ref warning, is not counted and never blocks Send. In the batch panel each such asset carries the same note. To put the new model in the library, use Add to library afterwards: it becomes the entry's next version, not a second entry.

**Built asset** adds a model a Blender script in the project makes. **Export pack** writes every accepted model of the project as one pack for another engine, checked in Blender (see [Export packs](/docs/views/library/#export-packs)). **By hand** opens the checklist of sends waiting for a model from the provider's site. A send there closes on its own once its asset gets a model another way (the Inbox, the library), also for a model that came in while Maquette was closed.

## Projects

The project switcher at the top of the rail (Cmd+P on macOS, Ctrl+P on Windows) lists projects with their picture, template, asset count and age. A search field shows once there are more than six. Switching starts every view of the new project clean. Runs, Lab images and provider jobs keep going in the project they belong to. The Inbox is shared by the whole workspace.
