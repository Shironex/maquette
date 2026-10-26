---
title: Lab
description: Turn a brief into concept images with the Codex CLI, star the good ones and send the picks to Refs.
sidebar:
  label: Lab
---

The Lab turns a brief into concept images with the Codex CLI. A brief is a direction name, a prompt, up to 8 reference images, an aspect ratio and 1 to 12 images. Each image is one Codex job, and every image spends Codex usage.

## Jobs

Jobs run in the project folder on their own lane, 4 at a time by default (Settings, up to 8). A job is stopped after 10 minutes by default. It succeeds when its file appears.

When Codex answers 429 Too Many Requests, the job waits and runs again on its own after 3, 6 and 12 minutes. After that it fails and asks you to try again later. Retry on a failed or cancelled job runs the same prompt, refs and target file again without making a new direction.

## Results

Results show per direction with Star and Pick, and a lightbox: arrow keys, S to star, P to pick, Escape to close. Contact sheet tiles every image into `contact_sheet.png` with ffmpeg. Send picks to Refs copies the picked images into `refs/` and moves the asset with the direction's name to Refs on the Board, or adds it there.

## Object refs

Make object refs turns one concept into a clean reference image per object, for a scene with many props. List up to 12 objects, add optional notes per object, pick the aspect (1:1 by default) and whether they become props or characters. Each object is one Codex job that asks for that object alone, centered, in a plain light grey background, with no text. The images land in a new direction, "name objects".
