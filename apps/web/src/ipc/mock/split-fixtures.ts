/**
 * The rest of the split mock: `?parts=compare`'s two lantern versions and the
 * sends that made them, and a re-texture going out through the fake queue.
 */
import type { Asset, KeyStatus, ProviderJob, RetextureRequest } from '../client';
import { clock } from '@/lib/clock';
import { MESHY_RETEXTURE_CREDITS, retextureProblem } from '@/state/split';
import { fail } from './backend-options';
import type { createFakeProviders } from './providers';

type Providers = ReturnType<typeof createFakeProviders>;

const stats = (triangles: number, bytes: number, maxTexture: number) => ({
  triangles,
  bytes,
  textures: 1,
  maxTexture,
  joints: 0,
  animations: 0,
});

/** The lantern with v2 a heavy Tripo model and v3 a lighter Meshy one; `next` holds v3. */
export function lanternPair(next: Asset): Asset {
  const tuned = next.versions.map(v =>
    v.version === 2
      ? { ...v, stats: stats(18_200, 1_100_000, 2048) }
      : v.version === 3
        ? { ...v, stats: stats(9_600, 620_000, 1024) }
        : v
  );
  return { ...next, versions: tuned };
}

/** The two sends that made the lantern's versions. */
export function seedPairJobs(providers: Providers) {
  for (const [version, provider, credits] of [
    [2, 'tripo', 30],
    [3, 'meshy', 20],
  ] as const) {
    providers.seed(
      providers.base({
        id: `frost-lantern-made-${version}`,
        asset: 'frost-lantern',
        assetName: 'Frost lantern',
        provider,
        state: 'accepted',
        version,
        credits,
        estimate: credits,
        cancel: 'none',
      })
    );
  }
}

/** A re-texture through the fake queue, refused as the core refuses it. */
export function sendRetexture(
  req: RetextureRequest,
  providers: Providers,
  assetOf: (id: string) => Asset,
  keys: readonly KeyStatus[]
): ProviderJob {
  const asset = assetOf(req.asset);
  const problem = retextureProblem(req.prompt);
  if (problem) fail('invalid', problem);
  if (!keys.find(k => k.service === 'meshy')?.saved) {
    fail('invalid', 'Save a Meshy API key first.');
  }
  const used = providers.jobs().filter(j => j.asset === asset.id).length;
  return providers.start(
    providers.base({
      id: `${asset.id}-${used + 1}`,
      project: req.project,
      asset: asset.id,
      assetName: asset.name,
      provider: 'meshy',
      service: 'meshy',
      tripo: null,
      meshy: null,
      estimate: MESHY_RETEXTURE_CREDITS,
      createdAtMs: clock.nowMs(),
      prompt: req.prompt.trim(),
      retexture: { fromVersion: req.version },
    })
  );
}
