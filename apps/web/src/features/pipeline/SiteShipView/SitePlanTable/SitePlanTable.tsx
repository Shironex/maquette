import { Icon, Select, TextField, Tooltip, cn } from '@maquette/ui';
import type { ShipChoice, SiteLeftFile, SitePlanFile } from '@/ipc/client';
import { formatBytes, formatK } from '@/lib/format';
import { changeWords, sizeOrDash } from '../SiteShipView.hooks';

export interface SitePlanTableProps {
  files: SitePlanFile[];
  left: SiteLeftFile[];
  /** The asset's own file name, when one is set. */
  fileOf: (asset: string) => string | null;
  /** False while a save runs. */
  editable: boolean;
  onChoice: (asset: string, choice: ShipChoice) => void;
  onFile: (asset: string, file: string) => void;
}

const CHOICES: readonly { value: ShipChoice; label: string }[] = [
  { value: 'auto', label: 'Auto' },
  { value: 'never', label: 'Never' },
  { value: 'always', label: 'Always' },
];

const CHOICE_TIP: Record<ShipChoice, string> = {
  auto: 'Ships unless the site file is not Maquette’s, or is smaller for a hand-built site',
  never: 'Never ships; the site keeps its own file',
  always: 'Always ships, replacing the site file whoever made it',
};

const span = (before: string | null, after: string | null) =>
  before && after && before !== after ? `${before} → ${after}` : (after ?? before ?? '·');

function PlanRow({ file, own, editable, onChoice, onFile }: PlanRowProps) {
  const asset = file.asset;
  const size = span(
    file.beforeBytes ? formatBytes(file.beforeBytes) : null,
    file.bytes ? formatBytes(file.bytes) : null
  );
  const tris = span(
    file.beforeTriangles === null ? null : formatK(file.beforeTriangles),
    file.triangles === null ? null : formatK(file.triangles)
  );
  const before = file.beforeBytes ? formatBytes(file.beforeBytes) : null;
  const stem = file.file.replace(/\.glb$/, '');
  const noAsset = 'This file has no asset, so it ships as Compress named it';
  const commit = (value: string) => {
    if (asset && value.trim() !== (own ?? '')) onFile(asset, value);
  };
  return (
    <tr className="border-t border-line">
      <td className="py-1.5 pl-3">
        <span className="block truncate font-mono text-[12px]" title={file.file}>
          {file.file}
        </span>
        <span className="block truncate text-meta text-fg-mute">
          {file.assetName ?? 'no asset'}
        </span>
      </td>
      <td className="px-2 py-1.5 font-mono text-[11.5px] text-fg-dim tabular-nums">{size}</td>
      <td className="px-2 py-1.5 font-mono text-[11.5px] text-fg-dim tabular-nums">{tris}</td>
      <td className="px-2 py-1.5">
        <Tooltip
          label="When Ship writes this file"
          detail={asset ? CHOICE_TIP[file.choice] : noAsset}
        >
          <Select
            aria-label={`Ship ${file.file}`}
            size="sm"
            options={CHOICES}
            value={file.choice}
            disabled={!asset || !editable}
            onChange={v => asset && onChoice(asset, v)}
          />
        </Tooltip>
      </td>
      <td className="px-2 py-1.5">
        <Tooltip
          label="The file name on the site"
          detail={asset ? 'Without .glb. Empty keeps the default name' : noAsset}
        >
          <TextField
            key={`${asset ?? file.file} ${own ?? ''}`}
            aria-label={`File name of ${file.assetName ?? file.file}`}
            mono
            defaultValue={own ?? ''}
            placeholder={stem}
            disabled={!asset || !editable}
            onBlur={e => commit(e.currentTarget.value)}
            onKeyDown={e => {
              if (e.key === 'Enter') e.currentTarget.blur();
            }}
          />
        </Tooltip>
      </td>
      <td className="py-1.5 pr-3 pl-2">
        <span className={cn('flex items-start gap-1.5', file.ships ? 'text-fg-dim' : 'text-amber')}>
          <Icon
            name={file.ships ? 'arrow-right' : 'lock-simple'}
            size={13}
            className="mt-0.5 flex-none"
          />
          {file.ships ? changeWords(file.change, before) : `Held: ${file.held ?? 'left alone'}`}
        </span>
      </td>
    </tr>
  );
}

interface PlanRowProps {
  file: SitePlanFile;
  own: string | null;
  editable: boolean;
  onChoice: SitePlanTableProps['onChoice'];
  onFile: SitePlanTableProps['onFile'];
}

/** What Ship would write, replace or leave alone in the site's models folder. */
export default function SitePlanTable({
  files,
  left,
  fileOf,
  editable,
  onChoice,
  onFile,
}: SitePlanTableProps) {
  const rows = files.map(f => (
    <PlanRow
      key={f.file}
      file={f}
      own={f.asset ? fileOf(f.asset) : null}
      editable={editable}
      onChoice={onChoice}
      onFile={onFile}
    />
  ));
  const leftRows = left.map(f => (
    <tr key={f.file} className="border-t border-line">
      <td className="truncate py-1.5 pl-3 font-mono text-[12px]" title={f.file}>
        {f.file}
      </td>
      <td className="px-2 py-1.5 font-mono text-[11.5px] text-fg-dim tabular-nums">
        {sizeOrDash(f.bytes, formatBytes)}
      </td>
      <td colSpan={3} />
      <td className={cn('py-1.5 pr-3 pl-2', f.removed ? 'text-amber' : 'text-fg-mute')}>
        {f.removed ? 'Maquette shipped it before, so Ship removes it' : 'Left alone'}
      </td>
    </tr>
  ));
  return (
    <div className="rounded-card border border-line bg-panel">
      <table className="w-full table-fixed border-collapse text-[12px]">
        <colgroup>
          <col />
          <col className="w-[150px]" />
          <col className="w-[110px]" />
          <col className="w-[96px]" />
          <col className="w-[150px]" />
          <col className="w-[210px]" />
        </colgroup>
        <thead>
          <tr className="text-left text-[11px] font-semibold tracking-[0.04em] text-fg-mute uppercase">
            <th className="py-1.5 pl-3 font-semibold">File</th>
            <th className="px-2 py-1.5 font-semibold">Size</th>
            <th className="px-2 py-1.5 font-semibold">Tris</th>
            <th className="px-2 py-1.5 font-semibold">Ship</th>
            <th className="px-2 py-1.5 font-semibold">File name</th>
            <th className="py-1.5 pr-3 pl-2 font-semibold">What Ship does</th>
          </tr>
        </thead>
        <tbody>{rows}</tbody>
        {left.length ? (
          <tbody>
            <tr>
              <th
                colSpan={6}
                className="bg-raised py-1.5 pl-3 text-left text-[11px] font-semibold tracking-[0.04em] text-fg-mute uppercase"
              >
                The site&apos;s other files <span className="ml-1 tabular-nums">{left.length}</span>
              </th>
            </tr>
            {leftRows}
          </tbody>
        ) : null}
      </table>
    </div>
  );
}
