import { useEffect, useState } from 'react';
import {
  commands,
  errorMessage,
  type MeshySplit,
  type SplitAfter,
  type SplitSheet,
  type TripoSplit,
} from '@/ipc/client';
import { PROVIDER_LABEL } from '@/state/providers';
import { notify, upsert } from '@/state/providers-store-helpers';
import {
  DEFAULT_MESHY,
  DEFAULT_TRIPO,
  afterLine,
  confirmLine,
  meshyLine,
  meshyProblem,
  splitCredits,
  tripoLine,
  type SplitWay,
} from '@/state/split';
import { useAppStore } from '@/state/use-app-store';
import { useProvidersStore } from '@/state/use-providers-store';
import { useSplitStore, type SplitTarget } from '@/state/use-split-store';

/** The sheet's model, loaded once per target. */
function useSheet(target: SplitTarget | null) {
  const [sheet, setSheet] = useState<SplitSheet | null>(null);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    setSheet(null);
    setError(null);
    if (!target) return;
    let live = true;
    commands.splitSheet(target.project, target.asset, target.version, target.item).then(
      got => live && setSheet(got),
      e => live && setError(errorMessage(e))
    );
    return () => {
      live = false;
    };
  }, [target]);
  return { sheet, error };
}

/**
 * The split sheet: the way, each provider's settings, what blocks a send,
 * the estimate, and the confirm question before credits go out.
 */
export function useSplitSheet() {
  const target = useSplitStore(s => s.split);
  const close = useSplitStore(s => s.closeSplit);
  const setView = useAppStore(s => s.setView);
  const { sheet, error } = useSheet(target);
  const [way, setWay] = useState<SplitWay>('tripo');
  const [tripo, setTripo] = useState<TripoSplit>(DEFAULT_TRIPO);
  const [meshy, setMeshy] = useState<MeshySplit>(DEFAULT_MESHY);
  const [after, setAfter] = useState<SplitAfter>('bake');
  const [texturePrompt, setTexturePrompt] = useState('');
  const [asking, setAsking] = useState(false);
  const [sending, setSending] = useState(false);

  useEffect(() => {
    setWay(target?.inBlender ? 'blender' : 'tripo');
    setTripo(DEFAULT_TRIPO);
    setMeshy(DEFAULT_MESHY);
    setAfter('bake');
    setTexturePrompt('');
    setAsking(false);
  }, [target]);

  const source = sheet?.source ?? null;
  const service = way === 'blender' ? null : way;
  const key = service ? sheet?.keys.find(k => k.service === service) : undefined;
  const blocker = !source
    ? null
    : way === 'blender'
      ? target?.inBlender
        ? null
        : 'Accept the model first: the free split is a fix-up of the asset.'
      : source.busy
        ? `A split of ${source.assetName} is on its way already.`
        : !key?.saved
          ? `Add a ${PROVIDER_LABEL[way]} key in Settings first.`
          : way === 'meshy'
            ? (source.meshyProblem ?? meshyProblem(meshy))
            : null;
  const credits = sheet ? splitCredits(way, tripo, sheet.prices) : null;
  const summary =
    way === 'blender'
      ? 'Split into parts fix-up, by loose pieces, run headless on a copy'
      : way === 'tripo'
        ? `Tripo segment · ${tripoLine(tripo)}`
        : `${meshyLine(meshy)}${afterLine(after, source?.textures ?? 0).replace(', then', ' · then')}`;
  const question =
    sheet && service
      ? confirmLine(service, tripo, meshy, sheet.prices, sheet.source.textures, after)
      : '';

  const send = async () => {
    if (!target || !service) return;
    setSending(true);
    try {
      const job = await commands.splitSend({
        project: target.project,
        asset: target.asset,
        version: target.version,
        item: target.item,
        provider: service,
        tripo: service === 'tripo' ? tripo : null,
        meshy: service === 'meshy' ? meshy : null,
        after: service === 'meshy' ? after : 'bake',
        texturePrompt: service === 'meshy' && after === 'retexture' ? texturePrompt : null,
      });
      useProvidersStore.setState(s => ({
        jobs: job.project === s.project ? upsert(s.jobs, job) : s.jobs,
      }));
      notify(
        'done',
        `${job.assetName} is splitting on ${PROVIDER_LABEL[service]}`,
        'It lands in the Inbox as the next version; the one it came from stays.'
      );
      close();
    } catch (e) {
      notify('failed', 'Could not send the split', errorMessage(e));
      setAsking(false);
    } finally {
      setSending(false);
    }
  };

  const runInBlender = async () => {
    close();
    await target?.inBlender?.();
  };

  const openSettings = () => {
    close();
    setView('settings');
  };

  return {
    target,
    sheet,
    error,
    way,
    setWay,
    tripo: {
      value: tripo,
      set: (patch: Partial<TripoSplit>) => setTripo(t => ({ ...t, ...patch })),
    },
    meshy: {
      value: meshy,
      set: (patch: Partial<MeshySplit>) => setMeshy(m => ({ ...m, ...patch })),
    },
    /** What follows for the texture Meshy drops. */
    texture: { after, setAfter, prompt: texturePrompt, setPrompt: setTexturePrompt },
    key,
    blocker,
    credits,
    summary,
    question,
    /** The question before credits go out. */
    confirm: { open: asking, ask: () => setAsking(true), keep: () => setAsking(false) },
    sending,
    send,
    runInBlender,
    openSettings,
    close,
  };
}

export type SplitSheetModel = ReturnType<typeof useSplitSheet>;
