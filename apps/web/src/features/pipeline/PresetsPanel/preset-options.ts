import type { MeshoptLevel, TextureCodec } from '@/ipc/client';

export const SIZES = ['512', '1024', '2048', '4096'] as const;
export const SAME = 'same';
export const DATA_SIZES = [
  { value: SAME, label: 'Same as base' },
  ...SIZES.map(v => ({ value: v, label: `${v} px` })),
];
export const CODECS: readonly { value: TextureCodec; label: string }[] = [
  { value: 'etc1s', label: 'ETC1S' },
  { value: 'uastc', label: 'UASTC' },
];
export const MESHOPT: readonly MeshoptLevel[] = ['low', 'medium', 'high'];
