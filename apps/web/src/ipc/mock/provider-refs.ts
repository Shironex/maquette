import type { Asset, AssetRef, ProjectManifest, RefImage } from '../client';
import { SLOTS } from '@/state/refs';

/** Refs the provider fixtures send, served from the Board renders. */
export const PROVIDER_REF_FILES: Record<string, string> = {
  'refs/veil_gate_front.png': 'veil-gate.webp',
  'refs/guardian_statues_front.png': 'guardian-statue.webp',
  'refs/rune_obelisk_front.png': 'rune-obelisk.webp',
};

export function refImages(lab: RefImage[]): RefImage[] {
  const extra = Object.keys(PROVIDER_REF_FILES).map(file => ({
    file,
    name: file.slice('refs/'.length),
    bytes: 1.6e6,
  }));
  return [...extra, ...lab.filter(r => !(r.file in PROVIDER_REF_FILES))];
}

/** Where the asset's refs come from before it is tagged: what its direction sent, else refs named after it. */
function ownRefs(
  lab: { sentFor: (project: string, assetName: string) => string[] } | undefined,
  p: ProjectManifest,
  asset: Asset,
  images: RefImage[]
): string[] {
  const slug = asset.id.replace(/-/g, '_');
  const fromLab = (lab?.sentFor(p.id, asset.name) ?? []).filter(file =>
    images.some(i => i.file === file)
  );
  const byName = images
    .filter(i => i.name.toLowerCase().replace(/-/g, '_').startsWith(slug))
    .map(i => i.file);
  return fromLab.length > 0 ? fromLab : byName;
}

/** Mirrors `refs::views_of`: the tags (minus gone files) and new refs as other, or views read from the order. */
export function refsOf(
  lab: { sentFor: (project: string, assetName: string) => string[] } | undefined,
  p: ProjectManifest,
  asset: Asset,
  images: RefImage[]
): AssetRef[] {
  const own = ownRefs(lab, p, asset, images);
  const tags = (asset.refs ?? []).filter(r => images.some(i => i.file === r.file));
  if ((asset.refs ?? []).length === 0)
    return own.map((file, n) => ({ file, view: SLOTS[n] ?? 'other' }));
  return [
    ...tags,
    ...own
      .filter(f => !tags.some(r => r.file === f))
      .map(file => ({ file, view: 'other' as const })),
  ];
}

/** The asset's refs in the view slots, front first: what a send picks by default. */
export function suggestedRefs(
  lab: { sentFor: (project: string, assetName: string) => string[] } | undefined,
  p: ProjectManifest,
  asset: Asset,
  images: RefImage[]
): string[] {
  const refs = refsOf(lab, p, asset, images);
  return SLOTS.flatMap(v => refs.filter(r => r.view === v).map(r => r.file));
}
