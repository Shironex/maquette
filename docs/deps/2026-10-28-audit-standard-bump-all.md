# Dependency Safety Report

**Date:** 2026-10-28
**Agent:** kirei-deps
**Depth:** standard (plus per-major call-site analysis and a compile probe)
**Package manager:** pnpm 10.34.5 (JS), cargo 1.94.1 (Rust)
**Scope:** full tree, JS workspaces (root, apps/web, apps/site, apps/desktop, packages/ui) and Rust workspace (crates/maquette-core, apps/desktop/src-tauri), plus pinned GitHub Actions

Age rule used everywhere: a version must be published before 2026-10-21 13:53 UTC (7 days before the time of this run). Anything newer is listed as "too young" with its fallback.

## Summary

JS: 8 advisories in the lockfile (6 moderate, 2 low, 0 high, 0 critical), all dev-only: `@vitest/mocker` 4.1.11 (fixed in 4.1.12 and 5.0.3, so vitest 5.0.5 clears it) and transitive `fast-uri` 3.1.8 under commitlint (fixed in 3.1.9, published 2026-10-19). Rust: 8 RustSec vulnerabilities and 9 warnings. The vulnerabilities are `rustls-webpki` 0.103.15, `webpki-root-certs` 1.0.9, and `zip` 4.6.1 plus 8.6.0 (RUSTSEC-2026-0367/0368/0369, fixed in zip 9). The unsound `tokio` 1.53.1 is fixed in 1.53.3. Applying the plan below leaves one residual: `zip` 4.6.1 pulled by `tauri-plugin-updater` (cannot be fixed from this repo, see Residual risk). Recommended next step: one branch applying everything below, with the five code fixes listed under "Majors".

Dependabot PRs: do NOT merge or cherry-pick PR 12's Cargo.lock. It moves transitive crates younger than 7 days (brotli 9.1.1, brotli-decompressor 6.2.0, rmcp-macros 3.8.0, wry 0.57.1, tao 0.37.2). Use the targeted `cargo update` commands below. Also PR 12 and PR 13 leave `tauri` crate on 2.11.x while the JS `@tauri-apps/api` / `@tauri-apps/cli` go to 2.12.x, a minor mismatch; and `tauri-plugin-updater` 2.14.0 requires `tauri ^2.12`. So the Rust `tauri` must move to 2.12.5.

## Audit Results

### JS (`pnpm audit`)

| Severity | Count | Direct | Transitive |
| -------- | ----- | ------ | ---------- |
| Critical | 0     | 0      | 0          |
| High     | 0     | 0      | 0          |
| Moderate | 6     | 0      | 6          |
| Low      | 2     | 0      | 2          |

| Package          | Current | Vulnerable             | Fixed in       | GHSA                                                                                                | Path                                                              |
| ---------------- | ------- | ---------------------- | -------------- | --------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------- |
| `@vitest/mocker` | 4.1.11  | >=2.1.0-beta.7 <4.1.12 | 4.1.12 / 5.0.3 | GHSA-qxg6-89mg-rqv8, GHSA-xp26-3454-mjgp                                                            | vitest (dev, root and apps/site)                                  |
| `fast-uri`       | 3.1.8   | >=3.0.0 <3.1.9         | 3.1.9          | GHSA-w74x-v6f9-j3xg, GHSA-wjr9-j795-jxcr (moderate), GHSA-xhq7-xmqj-769h, GHSA-333x-w4r6-6crp (low) | @commitlint/cli > load > config-validator > ajv 8.20.0 > fast-uri |

All dev-only. The vitest one only matters while the vitest dev server is reachable.

### Rust (`cargo audit`, RustSec db current as of run; cargo-audit 0.x installed at /opt/homebrew/bin)

