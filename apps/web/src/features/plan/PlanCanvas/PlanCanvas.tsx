import '@xyflow/react/dist/base.css';
import './plan-canvas.css';
import {
  Background,
  BackgroundVariant,
  ConnectionMode,
  MiniMap,
  ReactFlow,
  type NodeTypes,
} from '@xyflow/react';
import { Button, cn, Icon, IconButton, Kbd, TopBar, type IconName } from '@maquette/ui';
import { ChoiceDialog } from '@/features/common/ChoiceDialog';
import { KEYS } from '@/lib/keys';
import { AgentToast } from '../AgentToast';
import { AssetPicker } from '../AssetPicker';
import { LinkDialog } from '../LinkDialog';
import { PlanChanges } from '../PlanChanges';
import { PlanNodeCard } from '../PlanNodeCard';
import { usePlanCanvas, type AddKind } from './PlanCanvas.hooks';

const NODE_TYPES: NodeTypes = {
  note: PlanNodeCard,
  image: PlanNodeCard,
  link: PlanNodeCard,
  asset: PlanNodeCard,
  direction: PlanNodeCard,
};

const TOOLS: { kind: AddKind; label: string; icon: IconName; key: string; tip: string }[] = [
  {
    kind: 'note',
    label: 'Note',
    icon: 'note-pencil',
    key: 'N',
    tip: 'Adds a note in the middle of the view',
  },
  {
    kind: 'image',
    label: 'Image',
    icon: 'image',
    key: 'I',
    tip: 'Adds a picture from the project or your disk',
  },
  { kind: 'link', label: 'Link', icon: 'link-simple', key: 'L', tip: 'Adds a web page' },
  {
    kind: 'asset',
    label: 'Asset',
    icon: 'cube',
    key: 'A',
    tip: 'Links an asset of the Board or a Lab direction',
  },
];

const PRESET_NAME = { brainstorm: 'Brainstorm', story: 'Story', mechanics: 'Game mechanics' };

/**
 * An open canvas: notes, pictures and links to move, link and delete, a
 * floating bar to add them, zoom and a minimap. Every change saves into the
 * canvas file a moment later. The top bar undoes and redoes the person's
 * edits and opens the Changes panel, where any write (an agent's too) can
 * be undone; an agent's write shows a toast and marks the nodes it touched.
 */
