/**
 * The Lab's image backends as Settings and the Lab show them: Codex or
 * OpenAI's Image API per kind of job, the API's quality, and what a round
 * costs against the monthly cap. Mirrors `concepts::backend` and the price
 * list in `providers::openai_images` in the core.
 */
import type {
  ImageApiStatus,
  ImageBackend,
  ImageBackends,
  ImageQuality,
  ImageSettings,
} from '@/ipc/client';

export type ImageJobKind = keyof ImageBackends;

export const IMAGE_SETTINGS_DEFAULT: Required<Omit<ImageSettings, 'backends'>> & {
  backends: Required<ImageBackends>;
} = {
  backends: {
    concepts: 'codex',
    objectRefs: 'codex',
    otherViews: 'codex',
    cleanBackground: 'codex',
  },
  quality: 'low',
  monthlyCapUsd: 10,
};

export const BACKEND_LABEL: Record<ImageBackend, string> = {
  codex: 'Codex',
  'openai-images': 'OpenAI Images',
};

export const KIND_LABEL: Record<ImageJobKind, string> = {
  concepts: 'Concepts',
  objectRefs: 'Object refs',
  otherViews: 'Other views',
  cleanBackground: 'Clean background',
};

export const QUALITY_LABEL: Record<ImageQuality, string> = {
  low: 'Low',
  medium: 'Medium',
  high: 'High',
};

/** The settings with every default filled in. */
export function imageSettings(settings: ImageSettings | null | undefined) {
  return {
    backends: { ...IMAGE_SETTINGS_DEFAULT.backends, ...settings?.backends },
    quality: settings?.quality ?? IMAGE_SETTINGS_DEFAULT.quality,
    monthlyCapUsd: settings?.monthlyCapUsd ?? IMAGE_SETTINGS_DEFAULT.monthlyCapUsd,
  };
}

export function backendFor(settings: ImageSettings | null | undefined, kind: ImageJobKind) {
  return imageSettings(settings).backends[kind];
}

/** "$0.04", "$3.42", "$12": cents under ten dollars, whole dollars above. */
export function formatUsd(usd: number): string {
  if (usd > 0 && usd < 0.01) return '<$0.01';
  if (usd >= 10 && Number.isInteger(Math.round(usd * 100) / 100)) return `$${usd.toFixed(0)}`;
  return `$${usd.toFixed(2)}`;
}

/** One image's estimate at `quality`. */
export function perImage(status: ImageApiStatus, quality: ImageQuality): number {
  return status.estimates[quality] ?? 0;
}

export interface RoundCost {
  /** About what the round costs. */
  usd: number;
  spent: number;
  pending: number;
  cap: number;
  /** The round would take the month past the cap. */
  over: boolean;
}

/** What a round of `count` images likely costs, and whether the cap lets it run. */
export function roundCost(status: ImageApiStatus, count: number): RoundCost {
  const settings = imageSettings(status.settings);
  const usd = perImage(status, settings.quality) * Math.max(0, count);
  const spent = status.monthSpentUsd ?? 0;
  const pending = status.pendingUsd ?? 0;
  const cap = settings.monthlyCapUsd;
  return { usd, spent, pending, cap, over: spent + pending + usd > cap };
}

/** The Lab's line under Generate for a round on the Image API. */
export function roundLine(status: ImageApiStatus, count: number): string {
  const cost = roundCost(status, count);
  const quality = imageSettings(status.settings).quality;
  const month = `${formatUsd(cost.spent)} of ${formatUsd(cost.cap)} spent this month`;
  if (cost.over) {
    return `This round of ${count} (about ${formatUsd(cost.usd)}) would pass the monthly cap: ${month}. Raise it in Settings > Image generation.`;
  }
  return `Runs on OpenAI Images (${quality}), about ${formatUsd(cost.usd)} for ${count}. ${month}.`;
}
