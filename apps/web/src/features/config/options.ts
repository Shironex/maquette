import type { MeshoptLevel, RigPreset, TextureCodec, TextureSize, Topology } from '@/ipc/client';
import type { SelectOption, SegmentedOption } from '@maquette/ui';

export const TRIPO_MODELS = ['H3.1', 'H3.0', 'v2.5'] as const;

export function modelOptions(current: string): SelectOption<string>[] {
  const all = TRIPO_MODELS.includes(current as (typeof TRIPO_MODELS)[number])
    ? [...TRIPO_MODELS]
    : [current, ...TRIPO_MODELS];
  return all.map(m => ({ value: m, label: m }));
}

export const TOPOLOGY: SelectOption<Topology>[] = [
  { value: 'triangle', label: 'Triangle' },
  { value: 'quad', label: 'Quad' },
];

export const TEXTURE: SelectOption<TextureSize>[] = [
  { value: '1K', label: '1K' },
  { value: '2K', label: '2K' },
  { value: '4K', label: '4K' },
];

export const RIG: SelectOption<RigPreset>[] = [
  { value: 'mixamo', label: 'Mixamo' },
  { value: 'tripo', label: 'Tripo' },
];

export const CODEC: SegmentedOption<TextureCodec>[] = [
  { value: 'etc1s', label: 'ETC1S' },
  { value: 'uastc', label: 'UASTC' },
];

export const MESHOPT: SelectOption<MeshoptLevel>[] = [
  { value: 'low', label: 'Low' },
  { value: 'medium', label: 'Medium' },
  { value: 'high', label: 'High' },
];
