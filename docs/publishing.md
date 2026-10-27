# Publishing checklist

What to do, in order, when the repository goes from private to public. Everything here is a manual
step in GitHub's settings or a command to run by hand; nothing in the repo does it automatically.
Tick the boxes as you go.

The license is decided as AGPL-3.0 and is swapped in as its own step (the `LICENSE` file and the
`license` field of `package.json` and `Cargo.toml`). Do that before the repo turns public.

## Before the switch

- [ ] `pnpm gate` is green on `main`, and the tree has no secrets: run `gitleaks detect` over the
      full history, not only the working tree.
- [ ] The license swap is merged (`LICENSE`, `package.json`, `Cargo.toml`, README badge).
- [ ] The README, the docs and `apps/site` name no private repo, local path or personal account
      other than the public GitHub owner.
- [ ] The landing page's `SITE_URL` (`apps/site/src/lib/site-url.ts`) is the address it will be
      served from.

## Repository settings

Settings > General:

- [ ] **Description:** the one line that also leads the README: a desktop studio that turns concept
      art into optimised 3D assets for real-time web scenes.
- [ ] **Website:** `https://shironex.github.io/maquette/`.
- [ ] **Topics:** `tauri`, `rust`, `react`, `typescript`, `desktop-app`, `3d`, `gltf`, `glb`,
      `blender`, `threejs`, `web3d`, `mcp`, `asset-pipeline`.
- [ ] **Social preview:** upload a 1280 x 640 image under Settings > General > Social preview.
      GitHub has no API for it, so it is a by hand upload. Use the hero image from the README so the
      two match.
- [ ] **Features:** Issues on, Discussions on, Wikis off, Projects off.
- [ ] **Pull requests:** allow squash merging only, default the commit message to the PR title, and
      turn on "Automatically delete head branches".

## Discussions

Settings > General > Features > Discussions, then Discussions > Edit categories. GitHub has no API
to create categories, so use the defaults and make sure these exist:

- [ ] **Announcements** (maintainers only)
- [ ] **Q&A** (answerable)
- [ ] **Ideas** (the issue forms send large feature requests here)
- [ ] **Show and tell** (finished scenes and workflows)
- [ ] **General**

## Rules for `main`

Settings > Rules > Rulesets > New branch ruleset, named `Protect main`, enforcement Active, target
the default branch:

- [ ] Restrict deletions.
- [ ] Block force pushes (non fast forward).
- [ ] Require a pull request before merging, zero required approvals (a single maintainer cannot
      approve their own PR), allow squash only.
- [ ] Require status checks to pass, once the CI workflows exist (a separate slice). Add each job
      name as it appears in a PR's checks; do this after the first green run so the names are
      selectable.
- [ ] Leave bypass empty.

## Security

Settings > Code security:

- [ ] **Private vulnerability reporting:** enable. `SECURITY.md` and the issue chooser link to
      `/security/advisories/new`, which only works with this on.
- [ ] **Dependabot alerts** and **Dependabot security updates:** enable. `.github/dependabot.yml`
      covers the weekly version updates.
- [ ] **Secret scanning** and **push protection:** enable.
- [ ] **Code scanning:** optional, once CI exists.

## Labels

The label set lives in `.github/labels.yml` and is applied with the GitHub CLI. Sign in first with
`gh auth login` as the account that owns the repo.

```sh
node scripts/github/apply-labels.mjs --dry-run              # print the gh commands, change nothing
node scripts/github/apply-labels.mjs --repo Shironex/maquette
```

It uses `gh label create --force`, so a label that exists (the GitHub defaults such as `bug`) is
updated to the listed color and description, and a label that is not listed is left alone. It runs
the same on macOS and Windows.

- [ ] Labels applied. Check `gh label list` for the `area:*`, priority and type labels.
- [ ] Dependabot's own labels (`dependencies`, `javascript`, `rust`, `github_actions`) are in the
      set, so its PRs are labelled on the first run.

## Releases

Maquette is a desktop app, so a release is one version for the whole app.

- **Tag:** `vX.Y.Z` (semver, a leading `v`), for example `v0.2.0`. The tag is created from a commit
  on `main` after the version is bumped in all four places that carry it: the root `package.json`,
  `apps/desktop/package.json`, `apps/desktop/src-tauri/tauri.conf.json` and `[workspace.package]` in
  the root `Cargo.toml` (the crates inherit it).
- **Release title:** `vX.Y.Z: short summary`, for example `v0.2.0: Motion view and Meshy remesh`.
- **Release notes:** a short intro, then Highlights, What's New per feature, then Bug Fixes,
  Performance, Under the Hood and Dependencies, written from the real git history.
- **Pre-releases:** `vX.Y.Z-rc.N` or `-beta.N`, marked as a pre-release.
- **Assets:** the macOS and Windows installers, and the update manifest and signatures once the
  updater exists. They are produced by the release workflow (a separate slice).

- [ ] The first public release is drafted and not published until the installers are attached.

## Last look

- [ ] Open the repo in a signed out browser: README renders, the issue chooser shows the forms and
      the Discussions and docs links, and blank issues are not offered.
- [ ] Switch the visibility to Public (Settings > Danger zone).
- [ ] Confirm the docs site is published and the Website link works.
