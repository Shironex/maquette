import { Button } from '@maquette/ui';
import { useSyncHelp } from './SyncHelp.hooks';

/**
 * How to share the workspace between two computers with Syncthing: what to
 * leave out, how to keep old versions, and the one rule that avoids
 * conflict copies. Folded until asked for.
 */
export default function SyncHelp({ cache }: { cache: string }) {
  const { text, copied, copy } = useSyncHelp(cache);
  return (
    <details className="group rounded-[8px] border border-line bg-app px-3 py-2.5">
      <summary className="cursor-pointer select-none text-fg">Sync with another computer</summary>
      <div className="mt-2 flex flex-col gap-2 text-meta text-fg-dim">
        <p>
          The workspace can live in a Syncthing folder shared by your Mac and your Windows PC. Each
          computer keeps its own settings, API keys and repo paths, so Maquette asks once where a
          project&apos;s repo is on each one.
        </p>
        <ul className="m-0 flex list-disc flex-col gap-1 pl-4">
          <li>
            Use Maquette on one computer at a time. When both change the same file at once,
            Syncthing keeps a conflict copy and Maquette warns about it.
          </li>
          <li>
            Turn on staggered file versioning for the folder, so a bad sync can be undone from
            .stversions.
          </li>
          <li>
            Put this in the folder&apos;s .stignore. The cache is rebuilt on each computer. The lock
            file stays synced on purpose: it is how the other computer knows this one has it open.
          </li>
        </ul>
        <pre className="m-0 overflow-x-auto rounded-[6px] border border-line bg-panel px-2.5 py-2 font-mono text-[12px] leading-[17px] text-fg">
          {text}
        </pre>
        <div>
          <Button size="sm" icon="copy" onClick={copy}>
            {copied ? 'Copied' : 'Copy .stignore'}
          </Button>
        </div>
      </div>
    </details>
  );
}
