import type { Asset, ProjectManifest, RefImage } from '../client';

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

/** The asset's own refs, front first: what its direction sent, else refs named after it. */
export function suggestedRefs(
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
  return (fromLab.length > 0 ? fromLab : byName).slice(0, 4);
}
