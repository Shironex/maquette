import { useState } from 'react';
import { errorMessage } from '@/ipc/client';
import { useAppStore } from '@/state/use-app-store';
import { useLibraryStore, type AssetRef } from '@/state/use-library-store';

/** Splits "props, Lantern post" into tags; the core lowercases and trims them again. */
export function parseTags(text: string): string[] {
  return text
    .split(',')
    .map(t => t.trim())
    .filter(Boolean);
}

/**
 * The asset to add (fixed, or picked from a project's assets that have a
 * model), its library name and tags, and the add call.
 */
export function useAddToLibraryDialog(target: AssetRef | 'pick') {
  const projects = useAppStore(s => s.workspace?.projects ?? []);
  const current = useAppStore(s => s.workspace?.project?.id);
  const view = useLibraryStore(s => s.view);
  const { add, closeAdd, busy } = useLibraryStore();
  const picking = target === 'pick';
  const [projectId, setProjectId] = useState(
    picking ? (current ?? projects[0]?.id ?? '') : target.project
  );
  const project = projects.find(p => p.id === projectId) ?? null;
  const withModel = (project?.assets ?? []).filter(a => a.versions.length > 0);
  const [assetId, setAssetId] = useState(picking ? (withModel[0]?.id ?? '') : target.asset);
  const asset = (project?.assets ?? []).find(a => a.id === assetId) ?? null;
  const entry = view?.items.find(i => i.entry.id === asset?.origin?.entry)?.entry ?? null;
  const [name, setName] = useState(entry?.name ?? asset?.name ?? '');
  const [tags, setTags] = useState(entry?.tags.join(', ') ?? '');
  const [error, setError] = useState<string | null>(null);

  const pickProject = (id: string) => {
    setProjectId(id);
    const first = (projects.find(p => p.id === id)?.assets ?? []).find(a => a.versions.length);
    setAssetId(first?.id ?? '');
    setName(first?.name ?? '');
    setError(null);
  };
  const pickAsset = (id: string) => {
    setAssetId(id);
    setName(project?.assets?.find(a => a.id === id)?.name ?? '');
    setError(null);
  };

  const submit = async () => {
    if (!project || !asset) return setError('Pick an asset with a model.');
    if (!asset.versions.length) {
      return setError(`${asset.name} has no model yet. Accept or build one first.`);
    }
    if (!name.trim()) return setError('Give the model a name.');
    try {
      await add({ project: project.id, asset: asset.id, name: name.trim(), tags: parseTags(tags) });
    } catch (err) {
      setError(errorMessage(err));
    }
  };

  return {
    picking,
    projects,
    projectId,
    pickProject,
    withModel,
    assetId,
    pickAsset,
    asset,
    entry,
    latest: asset?.versions.at(-1)?.version ?? null,
    name,
    setName,
    tags,
    setTags,
    error,
    busy,
    close: closeAdd,
    submit,
  };
}
