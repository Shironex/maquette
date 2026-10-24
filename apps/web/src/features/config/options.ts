import type {
  MeshoptLevel,
  MeshyTexture,
  PoseMode,
  PresetVariant,
  ProviderId,
  TextureCodec,
  TextureSize,
  Topology,
  TripoGeometryQuality,
  TripoRigSpec,
  TripoTextureQuality,
} from '@/ipc/client';
import type { SelectOption, SegmentedOption } from '@maquette/ui';

export const TOPOLOGY: SelectOption<Topology>[] = [
  { value: 'triangle', label: 'Triangle' },
  { value: 'quad', label: 'Quad' },
];

export const TEXTURE: SelectOption<TextureSize>[] = [
  { value: '1K', label: '1K' },
  { value: '2K', label: '2K' },
  { value: '4K', label: '4K' },
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

export const SERVICES: (SegmentedOption<ProviderId> & { label: string })[] = [
  { value: 'tripo', label: 'Tripo' },
  { value: 'meshy', label: 'Meshy' },
];

export const PRESET_VARIANTS: (SegmentedOption<PresetVariant> & { label: string })[] = [
  { value: 'prop', label: 'Props and scenes' },
  { value: 'character', label: 'Characters' },
];

export const TRIPO_TEXTURE_QUALITY: SelectOption<TripoTextureQuality>[] = [
  { value: 'standard', label: 'Standard' },
  { value: 'detailed', label: 'Detailed' },
  { value: 'extreme', label: 'Extreme' },
];

export const TRIPO_GEOMETRY: SelectOption<TripoGeometryQuality>[] = [
  { value: 'standard', label: 'Standard' },
  { value: 'detailed', label: 'Ultra' },
];

export const RIG_SPEC: SelectOption<TripoRigSpec>[] = [
  { value: 'mixamo', label: 'Mixamo' },
  { value: 'tripo', label: 'Tripo' },
];

export const MESHY_TEXTURE: SelectOption<MeshyTexture>[] = [
  { value: '2k', label: '2K' },
  { value: '4k', label: '4K' },
  { value: '8k', label: '8K' },
];

export const POSE: SelectOption<PoseMode>[] = [
  { value: 'none', label: 'As drawn' },
  { value: 'a-pose', label: 'A-pose' },
  { value: 't-pose', label: 'T-pose' },
];
