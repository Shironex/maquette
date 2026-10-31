/** "Loading 66 models…", and how far it got once any are done. */
export function loadingLabel(models: number, done: number): string {
  return `Loading ${models} models…${done > 0 ? ` ${done} done` : ''}`;
}
