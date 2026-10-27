---
title: Updates
description: How Maquette updates itself once it is released, what it asks the network, and why it does nothing yet.
sidebar:
  label: Updates
---

Maquette checks for a newer version of itself, downloads it when you say so, and restarts into it. Nothing installs without a click.

## Until the first release

The app carries the key that signs updates, so a release build already checks for them. There is no public release to update to yet: until the first one, a check finds nothing and Settings > About says no release is out for this computer yet. Until then, run a new version from source as in [Install](/docs/start/install/).

## How it works once it is on

- **The feed.** Every release on GitHub carries a `latest.json` file that names the version, its notes and one download per platform. The app reads `https://github.com/Shironex/maquette/releases/latest/download/latest.json`.
- **When it checks.** Ten seconds after launch, then every six hours, and whenever you press Check now in Settings > About. A check that fails offline stays quiet.
- **When it finds one.** A small "Update 0.2.0" pill shows by the logo in the sidebar. Click it, or open Settings > About, to read the release notes.
- **Download.** About shows the progress. Every download is checked against the app's signing key before it can install; a file that does not match is refused.
- **Restart to update.** The app installs the update and restarts. It waits while a pipeline run or a build is going, or while changes are not applied yet, and says why.

A release that is still uploading its files for your computer counts as "no release yet", not as an error.

## What stays as it was

Updates replace the app only. Your workspace folder, your projects, the app settings and the API keys in the OS keychain stay where they are. Starter scripts in a project's `tools/` are never overwritten: the Pipeline view offers to update them, as it does today.

## What it sends

A check is one plain request for `latest.json` to GitHub, and a download fetches the file it names. Maquette sends no usage data, no account and no ID.

## When it is off

| Case                               | Why                                        |
| ---------------------------------- | ------------------------------------------ |
| No signing key set in the app      | A build made without the key in its config |
| A development build (`pnpm dev`)   | It never replaces itself                   |
| `MAQUETTE_NO_UPDATER=1` in the env | Turned off for one run, for scripted use   |
| Mock mode (`pnpm dev:web`)         | There is no app to update                  |

## macOS and Windows

The first install comes from a browser download, so macOS asks you to allow it under System Settings > Privacy & Security, and Windows SmartScreen asks once. Updates after that come through the app and do not ask again.
