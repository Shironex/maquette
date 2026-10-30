import type { MenuItem } from '@/features/common/Menu';
import type { ProviderJob } from '@/ipc/client';
import { projectFile } from '@/state/board';
import { isGame } from '@/state/game';
import { STAGES } from '@/state/pipeline';
import { exportNameOf } from '@/state/providers';
import { useStageLabels } from '@/state/use-stage-labels';
import { useBuiltStore } from '@/state/use-built-store';
import { useFileManager } from '@/state/use-file-manager';
import { useLabStore } from '@/state/use-lab-store';
import { updateFor } from '@/state/library';
import { useLibraryStore } from '@/state/use-library-store';
import { useProvidersStore } from '@/state/use-providers-store';
import { assetQuickView } from '@/state/quick-view';
import { useQuickViewStore } from '@/state/use-quick-view-store';
import { useRefsStore } from '@/state/use-refs-store';
import { STAGE_ICON } from '../../constants';
import { copyExportName } from '../../copyExportName';
import type { CardMenuProps } from './CardMenu.types';

/**
 * The By hand section, while the card waits for its model: the export name,
 * the front image and the folder for the site, then the card and Add model.
 */
function useHandoffItems(waiting: ProviderJob | undefined): MenuItem[] {
  const showHandoff = useProvidersStore(s => s.showHandoff);
  const addModel = useProvidersStore(s => s.addModel);
  const openHandoff = useProvidersStore(s => s.openHandoff);
  const revealImage = useProvidersStore(s => s.revealImage);
  const fileManager = useFileManager();
  if (!waiting) return [];
  const card = waiting.card;
  const name = card ? exportNameOf(card) : '';
  const front = card?.images[0];
  const section = 'handoff';
  return [
    ...(name
      ? [
          {
            id: 'export-name',
            label: 'Copy export name',
            icon: 'copy' as const,
            section,
            hint: name,
            onSelect: () => void copyExportName(name),
          },
        ]
      : []),
    ...(front
      ? [
          {
            id: 'reveal-image',
            label: fileManager.reveal('image'),
            icon: 'image' as const,
            section,
            hint: `Selects ${front}, ready to drag into the browser`,
            onSelect: () => void revealImage(waiting.id, front),
          },
        ]
      : []),
    ...(waiting.handoff
      ? [
          {
            id: 'handoff-folder',
            label: 'Open handoff folder',
            icon: 'folder-open' as const,
            section,
            onSelect: () => void openHandoff(waiting.id),
          },
        ]
      : []),
    {
      id: 'handoff',
      label: 'Settings card',
      icon: 'file-text',
      section,
      onSelect: () => showHandoff(waiting),
    },
    {
      id: 'hand-back',
      label: 'Add model...',
      icon: 'upload-simple',
      section,
      onSelect: () => void addModel(waiting.id),
    },
  ];
}

