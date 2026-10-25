/**
 * Bake settings in mock mode: the Bake light step's size, samples and
 * denoiser per project, checked the way the core checks them, and the
 * assets whose `prepare.json` entry skips the bake.
 */
import type { BakePatch, BakeSettings, BakeView, ScenePrep, StarterStatus } from '../client';
import { fail } from './backend-options';

export const BAKE_DEFAULT: BakeSettings = { size: 1024, samples: 32, denoise: true };
export const BAKE_SIZES = [256, 512, 1024, 2048, 4096] as const;
const SCRIPT = 'tools/bake_light.py';

export function createFakeBake(
  entries: (project: string) => ScenePrep,
  starters: (project: string) => StarterStatus[]
) {
  const saved = new Map<string, BakeSettings>();
  const script = (id: string) => starters(id).find(s => s.path === SCRIPT)?.state ?? 'missing';

  function view(id: string): BakeView {
    const current = script(id) === 'current';
    const fallback = current ? BAKE_DEFAULT : { size: 512, samples: 16, denoise: false };
    const skipped = Object.entries(entries(id).models)
      .filter(([, entry]) => entry.skipBake === true)
      .map(([asset]) => asset);
    return {
      settings: saved.get(id) ?? fallback,
      canDenoise: current,
      script: script(id),
      skipped,
    };
  }

  function save(id: string, patch: BakePatch): void {
    const now = view(id);
    const next: BakeSettings = {
      size: patch.size ?? now.settings.size,
      samples: patch.samples ?? now.settings.samples,
      denoise: patch.denoise ?? (now.settings.denoise && now.canDenoise),
    };
    if (!(BAKE_SIZES as readonly number[]).includes(next.size))
      fail('invalid', `A lightmap is 256, 512, 1024, 2048 or 4096 px, not ${next.size}.`);
    if (!(next.samples >= 1 && next.samples <= 4096))
      fail('invalid', `The bake takes 1 to 4096 samples, not ${next.samples}.`);
    if (next.denoise && !now.canDenoise)
      fail(
        'invalid',
        `${SCRIPT} is an older or edited copy that cannot denoise. Update it from the Pipeline view's starter banner first, or leave denoise off.`
      );
    saved.set(id, next);
  }

  return { view, save };
}
