# Security

This file says what counts as a vulnerability in Maquette, what does not, and where to send a
report.

## What counts

Maquette runs on your machine with your privileges, and it drives other programs and talks to
other services. These are the places where a flaw matters:

- **API keys and tokens.** Provider keys live in the OS keychain and the agent token in the app
  state. Anything that writes one to the workspace, a project file, a log, a screenshot or a
  network request to another host is a vulnerability.
- **The MCP server.** It is off by default, listens on `127.0.0.1` only and wants a bearer token.
  A way to reach it from another machine or from a web page, to use it without the token, or to
  make an agent do something the app's approval prompts (Ship, spending provider credits) are
  meant to stop, is a vulnerability.
- **Running programs.** Pipeline steps spawn Blender, `toktx`, `gltf-transform`, `ffmpeg` and
  project scripts with an args array, never through a shell. A way to make Maquette run something
  the project did not ask for, or to inject arguments from a model file, a file name or a site's
  `maquette.site.json`, is a vulnerability.
- **Files outside the workspace.** Ship, Remove from project, the Downloads watch and the model
  inbox read and write files. A path that escapes the workspace or the chosen site or game repo
  (traversal, a symlink followed out, an overwrite of a file Maquette did not ship) is a
  vulnerability.
- **Opening untrusted models.** A crafted `.glb` or `.gltf` that crashes the reader is a normal
  bug; one that runs code or reads files is a vulnerability.
- **The release artefacts.** An installer or update that does not match the tagged source, or a
  way to make the update path deliver something else.

## What does not count

- A pipeline script you wrote or edited doing what it says. Scripts in a project's `tools/` folder
  are yours and run with your privileges, like any build script.
- A provider's own model or API behavior.
- Advisories in the development toolchain that do not ship in the app. Send them as a normal issue
  or a PR bumping the lockfile.

## Reporting

Use GitHub's private vulnerability reporting for this repository:
<https://github.com/Shironex/maquette/security/advisories/new>. It opens a draft advisory that only
the maintainer can see.

If that page says reporting is not enabled, open a regular issue whose entire body is "security
report, need a private channel", with no details, and the maintainer will reply with one. Do not
put the finding in a public issue.

Include the version, your OS, the steps that trigger it and what it does. There is no bounty. You
will get a reply from the maintainer, a fix in the next release with a changelog entry that
credits you unless you ask otherwise, and the details published once the fix is out.

## Supported versions

Only the latest release. Maquette is `0.x`; fixes ship forward, never as a backport.
