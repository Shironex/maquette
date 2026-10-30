import { SegmentedControl } from '@maquette/ui';
import { Sheet } from '@/features/common/Sheet';
import { useSheetZoom } from './SheetZoom.hooks';

export type ZoomMode = 'shoulders' | 'before' | 'after';

const MODES: readonly { value: ZoomMode; label: string; hint: string }[] = [
  {
    value: 'shoulders',
    label: 'Shoulders',
    hint: 'The close look at the shoulders, before and after, side by side',
  },
  { value: 'before', label: 'Before', hint: 'The whole before sheet, full size' },
  { value: 'after', label: 'After', hint: 'The whole after sheet, full size' },
];

/** Rows of a sheet, top to bottom; its last column is the shoulders. */
const POSES = ['At rest', 'Arms down', 'Arms up', 'A step'];

export interface SheetZoomProps {
  before: string | null;
  after: string | null;
  mode: ZoomMode;
  onClose: () => void;
}

/** Only a sheet's shoulders column, each cell as wide as the column. */
function Shoulders({ src, label }: { src: string | null; label: string }) {
  return (
    <figure className="m-0 flex min-w-0 flex-1 flex-col gap-1.5">
      <figcaption className="text-meta font-medium text-fg-dim">{label}</figcaption>
      <div className="overflow-hidden rounded-[8px] border border-line bg-studio">
        {src ? (
          <img
            src={src}
            alt={`${label}: the shoulders in four poses`}
            className="ml-[-300%] block w-[400%] max-w-none"
          />
        ) : null}
      </div>
    </figure>
  );
}

/** A rest pose sheet up close, to judge the armpits, a cape or hair by the arms. */
export default function SheetZoom({ before, after, mode: start, onClose }: SheetZoomProps) {
  const [mode, setMode] = useSheetZoom(start);
  const whole = mode === 'before' ? before : after;
  return (
    <Sheet
      id="avatar-sheet-zoom"
      icon="magnifying-glass"
      title="Rest pose sheets"
      meta={`Rows: ${POSES.join(', ').toLowerCase()}. Columns: front, side, back, shoulders.`}
      onClose={onClose}
      width={1080}
    >
      <div className="flex flex-col gap-3 overflow-y-auto px-5 py-4">
        <SegmentedControl label="Show" options={MODES} value={mode} onChange={setMode} />
        {mode === 'shoulders' ? (
          <div className="flex gap-4">
            <Shoulders src={before} label="Before" />
            <Shoulders src={after} label="After" />
          </div>
        ) : whole ? (
          <img
            src={whole}
            alt={`The ${mode} sheet`}
            className="block w-full rounded-[8px] border border-line"
          />
        ) : null}
      </div>
    </Sheet>
  );
}
