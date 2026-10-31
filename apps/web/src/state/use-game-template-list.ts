import { useEffect, useState } from 'react';
import { commands, type GameTemplate } from '@/ipc/client';

/** The built-in game templates; empty until they load, or if they cannot. */
export function useGameTemplateList(): GameTemplate[] {
  const [templates, setTemplates] = useState<GameTemplate[]>([]);
  useEffect(() => {
    let live = true;
    commands
      .gameTemplateList()
      .then(list => {
        if (live) setTemplates(list);
      })
      .catch(() => undefined);
    return () => {
      live = false;
    };
  }, []);
  return templates;
}
