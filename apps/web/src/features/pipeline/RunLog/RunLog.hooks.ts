import { useMemo, useState } from 'react';
import type { PipelineView } from '@/ipc/client';
import { logRows } from '@/state/pipeline';
import { readFlag, writeFlag } from '@/lib/storage';
import { styledLine } from './styledLine';

const SCROLL_KEY = 'maquette.pipeline.manualScroll';
const NOISE_KEY = 'maquette.pipeline.hideNoise';

/** The open tab and the console lines, with the noise and scroll switches kept per machine. */
export function useRunLog(view: PipelineView, step: string) {
  const [tab, setTab] = useState('logs');
  const [auto, setAuto] = useState(() => !readFlag(SCROLL_KEY));
  const [hideNoise, setHideNoise] = useState(() => readFlag(NOISE_KEY));
  const rows = useMemo(() => logRows(view.log, step), [view.log, step]);
  const lines = useMemo(() => {
    const styled = rows.map(styledLine);
    return hideNoise ? styled.filter(l => !l.noise) : styled;
  }, [rows, hideNoise]);
  const copy = () => {
    const text = rows.map(r => `${r.t} ${r.text}`).join('\n');
    void navigator.clipboard?.writeText(text).catch(() => undefined);
  };
  const toggleNoise = (on: boolean) => {
    setHideNoise(on);
    writeFlag(NOISE_KEY, on);
  };
  const toggleAuto = (on: boolean) => {
    setAuto(on);
    writeFlag(SCROLL_KEY, !on);
  };
  return { tab, setTab, auto, toggleAuto, hideNoise, toggleNoise, lines, copy };
}