export default function PlanCanvas() {
  const v = usePlanCanvas();
  if (!v.open) return null;
  const { plan, file } = v.open;
  const h = v.history;
  const tools = TOOLS.map(t => (
    <Button
      key={t.kind}
      variant="ghost"
      size="sm"
      icon={t.icon}
      tooltip={t.tip}
      shortcut={t.key}
      onClick={() => v.add(t.kind)}
    >
      {t.label}
    </Button>
  ));
  const keys = TOOLS.map(t => (
    <span key={t.kind} className="inline-flex items-center gap-1.5">
      <Kbd keys={t.key} /> {t.label.toLowerCase()}
    </span>
  ));
  return (
    <>
      <TopBar
        lead={
          <nav aria-label="Canvas" className="flex min-w-0 items-center gap-2">
            <button
              type="button"
              aria-label="Plan, back to all canvases"
              onClick={v.back}
              className="flex items-center gap-1.5 text-[15px] font-semibold text-fg-dim hover:text-fg focus-visible:outline-2 focus-visible:outline-focus"
            >
              <Icon name="arrow-left" size={14} />
              Plan
            </button>
            <span className="text-fg-mute" aria-hidden="true">
              /
            </span>
            <h1 className="truncate text-[15px] font-semibold text-fg">{plan.name}</h1>
            <span className="inline-flex h-[22px] items-center rounded-[6px] bg-hover px-2 text-[11.5px] font-medium text-fg-dim">
              {PRESET_NAME[plan.preset]}
            </span>
          </nav>
        }
        right={
          <div className="flex min-w-0 items-center gap-2">
            <span
              className={cn(
                'min-w-0 truncate font-mono text-[11.5px]',
                v.saveFailed ? 'text-danger' : 'text-fg-mute'
              )}
              role="status"
            >
              <span className="max-[1279px]:hidden">{file} · </span>
              {v.saveLine}
            </span>
            {h.outside && !h.outside.asking ? (
              <Button
                size="sm"
                icon="warning"
                className="border-clay-line text-clay-text"
                tooltip="Nothing saves until you choose Keep mine or Use theirs"
                onClick={h.outside.ask}
              >
                Answer clash
              </Button>
            ) : null}
            {h.agent ? (
              <span className="plan-agent-chip">
                <Icon name="robot" size={14} />
                {h.agent.name}
                <span className="text-fg-dim">{h.agent.when}</span>
              </span>
            ) : null}
            <Button
              size="sm"
              icon="clock"
              aria-pressed={h.changes.open}
              aria-label={
                h.changes.unseen ? `Changes, ${h.changes.unseen} new from agents` : 'Changes'
              }
              tooltip="Every change to this canvas, yours and agents', to undo"
              className={cn(h.changes.open && 'border-clay-line text-clay-text')}
              onClick={h.changes.toggle}
            >
              Changes
              {h.changes.unseen ? (
                <span className="plan-count" aria-hidden="true">
                  {h.changes.unseen}
                </span>
              ) : null}
            </Button>
            <span className="h-5 w-px bg-line" aria-hidden="true" />
            <IconButton
              icon="arrow-u-up-left"
              size="sm"
              variant="ghost"
              label="Undo"
              shortcut={KEYS.undo}
              disabled={!h.canUndo}
              disabledReason="Nothing to undo yet"
              onClick={h.undo}
            />
            <IconButton
              icon="arrow-u-up-right"
              size="sm"
              variant="ghost"
              label="Redo"
              shortcut={KEYS.redo}
              disabled={!h.canRedo}
              disabledReason="Nothing to redo"
              onClick={h.redo}
            />
          </div>
        }
      />
      <div className="flex min-h-0 flex-1">
        <main
          ref={v.frame}
          aria-label={`${plan.name} canvas`}
          className="plan-canvas relative min-h-0 min-w-0 flex-1"
          onDoubleClick={v.onDoubleClick}
        >
          <ReactFlow
            nodes={v.nodes}
            edges={v.edges}
            nodeTypes={NODE_TYPES}
            onNodesChange={v.onNodesChange}
            onEdgesChange={v.onEdgesChange}
            onConnect={v.onConnect}
            connectionMode={ConnectionMode.Loose}
            colorMode="dark"
            fitView
            fitViewOptions={{ padding: 0.2, maxZoom: 1 }}
            minZoom={0.2}
            maxZoom={2}
            zoomOnDoubleClick={false}
            deleteKeyCode={['Backspace', 'Delete']}
            defaultEdgeOptions={{ interactionWidth: 16 }}
            proOptions={{ hideAttribution: true }}
          >
            <Background
              variant={BackgroundVariant.Dots}
              gap={22}
              size={1.5}
              color="var(--plan-dot)"
            />
            <MiniMap
              ariaLabel="Map of the canvas"
              pannable
              zoomable
              nodeColor="var(--line-strong)"
              maskColor="rgba(28, 27, 25, 0.6)"
              className="plan-mini"
            />
          </ReactFlow>
          <div role="toolbar" aria-label="Add to the canvas" className="plan-bar">
            {tools}
          </div>
          <div role="group" aria-label="Zoom" className="plan-zoom">
            <button type="button" aria-label="Zoom in" onClick={v.zoom.in}>
              <Icon name="plus" size={15} />
            </button>
            <button type="button" aria-label="Zoom out" onClick={v.zoom.out}>
              <Icon name="minus" size={15} />
            </button>
            <button type="button" aria-label="Fit the canvas in view" onClick={v.zoom.fit}>
              <Icon name="frame-corners" size={15} />
            </button>
            <span className="pct">{v.zoom.percent}</span>
          </div>
          {v.nodes.length === 0 ? (
            <section aria-label="This canvas is empty" className="plan-empty">
              <span className="grid size-14 place-items-center rounded-[16px] border border-line bg-raised text-fg-dim">
                <Icon name="graph" size={26} />
              </span>
              <h2 className="text-panel-title">Add notes, pictures, links and assets</h2>
              <p className="max-w-[380px] text-fg-dim">
                This canvas is empty. It is saved as {file}, and kept saved as you go.
              </p>
              <ul className="flex flex-col gap-1.5 text-left text-fg-dim">
                <li>Double click anywhere to write a note</li>
                <li>Bring in assets of the Board with Asset</li>
                <li>Drag from a node&apos;s edge to another node to link them</li>
                <li>Select a node or a link and press Delete to remove it</li>
              </ul>
              <p className="flex flex-wrap items-center justify-center gap-4 text-meta text-fg-mute">
                {keys}
                <span className="inline-flex items-center gap-1.5">
                  <Kbd keys="Drag" /> pan
                </span>
              </p>
            </section>
          ) : null}
          {h.changes.open ? null : <AgentToast />}
        </main>
        {h.changes.open ? <PlanChanges /> : null}
      </div>
      {v.linking ? <LinkDialog onAdd={v.addLink} onClose={v.closeLink} /> : null}
      {v.picker.open ? (
        <AssetPicker linked={v.picker.linked} onAdd={v.picker.add} onClose={v.picker.close} />
      ) : null}
      {v.leave.asking ? (
        <ChoiceDialog
          title="The canvas was not saved"
          onDismiss={v.leave.stay}
          choices={[
            { label: 'Leave without saving', variant: 'danger', onClick: v.leave.anyway },
            { label: 'Stay', onClick: v.leave.stay },
            { label: 'Try again', variant: 'primary', onClick: v.leave.retry },
          ]}
        >
          <p>
            {file} does not have your latest edits. {v.leave.error}
          </p>
          <p className="mt-2">
            Stay to keep editing, try saving again, or leave and drop the edits made since the last
            save.
          </p>
        </ChoiceDialog>
      ) : null}
      {h.outside?.asking && !v.leave.asking ? (
        <ChoiceDialog
          title="The canvas changed outside"
          onDismiss={h.outside.later}
          choices={[
            { label: 'Use theirs', onClick: h.outside.useTheirs },
            { label: 'Keep mine', variant: 'primary', onClick: h.outside.keepMine },
          ]}
        >
          <p>
            {file} was changed by an agent or another computer while you had unsaved edits.
            Everything that does not clash is already merged.
          </p>
          <p className="mt-2">
            {h.outside.count === 1 ? 'One node or link' : `${h.outside.count} nodes or links`}{' '}
            changed on both sides. Keep your edits there, or use the version on disk.
          </p>
        </ChoiceDialog>
      ) : null}
    </>
  );
}
