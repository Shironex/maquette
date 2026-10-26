import type { MenuItem } from '@/features/common/Menu';
import { projectFile } from '@/state/board';
import { isGame } from '@/state/game';
import { STAGES } from '@/state/pipeline';
import { useStageLabels } from '@/state/use-stage-labels';
import { useBuiltStore } from '@/state/use-built-store';
import { useLabStore } from '@/state/use-lab-store';
import { updateFor } from '@/state/library';
import { useLibraryStore } from '@/state/use-library-store';
import { useProvidersStore } from '@/state/use-providers-store';
import { STAGE_ICON } from '../../constants';
import type { CardMenuProps } from './CardMenu.types';

/** The items of a card's menu, and of its Copy to project list. */
export function useCardMenu({ asset, waiting, refPicture, built, board }: CardMenuProps) {
  const STAGE_LABEL = useStageLabels();
  const openPanel = useProvidersStore(s => s.openPanel);
  const rebuild = useBuiltStore(s => s.rebuild);
  const showHandoff = useProvidersStore(s => s.showHandoff);
  const addModel = useProvidersStore(s => s.addModel);
  const openObjects = useLabStore(s => s.openObjects);
  const { view: library, openAdd, openUpdate } = useLibraryStore();
  const waitingUpdate = updateFor(library, asset);
  const { workspace, project, otherProjects, actions } = board;
  const others = STAGES.filter(s => s !== asset.stage);

  const copyItems: MenuItem[] = otherProjects.map(p => ({
    id: p.id,
    label: p.name,
    icon: 'folder-simple' as const,
    disabled: actions.copying,
    hint: actions.copying ? 'Copying' : undefined,
    onSelect: () => void actions.copyTo(asset, p),
  }));
  const items: MenuItem[] = [
    ...(waiting
      ? [
          {
            id: 'handoff',
            label: 'Settings card',
            icon: 'file-text' as const,
            onSelect: () => showHandoff(waiting),
          },
          {
            id: 'hand-back',
            label: 'Add model...',
            icon: 'upload-simple' as const,
            onSelect: () => void addModel(waiting.id),
          },
        ]
      : []),
    ...(built
      ? [
          {
            id: 'rebuild',
            label: 'Rebuild',
            icon: 'arrow-clockwise' as const,
            disabled: built.state === 'building',
            hint:
              built.state === 'building'
                ? 'Building now'
                : `Runs ${built.source.script} in Blender headless again`,
            onSelect: () => {
              actions.setMenuFor(null);
              if (project) void rebuild(project.id, asset.id, true);
            },
          },
        ]
      : [
          {
            id: 'send',
            label: 'Send to 3D',
            icon: 'cube' as const,
            onSelect: () => void openPanel(project?.id ?? '', asset.id),
          },
        ]),
    ...(!built && refPicture && workspace && project
      ? [
          {
            id: 'objects',
            label: 'Make object refs...',
            icon: 'cube-focus' as const,
            onSelect: () =>
              openObjects({
                project: project.id,
                source: refPicture,
                sourcePath: projectFile(workspace, refPicture) ?? '',
                sourceName: asset.name,
              }),
          },
        ]
      : []),
    {
      id: 'rename',
      label: 'Rename...',
      icon: 'file-text',
      onSelect: () => actions.setRenaming(asset),
    },
    {
      id: 'size',
      label: 'Real size and fix-ups...',
      icon: 'ruler',
      hint: isGame(project)
        ? 'How big it really is, and repairs made on a copy before the model is exported'
        : 'How big it really is, and repairs made on a copy before the scene is built',
      onSelect: () => actions.setSizing(asset),
    },
    {
      id: 'copy',
      label: 'Copy to project...',
      icon: 'copy',
      hint: 'Copies the card with its refs and versions; it stays here too',
      keepOpen: true,
      onSelect: () => actions.setCopyFor(asset.id),
    },
    {
      id: 'library-add',
      label: 'Add to library...',
      icon: 'books',
      disabled: asset.versions.length === 0,
      hint:
        asset.versions.length === 0
          ? 'No model yet'
          : 'Copies its newest model so other projects can use it without spending credits',
      onSelect: () => project && openAdd({ project: project.id, asset: asset.id }),
    },
    ...(waitingUpdate && project
      ? [
          {
            id: 'library-update',
            label: `Update to library v${waitingUpdate.to}...`,
            icon: 'arrow-up' as const,
            hint: `It is on v${waitingUpdate.from} of ${waitingUpdate.entry.entry.name}`,
            onSelect: () => void openUpdate({ project: project.id, asset: asset.id }),
          },
        ]
      : []),
    ...others.map(s => ({
      id: `move-${s}`,
      label: `Move to ${STAGE_LABEL[s]}`,
      icon: STAGE_ICON[s],
      onSelect: () => void actions.move(asset, s),
    })),
    {
      id: 'blender',
      label: 'Open in Blender',
      icon: 'arrow-square-out',
      disabled: asset.versions.length === 0,
      hint: asset.versions.length === 0 ? 'No model yet' : undefined,
      onSelect: () => void actions.openInBlender(asset),
    },
  ];
  const closeAll = () => {
    actions.setMenuFor(null);
    actions.setCopyFor(null);
  };

  return { copyItems, items, closeAll };
}
