import type { CardRow, HandoffCard, MeshyPreset, ProviderJob, TripoPreset } from '../client';

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

export function handoffCard(job: ProviderJob): HandoffCard {
  const tripo = job.service === 'tripo';
  const multi = job.images.length > 1;
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
    rows: tripo && job.tripo ? tripoRows(job.tripo) : job.meshy ? meshyRows(job.meshy) : [],
    images: job.images.map((_, i) => `${VIEWS[i] ?? `view_${i + 1}`}.png`),
    returnName: `${job.id}.glb`,
  };
}
