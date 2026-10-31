import { Sheet } from '@/features/common/Sheet';
import { PackCheck } from '../PackCheck';
import { PackDone } from '../PackDone';
import { PackSettings } from '../PackSettings';
import { usePackSheet } from './PackSheet.hooks';

/**
 * Export pack: the settings for a selection, a collection or a project's
 * models, then the check while Blender writes and opens each model again,
 * then the finished pack. Closing it leaves a running pack running; Recent
 * packs opens it again.
 */
export default function PackSheet() {
  const s = usePackSheet();
  return (
    <Sheet
      id="export-pack"
      icon="package"
      title={s.title}
      meta={s.meta}
      onClose={s.close}
      width={s.width}
    >
      {s.phase === 'settings' ? <PackSettings /> : null}
      {s.phase === 'check' && s.record ? <PackCheck record={s.record} /> : null}
      {s.phase === 'done' && s.record ? <PackDone record={s.record} /> : null}
    </Sheet>
  );
}
