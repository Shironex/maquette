/**
 * The reads a real macOS prompt can hold up: Keychain reads while it asks
 * to use a key, and reads of a linked game folder while it asks for
 * Documents. In mock mode `?promptMs=` holds them that long, so the waiting
 * states can be seen.
 */
import type { Args } from './backend-options';

const HELD = [
  'provider_keys',
  'provider_send_options',
  'provider_batch_options',
  'split_sheet',
  'image_api_status',
  'game_template_preview',
] as const;

export function holdForPrompt(handlers: Record<string, (args: Args) => unknown>, ms: number) {
  if (ms <= 0) return;
  for (const name of HELD) {
    const handler = handlers[name];
    if (!handler) continue;
    handlers[name] = async args => {
      await new Promise(resolve => setTimeout(resolve, ms));
      return handler(args);
    };
  }
}
