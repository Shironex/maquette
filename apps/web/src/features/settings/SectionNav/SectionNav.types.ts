import type { SectionId } from '../sections';

export interface SectionNavProps {
  active: SectionId;
  onSelect: (id: SectionId) => void;
}
