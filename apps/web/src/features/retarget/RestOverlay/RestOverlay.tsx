import { KeyValueList, SegmentedControl } from '@maquette/ui';
import type { RestBone, RestCompare } from '@/ipc/client';
import { restPoseLines } from '@/state/animations';
import { SHAPE_LABEL, armsLine } from '@/state/retarget';
import { useRestLayout, type Layout } from './RestOverlay.hooks';

export interface RestOverlayProps {
  rest: RestCompare;
  clipLabel: string;
  modelLabel: string;
}

const LAYOUTS: { value: Layout; label: string }[] = [
  { value: 'overlay', label: 'Overlay' },
  { value: 'side', label: 'Side by side' },
];

const SIZE = 100;

function Bones({ bones, className }: { bones: readonly RestBone[]; className: string }) {
  const lines = restPoseLines(bones, SIZE).map((l, i) => (
    <line key={i} {...l} className={className} strokeWidth={1.4} strokeLinecap="round" />
  ));
  return <>{lines}</>;
}

/** Both skeletons fitted to one box, so one's size reads against the other's. */
function Together({ clip, model }: { clip: readonly RestBone[]; model: readonly RestBone[] }) {
  const lines = restPoseLines([...clip, ...model], SIZE);
  const draw = (from: number, to: number, className: string) =>
    lines
      .slice(from, to)
      .map((l, i) => (
        <line key={i} {...l} className={className} strokeWidth={1.4} strokeLinecap="round" />
      ));
  return (
    <>
      {draw(0, clip.length, 'stroke-fg-mute')}
      {draw(clip.length, lines.length, 'stroke-clay-text')}
    </>
  );
}

/**
 * The two rest poses, from the front, and what the bake does about them:
 * how the arms stand against the clip's, the hips heights and their scale.
 */
export default function RestOverlay({ rest, clipLabel, modelLabel }: RestOverlayProps) {
  const { layout, setLayout } = useRestLayout();
  const svg = (children: React.ReactNode, label: string) => (
    <svg
      viewBox={`0 0 ${SIZE} ${SIZE}`}
      role="img"
      aria-label={label}
      className="aspect-square min-w-0 flex-1 rounded-control bg-studio"
    >
      {children}
    </svg>
  );
  const clipPose = `${clipLabel} ${SHAPE_LABEL[rest.clipShape]}`;
  const modelPose = `${modelLabel} ${SHAPE_LABEL[rest.modelShape]}`;
  const facts = [
    {
      label: 'Arms at rest',
      value: armsLine(rest.clipArmsDeg ?? 0, rest.modelArmsDeg ?? 0, rest.clipShape),
    },
    {
      label: 'Hips height',
      value: `${(rest.modelHips ?? 0).toFixed(2)} m, ${clipLabel} ${(rest.clipHips ?? 0).toFixed(2)} m`,
    },
    { label: 'Hips moves scaled by', value: (rest.hipsScale ?? 1).toFixed(2), mono: true },
  ];
  return (
    <section aria-label="Rest poses" className="flex flex-col gap-2">
      <p className="text-caption tracking-[0.04em] text-fg-mute uppercase">Rest poses</p>
      <SegmentedControl
        label="How to show the rest poses"
        options={LAYOUTS}
        value={layout}
        onChange={setLayout}
        className="self-start"
      />
      <div className="flex gap-2">
        {layout === 'overlay' ? (
          svg(<Together clip={rest.clip} model={rest.model} />, `${clipPose} over ${modelPose}`)
        ) : (
          <>
            {svg(<Bones bones={rest.clip} className="stroke-fg-mute" />, clipPose)}
            {svg(<Bones bones={rest.model} className="stroke-clay-text" />, modelPose)}
          </>
        )}
      </div>
      <p className="flex flex-wrap gap-x-3 text-meta">
        <span className="text-fg-dim">
          <i aria-hidden="true" className="mr-1 inline-block h-0.5 w-3 bg-fg-mute align-middle" />
          {clipPose}
        </span>
        <span className="text-fg-dim">
          <i aria-hidden="true" className="mr-1 inline-block h-0.5 w-3 bg-clay-text align-middle" />
          {modelPose}
        </span>
      </p>
      <KeyValueList items={facts} columns={1} />
    </section>
  );
}
