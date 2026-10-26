import { NumberField, Select } from '@maquette/ui';
import { SectionLabel } from '@/features/common/SectionLabel';
import { EASINGS } from '@/state/motion-states';
import { curvePath, useTransitionEditor } from './TransitionEditor.hooks';

const W = 116;
const H = 52;

/** The timing of the move into the selected state. */
export default function TransitionEditor() {
  const editor = useTransitionEditor();
  if (!editor) return null;
  return (
    <section aria-label="Transition" className="flex flex-col gap-3">
      <SectionLabel>{editor.title}</SectionLabel>
      <div className="grid grid-cols-3 gap-2">
        <NumberField
          label="Duration"
          size="sm"
          unit="s"
          min={0}
          max={10}
          value={editor.duration}
          onChange={duration => editor.patch({ duration })}
        />
        <NumberField
          label="Delay"
          size="sm"
          unit="s"
          min={0}
          max={10}
          value={editor.delay}
          onChange={delay => editor.patch({ delay })}
        />
        <NumberField
          label="Stagger"
          size="sm"
          unit="s"
          min={0}
          max={2}
          value={editor.stagger}
          onChange={stagger => editor.patch({ stagger })}
        />
      </div>
      <div className="flex items-end gap-3">
        <Select
          label="Easing"
          size="sm"
          options={EASINGS}
          value={editor.easing}
          onChange={easing => editor.patch({ easing })}
          fieldClassName="min-w-0 flex-1"
        />
        <svg
          role="img"
          aria-label={`Easing curve, ${editor.easing}`}
          width={W + 8}
          height={H + 8}
          viewBox={`-4 -4 ${W + 8} ${H + 8}`}
          className="flex-none rounded-control border border-line bg-raised"
        >
          <path
            d={curvePath(editor.easing, W, H)}
            fill="none"
            stroke="var(--color-clay)"
            strokeWidth={1.6}
          />
          <circle cx={0} cy={H} r={2.5} fill="var(--color-fg)" />
          <circle cx={W} cy={0} r={2.5} fill="var(--color-fg)" />
        </svg>
      </div>
      <div className="flex items-center justify-between gap-2 text-meta">
        <span className="text-fg-dim">Parts in this move</span>
        <span className="truncate font-mono text-fg-dim">
          {editor.moving}, {editor.of}
        </span>
      </div>
    </section>
  );
}
