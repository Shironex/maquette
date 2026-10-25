import type { ModelOption } from '@/ipc/client';

/** Select options for a catalog list, keeping an unknown saved id visible. */
export function modelOptions(models: readonly ModelOption[], current: string) {
  const known = models.some(m => m.id === current);
  return [
    ...(known || !current ? [] : [{ value: current, label: current }]),
    ...models.map(m => ({ value: m.id, label: m.label })),
  ];
}
