/**
 * Dev only. Astro injects the React refresh preamble for pages that have a
 * React island, but the tour is mounted from a plain script, so the TSX
 * modules it imports would throw "can't detect preamble". This installs the
 * same preamble @vitejs/plugin-react documents for backend integrations.
 * The caller guards it with import.meta.env.DEV, so production drops it.
 */
const REFRESH_RUNTIME = '/@react-refresh';

type RefreshWindow = Window & {
  $RefreshReg$?: () => void;
  $RefreshSig$?: () => (type: unknown) => unknown;
  __vite_plugin_react_preamble_installed__?: boolean;
};

export async function installReactPreamble(): Promise<void> {
  const w = window as RefreshWindow;
  if (w.__vite_plugin_react_preamble_installed__) return;
  const runtime = (await import(/* @vite-ignore */ REFRESH_RUNTIME)) as {
    default: { injectIntoGlobalHook: (target: Window) => void };
  };
  runtime.default.injectIntoGlobalHook(window);
  w.$RefreshReg$ = () => {};
  w.$RefreshSig$ = () => type => type;
  w.__vite_plugin_react_preamble_installed__ = true;
}
