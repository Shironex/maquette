import { NEW_COLLECTION, isClash, summarise } from '@/state/add-many';
import { useAddManyStore } from '@/state/use-add-many-store';
import { useAppStore } from '@/state/use-app-store';
import { useLibraryStore } from '@/state/use-library-store';

const NONE: never[] = [];

/** The add many sheet's plan, what it adds, and the options of its pickers. */
export function useAddManySheet() {
  const s = useAddManyStore();
  const projects = useAppStore(st => st.workspace?.projects) ?? NONE;
  const collections = useLibraryStore(st => st.view?.collections) ?? NONE;
  const { plan } = s;
  const sum = plan ? summarise(plan) : null;
  const count = sum
    ? sum.adding.length + sum.clashes.filter(c => s.choices[c.asset] !== 'skip').length
    : 0;
  return {
    ...s,
    sum,
    count,
    busy: s.progress !== null,
    clashes: plan?.assets.filter(isClash) ?? [],
    title: plan
      ? `Add ${plan.assets.length} models from ${plan.projectName}`
      : 'Add a whole project',
    projectOptions: [
      { value: '', label: 'Pick a project' },
      ...projects.map(p => ({ value: p.id, label: p.name })),
    ],
    collectionOptions: [
      { value: '', label: 'No collection' },
      ...collections.map(c => ({ value: c.id, label: c.name })),
      { value: NEW_COLLECTION, label: 'New collection...' },
    ],
  };
}