| Crate                       | Current                       | Advisory                                                                             | Fixed in                                     | Pulled by                                                                   |
| --------------------------- | ----------------------------- | ------------------------------------------------------------------------------------ | -------------------------------------------- | --------------------------------------------------------------------------- |
| `rustls-webpki`             | 0.103.15                      | RUSTSEC-2026-0382 distrusted intermediates accepted                                  | 0.103.16 (2026-10-20, ok)                    | rustls via reqwest, rmcp, updater                                           |
| `webpki-root-certs`         | 1.0.9                         | RUSTSEC-2026-0383 distrusted emSign intermediates                                    | 1.0.10 (2026-10-19, ok; 1.0.11 is too young) | rustls-platform-verifier chain                                              |
| `zip`                       | 8.6.0 (direct, maquette-core) | RUSTSEC-2026-0367 (SUID/SGID), 0368 (symlink targets), 0369 (perms through symlinks) | 9.0.0, take 9.0.5                            | direct                                                                      |
| `zip`                       | 4.6.1                         | same three                                                                           | 9.0.0                                        | `tauri-plugin-updater` (optional dep, feature `zip`)                        |
| `tokio`                     | 1.53.1                        | RUSTSEC-2026-0386 unsound `AsyncFd::new` (warning)                                   | 1.53.3 (2026-10-13, ok)                      | direct                                                                      |
| `glib`                      | 0.18.5                        | RUSTSEC-2024-0429 unsound (warning), also Dependabot GHSA-wrw7-89jp-8q8g             | 0.20                                         | gtk3 stack under tauri/muda on Linux, not fixable until tauri moves to gtk4 |
| `paste`, `proc-macro-error` | 1.0.15, 1.0.4                 | unmaintained                                                                         | none                                         | specta, transitive                                                          |
| `unic-*` (5 crates)         | 0.9.0                         | unmaintained                                                                         | none                                         | cleared in the probe lock after the tauri 2.12 update                       |

Note: zip is only used here to read archives (`ZipArchive::new`, `by_name`, `file_names`, `size`), never `extract`, so the three zip CVEs are not reachable from maquette-core's own code. The bump is still right and cheap.

## Dependabot Alerts

`gh` is authed and the repo is on GitHub. 7 open alerts, all already in the audits above: fast-uri x4, @vitest/mocker x2, and `glib` GHSA-wrw7-89jp-8q8g (rust, runtime scope, medium, no fix before glib 0.20). No alerts that the local audits miss.

Open Dependabot PRs: 2, 4, 5, 6, 7, 9, 10, 12, 13.

## Dependabot PR review (CI as of this run)

| PR  | Change                                                              | CI                                       | Verdict                                                                                                                                                                                                                                  |
| --- | ------------------------------------------------------------------- | ---------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 2   | Swatinem/rust-cache 2.9.2 to 2.9.3                                  | no checks reported (actions-only branch) | Safe. SHA `d1527c8f67ef9af357290475d4af4b0fab8f12e0`, tag v2.9.3 published 2026-10-20, commit date 2026-10-20 13:45 UTC. Edit `.github/workflows/ci.yml:158` to `Swatinem/rust-cache@d1527c8f67ef9af357290475d4af4b0fab8f12e0 # v2.9.3`. |
| 4   | zip 8.6.0 to 9.0.5                                                  | rust macOS and Windows fail              | Major, needs 2 code fixes. 9.0.5 is old enough (2026-10-17); 9.1.x (10-24 and later) is too young.                                                                                                                                       |
| 5   | getrandom 0.3.4 to 0.4.3                                            | all green                                | Safe. `getrandom::fill` call sites compile unchanged.                                                                                                                                                                                    |
| 6   | sha2 0.10.9 to 0.11.1                                               | rust macOS and Windows fail              | Major, needs 3 code fixes.                                                                                                                                                                                                               |
| 7   | base64 0.22.1 to 0.23.1                                             | all green                                | Safe. The one use (`Engine as _`, `general_purpose::STANDARD.encode`) compiles unchanged.                                                                                                                                                |
| 9   | react, react-dom 19.3.0 to 19.3.1, @tauri-apps/api 2.11.1 to 2.12.4 | all green                                | Safe (React 19.3.1 is an RSC transport fix only). Pair with the tauri crate 2.12.x bump.                                                                                                                                                 |
| 10  | vitest 4.1.11 to 5.0.5                                              | test (web) fails, 1 of 1410 tests        | Major, needs 1 test fix. Also clears the two @vitest/mocker advisories.                                                                                                                                                                  |
| 12  | rust-dependencies, 13 updates                                       | rust Windows fail, macOS green           | The Windows failure is a flaky test, see below. Direct bumps are fine, but its lockfile is not (young transitives, see Summary).                                                                                                         |
| 13  | dev-dependencies, 14 updates                                        | rust Windows fail, macOS green           | The same flaky Windows test; this PR touches no Rust files at all. JS content is safe.                                                                                                                                                   |

