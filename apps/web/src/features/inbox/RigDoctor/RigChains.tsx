import { Button, Icon, Select, cn } from '@maquette/ui';
import type { RigChain, RigRecord } from '@/ipc/client';
import { canEditChains, isLeg } from '@/state/rig';
import { useChainEditor } from './RigChains.hooks';

const READ_TONE: Record<string, string> = {
  chain: 'text-fg-dim',
  stray: 'text-clay-text',
  swapped: 'text-amber',
  junk: 'text-clay-text',
};

const READ_WORD: Record<string, string> = {
  chain: 'Chain',
  stray: 'Stray weight',
  swapped: 'Sides swapped',
  junk: 'Junk',
};

const READ_OPTIONS = [
  { value: 'chain', label: 'Chain' },
  { value: 'junk', label: 'Junk, weights removed' },
];
const SIDE_OPTIONS = [
  { value: 'left', label: 'Left' },
  { value: 'right', label: 'Right' },
];

function chainWord(c: RigChain): string {
  return c.note ?? READ_WORD[c.readAs] ?? c.readAs;
}

/** Whether a person looked at the chains yet. */
function Looked({ yes }: { yes: boolean }) {
  return yes ? (
    <span className="inline-flex items-center gap-1 rounded-full bg-sage-soft px-2 py-0.5 text-meta text-sage">
      <Icon name="check" size={11} />
      Checked by you
    </span>
  ) : (
    <span className="rounded-full bg-amber-soft px-2 py-0.5 text-meta text-amber">
      Not checked yet
    </span>
  );
}

/** The chains read from the bone tree: a first pass, until a person says it is right. */
export function RigChains({ record }: { record: RigRecord }) {
  const e = useChainEditor(record);
  const chains = record.creature?.chains ?? [];
  const ignored = record.creature?.ignored ?? [];
  const open = canEditChains(record);
  const editsSide = (c: RigChain) => e.editing && isLeg(c) && e.readAsOf(c) !== 'junk';
  const rows = chains.map((c, i) => (
    <tr key={`${c.chain}-${i}`} className="border-t border-line">
      <td className="py-1.5 font-mono text-fg">{c.chain}</td>
      <td className="py-1.5 font-mono text-fg-dim">{c.bones}</td>
      <td className="py-1.5 font-mono text-fg-dim">
        {editsSide(c) ? (
          <Select
            size="sm"
            aria-label={`Side of ${c.chain}`}
            options={SIDE_OPTIONS}
            value={e.sideOf(c)}
            onChange={side => e.setSide(c, side)}
          />
        ) : (
          c.side
        )}
      </td>
      <td className={cn('py-1.5', READ_TONE[c.readAs] ?? 'text-fg-dim')}>
        {e.editing && c.key ? (
          <Select
            size="sm"
            aria-label={`Read ${c.chain} as`}
            options={READ_OPTIONS}
            value={e.readAsOf(c)}
            onChange={word => e.setRead(c, word)}
          />
        ) : (
          chainWord(c)
        )}
      </td>
    </tr>
  ));
  return (
    <>
      <div className="flex flex-none items-center justify-between gap-2">
        <h3 className="m-0 text-caption tracking-[0.06em] text-fg-mute uppercase">
          Chains, filled from the bone tree
        </h3>
        {chains.length > 0 ? (
          <div className="flex items-center gap-2">
            {e.editing ? null : <Looked yes={Boolean(record.chainsConfirmed)} />}
            {e.editing ? (
              <>
                <Button size="sm" variant="ghost" disabled={e.busy} onClick={e.cancel}>
                  Cancel
                </Button>
                <Button size="sm" variant="primary" disabled={e.busy} onClick={() => void e.save()}>
                  {e.changed ? 'Save and start over' : 'Chains are right'}
                </Button>
              </>
            ) : (
              <>
                <Button
                  size="sm"
                  variant="secondary"
                  icon="note-pencil"
                  disabled={!open}
                  disabledReason="Not while a run goes on"
                  onClick={e.start}
                >
                  Edit
                </Button>
                {record.chainsConfirmed ? null : (
                  <Button
                    size="sm"
                    variant="secondary"
                    icon="check"
                    disabled={!open || e.busy}
                    disabledReason="Not while a run goes on"
                    onClick={() => void e.save()}
                  >
                    Chains are right
                  </Button>
                )}
              </>
            )}
          </div>
        ) : null}
      </div>
      {chains.length > 0 ? (
        <table className="w-full flex-none border-collapse text-meta">
          <thead>
            <tr className="text-left text-caption tracking-[0.06em] text-fg-mute uppercase">
              <th className="py-1 font-medium">Chain</th>
              <th className="py-1 font-medium">Bones</th>
              <th className="py-1 font-medium">Side</th>
              <th className="py-1 font-medium">Read as</th>
            </tr>
          </thead>
          <tbody>{rows}</tbody>
        </table>
      ) : (
        <p className="m-0 flex-none text-meta text-fg-mute">
          The pose test reads them from the bone tree first. You check them here.
        </p>
      )}
      {ignored.length > 0 ? (
        <p className="m-0 flex-none text-meta text-fg-mute">
          {ignored.join(', ')} carry no weight: in no chain, and left in the file.
        </p>
      ) : null}
      {e.editing ? (
        <p className="m-0 flex-none text-meta text-fg-mute">
          A change starts again from the pose test, and any draft goes. v
          {record.source.version ?? 1} stays. Check against the weight map: the read goes by where
          bones sit, not their names.
        </p>
      ) : null}
    </>
  );
}
