import type { FixupKind } from '@/ipc/client';
import { formatDims, scaleFor } from '@/lib/real-size';
import { vec3 } from '@/state/inspector';
import {
  FIXUP_ORDER,
  defaultFixup,
  describeSize,
  hasNoPlan,
  skipsBake,
  startSize,
  withFixup,
  withSize,
  withSkipBake,
  withoutFixup,
} from '@/state/prepare';
import type { PrepEditorProps } from './PrepEditor.types';

/** What the editor shows: the measured and scaled sizes, the kinds left to add and the edits. */
export function usePrepEditor({
  value,
  onChange,
  suggestion,
  defaults,
  skipBakeDefault = false,
  ...props
}: PrepEditorProps) {
  const measured = props.measured ? vec3(props.measured) : null;
  const base = props.fixed ? vec3(props.fixed) : measured;
  const scale = value.size && base ? scaleFor(value.size, base) : null;
  const taken = JSON.stringify(value.size) === JSON.stringify(suggestion?.size);
  const suggested =
    suggestion && !taken
      ? {
          text: describeSize(suggestion.size),
          matched: suggestion.matched,
          use: () => onChange(withSize(value, suggestion.size)),
        }
      : null;
  return {
    measuredText: measured ? formatDims(measured) : null,
    /** "×0.643, so 32 × 45 × 32 cm", once there is a size and a model. */
    scaledText:
      scale && base
        ? `×${Number(scale.toPrecision(3))}, so ${formatDims(base.map(v => v * scale) as typeof base)}`
        : null,
    flat: Boolean(value.size && base && scale === null),
    suggested,
    missing: FIXUP_ORDER.filter(kind => !value.fixups.some(f => f.op === kind)),
    empty: hasNoPlan(value),
    skipBake: skipsBake(value, skipBakeDefault),
    setSkipBake: (skip: boolean) => onChange(withSkipBake(value, skip)),
    startSize: () => onChange(withSize(value, startSize(measured))),
    clearSize: () => onChange(withSize(value, null)),
    add: (kind: FixupKind) => onChange(withFixup(value, defaultFixup(kind, defaults))),
    remove: (kind: FixupKind) => onChange(withoutFixup(value, kind)),
  };
}
