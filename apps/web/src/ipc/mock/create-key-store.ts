import type { KeyStatus, KeyTest, ProviderId } from '../client';
import { fail, type Args } from './backend-options';

/** The fake key tails: Tripo has one saved, Meshy does not. */
const TRIPO_TAIL = '9f2c';

/** The saved API keys, as the status rows and the key commands see them. */
export function createKeyStore() {
  const keys = new Map<ProviderId, string>([['tripo', TRIPO_TAIL]]);

  const status = (service: ProviderId): KeyStatus => ({
    service,
    saved: keys.has(service),
    tail: keys.get(service) ?? null,
  });

  const handlers: Record<string, (args: Args) => unknown> = {
    provider_keys: () => [status('tripo'), status('meshy')],
    provider_key_save: ({ service, key }) => {
      const text = String(key).trim();
      if (!text || /\s/.test(text)) fail('invalid', 'That does not look like an API key.');
      keys.set(service as ProviderId, text.slice(-4));
      return status(service as ProviderId);
    },
    provider_key_remove: ({ service }) => {
      keys.delete(service as ProviderId);
      return status(service as ProviderId);
    },
    provider_key_test: ({ service, key }): KeyTest => {
      const id = service as ProviderId;
      const name = id === 'tripo' ? 'Tripo' : 'Meshy';
      if (!key && !keys.has(id)) fail('invalid', `Save a ${name} API key first.`);
      return {
        service: id,
        ok: true,
        balance: id === 'tripo' ? 1240 : 380,
        message: `The key works. ${id === 'tripo' ? '1,240' : '380'} credits left.`,
      };
    },
  };

  return { has: (service: ProviderId) => keys.has(service), status, handlers };
}
