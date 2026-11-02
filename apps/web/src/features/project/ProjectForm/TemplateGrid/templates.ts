import type { ProjectTemplate } from '@/ipc/client';
import diorama from '@/assets/template-diorama.jpg';
import room from '@/assets/template-room.jpg';
import character from '@/assets/template-character.jpg';

export const TEMPLATES: readonly {
  id: Exclude<ProjectTemplate, 'imported'>;
  title: string;
  detail: string;
  image: string;
  contain?: boolean;
}[] = [
  {
    id: 'floating-diorama',
    title: 'Floating diorama',
    detail: 'Creates one website scene: models on an island, with camera stops',
    image: diorama,
  },
  {
    id: 'room',
    title: 'Room',
    detail: 'Creates one website scene: an interior with baked light',
    image: room,
  },
  {
    id: 'character-showcase',
    title: 'Character showcase',
    detail: 'Creates a website page: one rigged model on a turntable',
    image: character,
    contain: true,
  },
];
