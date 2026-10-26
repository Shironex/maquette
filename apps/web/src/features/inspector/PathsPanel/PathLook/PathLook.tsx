import type { ReactNode } from 'react';
import {
  NumberField,
  SegmentedControl,
  Select,
  TextField,
  type SegmentedOption,
} from '@maquette/ui';
import type { PathCurve, ScenePath } from '@/ipc/client';
import { num } from '@/state/inspector';
import { MAX_WIDTH, MIN_WIDTH, PATH_NAME_MAX, setPath, type PathsDraft } from '@/state/paths';
import { useHexText } from '../../MaterialPanel/MaterialPanel.hooks';
import { SectionTitle } from '../../SectionTitle';

const CURVES: SegmentedOption<PathCurve>[] = [
  { value: 'catmull-rom', label: 'Smooth', hint: 'A smooth curve through every point' },
  { value: 'polyline', label: 'Straight', hint: 'Straight runs from point to point' },
];

export interface PathLookProps {
  path: ScenePath;
  change: (fn: (draft: PathsDraft, id: string) => PathsDraft) => void;
}

function Row({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="flex min-h-7 items-center justify-between gap-2">
      <span className="text-fg-dim">{label}</span>
      <div className="flex items-center gap-1.5">{children}</div>
    </div>
  );
}

function Colour({
  label,
  value,
  onChange,
}: {
  label: string;
  value: string;
  onChange: (hex: string) => void;
}) {
  const hex = useHexText(value, onChange);
  return (
    <Row label={label}>
      <input
        type="color"
        aria-label={`${label} picker`}
        value={value}
        onChange={e => onChange(e.target.value)}
        className="size-7 flex-none cursor-pointer rounded-control border border-line bg-app p-0.5"
      />
      <TextField
        aria-label={`${label} hex`}
        mono
        value={hex.text}
        onChange={e => hex.setText(e.target.value)}
        onBlur={hex.commit}
        onKeyDown={e => {
          if (e.key === 'Enter') hex.commit();
        }}
        className="h-7"
        fieldClassName="w-[92px]"
      />
    </Row>
  );
}

/** The selected path's name, curve and look. */
export default function PathLook({ path, change }: PathLookProps) {
  return (
    <div className="flex flex-col gap-2.5">
      <SectionTitle>Look</SectionTitle>
      <TextField
        label="Name"
        value={path.name}
        maxLength={PATH_NAME_MAX}
        onChange={e => {
          const name = e.target.value;
          change((d, id) => setPath(d, id, { name }));
        }}
      />
      <Row label="Curve">
        <SegmentedControl
          label="Curve"
          options={CURVES}
          value={path.curve}
          onChange={curve => change((d, id) => setPath(d, id, { curve }))}
        />
      </Row>
      <Colour
        label="Colour"
        value={path.color}
        onChange={color => change((d, id) => setPath(d, id, { color }))}
      />
      <Colour
        label="Glow"
        value={path.glow}
        onChange={glow => change((d, id) => setPath(d, id, { glow }))}
      />
      <Row label="Width">
        <NumberField
          aria-label="Width"
          size="sm"
          unit="cm"
          min={MIN_WIDTH * 100}
          max={MAX_WIDTH * 100}
          value={num(path.width) * 100}
          format={v => v.toFixed(1)}
          fieldClassName="w-[92px]"
          onChange={cm => change((d, id) => setPath(d, id, { width: cm / 100 }))}
        />
      </Row>
      <Row label="Ends">
        <Select
          aria-label="Ends"
          size="sm"
          options={[{ value: 'open', label: 'Open' }]}
          value="open"
          disabled
          title="Not built yet: the tube ends open"
          fieldClassName="w-[128px]"
        />
      </Row>
    </div>
  );
}
