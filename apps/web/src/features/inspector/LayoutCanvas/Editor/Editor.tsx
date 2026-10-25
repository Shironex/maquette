import { useEditor } from './Editor.hooks';
import type { EditorProps } from './Editor.types';

/** Everything inside the layout canvas; it draws through three directly, so it renders nothing. */
export default function Editor(props: EditorProps) {
  useEditor(props);
  return null;
}
