import { SegmentedControl } from '@maquette/ui';
import { Sheet } from '@/features/common/Sheet';
import { useSpringSheets } from './SpringSheets.hooks';
import type { SheetKind } from './SpringSheets.types';

const KINDS: readonly { value: SheetKind; label: string; hint: string }[] = [
  {
    value: 'candidates',
    label: 'Parts found',
    hint: 'Each part that could swing in its colour, front, side and back',
  },
  {
    value: 'before',
    label: 'Chains',
    hint: 'The confirmed parts and where their joints go, before the step',
  },
  {
    value: 'after',
    label: 'Sway test',
    hint: 'The chains swung left, right and back, from four sides',
  },
];

const META: Record<SheetKind, string> = {
  candidates: 'Columns: front, side, back. Each part in the colour its row shows.',
  before: 'Columns: front, side, back, close to where each part hangs. White marks the joints.',
  after:
    'Rows: at rest, swung left, swung right, swung back. Columns: front, side, back, close. A part should bend smoothly and stay on at the top.',
};

export interface SpringSheetsProps {
  sheets: Record<SheetKind, string | null>;
  start: SheetKind;
  onClose: () => void;
}

/** The springs' sheets up close: the parts found, the chains, and the sway test. */
export default function SpringSheets({ sheets, start, onClose }: SpringSheetsProps) {
  const { kind, setKind } = useSpringSheets(start);
  const options = KINDS.filter(k => sheets[k.value]);
  const src = sheets[kind];
  const alt = `The ${KINDS.find(k => k.value === kind)?.label.toLowerCase()} sheet`;
  return (
    <Sheet
      id="avatar-spring-sheets"
      icon="magnifying-glass"
      title="Spring sheets"
      meta={META[kind]}
      onClose={onClose}
      width={1080}
    >
      <div className="flex flex-col gap-3 overflow-y-auto px-5 py-4">
        {options.length > 1 ? (
          <SegmentedControl label="Show" options={options} value={kind} onChange={setKind} />
        ) : null}
        {src ? (
          <img src={src} alt={alt} className="block w-full rounded-[8px] border border-line" />
        ) : null}
      </div>
    </Sheet>
  );
}
