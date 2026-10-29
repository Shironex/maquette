---
title: Lab
description: Turn a brief into concept images with the Codex CLI or OpenAI Images, star the good ones and send the picks to Refs.
sidebar:
  label: Lab
---

The Lab turns a brief into concept images with the Codex CLI. A brief is a direction name, a prompt, up to 8 reference images, an aspect ratio and 1 to 12 images. Each image is one Codex job, and every image spends Codex usage.

## Jobs

Jobs run in the project folder on their own lane, 4 at a time by default (Settings, up to 8). A job is stopped after 10 minutes by default. It succeeds when its file appears.

When Codex answers 429 Too Many Requests, the job waits and runs again on its own after 3, 6 and 12 minutes. After that it fails and asks you to try again later. Retry on a failed or cancelled job runs the same prompt, refs and target file again without making a new direction.

When Codex says its usage limit is hit, the job and every image still queued wait until the limit lifts, then run on their own. A banner over the results says when: the reset time Codex's own usage read names, else the time in Codex's message, else a guess that grows like the rate limit wait. Limit lifted, retry now reads the usage again. When Codex allows ordinary usage and the five hour window is not full, the waiting images run at once; otherwise the banner says until when the limit still holds.

## Codex usage

While the Lab is open, Maquette asks Codex how much of its usage is used every minute, and again soon after an image finishes or meets the limit. The status bar shows both windows, for example "Codex 16% (5h) · 3% (week)", amber from 80% and red at the limit; its tooltip has each window's reset time in your local time, and a click reads again. Before a round of 4 images or more, the brief warns when a window is at 80% or more, and before any round while the limit holds. The read goes through `codex app-server` and spends nothing. An older Codex CLI, or one that does not answer within 10 seconds, shows "Codex usage unknown" and the jobs keep the times from Codex's messages.

## OpenAI Images

Settings > Image generation can send each kind of Lab image (concepts, object refs, other views, clean background) to OpenAI's Image API instead of Codex. Codex stays the default for every kind. An API image is paid per image from your own OpenAI API key, at the quality you pick (low by default, about $0.01 an image; high is about $0.22). It gets the same prompt and the same attached images in the same order as Codex would, lands in the same file and moves through the same states, so Retry, Cancel and Use this one work the same. When OpenAI asks it to slow down, it waits as long as asked and runs again on its own.

Under Generate the brief says what the round likely costs and how much of the monthly cap is spent. A round that would pass the cap does not start; raise the cap or switch the kind back to Codex. The estimate is what an image at that quality has cost lately (the average of the last 20 measured images), or the price list's guess until there are a few. A round you start yourself has no approval prompt: the cap is the guard. A round an agent asks for over MCP waits for you instead: the app shows Make N images with OpenAI Images? with the estimate and the month's spend, its jobs wait in the queue as Needs your approval (Cancel works there), and nothing is sent until you approve. Deny cancels them. Each image made is written to `credits.jsonl` at the workspace root in dollars, from the tokens OpenAI reports (or the price list's estimate when it reports none). The image queue names the backend each round runs on. When OpenAI refuses an image on safety grounds, its row says Refused for safety with the reason OpenAI gave (hover for the whole message); nothing was charged, and the prompt needs rewording, since Retry sends the same one.

## Results

Results show per direction with Star and Pick, and a lightbox: arrow keys, S to star, P to pick, Escape to close. Contact sheet tiles every image into `contact_sheet.png` with ffmpeg. Send picks to Refs copies the picked images into `refs/` and moves the asset with the direction's name to Refs on the Board, or adds it there.

## Style board

The style board keeps every direction of a project in one look. Open it from the palette button under the brief's references (Set up, while the project has none). It holds up to 4 images, one style line ("frosted glass, cool night light, chunky silhouettes") and up to 6 palette colours. Add images from the project's concepts and refs, or choose a file; each is copied into the project's `style/` folder. A concept's palette button adds it to the board in one click. Save board writes `style.json` at the project root and bumps its revision. Escape closes the board, and Ctrl+Enter (Cmd+Enter on a Mac) saves it.

Once the board has anything on it, every new direction and object ref carries it. The style line and palette go after the brief's own prompt, and the board's images are attached after the brief's own references, so the brief leads where the two disagree. Turn off Use the project style in the brief, or in Make object refs, to make one without it. A direction shows a chip with the revision its latest images carried, or Own style when they were made without it. Retry runs a job with the same board revision it started with.

## Object refs

Make object refs turns one concept into a clean reference image per object, for a scene with many props. List up to 12 objects, add optional notes per object, pick the aspect (1:1 by default) and whether they become props or characters. Each object is one Codex job that asks for that object alone, centered, in a plain light grey background, with no text. The images land in a new direction, "name objects".
