import type { ReactNode } from 'react';
import { Icon, SegmentedControl, type IconName } from '@maquette/ui';
import type { ImportFile, PlanClip } from '@/ipc/client';
import { RIG_LABEL, fileLine, goesIn, travelLine, type ImportChoices } from '@/state/animations';

export interface ImportFileBlockProps {
  file: ImportFile;
  choices: ImportChoices;
  onChoices: (choices: ImportChoices) => void;
}

function Line({ icon, tone, children }: { icon: IconName; tone?: string; children: ReactNode }) {
  return (
    <li className="flex items-start gap-2 text-meta text-fg-dim">
      <Icon name={icon} size={14} className={`mt-0.5 flex-none ${tone ?? 'text-fg-mute'}`} />
      <span className="min-w-0 flex-1">{children}</span>
    </li>
  );
}

const without = (list: readonly string[], key: string) => list.filter(k => k !== key);

/** What one dropped file holds: its clips, its rig, the skin dropped, and what stays out. */
export default function ImportFileBlock({ file, choices, onChoices }: ImportFileBlockProps) {
  const going = file.clips.filter(c => goesIn(c, choices)).length;
  const custom = file.rig === 'custom';
  const single = !file.archive ? file.clips[0] : undefined;
  const status =
    going === 0
      ? 'nothing to add'
      : file.archive
        ? `${going} to add`
        : custom
          ? 'add, check rig'
          : 'add';
  const keep = (clip: PlanClip, both: boolean) =>
    onChoices({
      ...choices,
      keepBoth: both ? [...choices.keepBoth, clip.key] : without(choices.keepBoth, clip.key),
    });
  const repeats = file.clips
    .filter(c => c.status !== 'new' && c.matched)
    .map(c => (
      <li key={c.key} className="flex items-center gap-2 text-meta text-fg-dim">
        <Icon name="copy" size={14} className="flex-none text-amber" />
        <span className="min-w-0 flex-1">
          <b className="font-semibold text-fg">{c.name}</b>{' '}
          {c.status === 'duplicate'
            ? `is already in ${c.matched?.pack} with the same keys`
            : `replaces the older take in ${c.matched?.pack}`}
        </span>
        <SegmentedControl
          label={`${c.name}: skip or keep both`}
          tone="clay"
          options={[
            { value: 'skip', label: c.status === 'duplicate' ? 'Skip' : 'Replace' },
            { value: 'both', label: 'Keep both' },
          ]}
          value={choices.keepBoth.includes(c.key) ? 'both' : 'skip'}
          onChange={v => keep(c, v === 'both')}
        />
      </li>
    ));
  const rig = file.rig
    ? custom
      ? `Rig: custom, ${file.bones} bones, not Mixamo. Goes in as source rig Custom; the first model it is used on asks for a bone map`
      : `Rig: ${RIG_LABEL[file.rig]}, ${file.bones} bones${file.prefix ? ` (${file.prefix} names)` : ''}`
    : null;
  const leftOut = file.leftOut.map(l => (
    <Line key={l.file} icon="minus" tone="text-fg-mute">
      <b className="font-semibold text-fg">{l.file}</b>: {l.reason}, so it is left out
    </Line>
  ));
  const fps = file.fps ? `${Math.round(file.fps)} fps` : 'mixed rates';
  return (
    <li className="flex flex-col gap-2 rounded-card border border-line bg-raised p-3">
      <div className="flex items-center gap-2.5">
        <Icon name={file.archive ? 'package' : 'file'} size={18} className="text-fg-dim" />
        <span className="min-w-0 flex-1">
          <b className="block truncate font-semibold">{file.name}</b>
          <span className="block font-mono text-[11.5px] text-fg-mute">{fileLine(file)}</span>
        </span>
        <span
          className={`rounded-pill px-2 text-[11px] leading-[18px] font-medium ${going ? 'bg-sage-soft text-sage' : 'bg-line text-fg-mute'}`}
        >
          {status}
        </span>
      </div>
      <ul className="flex list-none flex-col gap-1.5">
        {file.archive ? (
          <Line icon="film-strip">
            {file.clips.length} clips found, one take each, {fps}
          </Line>
        ) : null}
        {rig ? (
          <Line icon="bone" tone={custom ? 'text-amber' : undefined}>
            {rig}
          </Line>
        ) : null}
        {file.skinned > 0 ? (
          <Line icon="cube-transparent">
            {file.archive
              ? 'Saved with skin; the mesh is dropped'
              : `${RIG_LABEL[file.rig ?? 'custom']}, without skin`}
            {file.archive ? ' and only the skeleton and keys are kept' : ''}
          </Line>
        ) : null}
        {single ? (
          <Line icon={single.rootMotion ? 'path' : 'map-pin'}>
            {single.rootMotion
              ? `Root motion, ${travelLine(single)}`
              : `In place, ${Math.round(single.fps ?? 0)} fps, ${single.frames} frames`}
          </Line>
        ) : null}
        {repeats}
        {leftOut}
      </ul>
    </li>
  );
}
