import type { CardRow, HandoffCard, MeshyPreset, ProviderJob, TripoPreset } from '../client';
import { webCost } from './pricing';

const onOff = (on: boolean) => (on ? 'On' : 'Off');

const TEXTURE_QUALITY: Record<TripoPreset['textureQuality'], string> = {
  standard: '2K',
  detailed: '4K',
  extreme: '8K',
};

/** The Tripo card in Tripo Studio's words, as `handoff/studio.rs` writes it. */
function tripoRows(p: TripoPreset, character: boolean, name: string): CardRow[] {
  const recommended = 'recommended';
  const skeleton = p.rigSpec === 'tripo' ? 'Tripo skeleton' : 'Mixamo skeleton';
  return [
    { label: 'Tab', value: p.smartLowPoly ? 'Smart Mesh' : 'HD Model', note: null },
    {
      label: 'AI Model',
      value: p.model.startsWith('v3.1') ? 'H3.1' : p.model,
      note: `model id ${p.model}`,
    },
    {
      label: 'Ultra Mesh Quality',
      value: onOff(p.geometryQuality === 'detailed'),
      note: null,
    },
    { label: 'AI Complete', value: 'On', note: recommended },
    { label: 'Texture', value: onOff(p.texture), note: null },
    ...(p.texture
      ? [
          { label: 'Texture Quality', value: TEXTURE_QUALITY[p.textureQuality], note: null },
          { label: 'Remove Lighting', value: 'On', note: recommended },
        ]
      : []),
    { label: 'PBR', value: onOff(p.texture && p.pbr), note: null },
    { label: 'Topology', value: 'Triangle', note: 'quads export FBX only' },
    {
      label: 'Polycount',
      value: String(p.faceLimit || 20_000),
      note: p.faceLimit ? null : 'Studio has no adaptive',
    },
    { label: 'Generate in Parts', value: 'Off', note: recommended },
    ...(character && p.rig
      ? [
          {
            label: 'Rig',
            value: `${skeleton}, export GLB (or FBX) with Export Skeleton on, Texture Resolution 2K`,
            note: `rig model ${p.rigModel}`,
          },
        ]
      : []),
    { label: 'Export', value: name, note: 'GLB, with this file name' },
  ];
}

function meshyRows(p: MeshyPreset): CardRow[] {
  return [
    { label: 'AI model', value: p.aiModel === 'latest' ? 'Meshy 7.1' : p.aiModel, note: null },
    { label: 'Texture', value: onOff(p.shouldTexture), note: null },
    { label: 'PBR', value: onOff(p.enablePbr), note: null },
    { label: 'Remesh', value: onOff(p.shouldRemesh), note: null },
    {
      label: 'Target polycount',
      value: p.targetPolycount.toLocaleString('en-US'),
      note: p.shouldRemesh ? null : 'only with Remesh on',
    },
  ];
}

const VIEWS = ['front', 'left', 'back', 'right'];

/** `maq_<asset>_v<n>_<provider>`, n from the job id `<asset>-<n>`. */
export function exportName(job: Pick<ProviderJob, 'id' | 'asset' | 'service'>): string {
  const n = job.id.startsWith(`${job.asset}-`) ? job.id.slice(job.asset.length + 1) : job.id;
  return `maq_${job.asset}_v${n}_${job.service}`;
}

export function handoffCard(job: ProviderJob): HandoffCard {
  const tripo = job.service === 'tripo';
  const multi = job.images.length > 1;
  const name = exportName(job);
  const cost = webCost(job.service, job.tripo, job.meshy);
  const prompt = !tripo && job.prompt ? job.prompt : null;
  return {
    service: job.service,
    site: tripo ? 'https://studio.tripo3d.ai' : 'https://www.meshy.ai/workspace',
    mode: tripo
      ? multi
        ? 'Multiview to 3D'
        : 'Image to 3D'
      : multi
        ? 'Multi-image to 3D'
        : 'Image to 3D',
    rows: [
      ...(tripo && job.tripo
        ? tripoRows(job.tripo, job.variant === 'character', name)
        : job.meshy
          ? meshyRows(job.meshy)
          : []),
      ...(prompt ? [{ label: 'Texture prompt', value: prompt, note: null }] : []),
    ],
    images: job.images.map((_, i) => `${VIEWS[i] ?? `view_${i + 1}`}.png`),
    returnName: `${name}.glb`,
    exportName: name,
    credits: cost.credits,
    creditsNote: cost.what,
    prompt,
  };
}
