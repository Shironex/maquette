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
import { Button, cn, Icon, Kbd, TopBar, type IconName } from '@maquette/ui';
import type { NodeKind } from '@/ipc/client';
import { LinkDialog } from '../LinkDialog';
import { PlanNodeCard } from '../PlanNodeCard';
import { usePlanCanvas } from './PlanCanvas.hooks';

const NODE_TYPES: NodeTypes = { note: PlanNodeCard, image: PlanNodeCard, link: PlanNodeCard };

const TOOLS: { kind: NodeKind; label: string; icon: IconName; key: string; tip: string }[] = [
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
];

const PRESET_NAME = { brainstorm: 'Brainstorm', story: 'Story', mechanics: 'Game mechanics' };

/**
 * An open canvas: notes, pictures and links to move, link and delete, a
 * floating bar to add them, zoom and a minimap. Every change saves into the
 * canvas file a moment later.
 */
export default function PlanCanvas() {
  const v = usePlanCanvas();
  if (!v.open) return null;
  const { plan, file } = v.open;
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
          <span
            className={cn(
              'truncate font-mono text-[11.5px]',
              v.saveFailed ? 'text-danger' : 'text-fg-mute'
            )}
            role="status"
          >
            {file} · {v.saveLine}
          </span>
        }
      />
      <main
        ref={v.frame}
        aria-label={`${plan.name} canvas`}
        className="plan-canvas relative min-h-0 flex-1"
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
          <button type="button" aria-label="Zoom in" onClick={v.zoomIn}>
            <Icon name="plus" size={15} />
          </button>
          <button type="button" aria-label="Zoom out" onClick={v.zoomOut}>
            <Icon name="minus" size={15} />
          </button>
          <button type="button" aria-label="Fit the canvas in view" onClick={v.fit}>
            <Icon name="frame-corners" size={15} />
          </button>
          <span className="pct">{v.zoomPercent}</span>
        </div>
        {v.nodes.length === 0 ? (
          <section aria-label="This canvas is empty" className="plan-empty">
            <span className="grid size-14 place-items-center rounded-[16px] border border-line bg-raised text-fg-dim">
              <Icon name="graph" size={26} />
            </span>
            <h2 className="text-panel-title">Add notes, pictures and links</h2>
            <p className="max-w-[380px] text-fg-dim">
              This canvas is empty. It is saved as {file}, and kept saved as you go.
            </p>
            <ul className="flex flex-col gap-1.5 text-left text-fg-dim">
              <li>Double click anywhere to write a note</li>
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
      </main>
      {v.linking ? <LinkDialog onAdd={v.addLink} onClose={v.closeLink} /> : null}
    </>
  );
}
