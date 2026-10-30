/**
 * The Library's bulk actions: every one takes a set of entry ids, so tags,
 * collections, Use in project and Remove work the same on one model or on
 * two hundred. Each reloads the library after, says what it did, and Remove
 * offers Undo. None of it spends credits.
 */
import { commands, errorMessage, type CollectionChange, type LibraryView } from '@/ipc/client';
import { useAppStore } from './use-app-store';
import { useLibraryStore } from './use-library-store';
import { useLibrarySelectionStore } from './use-library-selection-store';

const models = (n: number) => `${n} ${n === 1 ? 'model' : 'models'}`;

function failed(title: string, error: unknown) {
  useAppStore.getState().notify({ status: 'failed', title, detail: errorMessage(error) });
}

async function reload() {
  await useLibraryStore.getState().load();
}

/** Puts `add` on every entry and takes `remove` off. Resolves false when refused. */
export async function tagEntries(
  entries: readonly string[],
  add: readonly string[],
  remove: readonly string[]
): Promise<boolean> {
  try {
    await commands.libraryTag({ entries: [...entries], add: [...add], remove: [...remove] });
    await reload();
    return true;
  } catch (error) {
    failed('Could not change the tags', error);
    return false;
  }
}

export async function renameEntry(entry: string, name: string): Promise<boolean> {
  try {
    await commands.libraryRename(entry, name);
    await reload();
    return true;
  } catch (error) {
    failed('Could not rename the model', error);
    return false;
  }
}

/** Creates, fills, empties, renames or deletes a collection. */
export async function changeCollection(change: CollectionChange): Promise<boolean> {
  try {
    await commands.libraryCollection(change);
    await reload();
    return true;
  } catch (error) {
    failed('Could not change the collection', error);
    return false;
  }
}

/** Brings removed models back, or undoes a merge. */
export async function restoreEntries(entries: readonly string[]): Promise<boolean> {
  const { notify, setProject, workspace } = useAppStore.getState();
  try {
    const done = await commands.libraryRestore([...entries]);
    const open = done.projects.find(p => p.id === workspace?.project?.id);
    if (open) setProject(open);
    await reload();
    notify({ status: 'done', title: `Restored ${models(done.entries.length)}` });
    return true;
  } catch (error) {
    failed('Could not restore', error);
    return false;
  }
}

/** Removes models from the library; copies in projects stay. Offers Undo. */
export async function removeEntries(entries: readonly string[]): Promise<void> {
  const { notify } = useAppStore.getState();
  try {
    const removed = await commands.libraryRemove([...entries]);
    useLibrarySelectionStore.getState().clear();
    await reload();
    notify({
      status: 'done',
      title: `Removed ${models(removed.length)} from the library`,
      detail: 'Copies in projects stay as they are.',
      action: { label: 'Undo', run: () => void restoreEntries(removed) },
    });
  } catch (error) {
    failed('Could not remove', error);
  }
}

/** Copies each entry's newest version into a project, one after another. */
export async function copyIntoProject(entries: readonly string[], project: string): Promise<void> {
  const { notify, setProject } = useAppStore.getState();
  const view: LibraryView | null = useLibraryStore.getState().view;
  const names = new Map(view?.items.map(i => [i.entry.id, i.entry.name]) ?? []);
  const refused: string[] = [];
  let last = null;
  for (const entry of entries) {
    try {
      last = await commands.libraryUse(entry, null, project);
    } catch (error) {
      refused.push(`${names.get(entry) ?? entry}: ${errorMessage(error)}`);
    }
  }
  if (last) setProject(last.project);
  await reload();
  const added = entries.length - refused.length;
  notify({
    status: refused.length ? (added ? 'info' : 'failed') : 'done',
    title: added
      ? `Added ${models(added)} to ${last?.project.name ?? project}, no credits`
      : 'Could not add the models',
    detail: refused.length ? refused.slice(0, 3).join(' ') : undefined,
  });
}
