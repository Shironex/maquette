import { useMemo } from 'react';
import { progressOf, runningOf, STYLE_LABEL } from '@/state/renders';
import { useRenderStore } from '@/state/use-render-store';

/** The batch sheet: its heading, what it would write, and how far the render got. */
export function useRenderSheet() {
  const s = useRenderStore();
  const { record, ticked, rows, plan } = s;
  const count = record ? record.items.length : ticked.length;
  const picked = useMemo(
    () => (plan?.targets ?? []).filter(t => ticked.includes(t.key)),
    [plan?.targets, ticked]
  );
  const fileCount = picked.reduce((sum, t) => sum + (rows[t.key]?.length ?? 0), 0);
  const running = record ? runningOf(record) : null;
  const progress = record ? progressOf(record) : null;
  const problems = record
    ? record.items.flatMap(item =>
        item.files.flatMap(f =>
          f.problem ? [{ name: item.target.name, file: f, problem: f.problem }] : []
        )
      )
    : [];
  const leftOut = record ? [] : picked.filter(t => !t.hasColour && rows[t.key]?.includes('toon'));
  return {
    ...s,
    count,
    picked,
    fileCount,
    progress,
    problems,
    leftOut,
    isProject: s.source?.from === 'project',
    title: `Render ${count} ${count === 1 ? 'asset' : 'assets'}`,
    runningLine: running
      ? `Headless Blender · ${running.name}, ${STYLE_LABEL[running.file.style].toLowerCase()}`
      : null,
  };
}
