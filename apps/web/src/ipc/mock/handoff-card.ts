import type { CardRow, HandoffCard, MeshyPreset, ProviderJob, TripoPreset } from '../client';
import { webCost } from './pricing';

const onOff = (on: boolean) => (on ? 'On' : 'Off');

function tripoRows(p: TripoPreset): CardRow[] {
  return [
    {
      label: 'Model',
      value: p.model.startsWith('v3.1') ? 'H3.1' : p.model,
      note: `model id ${p.model}`,
    },
    {
      label: 'Face limit',
      value: p.faceLimit ? p.faceLimit.toLocaleString('en-US') : 'Adaptive',
      note: null,
    },
    { label: 'Texture', value: onOff(p.texture), note: null },
    { label: 'PBR', value: onOff(p.texture && p.pbr), note: null },
    {
      label: 'Texture quality',
      value: p.textureQuality === 'detailed' ? 'Detailed' : 'Standard',
      note: null,
    },
    {
      label: 'Geometry',
      value: p.geometryQuality === 'detailed' ? 'Ultra' : 'Standard',
      note: null,
    },
    { label: 'Smart low poly', value: onOff(p.smartLowPoly), note: null },
    { label: 'Real-world size', value: onOff(p.autoSize), note: null },
    { label: 'Fix the image first', value: onOff(p.imageAutofix), note: null },
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
      ...(tripo && job.tripo ? tripoRows(job.tripo) : job.meshy ? meshyRows(job.meshy) : []),
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
