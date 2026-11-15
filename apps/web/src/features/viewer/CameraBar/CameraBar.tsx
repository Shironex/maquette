import { IconButton, Kbd, cn } from '@maquette/ui';
import { FRAME_KEY } from '../frame-key';

export interface CameraBarProps {
  /** The camera moves with a model whose root travels. */
  follow: boolean;
  onFollow: (follow: boolean) => void;
  className?: string;
}

/** Over a 3D stage: following the model on or off, and the key that frames it again. */
export default function CameraBar({ follow, onFollow, className }: CameraBarProps) {
  return (
    <div
      role="toolbar"
      aria-label="Camera"
      className={cn(
        'absolute top-2 right-2 flex items-center gap-1.5 rounded-control bg-app/85 py-0.5 pr-2 pl-0.5 text-[11.5px] leading-none text-fg-dim',
        className
      )}
    >
      <IconButton
        icon="person-simple-walk"
        label={follow ? 'Stop following the model' : 'Follow the model'}
        hint="The camera moves with a model that walks away, and the floor's shadow stays under it"
        size="sm"
        aria-pressed={follow}
        onClick={() => onFollow(!follow)}
      />
      <span className="inline-flex items-center gap-1">
        <Kbd keys={FRAME_KEY} />
        frame
      </span>
    </div>
  );
}