Windows flake (PRs 12 and 13): `site::preview::server::tests::a_server_starts_reports_its_address_and_stops` panics at `crates/maquette-core/src/site/preview/server/tests.rs:70` (`assert!(heard_running())` after a 5 s wait). PR 13 changes only JS files and fails the same way, PRs 5, 7, 9, 10 pass the same Windows job, so it is a pre-existing timing flake, not caused by the bumps. Last touched in commit 691c8d50 ("wait for the preview listener to hear the running state"). Expect it to bite again; re-run the Windows job before blaming a bump, and consider raising the 5 s deadline at tests.rs:68-70.

## Safe Bumps (Phase 1, single commit group)

All JS direct deps are exact-pinned in `package.json`; every bump below is an edit of the exact version. Do it per-workspace file, then `pnpm install`.

| Package                           | Current | Target         | Type  | Resolves CVE?         | Files                            | Notes                                                                                                                                                             |
| --------------------------------- | ------- | -------------- | ----- | --------------------- | -------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `@commitlint/cli`                 | 21.2.3  | 21.2.4         | patch | no (but see fast-uri) | package.json                     | published 10-18                                                                                                                                                   |
| `@commitlint/config-conventional` | 21.2.3  | 21.2.4         | patch | no                    | package.json                     |                                                                                                                                                                   |
| `eslint`                          | 10.11.0 | 10.13.0        | minor | no                    | package.json                     | 10-16                                                                                                                                                             |
| `globals`                         | 17.12.0 | 17.13.0        | minor | no                    | package.json                     | 10-01                                                                                                                                                             |
| `lint-staged`                     | 17.5.1  | 17.6.0         | minor | no                    | package.json                     | 09-26                                                                                                                                                             |
| `prettier`                        | 3.9.9   | 3.9.10         | patch | no                    | package.json                     | 10-10; run `pnpm format:check`                                                                                                                                    |
| `typescript-eslint`               | 8.70.1  | 8.72.0         | minor | no                    | package.json                     | 8.72.1 (10-26) is too young                                                                                                                                       |
| `react`, `react-dom`              | 19.3.0  | 19.3.1         | patch | no                    | apps/web, apps/site, packages/ui | 10-14. `peerDependencies` `^19.2.0` in packages/ui stays                                                                                                          |
| `@tauri-apps/api`                 | 2.11.1  | 2.12.4         | minor | no                    | apps/web/package.json            | 10-14. Must match the Rust `tauri` 2.12 minor                                                                                                                     |
| `@tauri-apps/cli`                 | 2.11.5  | 2.12.4         | minor | no                    | apps/desktop/package.json        | 10-21 12:20 UTC, old enough (Dependabot offers 2.12.3)                                                                                                            |
| `vite`                            | 8.3.4   | 8.3.5          | patch | no                    | apps/web, apps/site              | 10-21 06:38 UTC, ok                                                                                                                                               |
| `@testing-library/user-event`     | 14.6.7  | 14.6.8         | patch | no                    | apps/web, packages/ui            |                                                                                                                                                                   |
| `@vitejs/plugin-react`            | 6.1.1   | 6.1.2          | patch | no                    | apps/web                         |                                                                                                                                                                   |
| `jsdom`                           | 30.1.1  | 30.1.3         | patch | no                    | apps/web, packages/ui            |                                                                                                                                                                   |
| `astro`                           | 7.4.0   | 7.4.1          | patch | no                    | apps/site                        | 7.4.2 is 2026-10-21 14:18 UTC, only about 25 minutes short of 7 days at run time. Use 7.4.2 only if `date -u` is past 2026-10-28 14:20 when executing; else 7.4.1 |
| `@astrojs/check`                  | 0.9.10  | 0.9.11         | patch | no                    | apps/site                        | 0.9.12 is 10-21 14:13 UTC, same rule                                                                                                                              |
| `@astrojs/markdown-satteri`       | 0.4.4   | 0.4.4 or 0.4.5 | patch | no                    | apps/site                        | 0.4.5 is 10-21 14:13 UTC, same rule                                                                                                                               |
| `@astrojs/react`                  | 7.1.0   | 7.1.0 or 7.1.1 | patch | no                    | apps/site                        | 7.1.1 is 10-21 14:16 UTC, same rule                                                                                                                               |
| `fast-uri` (transitive)           | 3.1.8   | 3.1.9          | patch | yes, 4 advisories     | pnpm-lock.yaml                   | `pnpm update fast-uri -r` (ajv allows `^3`). If pnpm refuses, add `pnpm.overrides` `"fast-uri": "3.1.9"` to package.json                                          |

