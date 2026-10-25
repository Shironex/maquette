import { SegmentedControl, Select } from '@maquette/ui';
import type { WorkspaceConfig } from '@/ipc/client';
import { patchSection } from '@/state/settings-form';
import type { ConfigFieldsProps } from '../ConfigFieldsProps';
import { CODEC, MESHOPT, TEXTURE } from '../options';

export type CompressionFieldsProps = ConfigFieldsProps;

export default function CompressionFields({ config, onChange }: CompressionFieldsProps) {
  const set = (patch: Partial<WorkspaceConfig['compression']>) =>
    onChange(patchSection(config, 'compression', patch));
  return (
    <>
      <div className="flex flex-col gap-1.5">
        <span aria-hidden="true" className="text-meta font-medium text-fg-dim">
          Texture codec
        </span>
        <SegmentedControl
          label="Texture codec"
          tone="clay"
          block
          options={CODEC}
          value={config.compression.textureCodec}
          onChange={textureCodec => set({ textureCodec })}
        />
      </div>
      <Select
        label="Meshopt"
        options={MESHOPT}
        value={config.compression.meshopt}
        onChange={meshopt => set({ meshopt })}
      />
      <Select
        label="Largest texture"
        options={TEXTURE}
        value={config.compression.textureSize}
        onChange={textureSize => set({ textureSize })}
      />
    </>
  );
}
