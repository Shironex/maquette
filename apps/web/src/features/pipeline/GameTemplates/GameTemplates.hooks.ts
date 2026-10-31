import { useEffect, useState } from 'react';
import type { GameTemplateId, GameTemplatePreview } from '@/ipc/client';
import { useGameTemplateList } from '@/state/use-game-template-list';
import { useGameStore } from '@/state/use-game-store';

/** The sheet's list, the picked template and what it would change here. */
export function useGameTemplates(project: string, onClose: () => void) {
  const previewTemplate = useGameStore(s => s.previewTemplate);
  const applyTemplate = useGameStore(s => s.applyTemplate);
  const saving = useGameStore(s => s.saving);
  const templates = useGameTemplateList();
  const [picked, setPicked] = useState<GameTemplateId | null>(null);
  const [preview, setPreview] = useState<GameTemplatePreview | null>(null);
  const [reading, setReading] = useState(false);

  // A quick second pick must not show the first one's answer.
  useEffect(() => {
    setPreview(null);
    setReading(Boolean(picked));
    if (!picked) return;
    let live = true;
    void previewTemplate(project, picked).then(found => {
      if (!live) return;
      setPreview(found);
      setReading(false);
    });
    return () => {
      live = false;
    };
  }, [project, picked, previewTemplate]);

  const apply = async () => {
    if (picked && (await applyTemplate(project, picked))) onClose();
  };
  return { templates, picked, pick: setPicked, preview, reading, apply, saving };
}

/** Whether the sheet is open. */
export function useTemplatesOpen() {
  const [open, setOpen] = useState(false);
  return { open, setOpen };
}
