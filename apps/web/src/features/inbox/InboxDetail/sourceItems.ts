import type { KeyValueItem } from '@maquette/ui';
import type { InboxItem } from '@/ipc/client';
import { formatBytes } from '@/lib/format';

/** The Source list under the checks: kind, size, what it was sent for and the file's numbers. */
export function sourceItems(
  item: InboxItem,
  sentLabel: string | null,
  maker: string | null
): KeyValueItem[] {
  const report = item.report;
  return [
    {
      label: 'Kind',
      value: item.kind === 'character' ? 'Character' : item.kind === 'scene' ? 'Scene' : 'Prop',
    },
    { label: 'Size', value: formatBytes(item.bytes ?? 0), mono: true },
    ...(sentLabel ? [{ label: 'Sent for', value: sentLabel }] : []),
    ...(report
      ? [
          {
            label: 'Meshes',
            value: `${report.meshes} · ${report.primitives} primitives`,
            mono: true,
          },
          { label: 'Materials', value: String(report.materials), mono: true },
          ...(maker ? [{ label: 'Made by', value: maker }] : []),
        ]
      : []),
    { label: 'File', value: item.id, mono: true },
  ];
}
