import { Tooltip, cn } from '@maquette/ui';
import { useResizeDrag } from './PanelResizer.hooks';
import { SIDE_WIDTH } from './sideWidth';

export interface PanelResizerProps {
  /** The side panel's width now. */
  width: number;
  onChange: (width: number) => void;
  /** A double click puts the width back. */
  onReset: () => void;
  /** What the panel it sizes is called, for its accessible name. */
  label: string;
}

/** The divider between the log and the side panel, dragged or moved with the arrow keys. */
export default function PanelResizer({ width, onChange, onReset, label }: PanelResizerProps) {
  const drag = useResizeDrag(width, onChange);
  return (
    <Tooltip
      label={`Resize the ${label.toLowerCase()}`}
      detail="Drag, or use the left and right arrow keys. Double click puts it back."
      disabled={drag.dragging}
    >
      <div
        role="separator"
        aria-orientation="vertical"
        aria-label={`Resize the ${label.toLowerCase()}`}
        aria-valuenow={width}
        aria-valuemin={SIDE_WIDTH.min}
        aria-valuemax={SIDE_WIDTH.max}
        tabIndex={0}
        onPointerDown={drag.onPointerDown}
        onPointerMove={drag.onPointerMove}
        onPointerUp={drag.onPointerUp}
        onPointerCancel={drag.onPointerUp}
        onKeyDown={drag.onKeyDown}
        onDoubleClick={onReset}
        className="group flex w-3.5 flex-none cursor-col-resize touch-none justify-center rounded-control focus-visible:outline-offset-0"
      >
        <span
          aria-hidden="true"
          className={cn(
            'my-auto h-10 w-[3px] rounded-full bg-line transition-colors duration-120 group-hover:bg-line-strong group-focus-visible:bg-fg-mute',
            drag.dragging && 'bg-fg-mute'
          )}
        />
      </div>
    </Tooltip>
  );
}
