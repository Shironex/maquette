import type { DragEvent } from 'react';
import { Icon, type IconName } from '../../icons/Icon';
import { cn } from '../../lib/cn';
import { useDragOver } from './Dropzone.hooks';

export interface DropzoneProps {
  title?: string;
  hint?: string;
  icon?: IconName;
  active?: boolean;
  onFiles?: (files: FileList) => void;
  onActivate?: () => void;
  className?: string;
}

export function Dropzone({
  title = 'Drop images here',
  hint,
  icon = 'upload-simple',
  active,
  onFiles,
  onActivate,
  className,
}: DropzoneProps) {
  const [over, setOver] = useDragOver();
  const lit = active || over;
  return (
    <button
      type="button"
      onClick={onActivate}
      onDragOver={(e: DragEvent) => {
        e.preventDefault();
        setOver(true);
      }}
      onDragLeave={() => setOver(false)}
      onDrop={(e: DragEvent) => {
        e.preventDefault();
        setOver(false);
        if (onFiles) onFiles(e.dataTransfer.files);
      }}
      className={cn(
        'flex w-full cursor-pointer flex-col items-center justify-center gap-2 rounded-card border-[1.5px] border-dashed border-clay-line bg-clay-soft p-3 text-center text-fg transition-colors duration-120',
        lit && 'border-clay',
        className
      )}
    >
      <Icon name={icon} size={20} className="text-clay-text" />
      <span className="font-medium">{title}</span>
      {hint ? <span className="text-meta text-fg-mute">{hint}</span> : null}
    </button>
  );
}
