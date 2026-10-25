/**
 * Make object refs: list the objects in a concept, and Codex makes one
 * clean reference image of each, with the concept attached, into a new
 * "<name> objects" direction. Opens from a Lab image and a Refs card.
 */
import { ObjectRefsSheet } from './ObjectRefsSheet';
import { useObjectRefsDialog } from './ObjectRefsDialog.hooks';

export default function ObjectRefsDialog() {
  const objects = useObjectRefsDialog();
  if (!objects) return null;
  // Keyed on the source, so each open starts with an empty field.
  return <ObjectRefsSheet key={objects.source} objects={objects} />;
}
