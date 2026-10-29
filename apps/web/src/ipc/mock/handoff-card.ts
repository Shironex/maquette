import type {
  CardRow,
  HandoffCard,
  MeshyPreset,
  ProviderJob,
  StudioSettings,
  TripoPreset,
} from '../client';
import { isTripoPSeries } from '@/state/provider-presets';
import { STUDIO_RIG, studioDefaults } from '@/state/studio';
import { webCost } from './pricing';

const onOff = (on: boolean) => (on ? 'On' : 'Off');

const TEXTURE_QUALITY: Record<TripoPreset['textureQuality'], string> = {
  standard: '2K',
  detailed: '4K',
  extreme: '8K',
};

/** The Tripo card in Tripo Studio's words, as `handoff/studio.rs` writes it. */
function tripoRows(p: TripoPreset, studio: StudioSettings, name: string): CardRow[] {
  const recommended = 'recommended';
  const hd = studio.mesh === 'hd-model';
  const model = isTripoPSeries(p.model) ? 'v3.1-20260211' : p.model;
  const rig = STUDIO_RIG.find(r => r.value === studio.rig)?.label ?? studio.rig;
  return [
    { label: 'Tab', value: hd ? 'HD Model' : 'Smart Mesh (P2.0)', note: null },
    ...(hd
      ? [
          {
            label: 'AI Model',
            value: model.startsWith('v3.1') ? 'H3.1' : model,
            note: `model id ${model}`,
          },
          {
            label: 'Ultra Mesh Quality',
            value: onOff(p.geometryQuality === 'detailed'),
            note: null,
          },
        ]
      : []),
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
    {
      label: 'Rig',
      value: rig,
      note:
        studio.rig === 'none'
          ? 'skip Rig, export the model as is'
          : 'export GLB (or FBX) with Export Skeleton on, Texture Resolution 2K',
    },
    ...(studio.rig === 'biped' && p.animations.length > 0
      ? [
          {
            label: 'Animations',
            value: p.animations.map(animationLabel).join(', '),
            note: 'retarget them onto the rig',
          },
        ]
      : []),
    { label: 'Export', value: name, note: 'GLB, with this file name' },
  ];
}

function animationLabel(id: string): string {
  const name = (id.split(':').pop() ?? id).replace(/_/g, ' ');
  return name.charAt(0).toUpperCase() + name.slice(1);
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
        ? tripoRows(job.tripo, job.studio ?? studioDefaults(job.tripo, job.variant), name)
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
