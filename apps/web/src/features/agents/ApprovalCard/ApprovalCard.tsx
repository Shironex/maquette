import { Fragment } from 'react';
import { Button, Icon, cn } from '@maquette/ui';
import type { Approval } from '@/ipc/client';
import { formatAgo } from '@/lib/format';
import { COMMAND_SOURCE, approvalTitle } from '@/state/agents';
import { formatCredits } from '@/state/credits';
import { useApprovalCard } from './ApprovalCard.hooks';

export interface ApprovalCardProps {
  approval: Approval;
  floating?: boolean;
}

export default function ApprovalCard({ approval, floating }: ApprovalCardProps) {
  const { decide, deciding, site, now, cost, notes, pose } = useApprovalCard(approval);
  const titleId = `${approval.id}-title`;
  const weights = approval.kind === 'reweight';
  const chains = approval.kind === 'springs';
  const doctor = approval.kind === 'rig-doctor';
  const rest = approval.kind === 'rest-pose' || weights || chains || doctor;
  const send = approval.kind === 'send-to-3d';
  const round = approval.kind === 'image-round';
  const preview = approval.kind === 'page-preview' ? approval.preview : null;
  const shipTo = approval.kind === 'ship' ? approval.detail : null;
  const detailLine = (send || round || rest) && approval.detail ? approval.detail : null;
  const items = approval.items.map(item => (
    <li
      key={item.job}
      className="flex items-baseline gap-2 border-t border-line px-2.5 py-1 first:border-t-0"
    >
      <span className="flex-none font-medium text-fg">{item.assetName}</span>
      <span className="min-w-0 flex-1 truncate text-fg-mute" title={item.detail}>
        {item.detail}
      </span>
      {item.estimate !== null && item.estimate !== undefined ? (
        <span className="flex-none text-fg-dim tabular-nums">{formatCredits(item.estimate)}</span>
      ) : null}
    </li>
  ));
  const noteItems = notes.map(note => <li key={note}>{note}</li>);
  const hooks = preview?.hooks.map(hook => (
    <Fragment key={hook}>
      <dt className="text-fg-mute">Also runs</dt>
      <dd className="m-0 font-mono break-all text-fg">{hook}</dd>
    </Fragment>
  ));

  return (
    <section
      role="alertdialog"
      aria-labelledby={titleId}
      className={cn(
        'pointer-events-auto flex flex-col gap-3 rounded-card border border-amber/40 bg-panel p-3.5',
        floating && 'shadow-window'
      )}
    >
      <div className="flex items-start gap-3">
        <span className="grid size-8 flex-none place-items-center rounded-[9px] bg-amber-soft text-amber">
          <Icon
            name={
              rest
                ? 'person-arms-spread'
                : preview
                  ? 'terminal-window'
                  : round
                    ? 'image'
                    : send
                      ? 'cube'
                      : 'rocket-launch'
            }
            size={17}
          />
        </span>
        <div className="min-w-0 flex-1">
          <h2 id={titleId} className="text-emphasis leading-5 font-semibold">
            {approvalTitle(approval, site?.name)}
          </h2>
          {preview ? (
            <p className="mt-0.5 text-meta text-fg-mute">
              An agent set this site repo over MCP. Its dev command runs on this computer as you, so
              read it first. Nothing runs until you approve. A changed folder, command, package.json
              or listed manager config asks again. A yes trusts the repo, including nested packages
              and the files scripts load.
            </p>
          ) : doctor ? (
            <p className="mt-0.5 text-meta text-fg-mute">
              An agent started this rig doctor run over MCP. Nothing is added until you look at the
              sheets and add it yourself.
            </p>
          ) : chains ? (
            <p className="mt-0.5 text-meta text-fg-mute">
              An agent built this avatar over MCP and added a chain of joints to each confirmed
              spring. Nothing is written with them until you approve the sheets in the Avatar view.
            </p>
          ) : weights ? (
            <p className="mt-0.5 text-meta text-fg-mute">
              An agent built this avatar over MCP and moved cloth and hair off its arms, so lifting
              the arms does not drag them. Nothing is used until you approve the sheets in the
              Avatar view.
            </p>
          ) : rest ? (
            <p className="mt-0.5 text-meta text-fg-mute">
              An agent built this avatar over MCP and its rest pose changed. Nothing is written
              until you approve the sheets in the Avatar view.
            </p>
          ) : round ? (
            <p className="mt-0.5 text-meta text-fg-mute">
              An agent asked over MCP. This spends {cost}; nothing is sent to OpenAI until you
              approve.
            </p>
          ) : send ? (
            <p className="mt-0.5 text-meta text-fg-mute">
              An agent asked over MCP. This spends {cost}; nothing is sent until you approve.
            </p>
          ) : site?.target === 'game' ? (
            <p className="mt-0.5 text-meta text-fg-mute">
              Everything before {approval.step} is done. {approval.step} writes into the game repo
              and only replaces files it wrote. Review the change in the Pipeline view.
            </p>
          ) : (
            <p className="mt-0.5 text-meta text-fg-mute">
              An agent started this run over MCP. Everything before {approval.step} is done;{' '}
              {approval.step} writes to {site?.siteRepo ? 'the site repo' : 'the shipped folder'}.
            </p>
          )}
        </div>
        <span className="flex-none text-meta text-fg-mute tabular-nums">
          {formatAgo(approval.requestedAtMs ?? now, now)}
        </span>
      </div>
      {shipTo ? (
        <p className="ml-11 rounded-[7px] border border-line bg-raised px-2.5 py-1.5 font-mono text-meta break-all text-fg-dim">
          {shipTo}
        </p>
      ) : null}
      {preview ? (
        <dl className="m-0 ml-11 grid grid-cols-[auto_minmax(0,1fr)] gap-x-2.5 gap-y-1 rounded-[7px] border border-line bg-raised px-2.5 py-1.5 text-meta">
          <dt className="text-fg-mute">Folder</dt>
          <dd className="m-0 font-mono break-all text-fg-dim">{preview.repo}</dd>
          <dt className="text-fg-mute">Command</dt>
          <dd className="m-0 font-mono break-all text-fg">{preview.command}</dd>
          {preview.scriptRuns ? (
            <>
              <dt className="text-fg-mute">Script runs</dt>
              <dd className="m-0 font-mono break-all text-fg">{preview.scriptRuns}</dd>
            </>
          ) : null}
          {hooks}
          {preview.scriptShell ? (
            <>
              <dt className="text-fg-mute">Script shell</dt>
              <dd className="m-0 font-mono break-all text-amber">{preview.scriptShell}</dd>
            </>
          ) : null}
          {preview.scriptRuns ? null : (
            <>
              <dt className="text-fg-mute">Runs</dt>
              <dd className="m-0 text-amber">Maquette cannot tell what this command runs.</dd>
            </>
          )}
          {preview.managerConfig.length > 0 ? (
            <>
              <dt className="text-fg-mute">Shell or manager</dt>
              <dd className="m-0 text-fg-dim">
                Can be set in <span className="font-mono">{preview.managerConfig.join(', ')}</span>
              </dd>
            </>
          ) : null}
          <dt className="text-fg-mute">From</dt>
          <dd className="m-0 text-fg-dim">{COMMAND_SOURCE[preview.source]}</dd>
        </dl>
      ) : null}
      {send && approval.items.length > 0 ? (
        <div className="ml-11 rounded-[7px] border border-line bg-raised text-meta">
          <ul className="m-0 flex max-h-[180px] list-none flex-col overflow-y-auto p-0">{items}</ul>
          {approval.detail ? (
            <p className="border-t border-line px-2.5 py-1.5 font-medium text-fg-dim">
              {approval.detail}
            </p>
          ) : null}
        </div>
      ) : detailLine ? (
        <p className="ml-11 rounded-[7px] border border-line bg-raised px-2.5 py-1.5 text-meta text-fg-dim">
          {detailLine}
        </p>
      ) : null}
      {rest && pose.sheets ? (
        <div className="ml-11 grid grid-cols-2 gap-2">
          {pose.sheets.before ? (
            <img
              src={pose.sheets.before}
              alt="The before sheet"
              className="w-full rounded-[7px] border border-line"
            />
          ) : null}
          {pose.sheets.after ? (
            <img
              src={pose.sheets.after}
              alt="The after sheet"
              className="w-full rounded-[7px] border border-line"
            />
          ) : null}
        </div>
      ) : null}
      {notes.length > 0 ? (
        <ul className="m-0 ml-11 flex list-none flex-col gap-1 p-0 text-meta text-amber">
          {noteItems}
        </ul>
      ) : null}
      {rest ? (
        <div className="flex justify-end gap-2">
          <Button
            variant="secondary"
            disabled={pose.denying}
            loading={pose.denying}
            tooltip={
              doctor
                ? 'Adds nothing; the draft is kept and the agent can read that it waits'
                : chains
                  ? 'Writes the VRM without springs; the agent is told it was turned down'
                  : weights
                    ? 'Lifts the arms from the model as it is; the agent is told it was turned down'
                    : 'Writes nothing; the agent is told the pose was turned down'
            }
            onClick={() => void pose.deny()}
          >
            Deny
          </Button>
          <Button
            variant="primary"
            icon="magnifying-glass"
            tooltip={
              doctor
                ? 'Opens the rig doctor, to add the draft or discard it there'
                : 'Opens the before and after sheets large, to approve or deny there'
            }
            onClick={pose.open}
          >
            Open the sheets
          </Button>
        </div>
      ) : (
        <div className="flex justify-end gap-2">
          <Button
            variant="secondary"
            disabled={deciding}
            tooltip={
              preview
                ? 'Runs nothing; the server stays stopped and the agent is told you refused'
                : round
                  ? 'Makes nothing and spends nothing; the jobs end cancelled'
                  : send
                    ? 'Sends nothing and spends no credits'
                    : 'Ships nothing'
            }
            onClick={() => void decide(approval.id, false)}
          >
            Deny
          </Button>
          <Button
            variant="primary"
            icon="check"
            loading={deciding}
            disabled={deciding}
            tooltip={
              preview
                ? 'Runs this command in this folder and remembers the yes for this folder and command'
                : round
                  ? 'Queues the images on OpenAI Images, which spends from the OpenAI key'
                  : send
                    ? 'Sends it to the provider, which spends credits'
                    : 'Writes the files to the game repo'
            }
            onClick={() => void decide(approval.id, true)}
          >
            {preview
              ? 'Approve and run'
              : round
                ? `Approve and make ${approval.conceptJobs?.length ?? ''}`.trim()
                : send
                  ? approval.items.length > 1
                    ? `Approve and send ${approval.items.length}`
                    : 'Approve and send'
                  : 'Approve and ship'}
          </Button>
        </div>
      )}
    </section>
  );
}
