import type { CheckStatus } from '@maquette/ui';
import type { MotionModel } from '@/ipc/client';
import { formatAgo } from '@/lib/format';
import { useNow } from '@/lib/useNow';
import { useMotionStore } from '@/state/use-motion-store';

export interface RigLine {
  id: string;
  status: CheckStatus;
  title: string;
  detail?: string;
  /** The row offers the bone map. */
  map?: boolean;
}

/** Web skinning plays four influences per vertex at most. */
export const MAX_INFLUENCES = 4;

/** What the rig check says about a model's rig. */
export function rigLines(model: MotionModel): RigLine[] {
  const rig = model.rig;
  if (!rig) {
    return [
      {
        id: 'rig',
        status: 'warning',
        title: 'No rig',
        detail:
          'Clips that move the whole model or its parts still play. Library clips need a rig.',
      },
    ];
  }
  const lines: RigLine[] = [
    {
      id: 'rig',
      status: rig.humanoid ? 'pass' : 'warning',
      title: `${rig.joints} bones, ${rig.humanoid ? 'humanoid rig' : 'not humanoid'}`,
      detail: rig.humanoid
        ? rig.kind === 'mixamo'
          ? 'Mixamo bone names, so library clips need no manual map.'
          : 'Its bones map onto the humanoid names library clips use.'
        : 'Too few bones map onto the humanoid skeleton.',
    },
  ];
  const most = rig.maxInfluences;
  lines.push(
    most === null
      ? {
          id: 'weights',
          status: 'pass',
          title: 'Weights not readable',
          detail: 'The geometry is packed, so the influences per vertex are not counted.',
        }
      : {
          id: 'weights',
          status: most <= MAX_INFLUENCES ? 'pass' : 'fail',
          title: `${most} ${most === 1 ? 'influence' : 'influences'} per vertex at most`,
          detail:
            most <= MAX_INFLUENCES
              ? 'Fits the web skinning limit, nothing to trim.'
              : `The web plays ${MAX_INFLUENCES}; the rest are dropped on export.`,
        }
  );
  const missing = rig.map.filter(b => b.how === 'missing').map(b => b.bone);
  if (missing.length > 0) {
    lines.push({
      id: 'missing',
      status: 'warning',
      title: `${missing.length} ${missing.length === 1 ? 'bone' : 'bones'} the library expects ${missing.length === 1 ? 'is' : 'are'} missing`,
      detail: `${missing.slice(0, 3).join(', ')}${missing.length > 3 ? ` and ${missing.length - 3} more` : ''}. Library clips play; those bones stay still.`,
      map: true,
    });
  }
  return lines;
}

export function useRigCheck() {
  const model = useMotionStore(s => s.view?.models.find(m => m.asset === s.asset) ?? null);
  const setBoneMap = useMotionStore(s => s.setBoneMap);
  const now = useNow(60_000);
  return {
    facts: model
      ? [
          { label: 'Added', value: formatAgo(Date.parse(model.addedAt), now) },
          { label: 'Version', value: `v${model.version}`, mono: true },
          { label: 'File', value: model.path, mono: true },
        ]
      : [],
    model,
    lines: model ? rigLines(model) : [],
    file: model?.path.split('/').pop() ?? '',
    openMap: () => setBoneMap(true),
  };
}
