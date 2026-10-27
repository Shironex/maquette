<!--
Title format: conventional commit with a scope, `type(scope): lowercase summary`,
for example `fix(core): keep the lock file out of the sync ignore list`.
The PR title becomes the squash-merge commit subject. Scopes are in
commitlint.config.mjs. CONTRIBUTING.md has the long version.
-->

## Summary

<!-- One short paragraph: what does this PR do, in plain language? -->

## Context / why

<!-- Why is this change needed? Link any prior discussion. -->

## How to test

<!-- Steps a reviewer can take to verify the change. Include screenshots or a clip for UI work. -->

## Checklist

- [ ] `pnpm gate` is green locally.
- [ ] UI changes: `pnpm visual` run, and the screenshots looked at.
- [ ] Docs updated where behavior changed (the README section for the feature, `docs/architecture.md` for structure).
- [ ] Commits are small and use `type(scope): lowercase summary`.

## Linked issue

<!-- Use Closes #N / Fixes #N so the issue closes on merge. -->

Closes #
