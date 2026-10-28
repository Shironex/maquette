/**
 * The refs sheet of one asset, opened from its Board card or a ref
 * warning: its views (RefsBody), Make the other views and Clean
 * background, each a page of the same sheet.
 */
import { Icon, InlineBanner } from '@maquette/ui';
import { viewsLine } from '@/state/refs';
import { Sheet } from '@/features/common/Sheet';
import { CleanBackground } from './CleanBackground';
import { OtherViews } from './OtherViews';
import { RefsBody } from './RefsBody';
import { useRefsSheet, useTileWarnings } from './RefsSheet.hooks';

export default function RefsSheet() {
  const { sheet, view, error, close, send } = useRefsSheet();
  const warnings = useTileWarnings(sheet ? view : null);
  if (!sheet) return null;
  const name = view?.asset.name ?? 'Asset';
  const file = sheet.file;
  const page =
    sheet.mode === 'views'
      ? { icon: 'sparkle' as const, title: `Make the other views of ${name}` }
      : sheet.mode === 'clean'
        ? { icon: 'magic-wand' as const, title: `Clean background, ${name}` }
        : { icon: 'images' as const, title: `${name} refs` };
  const meta = !view
    ? 'Reading the refs'
    : sheet.mode === 'views'
      ? 'Codex draws them from the front'
      : sheet.mode === 'clean' && file
        ? file.slice(file.lastIndexOf('/') + 1)
        : `${viewsLine(view.refs)} · front first in every send`;

  return (
    <Sheet id="refs" icon={page.icon} title={page.title} meta={meta} onClose={close} width={560}>
      {error ? (
        <div className="p-5">
          <InlineBanner tone="danger" title="Could not read the refs" detail={error} />
        </div>
      ) : !view ? (
        <div className="flex flex-1 items-center justify-center gap-2 text-fg-mute">
          <Icon name="circle-notch" spin size={16} /> Reading the refs
        </div>
      ) : sheet.mode === 'views' ? (
        <OtherViews key={view.asset.id} view={view} />
      ) : sheet.mode === 'clean' && file ? (
        <CleanBackground key={file} view={view} file={file} />
      ) : (
        <RefsBody view={view} warnings={warnings} onSend={send} />
      )}
    </Sheet>
  );
}
