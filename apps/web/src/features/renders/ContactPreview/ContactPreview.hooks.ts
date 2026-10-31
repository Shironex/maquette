import { fileUrl } from '@/ipc/files';
import { fileOf, shortPath, STYLE_LABEL } from '@/state/renders';
import { useRenderStore } from '@/state/use-render-store';

/** The tiles of the contact sheet and the lines of the folder list. */
export function useContactPreview() {
  const s = useRenderStore();
  const targets = s.record
    ? s.record.items.map(i => i.target)
    : (s.plan?.targets ?? []).filter(t => s.ticked.includes(t.key));
  const stylesOf = (key: string) =>
    s.record
      ? (s.record.items.find(i => i.target.key === key)?.files.map(f => f.style) ?? [])
      : (s.rows[key] ?? []);
  return {
    settings: s.settings,
    locked: s.record !== null,
    sheet: s.sheet,
    setSheet: s.setSheet,
    change: s.change,
    look: STYLE_LABEL[s.settings.look].toLowerCase(),
    root: `${shortPath(s.record?.dir ?? s.plan?.dir ?? '')}/`,
    tiles: targets.map(t => ({
      key: t.key,
      name: t.name.toLowerCase(),
      url: fileUrl(t.thumbnail),
    })),
    folders: targets.map(t => ({
      key: t.key,
      line: `${t.folder}/ ${stylesOf(t.key)
        .map(x => fileOf(x, s.settings))
        .join(', ')}`,
    })),
  };
}
