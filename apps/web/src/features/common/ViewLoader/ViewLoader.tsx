import { Loader, type WaitProps } from '@maquette/ui';

/** The content area while a view's code or data is on its way. The shell stays usable around it. */
export default function ViewLoader({ label }: { label?: string }) {
  return (
    <main className="grid flex-1 place-items-center" aria-busy="true">
      <Loader label={label} />
    </main>
  );
}

export type StageLoaderProps = WaitProps;

/** Over a 3D stage while three.js or the model is on its way. Clicks pass through to the stage. */
export function StageLoader({ label = 'Loading the model...', count, estimate }: StageLoaderProps) {
  return (
    <div className="pointer-events-none absolute inset-0 grid place-items-center">
      <Loader label={label} count={count} estimate={estimate} />
    </div>
  );
}