Too young, leave pinned and revisit after the date shown: `zustand` 5.0.16 (ok after 2026-10-29 04:32 UTC), `@astrojs/starlight` 0.42.7 (ok after 2026-10-30 15:16 UTC), `typescript-eslint` 8.72.1 (ok after 2026-11-02).

Already at latest: all `@noctcore/*`, `@eslint/js`, `eslint-config-prettier`, `eslint-plugin-react(-hooks)`, `husky`, `playwright`, `@react-three/fiber`, `three`, `@types/three`, `@types/react(-dom)`, tailwind stack, testing-library dom/react/jest-dom, `@astrojs/sitemap`, `@astrojs/starlight-tailwind`, `clsx`, `tailwind-merge`, `@phosphor-icons/core`.

Not bumped by policy: `typescript` 6.0.3 to 7.0.2 and `@types/node` 22.20.5 to 26.x (both ignored in `.github/dependabot.yml`; @types/node tracks `.nvmrc` 22). Backlog only.

GitHub Actions: only `Swatinem/rust-cache` has a newer release. Every other pinned action (checkout v7.0.1, setup-node v7.1.0, pnpm/action-setup v6.1.0, upload-artifact v7.0.2, download-artifact v8.0.2, tauri-action v1.0.0, labeler v7.0.0, action-semantic-pull-request v6.1.1, configure-pages v6.0.0, upload-pages-artifact v5.0.0, deploy-pages v5.0.1) is at the latest stable release.

### Rust Phase 1 (Cargo.toml edits)

| Dependency             | Cargo.toml line (workspace)        | Change                                                                                                                                                    |
| ---------------------- | ---------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `rmcp`                 | `rmcp = { version = "=3.5.1" ...}` | `=3.6.0` (published 10-20, now old enough). 3.7.0 and 3.8.0 are too young. Update the comment above it (it says 3.6 and 3.7 are younger than seven days). |
| `tauri`                | `tauri = { version = "2.11", ...}` | `"2.12"`, lock to 2.12.5 (2026-10-21 11:54 UTC)                                                                                                           |
| `tauri-plugin-dialog`  | `"2.7"`                            | `"2.8"`, lock to 2.8.3 (2.8.4 is 10-28, too young)                                                                                                        |
| `tauri-plugin-updater` | `"=2.12.0"`                        | `"=2.14.0"` (2.14.1 is 10-28, too young). Fix the comment: 2.13+ needs tauri 2.12, which is now satisfied.                                                |
| `zip`                  | `"=8.6.0"`                         | `"=9.0.5"`, fix the comment about 9.x being too young                                                                                                     |
| `sha2`                 | `"0.10.9"`                         | `"0.11.1"` (2026-10-12)                                                                                                                                   |
| `getrandom`            | `"0.3.4"`                          | `"0.4.3"`                                                                                                                                                 |
| `base64`               | `"0.22.1"`                         | `"0.23.1"`                                                                                                                                                |

Lock-only bumps (no Cargo.toml edit needed): serde_json 1.0.154, thiserror 2.0.21, schemars 1.2.3, tokio 1.53.3, hyper 1.12.0, hyper-util 0.1.21, http-body-util 0.1.5, tokio-util 0.7.20, flate2 1.1.10, tauri-build 2.7.4, rustls-webpki 0.103.16, webpki-root-certs 1.0.10.

Exact command sequence that was verified in a scratch copy (cargo check --workspace --all-targets passes with the code fixes below; `cargo audit` afterwards shows only the zip 4.6.1 residual plus glib/paste/proc-macro-error warnings). Run after the Cargo.toml edits:

