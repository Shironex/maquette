import { IconButton, Kbd, SegmentedControl } from '@maquette/ui';
import type { GizmoSpace } from '@/state/use-pose-keys-store';
import { usePoseKeysBar } from './PoseKeysBar.hooks';

const SPACES: { value: GizmoSpace; label: string; hint: string }[] = [
  { value: 'local', label: 'Local', hint: "Rings on the bone's own axes, as keys are written" },
  { value: 'world', label: 'World', hint: "Rings on the world's axes" },
];

const SHORTCUTS: readonly (readonly [string | string[], string])[] = [
  ['R', 'turn'],
  ['K', 'key'],
  [['[', ']'], 'keys'],
  ['M', 'mirror pose'],
];

/** Under the preview while pose keys are edited: the gizmo's space, the turn on show and the shortcuts. */
export default function PoseKeysBar() {
  const b = usePoseKeysBar();
  if (!b.shown) return null;
  const keys = SHORTCUTS.map(([key, word]) => (
    <span key={word} className="inline-flex items-center gap-1">
      <Kbd keys={key} />
      {word}
    </span>
  ));
  return (
    <div
      role="toolbar"
      aria-label="Turn gizmo"
      className="flex flex-wrap items-center gap-x-3 gap-y-1.5 rounded-card border border-line bg-panel px-2.5 py-1.5 text-meta text-fg-dim"
    >
      <IconButton
        icon="arrows-clockwise"
        label={b.turning ? 'Hide the turn rings' : 'Show the turn rings'}
        size="sm"
        aria-pressed={b.turning}
        onClick={b.toggleTurning}
      />
      <SegmentedControl<GizmoSpace>
        label="Turn"
        options={SPACES}
        value={b.space}
        onChange={b.setSpace}
      />
      <span className="font-mono text-[11.5px] text-fg">{b.readout}</span>
      <span className="text-fg-mute">{b.here}</span>
      <span className="flex-1" />
      <span className="flex flex-wrap items-center gap-2.5 text-fg-mute">{keys}</span>
    </div>
  );
}
