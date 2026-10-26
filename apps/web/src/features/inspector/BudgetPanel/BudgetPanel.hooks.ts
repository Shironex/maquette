import { useEffect, useState } from 'react';
import type { Asset, BudgetCheck, FolderBudget } from '@/ipc/client';
import { checkAsset, fileOf, fixName, folderOf, gameOf } from '@/state/game';
import { useAppStore } from '@/state/use-app-store';
import { useGameStore } from '@/state/use-game-store';

export interface BudgetRow {
  asset: Asset;
  file: string;
  folder: string;
  check: BudgetCheck | null;
  fix: string | null;
}

/** 'all', 'over', or a folder name. */
export type BudgetFilter = string;

/** Every model against its folder's budget, the filter and the editable budget table. */
export function useBudgetPanel() {
  const project = useAppStore(s => s.workspace?.project ?? null);
  const game = gameOf(project);
  const saving = useGameStore(s => s.saving);
  const saveSettings = useGameStore(s => s.saveSettings);
  const addBudgetFixups = useGameStore(s => s.addBudgetFixups);
  const [filter, setFilter] = useState<BudgetFilter>('all');
  const [budgets, setBudgets] = useState<FolderBudget[]>(game?.budgets ?? []);
  const savedKey = JSON.stringify(game?.budgets ?? []);
  useEffect(() => {
    setBudgets(JSON.parse(savedKey) as FolderBudget[]);
  }, [project?.id, savedKey]);

  const rows: BudgetRow[] = game
    ? (project?.assets ?? []).map(asset => {
        const file = fileOf(game, asset);
        const check = checkAsset(game, asset);
        return { asset, file, folder: folderOf(file), check, fix: check ? fixName(check) : null };
      })
    : [];
  const over = rows.filter(r => r.fix);
  const shown = rows.filter(r =>
    filter === 'all' ? true : filter === 'over' ? Boolean(r.fix) : r.folder === filter
  );
  const dirty = JSON.stringify(budgets) !== savedKey;
  const edit = (folder: string, patch: Partial<FolderBudget>) =>
    setBudgets(rows => rows.map(b => (b.folder === folder ? { ...b, ...patch } : b)));
  const empty = (folder: string) => !rows.some(r => r.folder === folder);

  return {
    project,
    game,
    rows: shown,
    total: rows.length,
    over,
    filter,
    setFilter,
    folders: (game?.budgets ?? []).map(b => b.folder).filter(Boolean),
    budgets,
    edit,
    empty,
    dirty,
    saving,
    save: () => {
      if (project) void saveSettings(project.id, { budgets });
    },
    reset: () => setBudgets(JSON.parse(savedKey) as FolderBudget[]),
    addFixups: () => {
      if (project) void addBudgetFixups(project.id);
    },
  };
}