```
cargo update -p tauri --precise 2.12.5
cargo update -p tauri-build --precise 2.7.4
cargo update -p tauri-plugin-dialog --precise 2.8.3
cargo update -p tokio --precise 1.53.3
cargo update -p rustls-webpki --precise 0.103.16
cargo update -p webpki-root-certs --precise 1.0.10
cargo update -p serde_json -p thiserror -p schemars -p hyper -p hyper-util -p http-body-util -p tokio-util -p flate2
# tauri 2.12.5 pulls transitives younger than 7 days; step them back:
cargo update -p wry --precise 0.57.0
cargo update -p tao --precise 0.37.1
cargo update -p rmcp-macros --precise 3.6.0
cargo update -p brotli --precise 9.0.0
cargo update -p rfd --precise 0.16.0
```

(brotli-decompressor settles at 6.0.1 on its own when brotli goes to 9.0.0.) After this, a date check of the 64 changed lock entries showed none younger than 7 days (the only 10-21 entries are tauri 2.12.5 / tauri-build / tauri-codegen / tauri-macros 2.7.4 / tauri-runtime-wry 2.12.3, all published before 12:00 UTC). Do not run a bare `cargo update`: it would pull about 25 crates younger than 7 days (for example tokio 1.54.1, wry 0.57.1, tao 0.37.2, brotli 9.1.1, rustix 1.1.6, uuid 1.29.0, smallvec 1.16.4).

## Majors, breaking changes and call sites

### zip 8.6.0 to 9.0.5 (PR 4)

Breaking change that hits this repo: `ZipArchive::file_names()` now yields `Result<Cow<str>, ZipError>` instead of `&str` (the changelog only lists `ZipFileData::extra_data_start` removed; the iterator change is what the compiler reports, and what CI failed on). Two call sites:

- `crates/maquette-core/src/providers/handback.rs:61-67` (`zip_models`): `archive.file_names().filter(|n| is_model_entry(n)).map(str::to_string).collect()`
- `crates/maquette-core/src/animations/import/stage.rs:66-74` (`zip_entries`): `archive.file_names().filter(|n| is_fbx(n) && !n.starts_with("__MACOSX/") && !n.contains("/._")).map(str::to_string)`

Fix used in the probe (compiles): insert `.filter_map(Result::ok)` right after `.file_names()` and replace `.map(str::to_string)` with `.map(|n| n.into_owned())`. Alternative if you prefer not to drop unreadable names silently: collect into `Result<Vec<_>, _>` and map the error to `CoreError::Invalid` the way the neighbouring `ZipArchive::new` error is mapped. Other zip uses compile unchanged: `ZipArchive::new`, `by_name`, `entry.size()` (handback.rs:134-143, stage.rs:79-90), and the test-only `ZipWriter`, `SimpleFileOptions`, `start_file`, `CompressionMethod::Deflated` (handback/tests.rs:20-27, providers/tests.rs:1268-1270, animations/tests.rs:262-268, tests_blender.rs:50). Feature `deflate-flate2` with `default-features = false` still exists in 9.0.5.

### sha2 0.10.9 to 0.11.1 (PR 6)

Breaking change that hits this repo: digest output (`Array<u8, ...>`, from `hybrid-array`) no longer implements `LowerHex`, so `format!("{:x}", Sha256::digest(..))` does not compile. Three call sites, fix each with `hex::encode(Sha256::digest(..))` (the `hex` crate is already a dependency of both crates):

- `crates/maquette-core/src/pipeline/starter.rs:160` (`text_hash`) in non-test code, CI's only lib error
- `apps/desktop/src-tauri/tests/mcp_bake.rs:150` (`age_bake_script`)
- `apps/desktop/src-tauri/src/commands/pipeline/tests.rs:143` (`sha256_hex`)

The output string is identical (lowercase hex), so the stored `.starter.json` hashes stay valid. All other uses (`Sha256::new()`, `update`, `finalize()` then `hex::encode`, `Sha256::digest(..)` then `hex::encode`) compile unchanged: pipeline/cache.rs:169-178 and 227-232, prepare/preview.rs:136-141, built/mod.rs:211 and 235-246, library/files.rs:29. sha2 0.11 raises MSRV to 1.85 (repo is on 1.94.1) and removes the `std` feature (not used: `sha2 = "0.10.9"` with default features).

