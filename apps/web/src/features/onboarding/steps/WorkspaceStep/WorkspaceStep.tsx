import { Icon, PathField } from '@maquette/ui';
import { Panel } from '@/features/common/Panel';
import { SectionLabel } from '@/features/common/SectionLabel';
import { rootNote } from '@/state/onboarding';
import type { WorkspaceStepProps } from './WorkspaceStep.types';

export default function WorkspaceStep({ api }: WorkspaceStepProps) {
  const { draft, created } = api;
  const { layout } = draft.config;
  const folders = [
    {
      name: layout.projects,
      text: 'One folder per project: concepts, refs, models, the .blend and the web export',
    },
    {
      name: layout.inbox,
      text: 'Finished 3D models land here and get checked before they reach Blender',
    },
    {
      name: layout.cache,
      text: 'Light bakes and export results, so unchanged steps are skipped. Safe to clear.',
    },
  ];
  const rows = folders.map(f => (
    <div
      key={f.name}
      className="flex items-center gap-3 border-t border-line px-4 py-3.5 first:border-t-0"
    >
      <Icon name="folder" className="text-fg-mute" />
      <span className="w-[120px] flex-none font-mono text-[13px] font-medium tracking-[-0.01em]">
        {f.name}/
      </span>
      <span className="flex-1 text-fg-dim">{f.text}</span>
    </div>
  ));
  return (
    <>
      <PathField
        label="Root folder"
        size="lg"
        className="mt-8"
        value={draft.root}
        onChange={
          created ? undefined : root => api.update({ root, inspection: null, rootError: null })
        }
        onCommit={created ? undefined : root => void api.inspectRoot(root)}
        onBrowse={created ? undefined : () => void api.browseRoot()}
        note={rootNote(draft, created)}
      />
      <SectionLabel className="mt-8">I will create</SectionLabel>
      <Panel className="mt-2.5 gap-0 overflow-hidden p-0">{rows}</Panel>
      <div className="mt-4 flex gap-2.5 rounded-card border border-line bg-panel px-3.5 py-3 text-fg-dim">
        <Icon name="info" className="mt-px" />
        <span className="text-meta text-fg-dim">
          Project folders stay plain files. You can open the .blend in Blender or put a project
          under git without Maquette running.
        </span>
      </div>
    </>
  );
}