/** Making the model: Send to 3D (or Rebuild for a built asset) and Make object refs. */
function useMakeItems({ asset, refPicture, built, board }: CardMenuProps): MenuItem[] {
  const openPanel = useProvidersStore(s => s.openPanel);
  const rebuild = useBuiltStore(s => s.rebuild);
  const openObjects = useLabStore(s => s.openObjects);
  const openRefs = useRefsStore(s => s.open);
  const { workspace, project, actions } = board;
  const section = 'make';
  const make: MenuItem = built
    ? {
        id: 'rebuild',
        label: 'Rebuild',
        icon: 'arrow-clockwise',
        section,
        disabled: built.state === 'building',
        hint:
          built.state === 'building'
            ? 'Building now'
            : `Runs ${built.source.script} in Blender headless again`,
        onSelect: () => {
          actions.setMenuFor(null);
          if (project) void rebuild(project.id, asset.id, true);
        },
      }
    : {
        id: 'send',
        label: 'Send to 3D',
        icon: 'cube',
        section,
        onSelect: () => void openPanel(project?.id ?? '', asset.id),
      };
  return [
    make,
    ...(!built && project
      ? [
          {
            id: 'refs',
            label: 'Refs and views...',
            icon: 'images' as const,
            section,
            hint: 'Which ref is the front, left, back and right; sends follow these',
            onSelect: () => void openRefs(project.id, asset.id),
          },
        ]
      : []),
    ...(!built && refPicture && workspace && project
      ? [
          {
            id: 'objects',
            label: 'Make object refs...',
            icon: 'cube-focus' as const,
            section,
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
  ];
}

/**
 * The asset itself: rename, size, copy, the library, the 3D quick view,
 * Blender and, last, remove. The library, quick view and Blender items wait
 * for a model and stay hidden until then.
 */
function useAssetItems({ asset, built, board }: CardMenuProps): MenuItem[] {
  const { view: library, openAdd, openUpdate } = useLibraryStore();
  const addModelFile = useProvidersStore(s => s.addModelFile);
  const waitingUpdate = updateFor(library, asset);
  const openQuickView = useQuickViewStore(s => s.open);
  const { project, actions } = board;
  const hasModel = asset.versions.length > 0;
  const quickView = assetQuickView(asset, board.workspace);
  const section = 'asset';
  return [
    {
      id: 'rename',
      label: 'Rename...',
      icon: 'file-text',
      section,
      onSelect: () => actions.setRenaming(asset),
    },
    {
      id: 'size',
      label: 'Real size and fix-ups...',
      icon: 'ruler',
      section,
      hint: isGame(project)
        ? 'How big it really is, and repairs made on a copy before the model is exported'
        : 'How big it really is, and repairs made on a copy before the scene is built',
      onSelect: () => actions.setSizing(asset),
    },
    {
      id: 'copy',
      label: 'Copy to project...',
      icon: 'copy',
      section,
      hint: 'Copies the card with its refs and versions; it stays here too',
      keepOpen: true,
      onSelect: () => actions.setCopyFor(asset.id),
    },
    ...(project && !built
      ? [
          {
            id: 'model-file',
            label: 'Add a model file as the next version...',
            icon: 'upload-simple' as const,
            section,
            hint: 'A .glb, .gltf, .fbx or .obj you fixed yourself; it is copied into the Inbox for this card',
            onSelect: () => void addModelFile(project.id, asset.id),
          },
        ]
      : []),
    ...(hasModel
      ? [
          {
            id: 'library-add',
            label: 'Add to library...',
            icon: 'books' as const,
            section,
            hint: 'Copies its newest model so other projects can use it without spending credits',
            onSelect: () => project && openAdd({ project: project.id, asset: asset.id }),
          },
        ]
      : []),
    ...(waitingUpdate && project
      ? [
          {
            id: 'library-update',
            label: `Update to library v${waitingUpdate.to}...`,
            icon: 'arrow-up' as const,
            section,
            hint: `It is on v${waitingUpdate.from} of ${waitingUpdate.entry.entry.name}`,
            onSelect: () => void openUpdate({ project: project.id, asset: asset.id }),
          },
        ]
      : []),
    ...(quickView
      ? [
          {
            id: 'quick-view',
            label: 'Inspect 3D model',
            icon: 'cube-transparent' as const,
            section,
            hint: `Turns v${quickView.version} around in a 3D preview, without leaving the Board`,
            onSelect: () => openQuickView(quickView),
          },
        ]
      : []),
    ...(hasModel
      ? [
          {
            id: 'blender',
            label: 'Open in Blender',
            icon: 'arrow-square-out' as const,
            section,
            onSelect: () => void actions.openInBlender(asset),
          },
        ]
      : []),
    {
      id: 'remove',
      label: 'Remove from project...',
      icon: 'trash',
      tone: 'danger',
      section,
      hint: 'Takes the card off the Board and moves its files to the workspace trash, where Recently removed can restore it',
      onSelect: () => actions.setRemoving([asset]),
    },
  ];
}

/** The items of a card's menu, and of its Copy to project list. */
export function useCardMenu(props: CardMenuProps) {
  const { asset, waiting, board } = props;
  const STAGE_LABEL = useStageLabels();
  const handoff = useHandoffItems(waiting);
  const make = useMakeItems(props);
  const assetItems = useAssetItems(props);
  const { otherProjects, actions } = board;

  const copyItems: MenuItem[] = otherProjects.map(p => ({
    id: p.id,
    label: p.name,
    icon: 'folder-simple' as const,
    disabled: actions.copying,
    hint: actions.copying ? 'Copying' : undefined,
    onSelect: () => void actions.copyTo(asset, p),
  }));
  const moveTo: MenuItem = {
    id: 'move',
    label: 'Move to',
    icon: 'arrows-out-cardinal',
    section: 'move',
    submenu: STAGES.filter(s => s !== asset.stage).map(s => ({
      id: `move-${s}`,
      label: STAGE_LABEL[s],
      icon: STAGE_ICON[s],
      onSelect: () => void actions.move(asset, s),
    })),
  };
  const items = [...handoff, ...make, ...assetItems, moveTo];
  const closeAll = () => {
    actions.setMenuFor(null);
    actions.setCopyFor(null);
  };

  return { copyItems, items, closeAll };
}
