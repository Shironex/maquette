import { NumberField } from '@maquette/ui';
import type { WorkspaceConfig } from '@/ipc/client';
import { formatCount } from '@/lib/format';
import { patchSection } from '@/state/settings-form';
import type { ConfigFieldsProps } from '../ConfigFieldsProps';
import { parseCount } from '../parseCount';
import { errorNote } from './errorNote';

export type BudgetFieldsProps = ConfigFieldsProps;

export default function BudgetFields({ config, onChange, errors = {} }: BudgetFieldsProps) {
  const set = (patch: Partial<WorkspaceConfig['budgets']>) =>
    onChange(patchSection(config, 'budgets', patch));
  return (
    <>
      <NumberField
        label="Scene target"
        unit="MB"
        value={config.budgets.sceneMb ?? 0}
        min={0.1}
        max={1000}
        format={n => n.toFixed(1)}
        onChange={sceneMb => set({ sceneMb })}
        note={errorNote(errors.sceneMb)}
      />
      <NumberField
        label="First view target"
        unit="MB"
        value={config.budgets.firstViewMb ?? 0}
        min={0.1}
        max={1000}
        format={n => n.toFixed(1)}
        onChange={firstViewMb => set({ firstViewMb })}
        note={errorNote(errors.firstViewMb)}
        hint={errors.firstViewMb ? undefined : 'What the first camera stop may load.'}
      />
      <NumberField
        label="Tris per model"
        unit="tris"
        value={config.budgets.trisPerModel}
        min={1}
        format={formatCount}
        parse={parseCount}
        onChange={trisPerModel => set({ trisPerModel })}
        note={errorNote(errors.trisPerModel)}
      />
    </>
  );
}