### getrandom 0.3.4 to 0.4.3 (PR 5) and base64 0.22.1 to 0.23.1 (PR 7)

No code changes. `getrandom::fill` at `apps/desktop/src-tauri/src/workspace_lock.rs:25`, `.../mcp/mod.rs:62`, `.../commands/split.rs:241` compiles unchanged; base64 use is `crates/maquette-core/src/providers/meshy.rs:11` and `:95` only. Both PRs are green on macOS and Windows. Note getrandom 0.2.17 and 0.3.4 stay in the lock as transitives of other crates, so several versions coexisting is expected.

### vitest 4.1.11 to 5.0.5 (PR 10)

`package.json` locations to bump together: `package.json:76` (root), `apps/web/package.json:41`, `apps/site/package.json:38`, `packages/ui/package.json:41`. Vite 8.3.5 and Node 22.22 satisfy vitest 5's `vite >= 6.4.0`, `node >= 22.12.0`.

Single failure in CI: `apps/web/src/ipc/mock/lab.test.ts:129`
`await expect(handle('lab_retry', {...})).rejects.toThrow(/only a failed/)` got `''`. Cause: the mock backend rejects with a plain object, not an `Error` (`fail()` at `apps/web/src/ipc/mock/backend-options.ts:133-136` does `throw { code, message }`, thrown from `apps/web/src/ipc/mock/lab.ts:296`). Vitest 4 read `.message` from that object for `toThrow`; vitest 5 does not (its documented change is that `toThrow` semantics got stricter, and non-Error rejections yield an empty message). Fix: change the assertion to
`await expect(handle('lab_retry', {...})).rejects.toMatchObject({ code: 'invalid', message: expect.stringMatching(/only a failed/) });`
This is the only `rejects.toThrow` in the repo (grep over apps, packages, scripts). I did not run the web suite on vitest 5 locally (no node_modules in the worktree); verify with `pnpm test`.

Other vitest 5 breaking changes checked against this repo, none requiring edits:

- `clearMocks` now defaults to true: CI passed 1409 other tests, so nothing relies on call history surviving across tests. `apps/web/src/App.test.tsx:12` calls `clearMocks()` from `@tauri-apps/api/mocks`, which is unrelated.
- `vi.mock`/`vi.hoisted` must be top level: no indented `vi.mock`/`vi.unmock`/`vi.hoisted` calls found (41 top level calls in apps/web tests such as `src/test/fake-fiber.ts`, `InboxView.test.tsx`).
- Fake timers now also fake `Temporal`: 22 `useFakeTimers`/`setSystemTime` usages, no Temporal use, so no impact.
- Unawaited `rejects`/`resolves` now fail: the only `rejects` is awaited.
- `test.sequential` / `describe.sequential` removed: none used.
- Root `vitest.config.ts` uses `test.projects` with file paths, which is the supported form. Inline `projects` semantics (extends default true) do not apply.
- Reporter/coverage/browser-mode changes: not used (no coverage or browser config in the repo).

## Transitive CVE fixes (Phase 2)

| Vulnerable transitive     | Pulled in by                                    | Fix                                                                                            | Resolves?         |
| ------------------------- | ----------------------------------------------- | ---------------------------------------------------------------------------------------------- | ----------------- |
| `fast-uri` 3.1.8          | @commitlint/cli > load > config-validator > ajv | `pnpm update fast-uri -r` to 3.1.9, or `pnpm.overrides`. commitlint 21.2.4 does not change it. | yes, 4 advisories |
| `@vitest/mocker` 4.1.11   | vitest                                          | vitest 5.0.5 (carries mocker 5.0.x)                                                            | yes, 2 advisories |
| `rustls-webpki` 0.103.15  | rustls (reqwest, rmcp, updater)                 | `cargo update -p rustls-webpki --precise 0.103.16`                                             | yes               |
| `webpki-root-certs` 1.0.9 | rustls TLS stack                                | `--precise 1.0.10`                                                                             | yes               |
| `tokio` 1.53.1            | direct                                          | `--precise 1.53.3`                                                                             | yes (warning)     |
| `zip` 4.6.1               | `tauri-plugin-updater` optional `zip` feature   | cannot fix from here, see below                                                                | no                |
| `glib` 0.18.5             | tauri on Linux gtk3                             | wait for tauri on gtk4                                                                         | no                |

