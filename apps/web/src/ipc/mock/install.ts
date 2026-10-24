/**
 * Mock mode: imported first by `main.tsx` in a plain browser during
 * development, so every generated command resolves against the in-memory
 * backend. Tauri builds never reach this file.
 */
import { mockIPC } from '@tauri-apps/api/mocks';
import islandThumb from '../../assets/moon-island.jpg';
import characterThumb from '../../assets/template-character.jpg';
import houseThumb from '../../assets/template-room.jpg';
import { events } from '../generated/bindings';
import { registerMockFile } from '../files';
import { createBackend } from './backend';
import { DEFAULT_ROOT, HOME } from './fixtures';
import { PREVIEW_FILE } from './inbox-fixtures';
import crateGlb from './models/crate_clean.glb?url';
import characterGlb from './models/character_tails.glb?url';
import mergedGlb from './models/props_merged.glb?url';

const params = new URLSearchParams(window.location.search);
const root = DEFAULT_ROOT.replace('~', HOME);

const models: Record<string, string> = {
  'crate_clean.glb': crateGlb,
  'character_tails.glb': characterGlb,
  'props_merged.glb': mergedGlb,
};
for (const [file, fixture] of Object.entries(PREVIEW_FILE)) {
  const url = models[fixture];
  if (url) registerMockFile(`${root}/inbox/${file}`, url);
}
const thumbs: Record<string, string> = {
  'island.jpg': islandThumb,
  'shop-house.jpg': houseThumb,
  'character.jpg': characterThumb,
};
for (const [file, url] of Object.entries(thumbs)) {
  registerMockFile(`${root}/projects/moon-island/thumbs/${file}`, url);
}

const handle = createBackend({
  scenario: params.get('scenario') ?? 'first-run',
  probeDelay: Number(params.get('delay') ?? 700),
  pace: Number(params.get('pace') ?? 260),
  emit: event => {
    if (event.name === 'pipeline') void events.pipelineUpdate.emit(event.payload);
    else void events.inboxChanged.emit({ paths: [] });
  },
});

mockIPC((cmd, args) => handle(cmd, (args ?? {}) as Record<string, unknown>), {
  shouldMockEvents: true,
});
