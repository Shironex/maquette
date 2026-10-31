import { copyText } from '@/features/config/copyText';
import type { PackRecord } from '@/ipc/client';
import { fileUrl } from '@/ipc/files';
import { formatBytes } from '@/lib/format';
import {
  doneLine,
  formatTook,
  manifestSnippet,
  packTree,
  passes,
  shortTris,
  triangleTotals,
} from '@/state/packs';
import { useAppStore } from '@/state/use-app-store';
import { useFileManager } from '@/state/use-file-manager';
import { usePackStore } from '@/state/use-pack-store';

/** A finished pack's numbers, tree, manifest and contact sheet, and its actions. */
export function usePackDone(record: PackRecord) {
  const store = usePackStore();
  const files = useFileManager();
  const notify = useAppStore(s => s.notify);
  const kept = record.items.filter(passes);
  const { before, after } = triangleTotals(record);
  const took = record.finishedAt ? Date.parse(record.finishedAt) - Date.parse(record.createdAt) : 0;
  const path = record.zip ?? record.dir;
  const where = record.dir.replace(/[\\/][^\\/]+$/, '');
  const stats = [
    {
      label: 'Size',
      value: record.bytes ? formatBytes(record.bytes) : 'n/a',
      note: record.zip ? 'zipped' : 'in the folder',
    },
    {
      label: 'Models',
      value: String(kept.length),
      note: kept.length === record.items.length ? 'all pass' : `of ${record.items.length}`,
    },
    { label: 'Triangles', value: shortTris(after), note: `was ${shortTris(before)}` },
    { label: 'Took', value: formatTook(took), note: 'in Blender' },
  ];
  return {
    busy: store.busy,
    recent: store.recent,
    showRecord: store.showRecord,
    resume: store.resume,
    again: store.again,
    reveal: store.reveal,
    revealLabel: files.reveal(record.zip ? 'zip' : 'folder'),
    copy: async () => {
      const ok = await copyText(path);
      notify(
        ok
          ? { status: 'done', title: 'Copied the path' }
          : { status: 'failed', title: 'Could not copy the path' }
      );
    },
    line: doneLine(record),
    stats,
    tree: packTree(record),
    manifest: record.settings.manifest ? manifestSnippet(record) : null,
    sheet: record.settings.contactSheet ? fileUrl(`${record.dir}/contact-sheet.png`) : null,
    pictures: kept.map(i => ({ key: i.model.key, url: fileUrl(i.picture ?? i.model.thumbnail) })),
    where,
    kept: kept.length,
  };
}
