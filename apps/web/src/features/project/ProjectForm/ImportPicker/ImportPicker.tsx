import { PathField } from '@maquette/ui';
import { useImportPicker } from './ImportPicker.hooks';

export interface ImportPickerProps {
  folder: string;
  size: 'md' | 'lg';
  onChange: (folder: string) => void;
  onBrowse: () => void;
}

export default function ImportPicker({ folder, size, onChange, onBrowse }: ImportPickerProps) {
  const note = useImportPicker(folder);

  return (
    <PathField
      label="Folder to import"
      size={size}
      className="mt-4"
      placeholder="~/Documents/Projects/my-scene"
      value={folder}
      onChange={onChange}
      onBrowse={onBrowse}
      note={note}
    />
  );
}
