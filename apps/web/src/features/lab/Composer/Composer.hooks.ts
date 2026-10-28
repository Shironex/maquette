import { useEffect } from 'react';
import type { LabView } from '@/ipc/client';
import { useNow } from '@/lib/useNow';
import { roundWarning } from '@/state/codex-usage';
import { backendFor, roundCost, roundLine } from '@/state/image-api';
import { useCodexUsageStore } from '@/state/use-codex-usage-store';
import { useImageApiStore } from '@/state/use-image-api-store';
import { useAppStore } from '@/state/use-app-store';
import { directionId } from '@/state/lab';
import { useLabStore } from '@/state/use-lab-store';

/**
 * A round on OpenAI's Image API (Settings > Image generation): what it
 * likely costs against the month's cap. Null when concepts go to Codex.
 */
function useApiRound(view: LabView, count: number) {
  const images = useAppStore(s => s.workspace?.config.lab.images);
  const on = backendFor(images, 'concepts') === 'openai-images';
  const status = useImageApiStore(s => s.status);
  const load = useImageApiStore(s => s.load);
  const made = view.jobs.filter(j => j.backend === 'openai-images' && j.state === 'done').length;
  useEffect(() => {
    if (on) void load();
  }, [on, load, made]);
  if (!on) return null;
  return status
    ? { line: roundLine(status, count), over: roundCost(status, count).over }
    : { line: 'Runs on OpenAI Images.', over: false };
}

/**
 * The draft brief, whether it adds to an earlier direction, whether Codex
 * (or the Image API) can run it, and a warning when the round may meet
 * the usage limit or pass the Image API's cap.
 */
export function useComposer(view: LabView) {
  const draft = useLabStore(s => s.draft);
  const setDraft = useLabStore(s => s.setDraft);
  const generate = useLabStore(s => s.generate);
  const busy = useLabStore(s => s.busy);
  const newDraft = useLabStore(s => s.newDraft);
  const codex = useAppStore(s => s.tools.reports.find(r => r.id === 'codex'));
  const existing = view.directions.find(d => d.id === directionId(draft.name));
  const ready = draft.name.trim() !== '' && draft.prompt.trim() !== '';
  const api = useApiRound(view, draft.count);
  const codexReady = Boolean(api) || !codex || codex.status === 'found';
  const usage = useCodexUsageStore(s => s.usage);
  const now = useNow(30_000);
  const usageWarning = api ? null : roundWarning(usage, draft.count, now);
  return {
    draft,
    setDraft,
    generate,
    busy,
    newDraft,
    codex,
    existing,
    ready,
    codexReady,
    usageWarning,
    api,
  };
}
