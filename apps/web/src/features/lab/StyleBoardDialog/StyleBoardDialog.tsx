/**
 * The style board: a few reference images, one style line and a palette
 * that every new Lab direction and object ref carries after its own
 * prompt. Opens from the composer and the Make object refs dialog.
 */
import { StyleBoardSheet } from './StyleBoardSheet';
import { useStyleBoardDialog } from './StyleBoardDialog.hooks';

export default function StyleBoardDialog() {
  const { style, view } = useStyleBoardDialog();
  if (!style || !view) return null;
  return <StyleBoardSheet draft={style} view={view} />;
}
