import { useEffect, useState } from 'react';
import { folderProblem } from '@/state/game';

/** The folder being typed, its problem, and the commit that saves it when it changed. */
export function useFolderField(folder: string | null, onSave: (folder: string | null) => void) {
  const saved = folder ?? '';
  const [text, setText] = useState(saved);
  useEffect(() => setText(saved), [saved]);
  const problem = folderProblem(text);
  const commit = () => {
    const next = text.trim().replace(/\/+$/, '');
    if (problem || next === saved) return;
    onSave(next || null);
  };
  return { text, setText, problem, commit };
}
