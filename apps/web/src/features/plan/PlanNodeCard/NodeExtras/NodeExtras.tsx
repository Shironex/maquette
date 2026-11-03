import type { ReactNode } from 'react';
import { NodeToolbar, Position } from '@xyflow/react';
import { Button, cn, Icon } from '@maquette/ui';
import type { PlanNode } from '@/ipc/client';
import { useAsked, useNodeTools, useOverlapFlags } from './NodeExtras.hooks';

/**
 * A node's extras: the badge when an agent asked the person to remove it
 * (it wins over the added or edited badge, `fallback`), and over the
 * selected node the tool to start a canvas from it.
 */
export default function NodeExtras({
  node,
  selected,
  fallback,
}: {
  node: PlanNode;
  selected: boolean;
  fallback: ReactNode;
}) {
  const asked = useAsked(node.id);
  return (
    <>
      {asked ? (
        <span className={cn('plan-badge', 'is-ask')}>
          <Icon name="question" size={12} />
          asked to remove
          <span className="sr-only">: the person answers in Changes</span>
        </span>
      ) : (
        fallback
      )}
      <NodeTools node={node} selected={selected} />
    </>
  );
}

/** Under a system card: each card it overlaps and on what, until the person marks it fine. */
export function OverlapFlags({ id }: { id: string }) {
  const flags = useOverlapFlags(id).map(f => (
    <p key={f.key} className="plan-overlap-flag">
      <Icon name="warning" size={12} className="mt-px flex-none" />
      {f.text}
    </p>
  ));
  return flags;
}

/** Over the selected node: start a new canvas from it and the nodes linked to it. */
function NodeTools({ node, selected }: { node: PlanNode; selected: boolean }) {
  const v = useNodeTools(node);
  if (!v.canStart) return null;
  return (
    <NodeToolbar isVisible={selected} position={Position.Top} offset={14}>
      <div className="plan-node-tools nodrag">
        <Button
          size="sm"
          variant="ghost"
          icon="flow-arrow"
          tooltip="Makes a new canvas from this node and the nodes linked to it; this canvas stays as it is"
          onClick={v.start}
        >
          Start a canvas from this
        </Button>
      </div>
    </NodeToolbar>
  );
}
