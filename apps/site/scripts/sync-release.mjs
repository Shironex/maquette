/**
 * Before the Astro build: writes src/content/release.generated.json from the
 * latest published GitHub release, which copy.ts prefers over the checked-in
 * release.json. The generated file is git-ignored, so nothing is committed
 * back. It only calls the API when SITE_RELEASE_SYNC=1 (the pages workflow
 * sets it); local builds and `pnpm gate` stay offline and deterministic. Any
 * failure keeps the checked-in values and the build goes on.
 */
import { readFileSync, rmSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { loadReleaseData } from './release-data.mjs';

const CONTENT = new URL('../src/content/', import.meta.url);
const generated = fileURLToPath(new URL('release.generated.json', CONTENT));
const fallback = JSON.parse(readFileSync(new URL('release.json', CONTENT), 'utf8'));
const repo = process.env.GITHUB_REPOSITORY || 'Shironex/maquette';

// A stale file from an earlier synced build must not outlive its run.
rmSync(generated, { force: true });

if (process.env.SITE_RELEASE_SYNC !== '1') {
  console.log(`release: sync is off, using the checked-in values (v${fallback.version})`);
} else {
  const { data, source, reason } = await loadReleaseData({
    fallback,
    repo,
    token: process.env.GITHUB_TOKEN,
  });
  if (source === 'github') {
    writeFileSync(generated, `${JSON.stringify(data, null, 2)}\n`);
    console.log(`release: using v${data.version} from the latest GitHub release of ${repo}`);
  } else {
    console.log(
      `release: could not read the latest release (${reason}); using the checked-in values (v${fallback.version})`
    );
  }
}
