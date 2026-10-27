# Publishing checklist

What to do, in order, when the repository goes from private to public. Everything here is a manual
step in GitHub's settings or a command to run by hand; nothing in the repo does it automatically.
Tick the boxes as you go.

The code is licensed AGPL-3.0-only (`LICENSE`, and the `license` field of `package.json` and
`Cargo.toml`). The artwork is reserved and listed in `ARTWORK.md`; a new reserved file outside
those folders gets a line there.

## Before the switch

- [ ] `pnpm gate` is green on `main`, and the tree has no secrets: run `gitleaks detect` over the
      full history, not only the working tree.
- [x] The license swap is merged (`LICENSE`, `ARTWORK.md`, `package.json`, `Cargo.toml`, README badge).
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
- [ ] Require status checks to pass. Add them after the first green run on a pull request, so
      GitHub can offer the names (each is a job name, see [CI](#ci)): - `lint · format` - `typecheck` - `test · web` - `build · site` - `rust · macos-latest` - `rust · windows-latest` - `pr title`

      Leave `area labels` and `issue labels` out: they only add labels, and `area labels` skips
      on an edit of the title or body.

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

## CI

Every workflow lives in `.github/workflows/`, pins each action to a full commit SHA (the version
is in a comment next to it, and Dependabot bumps both), and asks for the least permissions it
needs.

| Workflow        | Runs on                                    | Jobs (check names)                                                                                         |
| --------------- | ------------------------------------------ | ---------------------------------------------------------------------------------------------------------- |
| `ci.yml`        | pull requests, pushes to `main`            | `lint · format`, `typecheck`, `test · web`, `build · site`, `rust · macos-latest`, `rust · windows-latest` |
| `pr-title.yml`  | pull requests (title and every new commit) | `pr title`                                                                                                 |
| `pr-labels.yml` | pull requests                              | `area labels`, `issue labels`                                                                              |
| `release.yml`   | pushed `v*` tags, or by hand as a dry run  | `check · version`, `build · macos-aarch64`, `build · windows-x86_64`, `verify · draft`, `publish`          |

- **`ci.yml`** is `pnpm gate` split into parallel jobs. The Node jobs run on Linux; the Rust jobs
  (clippy with warnings denied, then `cargo test`) run on macOS and Windows, the systems the app
  ships on, with a Rust cache that only `main` writes. Tests that need Blender skip on their own,
  as they do on any machine without it. After the tests, the macOS job fails if
  `apps/web/src/ipc/generated` changed, which means the IPC bindings were not regenerated and
  committed. A new push to a PR cancels the run it replaces.
- **`pr-title.yml`** checks the title the squash commit takes: `type(scope): lowercase summary`
  with a type from `feat`, `fix`, `chore`, `docs`, `refactor`, `test`, `ci`, `build`, `perf`, a
  scope from `commitlint.config.mjs` (always required), and at most 100 characters. A new scope
  goes into both files. Dependabot's titles (`ci(deps): ...`, `build(deps): ...`) pass as they are.
- **`pr-labels.yml`** adds `area:*` labels from `.github/labeler.yml` and copies the labels of the
  issue a PR closes (`Closes #N`), except the triage ones (`question`, `needs repro`,
  `good first issue`, `help wanted`, `duplicate`, `invalid`, `wontfix`). It never removes a label.

The title and label workflows run on `pull_request_target`, so a PR from a fork can be labelled and
cannot change the check that runs on it. They never check out the PR's code; keep it that way
(the header of each file says why).

## Releases

Maquette is a desktop app, so a release is one version for the whole app.

- **Tag:** `vX.Y.Z` (semver, a leading `v`), for example `v0.2.0`. The tag is created from a commit
  on `main` after the version is bumped in all four places that carry it: the root `package.json`,
  `apps/desktop/package.json`, `apps/desktop/src-tauri/tauri.conf.json` and `[workspace.package]` in
  the root `Cargo.toml` (the crates inherit it).
- **Release title:** `vX.Y.Z: short summary`, for example `v0.2.0: Motion view and Meshy remesh`.
- **Release notes:** `docs/releases/vX.Y.Z.md`, committed before the tag. Its first line,
  `# vX.Y.Z: short summary`, becomes the release title, and the rest the notes (on the release page
  and in About, through `latest.json`). A short intro, then Highlights, What's New per feature,
  then Bug Fixes, Performance, Under the Hood and Dependencies, written from the real git history.
  Without the file the release gets the title `vX.Y.Z` and a one line default body.
- **Pre-releases:** `vX.Y.Z-rc.N` or `-beta.N`, marked as a pre-release.
- **Assets:** produced by `release.yml`: `Maquette_X.Y.Z_aarch64.dmg` (macOS, Apple silicon) and
  `Maquette_X.Y.Z_x64-setup.exe` (Windows) to install, `Maquette_X.Y.Z_aarch64.app.tar.gz` and the
  setup `.exe` again as the update files, a `.sig` for each update file, and `latest.json`.

- [ ] The first public release is drafted and not published until the installers are attached.

## Updates and signing

The app updates itself through `tauri-plugin-updater` (see `apps/desktop/src-tauri/src/updater/`).
It reads `latest.json` from the latest GitHub release and installs only files signed with the app's
own minisign key. Until a public key is set, the updater is off: the plugin is not even registered,
no check runs, and Settings > About says updates arrive with the first public release.

### Generate the key, once, by hand

Do this on one machine, yourself. An agent never generates, reads or commits the key.

```sh
pnpm tauri signer generate -w ~/.maquette-keys/updater.key
```

It asks for a password; use a long one. It writes `updater.key` (the private key) and
`updater.key.pub` (the public key), and prints both.

- [ ] Store the private key file and its password in the password manager.
- [ ] Keep one more copy offline (an encrypted USB stick or a printed copy of the key text).
- [ ] Never commit the private key, never paste it into an issue or a chat, and never reuse it for
      another app.

**Losing the private key means installed copies can never be updated again.** Every install trusts
only this key, and a new key reaches users only through an update signed with the old one. Rotating
works only while the old key is still at hand: ship one release signed with the old key whose config
carries the new public key, then sign with the new key from the next release on.

### Turn updates on

Done on 2026-10-27: `pubkey` under `plugins.updater` in `apps/desktop/src-tauri/tauri.conf.json`
holds the whole content of `updater.key.pub` (one base64 line, key id DDAA4D2DC9F23829), next to
the endpoint `https://github.com/Shironex/maquette/releases/latest/download/latest.json`. Release
builds now check the feed; a debug build and `MAQUETTE_NO_UPDATER=1` still keep it off. Until the
first public release the feed answers 404, which About shows as no release out yet. The test
`the_shipped_config_has_the_feed_and_the_key` in `apps/desktop/src-tauri/src/updater/tests.rs` pins
it. A new key means changing both in one commit, and only through the rotation above.

### Build the signed update files

`bundle.createUpdaterArtifacts` is left out of `tauri.conf.json` on purpose. With it on and the
public key set, the Tauri CLI stops a build that has no private key ("A public key has been found,
but no private key"), and a local `pnpm tauri build` should never need the key. The release build
turns it on with the overlay `apps/desktop/src-tauri/tauri.updater.conf.json` and gives it the key
through the environment. `release.yml` does exactly this; by hand it is:

```sh
export TAURI_SIGNING_PRIVATE_KEY="$(cat ~/.maquette-keys/updater.key)"
export TAURI_SIGNING_PRIVATE_KEY_PASSWORD="..."
pnpm tauri build --config src-tauri/tauri.updater.conf.json
```

(`pnpm tauri` runs in `apps/desktop`, so the path is relative to it.)

On Windows PowerShell, set the two variables with `$env:TAURI_SIGNING_PRIVATE_KEY = Get-Content
-Raw $HOME\.maquette-keys\updater.key` and `$env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = "..."`.

That writes, next to the installers, `Maquette.app.tar.gz` and its `.sig` on macOS, and the NSIS
`*-setup.exe` and its `.sig` on Windows. The `.sig` content goes into `latest.json`:

```json
{
  "version": "0.2.0",
  "notes": "The release notes, as plain text or Markdown.",
  "pub_date": "2026-11-03T12:00:00Z",
  "platforms": {
    "darwin-aarch64": {
      "signature": "<content of the .app.tar.gz.sig>",
      "url": "https://github.com/Shironex/maquette/releases/download/v0.2.0/Maquette_0.2.0_aarch64.app.tar.gz"
    },
    "windows-x86_64": {
      "signature": "<content of the -setup.exe.sig>",
      "url": "https://github.com/Shironex/maquette/releases/download/v0.2.0/Maquette_0.2.0_x64-setup.exe"
    }
  }
}
```

The release workflow writes `latest.json` with `scripts/release/latest-json.mjs`, in one job after
both builds, and uploads it as an asset of the release. The app reads `notes` as the release notes
it shows in About. Renaming an artifact keeps its signature valid (it covers the bytes, not the name).

Release builds stamp the commit and build date into About from git; the release workflow passes
them as `MAQUETTE_COMMIT` and `MAQUETTE_BUILD_DATE`.

### CI secrets, for the release workflow

`release.yml` reads these from a GitHub environment named `release`, limited to `v*` tags with you
as the required reviewer:

| Secret                               | Value                          |
| ------------------------------------ | ------------------------------ |
| `TAURI_SIGNING_PRIVATE_KEY`          | The full text of `updater.key` |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Its password                   |

`scripts/github/setup-release-env.sh` does both steps below (and is safe to run again): it reads
the key from `~/.maquette-keys/updater.key` and asks for the password without printing either.

- [ ] Environment `release` created, deployment tags `v*`, required reviewer set.
- [ ] Both secrets added to that environment (not to the repository secrets).

### The first release with updates

- [ ] The repository is public: release assets of a private repo answer 404, which the app treats
      as "no release yet".
- [ ] The public key is in `tauri.conf.json` before the first installer is built. An install
      without the key can never update itself; its users must reinstall by hand once.
- [ ] macOS is ad-hoc signed as a whole bundle (`signingIdentity: "-"`, already set); Windows
      ships unsigned for now. The first install shows Gatekeeper's "Open Anyway" or SmartScreen
      once; updates through the app do not.
- [ ] Publish the release (not a draft, not a pre-release): `releases/latest` ignores both.
- [ ] Prove it: install version N, release N+1, and check that the installed app offers it,
      downloads it, restarts into it, and keeps the workspace and the keychain keys.
- [ ] Never re-release the same version number; bump it for every fix users must get.

## Cut a release

1. **Bump the version** in every file that carries it, on a branch:

   ```sh
   node scripts/release/bump-version.mjs 0.2.0
   ```

   It sets the root `package.json`, `apps/desktop/package.json`, `tauri.conf.json` and
   `[workspace.package]` in `Cargo.toml`, plus the crates' entries in `Cargo.lock`.
   `node scripts/release/bump-version.mjs --check v0.2.0` is the same check the release runs on
   the tag.

2. **Write the notes** in `docs/releases/v0.2.0.md`, starting with `# v0.2.0: short summary`.
3. **Merge** the PR (for example `build(workspace): release 0.2.0`) once CI is green.
4. **Tag** the merge commit on `main` and push the tag:

   ```sh
   git switch main && git pull
   git tag v0.2.0 && git push origin v0.2.0
   ```

5. **Approve the environment.** The run (Actions > release) first checks that the tag matches the
   version in every file, and fails with the files that disagree if not. Both build jobs then wait
   for `release`; approve them together under "Review deployments".
6. **Check the draft.** When the builds finish, `verify · draft` verifies every `.sig` against the
   public key in `tauri.conf.json`, writes `latest.json` and uploads everything to a draft
   release. Open it under Releases: title, notes, the `.dmg`, the setup `.exe`, both update files
   with their `.sig`, and `latest.json` with `darwin-aarch64` and `windows-x86_64`.
7. **Published.** `publish` waits for a second approval of `release`. Approve it once the draft
   looks right: it downloads the draft's files, verifies the signatures again and publishes the
   release, marked as latest (a `-rc.N` or `-beta.N` version is published as a pre-release, which
   the updater skips).

If a build fails, fix it on `main`, bump to the next patch version and tag again; never move a
tag that was pushed. Rerunning the failed jobs of the same run is fine while the release is still
a draft.

**Dry run.** Actions > release > Run workflow, from `main` or any branch, checks that the version
files agree and builds both installers, unsigned (the `release` environment admits only `v*`
tags, so the key is not available). The installers stay on the run as artifacts for two weeks.
It never creates or publishes a release.

## Last look

- [ ] Open the repo in a signed out browser: README renders, the issue chooser shows the forms and
      the Discussions and docs links, and blank issues are not offered.
- [ ] Switch the visibility to Public (Settings > Danger zone).
- [ ] Confirm the docs site is published and the Website link works.
