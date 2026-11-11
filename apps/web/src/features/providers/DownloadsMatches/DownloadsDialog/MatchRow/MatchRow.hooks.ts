import { jobKey } from '@/state/providers';
import { useProvidersStore } from '@/state/use-providers-store';

/**
 * Picking the send a file is for, and using it, busy while that send imports.
 * The scan lists one project's sends, so `choice` is an id of that project.
 */
export function useMatchRow(choice: string | null) {
  const choose = useProvidersStore(s => s.chooseMatch);
  const use = useProvidersStore(s => s.useDownload);
  const busy = useProvidersStore(s => {
    const project = s.downloads?.project;
    return choice && project ? s.busy.includes(jobKey({ project, id: choice })) : false;
  });
  return { choose, use, busy };
}
