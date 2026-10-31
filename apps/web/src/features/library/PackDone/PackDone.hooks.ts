import { useState } from 'react';
import { copyText } from '@/features/config/copyText';
import type { PackRecord } from '@/ipc/client';
import { fileUrl } from '@/ipc/files';
import { formatBytes } from '@/lib/format';
import {
  doneLine,
  importRows,
  formatTook,
  manifestSnippet,
  packTree,
  shippedItems,
  shippedWarnings,
  shortTris,
  triangleTotals,
} from '@/state/packs';
import { useAppStore } from '@/state/use-app-store';
import { useFileManager } from '@/state/use-file-manager';
import { usePackStore } from '@/state/use-pack-store';

/** A finished pack's numbers, tree, manifest and contact sheet, and its actions. */
export function usePackDone(record: PackRecord) {
  const store = usePackStore();
  // A pack written outside the workspace is out of the web view's reach, so
  // its sheet gives way to the pictures it was made from.
  const [sheetFailed, setSheetFailed] = useState(false);
  const files = useFileManager();
  const notify = useAppStore(s => s.notify);
  const kept = shippedItems(record);
  const warned = shippedWarnings(kept).length;
  const { before, after } = triangleTotals(record);
  // The time in Blender, not since it started: a pack may have sat stopped.
  const took = record.items.reduce((ms, i) => ms + (i.tookMs ?? 0), 0);
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
      note:
        kept.length < record.items.length
          ? `of ${record.items.length}`
          : warned
            ? `${warned} with a warning`
            : 'all pass',
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
    sheet:
      record.settings.contactSheet && !sheetFailed
        ? fileUrl(`${record.dir}/contact-sheet.png`)
        : null,
    sheetFailed: () => setSheetFailed(true),
    kept,
    where,
    imports: importRows(record),
  };
}
