---
title: Keyboard shortcuts
description: Every keyboard shortcut in the app, grouped by view, with the Mac and Windows keys.
sidebar:
  label: Shortcuts
---

Hover a button to see its shortcut in the tooltip. This page lists every shortcut the app has.

## Mac and Windows

**Mod** below means Command (`⌘`) on macOS and Ctrl on Windows. Shift and Alt are the same keys on both. Letter keys do not care about Caps Lock.

Shortcuts that act on a view do nothing while the cursor is in a text field, so typing a name never moves a gizmo. The Motion and Timeline keys also skip a key pressed with Mod or Alt.

## Everywhere

| Keys      | What it does                                                                         |
| --------- | ------------------------------------------------------------------------------------ |
| `Mod` `P` | Opens the project switcher, and closes it again. Not with Shift or Alt.              |
| `Esc`     | Closes the open dialog, sheet, lightbox, menu or tooltip.                            |
| `⌘` `Q`   | Quits the app. macOS only, from the app menu. Windows has the window's close button. |

In the project switcher, `↓` and `↑` move through the projects, `Home` and `End` jump to the first and last, and `Enter` in the search field opens the first match. Typing anywhere in the list starts a search.

## Board

| Keys  | What it does                                                            |
| ----- | ----------------------------------------------------------------------- |
| `Esc` | Clears the selected cards. It leaves them alone while a dialog is open. |

In the list layout, with a row focused:

| Keys                               | What it does                                                  |
| ---------------------------------- | ------------------------------------------------------------- |
| `↓` `↑`                            | Moves to the next or previous row.                            |
| `Home` `End`                       | Moves to the first or last row.                               |
| `Shift` `↓` / `↑` / `Home` / `End` | Moves and ticks the rows on the way, extending the selection. |
| `Space`                            | Ticks or unticks the row.                                     |
| `Enter`                            | Opens or closes the row's menu.                               |

New project and Make object refs (from the Lab) both take `Mod` `Enter` to confirm and `Esc` to close. See [Board](/docs/views/board/).

## Lab

In the lightbox:

| Keys    | What it does                      |
| ------- | --------------------------------- |
| `←` `→` | Shows the previous or next image. |
| `S`     | Stars or unstars the image.       |
| `P`     | Picks or unpicks the image.       |
| `Esc`   | Closes the lightbox.              |

In the Make object refs sheet:

| Keys           | What it does                                                           |
| -------------- | ---------------------------------------------------------------------- |
| `Enter` or `,` | Adds the name you typed as an object.                                  |
| `Backspace`    | In an empty field, removes the last object.                            |
| `Mod` `Enter`  | Makes the refs. Inside an object's editor it saves the object instead. |
| `Esc`          | Closes the sheet.                                                      |

See [Lab](/docs/views/lab/).

## Inspector

### Layout tab

| Keys                            | What it does                                                                               |
| ------------------------------- | ------------------------------------------------------------------------------------------ |
| `W`                             | Move tool.                                                                                 |
| `E`                             | Turn tool.                                                                                 |
| `R`                             | Scale tool.                                                                                |
| `F`                             | Frames every model.                                                                        |
| `Mod` `Z`                       | Undo.                                                                                      |
| `Shift` `Mod` `Z`, or `Mod` `Y` | Redo. Both work on both systems. The tooltip shows `⇧⌘Z` on a Mac and `Ctrl+Y` on Windows. |
| `Mod` `S`                       | Saves `layout.json`.                                                                       |
| `Esc`                           | Leaves look through when a stop is being previewed, otherwise deselects.                   |

### Paths tab

| Keys                    | What it does                                         |
| ----------------------- | ---------------------------------------------------- |
| `V`                     | Select tool.                                         |
| `A`                     | Add points tool. Only while a path is picked.        |
| `D`                     | Draw on surfaces tool. Only while a path is picked.  |
| `Delete` or `Backspace` | Removes the picked point.                            |
| `F`                     | Frames the scene.                                    |
| `Mod` `S`               | Saves the paths.                                     |
| `Esc`                   | Leaves the add tool, or lets go of the picked point. |

### Compare slider

With the split handle focused, `←` and `→` move it by 5%.

See [Inspector](/docs/views/inspector/) and [Paths](/docs/views/paths/).

## Motion

| Keys      | What it does                                       |
| --------- | -------------------------------------------------- |
| `Space`   | Plays or pauses the clip.                          |
| `,` `.`   | Steps back or forward one frame.                   |
| `I` `O`   | Sets the In and Out frames of the preview loop.    |
| `L`       | Turns looping on or off.                           |
| `Mod` `S` | Saves the motion, when there is something to save. |

See [Motion](/docs/views/motion/).

## Timeline

| Keys    | What it does                                     |
| ------- | ------------------------------------------------ |
| `Space` | Plays the scroll at reading speed, or pauses it. |
| `[` `]` | Jumps to the previous or next camera stop.       |

With the ruler focused, `←` and `→` move the playhead by 1%, `Page Up` and `Page Down` by 10%, and `Home` and `End` go to the top and the bottom of the page. See [Timeline](/docs/views/timeline/).

## Page

In the path field above the preview, `Esc` closes the field without changing the path. See [Page](/docs/views/page/).

## Dialogs

`Esc` closes the New project, Import game, Add built asset, Rename, Prepare, Downloads and library dialogs, the provider sheets, the 3D queue and the menus. Their close buttons show `Esc` in the tooltip.
