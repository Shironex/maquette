import type { ProviderJob, StudioChoice, StudioSettings, WorkspaceConfig } from '../client';
import { studioDefaults } from '@/state/studio';

/** By hand on Tripo: the Studio settings asked for over the preset's, as `send.rs` resolves them. */
export function studioOf(
  config: WorkspaceConfig,
  job: Pick<ProviderJob, 'provider' | 'service' | 'variant' | 'tripo'>,
  asked: StudioChoice | null | undefined
): StudioSettings | null {
  if (job.provider !== 'by-hand' || job.service !== 'tripo') return null;
  const preset = job.tripo ?? config.providers.tripo[job.variant];
  const fallback = studioDefaults(preset, job.variant);
  return { mesh: asked?.mesh ?? fallback.mesh, rig: asked?.rig ?? fallback.rig };
}

/** An item's own choice laid over the batch's. */
export function over(
  own: StudioChoice | null | undefined,
  batch: StudioChoice | null | undefined
): StudioChoice {
  return { mesh: own?.mesh ?? batch?.mesh ?? null, rig: own?.rig ?? batch?.rig ?? null };
}