## Residual risk

- `zip` 4.6.1 via `tauri-plugin-updater` stays, because even 2.14.0 depends on `zip ^4`. The advisories concern `ZipArchive::extract`. `apps/desktop/src-tauri/tauri.conf.json:40` bundles only `app`, `dmg`, `nsis`, so no zip updater artifact is produced. Option for a follow-up (not part of this bump, needs a release dry run): drop the `"zip"` feature from the updater line in the workspace `Cargo.toml` (`features = ["rustls-tls", "zip"]`) and check the updater still installs the NSIS and macOS archives. Otherwise add a documented ignore for RUSTSEC-2026-0367/0368/0369 if a gate ever runs cargo audit (none does today; `pnpm gate` does not call cargo audit or pnpm audit).
- Linux gtk3 / `glib` 0.18.5 and the `paste`/`proc-macro-error` unmaintained notices are upstream.

## Upgrade Plan

One branch (`chore/deps-bump`), commits in this order so each stays green:

1. `ci(deps)`: rust-cache to v2.9.3 SHA (`.github/workflows/ci.yml:158`). Closes PR 2.
2. `build(deps)`: Rust majors with code fixes: zip 9.0.5 (2 files), sha2 0.11.1 (3 files), getrandom 0.4.3, base64 0.23.1. Closes PRs 4, 5, 6, 7. Gate: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`.
3. `build(deps)`: Rust minors and security lock bumps (rmcp =3.6.0, tauri 2.12, dialog 2.8, updater =2.14.0, lock commands above). Closes PR 12 (superseded, not merged). Smoke check: tauri-specta regenerate-and-diff guard (bindings must not drift), `tauri.conf.json` still valid, app starts, updater still checks.
4. `build(deps)`: JS minors and patches including @tauri-apps/api/cli 2.12.4, react 19.3.1, fast-uri. Closes PRs 9 and 13 (superseded).
5. `build(deps)`: vitest 5.0.5 in four package.json files plus the `lab.test.ts:129` assertion change. Closes PR 10.

Then run the full `pnpm gate`, push once, let CI run Windows. If the Windows `a_server_starts_reports_its_address_and_stops` test fails, re-run before investigating.

Phase 3 (risky majors needing a real migration plan): none among the proposed ones. Each major here was probed and needs only the small fixes above.
Phase 4 backlog: typescript 7.0.2 (native compiler, tsconfig migration; ignored in dependabot.yml), @types/node 26 (stays on 22 with `.nvmrc`), zustand 5.0.16 / starlight 0.42.7 / typescript-eslint 8.72.1 / tokio 1.54.x / tauri-plugin-dialog 2.8.4 / tauri-plugin-updater 2.14.1 once they pass 7 days, drop the updater `zip` feature.

## Verification

- `pnpm audit` after Phase 4 and 5: expect 0 advisories (fast-uri and @vitest/mocker both gone).
- `cargo audit` after step 3: expect only the three zip 4.6.1 vulnerabilities (updater) and the glib/paste/proc-macro-error warnings. This was reproduced in the scratch copy.
- `cargo check --workspace --all-targets` was run in a scratch copy of this worktree with all Rust changes in this report applied (Cargo.toml edits, the lock commands, and the five code fixes): passes, exit 0.
- Not run: the JS side (no node_modules in this worktree, no install performed), `pnpm gate`, cargo clippy and cargo test, any run on Windows.

## Tool Gaps Noted

- Ref MCP was not available in this session, so changelogs came from WebFetch (vitest migration guide, zip and sha2 changelogs) and from the compile errors in CI logs and the scratch build. The zip `file_names` change is not in the zip changelog text; it was confirmed by the compiler.
- `cargo outdated` is not installed; `cargo update --dry-run` plus the crates.io API (publish dates) were used instead.
- `pnpm outdated` needs node_modules for current versions; versions and dates came from the npm registry (`npm view <pkg> time --min-release-age=0`) compared against the pinned versions.
- Cargo has no stable publish-age gate, so the age check on the lockfile was done by hand against crates.io `created_at`.

## Out of Scope

- Anything under `/Users/shirone/Documents/Projects/maquette` (main checkout), untouched.
- Private or non-registry deps: none found.
